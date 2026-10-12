use super::*;

impl App {
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
}
