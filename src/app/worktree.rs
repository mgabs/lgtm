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
                    .find(|w| w.path == std::path::Path::new(target) || w.path.ends_with(target))
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
            orphaned: false,
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

    fn git(dir: &std::path::Path, args: &[&str]) {
        let out = std::process::Command::new("git")
            .current_dir(dir)
            .env_remove("GIT_DIR")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_PREFIX")
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// Repo with one commit and a linked worktree `<repo>-wt` on branch `feat-x`.
    fn app_with_linked_worktree() -> (App, PathBuf) {
        let app = create_test_app();
        let root = app.git_engine.repo_path.clone();
        git(&root, &["init", "-q"]);
        std::fs::write(root.join("a.txt"), "x\n").unwrap();
        git(&root, &["add", "a.txt"]);
        git(
            &root,
            &[
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "commit",
                "-q",
                "-m",
                "init",
            ],
        );
        let wt = PathBuf::from(format!("{}-wt", root.display()));
        git(
            &root,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "feat-x",
                wt.to_str().unwrap(),
            ],
        );
        (app, wt)
    }

    #[test]
    fn test_picker_lists_worktrees_and_resets_selection() {
        let (mut app, _wt) = app_with_linked_worktree();
        app.worktree_picker.selected_idx = 5;
        app.open_worktree_picker();
        assert_eq!(app.active_view, ActiveView::WorktreePicker);
        assert_eq!(app.worktree_picker.selected_idx, 0);
        let branches: Vec<_> = app
            .worktree_picker
            .list
            .iter()
            .filter_map(|w| w.branch.clone())
            .collect();
        assert_eq!(branches.len(), 2);
        assert!(branches.contains(&"feat-x".to_string()));
    }

    #[test]
    fn test_picker_reports_list_failure_outside_repo() {
        let mut app = create_test_app();
        app.active_view = ActiveView::WorktreePicker;
        app.open_worktree_picker();
        assert_eq!(app.active_view, ActiveView::Diff);
        assert!(app
            .status_message
            .as_deref()
            .unwrap()
            .starts_with("Failed to list worktrees: "));

        app.status_message = None;
        app.jump_to_worktree("anything");
        assert!(app
            .status_message
            .as_deref()
            .unwrap()
            .starts_with("Failed to list worktrees: "));
    }

    #[test]
    fn test_jump_by_branch_dir_name_and_path_switches_repo() {
        for how in ["branch", "dirname", "path"] {
            let (mut app, wt) = app_with_linked_worktree();
            let target = match how {
                "branch" => "feat-x".to_string(),
                "dirname" => wt.file_name().unwrap().to_string_lossy().into_owned(),
                _ => wt
                    .canonicalize()
                    .unwrap()
                    .to_string_lossy()
                    .trim_start_matches(r"\\?\")
                    .to_owned(),
            };
            app.jump_to_worktree(&target);
            let name = wt.file_name().unwrap().to_string_lossy().into_owned();
            assert_eq!(
                app.status_message.as_deref(),
                Some(format!("Switched to worktree: {name}").as_str()),
                "{how}"
            );
            assert_eq!(app.active_view, ActiveView::Diff);
            assert_eq!(
                app.git_engine.repo_path.file_name(),
                wt.file_name(),
                "{how}"
            );
        }
    }

    #[test]
    fn test_jump_unknown_target_in_real_repo() {
        let (mut app, _wt) = app_with_linked_worktree();
        app.active_view = ActiveView::WorktreePicker;
        app.jump_to_worktree("no-such-branch");
        assert_eq!(
            app.status_message.as_deref(),
            Some("Worktree not found: no-such-branch")
        );
        assert_eq!(app.active_view, ActiveView::Diff);
    }
}
