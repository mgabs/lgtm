use crate::comment::{CommentAnchor, CommentStore};
use crate::git::GitEngine;
use crate::guard::GuardAlert;
use crate::ipc::IpcBridge;
use crate::model::{DiffHunk, DiffSnapshot, FileDiff};
use crate::turn::TurnManager;
use crate::watcher::WatcherMessage;
use crossbeam_channel::Receiver;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;

pub use state::*;

mod audit;
#[cfg(test)]
mod background_tests;
mod command;
mod comments;
mod commit;
mod external;
mod input;
pub mod markdown;
mod motion;
mod pr;
mod refresh;
pub(crate) mod rows;
mod staging;
mod state;
#[cfg(test)]
pub(crate) mod test_util;
pub(crate) mod view_data;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    Diff,
    MarkdownPreview,
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
    forge_factory: fn(&std::path::Path) -> Box<dyn lgtm_core::forge::Forge>,
    pub comment_store: CommentStore,
    pub turn_manager: TurnManager,
    pub snapshot: DiffSnapshot,
    diff_revision: u64,
    fold_revision: u64,
    rows_cache: std::cell::RefCell<rows::Cache>,
    pub(crate) render_cache: std::cell::RefCell<crate::ui::RenderCache>,
    pub alerts: Vec<GuardAlert>,
    pub comments: Vec<CommentAnchor>,
    /// Reviewed hunks for the current review target, keyed by content hash.
    pub marks: Vec<lgtm_core::review::HunkMark>,
    /// Read-only GitHub state for the open PR, refreshed on demand via `:checks` / `:threads`.
    pub ci: Option<lgtm_core::github::CiSummary>,
    pub threads: Vec<lgtm_core::github::Thread>,

    // Full Repo Audit state
    pub audit: AuditState,
    pub(crate) view_data: view_data::Prepared,
    audit_jobs: audit::AuditJobs,
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
    pub agent_sink_kind: Option<crate::ipc::SinkKind>,
    pub status_message: Option<String>,
    pub should_quit: bool,
    quit_after_jobs: bool,

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
    pub comment_source_view: ActiveView,
    pub comment_target_alert: Option<crate::guard::GuardAlert>,

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

    // Editor configuration override
    pub editor_override: Option<String>,
    pub needs_terminal_clear: bool,

    // Markdown preview state
    pub markdown_preview: markdown::MarkdownPreviewState,

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
    local_refresh: refresh::LocalRefresh,
    review_generation: u64,
    pr_completion_revision: u64,
    pr_loading: Option<(u64, u64)>,
    ready_pr: Option<pr::LoadedPr>,
    pr_load_lock: std::sync::Arc<std::sync::Mutex<()>>,
    pr_load_latest: std::sync::Arc<std::sync::atomic::AtomicU64>,
    thread_revision: u64,
    thread_reconcile: std::collections::HashSet<(u64, String)>,
    thread_jobs:
        std::collections::HashMap<(u64, String), std::collections::VecDeque<comments::ThreadJob>>,
    posting_prs: std::collections::HashSet<u64>,
    pub posting_comments: std::collections::HashSet<String>,
    review_posting_prs: std::collections::HashSet<u64>,
    reply_drafts: std::collections::HashMap<(u64, String), String>,
    uncertain_replies: std::collections::HashMap<(u64, String), String>,
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
    let _timing = lgtm_core::debug::Span::new("review.scan");
    // a broken baseline file must not hide findings
    lgtm_core::findings::run_in(repo, files, cfg)
        .unwrap_or_else(|_| lgtm_core::findings::run(files, cfg))
}

impl App {
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
        let snapshot = DiffSnapshot::default();
        let initial_status = if is_git || explicit_pr.is_some() || explicit_rev.is_some() {
            Some("Loading diff…".into())
        } else {
            Some("Not a git repository (run git init)".into())
        };

        let config = lgtm_core::config::LgtmConfig::load(
            &repo_path,
            lgtm_core::config::user_config_path(false).as_deref(),
        )
        .unwrap_or_default();
        let found = lgtm_core::findings::Findings::default();
        let hidden = Hidden {
            suppressed: found.suppressed,
            existing: found.existing,
        };
        let alerts = found.active;
        let comments = comment_store.load_comments();
        let agent_pane = explicit_pane.or_else(IpcBridge::detect_agent_pane);

        let mut app = Self {
            view_data: view_data::Prepared::new(is_git),
            git_engine,
            forge_factory: lgtm_core::forge::open,
            comment_store,
            turn_manager,
            snapshot,
            diff_revision: 0,
            fold_revision: 0,
            rows_cache: Default::default(),
            render_cache: Default::default(),
            alerts,
            marks: Vec::new(),
            ci: None,
            threads: Vec::new(),
            config,
            hidden: std::cell::RefCell::new(hidden),
            comments,
            audit: AuditState::new(repo_path.clone()),
            audit_jobs: Default::default(),
            selected_file_idx: 0,
            diff_scroll_offset: 0,
            cursor_line_idx: 0,
            show_file_drawer: true,
            is_split_view: false,
            active_view: ActiveView::Diff,
            agent_pane,
            agent_sink_kind: None,
            status_message: initial_status,
            should_quit: false,
            quit_after_jobs: false,
            pr: PrState::new(None),
            active_revision: explicit_rev,
            editor_override: None,
            needs_terminal_clear: false,
            markdown_preview: Default::default(),
            file_picker: FilePicker::default(),
            pane_picker: PanePicker::default(),
            comment_input: String::new(),
            comment_reply_input: String::new(),
            comment_reply_target_idx: None,
            comment_list_selected_idx: 0,
            comment_source_view: ActiveView::Diff,
            comment_target_alert: None,
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
            local_refresh: Default::default(),
            review_generation: 0,
            pr_completion_revision: 0,
            pr_loading: None,
            ready_pr: None,
            pr_load_lock: Default::default(),
            pr_load_latest: Default::default(),
            thread_revision: 0,
            thread_reconcile: Default::default(),
            thread_jobs: Default::default(),
            posting_prs: Default::default(),
            posting_comments: Default::default(),
            review_posting_prs: Default::default(),
            reply_drafts: Default::default(),
            uncertain_replies: Default::default(),
        };

        app.load_marks();
        app.update_file_picker_filter();
        if let Some(pr) = explicit_pr {
            app.load_pr_async(pr);
        } else if is_git || app.active_revision.is_some() {
            app.reload_diff();
        }
        app
    }

    pub(crate) fn handle_background_events(&mut self) -> bool {
        if self.watcher_rx.is_empty()
            && self.jobs_rx.is_empty()
            && !(self.ready_pr.is_some()
                && self.active_view == ActiveView::Diff
                && !self.visual_mode)
            && !self.local_ready_to_apply()
            && !(self.quit_after_jobs && !self.has_pending_writes())
        {
            return false;
        }
        // Start timing only for UI work; otherwise logging an ignored debug-file
        // notification would itself produce another watcher notification.
        let mut timing = None;
        let mut watchers = 0;
        let mut callbacks = 0;
        let mut reload_needed = false;
        let mut watcher_changed_view = false;
        let mut watcher_budget = crate::event_loop::Budget::new(32);
        while watcher_budget.take() {
            let Ok(msg) = self.watcher_rx.try_recv() else {
                break;
            };
            watchers += 1;
            match msg {
                WatcherMessage::FilesChanged(paths) => {
                    if paths.iter().any(|path| {
                        path.ends_with(".lgtm/state.json")
                            || path
                                .strip_prefix(&self.git_engine.repo_path)
                                .is_ok_and(|relative| relative.starts_with(".lgtm/reviews"))
                    }) {
                        self.invalidate_pr_completion();
                        watcher_changed_view = true;
                    }
                    if paths.iter().any(|path| {
                        lgtm_core::watcher::affects_diff(&self.git_engine.repo_path, path)
                    }) {
                        self.invalidate_audit(false);
                    }
                    watcher_changed_view |= paths.iter().any(|path| {
                        lgtm_core::watcher::affects_diff(&self.git_engine.repo_path, path)
                            || [
                                ".lgtm/comments.jsonl",
                                ".lgtm/gate.json",
                                ".lgtm/state.json",
                                ".lgtm/prompt.txt",
                                ".lgtm/last-prompt.md",
                                ".lgtm/prompt.md",
                            ]
                            .iter()
                            .any(|name| path.ends_with(name))
                    });
                    if watcher_changed_view {
                        timing.get_or_insert_with(|| {
                            lgtm_core::debug::Span::new("ui.background_events")
                        });
                    }
                    // PR snapshots describe remote commits; workspace edits cannot change them.
                    // Still pick up directives written by MCP/other lgtm processes.
                    if paths.iter().any(|p| p.ends_with(".lgtm/comments.jsonl")) {
                        self.comments = self.comment_store.load_comments();
                    }
                    reload_needed |= (self.pr.current_number.is_none()
                        || self.local_refresh_requested())
                        && self.pr_loading.is_none()
                        && paths.iter().any(|p| {
                            lgtm_core::watcher::affects_diff(&self.git_engine.repo_path, p)
                        });
                }
                WatcherMessage::Error(err) => {
                    watcher_changed_view = true;
                    timing
                        .get_or_insert_with(|| lgtm_core::debug::Span::new("ui.background_events"));
                    self.status_message = Some(format!("Watcher error: {}", err));
                }
            }
        }

        if watcher_changed_view {
            self.invalidate_view_data();
        }

        // A bounded batch may leave changes unread. Conservatively supersede the local
        // snapshot before callbacks so an older worker cannot publish over those changes.
        reload_needed |= !self.watcher_rx.is_empty()
            && (self.pr.current_number.is_none() || self.local_refresh_requested())
            && self.pr_loading.is_none();
        if !self.watcher_rx.is_empty() {
            self.invalidate_audit(false);
        }
        // An unread notification may announce newer directives. Completion callbacks can
        // save re-anchored directives, so load durable edits before allowing those writes.
        if !self.watcher_rx.is_empty()
            && (!self.jobs_rx.is_empty() || self.ready_pr.is_some() || self.local_ready_to_apply())
        {
            timing.get_or_insert_with(|| lgtm_core::debug::Span::new("ui.background_events"));
            self.comments = self.comment_store.load_comments();
            watcher_changed_view = true;
        }
        if reload_needed
            || !self.jobs_rx.is_empty()
            || (self.ready_pr.is_some()
                && self.active_view == ActiveView::Diff
                && !self.visual_mode)
            || self.local_ready_to_apply()
            || self.quit_after_jobs
        {
            timing.get_or_insert_with(|| lgtm_core::debug::Span::new("ui.background_events"));
        }
        // Invalidate an older completion before draining callbacks from that worker.
        if reload_needed {
            self.invalidate_audit(false);
            self.reload_diff();
        }
        let mut callback_budget = crate::event_loop::Budget::new(8);
        while callback_budget.take() {
            let Ok(job) = self.jobs_rx.try_recv() else {
                break;
            };
            callbacks += 1;
            let _timing = lgtm_core::debug::Span::new("ui.callback");
            job(self);
        }

        let applying_pr =
            self.ready_pr.is_some() && self.active_view == ActiveView::Diff && !self.visual_mode;
        self.apply_ready_pr();
        let applying_local = self.apply_ready_local();
        if self.quit_after_jobs && !self.has_pending_writes() {
            self.should_quit = true;
        }
        if let Some(mut timing) = timing {
            timing.count("watcher_events", watchers);
            timing.count("callbacks", callbacks);
            timing.count("watchers_remaining", self.watcher_rx.len() as u64);
            timing.count("callbacks_remaining", self.jobs_rx.len() as u64);
            timing.count("applying_pr", u64::from(applying_pr));
            timing.count("applying_local", u64::from(applying_local));
        }
        watcher_changed_view
            || reload_needed
            || callbacks > 0
            || applying_pr
            || applying_local
            || self.should_quit
    }

    fn has_pending_writes(&self) -> bool {
        self.audit_exports_pending()
            || !self.posting_prs.is_empty()
            || !self.review_posting_prs.is_empty()
            || !self.thread_jobs.is_empty()
    }

    pub(crate) fn request_quit(&mut self) {
        if self.has_pending_writes() {
            self.quit_after_jobs = true;
            self.status_message = Some("Finishing pending updates before closing…".into());
        } else {
            self.should_quit = true;
        }
    }

    pub fn gate_directives_feedback(&self) -> Option<String> {
        let open_directives: Vec<String> = self
            .comments
            .iter()
            .filter(|c| !c.is_resolved && !c.orphaned)
            .map(|c| {
                let mut d = format!(
                    "{}:{} DIRECTIVE: {}",
                    c.file_path, c.target_line_no, c.comment_text
                );
                for reply in &c.replies {
                    d.push_str(&format!("\n↳ Reply: {}", reply));
                }
                d
            })
            .collect();
        if open_directives.is_empty() {
            None
        } else {
            Some(open_directives.join("\n"))
        }
    }

    pub fn resolved_editor(&self) -> String {
        self.editor_override
            .clone()
            .or_else(|| self.config.editor.clone())
            .or_else(|| std::env::var("LGTM_EDITOR").ok())
            .or_else(|| std::env::var("VISUAL").ok())
            .or_else(|| std::env::var("EDITOR").ok())
            .unwrap_or_else(|| "nvim".to_string())
    }

    pub fn approve_gate(&mut self) {
        self.invalidate_view_data();
        let gate_file = self.git_engine.repo_path.join(".lgtm/gate.json");
        if gate_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&gate_file) {
                if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(obj) = val.as_object_mut() {
                        obj.insert("status".to_string(), serde_json::json!("approved"));
                        if let Some(fb) = self.gate_directives_feedback() {
                            obj.insert("feedback".to_string(), serde_json::json!(fb));
                        }
                    }
                    let json_out = serde_json::to_string_pretty(&val).unwrap_or_default();
                    let _ = std::fs::write(&gate_file, json_out); // lgtm:ignore
                    self.status_message =
                        Some("✅ Approved agent gate! Agent unblocked.".to_string());
                    return;
                }
            }
        }
        self.status_message = Some("No active agent gate waiting.".to_string());
    }

    pub fn reject_gate(&mut self) {
        self.invalidate_view_data();
        let gate_file = self.git_engine.repo_path.join(".lgtm/gate.json");
        if gate_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&gate_file) {
                if let Ok(mut val) = serde_json::from_str::<serde_json::Value>(&content) {
                    let feedback = self.gate_directives_feedback();
                    if let Some(obj) = val.as_object_mut() {
                        obj.insert("status".to_string(), serde_json::json!("rejected"));
                        if let Some(ref fb) = feedback {
                            obj.insert("feedback".to_string(), serde_json::json!(fb));
                        }
                    }
                    let json_out = serde_json::to_string_pretty(&val).unwrap_or_default();
                    let _ = std::fs::write(&gate_file, json_out); // lgtm:ignore
                    let sink = self.agent_sink();
                    let msg = if let Some(ref fb) = feedback {
                        format!("❌ Rejected agent gate:\n{}", fb)
                    } else {
                        "❌ Rejected agent gate.".to_string()
                    };
                    let _ = sink.send(&msg);
                    let label = sink.label();
                    self.status_message =
                        Some(format!("❌ Rejected agent gate! Delivered to {}", label));
                    return;
                }
            }
        }
        self.status_message = Some("No active agent gate waiting.".to_string());
    }

    pub fn toggle_markdown_preview(&mut self) {
        if self.active_view == ActiveView::MarkdownPreview {
            self.active_view = ActiveView::Diff;
            return;
        }

        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            if markdown::is_markdown_file(&file.new_path) {
                self.markdown_preview.scroll_offset = 0;
                self.markdown_preview.rendered_rows.clear();
                self.markdown_preview.cached_file_idx = None;
                self.active_view = ActiveView::MarkdownPreview;
            } else {
                self.status_message =
                    Some("Markdown preview only available for .md files".to_string());
            }
        } else {
            self.status_message = Some("No file selected".to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_util::create_test_app;
    use crate::comment::CommentAnchor;

    #[test]
    fn test_gate_feedback_includes_directive() {
        let mut app = create_test_app();
        let gate_dir = app.git_engine.repo_path.join(".lgtm");
        let _ = std::fs::create_dir_all(&gate_dir);
        let gate_file = gate_dir.join("gate.json");
        std::fs::write(&gate_file, r#"{"status": "waiting"}"#).unwrap();

        app.comments.push(CommentAnchor {
            id: lgtm_core::comment::new_id(),
            file_path: "src/auth.rs".into(),
            target_line_no: 42,
            target_code: "password.verify()".into(),
            context_before: vec![],
            context_after: vec![],
            comment_text: "Enforce rate limiting on auth attempts".into(),
            replies: vec![],
            created_at_turn: 1,
            is_resolved: false,
            pr_number: None,
            remote_id: None,
            orphaned: false,
            track: Default::default(),
        });

        app.reject_gate();

        let updated: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&gate_file).unwrap()).unwrap();
        assert_eq!(updated.get("status").unwrap(), "rejected");
        let fb = updated.get("feedback").unwrap().as_str().unwrap();
        assert!(fb.contains("src/auth.rs:42 DIRECTIVE: Enforce rate limiting on auth attempts"));
    }

    #[test]
    fn test_approve_gate_feedback_includes_directive() {
        let mut app = create_test_app();
        let gate_dir = app.git_engine.repo_path.join(".lgtm");
        let _ = std::fs::create_dir_all(&gate_dir);
        let gate_file = gate_dir.join("gate.json");
        std::fs::write(&gate_file, r#"{"status": "waiting"}"#).unwrap();

        app.comments.push(CommentAnchor {
            id: lgtm_core::comment::new_id(),
            file_path: "src/auth.rs".into(),
            target_line_no: 10,
            target_code: "foo()".into(),
            context_before: vec![],
            context_after: vec![],
            comment_text: "Add unit test".into(),
            replies: vec![],
            created_at_turn: 1,
            is_resolved: false,
            pr_number: None,
            remote_id: None,
            orphaned: false,
            track: Default::default(),
        });

        app.approve_gate();

        let updated: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&gate_file).unwrap()).unwrap();
        assert_eq!(updated.get("status").unwrap(), "approved");
        let fb = updated.get("feedback").unwrap().as_str().unwrap();
        assert!(fb.contains("src/auth.rs:10 DIRECTIVE: Add unit test"));
    }
}
