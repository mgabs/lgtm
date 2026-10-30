use super::*;

impl App {
    pub fn handle_key(&mut self, key: KeyEvent) {
        let _timing = lgtm_core::debug::Span::new("ui.input");
        match self.active_view {
            ActiveView::Diff => self.handle_diff_key(key),
            ActiveView::FilePicker => self.handle_file_picker_key(key),
            ActiveView::PanePicker => self.handle_pane_picker_key(key),
            ActiveView::CommentInput => self.handle_comment_input_key(key),
            ActiveView::CommentReply => self.handle_comment_reply_key(key),
            ActiveView::CommentList => self.handle_comment_list_key(key),
            ActiveView::ThreadView => self.handle_thread_view_key(key),
            ActiveView::PrPicker => self.handle_pr_picker_key(key),
            ActiveView::WorktreePicker => self.handle_worktree_picker_key(key),
            ActiveView::WorktreeConfirmGuard => self.handle_worktree_confirm_guard_key(key),
            ActiveView::Help => self.handle_help_key(key),
            ActiveView::Search => self.handle_search_key(key),
            ActiveView::CommandLine => self.handle_command_line_key(key),
            ActiveView::CommitModal => self.handle_commit_modal_key(key),
            ActiveView::AuditReport => self.handle_audit_report_key(key),
            ActiveView::AgentPrompt => self.handle_agent_prompt_key(key),
            ActiveView::MarkdownPreview => self.handle_markdown_preview_key(key),
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
                (']', KeyCode::Char(']')) => {
                    self.next_file();
                    self.pending_leader = Some(']');
                    return;
                }
                (']', KeyCode::Char('f')) => {
                    self.next_file();
                    return;
                }
                (']', KeyCode::Char('[')) => {
                    self.prev_file();
                    self.pending_leader = Some('[');
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
                ('[', KeyCode::Char('[')) => {
                    self.prev_file();
                    self.pending_leader = Some('[');
                    return;
                }
                ('[', KeyCode::Char('f')) => {
                    self.prev_file();
                    return;
                }
                ('[', KeyCode::Char(']')) => {
                    self.next_file();
                    self.pending_leader = Some(']');
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
                ('z', KeyCode::Char('a')) => {
                    self.toggle_fold_at_cursor();
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
                    if self.thread_at_cursor().is_some() {
                        self.toggle_resolve_thread_at_cursor();
                    } else {
                        self.toggle_resolve_current_comment();
                    }
                    return;
                }
                (' ', KeyCode::Char('d')) => {
                    self.delete_current_line_comment();
                    return;
                }
                (' ', KeyCode::Char('x')) => {
                    self.reject_hunk_and_reprompt();
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
                self.request_quit();
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
                if let Some(t_idx) = self.thread_at_cursor() {
                    self.start_thread_reply(t_idx);
                } else if let Some(idx) = self.comment_idx_at_cursor() {
                    self.start_reply_to_comment(idx);
                } else {
                    self.comment_input.clear();
                    self.active_view = ActiveView::CommentInput;
                }
            }
            (_, KeyCode::Char('R')) => {
                if self.thread_at_cursor().is_some() {
                    self.toggle_resolve_thread_at_cursor();
                } else {
                    self.toggle_resolve_current_comment();
                }
            }
            (_, KeyCode::Char('A')) => {
                self.approve_gate();
            }
            (_, KeyCode::Char('P')) => {
                self.open_agent_prompt();
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
                self.status_message = Some(
                    "] (f/]: file, h: hunk, c: comment, w: alert, u: unreviewed, t: thread)".into(),
                );
            }
            (_, KeyCode::Char('[')) => {
                self.pending_leader = Some('[');
                self.status_message = Some(
                    "[ (f/[: file, h: hunk, c: comment, w: alert, u: unreviewed, t: thread)".into(),
                );
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
                let kind = self.resolved_sink_kind();
                match IpcBridge::list_panes(kind) {
                    Ok(panes) if !panes.is_empty() => {
                        self.pane_picker.list = panes;
                        self.pane_picker.selected_idx = 0;
                        self.active_view = ActiveView::PanePicker;
                    }
                    _ => {
                        self.status_message = Some(format!(
                            "No targets detected for {} sink (configure with :sink <name>)",
                            kind
                        ));
                    }
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
                self.command.reset();
                self.active_view = ActiveView::CommandLine;
            }
            (_, KeyCode::Char('m')) => {
                self.toggle_markdown_preview();
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
                self.command.reset();
                self.active_view = ActiveView::Diff;
            }
            (_, KeyCode::Left) => {
                self.command.move_left();
            }
            (_, KeyCode::Right) => {
                self.command.move_right();
            }
            (_, KeyCode::Home) | (KeyModifiers::CONTROL, KeyCode::Char('a')) => {
                self.command.move_home();
            }
            (_, KeyCode::End) | (KeyModifiers::CONTROL, KeyCode::Char('e')) => {
                self.command.move_end();
            }
            (KeyModifiers::CONTROL, KeyCode::Char('w'))
            | (KeyModifiers::ALT, KeyCode::Backspace) => {
                self.command.completions.clear();
                self.command.completion_idx = None;
                self.command.prefix.clear();
                self.command.delete_word_backward();
            }
            (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
                self.command.completions.clear();
                self.command.completion_idx = None;
                self.command.prefix.clear();
                self.command.delete_to_start();
            }
            (KeyModifiers::CONTROL, KeyCode::Char('k')) => {
                self.command.completions.clear();
                self.command.completion_idx = None;
                self.command.prefix.clear();
                self.command.delete_to_end();
            }
            (_, KeyCode::Delete) | (KeyModifiers::CONTROL, KeyCode::Char('d')) => {
                self.command.completions.clear();
                self.command.completion_idx = None;
                self.command.prefix.clear();
                self.command.delete_forward();
            }
            (_, KeyCode::Backspace) => {
                self.command.completions.clear();
                self.command.completion_idx = None;
                self.command.prefix.clear();
                if !self.command.delete_backward() {
                    self.active_view = ActiveView::Diff;
                }
            }
            (_, KeyCode::Up) if !self.command.completions.is_empty() => {
                self.command.autocomplete_prev();
            }
            (_, KeyCode::Down) if !self.command.completions.is_empty() => {
                self.command.autocomplete_next();
            }
            (_, KeyCode::Up) | (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
                if !self.command.history.is_empty() {
                    let next_idx = match self.command.history_idx {
                        Some(0) => 0,
                        Some(i) => i - 1,
                        None => self.command.history.len() - 1,
                    };
                    self.command.history_idx = Some(next_idx);
                    self.command
                        .set_text(self.command.history[next_idx].clone());
                    self.command.completions.clear();
                    self.command.completion_idx = None;
                }
            }
            (_, KeyCode::Down) | (KeyModifiers::CONTROL, KeyCode::Char('n')) => {
                if let Some(i) = self.command.history_idx {
                    if i + 1 < self.command.history.len() {
                        let next_idx = i + 1;
                        self.command.history_idx = Some(next_idx);
                        self.command
                            .set_text(self.command.history[next_idx].clone());
                    } else {
                        self.command.history_idx = None;
                        self.command.set_text(String::new());
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
                self.command.insert_char(c);
            }
            _ => {}
        }
    }

    pub(crate) fn handle_comment_list_key(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) | (_, KeyCode::Char('q')) => {
                self.leave_directive_modal();
            }
            (_, KeyCode::Enter) => {
                if let Some(comment) = self.comments.get(self.comment_list_selected_idx) {
                    let file_path = comment.file_path.clone();
                    let target_line = comment.target_line_no;
                    self.jump_to_file_and_line(&file_path, target_line);
                }
                self.leave_directive_modal();
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
                    self.leave_directive_modal();
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
                self.draft_resolve(self.thread_selected_idx, None);
            }
            (_, KeyCode::Char('r')) if !self.threads.is_empty() => {
                self.start_thread_reply(self.thread_selected_idx);
            }
            (_, KeyCode::Char('s')) => self.thread_command(&["submit"]),
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
            (_, KeyCode::Char('1')) => self.open_pr_picker_state(crate::pr::PullRequestState::Open),
            (_, KeyCode::Char('2')) => {
                self.open_pr_picker_state(crate::pr::PullRequestState::Closed)
            }
            (_, KeyCode::Char('3')) => self.open_pr_picker_state(crate::pr::PullRequestState::All),
            (_, KeyCode::Enter) => {
                if let Some(pr) = self.pr.list.get(self.pr.selected_idx) {
                    self.load_pr_async(pr.number);
                }
                self.active_view = ActiveView::Diff;
            }
            _ => {}
        }
    }

    pub(crate) fn handle_worktree_picker_key(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) | (_, KeyCode::Char('q')) => {
                self.active_view = ActiveView::Diff;
            }
            (_, KeyCode::Up) | (_, KeyCode::Char('k')) => {
                if self.worktree_picker.selected_idx > 0 {
                    self.worktree_picker.selected_idx -= 1;
                }
            }
            (_, KeyCode::Down) | (_, KeyCode::Char('j')) => {
                if !self.worktree_picker.list.is_empty()
                    && self.worktree_picker.selected_idx < self.worktree_picker.list.len() - 1
                {
                    self.worktree_picker.selected_idx += 1;
                }
            }
            (_, KeyCode::Enter) => {
                if let Some(target) = self
                    .worktree_picker
                    .list
                    .get(self.worktree_picker.selected_idx)
                    .cloned()
                {
                    if target.is_bare {
                        self.status_message = Some("Cannot switch to bare worktree".to_string());
                    } else {
                        self.initiate_worktree_switch(target.path);
                    }
                }
            }
            _ => {}
        }
    }

    pub(crate) fn handle_worktree_confirm_guard_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('s') | KeyCode::Char('S') => {
                let _ = self.comment_store.save_comments(&self.comments);
                if let Some(target) = self.worktree_picker.pending_target.take() {
                    self.apply_worktree_switch(target);
                } else {
                    self.active_view = ActiveView::Diff;
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                self.comments.clear();
                if let Some(target) = self.worktree_picker.pending_target.take() {
                    self.apply_worktree_switch(target);
                } else {
                    self.active_view = ActiveView::Diff;
                }
            }
            KeyCode::Esc | KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Char('q') => {
                self.worktree_picker.pending_target = None;
                self.active_view = ActiveView::Diff;
                self.status_message = Some("Worktree switch aborted".to_string());
            }
            _ => {}
        }
    }

    pub fn initiate_worktree_switch(&mut self, target_path: std::path::PathBuf) {
        let has_pending = self.comments.iter().any(|c| c.track.outcome.is_none());
        if has_pending {
            self.worktree_picker.pending_target = Some(target_path);
            self.active_view = ActiveView::WorktreeConfirmGuard;
        } else {
            self.apply_worktree_switch(target_path);
        }
    }

    pub fn apply_worktree_switch(&mut self, target_path: std::path::PathBuf) {
        self.pr.current_number = None;
        self.active_revision = None;
        self.git_engine = crate::git::GitEngine::new(&target_path);
        self.comment_store = lgtm_core::comment::CommentStore::new(&target_path);
        self.comments = self.comment_store.load_comments();
        self.marks.clear();
        self.reload_diff();
        self.cursor_to_top();
        self.active_view = ActiveView::Diff;
        self.status_message = Some(format!(
            "Switched to worktree: {}",
            target_path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| target_path.display().to_string())
        ));
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
                            if let Err(error) = self.comment_store.save_comments(&self.comments) {
                                self.comments[idx].replies.pop();
                                self.status_message =
                                    Some(format!("Reply not saved: {error}; draft retained"));
                                return;
                            }
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
                self.comment_target_alert = None;
                self.active_view = self.comment_source_view;
            }
            KeyCode::Enter => {
                if !self.comment_input.trim().is_empty() {
                    if !self.save_current_line_comment() {
                        return;
                    }
                } else {
                    self.comment_target_alert = None;
                }
                self.active_view = self.comment_source_view;
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
                self.request_quit();
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
            (_, KeyCode::Char('c')) => {
                if let Some(alert) = filtered.get(self.audit.selected_idx) {
                    self.comment_target_alert = Some((*alert).clone());
                    self.comment_source_view = ActiveView::AuditReport;
                    self.comment_input.clear();
                    self.active_view = ActiveView::CommentInput;
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

    /// Whether the terminal should report mouse events (only the preview uses them).
    pub(crate) fn wants_mouse_capture(&self) -> bool {
        self.active_view == ActiveView::MarkdownPreview
    }

    pub(crate) fn handle_mouse(&mut self, mouse: crossterm::event::MouseEvent) {
        use crossterm::event::MouseEventKind;
        if self.active_view != ActiveView::MarkdownPreview {
            return;
        }
        let vh = self.markdown_preview.viewport_height;
        match mouse.kind {
            MouseEventKind::ScrollDown => self.markdown_preview.scroll_down(3, vh),
            MouseEventKind::ScrollUp => self.markdown_preview.scroll_up(3, vh),
            _ => {}
        }
    }

    pub(crate) fn handle_markdown_preview_key(&mut self, key: KeyEvent) {
        let vh = self.markdown_preview.viewport_height;
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Char('m')) | (KeyModifiers::NONE, KeyCode::Esc) => {
                self.comment_source_view = ActiveView::Diff;
                self.active_view = ActiveView::Diff;
            }
            (_, KeyCode::Tab) | (_, KeyCode::BackTab) => {
                self.show_file_drawer = !self.show_file_drawer;
            }
            (KeyModifiers::NONE, KeyCode::Char('c')) => {
                if self.sync_cursor_to_markdown_row() {
                    self.comment_input.clear();
                    self.comment_source_view = ActiveView::MarkdownPreview;
                    self.active_view = ActiveView::CommentInput;
                }
            }
            (KeyModifiers::NONE, KeyCode::Char('l')) => {
                self.sync_cursor_to_markdown_row();
                self.comment_source_view = ActiveView::MarkdownPreview;
                self.open_comment_list();
            }
            (KeyModifiers::NONE, KeyCode::Char('R'))
            | (KeyModifiers::SHIFT, KeyCode::Char('R')) => {
                self.sync_cursor_to_markdown_row();
                self.toggle_resolve_current_comment();
            }
            (KeyModifiers::NONE, KeyCode::Char('x')) => {
                self.sync_cursor_to_markdown_row();
                self.delete_current_line_comment();
            }
            (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
                self.markdown_preview.move_cursor(1, vh);
            }
            (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
                self.markdown_preview.move_cursor(-1, vh);
            }
            (KeyModifiers::CONTROL, KeyCode::Char('d'))
            | (KeyModifiers::NONE, KeyCode::PageDown) => {
                self.markdown_preview.move_cursor(15, vh);
            }
            (KeyModifiers::CONTROL, KeyCode::Char('u')) | (KeyModifiers::NONE, KeyCode::PageUp) => {
                self.markdown_preview.move_cursor(-15, vh);
            }
            (KeyModifiers::NONE, KeyCode::Char('g')) | (KeyModifiers::NONE, KeyCode::Home) => {
                self.markdown_preview.cursor_to_top(vh);
            }
            (KeyModifiers::NONE, KeyCode::Char('G')) | (KeyModifiers::NONE, KeyCode::End) => {
                self.markdown_preview.cursor_to_bottom(vh);
            }
            (KeyModifiers::NONE, KeyCode::Char(':')) => {
                self.command.input.clear();
                self.command.history_idx = None;
                self.active_view = ActiveView::CommandLine;
            }
            (KeyModifiers::NONE, KeyCode::Char('q')) => {
                self.should_quit = true;
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

    #[test]
    fn test_command_line_key_dispatch() {
        use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
        let mut app = create_test_app();
        let make_key = |code: KeyCode, modifiers: KeyModifiers| KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };

        // Open command line via ':'
        app.active_view = ActiveView::Diff;
        app.handle_key(make_key(KeyCode::Char(':'), KeyModifiers::NONE));
        assert_eq!(app.active_view, ActiveView::CommandLine);
        assert_eq!(app.command.input, "");
        assert_eq!(app.command.cursor, 0);

        // Typing "diff main"
        for c in "diff main".chars() {
            app.handle_key(make_key(KeyCode::Char(c), KeyModifiers::NONE));
        }
        assert_eq!(app.command.input, "diff main");
        assert_eq!(app.command.cursor, 9);

        // Cursor movement
        app.handle_key(make_key(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(app.command.cursor, 8);
        app.handle_key(make_key(KeyCode::Char('a'), KeyModifiers::CONTROL));
        assert_eq!(app.command.cursor, 0);
        app.handle_key(make_key(KeyCode::Char('e'), KeyModifiers::CONTROL));
        assert_eq!(app.command.cursor, 9);

        // Ctrl-w word deletion
        app.handle_key(make_key(KeyCode::Char('w'), KeyModifiers::CONTROL));
        assert_eq!(app.command.input, "diff ");
        assert_eq!(app.command.cursor, 5);

        // Alt-Backspace word deletion
        app.handle_key(make_key(KeyCode::Backspace, KeyModifiers::ALT));
        assert_eq!(app.command.input, "");
        assert_eq!(app.command.cursor, 0);

        // Type "help"
        for c in "help".chars() {
            app.handle_key(make_key(KeyCode::Char(c), KeyModifiers::NONE));
        }
        assert_eq!(app.command.input, "help");
        assert_eq!(app.command.cursor, 4);

        // Move to index 2 (between e and l)
        app.handle_key(make_key(KeyCode::Left, KeyModifiers::NONE));
        app.handle_key(make_key(KeyCode::Left, KeyModifiers::NONE));
        assert_eq!(app.command.cursor, 2);

        // Backspace deletes 'e'
        app.handle_key(make_key(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(app.command.input, "hlp");
        assert_eq!(app.command.cursor, 1);

        // Delete forward with Ctrl-d deletes 'l'
        app.handle_key(make_key(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert_eq!(app.command.input, "hp");
        assert_eq!(app.command.cursor, 1);

        // Delete with Delete key deletes 'p'
        app.handle_key(make_key(KeyCode::Delete, KeyModifiers::NONE));
        assert_eq!(app.command.input, "h");
        assert_eq!(app.command.cursor, 1);

        // Backspace deletes 'h'
        app.handle_key(make_key(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(app.command.input, "");
        assert_eq!(app.command.cursor, 0);

        // Backspace on empty string exits to Diff
        app.handle_key(make_key(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(app.active_view, ActiveView::Diff);

        // Re-enter command line, test Ctrl-u and Ctrl-k
        app.handle_key(make_key(KeyCode::Char(':'), KeyModifiers::NONE));
        for c in "foo bar".chars() {
            app.handle_key(make_key(KeyCode::Char(c), KeyModifiers::NONE));
        }
        app.command.cursor = 3;
        app.handle_key(make_key(KeyCode::Char('k'), KeyModifiers::CONTROL));
        assert_eq!(app.command.input, "foo");
        assert_eq!(app.command.cursor, 3);

        app.handle_key(make_key(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert_eq!(app.command.input, "");
        assert_eq!(app.command.cursor, 0);

        // History navigation
        app.command.history = vec!["audit".into(), "export patch".into()];
        app.handle_key(make_key(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(app.command.input, "export patch");
        assert_eq!(app.command.cursor, 12);
        app.handle_key(make_key(KeyCode::Char('p'), KeyModifiers::CONTROL));
        assert_eq!(app.command.input, "audit");
        assert_eq!(app.command.cursor, 5);
        app.handle_key(make_key(KeyCode::Char('n'), KeyModifiers::CONTROL));
        assert_eq!(app.command.input, "export patch");
        assert_eq!(app.command.cursor, 12);
        app.handle_key(make_key(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.command.input, "");
        assert_eq!(app.command.cursor, 0);

        // Ctrl-c clears and exits
        app.command.set_text("cancelled".into());
        app.handle_key(make_key(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert_eq!(app.active_view, ActiveView::Diff);
        assert_eq!(app.command.input, "");
        assert_eq!(app.command.cursor, 0);
    }

    #[test]
    fn test_markdown_preview_toggle_and_guard() {
        use crossterm::event::{KeyEventKind, KeyEventState};
        let make_key = |code: KeyCode| KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        let mut app = create_test_app();
        app.snapshot.files = vec![
            FileDiff {
                old_path: None,
                new_path: PathBuf::from("src/main.rs"),
                change_type: ChangeType::Added,
                additions: 10,
                deletions: 0,
                hunks: Vec::new(),
                is_folded: false,
                is_reviewed: false,
            },
            FileDiff {
                old_path: None,
                new_path: PathBuf::from("README.md"),
                change_type: ChangeType::Added,
                additions: 20,
                deletions: 0,
                hunks: Vec::new(),
                is_folded: false,
                is_reviewed: false,
            },
        ];

        // On non-markdown file: toggle does not enter preview, sets warning
        app.selected_file_idx = 0;
        app.handle_key(make_key(KeyCode::Char('m')));
        assert_eq!(app.active_view, ActiveView::Diff);
        assert_eq!(
            app.status_message.as_deref(),
            Some("Markdown preview only available for .md files")
        );

        // On markdown file: toggle enters preview
        app.selected_file_idx = 1;
        app.handle_key(make_key(KeyCode::Char('m')));
        assert_eq!(app.active_view, ActiveView::MarkdownPreview);

        // Navigation in preview
        app.handle_key(make_key(KeyCode::Char('j')));
        app.handle_key(make_key(KeyCode::Char('k')));
        app.handle_key(make_key(KeyCode::Char('g')));
        app.handle_key(make_key(KeyCode::Char('G')));

        // Toggle back returns to Diff
        app.handle_key(make_key(KeyCode::Char('m')));
        assert_eq!(app.active_view, ActiveView::Diff);

        // Esc returns from preview to Diff
        app.selected_file_idx = 1;
        app.handle_key(make_key(KeyCode::Char('m')));
        assert_eq!(app.active_view, ActiveView::MarkdownPreview);
        app.handle_key(make_key(KeyCode::Esc));
        assert_eq!(app.active_view, ActiveView::Diff);

        // Colon command :md toggles preview
        app.execute_command("md");
        assert_eq!(app.active_view, ActiveView::MarkdownPreview);

        // Colon command :preview toggles back
        app.execute_command("preview");
        assert_eq!(app.active_view, ActiveView::Diff);
    }

    #[test]
    fn arrows_walk_the_tab_completion_list() {
        use crossterm::event::{KeyEventKind, KeyEventState};
        let key = |code: KeyCode| KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        let mut app = create_test_app();
        app.active_view = ActiveView::CommandLine;
        app.command.history.push("older".into());
        app.handle_key(key(KeyCode::Char('e')));
        app.handle_key(key(KeyCode::Tab));
        let first = app.command.input.clone();
        let n = app.command.completions.len();
        assert!(n > 1);
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.command.completion_idx, Some(1));
        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.command.input, first);
        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.command.completion_idx, Some(n - 1));
        assert_ne!(app.command.input, "older");
    }

    #[test]
    fn preview_comment_targets_cursor_row_and_wheel_scrolls_only_in_preview() {
        use crossterm::event::{MouseEvent, MouseEventKind};
        let wheel = |kind| MouseEvent {
            kind,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::empty(),
        };
        let mut app = create_test_app();
        app.handle_mouse(wheel(MouseEventKind::ScrollDown));
        assert!(!app.wants_mouse_capture());
        assert_eq!(app.markdown_preview.scroll_offset, 0);

        app.active_view = ActiveView::MarkdownPreview;
        app.markdown_preview.viewport_height = 10;
        app.markdown_preview.rendered_rows = (0..50)
            .map(|i| crate::app::markdown::RenderedMarkdownRow {
                line_number: Some(i + 1),
                diff_kind: crate::app::markdown::MarkdownDiffKind::Unchanged,
                spans: vec![],
            })
            .collect();
        app.handle_mouse(wheel(MouseEventKind::ScrollDown));
        assert_eq!(app.markdown_preview.scroll_offset, 3);
        app.handle_mouse(wheel(MouseEventKind::ScrollUp));
        app.handle_mouse(wheel(MouseEventKind::ScrollUp));
        assert_eq!(app.markdown_preview.scroll_offset, 0);
    }

    #[test]
    fn markdown_preview_follows_tab_and_manages_directives() {
        use crossterm::event::{KeyEventKind, KeyEventState};
        let key = |code: KeyCode| KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        let line = |t: LineType, old: Option<u32>, new: Option<u32>| DiffLine {
            line_type: t,
            content: "text".into(),
            old_line_num: old,
            new_line_num: new,
        };
        let mut app = create_test_app();
        app.snapshot.files = vec![FileDiff {
            old_path: None,
            new_path: PathBuf::from("README.md"),
            change_type: ChangeType::Modified,
            additions: 1,
            deletions: 1,
            hunks: vec![DiffHunk {
                header: "@@".into(),
                lines: vec![
                    line(LineType::HunkHeader, None, None),
                    line(LineType::Deletion, Some(5), None),
                    line(LineType::Addition, None, Some(5)),
                ],
                old_start: 5,
                old_lines: 1,
                new_start: 5,
                new_lines: 1,
                is_folded: false,
                folded_blocks: Vec::new(),
            }],
            is_folded: false,
            is_reviewed: false,
        }];
        app.selected_file_idx = 0;
        app.handle_key(key(KeyCode::Char('m')));
        app.markdown_preview.rendered_rows =
            crate::app::markdown::build_markdown_preview(&app.snapshot.files[0], 80);

        let drawer = app.show_file_drawer;
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.show_file_drawer, !drawer);
        app.handle_key(key(KeyCode::BackTab));
        assert_eq!(app.show_file_drawer, drawer);

        // Cursor row 0 is the deleted line (diff-line index 1); `j` moves it to the addition.
        app.handle_key(key(KeyCode::Char('j')));
        app.handle_key(key(KeyCode::Char('k')));
        assert!(app.wants_mouse_capture());
        app.handle_key(key(KeyCode::Char('c')));
        assert_eq!(app.active_view, ActiveView::CommentInput);
        assert_eq!(app.cursor_line_idx, 1);
        for c in "fix".chars() {
            app.handle_key(key(KeyCode::Char(c)));
        }
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.active_view, ActiveView::MarkdownPreview);
        assert_eq!(app.comments.len(), 1);
        assert_eq!(app.comments[0].comment_text, "fix");

        app.handle_key(key(KeyCode::Char('R')));
        assert!(app.comments[0].is_resolved);

        app.handle_key(key(KeyCode::Char('l')));
        assert_eq!(app.active_view, ActiveView::CommentList);
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.active_view, ActiveView::MarkdownPreview);

        app.handle_key(key(KeyCode::Char('x')));
        assert!(app.comments.is_empty());

        // Leaving the preview resets the modal return view.
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.active_view, ActiveView::Diff);
        assert_eq!(app.comment_source_view, ActiveView::Diff);
    }

    #[test]
    fn test_capital_x_triggers_file_discard_not_reprompt() {
        use crossterm::event::{KeyEventKind, KeyEventState};
        let mut app = create_test_app();
        let key = KeyEvent {
            code: KeyCode::Char('X'),
            modifiers: KeyModifiers::SHIFT,
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        app.handle_key(key);
        // Ensure active_view did not open CommentInput (which was previously triggered by reject prompt)
        assert_ne!(app.active_view, ActiveView::CommentInput);
    }

    #[test]
    fn test_space_x_triggers_reject_and_reprompt() {
        use crossterm::event::{KeyEventKind, KeyEventState};
        let mut app = create_test_app();
        let make_key = |code: KeyCode| KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::empty(),
        };
        app.handle_key(make_key(KeyCode::Char(' ')));
        assert_eq!(app.pending_leader, Some(' '));
        app.handle_key(make_key(KeyCode::Char('x')));
        assert_eq!(app.pending_leader, None);
        assert!(app.status_message.is_some() || app.active_view == ActiveView::AgentPrompt);
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    fn press(app: &mut App, keys: &str) {
        for c in keys.chars() {
            app.handle_key(key(KeyCode::Char(c)));
        }
    }

    fn dl(t: LineType, content: &str, new: Option<u32>, old: Option<u32>) -> DiffLine {
        DiffLine {
            line_type: t,
            content: content.into(),
            old_line_num: old,
            new_line_num: new,
        }
    }

    fn hunk(start: u32) -> DiffHunk {
        DiffHunk {
            header: format!("@@ -{start},2 +{start},3 @@"),
            old_start: start,
            old_lines: 2,
            new_start: start,
            new_lines: 3,
            lines: vec![
                dl(LineType::Context, "fn a() {", Some(start), Some(start)),
                dl(LineType::Addition, "    let v = 1;", Some(start + 1), None),
                dl(LineType::Context, "}", Some(start + 2), Some(start + 1)),
            ],
            is_folded: false,
            folded_blocks: Vec::new(),
        }
    }

    fn diff_app() -> App {
        let mut app = create_test_app();
        let files = ["f0.rs", "f1.rs", "f2.rs"]
            .iter()
            .map(|name| FileDiff {
                old_path: None,
                new_path: PathBuf::from(name),
                change_type: ChangeType::Modified,
                additions: 2,
                deletions: 0,
                hunks: vec![hunk(1), hunk(20)],
                is_folded: false,
                is_reviewed: false,
            })
            .collect();
        app.snapshot = DiffSnapshot::new(files);
        app.selected_file_idx = 0;
        app.cursor_line_idx = 0;
        app
    }

    fn comment_at(file: &str, line: u32, text: &str) -> lgtm_core::comment::CommentAnchor {
        lgtm_core::comment::CommentAnchor {
            id: lgtm_core::comment::new_id(),
            file_path: file.into(),
            target_line_no: line,
            target_code: "x".into(),
            context_before: vec![],
            context_after: vec![],
            comment_text: text.into(),
            replies: vec![],
            created_at_turn: 1,
            is_resolved: false,
            pr_number: None,
            orphaned: false,
            track: Default::default(),
        }
    }

    fn alert_at(file: &str, line: u32, sev: crate::guard::Severity) -> crate::guard::GuardAlert {
        crate::guard::GuardAlert {
            rule_id: "r/x".into(),
            fingerprint: format!("{file}{line}"),
            severity: sev,
            title: "Title".into(),
            description: "Desc".into(),
            line_no: Some(line),
            file_path: file.into(),
        }
    }

    #[test]
    fn audit_report_keys_navigate_filter_and_exit() {
        use crate::guard::Severity;
        let mut app = diff_app();
        app.audit.has_result = true;
        app.audit.alerts = vec![
            alert_at("f0.rs", 1, Severity::High),
            alert_at("f1.rs", 2, Severity::Critical),
            alert_at("f2.rs", 3, Severity::Warn),
        ];
        app.active_view = ActiveView::AuditReport;
        app.audit.focus = AuditPaneFocus::Issues;
        app.audit.selected_idx = 0;
        assert_eq!(app.filtered_audit_alerts().len(), 3);

        // `]` jumps to the next file immediately; `c` then advances one issue
        press(&mut app, "]c");
        assert_eq!(app.audit.selected_idx, 2);
        press(&mut app, "[c");
        assert_eq!(app.audit.selected_idx, 0);
        app.audit.selected_idx = 2;
        press(&mut app, "gg");
        assert_eq!(app.audit.selected_idx, 0);
        press(&mut app, "]f");
        press(&mut app, "[f");

        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.audit.focus, AuditPaneFocus::Inspector);
        press(&mut app, "gg");
        assert_eq!(app.audit.inspector_scroll, 0);
        app.handle_key(key(KeyCode::BackTab));
        assert_eq!(app.audit.focus, AuditPaneFocus::Issues);

        let shown = app.show_file_drawer;
        press(&mut app, "b");
        assert_eq!(app.show_file_drawer, !shown);

        app.audit.maximized_inspector = true;
        app.handle_key(key(KeyCode::Esc));
        assert!(!app.audit.maximized_inspector);
        app.audit.file_filter = Some("f1.rs".to_string());
        app.handle_key(key(KeyCode::Esc));
        assert!(app.audit.file_filter.is_none());
        assert_eq!(
            app.status_message.as_deref(),
            Some("Cleared file filter: showing all issues")
        );
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.active_view, ActiveView::Diff);
    }

    #[test]
    fn diff_file_and_hunk_leaders_navigate() {
        let mut app = diff_app();
        press(&mut app, "]f");
        assert_eq!(app.selected_file_idx, 1);
        press(&mut app, "[f");
        assert_eq!(app.selected_file_idx, 0);

        press(&mut app, "]]");
        assert_eq!((app.selected_file_idx, app.pending_leader), (1, Some(']')));
        press(&mut app, "[");
        assert_eq!((app.selected_file_idx, app.pending_leader), (0, Some('[')));
        press(&mut app, "[");
        assert_eq!((app.selected_file_idx, app.pending_leader), (0, Some('[')));
        press(&mut app, "]");
        assert_eq!((app.selected_file_idx, app.pending_leader), (1, Some(']')));
        app.pending_leader = None;
        press(&mut app, "[[");
        assert_eq!((app.selected_file_idx, app.pending_leader), (0, Some('[')));
        app.pending_leader = None;

        app.snapshot.files[1].is_reviewed = true;
        press(&mut app, "]u");
        assert_eq!(app.selected_file_idx, 2);
        press(&mut app, "[u");
        assert_eq!(app.selected_file_idx, 0);

        app.cursor_line_idx = 0;
        press(&mut app, "]h");
        assert_eq!(app.cursor_line_idx, 3);
        press(&mut app, "[h");
        assert_eq!(app.cursor_line_idx, 0);
        assert_eq!(app.pending_leader, None);
    }

    #[test]
    fn diff_comment_alert_and_thread_leaders() {
        let mut app = diff_app();
        app.comments.push(comment_at("f0.rs", 21, "later"));
        press(&mut app, "]c");
        assert_eq!(app.cursor_line_idx, 4);
        app.cursor_line_idx = 5;
        press(&mut app, "[c");
        assert_eq!(app.cursor_line_idx, 4);

        app.alerts
            .push(alert_at("f0.rs", 21, crate::guard::Severity::High));
        app.cursor_line_idx = 0;
        press(&mut app, "]w");
        assert_eq!(app.cursor_line_idx, 4);
        app.cursor_line_idx = 5;
        press(&mut app, "[w");
        assert_eq!(app.cursor_line_idx, 4);
        app.cursor_line_idx = 0;
        press(&mut app, " w");
        assert_eq!(app.cursor_line_idx, 4);

        press(&mut app, "]t");
        assert_eq!(
            app.status_message.as_deref(),
            Some("No threads; run :threads first")
        );
        app.status_message = None;
        press(&mut app, "[t");
        assert_eq!(
            app.status_message.as_deref(),
            Some("No threads; run :threads first")
        );
    }

    #[test]
    fn diff_view_and_fold_leaders() {
        let mut app = diff_app();
        app.cursor_line_idx = 40;
        press(&mut app, "zz");
        assert_eq!(app.diff_scroll_offset, 25);
        press(&mut app, "zt");
        assert_eq!(app.diff_scroll_offset, 40);
        press(&mut app, "zb");
        assert_eq!(app.diff_scroll_offset, 10);

        app.cursor_line_idx = 0;
        press(&mut app, "za");
        assert_eq!(
            app.status_message.as_deref(),
            Some("Folded AST block (L1-L3)")
        );
        press(&mut app, "zo");
        assert_eq!(
            app.status_message.as_deref(),
            Some("Unfolded AST block (L1-L3)")
        );
        press(&mut app, "zc");
        assert_eq!(
            app.status_message.as_deref(),
            Some("Folded AST block (L1-L3)")
        );
        press(&mut app, "zM");
        assert!(app
            .snapshot
            .files
            .iter()
            .flat_map(|f| &f.hunks)
            .all(|h| h.is_folded));
        press(&mut app, "zR");
        assert!(app
            .snapshot
            .files
            .iter()
            .flat_map(|f| &f.hunks)
            .all(|h| !h.is_folded && h.folded_blocks.is_empty()));
    }

    #[test]
    fn diff_comment_leaders_prefill_templates() {
        let cases = [
            ("ca", "[COVERAGE]"),
            ("cs", "[SECURITY]"),
            ("cr", "[REFACTOR]"),
            ("cp", "[PERF]"),
            ("cw", "[WHY]"),
        ];
        for (keys, tag) in cases {
            let mut app = diff_app();
            press(&mut app, keys);
            assert_eq!(app.active_view, ActiveView::CommentInput, "{keys}");
            assert!(
                app.comment_input.contains(tag),
                "{keys}: {}",
                app.comment_input
            );
        }
        for keys in ["cc", "cz", " c"] {
            let mut app = diff_app();
            app.comment_input = "stale".into();
            press(&mut app, keys);
            assert_eq!(app.active_view, ActiveView::CommentInput, "{keys}");
            assert!(app.comment_input.is_empty(), "{keys}");
        }
        let mut app = diff_app();
        press(&mut app, "c");
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.active_view, ActiveView::CommentInput);
    }

    #[test]
    fn diff_space_leaders() {
        let mut app = diff_app();
        press(&mut app, " y");
        assert_eq!(
            app.status_message.as_deref(),
            Some("Copied to clipboard: #ref f0.rs:1")
        );
        press(&mut app, " l");
        assert_eq!(app.active_view, ActiveView::CommentList);
        app.active_view = ActiveView::Diff;
        press(&mut app, " P");
        assert_eq!(app.active_view, ActiveView::PrPicker);
        app.active_view = ActiveView::Diff;
        press(&mut app, " t");
        assert_eq!(
            app.status_message.as_deref(),
            Some("Open a PR first (:pr <n>)")
        );

        press(&mut app, " d");
        assert_eq!(app.status_message.as_deref(), Some("No comment on f0.rs:1"));
        press(&mut app, " r");
        assert_eq!(
            app.status_message.as_deref(),
            Some("No directive on current line")
        );
        app.comments.push(comment_at("f0.rs", 1, "fix"));
        press(&mut app, " r");
        assert!(app.comments[0].is_resolved);
        press(&mut app, " R");
        assert!(!app.comments[0].is_resolved);
        press(&mut app, " d");
        assert!(app.comments.is_empty());

        press(&mut app, " x");
        assert_eq!(app.status_message.as_deref(), Some("Not a git repository"));
        // Pane picking depends on whether tmux is available; either outcome must leave the leader consumed.
        press(&mut app, " p");
        assert_eq!(app.pending_leader, None);
        app.active_view = ActiveView::Diff;

        press(&mut app, " s");
        assert!(app
            .status_message
            .as_deref()
            .unwrap()
            .starts_with("Exported .lgtm/review-"));
    }

    #[test]
    fn diff_single_keys_drive_state() {
        let mut app = diff_app();
        press(&mut app, "?");
        assert_eq!(app.active_view, ActiveView::Help);
        app.active_view = ActiveView::Diff;

        app.handle_key(ctrl('p'));
        assert_eq!(app.active_view, ActiveView::FilePicker);
        assert_eq!(app.file_picker.filtered_indices, vec![0, 1, 2]);
        app.active_view = ActiveView::Diff;
        press(&mut app, "f");
        assert_eq!(app.active_view, ActiveView::FilePicker);
        app.active_view = ActiveView::Diff;

        app.handle_key(ctrl('s'));
        assert!(app
            .status_message
            .as_deref()
            .unwrap()
            .starts_with("Exported"));

        let drawer = app.show_file_drawer;
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.show_file_drawer, !drawer);

        let split = app.is_split_view;
        press(&mut app, "d");
        assert_eq!(app.is_split_view, !split);
        assert_eq!(
            app.status_message.as_deref(),
            Some(if split {
                "View mode: Unified View"
            } else {
                "View mode: Split View"
            })
        );
        press(&mut app, "d");
        assert_eq!(app.is_split_view, split);

        press(&mut app, "G");
        assert_eq!(app.cursor_line_idx, 5);
        press(&mut app, "0");
        assert_eq!(app.cursor_line_idx, 0);
        press(&mut app, "$");
        assert_eq!(app.cursor_line_idx, 5);
        press(&mut app, "^");
        assert_eq!(app.cursor_line_idx, 0);
        press(&mut app, "j");
        assert_eq!(app.cursor_line_idx, 1);
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.cursor_line_idx, 2);
        press(&mut app, "k");
        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.cursor_line_idx, 0);
        press(&mut app, "}");
        assert_eq!(app.cursor_line_idx, 5);
        press(&mut app, "{");
        assert_eq!(app.cursor_line_idx, 0);
        app.handle_key(ctrl('d'));
        assert_eq!(app.cursor_line_idx, 5);
        app.handle_key(ctrl('u'));
        assert_eq!(app.cursor_line_idx, 0);
        app.handle_key(key(KeyCode::PageDown));
        assert_eq!(app.cursor_line_idx, 5);
        app.handle_key(key(KeyCode::PageUp));
        assert_eq!(app.cursor_line_idx, 0);
        app.handle_key(ctrl('f'));
        assert_eq!(app.cursor_line_idx, 5);
        app.handle_key(ctrl('b'));
        assert_eq!(app.cursor_line_idx, 0);

        press(&mut app, "y");
        assert_eq!(app.status_message.as_deref(), Some("Yanked: \"fn a() {\""));
        press(&mut app, "v");
        assert!(app.snapshot.files[0].is_reviewed);
        press(&mut app, "M");
        assert_eq!(app.status_message.as_deref(), Some("Hunk reviewed"));
        press(&mut app, "o");
        assert_eq!(
            app.status_message.as_deref(),
            Some("Opened f1.rs:1 in nvim")
        );
        press(&mut app, "e");
        assert_eq!(
            app.status_message.as_deref(),
            Some("Opened f1.rs:1 in nvim")
        );
        press(&mut app, "A");
        assert_eq!(
            app.status_message.as_deref(),
            Some("No active agent gate waiting.")
        );
        press(&mut app, "T");
        assert!(app
            .status_message
            .as_deref()
            .unwrap()
            .contains("No test neutering"));
        press(&mut app, "t");
        assert_eq!(
            app.status_message.as_deref(),
            Some("Open a PR first (:pr <n>)")
        );
        press(&mut app, "m");
        assert_eq!(
            app.status_message.as_deref(),
            Some("Markdown preview only available for .md files")
        );
        press(&mut app, "P");
        assert_eq!(app.active_view, ActiveView::AgentPrompt);
        app.active_view = ActiveView::Diff;
        press(&mut app, "p");
        assert_eq!(
            app.status_message.as_deref(),
            Some("No targets detected for dummy sink (configure with :sink <name>)")
        );
        app.handle_key(key(KeyCode::Enter));
        assert!(app
            .status_message
            .as_deref()
            .unwrap()
            .starts_with("Sent to"));
    }

    #[test]
    fn diff_pending_leader_keys_and_prompts() {
        let mut app = diff_app();
        for (k, leader) in [("g", 'g'), ("z", 'z'), (" ", ' '), ("c", 'c')] {
            press(&mut app, k);
            assert_eq!(app.pending_leader, Some(leader));
            app.pending_leader = None;
        }
        press(&mut app, "]");
        assert_eq!(app.pending_leader, Some(']'));
        assert!(app
            .status_message
            .as_deref()
            .unwrap()
            .starts_with("] (f/]: file"));
        press(&mut app, "x");
        press(&mut app, "[");
        assert_eq!(app.pending_leader, Some('['));
        assert!(app
            .status_message
            .as_deref()
            .unwrap()
            .starts_with("[ (f/[: file"));
        app.pending_leader = None;

        press(&mut app, "/");
        assert_eq!(app.active_view, ActiveView::Search);
        app.active_view = ActiveView::Diff;
        press(&mut app, ":");
        assert_eq!(app.active_view, ActiveView::CommandLine);
        app.active_view = ActiveView::Diff;
        press(&mut app, "C");
        assert_eq!(
            app.status_message.as_deref(),
            Some("⚠️ No staged changes to commit (stage with 's' or 'S')")
        );
        press(&mut app, "r");
        assert_eq!(app.active_view, ActiveView::CommentInput);
        app.active_view = ActiveView::Diff;
        press(&mut app, "R");
        assert_eq!(
            app.status_message.as_deref(),
            Some("No directive on current line")
        );

        app.comments.push(comment_at("f0.rs", 1, "hello"));
        press(&mut app, "r");
        assert_eq!(app.active_view, ActiveView::CommentReply);
        app.active_view = ActiveView::Diff;
        press(&mut app, "x");
        assert!(app.comments.is_empty());

        press(&mut app, "q");
        assert!(app.should_quit || app.status_message.is_some());
    }

    #[test]
    fn diff_search_and_symbol_keys() {
        let mut app = diff_app();
        press(&mut app, "n");
        assert_eq!(
            app.status_message.as_deref(),
            Some("No search pattern active (press / to search)")
        );
        press(&mut app, "N");
        assert_eq!(
            app.status_message.as_deref(),
            Some("No search pattern active (press / to search)")
        );
        press(&mut app, "*");
        assert_eq!(app.status_message.as_deref(), Some("Match 1/6 for 'fn'"));
        press(&mut app, "#");
        assert_eq!(app.status_message.as_deref(), Some("Match 6/6 for 'fn'"));
        assert_eq!((app.selected_file_idx, app.cursor_line_idx), (2, 3));
        press(&mut app, "n");
        assert_eq!((app.selected_file_idx, app.cursor_line_idx), (0, 0));
        press(&mut app, "N");
        assert_eq!((app.selected_file_idx, app.cursor_line_idx), (2, 3));
    }

    #[test]
    fn diff_visual_mode_and_git_keys_outside_repo() {
        let mut app = diff_app();
        press(&mut app, "V");
        assert!(app.visual_mode);
        app.handle_key(key(KeyCode::Esc));
        assert!(!app.visual_mode);
        app.handle_key(key(KeyCode::Esc));
        assert!(!app.visual_mode);

        for k in ["s", "u", "S", "U", "X", "x"] {
            app.status_message = None;
            press(&mut app, k);
            assert_eq!(
                app.status_message.as_deref(),
                Some("Not a git repository"),
                "{k}"
            );
        }
        press(&mut app, "V");
        for k in ["s", "u", "x"] {
            app.status_message = None;
            press(&mut app, k);
            assert_eq!(
                app.status_message.as_deref(),
                Some("Not a git repository"),
                "{k}"
            );
        }
    }

    #[test]
    fn search_key_edits_and_executes_query() {
        let mut app = diff_app();
        app.active_view = ActiveView::Search;
        press(&mut app, "lett");
        app.handle_key(key(KeyCode::Backspace));
        assert_eq!(app.search_query, "let");
        app.handle_key(key(KeyCode::Left));
        assert_eq!(app.search_query, "let");
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.active_view, ActiveView::Diff);
        assert_eq!(app.status_message.as_deref(), Some("Match 1/6 for 'let'"));
        assert_eq!(app.cursor_line_idx, 1);

        app.active_view = ActiveView::Search;
        app.search_query.clear();
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.active_view, ActiveView::Diff);
        app.active_view = ActiveView::Search;
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.active_view, ActiveView::Diff);
    }

    fn seeded_comments(app: &mut App) {
        app.comments = vec![
            comment_at("f0.rs", 1, "first"),
            comment_at("f1.rs", 21, "second"),
            comment_at("f2.rs", 2, "third"),
        ];
    }

    #[test]
    fn comment_list_navigation_and_resolution() {
        let mut app = diff_app();
        seeded_comments(&mut app);
        app.active_view = ActiveView::CommentList;
        press(&mut app, "j");
        assert_eq!(app.comment_list_selected_idx, 1);
        app.handle_key(key(KeyCode::Down));
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.comment_list_selected_idx, 2);
        press(&mut app, "k");
        app.handle_key(key(KeyCode::Up));
        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.comment_list_selected_idx, 0);
        app.handle_key(key(KeyCode::PageDown));
        assert_eq!(app.comment_list_selected_idx, 2);
        app.handle_key(key(KeyCode::PageUp));
        assert_eq!(app.comment_list_selected_idx, 0);
        app.handle_key(ctrl('d'));
        app.handle_key(ctrl('b'));
        assert_eq!(app.comment_list_selected_idx, 0);
        app.handle_key(ctrl('f'));
        app.handle_key(ctrl('u'));
        assert_eq!(app.comment_list_selected_idx, 0);

        for k in ['v', ' ', 'R'] {
            app.handle_key(key(KeyCode::Char(k)));
            assert!(app.comments[0].is_resolved, "{k}");
            assert_eq!(
                app.status_message.as_deref(),
                Some("✅ Directive marked resolved")
            );
            app.handle_key(key(KeyCode::Char(k)));
            assert!(!app.comments[0].is_resolved);
            assert_eq!(
                app.status_message.as_deref(),
                Some("💬 Directive marked active")
            );
        }
        assert_eq!(app.active_view, ActiveView::CommentList);
    }

    #[test]
    fn comment_list_actions() {
        let mut app = diff_app();
        seeded_comments(&mut app);
        app.active_view = ActiveView::CommentList;
        app.comment_list_selected_idx = 1;
        press(&mut app, "r");
        assert_eq!(app.active_view, ActiveView::CommentReply);
        assert_eq!(app.comment_reply_target_idx, Some(1));

        app.active_view = ActiveView::CommentList;
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.active_view, ActiveView::Diff);
        assert_eq!(app.selected_file_idx, 1);

        app.active_view = ActiveView::CommentList;
        app.comment_list_selected_idx = 2;
        press(&mut app, "d");
        assert_eq!(
            app.status_message.as_deref(),
            Some("Deleted comment at f2.rs:2")
        );
        assert_eq!(app.comments.len(), 2);
        assert_eq!(app.comment_list_selected_idx, 1);
        app.handle_key(key(KeyCode::Delete));
        press(&mut app, "x");
        assert!(app.comments.is_empty());
        press(&mut app, "x");
        press(&mut app, "v");
        press(&mut app, "a");
        press(&mut app, "r");
        press(&mut app, "j");
        app.handle_key(key(KeyCode::PageDown));
        assert_eq!(app.comment_list_selected_idx, 0);

        seeded_comments(&mut app);
        app.active_view = ActiveView::CommentList;
        press(&mut app, "a");
        assert_eq!(app.active_view, ActiveView::Diff);

        app.active_view = ActiveView::CommentList;
        app.handle_key(ctrl('s'));
        assert_eq!(app.active_view, ActiveView::Diff);
        assert!(app
            .status_message
            .as_deref()
            .unwrap()
            .starts_with("Exported"));

        for k in [KeyCode::Esc, KeyCode::Char('q')] {
            app.active_view = ActiveView::CommentList;
            app.handle_key(key(k));
            assert_eq!(app.active_view, ActiveView::Diff);
        }
    }

    fn thread(id: &str, path: &str, line: Option<u32>) -> lgtm_core::github::Thread {
        lgtm_core::github::Thread {
            id: id.into(),
            path: path.into(),
            line,
            resolved: false,
            comments: vec![],
        }
    }

    #[test]
    fn thread_view_keys() {
        let mut app = diff_app();
        app.threads = vec![thread("t1", "f0.rs", Some(1)), thread("t2", "f2.rs", None)];
        app.active_view = ActiveView::ThreadView;
        press(&mut app, "J");
        assert_eq!(app.thread_scroll_offset, 3);
        app.handle_key(key(KeyCode::PageDown));
        app.handle_key(ctrl('d'));
        assert_eq!(app.thread_scroll_offset, 9);
        press(&mut app, "K");
        app.handle_key(key(KeyCode::PageUp));
        app.handle_key(ctrl('u'));
        app.handle_key(key(KeyCode::PageUp));
        assert_eq!(app.thread_scroll_offset, 0);

        press(&mut app, "J");
        press(&mut app, "j");
        assert_eq!((app.thread_selected_idx, app.thread_scroll_offset), (1, 0));
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.thread_selected_idx, 1);
        press(&mut app, "J");
        press(&mut app, "k");
        assert_eq!((app.thread_selected_idx, app.thread_scroll_offset), (0, 0));
        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.thread_selected_idx, 0);

        press(&mut app, "v");
        assert_eq!(
            app.status_message.as_deref(),
            Some("Comments require a PR (:pr <n>)")
        );
        press(&mut app, "s");
        assert!(app.status_message.is_some());

        press(&mut app, "r");
        assert_eq!(app.active_view, ActiveView::CommandLine);
        assert_eq!(app.command.input, "thread reply 1 ");

        app.active_view = ActiveView::ThreadView;
        app.thread_selected_idx = 1;
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(
            (app.active_view, app.selected_file_idx),
            (ActiveView::Diff, 2)
        );

        app.active_view = ActiveView::ThreadView;
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.active_view, ActiveView::Diff);
        app.active_view = ActiveView::ThreadView;
        press(&mut app, "q");
        assert_eq!(app.active_view, ActiveView::Diff);

        app.threads.clear();
        app.active_view = ActiveView::ThreadView;
        press(&mut app, "j");
        press(&mut app, "r");
        assert_eq!(app.active_view, ActiveView::ThreadView);
    }

    fn pr_summary(n: u64) -> crate::pr::PullRequestSummary {
        crate::pr::PullRequestSummary {
            number: n,
            title: format!("pr {n}"),
            author: crate::pr::PrAuthor { login: "me".into() },
            head_ref_name: "b".into(),
            additions: 1,
            deletions: 1,
            state: "OPEN".into(),
        }
    }

    #[test]
    fn pr_picker_keys() {
        let mut app = diff_app();
        app.pr.list = (1..=8).map(pr_summary).collect();
        app.active_view = ActiveView::PrPicker;
        press(&mut app, "j");
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.pr.selected_idx, 2);
        press(&mut app, "k");
        app.handle_key(key(KeyCode::Up));
        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.pr.selected_idx, 0);
        app.handle_key(key(KeyCode::PageDown));
        assert_eq!(app.pr.selected_idx, 5);
        app.handle_key(ctrl('d'));
        assert_eq!(app.pr.selected_idx, 7);
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.pr.selected_idx, 7);
        app.handle_key(key(KeyCode::PageUp));
        assert_eq!(app.pr.selected_idx, 2);
        app.handle_key(ctrl('f'));
        assert_eq!(app.pr.selected_idx, 7);
        app.handle_key(ctrl('u'));
        app.handle_key(ctrl('b'));
        assert_eq!(app.pr.selected_idx, 0);

        app.pr.selected_idx = 3;
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.active_view, ActiveView::Diff);
        assert_eq!(app.pr_loading.map(|(n, _)| n), Some(4));

        for (k, state) in [
            ('1', crate::pr::PullRequestState::Open),
            ('2', crate::pr::PullRequestState::Closed),
            ('3', crate::pr::PullRequestState::All),
        ] {
            app.active_view = ActiveView::PrPicker;
            press(&mut app, &k.to_string());
            assert_eq!(app.pr.filter, state);
            assert!(app.pr.list.is_empty());
            assert_eq!(app.active_view, ActiveView::PrPicker);
        }
        press(&mut app, "q");
        assert_eq!(app.active_view, ActiveView::Diff);
        app.active_view = ActiveView::PrPicker;
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.active_view, ActiveView::Diff);
    }

    #[test]
    fn worktree_picker_enter_and_guard_keys() {
        use lgtm_core::git::WorktreeInfo;
        let mut app = diff_app();
        let target = app.git_engine.repo_path.join("elsewhere");
        std::fs::create_dir_all(&target).unwrap();
        app.worktree_picker.list = vec![
            WorktreeInfo {
                path: PathBuf::from("/bare"),
                head: "0".into(),
                branch: None,
                is_bare: true,
                is_current: false,
            },
            WorktreeInfo {
                path: target.clone(),
                head: "1".into(),
                branch: Some("x".into()),
                is_bare: false,
                is_current: false,
            },
        ];
        app.active_view = ActiveView::WorktreePicker;
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(
            app.status_message.as_deref(),
            Some("Cannot switch to bare worktree")
        );
        app.handle_key(key(KeyCode::Down));
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.worktree_picker.selected_idx, 1);
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(
            app.status_message.as_deref(),
            Some("Switched to worktree: elsewhere")
        );
        assert_eq!(app.git_engine.repo_path, target);
        assert_eq!(app.active_view, ActiveView::Diff);

        app.active_view = ActiveView::WorktreePicker;
        press(&mut app, "q");
        assert_eq!(app.active_view, ActiveView::Diff);
        app.active_view = ActiveView::WorktreeConfirmGuard;
        for k in ['s', 'S', 'd', 'D'] {
            app.worktree_picker.pending_target = None;
            app.active_view = ActiveView::WorktreeConfirmGuard;
            press(&mut app, &k.to_string());
            assert_eq!(app.active_view, ActiveView::Diff, "{k}");
        }
        app.worktree_picker.pending_target = Some(target.clone());
        app.comments = vec![comment_at("f0.rs", 1, "keep")];
        app.active_view = ActiveView::WorktreeConfirmGuard;
        press(&mut app, "s");
        assert_eq!(
            app.status_message.as_deref(),
            Some("Switched to worktree: elsewhere")
        );
        assert_eq!(app.comments.len(), 1);

        for k in [KeyCode::Esc, KeyCode::Char('C'), KeyCode::Char('q')] {
            app.worktree_picker.pending_target = Some(target.clone());
            app.active_view = ActiveView::WorktreeConfirmGuard;
            app.handle_key(key(k));
            assert_eq!(app.worktree_picker.pending_target, None);
            assert_eq!(
                app.status_message.as_deref(),
                Some("Worktree switch aborted")
            );
        }
        app.active_view = ActiveView::WorktreeConfirmGuard;
        press(&mut app, "z");
        assert_eq!(app.active_view, ActiveView::WorktreeConfirmGuard);
    }

    #[test]
    fn comment_reply_and_input_keys() {
        let mut app = diff_app();
        seeded_comments(&mut app);
        app.active_view = ActiveView::CommentReply;
        app.comment_reply_target_idx = Some(1);
        press(&mut app, "okk");
        app.handle_key(key(KeyCode::Backspace));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.comments[1].replies, vec!["ok".to_string()]);
        assert_eq!(
            app.status_message.as_deref(),
            Some("Reply added to comment on f1.rs:21")
        );
        assert!(app.comment_reply_input.is_empty());
        assert_eq!(app.active_view, ActiveView::Diff);

        app.active_view = ActiveView::CommentReply;
        app.comment_reply_target_idx = Some(0);
        press(&mut app, "  ");
        app.handle_key(key(KeyCode::Enter));
        assert!(app.comments[0].replies.is_empty());
        assert_eq!(app.comment_reply_target_idx, None);

        app.active_view = ActiveView::CommentReply;
        app.comment_reply_target_idx = Some(0);
        press(&mut app, "x");
        app.handle_key(key(KeyCode::Esc));
        assert!(app.comment_reply_input.is_empty());
        assert_eq!(app.comment_reply_target_idx, None);
        app.active_view = ActiveView::CommentReply;
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.active_view, ActiveView::CommentReply);

        let mut app = diff_app();
        app.comment_source_view = ActiveView::Diff;
        app.active_view = ActiveView::CommentInput;
        press(&mut app, "note!");
        app.handle_key(key(KeyCode::Backspace));
        assert_eq!(app.comment_input, "note");
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.active_view, ActiveView::Diff);
        assert_eq!(app.comments.len(), 1);
        assert_eq!(app.comments[0].comment_text, "note");

        app.active_view = ActiveView::CommentInput;
        app.comment_input = "  ".into();
        app.comment_target_alert = Some(alert_at("f0.rs", 1, crate::guard::Severity::High));
        app.handle_key(key(KeyCode::Enter));
        assert!(app.comment_target_alert.is_none());
        assert_eq!(app.comments.len(), 1);

        app.active_view = ActiveView::CommentInput;
        app.comment_source_view = ActiveView::AuditReport;
        app.comment_target_alert = Some(alert_at("f0.rs", 1, crate::guard::Severity::High));
        app.handle_key(key(KeyCode::Esc));
        assert!(app.comment_target_alert.is_none());
        assert_eq!(app.active_view, ActiveView::AuditReport);
        app.active_view = ActiveView::CommentInput;
        app.handle_key(key(KeyCode::Tab));
        assert_eq!(app.active_view, ActiveView::CommentInput);
    }

    #[test]
    fn help_tabs_wrap_and_jump() {
        let mut app = diff_app();
        app.active_view = ActiveView::Help;
        for k in [
            KeyCode::Tab,
            KeyCode::Right,
            KeyCode::Char('l'),
            KeyCode::Char(']'),
        ] {
            app.help_selected_tab = 6;
            app.handle_key(key(k));
            assert_eq!(app.help_selected_tab, 0, "{k:?}");
            app.handle_key(key(k));
            assert_eq!(app.help_selected_tab, 1);
        }
        for k in [
            KeyCode::BackTab,
            KeyCode::Left,
            KeyCode::Char('h'),
            KeyCode::Char('['),
        ] {
            app.help_selected_tab = 0;
            app.handle_key(key(k));
            assert_eq!(app.help_selected_tab, 6, "{k:?}");
            app.handle_key(key(k));
            assert_eq!(app.help_selected_tab, 5);
        }
        for (i, c) in "1234567".chars().enumerate() {
            app.handle_key(key(KeyCode::Char(c)));
            assert_eq!(app.help_selected_tab, i);
        }
        for k in [KeyCode::Esc, KeyCode::Char('?'), KeyCode::Char('q')] {
            app.active_view = ActiveView::Help;
            app.handle_key(key(k));
            assert_eq!(app.active_view, ActiveView::Diff);
        }
    }

    #[test]
    fn file_picker_filters_navigates_and_selects() {
        let mut app = diff_app();
        app.handle_key(ctrl('p'));
        assert_eq!(app.file_picker.filtered_indices.len(), 3);
        press(&mut app, "f2");
        assert_eq!(app.file_picker.filtered_indices, vec![2]);
        app.handle_key(key(KeyCode::Backspace));
        press(&mut app, "1");
        assert_eq!(app.file_picker.filtered_indices, vec![1]);
        app.handle_key(key(KeyCode::Backspace));
        app.handle_key(key(KeyCode::Backspace));
        assert_eq!(app.file_picker.filtered_indices.len(), 3);

        app.handle_key(key(KeyCode::Down));
        app.handle_key(ctrl('j'));
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.file_picker.selected_idx, 2);
        app.handle_key(key(KeyCode::Up));
        app.handle_key(ctrl('k'));
        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.file_picker.selected_idx, 0);
        app.handle_key(key(KeyCode::PageDown));
        assert_eq!(app.file_picker.selected_idx, 2);
        app.handle_key(key(KeyCode::PageUp));
        assert_eq!(app.file_picker.selected_idx, 0);
        app.handle_key(ctrl('d'));
        app.handle_key(ctrl('u'));
        app.handle_key(ctrl('f'));
        app.handle_key(ctrl('b'));
        assert_eq!(app.file_picker.selected_idx, 0);

        app.file_picker.selected_idx = 2;
        app.cursor_line_idx = 4;
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(
            (app.active_view, app.selected_file_idx, app.cursor_line_idx),
            (ActiveView::Diff, 2, 0)
        );

        app.active_view = ActiveView::FilePicker;
        press(&mut app, "zzz");
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.selected_file_idx, 2);
        app.active_view = ActiveView::FilePicker;
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.active_view, ActiveView::Diff);
    }

    #[test]
    fn pane_picker_keys() {
        let mut app = diff_app();
        app.pane_picker.list = (0..8)
            .map(|i| crate::ipc::TmuxPane {
                id: format!("%{i}"),
                title: "t".into(),
                active: false,
            })
            .collect();
        app.active_view = ActiveView::PanePicker;
        press(&mut app, "j");
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.pane_picker.selected_idx, 2);
        press(&mut app, "k");
        app.handle_key(key(KeyCode::Up));
        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.pane_picker.selected_idx, 0);
        app.handle_key(key(KeyCode::PageDown));
        assert_eq!(app.pane_picker.selected_idx, 5);
        app.handle_key(ctrl('d'));
        assert_eq!(app.pane_picker.selected_idx, 7);
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.pane_picker.selected_idx, 7);
        app.handle_key(ctrl('u'));
        assert_eq!(app.pane_picker.selected_idx, 2);
        app.handle_key(ctrl('f'));
        app.handle_key(key(KeyCode::PageUp));
        app.handle_key(ctrl('b'));
        assert_eq!(app.pane_picker.selected_idx, 0);

        app.pane_picker.selected_idx = 3;
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.agent_pane.as_deref(), Some("%3"));
        assert_eq!(app.status_message.as_deref(), Some("Target pane set to %3"));
        assert_eq!(app.active_view, ActiveView::Diff);

        app.active_view = ActiveView::PanePicker;
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(app.active_view, ActiveView::Diff);
        app.pane_picker.list.clear();
        app.active_view = ActiveView::PanePicker;
        app.handle_key(key(KeyCode::Down));
        app.handle_key(key(KeyCode::PageDown));
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.agent_pane.as_deref(), Some("%3"));
    }

    #[test]
    fn commit_modal_agent_prompt_and_command_keys() {
        let mut app = diff_app();
        app.active_view = ActiveView::CommitModal;
        press(&mut app, "wip");
        app.handle_key(ctrl('u'));
        assert!(app.commit_message_input.is_empty());
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(
            app.status_message.as_deref(),
            Some("⚠️ Commit message cannot be empty")
        );
        app.handle_key(ctrl('s'));
        assert_eq!(app.active_view, ActiveView::CommitModal);
        app.handle_key(ctrl('c'));
        assert_eq!(app.active_view, ActiveView::Diff);

        for k in [KeyCode::Esc, KeyCode::Char('q'), KeyCode::Char('P')] {
            app.active_view = ActiveView::AgentPrompt;
            app.handle_key(key(k));
            assert_eq!(app.active_view, ActiveView::Diff);
        }
        app.active_view = ActiveView::AgentPrompt;
        press(&mut app, "z");
        assert_eq!(app.active_view, ActiveView::AgentPrompt);
    }

    #[test]
    fn command_line_editing_history_and_completion() {
        let mut app = diff_app();
        app.command.history = vec!["first".into(), "second".into()];
        app.active_view = ActiveView::CommandLine;
        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.command.input, "second");
        app.handle_key(key(KeyCode::Up));
        app.handle_key(key(KeyCode::Up));
        assert_eq!(app.command.input, "first");
        app.handle_key(key(KeyCode::Down));
        assert_eq!(app.command.input, "second");
        app.handle_key(ctrl('n'));
        assert_eq!(app.command.input, "");
        app.handle_key(ctrl('p'));
        assert_eq!(app.command.input, "second");

        app.command.set_text("hello world".into());
        app.handle_key(ctrl('w'));
        assert_eq!(app.command.input, "hello ");
        app.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::ALT));
        assert_eq!(app.command.input, "");
        app.command.set_text("abcdef".into());
        app.handle_key(key(KeyCode::Home));
        app.handle_key(key(KeyCode::Right));
        app.handle_key(key(KeyCode::Delete));
        assert_eq!(app.command.input, "acdef");
        app.handle_key(ctrl('d'));
        assert_eq!(app.command.input, "adef");
        app.handle_key(ctrl('k'));
        assert_eq!(app.command.input, "a");
        app.handle_key(key(KeyCode::End));
        app.handle_key(key(KeyCode::Left));
        app.handle_key(ctrl('e'));
        app.handle_key(ctrl('a'));
        press(&mut app, "X");
        assert_eq!(app.command.input, "Xa");
        app.handle_key(ctrl('u'));
        assert_eq!(app.command.input, "a");
        app.handle_key(key(KeyCode::End));
        app.handle_key(ctrl('u'));
        assert_eq!(app.command.input, "");

        press(&mut app, "ab");
        app.handle_key(key(KeyCode::Backspace));
        app.handle_key(key(KeyCode::Backspace));
        assert_eq!(app.active_view, ActiveView::CommandLine);
        app.handle_key(key(KeyCode::Backspace));
        assert_eq!(app.active_view, ActiveView::Diff);

        app.active_view = ActiveView::CommandLine;
        press(&mut app, "wor");
        app.handle_key(key(KeyCode::Tab));
        assert!(app.command.input.starts_with("wor"));
        app.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT));
        app.handle_key(key(KeyCode::Esc));
        assert_eq!(
            (app.active_view, app.command.input.as_str()),
            (ActiveView::Diff, "")
        );
        app.active_view = ActiveView::CommandLine;
        app.handle_key(ctrl('c'));
        assert_eq!(app.active_view, ActiveView::Diff);

        app.active_view = ActiveView::CommandLine;
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.active_view, ActiveView::Diff);
        app.active_view = ActiveView::CommandLine;
        app.command.history.clear();
        press(&mut app, "nosuchcmd");
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.command.history, vec!["nosuchcmd".to_string()]);
        app.active_view = ActiveView::CommandLine;
        app.command.set_text("nosuchcmd".into());
        app.handle_key(key(KeyCode::Enter));
        assert_eq!(app.command.history.len(), 1);
    }
}
