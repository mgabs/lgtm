use super::*;

impl App {
    pub fn handle_key(&mut self, key: KeyEvent) {
        match self.active_view {
            ActiveView::Diff => self.handle_diff_key(key),
            ActiveView::FilePicker => self.handle_file_picker_key(key),
            ActiveView::PanePicker => self.handle_pane_picker_key(key),
            ActiveView::CommentInput => self.handle_comment_input_key(key),
            ActiveView::CommentReply => self.handle_comment_reply_key(key),
            ActiveView::CommentList => self.handle_comment_list_key(key),
            ActiveView::ThreadView => self.handle_thread_view_key(key),
            ActiveView::PrPicker => self.handle_pr_picker_key(key),
            ActiveView::Help => self.handle_help_key(key),
            ActiveView::Search => self.handle_search_key(key),
            ActiveView::CommandLine => self.handle_command_line_key(key),
            ActiveView::CommitModal => self.handle_commit_modal_key(key),
            ActiveView::AuditReport => self.handle_audit_report_key(key),
            ActiveView::AgentPrompt => self.handle_agent_prompt_key(key),
        }
    }

    pub(crate) fn handle_diff_key(&mut self, key: KeyEvent) {
        // Multi-key leader sequences (g, ], [, ' ', z)
        if let Some(leader) = self.pending_leader.take() {
            match (leader, key.code) {
                ('g', KeyCode::Char('g')) => {
                    self.cursor_to_top();
                    return;
                }
                (']', KeyCode::Char('f')) | (']', KeyCode::Char(']')) => {
                    self.next_file();
                    return;
                }
                (']', KeyCode::Char('[')) => {
                    self.prev_file();
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
                (']', KeyCode::Char('t')) => {
                    self.jump_thread(true);
                    return;
                }
                ('[', KeyCode::Char('t')) => {
                    self.jump_thread(false);
                    return;
                }
                ('[', KeyCode::Char('f')) | ('[', KeyCode::Char('[')) => {
                    self.prev_file();
                    return;
                }
                ('[', KeyCode::Char(']')) => {
                    self.next_file();
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
                ('c', KeyCode::Char('a')) => {
                    self.comment_input =
                        "⚡ [COVERAGE] Add unit/regression tests covering these branches."
                            .to_string();
                    self.active_view = ActiveView::CommentInput;
                    return;
                }
                ('c', KeyCode::Char('s')) => {
                    self.comment_input = "🔒 [SECURITY] Ensure inputs are sanitized, parameterized, and auth checks enforced.".to_string();
                    self.active_view = ActiveView::CommentInput;
                    return;
                }
                ('c', KeyCode::Char('r')) => {
                    self.comment_input = "🧹 [REFACTOR] Simplify logic, reduce cognitive nesting, eliminate duplication.".to_string();
                    self.active_view = ActiveView::CommentInput;
                    return;
                }
                ('c', KeyCode::Char('p')) => {
                    self.comment_input =
                        "🚀 [PERF] Optimize hot-path allocations and database query batching."
                            .to_string();
                    self.active_view = ActiveView::CommentInput;
                    return;
                }
                ('c', KeyCode::Char('w')) => {
                    self.comment_input =
                        "❓ [WHY] Explain rationale and verify architectural invariants."
                            .to_string();
                    self.active_view = ActiveView::CommentInput;
                    return;
                }
                ('c', KeyCode::Char('c')) | ('c', KeyCode::Enter) => {
                    self.comment_input.clear();
                    self.active_view = ActiveView::CommentInput;
                    return;
                }
                ('c', _) => {
                    self.comment_input.clear();
                    self.active_view = ActiveView::CommentInput;
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
                    self.open_comment_list();
                    return;
                }
                (' ', KeyCode::Char('t')) => {
                    self.open_threads_modal();
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
                        self.pane_picker.list = panes;
                        self.pane_picker.selected_idx = 0;
                        self.active_view = ActiveView::PanePicker;
                    }
                    return;
                }
                _ => {}
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
                self.file_picker.query.clear();
                self.file_picker.selected_idx = 0;
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
            (KeyModifiers::SHIFT, KeyCode::Char('X'))
            | (KeyModifiers::NONE, KeyCode::Char('X')) => {
                self.reject_hunk_and_reprompt();
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
                self.pending_leader = Some('c');
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
            (_, KeyCode::Char('A')) => {
                self.approve_gate();
            }
            (_, KeyCode::Char('P')) => {
                self.active_view = ActiveView::AgentPrompt;
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
                self.next_file();
            }
            (_, KeyCode::Char('[')) => {
                self.prev_file();
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
            (_, KeyCode::Char('M')) => {
                self.toggle_review_current_hunk();
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
                    self.pane_picker.list = panes;
                    self.pane_picker.selected_idx = 0;
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
                self.command.input.clear();
                self.command.history_idx = None;
                self.active_view = ActiveView::CommandLine;
            }
            (_, KeyCode::Char('t')) => {
                self.open_threads_modal();
            }
            (_, KeyCode::Enter) => {
                self.inject_reference_to_agent();
            }
            _ => {}
        }
    }

    pub(crate) fn handle_search_key(&mut self, key: KeyEvent) {
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

    pub(crate) fn handle_command_line_key(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('c')) | (_, KeyCode::Esc) => {
                self.command.input.clear();
                self.command.history_idx = None;
                self.command.completions.clear();
                self.command.completion_idx = None;
                self.command.prefix.clear();
                self.active_view = ActiveView::Diff;
            }
            (_, KeyCode::Backspace) => {
                self.command.completions.clear();
                self.command.completion_idx = None;
                self.command.prefix.clear();
                if self.command.input.is_empty() {
                    self.active_view = ActiveView::Diff;
                } else {
                    self.command.input.pop();
                }
            }
            (_, KeyCode::Up) | (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
                if !self.command.history.is_empty() {
                    let next_idx = match self.command.history_idx {
                        Some(0) => 0,
                        Some(i) => i - 1,
                        None => self.command.history.len() - 1,
                    };
                    self.command.history_idx = Some(next_idx);
                    self.command.input = self.command.history[next_idx].clone();
                    self.command.completions.clear();
                    self.command.completion_idx = None;
                }
            }
            (_, KeyCode::Down) | (KeyModifiers::CONTROL, KeyCode::Char('n')) => {
                if let Some(i) = self.command.history_idx {
                    if i + 1 < self.command.history.len() {
                        let next_idx = i + 1;
                        self.command.history_idx = Some(next_idx);
                        self.command.input = self.command.history[next_idx].clone();
                    } else {
                        self.command.history_idx = None;
                        self.command.input.clear();
                    }
                    self.command.completions.clear();
                    self.command.completion_idx = None;
                }
            }
            (_, KeyCode::BackTab) | (KeyModifiers::SHIFT, KeyCode::Tab) => {
                self.command.autocomplete_prev();
            }
            (_, KeyCode::Tab) => {
                self.command.autocomplete_next();
            }
            (_, KeyCode::Enter) => {
                let cmd = self.command.input.trim().to_string();
                self.command.completions.clear();
                self.command.completion_idx = None;
                self.command.prefix.clear();
                if !cmd.is_empty() {
                    if self.command.history.last().map(|s| s.as_str()) != Some(&cmd) {
                        self.command.history.push(cmd.clone());
                    }
                    self.execute_command(&cmd);
                } else {
                    self.active_view = ActiveView::Diff;
                }
            }
            (_, KeyCode::Char(c)) => {
                self.command.completions.clear();
                self.command.completion_idx = None;
                self.command.prefix.clear();
                self.command.input.push(c);
            }
            _ => {}
        }
    }

    pub(crate) fn handle_comment_list_key(&mut self, key: KeyEvent) {
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

    pub(crate) fn handle_thread_view_key(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) | (_, KeyCode::Char('q')) => {
                self.active_view = ActiveView::Diff;
            }
            (_, KeyCode::Enter) => {
                if let Some(t) = self.threads.get(self.thread_selected_idx) {
                    let path = t.path.clone();
                    let line = t.line.unwrap_or(1);
                    self.jump_to_file_and_line(&path, line);
                }
                self.active_view = ActiveView::Diff;
            }
            (_, KeyCode::Up) | (_, KeyCode::Char('k')) => {
                if self.thread_selected_idx > 0 {
                    self.thread_selected_idx -= 1;
                    self.thread_scroll_offset = 0;
                }
            }
            (_, KeyCode::Down) | (_, KeyCode::Char('j')) => {
                if !self.threads.is_empty() && self.thread_selected_idx < self.threads.len() - 1 {
                    self.thread_selected_idx += 1;
                    self.thread_scroll_offset = 0;
                }
            }
            (_, KeyCode::PageDown)
            | (KeyModifiers::CONTROL, KeyCode::Char('d'))
            | (_, KeyCode::Char('J')) => {
                self.thread_scroll_offset += 3;
            }
            (_, KeyCode::PageUp)
            | (KeyModifiers::CONTROL, KeyCode::Char('u'))
            | (_, KeyCode::Char('K')) => {
                self.thread_scroll_offset = self.thread_scroll_offset.saturating_sub(3);
            }
            (_, KeyCode::Char('v')) | (_, KeyCode::Char('R')) | (_, KeyCode::Char(' ')) => {
                if let Some(t) = self.threads.get(self.thread_selected_idx) {
                    let thread_id = t.id.clone();
                    let next_resolved = !t.resolved;
                    if let Some(pr) = self.pr.current_number {
                        let forge = lgtm_core::forge::open(&self.git_engine.repo_path);
                        match forge.set_resolved(pr, &thread_id, next_resolved) {
                            Ok(()) => {
                                if let Some(t_mut) = self.threads.get_mut(self.thread_selected_idx)
                                {
                                    t_mut.resolved = next_resolved;
                                }
                                self.status_message = Some(if next_resolved {
                                    "✅ Thread marked resolved".into()
                                } else {
                                    "💬 Thread marked active".into()
                                });
                            }
                            Err(e) => {
                                self.status_message = Some(format!("Failed to update thread: {e}"));
                            }
                        }
                    }
                }
            }
            (_, KeyCode::Char('r')) if !self.threads.is_empty() => {
                self.command.input = format!("thread reply {} ", self.thread_selected_idx + 1);
                self.command.history_idx = None;
                self.active_view = ActiveView::CommandLine;
            }
            _ => {}
        }
    }

    pub(crate) fn handle_pr_picker_key(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) | (_, KeyCode::Char('q')) => {
                self.active_view = ActiveView::Diff;
            }
            (_, KeyCode::Up) | (_, KeyCode::Char('k')) => {
                if self.pr.selected_idx > 0 {
                    self.pr.selected_idx -= 1;
                }
            }
            (_, KeyCode::Down) | (_, KeyCode::Char('j')) => {
                if !self.pr.list.is_empty() && self.pr.selected_idx < self.pr.list.len() - 1 {
                    self.pr.selected_idx += 1;
                }
            }
            (_, KeyCode::PageDown)
            | (KeyModifiers::CONTROL, KeyCode::Char('d'))
            | (KeyModifiers::CONTROL, KeyCode::Char('f')) => {
                if !self.pr.list.is_empty() {
                    self.pr.selected_idx = (self.pr.selected_idx + 5).min(self.pr.list.len() - 1);
                }
            }
            (_, KeyCode::PageUp)
            | (KeyModifiers::CONTROL, KeyCode::Char('u'))
            | (KeyModifiers::CONTROL, KeyCode::Char('b')) => {
                self.pr.selected_idx = self.pr.selected_idx.saturating_sub(5);
            }
            (_, KeyCode::Enter) => {
                if let Some(pr) = self.pr.list.get(self.pr.selected_idx) {
                    let pr_num = pr.number;
                    match crate::diff_source::DiffSource::snapshot(
                        &crate::diff_source::PullRequestSource {
                            pr: pr_num,
                            repo: self.git_engine.repo_path.clone(),
                        },
                    ) {
                        Ok(snap) => {
                            self.pr.current_number = Some(pr_num);
                            self.alerts = self.scan(&snap.files);
                            self.snapshot = snap;
                            self.selected_file_idx = 0;
                            self.cursor_line_idx = 0;
                            self.status_message =
                                Some(format!("Loaded PR #{}: {}", pr_num, pr.title));
                            self.refresh_github_async(true);
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

    pub(crate) fn handle_comment_reply_key(&mut self, key: KeyEvent) {
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

    pub(crate) fn handle_comment_input_key(&mut self, key: KeyEvent) {
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

    pub(crate) fn handle_help_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
                self.active_view = ActiveView::Diff;
            }
            KeyCode::Tab | KeyCode::Right | KeyCode::Char('l') | KeyCode::Char(']') => {
                self.help_selected_tab = (self.help_selected_tab + 1) % 7;
            }
            KeyCode::BackTab | KeyCode::Left | KeyCode::Char('h') | KeyCode::Char('[') => {
                if self.help_selected_tab == 0 {
                    self.help_selected_tab = 6;
                } else {
                    self.help_selected_tab -= 1;
                }
            }
            KeyCode::Char('1') => self.help_selected_tab = 0,
            KeyCode::Char('2') => self.help_selected_tab = 1,
            KeyCode::Char('3') => self.help_selected_tab = 2,
            KeyCode::Char('4') => self.help_selected_tab = 3,
            KeyCode::Char('5') => self.help_selected_tab = 4,
            KeyCode::Char('6') => self.help_selected_tab = 5,
            KeyCode::Char('7') => self.help_selected_tab = 6,
            _ => {}
        }
    }

    pub(crate) fn handle_file_picker_key(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                self.active_view = ActiveView::Diff;
            }
            (_, KeyCode::Enter) => {
                if let Some(&file_idx) = self
                    .file_picker
                    .filtered_indices
                    .get(self.file_picker.selected_idx)
                {
                    self.selected_file_idx = file_idx;
                    self.diff_scroll_offset = 0;
                    self.cursor_line_idx = 0;
                }
                self.active_view = ActiveView::Diff;
            }
            (_, KeyCode::Up) | (KeyModifiers::CONTROL, KeyCode::Char('k')) => {
                if self.file_picker.selected_idx > 0 {
                    self.file_picker.selected_idx -= 1;
                }
            }
            (_, KeyCode::Down) | (KeyModifiers::CONTROL, KeyCode::Char('j')) => {
                if !self.file_picker.filtered_indices.is_empty()
                    && self.file_picker.selected_idx < self.file_picker.filtered_indices.len() - 1
                {
                    self.file_picker.selected_idx += 1;
                }
            }
            (_, KeyCode::PageDown)
            | (KeyModifiers::CONTROL, KeyCode::Char('d'))
            | (KeyModifiers::CONTROL, KeyCode::Char('f')) => {
                if !self.file_picker.filtered_indices.is_empty() {
                    self.file_picker.selected_idx = (self.file_picker.selected_idx + 5)
                        .min(self.file_picker.filtered_indices.len() - 1);
                }
            }
            (_, KeyCode::PageUp)
            | (KeyModifiers::CONTROL, KeyCode::Char('u'))
            | (KeyModifiers::CONTROL, KeyCode::Char('b')) => {
                self.file_picker.selected_idx = self.file_picker.selected_idx.saturating_sub(5);
            }
            (_, KeyCode::Backspace) => {
                self.file_picker.query.pop();
                self.update_file_picker_filter();
            }
            (_, KeyCode::Char(c)) => {
                self.file_picker.query.push(c);
                self.update_file_picker_filter();
            }
            _ => {}
        }
    }

    pub(crate) fn handle_pane_picker_key(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) => {
                self.active_view = ActiveView::Diff;
            }
            (_, KeyCode::Enter) => {
                if let Some(pane) = self.pane_picker.list.get(self.pane_picker.selected_idx) {
                    self.agent_pane = Some(pane.id.clone());
                    self.status_message = Some(format!("Target pane set to {}", pane.id));
                }
                self.active_view = ActiveView::Diff;
            }
            (_, KeyCode::Up) | (_, KeyCode::Char('k')) => {
                if self.pane_picker.selected_idx > 0 {
                    self.pane_picker.selected_idx -= 1;
                }
            }
            (_, KeyCode::Down) | (_, KeyCode::Char('j')) => {
                if !self.pane_picker.list.is_empty()
                    && self.pane_picker.selected_idx < self.pane_picker.list.len() - 1
                {
                    self.pane_picker.selected_idx += 1;
                }
            }
            (_, KeyCode::PageDown)
            | (KeyModifiers::CONTROL, KeyCode::Char('d'))
            | (KeyModifiers::CONTROL, KeyCode::Char('f')) => {
                if !self.pane_picker.list.is_empty() {
                    self.pane_picker.selected_idx =
                        (self.pane_picker.selected_idx + 5).min(self.pane_picker.list.len() - 1);
                }
            }
            (_, KeyCode::PageUp)
            | (KeyModifiers::CONTROL, KeyCode::Char('u'))
            | (KeyModifiers::CONTROL, KeyCode::Char('b')) => {
                self.pane_picker.selected_idx = self.pane_picker.selected_idx.saturating_sub(5);
            }
            _ => {}
        }
    }

    pub fn handle_audit_report_key(&mut self, key: KeyEvent) {
        if let Some(leader) = self.pending_leader {
            self.pending_leader = None;
            match (leader, key.code) {
                ('g', KeyCode::Char('g')) => {
                    match self.audit.focus {
                        AuditPaneFocus::Files => {
                            self.selected_file_idx = 0;
                        }
                        AuditPaneFocus::Issues => {
                            self.audit.selected_idx = 0;
                            self.sync_audit_selected_file();
                        }
                        AuditPaneFocus::Inspector => {
                            self.audit.inspector_scroll = 0;
                        }
                    }
                    return;
                }
                (']', KeyCode::Char('f')) | (']', KeyCode::Char(']')) => {
                    self.audit_next_file();
                    return;
                }
                ('[', KeyCode::Char('f')) | ('[', KeyCode::Char('[')) => {
                    self.audit_prev_file();
                    return;
                }
                (']', KeyCode::Char('c')) => {
                    let total = self.filtered_audit_alerts().len();
                    if total > 0 && self.audit.selected_idx + 1 < total {
                        self.audit.selected_idx += 1;
                        self.audit.inspector_scroll = 0;
                        self.sync_audit_selected_file();
                    }
                    return;
                }
                ('[', KeyCode::Char('c')) => {
                    if self.audit.selected_idx > 0 {
                        self.audit.selected_idx -= 1;
                        self.audit.inspector_scroll = 0;
                        self.sync_audit_selected_file();
                    }
                    return;
                }
                _ => {}
            }
        }

        let filtered = self.filtered_audit_alerts();
        let total = filtered.len();

        match (key.modifiers, key.code) {
            (_, KeyCode::Char('q')) => {
                self.should_quit = true;
            }
            (_, KeyCode::Esc) => {
                if self.audit.maximized_inspector {
                    self.audit.maximized_inspector = false;
                    self.status_message = Some("Exited full inspector view".into());
                } else if self.audit.file_filter.is_some() {
                    self.audit.file_filter = None;
                    self.audit.selected_idx = 0;
                    self.sync_audit_selected_file();
                    self.status_message = Some("Cleared file filter: showing all issues".into());
                } else {
                    self.active_view = ActiveView::Diff;
                    self.status_message =
                        Some("Returned to diff view (use :audit or :scan to re-enter)".into());
                }
            }
            // Tab / BackTab / h / l pane focus cycling
            (_, KeyCode::Tab) => {
                self.cycle_audit_focus(true);
            }
            (_, KeyCode::BackTab) => {
                self.cycle_audit_focus(false);
            }
            (_, KeyCode::Char('h')) | (_, KeyCode::Left) => {
                self.cycle_audit_focus(false);
            }
            (_, KeyCode::Char('l')) | (_, KeyCode::Right) => {
                self.cycle_audit_focus(true);
            }
            // Toggle sidebar drawer
            (_, KeyCode::Char('b')) => {
                self.show_file_drawer = !self.show_file_drawer;
                if !self.show_file_drawer && self.audit.focus == AuditPaneFocus::Files {
                    self.audit.focus = AuditPaneFocus::Issues;
                }
            }
            // Maximize / Restore inspector view
            (_, KeyCode::Char('Z')) | (_, KeyCode::Char('m')) => {
                self.audit.maximized_inspector = !self.audit.maximized_inspector;
                if self.audit.maximized_inspector {
                    self.audit.focus = AuditPaneFocus::Inspector;
                    self.status_message =
                        Some("Maximized Inspector [Z/m or Esc to restore]".into());
                } else {
                    self.status_message = Some("Restored 3-pane layout".into());
                }
            }
            // File jumping (immediate jump + chord support)
            (_, KeyCode::Char(']')) => {
                self.audit_next_file();
                self.pending_leader = Some(']');
            }
            (_, KeyCode::Char('[')) => {
                self.audit_prev_file();
                self.pending_leader = Some('[');
            }
            // Up/Down navigation based on active focus
            (_, KeyCode::Char('j')) | (_, KeyCode::Down) => match self.audit.focus {
                AuditPaneFocus::Files => {
                    if !self.audit.files.is_empty()
                        && self.selected_file_idx + 1 < self.audit.files.len()
                    {
                        self.selected_file_idx += 1;
                    }
                }
                AuditPaneFocus::Issues => {
                    if total > 0 && self.audit.selected_idx + 1 < total {
                        self.audit.selected_idx += 1;
                        self.audit.inspector_scroll = 0;
                        self.sync_audit_selected_file();
                    }
                }
                AuditPaneFocus::Inspector => {
                    self.audit.inspector_scroll = self.audit.inspector_scroll.saturating_add(1);
                }
            },
            (_, KeyCode::Char('k')) | (_, KeyCode::Up) => match self.audit.focus {
                AuditPaneFocus::Files => {
                    if self.selected_file_idx > 0 {
                        self.selected_file_idx -= 1;
                    }
                }
                AuditPaneFocus::Issues => {
                    if self.audit.selected_idx > 0 {
                        self.audit.selected_idx -= 1;
                        self.audit.inspector_scroll = 0;
                        self.sync_audit_selected_file();
                    }
                }
                AuditPaneFocus::Inspector => {
                    self.audit.inspector_scroll = self.audit.inspector_scroll.saturating_sub(1);
                }
            },
            (KeyModifiers::CONTROL, KeyCode::Char('d')) | (_, KeyCode::PageDown) => {
                match self.audit.focus {
                    AuditPaneFocus::Files => {
                        if !self.audit.files.is_empty() {
                            self.selected_file_idx =
                                (self.selected_file_idx + 10).min(self.audit.files.len() - 1);
                        }
                    }
                    AuditPaneFocus::Issues => {
                        if total > 0 {
                            self.audit.selected_idx = (self.audit.selected_idx + 10).min(total - 1);
                            self.audit.inspector_scroll = 0;
                            self.sync_audit_selected_file();
                        }
                    }
                    AuditPaneFocus::Inspector => {
                        self.audit.inspector_scroll =
                            self.audit.inspector_scroll.saturating_add(10);
                    }
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('u')) | (_, KeyCode::PageUp) => {
                match self.audit.focus {
                    AuditPaneFocus::Files => {
                        self.selected_file_idx = self.selected_file_idx.saturating_sub(10);
                    }
                    AuditPaneFocus::Issues => {
                        self.audit.selected_idx = self.audit.selected_idx.saturating_sub(10);
                        self.audit.inspector_scroll = 0;
                        self.sync_audit_selected_file();
                    }
                    AuditPaneFocus::Inspector => {
                        self.audit.inspector_scroll =
                            self.audit.inspector_scroll.saturating_sub(10);
                    }
                }
            }
            (_, KeyCode::Char('}')) => match self.audit.focus {
                AuditPaneFocus::Inspector => {
                    self.audit.inspector_scroll = self.audit.inspector_scroll.saturating_add(5);
                }
                _ => {
                    if total > 0 {
                        self.audit.selected_idx = (self.audit.selected_idx + 5).min(total - 1);
                        self.audit.inspector_scroll = 0;
                        self.sync_audit_selected_file();
                    }
                }
            },
            (_, KeyCode::Char('{')) => match self.audit.focus {
                AuditPaneFocus::Inspector => {
                    self.audit.inspector_scroll = self.audit.inspector_scroll.saturating_sub(5);
                }
                _ => {
                    self.audit.selected_idx = self.audit.selected_idx.saturating_sub(5);
                    self.audit.inspector_scroll = 0;
                    self.sync_audit_selected_file();
                }
            },
            (_, KeyCode::Char('g')) => {
                self.pending_leader = Some('g');
            }
            (_, KeyCode::Char('G')) => match self.audit.focus {
                AuditPaneFocus::Files => {
                    if !self.audit.files.is_empty() {
                        self.selected_file_idx = self.audit.files.len() - 1;
                    }
                }
                AuditPaneFocus::Issues => {
                    if total > 0 {
                        self.audit.selected_idx = total - 1;
                        self.sync_audit_selected_file();
                    }
                }
                AuditPaneFocus::Inspector => {
                    self.audit.inspector_scroll = self.audit.inspector_scroll.saturating_add(50);
                }
            },
            // Filter severities
            (_, KeyCode::Char('1')) => {
                self.audit.filter_severity = Some(crate::guard::Severity::Critical);
                self.audit.selected_idx = 0;
                self.sync_audit_selected_file();
                self.status_message = Some("Filter: 🔴 Critical issues only".into());
            }
            (_, KeyCode::Char('2')) => {
                self.audit.filter_severity = Some(crate::guard::Severity::High);
                self.audit.selected_idx = 0;
                self.sync_audit_selected_file();
                self.status_message = Some("Filter: 🟠 High & 🔴 Critical issues".into());
            }
            (_, KeyCode::Char('3')) => {
                self.audit.filter_severity = None;
                self.audit.selected_idx = 0;
                self.sync_audit_selected_file();
                self.status_message = Some("Filter: All issues".into());
            }
            (_, KeyCode::Char('a')) => {
                self.audit.filter_severity = None;
                self.audit.file_filter = None;
                self.audit.selected_idx = 0;
                self.sync_audit_selected_file();
                self.status_message = Some("Filter reset: Showing all files and severities".into());
            }
            // Space: in Files pane toggles file filter
            (_, KeyCode::Char(' ')) => {
                if self.audit.focus == AuditPaneFocus::Files {
                    if let Some(file) = self.audit.files.get(self.selected_file_idx) {
                        let name = file.display_name();
                        if self.audit.file_filter.as_deref() == Some(&name) {
                            self.audit.file_filter = None;
                            self.status_message = Some("Showing issues for all files".into());
                        } else {
                            self.audit.file_filter = Some(name.clone());
                            self.status_message = Some(format!("Filtered issues to {}", name));
                        }
                        self.audit.selected_idx = 0;
                    }
                }
            }
            // Enter: in Files pane toggles file filter & jumps to Issues; in Issues pane injects directive
            (_, KeyCode::Enter) => match self.audit.focus {
                AuditPaneFocus::Files => {
                    if let Some(file) = self.audit.files.get(self.selected_file_idx) {
                        let name = file.display_name();
                        self.audit.file_filter = Some(name.clone());
                        self.audit.selected_idx = 0;
                        self.audit.focus = AuditPaneFocus::Issues;
                        self.status_message =
                            Some(format!("Filtered issues to {} (Focus: Issues)", name));
                    }
                }
                _ => {
                    if let Some(alert) = filtered.get(self.audit.selected_idx) {
                        let loc = match alert.line_no {
                            Some(l) => format!("{}:{}", alert.file_path, l),
                            None => alert.file_path.clone(),
                        };
                        let directive = format!(
                            "Fix security/architectural violation in {}: [{}] {} - {}\n",
                            loc,
                            alert.severity.badge(),
                            alert.title,
                            alert.description
                        );
                        let sink = self.agent_sink();
                        let _ = sink.send(&directive);
                        let label = sink.label();
                        drop(sink);
                        self.status_message = Some(format!("Sent directive to {}", label));
                    }
                }
            },
            // Next issue: 'w' / 'n'
            (_, KeyCode::Char('w')) | (_, KeyCode::Char('n')) => {
                if total > 0 && self.audit.selected_idx + 1 < total {
                    self.audit.selected_idx += 1;
                    self.audit.inspector_scroll = 0;
                    self.sync_audit_selected_file();
                } else if total > 0 {
                    self.audit.selected_idx = 0;
                    self.audit.inspector_scroll = 0;
                    self.sync_audit_selected_file();
                }
            }
            // Previous issue: 'W' / 'N'
            (_, KeyCode::Char('W')) | (_, KeyCode::Char('N')) => {
                if self.audit.selected_idx > 0 {
                    self.audit.selected_idx -= 1;
                    self.audit.inspector_scroll = 0;
                    self.sync_audit_selected_file();
                } else if total > 0 {
                    self.audit.selected_idx = total - 1;
                    self.audit.inspector_scroll = 0;
                    self.sync_audit_selected_file();
                }
            }
            // Export report: 'E'
            (_, KeyCode::Char('E')) => {
                let _ = self.export_audit_report(None);
            }
            // Yank remediation directive to clipboard (OSC 52)
            (_, KeyCode::Char('y')) => {
                if let Some(alert) = filtered.get(self.audit.selected_idx) {
                    let loc = match alert.line_no {
                        Some(l) => format!("{}:{}", alert.file_path, l),
                        None => alert.file_path.clone(),
                    };
                    let directive = format!(
                        "Fix security/architectural violation in {}: [{}] {} - {}\n",
                        loc,
                        alert.severity.badge(),
                        alert.title,
                        alert.description
                    );
                    IpcBridge::send_osc52_clipboard(&directive);
                    self.status_message =
                        Some("Copied remediation directive to clipboard (OSC 52)".into());
                }
            }
            // Batch inject all filtered alerts to agent pane: Ctrl-s
            (KeyModifiers::CONTROL, KeyCode::Char('s')) => {
                if filtered.is_empty() {
                    self.status_message = Some("No issues to inject".into());
                } else {
                    let mut payload = String::new();
                    for alert in &filtered {
                        let loc = match alert.line_no {
                            Some(l) => format!("{}:{}", alert.file_path, l),
                            None => alert.file_path.clone(),
                        };
                        payload.push_str(&format!(
                            "Fix {}: [{}] {} - {}\n",
                            loc,
                            alert.severity.badge(),
                            alert.title,
                            alert.description
                        ));
                    }
                    let sink = self.agent_sink();
                    let _ = sink.send(&payload);
                    let label = sink.label();
                    drop(sink);
                    self.status_message =
                        Some(format!("Sent {} issues to {}", filtered.len(), label));
                }
            }
            // File picker: 'f' or Ctrl-p
            (_, KeyCode::Char('f')) | (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
                self.active_view = ActiveView::FilePicker;
                self.file_picker.query.clear();
                self.file_picker.selected_idx = 0;
                self.update_file_picker_filter();
            }
            // Help modal: '?'
            (_, KeyCode::Char('?')) => {
                self.active_view = ActiveView::Help;
            }
            // Stage/Unstage selected file in git repo
            (_, KeyCode::Char('S')) => {
                self.toggle_stage_current_file();
            }
            (_, KeyCode::Char('U')) => {
                self.unstage_current_file();
            }
            (_, KeyCode::Char('e')) | (_, KeyCode::Char('o')) => {
                if let Some(alert) = filtered.get(self.audit.selected_idx) {
                    let file_path = alert.file_path.clone();
                    let line_no = alert.line_no.unwrap_or(1);
                    self.open_file_in_editor(&file_path, line_no);
                }
            }
            (_, KeyCode::Char(':')) => {
                self.command.input.clear();
                self.command.completions.clear();
                self.command.completion_idx = None;
                self.command.prefix.clear();
                self.active_view = ActiveView::CommandLine;
            }
            _ => {
                self.pending_leader = None;
            }
        }
    }

    pub(crate) fn handle_commit_modal_key(&mut self, key: KeyEvent) {
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

    pub(crate) fn handle_agent_prompt_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('P') => {
                self.active_view = ActiveView::Diff;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(unused_imports)]
    use super::*;
    use crate::app::test_util::create_test_app;
    use crate::model::*;
    use crossbeam_channel::unbounded;
    use std::path::PathBuf;

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

        // Thread View Key Events
        app.active_view = ActiveView::Diff;
        app.threads = vec![
            lgtm_core::github::Thread {
                id: "t1".into(),
                path: "src/main.rs".into(),
                line: Some(10),
                resolved: false,
                comments: vec![lgtm_core::github::ThreadComment {
                    id: 101,
                    author: "alice".into(),
                    body: "Please fix this logic".into(),
                }],
            },
            lgtm_core::github::Thread {
                id: "t2".into(),
                path: "src/lib.rs".into(),
                line: Some(20),
                resolved: true,
                comments: vec![lgtm_core::github::ThreadComment {
                    id: 102,
                    author: "bob".into(),
                    body: "LGTM".into(),
                }],
            },
        ];
        app.pr.current_number = Some(42);

        // Open thread view via 't' key
        app.handle_key(make_key(KeyCode::Char('t')));
        assert_eq!(app.active_view, ActiveView::ThreadView);
        assert_eq!(app.thread_selected_idx, 0);

        // Down / 'j' selects next thread
        app.handle_key(make_key(KeyCode::Char('j')));
        assert_eq!(app.thread_selected_idx, 1);

        // Up / 'k' selects previous thread
        app.handle_key(make_key(KeyCode::Char('k')));
        assert_eq!(app.thread_selected_idx, 0);

        // 'J' / 'K' scroll offset
        app.handle_key(make_key(KeyCode::Char('J')));
        assert_eq!(app.thread_scroll_offset, 3);
        app.handle_key(make_key(KeyCode::Char('K')));
        assert_eq!(app.thread_scroll_offset, 0);

        // 'r' switches to command line prefilled with reply
        app.handle_key(make_key(KeyCode::Char('r')));
        assert_eq!(app.active_view, ActiveView::CommandLine);
        assert_eq!(app.command.input, "thread reply 1 ");

        // Esc closes modal
        app.active_view = ActiveView::ThreadView;
        app.handle_key(make_key(KeyCode::Esc));
        assert_eq!(app.active_view, ActiveView::Diff);
    }
}
