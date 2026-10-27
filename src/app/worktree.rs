use super::*;

impl App {
    pub fn open_worktree_picker(&mut self) {
        match self.git_engine.list_worktrees() {
            Ok(worktrees) => {
                let non_bare: Vec<_> = worktrees.into_iter().filter(|w| !w.is_bare).collect();
                if non_bare.is_empty() {
                    self.status_message = Some("No worktrees found".to_string());
                    self.active_view = ActiveView::Diff;
                } else {
                    self.worktree_picker.list = non_bare;
                    self.worktree_picker.selected_idx = 0;
                    self.active_view = ActiveView::WorktreePicker;
                }
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to list worktrees: {}", e));
                self.active_view = ActiveView::Diff;
            }
        }
    }

    pub fn jump_to_worktree(&mut self, target: &str) {
        let worktrees = match self.git_engine.list_worktrees() {
            Ok(wt) => wt,
            Err(e) => {
                self.status_message = Some(format!("Failed to list worktrees: {}", e));
                return;
            }
        };

        let found = worktrees
            .iter()
            .find(|w| w.branch.as_deref() == Some(target))
            .or_else(|| {
                worktrees.iter().find(|w| {
                    w.path
                        .file_name()
                        .and_then(|f| f.to_str())
                        .map(|name| name == target)
                        .unwrap_or(false)
                })
            })
            .or_else(|| {
                worktrees
                    .iter()
                    .find(|w| w.path.to_string_lossy() == target || w.path.ends_with(target))
            });

        if let Some(wt) = found {
            if wt.is_bare {
                self.status_message = Some("Cannot switch to bare worktree".to_string());
            } else {
                self.initiate_worktree_switch(wt.path.clone());
            }
        } else {
            self.status_message = Some(format!("Worktree not found: {}", target));
            self.active_view = ActiveView::Diff;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::test_util::create_test_app;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use lgtm_core::comment::CommentAnchor;
    use lgtm_core::git::WorktreeInfo;
    use std::path::PathBuf;

    #[test]
    fn test_open_worktree_picker() {
        let mut app = create_test_app();
        app.execute_command(":wt");
        // In test app repo, either worktrees found or status message set
        assert!(app.active_view == ActiveView::WorktreePicker || app.status_message.is_some());
    }

    #[test]
    fn test_jump_to_unknown_worktree() {
        let mut app = create_test_app();
        app.execute_command(":wt nonexistent-branch-xyz");
        assert!(
            app.status_message
                .as_ref()
                .unwrap()
                .contains("Worktree not found")
                || app
                    .status_message
                    .as_ref()
                    .unwrap()
                    .contains("Failed to list worktrees")
        );
    }

    #[test]
    fn test_worktree_picker_navigation_and_cancel() {
        let mut app = create_test_app();
        app.worktree_picker.list = vec![
            WorktreeInfo {
                path: PathBuf::from("/repo"),
                head: "1111111".to_string(),
                branch: Some("main".to_string()),
                is_bare: false,
                is_current: true,
            },
            WorktreeInfo {
                path: PathBuf::from("/repo/.worktrees/feat-1"),
                head: "2222222".to_string(),
                branch: Some("feat-1".to_string()),
                is_bare: false,
                is_current: false,
            },
        ];
        app.active_view = ActiveView::WorktreePicker;
        app.worktree_picker.selected_idx = 0;

        // Down/j
        app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        assert_eq!(app.worktree_picker.selected_idx, 1);

        // Up/k
        app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        assert_eq!(app.worktree_picker.selected_idx, 0);

        // Esc cancels
        app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.active_view, ActiveView::Diff);
    }

    #[test]
    fn test_dirty_directive_guard_flow() {
        let mut app = create_test_app();
        // Add a pending comment (outcome is None)
        let anchor = CommentAnchor {
            id: lgtm_core::comment::new_id(),
            file_path: "src/main.rs".into(),
            target_line_no: 1,
            target_code: "fn main()".into(),
            context_before: vec![],
            context_after: vec![],
            comment_text: "needs refactor".into(),
            replies: vec![],
            created_at_turn: 1,
            is_resolved: false,
            pr_number: None,
            remote_id: None,
            orphaned: false,
            side: lgtm_core::comment::CommentSide::Right,
            track: Default::default(),
        };
        app.comments.push(anchor);

        let target = PathBuf::from("/tmp/test-wt-target");
        app.initiate_worktree_switch(target.clone());

        // Must show confirmation guard
        assert_eq!(app.active_view, ActiveView::WorktreeConfirmGuard);
        assert_eq!(app.worktree_picker.pending_target, Some(target.clone()));

        // Cancel with 'c'
        app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
        assert_eq!(app.active_view, ActiveView::Diff);
        assert_eq!(app.worktree_picker.pending_target, None);
        assert_eq!(app.comments.len(), 1);

        // Discard with 'd'
        app.initiate_worktree_switch(target);
        assert_eq!(app.active_view, ActiveView::WorktreeConfirmGuard);
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert_eq!(app.active_view, ActiveView::Diff);
        assert!(app.comments.is_empty());
    }
}
