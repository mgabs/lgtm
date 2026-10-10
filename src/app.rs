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
}

pub struct App {
    pub git_engine: GitEngine,
    pub comment_store: CommentStore,
    pub turn_manager: TurnManager,
    pub snapshot: DiffSnapshot,
    pub alerts: Vec<GuardAlert>,
    pub comments: Vec<CommentAnchor>,

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

    // Ex-command line state (:cmds)
    pub command_input: String,
    pub command_history: Vec<String>,
    pub command_history_idx: Option<usize>,
    pub command_completions: Vec<String>,
    pub command_completion_idx: Option<usize>,
    pub command_prefix: String,

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
            command_input: String::new(),
            command_history: Vec::new(),
            command_history_idx: None,
            command_completions: Vec::new(),
            command_completion_idx: None,
            command_prefix: String::new(),
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

    pub fn handle_key(&mut self, key: KeyEvent) {
        match self.active_view {
            ActiveView::Diff => self.handle_diff_key(key),
            ActiveView::FilePicker => self.handle_file_picker_key(key),
            ActiveView::PanePicker => self.handle_pane_picker_key(key),
            ActiveView::CommentInput => self.handle_comment_input_key(key),
            ActiveView::CommentReply => self.handle_comment_reply_key(key),
            ActiveView::CommentList => self.handle_comment_list_key(key),
            ActiveView::PrPicker => self.handle_pr_picker_key(key),
            ActiveView::Help => self.handle_help_key(key),
            ActiveView::Search => self.handle_search_key(key),
            ActiveView::CommandLine => self.handle_command_line_key(key),
            ActiveView::CommitModal => self.handle_commit_modal_key(key),
        }
    }

    fn handle_diff_key(&mut self, key: KeyEvent) {
        // Multi-key leader sequences (g, ], [, ' ', z)
        if let Some(leader) = self.pending_leader.take() {
            match (leader, key.code) {
                ('g', KeyCode::Char('g')) => {
                    self.cursor_to_top();
                    return;
                }
                (']', KeyCode::Char('f')) => {
                    self.next_file();
                    return;
                }
                (']', KeyCode::Char('u')) => {
                    self.next_unreviewed_file();
                    return;
                }
                (']', KeyCode::Char('h')) => {
                    self.next_hunk();
                    return;
                }
                (']', KeyCode::Char('c')) => {
                    self.next_comment();
                    return;
                }
                (']', KeyCode::Char('w')) => {
                    self.next_alert();
                    return;
                }
                ('[', KeyCode::Char('f')) => {
                    self.prev_file();
                    return;
                }
                ('[', KeyCode::Char('u')) => {
                    self.prev_unreviewed_file();
                    return;
                }
                ('[', KeyCode::Char('h')) => {
                    self.prev_hunk();
                    return;
                }
                ('[', KeyCode::Char('c')) => {
                    self.prev_comment();
                    return;
                }
                ('[', KeyCode::Char('w')) => {
                    self.prev_alert();
                    return;
                }
                ('z', KeyCode::Char('z')) => {
                    self.center_cursor();
                    return;
                }
                ('z', KeyCode::Char('t')) => {
                    self.cursor_to_top_of_view();
                    return;
                }
                ('z', KeyCode::Char('b')) => {
                    self.cursor_to_bottom_of_view();
                    return;
                }
                ('z', KeyCode::Char('c')) => {
                    self.fold_current_hunk();
                    return;
                }
                ('z', KeyCode::Char('o')) => {
                    self.unfold_current_hunk();
                    return;
                }
                ('z', KeyCode::Char('M')) => {
                    self.fold_all_hunks();
                    return;
                }
                ('z', KeyCode::Char('R')) => {
                    self.unfold_all_hunks();
                    return;
                }
                (' ', KeyCode::Char('y')) => {
                    self.yank_reference_to_clipboard();
                    return;
                }
                (' ', KeyCode::Char('c')) => {
                    self.comment_input.clear();
                    self.active_view = ActiveView::CommentInput;
                    return;
                }
                (' ', KeyCode::Char('l')) => {
                    self.comment_list_selected_idx = 0;
                    self.active_view = ActiveView::CommentList;
                    return;
                }
                (' ', KeyCode::Char('P')) => {
                    self.open_pr_picker();
                    return;
                }
                (' ', KeyCode::Char('r')) | (' ', KeyCode::Char('R')) => {
                    self.toggle_resolve_current_comment();
                    return;
                }
                (' ', KeyCode::Char('d')) => {
                    self.delete_current_line_comment();
                    return;
                }
                (' ', KeyCode::Char('x')) => {
                    self.discard_current_hunk();
                    return;
                }
                (' ', KeyCode::Char('w')) => {
                    self.next_alert();
                    return;
                }
                (' ', KeyCode::Char('s')) => {
                    self.export_and_send_batch_review();
                    return;
                }
                (' ', KeyCode::Char('p')) => {
                    if let Ok(panes) = IpcBridge::list_tmux_panes() {
                        self.pane_list = panes;
                        self.pane_selected_idx = 0;
                        self.active_view = ActiveView::PanePicker;
                    }
                    return;
                }
                _ => {
                    if leader == ']' {
                        self.next_file();
                    } else if leader == '[' {
                        self.prev_file();
                    }
                }
            }
        }

        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('c')) | (_, KeyCode::Char('q')) => {
                self.should_quit = true;
            }
            (_, KeyCode::Char('?')) => {
                self.active_view = ActiveView::Help;
            }
            (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
                self.export_and_send_batch_review();
            }
            (KeyModifiers::CONTROL, KeyCode::Char('p'))
            | (KeyModifiers::NONE, KeyCode::Char('f')) => {
                self.active_view = ActiveView::FilePicker;
                self.picker_query.clear();
                self.picker_selected_idx = 0;
                self.update_file_picker_filter();
            }
            (_, KeyCode::Esc) => {
                if self.visual_mode {
                    self.toggle_visual_mode();
                }
            }
            (KeyModifiers::NONE, KeyCode::Char('x')) => {
                if self.visual_mode {
                    self.discard_selected_lines();
                } else if self.has_comment_on_cursor() {
                    self.delete_current_line_comment();
                } else {
                    self.discard_current_hunk();
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
                self.page_down(15);
            }
            (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
                self.page_up(15);
            }
            (_, KeyCode::Tab) => {
                self.show_file_drawer = !self.show_file_drawer;
            }
            (_, KeyCode::Char('d')) => {
                self.is_split_view = !self.is_split_view;
                let mode = if self.is_split_view {
                    "Split View"
                } else {
                    "Unified View"
                };
                self.status_message = Some(format!("View mode: {}", mode));
            }
            (_, KeyCode::Char('c')) => {
                self.comment_input.clear();
                self.active_view = ActiveView::CommentInput;
            }
            (_, KeyCode::Char('C')) => {
                self.open_commit_modal();
            }
            (_, KeyCode::Char('r')) => {
                if let Some(idx) = self.comment_idx_at_cursor() {
                    self.start_reply_to_comment(idx);
                } else {
                    self.comment_input.clear();
                    self.active_view = ActiveView::CommentInput;
                }
            }
            (_, KeyCode::Char('R')) => {
                self.toggle_resolve_current_comment();
            }
            (_, KeyCode::Char('T')) => {
                self.run_test_mutation_probe();
            }
            (_, KeyCode::Char('g')) => {
                self.pending_leader = Some('g');
            }
            (_, KeyCode::Char('G')) => {
                self.cursor_to_bottom();
            }
            (_, KeyCode::Char(']')) => {
                self.pending_leader = Some(']');
            }
            (_, KeyCode::Char('[')) => {
                self.pending_leader = Some('[');
            }
            (_, KeyCode::Char(' ')) => {
                self.pending_leader = Some(' ');
            }
            (_, KeyCode::Char('0')) | (_, KeyCode::Char('^')) => {
                self.cursor_line_idx = 0;
            }
            (_, KeyCode::Char('$')) => {
                self.cursor_to_bottom();
            }
            (_, KeyCode::Char('y')) => {
                self.yank_current_code_line();
            }
            (_, KeyCode::Char('/')) => {
                self.search_query.clear();
                self.active_view = ActiveView::Search;
            }
            (_, KeyCode::Char('n')) => {
                self.next_search_match();
            }
            (_, KeyCode::Char('N')) => {
                self.prev_search_match();
            }
            (_, KeyCode::Char('*')) => {
                self.search_symbol_under_cursor(true);
            }
            (_, KeyCode::Char('#')) => {
                self.search_symbol_under_cursor(false);
            }
            (_, KeyCode::Char('w')) => {
                self.next_alert();
            }
            (_, KeyCode::Char('W')) => {
                self.prev_alert();
            }
            (_, KeyCode::Char('j')) | (_, KeyCode::Down) => {
                self.cursor_down();
            }
            (_, KeyCode::Char('k')) | (_, KeyCode::Up) => {
                self.cursor_up();
            }
            (_, KeyCode::Char('V')) => {
                self.toggle_visual_mode();
            }
            (_, KeyCode::Char('v')) => {
                self.toggle_review_current_file();
            }
            (_, KeyCode::Char('s')) => {
                if self.visual_mode {
                    self.stage_selected_lines();
                } else {
                    self.stage_current_hunk();
                }
            }
            (_, KeyCode::Char('u')) => {
                if self.visual_mode {
                    self.unstage_selected_lines();
                } else {
                    self.unstage_current_hunk();
                }
            }
            (_, KeyCode::Char('S')) => {
                self.toggle_stage_current_file();
            }
            (_, KeyCode::Char('U')) => {
                self.unstage_current_file();
            }
            (_, KeyCode::Char('X')) => {
                self.discard_current_file_changes();
            }
            (_, KeyCode::Char('o')) | (_, KeyCode::Char('e')) => {
                self.open_in_editor();
            }
            (_, KeyCode::Char('z')) => {
                self.pending_leader = Some('z');
            }
            (_, KeyCode::Char('p')) => {
                if let Ok(panes) = IpcBridge::list_tmux_panes() {
                    self.pane_list = panes;
                    self.pane_selected_idx = 0;
                    self.active_view = ActiveView::PanePicker;
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('f')) | (_, KeyCode::PageDown) => {
                self.page_down(25);
            }
            (KeyModifiers::CONTROL, KeyCode::Char('b')) | (_, KeyCode::PageUp) => {
                self.page_up(25);
            }
            (_, KeyCode::Char('}')) => {
                self.next_paragraph();
            }
            (_, KeyCode::Char('{')) => {
                self.prev_paragraph();
            }
            (_, KeyCode::Char(':')) => {
                self.command_input.clear();
                self.command_history_idx = None;
                self.active_view = ActiveView::CommandLine;
            }
            (_, KeyCode::Enter) => {
                self.inject_reference_to_agent();
            }
            _ => {}
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.active_view = ActiveView::Diff;
            }
            KeyCode::Enter => {
                let query = self.search_query.clone();
                if !query.is_empty() {
                    self.execute_search(&query);
                }
                self.active_view = ActiveView::Diff;
            }
            KeyCode::Backspace => {
                self.search_query.pop();
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
            }
            _ => {}
        }
    }

    fn handle_command_line_key(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('c')) | (_, KeyCode::Esc) => {
                self.command_input.clear();
                self.command_history_idx = None;
                self.command_completions.clear();
                self.command_completion_idx = None;
                self.command_prefix.clear();
                self.active_view = ActiveView::Diff;
            }
            (_, KeyCode::Backspace) => {
                self.command_completions.clear();
                self.command_completion_idx = None;
                self.command_prefix.clear();
                if self.command_input.is_empty() {
                    self.active_view = ActiveView::Diff;
                } else {
                    self.command_input.pop();
                }
            }
            (_, KeyCode::Up) | (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
                if !self.command_history.is_empty() {
                    let next_idx = match self.command_history_idx {
                        Some(0) => 0,
                        Some(i) => i - 1,
                        None => self.command_history.len() - 1,
                    };
                    self.command_history_idx = Some(next_idx);
                    self.command_input = self.command_history[next_idx].clone();
                    self.command_completions.clear();
                    self.command_completion_idx = None;
                }
            }
            (_, KeyCode::Down) | (KeyModifiers::CONTROL, KeyCode::Char('n')) => {
                if let Some(i) = self.command_history_idx {
                    if i + 1 < self.command_history.len() {
                        let next_idx = i + 1;
                        self.command_history_idx = Some(next_idx);
                        self.command_input = self.command_history[next_idx].clone();
                    } else {
                        self.command_history_idx = None;
                        self.command_input.clear();
                    }
                    self.command_completions.clear();
                    self.command_completion_idx = None;
                }
            }
            (_, KeyCode::BackTab) | (KeyModifiers::SHIFT, KeyCode::Tab) => {
                self.autocomplete_prev();
            }
            (_, KeyCode::Tab) => {
                self.autocomplete_next();
            }
            (_, KeyCode::Enter) => {
                let cmd = self.command_input.trim().to_string();
                self.command_completions.clear();
                self.command_completion_idx = None;
                self.command_prefix.clear();
                if !cmd.is_empty() {
                    if self.command_history.last().map(|s| s.as_str()) != Some(&cmd) {
                        self.command_history.push(cmd.clone());
                    }
                    self.execute_command(&cmd);
                } else {
                    self.active_view = ActiveView::Diff;
                }
            }
            (_, KeyCode::Char(c)) => {
                self.command_completions.clear();
                self.command_completion_idx = None;
                self.command_prefix.clear();
                self.command_input.push(c);
            }
            _ => {}
        }
    }

    pub fn autocomplete_next(&mut self) {
        let commands = [
            "audit",
            "c",
            "clear",
            "clear-resolved",
            "comments",
            "commit",
            "d",
            "del",
            "delete",
            "diff",
            "directives",
            "drawer",
            "e",
            "edit",
            "exit",
            "f",
            "files",
            "find",
            "h",
            "help",
            "pane",
            "panes",
            "pr",
            "prs",
            "push",
            "q",
            "quit",
            "r",
            "ref",
            "reload",
            "res",
            "resolve",
            "review",
            "risk",
            "s",
            "scan",
            "send",
            "sidebar",
            "sp",
            "split",
            "submit",
            "todo",
            "tree",
            "unresolve",
            "vsp",
            "vsplit",
            "w",
            "write",
            "wq",
            "x",
            "yank",
            "yr",
        ];

        if self.command_completions.is_empty() {
            let current = self.command_input.trim_start_matches(':').trim();
            self.command_prefix = current.to_string();
            let matches: Vec<String> = commands
                .iter()
                .filter(|&&cmd| current.is_empty() || cmd.starts_with(current))
                .map(|&s| s.to_string())
                .collect();

            if !matches.is_empty() {
                self.command_completions = matches;
                self.command_completion_idx = Some(0);
                self.command_input = self.command_completions[0].clone();
            }
        } else if let Some(idx) = self.command_completion_idx {
            let next_idx = (idx + 1) % self.command_completions.len();
            self.command_completion_idx = Some(next_idx);
            self.command_input = self.command_completions[next_idx].clone();
        }
    }

    pub fn autocomplete_prev(&mut self) {
        if !self.command_completions.is_empty() {
            if let Some(idx) = self.command_completion_idx {
                let prev_idx = if idx == 0 {
                    self.command_completions.len() - 1
                } else {
                    idx - 1
                };
                self.command_completion_idx = Some(prev_idx);
                self.command_input = self.command_completions[prev_idx].clone();
            }
        }
    }

    pub fn execute_command(&mut self, raw_cmd: &str) {
        let trimmed = raw_cmd.trim().trim_start_matches(':').trim();
        if trimmed.is_empty() {
            self.active_view = ActiveView::Diff;
            return;
        }

        // Check for line jump like :42 or :123
        if let Ok(line_num) = trimmed.parse::<u32>() {
            self.active_view = ActiveView::Diff;
            if line_num > 0 {
                self.jump_to_line(line_num);
                self.status_message = Some(format!("Jumped to line {}", line_num));
            }
            return;
        }

        let mut parts = trimmed.split_whitespace();
        let cmd_name = parts.next().unwrap_or("");
        let args: Vec<&str> = parts.collect();

        match cmd_name {
            "q" | "quit" | "q!" | "quit!" => {
                self.should_quit = true;
            }
            "w" | "write" => {
                let _ = self.comment_store.save_comments(&self.comments);
                self.status_message = Some(format!(
                    "Saved {} review directives to .lgtm/comments.jsonl",
                    self.comments.len()
                ));
                self.active_view = ActiveView::Diff;
            }
            "wq" | "x" | "exit" => {
                let _ = self.comment_store.save_comments(&self.comments);
                self.should_quit = true;
            }
            "h" | "help" | "?" => {
                self.active_view = ActiveView::Help;
            }
            "commit" | "ci" => {
                if !args.is_empty() {
                    let full_msg = args.join(" ");
                    self.commit_message_input = full_msg;
                    self.execute_commit();
                } else {
                    self.open_commit_modal();
                }
            }
            "c" | "comments" | "directives" | "todo" => {
                self.comment_list_selected_idx = 0;
                self.active_view = ActiveView::CommentList;
            }
            "prs" => {
                self.open_pr_picker();
            }
            "pr" => {
                if let Some(arg) = args.first() {
                    if let Ok(pr_num) = arg.parse::<u64>() {
                        match crate::pr::PrEngine::fetch_pr_diff(pr_num) {
                            Ok(snap) => {
                                self.current_pr_number = Some(pr_num);
                                self.snapshot = snap;
                                self.alerts =
                                    SecurityAndTestGuard::scan_snapshot(&self.snapshot.files);
                                self.selected_file_idx = 0;
                                self.cursor_line_idx = 0;
                                self.diff_scroll_offset = 0;
                                self.status_message = Some(format!(
                                    "Loaded PR #{} ({} files changed, {} alerts)",
                                    pr_num,
                                    self.snapshot.files.len(),
                                    self.alerts.len()
                                ));
                            }
                            Err(e) => {
                                self.status_message =
                                    Some(format!("Error loading PR #{}: {}", pr_num, e));
                            }
                        }
                        self.active_view = ActiveView::Diff;
                    } else {
                        self.status_message = Some("Usage: :pr <number>".to_string());
                        self.active_view = ActiveView::Diff;
                    }
                } else {
                    self.open_pr_picker();
                }
            }
            "pane" | "panes" => {
                if let Some(pane_id) = args.first() {
                    self.agent_pane = Some(pane_id.to_string());
                    self.status_message = Some(format!("Target agent pane set to {}", pane_id));
                    self.active_view = ActiveView::Diff;
                } else if let Ok(panes) = IpcBridge::list_tmux_panes() {
                    self.pane_list = panes;
                    self.pane_selected_idx = 0;
                    self.active_view = ActiveView::PanePicker;
                } else {
                    self.status_message = Some("No tmux session running".to_string());
                    self.active_view = ActiveView::Diff;
                }
            }
            "f" | "files" | "find" => {
                self.active_view = ActiveView::FilePicker;
                self.picker_query.clear();
                self.picker_selected_idx = 0;
                self.update_file_picker_filter();
            }
            "tree" | "drawer" | "sidebar" => {
                self.show_file_drawer = !self.show_file_drawer;
                self.active_view = ActiveView::Diff;
            }
            "sp" | "split" | "vsp" | "vsplit" => {
                self.is_split_view = !self.is_split_view;
                let mode = if self.is_split_view {
                    "Split View"
                } else {
                    "Unified View"
                };
                self.status_message = Some(format!("View mode: {}", mode));
                self.active_view = ActiveView::Diff;
            }
            "diff" => {
                self.active_view = ActiveView::Diff;
            }
            "r" | "review" => {
                self.export_and_send_batch_review();
                self.active_view = ActiveView::Diff;
            }
            "send" | "push" => {
                self.export_and_send_batch_review();
                self.active_view = ActiveView::Diff;
            }
            "risk" | "audit" | "scan" => {
                self.alerts = SecurityAndTestGuard::scan_snapshot(&self.snapshot.files);
                self.status_message = Some(format!(
                    "Scanned {} files — {} security & test alerts found",
                    self.snapshot.files.len(),
                    self.alerts.len()
                ));
                self.active_view = ActiveView::Diff;
            }
            "reload" | "e" | "edit" => {
                if let Some(pr_num) = self.current_pr_number {
                    if let Ok(snap) = crate::pr::PrEngine::fetch_pr_diff(pr_num) {
                        self.snapshot = snap;
                        self.alerts = SecurityAndTestGuard::scan_snapshot(&self.snapshot.files);
                        self.status_message = Some(format!("Reloaded PR #{}", pr_num));
                    }
                } else if self.git_engine.is_git_repo() {
                    if let Ok(snap) = self.git_engine.compute_diff_snapshot() {
                        self.snapshot = snap;
                        self.alerts = SecurityAndTestGuard::scan_snapshot(&self.snapshot.files);
                        self.status_message = Some(format!(
                            "Reloaded diff ({} files)",
                            self.snapshot.files.len()
                        ));
                    }
                }
                self.active_view = ActiveView::Diff;
            }
            "clear" => {
                self.comments.clear();
                let _ = self.comment_store.save_comments(&self.comments);
                self.status_message = Some("Cleared all directives".to_string());
                self.active_view = ActiveView::Diff;
            }
            "clear-resolved" => {
                let initial = self.comments.len();
                self.comments.retain(|c| !c.is_resolved);
                let _ = self.comment_store.save_comments(&self.comments);
                self.status_message = Some(format!(
                    "Removed {} resolved directives",
                    initial - self.comments.len()
                ));
                self.active_view = ActiveView::Diff;
            }
            "resolve" | "res" => {
                if args.first() == Some(&"all") {
                    for c in &mut self.comments {
                        c.is_resolved = true;
                    }
                    let _ = self.comment_store.save_comments(&self.comments);
                    self.status_message =
                        Some(format!("Resolved all {} directives", self.comments.len()));
                } else {
                    self.toggle_resolve_current_comment();
                }
                self.active_view = ActiveView::Diff;
            }
            "unresolve" => {
                if args.first() == Some(&"all") {
                    for c in &mut self.comments {
                        c.is_resolved = false;
                    }
                    let _ = self.comment_store.save_comments(&self.comments);
                    self.status_message = Some(format!(
                        "Marked all {} directives active",
                        self.comments.len()
                    ));
                } else {
                    self.toggle_resolve_current_comment();
                }
                self.active_view = ActiveView::Diff;
            }
            "y" | "yank" => {
                self.yank_current_code_line();
                self.active_view = ActiveView::Diff;
            }
            "yr" | "ref" => {
                self.yank_reference_to_clipboard();
                self.active_view = ActiveView::Diff;
            }
            _ => {
                self.status_message = Some(format!("Unknown command: :{}", cmd_name));
                self.active_view = ActiveView::Diff;
            }
        }
    }

    pub fn jump_to_line(&mut self, target_line_no: u32) {
        if let Some(f) = self.snapshot.files.get(self.selected_file_idx) {
            let mut line_counter = 0;
            for hunk in &f.hunks {
                for line in &hunk.lines {
                    let line_num = line.new_line_num.or(line.old_line_num);
                    if line_num == Some(target_line_no) {
                        self.cursor_line_idx = line_counter;
                        return;
                    }
                    line_counter += 1;
                }
            }
            if line_counter > 0 {
                self.cursor_line_idx = line_counter - 1;
            }
        }
    }

    fn handle_comment_list_key(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) | (_, KeyCode::Char('q')) => {
                self.active_view = ActiveView::Diff;
            }
            (_, KeyCode::Enter) => {
                if let Some(comment) = self.comments.get(self.comment_list_selected_idx) {
                    let file_path = comment.file_path.clone();
                    let target_line = comment.target_line_no;
                    self.jump_to_file_and_line(&file_path, target_line);
                }
                self.active_view = ActiveView::Diff;
            }
            (_, KeyCode::Up) | (_, KeyCode::Char('k')) => {
                if self.comment_list_selected_idx > 0 {
                    self.comment_list_selected_idx -= 1;
                }
            }
            (_, KeyCode::Down) | (_, KeyCode::Char('j')) => {
                if !self.comments.is_empty()
                    && self.comment_list_selected_idx < self.comments.len() - 1
                {
                    self.comment_list_selected_idx += 1;
                }
            }
            (_, KeyCode::PageDown)
            | (KeyModifiers::CONTROL, KeyCode::Char('d'))
            | (KeyModifiers::CONTROL, KeyCode::Char('f')) => {
                if !self.comments.is_empty() {
                    self.comment_list_selected_idx =
                        (self.comment_list_selected_idx + 5).min(self.comments.len() - 1);
                }
            }
            (_, KeyCode::PageUp)
            | (KeyModifiers::CONTROL, KeyCode::Char('u'))
            | (KeyModifiers::CONTROL, KeyCode::Char('b')) => {
                self.comment_list_selected_idx = self.comment_list_selected_idx.saturating_sub(5);
            }
            (_, KeyCode::Char('r')) => {
                if !self.comments.is_empty() && self.comment_list_selected_idx < self.comments.len()
                {
                    self.start_reply_to_comment(self.comment_list_selected_idx);
                }
            }
            (_, KeyCode::Char('v')) | (_, KeyCode::Char(' ')) | (_, KeyCode::Char('R')) => {
                if !self.comments.is_empty() && self.comment_list_selected_idx < self.comments.len()
                {
                    let is_res = self.comments[self.comment_list_selected_idx].toggle_resolved();
                    let _ = self.comment_store.save_comments(&self.comments);
                    self.status_message = Some(if is_res {
                        "✅ Directive marked resolved".to_string()
                    } else {
                        "💬 Directive marked active".to_string()
                    });
                }
            }
            (_, KeyCode::Char('a')) => {
                if !self.comments.is_empty() && self.comment_list_selected_idx < self.comments.len()
                {
                    self.action_comment_to_agent(self.comment_list_selected_idx);
                    self.active_view = ActiveView::Diff;
                }
            }
            (_, KeyCode::Char('d')) | (_, KeyCode::Char('x')) | (_, KeyCode::Delete) => {
                if !self.comments.is_empty() && self.comment_list_selected_idx < self.comments.len()
                {
                    let removed = self.comments.remove(self.comment_list_selected_idx);
                    let _ = self.comment_store.save_comments(&self.comments);
                    if self.comment_list_selected_idx >= self.comments.len()
                        && !self.comments.is_empty()
                    {
                        self.comment_list_selected_idx = self.comments.len() - 1;
                    }
                    self.status_message = Some(format!(
                        "Deleted comment at {}:{}",
                        removed.file_path, removed.target_line_no
                    ));
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
                self.export_and_send_batch_review();
                self.active_view = ActiveView::Diff;
            }
            _ => {}
        }
    }

    pub fn open_pr_picker(&mut self) {
        match crate::pr::PrEngine::list_pull_requests() {
            Ok(prs) => {
                self.pr_list = prs;
                self.pr_selected_idx = 0;
                self.active_view = ActiveView::PrPicker;
            }
            Err(e) => {
                self.pr_list.clear();
                self.status_message = Some(format!("`gh` PR list failed: {}", e));
                self.active_view = ActiveView::PrPicker;
            }
        }
    }

    fn handle_pr_picker_key(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) | (_, KeyCode::Char('q')) => {
                self.active_view = ActiveView::Diff;
            }
            (_, KeyCode::Up) | (_, KeyCode::Char('k')) => {
                if self.pr_selected_idx > 0 {
                    self.pr_selected_idx -= 1;
                }
            }
            (_, KeyCode::Down) | (_, KeyCode::Char('j')) => {
                if !self.pr_list.is_empty() && self.pr_selected_idx < self.pr_list.len() - 1 {
                    self.pr_selected_idx += 1;
                }
            }
            (_, KeyCode::PageDown)
            | (KeyModifiers::CONTROL, KeyCode::Char('d'))
            | (KeyModifiers::CONTROL, KeyCode::Char('f')) => {
                if !self.pr_list.is_empty() {
                    self.pr_selected_idx = (self.pr_selected_idx + 5).min(self.pr_list.len() - 1);
                }
            }
            (_, KeyCode::PageUp)
            | (KeyModifiers::CONTROL, KeyCode::Char('u'))
            | (KeyModifiers::CONTROL, KeyCode::Char('b')) => {
                self.pr_selected_idx = self.pr_selected_idx.saturating_sub(5);
            }
            (_, KeyCode::Enter) => {
                if let Some(pr) = self.pr_list.get(self.pr_selected_idx) {
                    let pr_num = pr.number;
                    match crate::pr::PrEngine::fetch_pr_diff(pr_num) {
                        Ok(snap) => {
                            self.current_pr_number = Some(pr_num);
                            self.alerts = SecurityAndTestGuard::scan_snapshot(&snap.files);
                            self.snapshot = snap;
                            self.selected_file_idx = 0;
                            self.cursor_line_idx = 0;
                            self.status_message =
                                Some(format!("Loaded PR #{}: {}", pr_num, pr.title));
                        }
                        Err(e) => {
                            self.status_message =
                                Some(format!("Failed to fetch PR #{}: {}", pr_num, e));
                        }
                    }
                }
                self.active_view = ActiveView::Diff;
            }
            _ => {}
        }
    }

    pub fn has_comment_on_cursor(&self) -> bool {
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let path_str = file.display_name();
            let mut line_counter = 0;
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    if line_counter == self.cursor_line_idx {
                        if let Some(lno) = line.new_line_num.or(line.old_line_num) {
                            return self
                                .comments
                                .iter()
                                .any(|c| c.file_path == path_str && c.target_line_no == lno);
                        }
                    }
                    line_counter += 1;
                }
            }
        }
        false
    }

    pub fn delete_current_line_comment(&mut self) {
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let path_str = file.display_name();
            let mut line_counter = 0;
            let mut target_line_no = None;
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    if line_counter == self.cursor_line_idx {
                        target_line_no = line.new_line_num.or(line.old_line_num);
                        break;
                    }
                    line_counter += 1;
                }
                if target_line_no.is_some() {
                    break;
                }
            }

            if let Some(lno) = target_line_no {
                let initial_len = self.comments.len();
                self.comments
                    .retain(|c| !(c.file_path == path_str && c.target_line_no == lno));
                if self.comments.len() < initial_len {
                    let _ = self.comment_store.save_comments(&self.comments);
                    self.status_message = Some(format!("Deleted comment on {}:{}", path_str, lno));
                } else {
                    self.status_message = Some(format!("No comment on {}:{}", path_str, lno));
                }
            }
        }
    }

    pub fn jump_to_file_and_line(&mut self, file_path: &str, target_line: u32) {
        if let Some(idx) = self
            .snapshot
            .files
            .iter()
            .position(|f| f.display_name() == file_path)
        {
            self.selected_file_idx = idx;
            if let Some(f) = self.snapshot.files.get_mut(idx) {
                f.is_folded = false;
            }
            self.cursor_line_idx = 0;
            let mut line_counter = 0;
            if let Some(f) = self.snapshot.files.get(idx) {
                for hunk in &f.hunks {
                    for line in &hunk.lines {
                        let line_num = line.new_line_num.or(line.old_line_num);
                        if line_num == Some(target_line) {
                            self.cursor_line_idx = line_counter;
                            self.diff_scroll_offset = line_counter.saturating_sub(10);
                            return;
                        }
                        line_counter += 1;
                    }
                }
            }
        }
    }

    pub fn comment_idx_at_cursor(&self) -> Option<usize> {
        let file = self.snapshot.files.get(self.selected_file_idx)?;
        let path_str = file.display_name();
        let mut line_counter = 0;
        let mut target_line_no = None;
        for hunk in &file.hunks {
            for line in &hunk.lines {
                if line_counter == self.cursor_line_idx {
                    target_line_no = line.new_line_num.or(line.old_line_num);
                    break;
                }
                line_counter += 1;
            }
            if target_line_no.is_some() {
                break;
            }
        }
        let lno = target_line_no?;
        self.comments
            .iter()
            .position(|c| c.file_path == path_str && c.target_line_no == lno)
    }

    pub fn toggle_resolve_current_comment(&mut self) {
        if let Some(idx) = self.comment_idx_at_cursor() {
            let is_res = self.comments[idx].toggle_resolved();
            let _ = self.comment_store.save_comments(&self.comments);
            self.status_message = Some(if is_res {
                "✅ Directive marked resolved".to_string()
            } else {
                "💬 Directive marked active".to_string()
            });
        } else {
            self.status_message = Some("No directive on current line".to_string());
        }
    }

    pub fn start_reply_to_comment(&mut self, idx: usize) {
        if idx < self.comments.len() {
            self.comment_reply_target_idx = Some(idx);
            self.comment_reply_input.clear();
            self.active_view = ActiveView::CommentReply;
        }
    }

    pub fn action_comment_to_agent(&mut self, idx: usize) {
        if let Some(comment) = self.comments.get(idx) {
            let mut payload = format!(
                "#ref {}:{} DIRECTIVE: {}",
                comment.file_path, comment.target_line_no, comment.comment_text
            );
            for reply in &comment.replies {
                payload.push_str(&format!("\n↳ Reply: {}", reply));
            }
            if let Some(ref pane) = self.agent_pane {
                if let Err(e) = IpcBridge::send_to_tmux(pane, &payload) {
                    self.status_message = Some(format!("Send failed: {}", e));
                    IpcBridge::send_osc52_clipboard(&payload);
                } else {
                    self.status_message =
                        Some(format!("Sent to pane {}: {}", pane, comment.file_path));
                }
            } else {
                IpcBridge::send_osc52_clipboard(&payload);
                self.status_message = Some(format!(
                    "Copied directive to clipboard: {}:{}",
                    comment.file_path, comment.target_line_no
                ));
            }
        }
    }

    fn handle_comment_reply_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.comment_reply_input.clear();
                self.comment_reply_target_idx = None;
                self.active_view = ActiveView::Diff;
            }
            KeyCode::Enter => {
                if !self.comment_reply_input.trim().is_empty() {
                    if let Some(idx) = self.comment_reply_target_idx {
                        if idx < self.comments.len() {
                            self.comments[idx].add_reply(self.comment_reply_input.trim());
                            let _ = self.comment_store.save_comments(&self.comments);
                            self.status_message = Some(format!(
                                "Reply added to comment on {}:{}",
                                self.comments[idx].file_path, self.comments[idx].target_line_no
                            ));
                        }
                    }
                }
                self.comment_reply_input.clear();
                self.comment_reply_target_idx = None;
                self.active_view = ActiveView::Diff;
            }
            KeyCode::Backspace => {
                self.comment_reply_input.pop();
            }
            KeyCode::Char(c) => {
                self.comment_reply_input.push(c);
            }
            _ => {}
        }
    }

    fn handle_comment_input_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.active_view = ActiveView::Diff;
            }
            KeyCode::Enter => {
                if !self.comment_input.trim().is_empty() {
                    self.save_current_line_comment();
                }
                self.active_view = ActiveView::Diff;
            }
            KeyCode::Backspace => {
                self.comment_input.pop();
            }
            KeyCode::Char(c) => {
                self.comment_input.push(c);
            }
            _ => {}
        }
    }

    fn handle_help_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') | KeyCode::Enter => {
                self.active_view = ActiveView::Diff;
            }
            _ => {}
        }
    }

    fn handle_file_picker_key(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                self.active_view = ActiveView::Diff;
            }
            (_, KeyCode::Enter) => {
                if let Some(&file_idx) = self.picker_filtered_indices.get(self.picker_selected_idx)
                {
                    self.selected_file_idx = file_idx;
                    self.diff_scroll_offset = 0;
                    self.cursor_line_idx = 0;
                }
                self.active_view = ActiveView::Diff;
            }
            (_, KeyCode::Up) | (KeyModifiers::CONTROL, KeyCode::Char('k')) => {
                if self.picker_selected_idx > 0 {
                    self.picker_selected_idx -= 1;
                }
            }
            (_, KeyCode::Down) | (KeyModifiers::CONTROL, KeyCode::Char('j')) => {
                if !self.picker_filtered_indices.is_empty()
                    && self.picker_selected_idx < self.picker_filtered_indices.len() - 1
                {
                    self.picker_selected_idx += 1;
                }
            }
            (_, KeyCode::PageDown)
            | (KeyModifiers::CONTROL, KeyCode::Char('d'))
            | (KeyModifiers::CONTROL, KeyCode::Char('f')) => {
                if !self.picker_filtered_indices.is_empty() {
                    self.picker_selected_idx =
                        (self.picker_selected_idx + 5).min(self.picker_filtered_indices.len() - 1);
                }
            }
            (_, KeyCode::PageUp)
            | (KeyModifiers::CONTROL, KeyCode::Char('u'))
            | (KeyModifiers::CONTROL, KeyCode::Char('b')) => {
                self.picker_selected_idx = self.picker_selected_idx.saturating_sub(5);
            }
            (_, KeyCode::Backspace) => {
                self.picker_query.pop();
                self.update_file_picker_filter();
            }
            (_, KeyCode::Char(c)) => {
                self.picker_query.push(c);
                self.update_file_picker_filter();
            }
            _ => {}
        }
    }

    fn handle_pane_picker_key(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                self.active_view = ActiveView::Diff;
            }
            (_, KeyCode::Enter) => {
                if let Some(pane) = self.pane_list.get(self.pane_selected_idx) {
                    self.agent_pane = Some(pane.id.clone());
                    self.status_message = Some(format!("Target pane set to {}", pane.id));
                }
                self.active_view = ActiveView::Diff;
            }
            (_, KeyCode::Up) | (_, KeyCode::Char('k')) => {
                if self.pane_selected_idx > 0 {
                    self.pane_selected_idx -= 1;
                }
            }
            (_, KeyCode::Down) | (_, KeyCode::Char('j')) => {
                if !self.pane_list.is_empty() && self.pane_selected_idx < self.pane_list.len() - 1 {
                    self.pane_selected_idx += 1;
                }
            }
            (_, KeyCode::PageDown)
            | (KeyModifiers::CONTROL, KeyCode::Char('d'))
            | (KeyModifiers::CONTROL, KeyCode::Char('f')) => {
                if !self.pane_list.is_empty() {
                    self.pane_selected_idx =
                        (self.pane_selected_idx + 5).min(self.pane_list.len() - 1);
                }
            }
            (_, KeyCode::PageUp)
            | (KeyModifiers::CONTROL, KeyCode::Char('u'))
            | (KeyModifiers::CONTROL, KeyCode::Char('b')) => {
                self.pane_selected_idx = self.pane_selected_idx.saturating_sub(5);
            }
            _ => {}
        }
    }

    fn update_file_picker_filter(&mut self) {
        let matcher = SkimMatcherV2::default();
        if self.picker_query.is_empty() {
            self.picker_filtered_indices = (0..self.snapshot.files.len()).collect();
        } else {
            let mut scored: Vec<(i64, usize)> = self
                .snapshot
                .files
                .iter()
                .enumerate()
                .filter_map(|(idx, file)| {
                    matcher
                        .fuzzy_match(&file.display_name(), &self.picker_query)
                        .map(|score| (score, idx))
                })
                .collect();
            scored.sort_by_key(|a| std::cmp::Reverse(a.0));
            self.picker_filtered_indices = scored.into_iter().map(|(_, idx)| idx).collect();
        }

        if self.picker_selected_idx >= self.picker_filtered_indices.len() {
            self.picker_selected_idx = 0;
        }
    }

    pub fn next_file(&mut self) {
        if !self.snapshot.files.is_empty() && self.selected_file_idx < self.snapshot.files.len() - 1
        {
            self.selected_file_idx += 1;
            self.diff_scroll_offset = 0;
            self.cursor_line_idx = 0;
        }
    }

    pub fn prev_file(&mut self) {
        if self.selected_file_idx > 0 {
            self.selected_file_idx -= 1;
            self.diff_scroll_offset = 0;
            self.cursor_line_idx = 0;
        }
    }

    pub fn next_alert(&mut self) {
        if self.alerts.is_empty() {
            self.status_message = Some("No warnings detected".to_string());
            return;
        }
        self.selected_alert_idx = (self.selected_alert_idx + 1) % self.alerts.len();
        self.jump_to_selected_alert();
    }

    pub fn prev_alert(&mut self) {
        if self.alerts.is_empty() {
            self.status_message = Some("No warnings detected".to_string());
            return;
        }
        if self.selected_alert_idx == 0 {
            self.selected_alert_idx = self.alerts.len() - 1;
        } else {
            self.selected_alert_idx -= 1;
        }
        self.jump_to_selected_alert();
    }

    fn jump_to_selected_alert(&mut self) {
        let alert = &self.alerts[self.selected_alert_idx];
        let badge = alert.severity.badge();
        let title = alert.title.clone();
        let file_path = alert.file_path.clone();
        let target_line = alert.line_no;

        if let Some(idx) = self
            .snapshot
            .files
            .iter()
            .position(|f| f.display_name() == file_path)
        {
            self.selected_file_idx = idx;
            self.cursor_line_idx = 0;
            if let Some(lno) = target_line {
                let mut line_counter = 0;
                if let Some(f) = self.snapshot.files.get(idx) {
                    for hunk in &f.hunks {
                        for line in &hunk.lines {
                            let line_num = line.new_line_num.or(line.old_line_num);
                            if line_num == Some(lno) {
                                self.cursor_line_idx = line_counter;
                                break;
                            }
                            line_counter += 1;
                        }
                    }
                }
            }
        }
        self.status_message = Some(format!("{}: {}", badge, title));
    }

    pub fn cursor_down(&mut self) {
        self.cursor_line_idx += 1;
    }

    pub fn cursor_up(&mut self) {
        if self.cursor_line_idx > 0 {
            self.cursor_line_idx -= 1;
        }
    }

    pub fn toggle_review_current_file(&mut self) {
        if let Some(file) = self.snapshot.files.get_mut(self.selected_file_idx) {
            file.is_reviewed = !file.is_reviewed;
            if file.is_reviewed {
                self.next_file();
            }
        }
    }

    pub fn save_current_line_comment(&mut self) {
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let path_str = file.display_name();
            let mut target_line_no = 1;
            let mut target_code = String::new();
            let mut line_counter = 0;

            for hunk in &file.hunks {
                for line in &hunk.lines {
                    if line_counter == self.cursor_line_idx {
                        target_line_no = line.new_line_num.or(line.old_line_num).unwrap_or(1);
                        target_code = line.content.clone();
                        break;
                    }
                    line_counter += 1;
                }
            }

            let turn_state = self.turn_manager.load_state();
            let anchor = CommentAnchor {
                file_path: path_str,
                target_line_no,
                target_code,
                context_before: Vec::new(),
                context_after: Vec::new(),
                comment_text: self.comment_input.clone(),
                replies: Vec::new(),
                created_at_turn: turn_state.current_turn,
                is_resolved: false,
                pr_number: self.current_pr_number,
            };

            let _ = self.comment_store.add_comment(&anchor);
            self.comments.push(anchor);
            if let Some(pr_num) = self.current_pr_number {
                self.status_message = Some(format!("Comment added to PR #{} review", pr_num));
            } else {
                self.status_message = Some("Comment added to .lgtm/comments.jsonl".to_string());
            }
        }
    }

    pub fn export_and_send_batch_review(&mut self) {
        if let Some(pr_num) = self.current_pr_number {
            let (prompt, file_path) =
                CommentStore::compile_pr_review_prompt(&self.comments, pr_num);
            let full_path = self.git_engine.repo_path.join(&file_path);
            let _ = std::fs::write(&full_path, &prompt);

            let mut notes = vec![format!("Saved to {}", file_path.display())];

            if crate::pr::PrEngine::is_gh_installed() {
                match crate::pr::PrEngine::post_review(pr_num, "COMMENT", &prompt) {
                    Ok(_) => notes.push("Posted review to PR".to_string()),
                    Err(e) => notes.push(format!("gh: {}", e)),
                }
            }

            if let Some(ref pane) = self.agent_pane {
                let inject_msg = format!("Please review directives in {}", file_path.display());
                let _ = IpcBridge::send_to_tmux(pane, &inject_msg);
                notes.push(format!("Notified pane {}", pane));
            } else {
                IpcBridge::send_osc52_clipboard(&prompt);
                notes.push("Copied prompt to clipboard".to_string());
            }

            self.status_message = Some(notes.join(" | "));
            return;
        }

        let turn = self.turn_manager.load_state().current_turn;
        let (prompt, file_path) = CommentStore::compile_batch_review_prompt(&self.comments, turn);

        let full_path = self.git_engine.repo_path.join(&file_path);
        let _ = std::fs::write(&full_path, &prompt);

        if let Some(ref pane) = self.agent_pane {
            let inject_msg = format!("Please review directives in {}", file_path.display());
            let _ = IpcBridge::send_to_tmux(pane, &inject_msg);
            self.status_message = Some(format!(
                "Exported {} and notified pane {}",
                file_path.display(),
                pane
            ));
        } else {
            IpcBridge::send_osc52_clipboard(&prompt);
            self.status_message = Some(format!(
                "Exported {} and copied prompt to clipboard",
                file_path.display()
            ));
        }

        let _ = self.turn_manager.advance_turn(None);
    }

    pub fn run_test_mutation_probe(&mut self) {
        self.status_message =
            Some("Running test validity probe (checking for vacuous assertions)...".to_string());
    }

    pub fn inject_reference_to_agent(&mut self) {
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let path_str = file.display_name();

            let mut target_line: Option<u32> = None;
            let mut line_counter = 0;

            for hunk in &file.hunks {
                for line in &hunk.lines {
                    if line_counter == self.cursor_line_idx {
                        target_line = line.new_line_num.or(line.old_line_num);
                        break;
                    }
                    line_counter += 1;
                }
                if target_line.is_some() {
                    break;
                }
            }

            let payload = match target_line {
                Some(line_no) => format!("#ref {}:{}", path_str, line_no),
                None => format!("#ref {}", path_str),
            };

            if let Some(ref pane) = self.agent_pane {
                if let Err(e) = IpcBridge::send_to_tmux(pane, &payload) {
                    self.status_message = Some(format!("Send failed: {}", e));
                    IpcBridge::send_osc52_clipboard(&payload);
                } else {
                    self.status_message = Some(format!("Sent to pane {}: {}", pane, payload));
                }
            } else {
                IpcBridge::send_osc52_clipboard(&payload);
                self.status_message = Some(format!("Copied to clipboard: {}", payload));
            }
        }
    }

    pub fn current_file_line_count(&self) -> usize {
        self.snapshot
            .files
            .get(self.selected_file_idx)
            .map(|f| f.hunks.iter().map(|h| h.lines.len()).sum())
            .unwrap_or(0)
    }

    pub fn cursor_to_top(&mut self) {
        self.cursor_line_idx = 0;
        self.diff_scroll_offset = 0;
    }

    pub fn cursor_to_bottom(&mut self) {
        let total = self.current_file_line_count();
        if total > 0 {
            self.cursor_line_idx = total - 1;
        }
    }

    pub fn page_down(&mut self, amount: usize) {
        let total = self.current_file_line_count();
        if total > 0 {
            self.cursor_line_idx = (self.cursor_line_idx + amount).min(total - 1);
        }
    }

    pub fn page_up(&mut self, amount: usize) {
        if self.cursor_line_idx >= amount {
            self.cursor_line_idx -= amount;
        } else {
            self.cursor_line_idx = 0;
        }
    }

    pub fn center_cursor(&mut self) {
        self.diff_scroll_offset = self.cursor_line_idx.saturating_sub(15);
    }

    pub fn cursor_to_top_of_view(&mut self) {
        self.diff_scroll_offset = self.cursor_line_idx;
    }

    pub fn cursor_to_bottom_of_view(&mut self) {
        self.diff_scroll_offset = self.cursor_line_idx.saturating_sub(30);
    }

    pub fn next_unreviewed_file(&mut self) {
        if self.snapshot.files.is_empty() {
            return;
        }
        for i in (self.selected_file_idx + 1)..self.snapshot.files.len() {
            if !self.snapshot.files[i].is_reviewed {
                self.selected_file_idx = i;
                self.diff_scroll_offset = 0;
                self.cursor_line_idx = 0;
                return;
            }
        }
        for i in 0..self.selected_file_idx {
            if !self.snapshot.files[i].is_reviewed {
                self.selected_file_idx = i;
                self.diff_scroll_offset = 0;
                self.cursor_line_idx = 0;
                return;
            }
        }
        self.status_message = Some("All files marked reviewed! 🎉".to_string());
    }

    pub fn prev_unreviewed_file(&mut self) {
        if self.snapshot.files.is_empty() {
            return;
        }
        for i in (0..self.selected_file_idx).rev() {
            if !self.snapshot.files[i].is_reviewed {
                self.selected_file_idx = i;
                self.diff_scroll_offset = 0;
                self.cursor_line_idx = 0;
                return;
            }
        }
        for i in ((self.selected_file_idx + 1)..self.snapshot.files.len()).rev() {
            if !self.snapshot.files[i].is_reviewed {
                self.selected_file_idx = i;
                self.diff_scroll_offset = 0;
                self.cursor_line_idx = 0;
                return;
            }
        }
        self.status_message = Some("All files marked reviewed! 🎉".to_string());
    }

    pub fn open_in_editor(&mut self) {
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let path = self.git_engine.repo_path.join(&file.new_path);
            let mut line_no = 1;
            let mut line_counter = 0;
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    if line_counter == self.cursor_line_idx {
                        line_no = line.new_line_num.or(line.old_line_num).unwrap_or(1);
                        break;
                    }
                    line_counter += 1;
                }
            }

            let editor = std::env::var("EDITOR")
                .or_else(|_| std::env::var("VISUAL"))
                .unwrap_or_else(|_| "nvim".to_string());

            let _ = std::process::Command::new(&editor)
                .arg(format!("+{}", line_no))
                .arg(&path)
                .status();

            self.status_message = Some(format!(
                "Opened {}:{} in {}",
                file.display_name(),
                line_no,
                editor
            ));
        }
    }

    pub fn toggle_stage_current_file(&mut self) {
        if !self.git_engine.is_git_repo() {
            self.status_message = Some("Not a git repository".to_string());
            return;
        }
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let path = file.display_name();
            match self.git_engine.stage_file(std::path::Path::new(&path)) {
                Ok(_) => {
                    self.status_message = Some(format!("Staged {}", path));
                    self.reload_diff();
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to stage {}: {}", path, e));
                }
            }
        }
    }

    pub fn discard_current_file_changes(&mut self) {
        if !self.git_engine.is_git_repo() {
            return;
        }
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let path = file.display_name();
            let res = std::process::Command::new("git")
                .current_dir(&self.git_engine.repo_path)
                .args(["restore", &path])
                .status();

            match res {
                Ok(s) if s.success() => {
                    self.status_message = Some(format!("Discarded changes in {}", path));
                    self.reload_diff();
                }
                _ => {
                    self.status_message = Some(format!("Failed to restore {}", path));
                }
            }
        }
    }

    pub fn current_hunk_info(&self) -> Option<(&FileDiff, &DiffHunk, usize)> {
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let mut line_offset = 0;
            for (h_idx, hunk) in file.hunks.iter().enumerate() {
                let hunk_len = hunk.lines.len();
                if self.cursor_line_idx >= line_offset
                    && self.cursor_line_idx < line_offset + hunk_len
                {
                    return Some((file, hunk, h_idx));
                }
                line_offset += hunk_len;
            }
            if let Some(last_hunk) = file.hunks.last() {
                return Some((file, last_hunk, file.hunks.len().saturating_sub(1)));
            }
        }
        None
    }

    pub fn stage_current_hunk(&mut self) {
        if !self.git_engine.is_git_repo() {
            self.status_message = Some("Not a git repository".to_string());
            return;
        }
        if let Some((file, hunk, h_idx)) = self.current_hunk_info() {
            let file_clone = file.clone();
            let hunk_clone = hunk.clone();
            let file_name = file_clone.display_name();
            match self.git_engine.stage_hunk(&file_clone, &hunk_clone) {
                Ok(_) => {
                    self.status_message =
                        Some(format!("✅ Staged hunk {} in {}", h_idx + 1, file_name));
                    self.reload_diff();
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to stage hunk: {}", e));
                }
            }
        }
    }

    pub fn unstage_current_hunk(&mut self) {
        if !self.git_engine.is_git_repo() {
            self.status_message = Some("Not a git repository".to_string());
            return;
        }
        if let Some((file, hunk, h_idx)) = self.current_hunk_info() {
            let file_clone = file.clone();
            let hunk_clone = hunk.clone();
            let file_name = file_clone.display_name();
            match self.git_engine.unstage_hunk(&file_clone, &hunk_clone) {
                Ok(_) => {
                    self.status_message =
                        Some(format!("↩️ Unstaged hunk {} in {}", h_idx + 1, file_name));
                    self.reload_diff();
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to unstage hunk: {}", e));
                }
            }
        }
    }

    pub fn discard_current_hunk(&mut self) {
        if !self.git_engine.is_git_repo() {
            self.status_message = Some("Not a git repository".to_string());
            return;
        }
        if let Some((file, hunk, h_idx)) = self.current_hunk_info() {
            let file_clone = file.clone();
            let hunk_clone = hunk.clone();
            let file_name = file_clone.display_name();
            match self.git_engine.discard_hunk(&file_clone, &hunk_clone) {
                Ok(_) => {
                    self.status_message =
                        Some(format!("🗑️ Discarded hunk {} in {}", h_idx + 1, file_name));
                    self.reload_diff();
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to discard hunk: {}", e));
                }
            }
        }
    }

    pub fn unstage_current_file(&mut self) {
        if !self.git_engine.is_git_repo() {
            self.status_message = Some("Not a git repository".to_string());
            return;
        }
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let path = file.display_name();
            match self.git_engine.unstage_file(std::path::Path::new(&path)) {
                Ok(_) => {
                    self.status_message = Some(format!("↩️ Unstaged file {}", path));
                    self.reload_diff();
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to unstage file {}: {}", path, e));
                }
            }
        }
    }

    pub fn fold_current_hunk(&mut self) {
        if let Some(file) = self.snapshot.files.get_mut(self.selected_file_idx) {
            let mut line_offset = 0;
            for hunk in file.hunks.iter_mut() {
                let hunk_len = hunk.lines.len();
                if self.cursor_line_idx >= line_offset
                    && self.cursor_line_idx < line_offset + hunk_len
                {
                    hunk.is_folded = true;
                    self.status_message = Some("Folded hunk".to_string());
                    return;
                }
                line_offset += hunk_len;
            }
            if let Some(last_hunk) = file.hunks.last_mut() {
                last_hunk.is_folded = true;
                self.status_message = Some("Folded hunk".to_string());
            }
        }
    }

    pub fn unfold_current_hunk(&mut self) {
        if let Some(file) = self.snapshot.files.get_mut(self.selected_file_idx) {
            let mut line_offset = 0;
            for hunk in file.hunks.iter_mut() {
                let hunk_len = hunk.lines.len();
                if self.cursor_line_idx >= line_offset
                    && self.cursor_line_idx < line_offset + hunk_len
                {
                    hunk.is_folded = false;
                    self.status_message = Some("Unfolded hunk".to_string());
                    return;
                }
                line_offset += hunk_len;
            }
            if let Some(last_hunk) = file.hunks.last_mut() {
                last_hunk.is_folded = false;
                self.status_message = Some("Unfolded hunk".to_string());
            }
        }
    }

    pub fn fold_all_hunks(&mut self) {
        for file in &mut self.snapshot.files {
            for hunk in &mut file.hunks {
                hunk.is_folded = true;
            }
        }
        self.status_message = Some("Folded all hunks in diff".to_string());
    }

    pub fn unfold_all_hunks(&mut self) {
        for file in &mut self.snapshot.files {
            for hunk in &mut file.hunks {
                hunk.is_folded = false;
            }
        }
        self.status_message = Some("Unfolded all hunks in diff".to_string());
    }

    pub fn staged_files_count(&self) -> usize {
        if let Ok(repo) = self.git_engine.open_repo() {
            if let Ok(statuses) = repo.statuses(None) {
                return statuses
                    .iter()
                    .filter(|s| {
                        let st = s.status();
                        st.is_index_new()
                            || st.is_index_modified()
                            || st.is_index_deleted()
                            || st.is_index_renamed()
                            || st.is_index_typechange()
                    })
                    .count();
            }
        }
        0
    }

    pub fn open_commit_modal(&mut self) {
        let staged_count = self.staged_files_count();
        if staged_count == 0 {
            self.status_message =
                Some("⚠️ No staged changes to commit (stage with 's' or 'S')".to_string());
            return;
        }
        self.commit_message_input.clear();
        self.active_view = ActiveView::CommitModal;
    }

    pub fn execute_commit(&mut self) {
        let msg = self.commit_message_input.trim().to_string();
        if msg.is_empty() {
            self.status_message = Some("⚠️ Commit message cannot be empty".to_string());
            return;
        }
        match self.git_engine.commit(&msg) {
            Ok(oid) => {
                let short_oid = &oid.to_string()[..7.min(oid.to_string().len())];
                self.status_message = Some(format!("🚀 Committed [{}] {}", short_oid, msg));
                self.commit_message_input.clear();
                self.active_view = ActiveView::Diff;
                self.reload_diff();
            }
            Err(e) => {
                self.status_message = Some(format!("Commit failed: {}", e));
            }
        }
    }

    fn handle_commit_modal_key(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('c')) | (_, KeyCode::Esc) => {
                self.active_view = ActiveView::Diff;
            }
            (KeyModifiers::CONTROL, KeyCode::Char('s')) | (_, KeyCode::Enter) => {
                self.execute_commit();
            }
            (_, KeyCode::Backspace) => {
                self.commit_message_input.pop();
            }
            (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
                self.commit_message_input.clear();
            }
            (_, KeyCode::Char(c)) => {
                self.commit_message_input.push(c);
            }
            _ => {}
        }
    }

    pub fn toggle_visual_mode(&mut self) {
        self.visual_mode = !self.visual_mode;
        if self.visual_mode {
            self.visual_anchor_idx = Some(self.cursor_line_idx);
            self.status_message = Some(
                "-- VISUAL LINE SELECTION (s: stage, u: unstage, x: discard, Esc: cancel) --"
                    .to_string(),
            );
        } else {
            self.visual_anchor_idx = None;
            self.status_message = Some("Visual mode exited".to_string());
        }
    }

    pub fn is_in_visual_selection(&self, line_idx: usize) -> bool {
        if !self.visual_mode {
            return false;
        }
        if let Some(anchor) = self.visual_anchor_idx {
            let min = anchor.min(self.cursor_line_idx);
            let max = anchor.max(self.cursor_line_idx);
            line_idx >= min && line_idx <= max
        } else {
            false
        }
    }

    pub fn selected_line_indices_in_current_hunk(
        &self,
    ) -> Option<(&FileDiff, &DiffHunk, Vec<usize>)> {
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let anchor = self.visual_anchor_idx.unwrap_or(self.cursor_line_idx);
            let sel_min = anchor.min(self.cursor_line_idx);
            let sel_max = anchor.max(self.cursor_line_idx);

            let mut line_offset = 0;
            for hunk in &file.hunks {
                let hunk_len = hunk.lines.len();
                let hunk_end = line_offset + hunk_len;

                // Check overlap
                if sel_max >= line_offset && sel_min < hunk_end {
                    let mut hunk_selected_indices = Vec::new();
                    for (h_line_idx, _) in hunk.lines.iter().enumerate() {
                        let global_line_idx = line_offset + h_line_idx;
                        if global_line_idx >= sel_min && global_line_idx <= sel_max {
                            hunk_selected_indices.push(h_line_idx);
                        }
                    }
                    if !hunk_selected_indices.is_empty() {
                        return Some((file, hunk, hunk_selected_indices));
                    }
                }
                line_offset += hunk_len;
            }
        }
        None
    }

    pub fn stage_selected_lines(&mut self) {
        if !self.git_engine.is_git_repo() {
            return;
        }
        if let Some((file, hunk, indices)) = self.selected_line_indices_in_current_hunk() {
            let file_clone = file.clone();
            let hunk_clone = hunk.clone();
            let count = indices.len();
            match self
                .git_engine
                .stage_lines(&file_clone, &hunk_clone, &indices)
            {
                Ok(_) => {
                    self.visual_mode = false;
                    self.visual_anchor_idx = None;
                    self.status_message = Some(format!("✅ Staged {} selected lines", count));
                    self.reload_diff();
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to stage lines: {}", e));
                }
            }
        }
    }

    pub fn unstage_selected_lines(&mut self) {
        if !self.git_engine.is_git_repo() {
            return;
        }
        if let Some((file, hunk, indices)) = self.selected_line_indices_in_current_hunk() {
            let file_clone = file.clone();
            let hunk_clone = hunk.clone();
            let count = indices.len();
            match self
                .git_engine
                .unstage_lines(&file_clone, &hunk_clone, &indices)
            {
                Ok(_) => {
                    self.visual_mode = false;
                    self.visual_anchor_idx = None;
                    self.status_message = Some(format!("↩️ Unstaged {} selected lines", count));
                    self.reload_diff();
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to unstage lines: {}", e));
                }
            }
        }
    }

    pub fn discard_selected_lines(&mut self) {
        if !self.git_engine.is_git_repo() {
            return;
        }
        if let Some((file, hunk, indices)) = self.selected_line_indices_in_current_hunk() {
            let file_clone = file.clone();
            let hunk_clone = hunk.clone();
            let count = indices.len();
            match self
                .git_engine
                .discard_lines(&file_clone, &hunk_clone, &indices)
            {
                Ok(_) => {
                    self.visual_mode = false;
                    self.visual_anchor_idx = None;
                    self.status_message = Some(format!("🗑️ Discarded {} selected lines", count));
                    self.reload_diff();
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to discard lines: {}", e));
                }
            }
        }
    }

    pub fn next_hunk(&mut self) {
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let mut line_offset = 0;
            let mut target_cursor = None;
            for hunk in &file.hunks {
                if line_offset > self.cursor_line_idx {
                    target_cursor = Some(line_offset);
                    break;
                }
                line_offset += hunk.lines.len();
            }

            if let Some(target) = target_cursor {
                self.cursor_line_idx = target;
            } else if self.selected_file_idx + 1 < self.snapshot.files.len() {
                self.next_file();
            }
        }
    }

    pub fn prev_hunk(&mut self) {
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let mut hunk_starts = Vec::new();
            let mut line_offset = 0;
            for hunk in &file.hunks {
                hunk_starts.push(line_offset);
                line_offset += hunk.lines.len();
            }

            let mut target = None;
            for &start in hunk_starts.iter().rev() {
                if start < self.cursor_line_idx {
                    target = Some(start);
                    break;
                }
            }

            if let Some(t) = target {
                self.cursor_line_idx = t;
            } else if self.selected_file_idx > 0 {
                self.prev_file();
                self.cursor_to_bottom();
            }
        }
    }

    pub fn next_paragraph(&mut self) {
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let mut lines: Vec<&str> = Vec::new();
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    lines.push(&line.content);
                }
            }
            if lines.is_empty() {
                return;
            }
            let mut target = None;
            let mut found_non_empty = false;
            for (i, content) in lines.iter().enumerate().skip(self.cursor_line_idx + 1) {
                let is_blank = content.trim().is_empty();
                if !is_blank {
                    found_non_empty = true;
                } else if found_non_empty && is_blank {
                    target = Some(i);
                    break;
                }
            }
            if let Some(t) = target {
                self.cursor_line_idx = t;
            } else if !lines.is_empty() {
                self.cursor_line_idx = lines.len() - 1;
            }
        }
    }

    pub fn prev_paragraph(&mut self) {
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let mut lines: Vec<&str> = Vec::new();
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    lines.push(&line.content);
                }
            }
            if lines.is_empty() {
                return;
            }
            let mut target = None;
            let mut found_non_empty = false;
            for i in (0..self.cursor_line_idx).rev() {
                let is_blank = lines[i].trim().is_empty();
                if !is_blank {
                    found_non_empty = true;
                } else if found_non_empty && is_blank {
                    target = Some(i);
                    break;
                }
            }
            if let Some(t) = target {
                self.cursor_line_idx = t;
            } else {
                self.cursor_line_idx = 0;
            }
        }
    }

    pub fn next_comment(&mut self) {
        if self.comments.is_empty() {
            self.status_message = Some("No comments saved".to_string());
            return;
        }
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let path_str = file.display_name();
            let mut line_counter = 0;
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    let lno = line.new_line_num.or(line.old_line_num);
                    if line_counter > self.cursor_line_idx {
                        if let Some(num) = lno {
                            if self
                                .comments
                                .iter()
                                .any(|c| c.file_path == path_str && c.target_line_no == num)
                            {
                                self.cursor_line_idx = line_counter;
                                self.status_message =
                                    Some(format!("Comment at {}:{}", path_str, num));
                                return;
                            }
                        }
                    }
                    line_counter += 1;
                }
            }
        }
        self.status_message = Some("End of comments in file".to_string());
    }

    pub fn prev_comment(&mut self) {
        if self.comments.is_empty() {
            self.status_message = Some("No comments saved".to_string());
            return;
        }
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let path_str = file.display_name();
            let mut match_line = None;
            let mut line_counter = 0;
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    let lno = line.new_line_num.or(line.old_line_num);
                    if line_counter < self.cursor_line_idx {
                        if let Some(num) = lno {
                            if self
                                .comments
                                .iter()
                                .any(|c| c.file_path == path_str && c.target_line_no == num)
                            {
                                match_line = Some(line_counter);
                            }
                        }
                    }
                    line_counter += 1;
                }
            }
            if let Some(idx) = match_line {
                self.cursor_line_idx = idx;
                self.status_message = Some("Jumped to previous comment".to_string());
            }
        }
    }

    pub fn yank_current_code_line(&mut self) {
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let mut line_counter = 0;
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    if line_counter == self.cursor_line_idx {
                        IpcBridge::send_osc52_clipboard(&line.content);
                        self.status_message = Some(format!("Yanked: \"{}\"", line.content.trim()));
                        return;
                    }
                    line_counter += 1;
                }
            }
        }
    }

    pub fn yank_reference_to_clipboard(&mut self) {
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let mut line_num = 1;
            let mut line_counter = 0;
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    if line_counter == self.cursor_line_idx {
                        line_num = line.new_line_num.or(line.old_line_num).unwrap_or(1);
                        break;
                    }
                    line_counter += 1;
                }
            }
            let reference = format!("#ref {}:{}", file.display_name(), line_num);
            IpcBridge::send_osc52_clipboard(&reference);
            self.status_message = Some(format!("Copied to clipboard: {}", reference));
        }
    }

    pub fn execute_search(&mut self, query: &str) {
        self.search_matches.clear();
        let q = query.to_lowercase();
        if q.is_empty() {
            return;
        }

        for (f_idx, file) in self.snapshot.files.iter().enumerate() {
            let mut line_counter = 0;
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    if line.content.to_lowercase().contains(&q) {
                        self.search_matches.push((f_idx, line_counter));
                    }
                    line_counter += 1;
                }
            }
        }

        if !self.search_matches.is_empty() {
            self.search_match_idx = 0;
            let (f_idx, l_idx) = self.search_matches[0];
            self.selected_file_idx = f_idx;
            self.cursor_line_idx = l_idx;
            self.status_message = Some(format!(
                "Match 1/{} for '{}'",
                self.search_matches.len(),
                query
            ));
        } else {
            self.status_message = Some(format!("Pattern not found: '{}'", query));
        }
    }

    pub fn next_search_match(&mut self) {
        if self.search_matches.is_empty() {
            if !self.search_query.is_empty() {
                self.execute_search(&self.search_query.clone());
            }
            if self.search_matches.is_empty() {
                self.status_message =
                    Some("No search pattern active (press / to search)".to_string());
                return;
            }
        }
        self.search_match_idx = (self.search_match_idx + 1) % self.search_matches.len();
        let (f_idx, l_idx) = self.search_matches[self.search_match_idx];
        self.selected_file_idx = f_idx;
        self.cursor_line_idx = l_idx;
        self.status_message = Some(format!(
            "Match {}/{} for '{}'",
            self.search_match_idx + 1,
            self.search_matches.len(),
            self.search_query
        ));
    }

    pub fn prev_search_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        if self.search_match_idx == 0 {
            self.search_match_idx = self.search_matches.len() - 1;
        } else {
            self.search_match_idx -= 1;
        }
        let (f_idx, l_idx) = self.search_matches[self.search_match_idx];
        self.selected_file_idx = f_idx;
        self.cursor_line_idx = l_idx;
        self.status_message = Some(format!(
            "Match {}/{} for '{}'",
            self.search_match_idx + 1,
            self.search_matches.len(),
            self.search_query
        ));
    }

    pub fn search_symbol_under_cursor(&mut self, forward: bool) {
        let mut target_word = None;
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let mut line_counter = 0;
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    if line_counter == self.cursor_line_idx {
                        let text = line.content.trim();
                        let words: Vec<&str> = text
                            .split(|c: char| !c.is_alphanumeric() && c != '_')
                            .filter(|w| !w.is_empty())
                            .collect();
                        target_word = words.first().map(|&s| s.to_string());
                        break;
                    }
                    line_counter += 1;
                }
                if target_word.is_some() {
                    break;
                }
            }
        }

        if let Some(word) = target_word {
            self.search_query = word.clone();
            self.execute_search(&word);
            if !forward {
                self.prev_search_match();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use crossbeam_channel::unbounded;
    use std::path::PathBuf;

    fn create_test_app() -> App {
        let (_tx, rx) = unbounded();
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let tmp = std::env::temp_dir().join(format!("lgtm-test-{}", unique));
        let _ = std::fs::create_dir_all(&tmp);
        let mut app = App::new(tmp, rx, None, None);
        app.comments.clear();
        app
    }

    #[test]
    fn test_ex_command_quit() {
        let mut app = create_test_app();
        assert!(!app.should_quit);

        app.execute_command(":q");
        assert!(app.should_quit);

        let mut app2 = create_test_app();
        app2.execute_command("quit!");
        assert!(app2.should_quit);
    }

    #[test]
    fn test_ex_command_toggle_views() {
        let mut app = create_test_app();
        let initial_split = app.is_split_view;
        app.execute_command(":sp");
        assert_ne!(app.is_split_view, initial_split);

        let initial_drawer = app.show_file_drawer;
        app.execute_command(":drawer");
        assert_ne!(app.show_file_drawer, initial_drawer);

        app.execute_command(":help");
        assert_eq!(app.active_view, ActiveView::Help);

        app.execute_command(":files");
        assert_eq!(app.active_view, ActiveView::FilePicker);

        app.execute_command(":comments");
        assert_eq!(app.active_view, ActiveView::CommentList);
    }

    #[test]
    fn test_ex_command_pane_and_line_jump() {
        let mut app = create_test_app();
        app.execute_command(":pane %42");
        assert_eq!(app.agent_pane.as_deref(), Some("%42"));

        app.execute_command(":10");
        assert!(app
            .status_message
            .as_ref()
            .unwrap()
            .contains("Jumped to line 10"));
    }

    #[test]
    fn test_ex_command_autocomplete() {
        let mut app = create_test_app();
        app.command_input = "w".to_string();
        app.autocomplete_next();
        assert!(!app.command_completions.is_empty());
        assert_eq!(app.command_input, "w");

        app.autocomplete_next();
        assert_eq!(app.command_input, "write");

        app.autocomplete_next();
        assert_eq!(app.command_input, "wq");

        app.autocomplete_prev();
        assert_eq!(app.command_input, "write");

        // Test :e<tab>
        let mut app_e = create_test_app();
        app_e.command_input = "e".to_string();
        app_e.autocomplete_next();
        assert_eq!(app_e.command_completions, vec!["e", "edit", "exit"]);
        assert_eq!(app_e.command_input, "e");
        app_e.autocomplete_next();
        assert_eq!(app_e.command_input, "edit");
        app_e.autocomplete_next();
        assert_eq!(app_e.command_input, "exit");

        // Test :<tab> (empty input shows all commands)
        let mut app_all = create_test_app();
        app_all.command_input = "".to_string();
        app_all.autocomplete_next();
        assert!(app_all.command_completions.len() > 30);
        assert_eq!(app_all.command_input, "audit");
        app_all.autocomplete_next();
        assert_eq!(app_all.command_input, "c");
    }

    #[test]
    fn test_paragraph_jumps_and_page_scroll() {
        use crate::model::{ChangeType, DiffHunk, DiffLine, FileDiff, LineType};
        let mut app = create_test_app();
        let lines = vec![
            DiffLine {
                line_type: LineType::Context,
                content: "fn main() {".into(),
                old_line_num: Some(1),
                new_line_num: Some(1),
            },
            DiffLine {
                line_type: LineType::Addition,
                content: "    let x = 1;".into(),
                old_line_num: None,
                new_line_num: Some(2),
            },
            DiffLine {
                line_type: LineType::Context,
                content: "".into(),
                old_line_num: Some(2),
                new_line_num: Some(3),
            }, // blank paragraph 1
            DiffLine {
                line_type: LineType::Addition,
                content: "    let y = 2;".into(),
                old_line_num: None,
                new_line_num: Some(4),
            },
            DiffLine {
                line_type: LineType::Context,
                content: "".into(),
                old_line_num: Some(3),
                new_line_num: Some(5),
            }, // blank paragraph 2
            DiffLine {
                line_type: LineType::Context,
                content: "}".into(),
                old_line_num: Some(4),
                new_line_num: Some(6),
            },
        ];
        let hunk = DiffHunk {
            header: "@@ -1,4 +1,6 @@".into(),
            old_start: 1,
            old_lines: 4,
            new_start: 1,
            new_lines: 6,
            lines,
            is_folded: false,
        };
        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("test.rs"),
            change_type: ChangeType::Modified,
            additions: 2,
            deletions: 0,
            hunks: vec![hunk],
            is_folded: false,
            is_reviewed: false,
        };
        app.snapshot = DiffSnapshot::new(vec![file]);

        app.cursor_line_idx = 0;
        app.next_paragraph();
        assert_eq!(app.cursor_line_idx, 2);

        app.next_paragraph();
        assert_eq!(app.cursor_line_idx, 4);

        app.prev_paragraph();
        assert_eq!(app.cursor_line_idx, 2);

        app.prev_paragraph();
        assert_eq!(app.cursor_line_idx, 0);

        app.page_down(3);
        assert_eq!(app.cursor_line_idx, 3);
        app.page_up(2);
        assert_eq!(app.cursor_line_idx, 1);
    }

    #[test]
    fn test_comment_add_and_remove_and_pr_review() {
        use crate::model::{ChangeType, DiffHunk, DiffLine, FileDiff, LineType};
        let mut app = create_test_app();
        let lines = vec![DiffLine {
            line_type: LineType::Addition,
            content: "    let secret = 123;".into(),
            old_line_num: None,
            new_line_num: Some(1),
        }];
        let hunk = DiffHunk {
            header: "@@ -0,0 +1,1 @@".into(),
            old_start: 0,
            old_lines: 0,
            new_start: 1,
            new_lines: 1,
            lines,
            is_folded: false,
        };
        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("auth.rs"),
            change_type: ChangeType::Added,
            additions: 1,
            deletions: 0,
            hunks: vec![hunk],
            is_folded: false,
            is_reviewed: false,
        };
        app.snapshot = DiffSnapshot::new(vec![file]);
        app.current_pr_number = Some(99);

        app.cursor_line_idx = 0;
        app.comment_input = "Do not hardcode secret".into();
        app.save_current_line_comment();

        assert_eq!(app.comments.len(), 1);
        assert_eq!(app.comments[0].pr_number, Some(99));

        let (prompt, file_path) = CommentStore::compile_pr_review_prompt(&app.comments, 99);
        assert!(prompt.contains("PR #99"));
        assert!(prompt.contains("Do not hardcode secret"));
        assert_eq!(file_path, PathBuf::from(".lgtm/pr-99-review.md"));

        // Test delete comment
        app.delete_current_line_comment();
        assert_eq!(app.comments.len(), 0);
    }

    #[test]
    fn test_unreviewed_file_navigation() {
        use crate::model::{ChangeType, FileDiff};
        let mut app = create_test_app();
        let f1 = FileDiff {
            old_path: None,
            new_path: PathBuf::from("a.rs"),
            change_type: ChangeType::Modified,
            additions: 1,
            deletions: 0,
            hunks: vec![],
            is_folded: false,
            is_reviewed: true,
        };
        let f2 = FileDiff {
            old_path: None,
            new_path: PathBuf::from("b.rs"),
            change_type: ChangeType::Modified,
            additions: 1,
            deletions: 0,
            hunks: vec![],
            is_folded: false,
            is_reviewed: false,
        };
        let f3 = FileDiff {
            old_path: None,
            new_path: PathBuf::from("c.rs"),
            change_type: ChangeType::Modified,
            additions: 1,
            deletions: 0,
            hunks: vec![],
            is_folded: false,
            is_reviewed: true,
        };
        let f4 = FileDiff {
            old_path: None,
            new_path: PathBuf::from("d.rs"),
            change_type: ChangeType::Modified,
            additions: 1,
            deletions: 0,
            hunks: vec![],
            is_folded: false,
            is_reviewed: false,
        };

        app.snapshot = DiffSnapshot::new(vec![f1, f2, f3, f4]);
        app.selected_file_idx = 0;

        app.next_unreviewed_file();
        assert_eq!(app.selected_file_idx, 1);

        app.next_unreviewed_file();
        assert_eq!(app.selected_file_idx, 3);

        app.prev_unreviewed_file();
        assert_eq!(app.selected_file_idx, 1);
    }

    #[test]
    fn test_view_centering() {
        let mut app = create_test_app();
        app.cursor_line_idx = 50;
        app.center_cursor();
        assert_eq!(app.diff_scroll_offset, 35);

        app.cursor_to_top_of_view();
        assert_eq!(app.diff_scroll_offset, 50);

        app.cursor_to_bottom_of_view();
        assert_eq!(app.diff_scroll_offset, 20);
    }

    #[test]
    fn test_search_and_symbol_under_cursor() {
        use crate::model::{ChangeType, DiffHunk, DiffLine, FileDiff, LineType};
        let mut app = create_test_app();
        let lines = vec![
            DiffLine {
                line_type: LineType::Context,
                content: "fn calculate_total() {".into(),
                old_line_num: Some(1),
                new_line_num: Some(1),
            },
            DiffLine {
                line_type: LineType::Addition,
                content: "    let total = 100;".into(),
                old_line_num: None,
                new_line_num: Some(2),
            },
            DiffLine {
                line_type: LineType::Addition,
                content: "    println!(\"{}\", total);".into(),
                old_line_num: None,
                new_line_num: Some(3),
            },
            DiffLine {
                line_type: LineType::Context,
                content: "}".into(),
                old_line_num: Some(2),
                new_line_num: Some(4),
            },
        ];
        let hunk = DiffHunk {
            header: "@@ -1,2 +1,4 @@".into(),
            old_start: 1,
            old_lines: 2,
            new_start: 1,
            new_lines: 4,
            lines,
            is_folded: false,
        };
        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("calc.rs"),
            change_type: ChangeType::Modified,
            additions: 2,
            deletions: 0,
            hunks: vec![hunk],
            is_folded: false,
            is_reviewed: false,
        };
        app.snapshot = DiffSnapshot::new(vec![file]);

        app.cursor_line_idx = 1; // "    let total = 100;"
        app.search_symbol_under_cursor(true);
        assert_eq!(app.search_query, "let");

        app.execute_search("total");
        assert_eq!(app.search_matches.len(), 3);

        app.next_search_match();
        assert_eq!(app.cursor_line_idx, 1);

        app.next_search_match();
        assert_eq!(app.cursor_line_idx, 2);

        app.prev_search_match();
        assert_eq!(app.cursor_line_idx, 1);
    }

    #[test]
    fn test_hunk_and_alert_and_file_jumps() {
        use crate::guard::{GuardAlert, Severity};
        use crate::model::{ChangeType, DiffHunk, DiffLine, FileDiff, LineType};
        let mut app = create_test_app();

        let h1 = DiffHunk {
            header: "@@ -1,1 +1,2 @@".into(),
            old_start: 1,
            old_lines: 1,
            new_start: 1,
            new_lines: 2,
            lines: vec![
                DiffLine {
                    line_type: LineType::Addition,
                    content: "a".into(),
                    old_line_num: None,
                    new_line_num: Some(1),
                },
                DiffLine {
                    line_type: LineType::Addition,
                    content: "b".into(),
                    old_line_num: None,
                    new_line_num: Some(2),
                },
            ],
            is_folded: false,
        };
        let h2 = DiffHunk {
            header: "@@ -10,1 +10,2 @@".into(),
            old_start: 10,
            old_lines: 1,
            new_start: 10,
            new_lines: 2,
            lines: vec![
                DiffLine {
                    line_type: LineType::Addition,
                    content: "c".into(),
                    old_line_num: None,
                    new_line_num: Some(10),
                },
                DiffLine {
                    line_type: LineType::Addition,
                    content: "d".into(),
                    old_line_num: None,
                    new_line_num: Some(11),
                },
            ],
            is_folded: false,
        };

        let f1 = FileDiff {
            old_path: None,
            new_path: PathBuf::from("f1.rs"),
            change_type: ChangeType::Modified,
            additions: 4,
            deletions: 0,
            hunks: vec![h1, h2],
            is_folded: false,
            is_reviewed: false,
        };
        let f2 = FileDiff {
            old_path: None,
            new_path: PathBuf::from("f2.rs"),
            change_type: ChangeType::Modified,
            additions: 0,
            deletions: 0,
            hunks: vec![],
            is_folded: false,
            is_reviewed: false,
        };

        app.snapshot = DiffSnapshot::new(vec![f1, f2]);
        app.alerts = vec![
            GuardAlert {
                severity: Severity::Critical,
                title: "Alert 1".into(),
                description: "d1".into(),
                line_no: Some(1),
                file_path: "f1.rs".into(),
            },
            GuardAlert {
                severity: Severity::Warn,
                title: "Alert 2".into(),
                description: "d2".into(),
                line_no: Some(10),
                file_path: "f1.rs".into(),
            },
        ];

        // Hunk hopping
        app.cursor_line_idx = 0;
        app.next_hunk();
        assert_eq!(app.cursor_line_idx, 2);

        app.prev_hunk();
        assert_eq!(app.cursor_line_idx, 0);

        // Alert hopping
        app.next_alert();
        assert_eq!(app.selected_alert_idx, 1);
        app.prev_alert();
        assert_eq!(app.selected_alert_idx, 0);

        // File hopping
        app.next_file();
        assert_eq!(app.selected_file_idx, 1);
        app.prev_file();
        assert_eq!(app.selected_file_idx, 0);

        // Review toggle
        app.toggle_review_current_file();
        assert!(app.snapshot.files[0].is_reviewed);
    }

    #[test]
    fn test_modal_key_events() {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
        let mut app = create_test_app();
        let make_key = |code: KeyCode| KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        // Help Modal Key
        app.active_view = ActiveView::Help;
        app.handle_key(make_key(KeyCode::Esc));
        assert_eq!(app.active_view, ActiveView::Diff);

        // Search Input Key
        app.active_view = ActiveView::Search;
        app.handle_key(make_key(KeyCode::Char('f')));
        app.handle_key(make_key(KeyCode::Char('o')));
        app.handle_key(make_key(KeyCode::Char('o')));
        assert_eq!(app.search_query, "foo");
        app.handle_key(make_key(KeyCode::Backspace));
        assert_eq!(app.search_query, "fo");
        app.handle_key(make_key(KeyCode::Enter));
        assert_eq!(app.active_view, ActiveView::Diff);

        // Comment Input Key
        app.active_view = ActiveView::CommentInput;
        app.handle_key(make_key(KeyCode::Char('H')));
        app.handle_key(make_key(KeyCode::Char('i')));
        assert_eq!(app.comment_input, "Hi");
        app.handle_key(make_key(KeyCode::Esc));
        assert_eq!(app.active_view, ActiveView::Diff);

        // Leader Keys in Diff View
        app.handle_key(make_key(KeyCode::Char(' ')));
        assert_eq!(app.pending_leader, Some(' '));
        app.handle_key(make_key(KeyCode::Char('q')));
        assert!(app.should_quit);
    }

    #[test]
    fn test_fuzzy_file_picker_filtering() {
        use crate::model::{ChangeType, FileDiff};
        let mut app = create_test_app();
        let f1 = FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/app.rs"),
            change_type: ChangeType::Modified,
            additions: 1,
            deletions: 0,
            hunks: vec![],
            is_folded: false,
            is_reviewed: false,
        };
        let f2 = FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/ui/mod.rs"),
            change_type: ChangeType::Modified,
            additions: 1,
            deletions: 0,
            hunks: vec![],
            is_folded: false,
            is_reviewed: false,
        };
        let f3 = FileDiff {
            old_path: None,
            new_path: PathBuf::from("tests/guard_test.rs"),
            change_type: ChangeType::Added,
            additions: 1,
            deletions: 0,
            hunks: vec![],
            is_folded: false,
            is_reviewed: false,
        };
        app.snapshot = DiffSnapshot::new(vec![f1, f2, f3]);

        app.picker_query = "ui".to_string();
        app.update_file_picker_filter();
        assert_eq!(app.picker_filtered_indices.len(), 1);
        assert_eq!(app.picker_filtered_indices[0], 1); // src/ui/mod.rs

        app.picker_query = "xyz123nonexistent".to_string();
        app.update_file_picker_filter();
        assert_eq!(app.picker_filtered_indices.len(), 0);

        app.picker_query = "".to_string();
        app.update_file_picker_filter();
        assert_eq!(app.picker_filtered_indices.len(), 3);
    }

    #[test]
    fn test_jump_to_line_and_out_of_bounds() {
        use crate::model::{ChangeType, DiffHunk, DiffLine, FileDiff, LineType};
        let mut app = create_test_app();
        let lines = vec![
            DiffLine {
                line_type: LineType::Context,
                content: "line 1".into(),
                old_line_num: Some(1),
                new_line_num: Some(1),
            },
            DiffLine {
                line_type: LineType::Addition,
                content: "line 2".into(),
                old_line_num: None,
                new_line_num: Some(2),
            },
            DiffLine {
                line_type: LineType::Addition,
                content: "line 3".into(),
                old_line_num: None,
                new_line_num: Some(3),
            },
        ];
        let hunk = DiffHunk {
            header: "@@ -1,1 +1,3 @@".into(),
            old_start: 1,
            old_lines: 1,
            new_start: 1,
            new_lines: 3,
            lines,
            is_folded: false,
        };
        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/main.rs"),
            change_type: ChangeType::Modified,
            additions: 2,
            deletions: 0,
            hunks: vec![hunk],
            is_folded: false,
            is_reviewed: false,
        };
        app.snapshot = DiffSnapshot::new(vec![file]);

        app.jump_to_line(2);
        assert_eq!(app.cursor_line_idx, 1);

        // Jump out of bounds should clamp to max lines
        app.jump_to_line(9999);
        assert_eq!(app.cursor_line_idx, 2);

        // Jump to specific file and line
        app.jump_to_file_and_line("src/main.rs", 3);
        assert_eq!(app.cursor_line_idx, 2);
    }

    #[test]
    fn test_current_hunk_info_and_staging() {
        use crate::model::{ChangeType, DiffHunk, DiffLine, FileDiff, LineType};
        let mut app = create_test_app();
        let hunk1 = DiffHunk {
            header: "@@ -1,2 +1,3 @@".into(),
            old_start: 1,
            old_lines: 2,
            new_start: 1,
            new_lines: 3,
            lines: vec![
                DiffLine {
                    line_type: LineType::Context,
                    content: "fn main() {".into(),
                    old_line_num: Some(1),
                    new_line_num: Some(1),
                },
                DiffLine {
                    line_type: LineType::Addition,
                    content: "    println!(\"hello\");".into(),
                    old_line_num: None,
                    new_line_num: Some(2),
                },
            ],
            is_folded: false,
        };
        let hunk2 = DiffHunk {
            header: "@@ -10,2 +10,3 @@".into(),
            old_start: 10,
            old_lines: 2,
            new_start: 10,
            new_lines: 3,
            lines: vec![DiffLine {
                line_type: LineType::Addition,
                content: "fn helper() {}".into(),
                old_line_num: None,
                new_line_num: Some(10),
            }],
            is_folded: false,
        };

        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/main.rs"),
            change_type: ChangeType::Modified,
            additions: 2,
            deletions: 0,
            hunks: vec![hunk1, hunk2],
            is_folded: false,
            is_reviewed: false,
        };
        app.snapshot = DiffSnapshot::new(vec![file]);

        app.cursor_line_idx = 1;
        let (f, h, h_idx) = app.current_hunk_info().unwrap();
        assert_eq!(f.display_name(), "src/main.rs");
        assert_eq!(h.header, "@@ -1,2 +1,3 @@");
        assert_eq!(h_idx, 0);

        app.cursor_line_idx = 2;
        let (_, h2, h2_idx) = app.current_hunk_info().unwrap();
        assert_eq!(h2.header, "@@ -10,2 +10,3 @@");
        assert_eq!(h2_idx, 1);
    }

    #[test]
    fn test_app_visual_mode_selection() {
        let (_tx, rx) = crossbeam_channel::unbounded();
        let mut app = App::new(PathBuf::from("."), rx, None, None);

        assert!(!app.visual_mode);
        app.toggle_visual_mode();
        assert!(app.visual_mode);
        assert_eq!(app.visual_anchor_idx, Some(0));

        app.cursor_line_idx = 3;
        assert!(app.is_in_visual_selection(0));
        assert!(app.is_in_visual_selection(2));
        assert!(app.is_in_visual_selection(3));
        assert!(!app.is_in_visual_selection(4));

        app.toggle_visual_mode();
        assert!(!app.visual_mode);
        assert!(!app.is_in_visual_selection(2));
    }

    #[test]
    fn test_comment_actions_and_jump() {
        let mut app = create_test_app();

        let hunk = DiffHunk {
            header: "@@ -1,15 +1,15 @@".into(),
            old_start: 1,
            old_lines: 15,
            new_start: 1,
            new_lines: 15,
            lines: (1..=15)
                .map(|i| DiffLine {
                    line_type: LineType::Context,
                    content: format!("line {}", i),
                    old_line_num: Some(i),
                    new_line_num: Some(i),
                })
                .collect(),
            is_folded: false,
        };

        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/lib.rs"),
            change_type: ChangeType::Modified,
            additions: 0,
            deletions: 0,
            hunks: vec![hunk],
            is_folded: true,
            is_reviewed: false,
        };

        app.snapshot = DiffSnapshot::new(vec![file]);

        let anchor = CommentAnchor {
            file_path: "src/lib.rs".into(),
            target_line_no: 12,
            target_code: "line 12".into(),
            context_before: vec![],
            context_after: vec![],
            comment_text: "Need refactoring here".into(),
            replies: vec![],
            created_at_turn: 1,
            is_resolved: false,
            pr_number: None,
        };
        app.comments.push(anchor);

        // Test jump unfolds file, sets cursor and diff_scroll_offset
        app.jump_to_file_and_line("src/lib.rs", 12);
        assert!(!app.snapshot.files[0].is_folded);
        assert_eq!(app.cursor_line_idx, 11);
        assert_eq!(app.diff_scroll_offset, 1);

        // Test cursor comment detection
        assert_eq!(app.comment_idx_at_cursor(), Some(0));

        // Test toggle resolve
        app.toggle_resolve_current_comment();
        assert!(app.comments[0].is_resolved);
        app.toggle_resolve_current_comment();
        assert!(!app.comments[0].is_resolved);

        // Test start reply and submit reply
        app.start_reply_to_comment(0);
        assert_eq!(app.active_view, ActiveView::CommentReply);
        assert_eq!(app.comment_reply_target_idx, Some(0));

        app.comment_reply_input = "Will fix in next commit".into();
        app.handle_comment_reply_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.active_view, ActiveView::Diff);
        assert_eq!(app.comments[0].replies.len(), 1);
        assert_eq!(app.comments[0].replies[0], "Will fix in next commit");

        // Test CommentList actions (resolve toggle, delete)
        app.active_view = ActiveView::CommentList;
        app.comment_list_selected_idx = 0;

        // Toggle resolve via 'v'
        app.handle_comment_list_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE));
        assert!(app.comments[0].is_resolved);

        // Test ex command :unresolve and :resolve
        app.execute_command(":unresolve");
        assert!(!app.comments[0].is_resolved);
        app.execute_command(":resolve all");
        assert!(app.comments[0].is_resolved);

        // Test :clear-resolved
        app.execute_command(":clear-resolved");
        assert!(app.comments.is_empty());
    }

    #[test]
    fn test_hunk_folding_and_unfolding() {
        let mut app = create_test_app();
        let hunk1 = DiffHunk {
            header: "@@ -1,3 +1,3 @@".into(),
            old_start: 1,
            old_lines: 3,
            new_start: 1,
            new_lines: 3,
            lines: vec![DiffLine {
                line_type: LineType::Addition,
                content: "fn one() {}".into(),
                old_line_num: None,
                new_line_num: Some(1),
            }],
            is_folded: false,
        };
        let hunk2 = DiffHunk {
            header: "@@ -10,3 +10,3 @@".into(),
            old_start: 10,
            old_lines: 3,
            new_start: 10,
            new_lines: 3,
            lines: vec![DiffLine {
                line_type: LineType::Addition,
                content: "fn two() {}".into(),
                old_line_num: None,
                new_line_num: Some(10),
            }],
            is_folded: false,
        };

        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/fold_test.rs"),
            change_type: ChangeType::Modified,
            additions: 2,
            deletions: 0,
            hunks: vec![hunk1, hunk2],
            is_folded: false,
            is_reviewed: false,
        };

        app.snapshot = DiffSnapshot::new(vec![file]);

        // Fold first hunk
        app.cursor_line_idx = 0;
        app.fold_current_hunk();
        assert!(app.snapshot.files[0].hunks[0].is_folded);
        assert!(!app.snapshot.files[0].hunks[1].is_folded);

        // Unfold first hunk
        app.unfold_current_hunk();
        assert!(!app.snapshot.files[0].hunks[0].is_folded);

        // Fold all hunks
        app.fold_all_hunks();
        assert!(app.snapshot.files[0].hunks[0].is_folded);
        assert!(app.snapshot.files[0].hunks[1].is_folded);

        // Unfold all hunks
        app.unfold_all_hunks();
        assert!(!app.snapshot.files[0].hunks[0].is_folded);
        assert!(!app.snapshot.files[0].hunks[1].is_folded);
    }

    #[test]
    fn test_commit_modal_interactions() {
        let mut app = create_test_app();
        app.active_view = ActiveView::CommitModal;
        app.commit_message_input = "feat: add commit forge".to_string();

        // Key typing
        app.handle_commit_modal_key(KeyEvent::new(KeyCode::Char('!'), KeyModifiers::NONE));
        assert_eq!(app.commit_message_input, "feat: add commit forge!");

        // Backspace
        app.handle_commit_modal_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(app.commit_message_input, "feat: add commit forge");

        // Esc cancels back to diff view
        app.handle_commit_modal_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.active_view, ActiveView::Diff);
    }
}
