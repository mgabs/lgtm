use crate::comment::{CommentAnchor, CommentStore};
use crate::git::GitEngine;
use crate::guard::GuardAlert;
use crate::ipc::sink::AgentSink;
use crate::ipc::IpcBridge;
use crate::model::{DiffHunk, DiffSnapshot, FileDiff};
use crate::turn::TurnManager;
use crate::watcher::WatcherMessage;
use crossbeam_channel::Receiver;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;

pub use state::*;

mod audit;
mod command;
mod comments;
mod commit;
mod external;
mod input;
mod motion;
mod pr;
mod staging;
mod state;
#[cfg(test)]
mod test_util;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    Diff,
    FilePicker,
    PanePicker,
    CommentInput,
    CommentReply,
    CommentList,
    ThreadView,
    PrPicker,
    Help,
    Search,
    CommandLine,
    CommitModal,
    AuditReport,
    AgentPrompt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditPaneFocus {
    Files,
    Issues,
    Inspector,
}

pub struct App {
    pub git_engine: GitEngine,
    pub comment_store: CommentStore,
    pub turn_manager: TurnManager,
    pub snapshot: DiffSnapshot,
    pub alerts: Vec<GuardAlert>,
    pub comments: Vec<CommentAnchor>,
    /// Reviewed hunks for the current review target, keyed by content hash.
    pub marks: Vec<lgtm_core::review::HunkMark>,
    /// Read-only GitHub state for the open PR, refreshed on demand via `:checks` / `:threads`.
    pub ci: Option<lgtm_core::github::CiSummary>,
    pub threads: Vec<lgtm_core::github::Thread>,

    // Full Repo Audit state
    pub audit: AuditState,
    pub config: lgtm_core::config::LgtmConfig,
    /// Findings hidden from the alert list: inline/config suppressed and baselined.
    pub hidden: std::cell::RefCell<Hidden>,

    pub selected_file_idx: usize,
    pub diff_scroll_offset: usize,
    pub cursor_line_idx: usize,
    pub show_file_drawer: bool,
    pub is_split_view: bool,
    pub active_view: ActiveView,
    pub agent_pane: Option<String>,
    pub status_message: Option<String>,
    pub should_quit: bool,

    // PR Forge state
    pub pr: PrState,

    // File picker state
    pub file_picker: FilePicker,

    // Pane picker state
    pub pane_picker: PanePicker,

    // Comment input and reply state
    pub comment_input: String,
    pub comment_reply_input: String,
    pub comment_reply_target_idx: Option<usize>,
    pub comment_list_selected_idx: usize,

    // PR Thread inspection state
    pub thread_selected_idx: usize,
    pub thread_scroll_offset: usize,

    // Commit Forge modal state
    pub commit_message_input: String,

    // Help modal tab state
    pub help_selected_tab: usize,

    // Ex-command line state (:cmds)
    pub command: CommandLine,

    // Active Git Revision
    pub active_revision: Option<String>,

    // Alert navigation index
    pub selected_alert_idx: usize,

    // Vim motion & search state
    pub pending_leader: Option<char>,
    pub search_query: String,
    pub search_matches: Vec<(usize, usize)>, // (file_idx, line_idx)
    pub search_match_idx: usize,

    // Visual line selection mode (V)
    pub visual_mode: bool,
    pub visual_anchor_idx: Option<usize>,

    // Watcher receiver
    pub watcher_rx: Receiver<WatcherMessage>,

    // Results of slow remote calls, applied on the UI thread by `handle_background_events`
    jobs_tx: crossbeam_channel::Sender<Job>,
    jobs_rx: Receiver<Job>,
}

pub type Job = Box<dyn FnOnce(&mut App) + Send>;

#[derive(Debug, Default, Clone)]
pub struct Hidden {
    pub suppressed: Vec<GuardAlert>,
    pub existing: Vec<GuardAlert>,
}

fn scan_findings(
    repo: &std::path::Path,
    files: &[crate::model::FileDiff],
    cfg: &lgtm_core::config::LgtmConfig,
) -> lgtm_core::findings::Findings {
    // a broken baseline file must not hide findings
    lgtm_core::findings::run_in(repo, files, cfg)
        .unwrap_or_else(|_| lgtm_core::findings::run(files, cfg))
}

impl App {
    /// Guards plus config pipeline; hidden findings are kept in `self.hidden`.
    pub fn scan(&self, files: &[crate::model::FileDiff]) -> Vec<GuardAlert> {
        let f = scan_findings(&self.git_engine.repo_path, files, &self.config);
        *self.hidden.borrow_mut() = Hidden {
            suppressed: f.suppressed,
            existing: f.existing,
        };
        f.active
    }

    pub fn new(
        repo_path: PathBuf,
        watcher_rx: Receiver<WatcherMessage>,
        explicit_pane: Option<String>,
        explicit_pr: Option<u64>,
        explicit_rev: Option<String>,
    ) -> Self {
        let (jobs_tx, jobs_rx) = crossbeam_channel::unbounded();
        let git_engine = GitEngine::new(&repo_path);
        let comment_store = CommentStore::new(&repo_path);
        let turn_manager = TurnManager::new(&repo_path);

        let is_git = git_engine.is_git_repo();
        let (snapshot, initial_status) = if let Some(pr_num) = explicit_pr {
            match crate::diff_source::DiffSource::snapshot(&crate::diff_source::PullRequestSource {
                pr: pr_num,
                repo: repo_path.clone(),
            }) {
                Ok(snap) => {
                    let msg = format!("Loaded PR #{} ({} files changed)", pr_num, snap.files.len());
                    (snap, Some(msg))
                }
                Err(e) => {
                    let msg = format!("Failed to fetch PR #{}: {}", pr_num, e);
                    (DiffSnapshot::default(), Some(msg))
                }
            }
        } else if let Some(ref rev) = explicit_rev {
            let expanded = git_engine.expand_branch_spec(rev);
            match git_engine.compute_revision_diff(&expanded) {
                Ok(snap) => {
                    let msg = format!(
                        "Loaded revision diff '{}' ({} files changed)",
                        expanded,
                        snap.files.len()
                    );
                    (snap, Some(msg))
                }
                Err(e) => {
                    let msg = format!("Failed to load revision '{}': {}", rev, e);
                    (DiffSnapshot::default(), Some(msg))
                }
            }
        } else if is_git {
            let snap = git_engine.compute_diff_snapshot().unwrap_or_default();
            let msg = if snap.files.is_empty() {
                Some("Working tree clean (monitoring workspace)".to_string())
            } else {
                Some(format!("Diff loaded ({} changed files)", snap.files.len()))
            };
            (snap, msg)
        } else {
            (
                DiffSnapshot::default(),
                Some("Not a git repository (run git init)".to_string()),
            )
        };

        let config = lgtm_core::config::LgtmConfig::load(
            &repo_path,
            lgtm_core::config::user_config_path(false).as_deref(),
        )
        .unwrap_or_default();
        let found = scan_findings(&repo_path, &snapshot.files, &config);
        let hidden = Hidden {
            suppressed: found.suppressed,
            existing: found.existing,
        };
        let alerts = found.active;
        let comments = comment_store.load_comments();
        let agent_pane = explicit_pane.or_else(IpcBridge::detect_agent_pane);

        let mut app = Self {
            git_engine,
            comment_store,
            turn_manager,
            snapshot,
            alerts,
            marks: Vec::new(),
            ci: None,
            threads: Vec::new(),
            config,
            hidden: std::cell::RefCell::new(hidden),
            comments,
            audit: AuditState::new(repo_path.clone()),
            selected_file_idx: 0,
            diff_scroll_offset: 0,
            cursor_line_idx: 0,
            show_file_drawer: true,
            is_split_view: false,
            active_view: ActiveView::Diff,
            agent_pane,
            status_message: initial_status,
            should_quit: false,
            pr: PrState::new(explicit_pr),
            active_revision: explicit_rev,
            file_picker: FilePicker::default(),
            pane_picker: PanePicker::default(),
            comment_input: String::new(),
            comment_reply_input: String::new(),
            comment_reply_target_idx: None,
            comment_list_selected_idx: 0,
            thread_selected_idx: 0,
            thread_scroll_offset: 0,
            commit_message_input: String::new(),
            help_selected_tab: 0,
            command: CommandLine::default(),
            selected_alert_idx: 0,
            pending_leader: None,
            search_query: String::new(),
            search_matches: Vec::new(),
            search_match_idx: 0,
            visual_mode: false,
            visual_anchor_idx: None,
            watcher_rx,
            jobs_tx,
            jobs_rx,
        };

        app.load_marks();
        app.update_file_picker_filter();
        if explicit_pr.is_some() {
            app.refresh_github_async(true);
        }
        app
    }

    pub fn handle_background_events(&mut self) {
        let mut reload_needed = false;
        while let Ok(msg) = self.watcher_rx.try_recv() {
            match msg {
                WatcherMessage::FilesChanged(_) => {
                    reload_needed = true;
                }
                WatcherMessage::Error(err) => {
                    self.status_message = Some(format!("Watcher error: {}", err));
                }
            }
        }

        while let Ok(job) = self.jobs_rx.try_recv() {
            job(self);
        }

        if reload_needed {
            self.reload_diff();
        }
    }

    pub fn diff_source(&self) -> Box<dyn crate::diff_source::DiffSource + '_> {
        use crate::diff_source::*;
        if let Some(n) = self.pr.current_number {
            Box::new(PullRequestSource {
                pr: n,
                repo: self.git_engine.repo_path.clone(),
            })
        } else if let Some(rev) = self.active_revision.as_deref() {
            Box::new(RevisionSource {
                engine: &self.git_engine,
                rev,
            })
        } else {
            Box::new(WorkingTreeSource(&self.git_engine))
        }
    }

    pub fn reload_diff(&mut self) {
        let source = self.diff_source();
        let label = source.describe();
        let Ok(new_snapshot) = source.snapshot() else {
            return;
        };
        drop(source);
        self.alerts = self.scan(&new_snapshot.files);
        self.snapshot = new_snapshot;
        self.comments = self.comment_store.load_comments();
        let turn = self.turn_manager.load_state().current_turn;
        let moved = lgtm_core::review::reanchor_anchors(&mut self.comments, &self.snapshot.files);
        let rechecked = lgtm_core::outcome::recheck(&mut self.comments, &self.snapshot.files, turn);
        if moved || rechecked {
            let _ = self.comment_store.save_comments(&self.comments);
        }
        self.load_marks();
        if self.selected_file_idx >= self.snapshot.files.len() && !self.snapshot.files.is_empty() {
            self.selected_file_idx = self.snapshot.files.len() - 1;
        }
        self.update_file_picker_filter();
        self.status_message = Some(format!("{} updated ({} alerts)", label, self.alerts.len()));
    }

    pub fn check_gate_status(&self) -> Option<(String, String)> {
        let gate_file = self.git_engine.repo_path.join(".lgtm/gate.json");
        if gate_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&gate_file) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    let status = val
                        .get("status")
                        .and_then(|s| s.as_str())
                        .unwrap_or("waiting")
                        .to_string();
                    let msg = val
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("Agent awaiting approval")
                        .to_string();
                    return Some((status, msg));
                }
            }
        }
        None
    }

    pub fn approve_gate(&mut self) {
        let gate_file = self.git_engine.repo_path.join(".lgtm/gate.json");
        if gate_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&gate_file) {
                if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(obj) = val.as_object_mut() {
                        obj.insert("status".to_string(), serde_json::json!("approved"));
                    }
                    let _ = std::fs::write(
                        &gate_file,
                        serde_json::to_string_pretty(&val).unwrap_or_default(),
                    );
                    self.status_message =
                        Some("✅ Approved agent gate! Agent unblocked.".to_string());
                    return;
                }
            }
        }
        self.status_message = Some("No active agent gate waiting.".to_string());
    }

    pub fn reject_gate(&mut self) {
        let gate_file = self.git_engine.repo_path.join(".lgtm/gate.json");
        if gate_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&gate_file) {
                if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(obj) = val.as_object_mut() {
                        obj.insert("status".to_string(), serde_json::json!("rejected"));
                    }
                    let _ = std::fs::write(
                        &gate_file,
                        serde_json::to_string_pretty(&val).unwrap_or_default(),
                    );
                    self.status_message =
                        Some("❌ Rejected agent gate! Agent notified.".to_string());
                    return;
                }
            }
        }
        self.status_message = Some("No active agent gate waiting.".to_string());
    }

    pub fn load_agent_prompt_context(&self) -> (String, String) {
        // 1. Check .lgtm/gate.json
        if let Some((_, msg)) = self.check_gate_status() {
            if !msg.is_empty() && msg != "Agent awaiting approval" {
                return ("Gate Context".to_string(), msg);
            }
        }

        // 2. Check .lgtm/prompt.txt or .lgtm/last-prompt.md
        for name in &["prompt.txt", "last-prompt.md", "prompt.md"] {
            let p = self.git_engine.repo_path.join(".lgtm").join(name);
            if p.exists() {
                if let Ok(content) = std::fs::read_to_string(&p) {
                    let trimmed = content.trim();
                    if !trimmed.is_empty() {
                        return (format!(".lgtm/{}", name), trimmed.to_string());
                    }
                }
            }
        }

        // 3. Check git HEAD commit message
        if let Some(msg) = self.git_engine.last_commit_message() {
            if !msg.is_empty() {
                return ("Git HEAD Commit".to_string(), msg);
            }
        }

        (
            "No Prompt Found".to_string(),
            "No prompt context found.\n\nTo provide agent prompt context:\n- Pass message with `lgtm gate -m '<prompt>'`\n- Write prompt to `.lgtm/prompt.txt`\n- Or commit with a descriptive message.".to_string(),
        )
    }
}
