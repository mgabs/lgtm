use crate::comment::{CommentAnchor, CommentStore};
use crate::git::GitEngine;
use crate::guard::{GuardAlert, SecurityAndTestGuard};
use crate::ipc::IpcBridge;
use crate::model::{DiffHunk, DiffSnapshot, FileDiff};
use crate::turn::TurnManager;
use crate::watcher::WatcherMessage;
use crossbeam_channel::Receiver;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
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
    PrPicker,
    Help,
    Search,
    CommandLine,
    CommitModal,
    AuditReport,
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

    // Full Repo Audit state
    pub audit_alerts: Vec<GuardAlert>,
    pub audit_files: Vec<FileDiff>,
    pub audit_selected_idx: usize,
    pub audit_filter_severity: Option<crate::guard::Severity>,
    pub audit_root_dir: PathBuf,
    pub audit_focus: AuditPaneFocus,
    pub audit_file_filter: Option<String>,
    pub audit_inspector_scroll: usize,
    pub audit_maximized_inspector: bool,

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
    pub current_pr_number: Option<u64>,
    pub pr_list: Vec<crate::pr::PullRequestSummary>,
    pub pr_selected_idx: usize,

    // File picker state
    pub picker_query: String,
    pub picker_selected_idx: usize,
    pub picker_filtered_indices: Vec<usize>,

    // Pane picker state
    pub pane_list: Vec<crate::ipc::TmuxPane>,
    pub pane_selected_idx: usize,

    // Comment input and reply state
    pub comment_input: String,
    pub comment_reply_input: String,
    pub comment_reply_target_idx: Option<usize>,
    pub comment_list_selected_idx: usize,

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
}

impl App {
    pub fn new(
        repo_path: PathBuf,
        watcher_rx: Receiver<WatcherMessage>,
        explicit_pane: Option<String>,
        explicit_pr: Option<u64>,
        explicit_rev: Option<String>,
    ) -> Self {
        let git_engine = GitEngine::new(&repo_path);
        let comment_store = CommentStore::new(&repo_path);
        let turn_manager = TurnManager::new(&repo_path);

        let is_git = git_engine.is_git_repo();
        let (snapshot, initial_status) = if let Some(pr_num) = explicit_pr {
            match crate::pr::PrEngine::fetch_pr_diff(pr_num) {
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
            match git_engine.compute_revision_diff(rev) {
                Ok(snap) => {
                    let msg = format!(
                        "Loaded revision diff '{}' ({} files changed)",
                        rev,
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

        let alerts = SecurityAndTestGuard::scan_snapshot(&snapshot.files);
        let comments = comment_store.load_comments();
        let agent_pane = explicit_pane.or_else(IpcBridge::detect_agent_pane);

        let mut app = Self {
            git_engine,
            comment_store,
            turn_manager,
            snapshot,
            alerts,
            comments,
            audit_alerts: Vec::new(),
            audit_files: Vec::new(),
            audit_selected_idx: 0,
            audit_filter_severity: None,
            audit_root_dir: repo_path.clone(),
            audit_focus: AuditPaneFocus::Issues,
            audit_file_filter: None,
            audit_inspector_scroll: 0,
            audit_maximized_inspector: false,
            selected_file_idx: 0,
            diff_scroll_offset: 0,
            cursor_line_idx: 0,
            show_file_drawer: true,
            is_split_view: false,
            active_view: ActiveView::Diff,
            agent_pane,
            status_message: initial_status,
            should_quit: false,
            current_pr_number: explicit_pr,
            active_revision: explicit_rev,
            pr_list: Vec::new(),
            pr_selected_idx: 0,
            picker_query: String::new(),
            picker_selected_idx: 0,
            picker_filtered_indices: Vec::new(),
            pane_list: Vec::new(),
            pane_selected_idx: 0,
            comment_input: String::new(),
            comment_reply_input: String::new(),
            comment_reply_target_idx: None,
            comment_list_selected_idx: 0,
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
        };

        app.update_file_picker_filter();
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

        if reload_needed {
            self.reload_diff();
        }
    }

    pub fn reload_diff(&mut self) {
        if let Some(pr_num) = self.current_pr_number {
            if let Ok(new_snapshot) = crate::pr::PrEngine::fetch_pr_diff(pr_num) {
                self.alerts = SecurityAndTestGuard::scan_snapshot(&new_snapshot.files);
                self.snapshot = new_snapshot;
                self.comments = self.comment_store.load_comments();

                if self.selected_file_idx >= self.snapshot.files.len()
                    && !self.snapshot.files.is_empty()
                {
                    self.selected_file_idx = self.snapshot.files.len() - 1;
                }
                self.update_file_picker_filter();
                self.status_message = Some(format!(
                    "PR #{} diff updated ({} alerts)",
                    pr_num,
                    self.alerts.len()
                ));
            }
            return;
        }

        if let Some(rev) = self.active_revision.clone() {
            if let Ok(new_snapshot) = self.git_engine.compute_revision_diff(&rev) {
                self.alerts = SecurityAndTestGuard::scan_snapshot(&new_snapshot.files);
                self.snapshot = new_snapshot;
                self.comments = self.comment_store.load_comments();

                if self.selected_file_idx >= self.snapshot.files.len()
                    && !self.snapshot.files.is_empty()
                {
                    self.selected_file_idx = self.snapshot.files.len() - 1;
                }
                self.update_file_picker_filter();
                self.status_message = Some(format!(
                    "Revision '{}' updated ({} alerts)",
                    rev,
                    self.alerts.len()
                ));
            }
            return;
        }

        if let Ok(new_snapshot) = self.git_engine.compute_diff_snapshot() {
            self.alerts = SecurityAndTestGuard::scan_snapshot(&new_snapshot.files);
            self.snapshot = new_snapshot;
            self.comments = self.comment_store.load_comments();

            if self.selected_file_idx >= self.snapshot.files.len()
                && !self.snapshot.files.is_empty()
            {
                self.selected_file_idx = self.snapshot.files.len() - 1;
            }
            self.update_file_picker_filter();
            self.status_message = Some(format!("Diff updated ({} alerts)", self.alerts.len()));
        }
    }
}
