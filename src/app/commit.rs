use super::*;

impl App {
    pub fn open_commit_modal(&mut self) {
        self.invalidate_view_data();
        let staged_count = self.staged_files_count();
        self.record_staged_count(staged_count);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::test_util::create_test_app;

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

    fn app_with_staged_file() -> App {
        let app = create_test_app();
        let root = app.git_engine.repo_path.clone();
        git(&root, &["init", "-q"]);
        std::fs::write(root.join("a.txt"), "hello\n").unwrap();
        git(&root, &["add", "a.txt"]);
        app
    }

    #[test]
    fn test_open_commit_modal_requires_staged_changes() {
        let mut app = create_test_app();
        git(&app.git_engine.repo_path.clone(), &["init", "-q"]);
        app.commit_message_input = "stale".into();
        app.open_commit_modal();
        assert_eq!(app.active_view, ActiveView::Diff);
        assert_eq!(
            app.status_message.as_deref(),
            Some("⚠️ No staged changes to commit (stage with 's' or 'S')")
        );
        assert_eq!(app.commit_message_input, "stale");
    }

    #[test]
    fn test_open_commit_modal_with_staged_changes_clears_input() {
        let mut app = app_with_staged_file();
        app.commit_message_input = "stale".into();
        app.open_commit_modal();
        assert_eq!(app.active_view, ActiveView::CommitModal);
        assert!(app.commit_message_input.is_empty());
    }

    #[test]
    fn test_execute_commit_rejects_blank_message() {
        let mut app = app_with_staged_file();
        app.active_view = ActiveView::CommitModal;
        app.commit_message_input = "   ".into();
        app.execute_commit();
        assert_eq!(
            app.status_message.as_deref(),
            Some("⚠️ Commit message cannot be empty")
        );
        assert_eq!(app.active_view, ActiveView::CommitModal);
    }

    #[test]
    fn test_execute_commit_creates_commit_and_returns_to_diff() {
        let mut app = app_with_staged_file();
        app.active_view = ActiveView::CommitModal;
        app.commit_message_input = "  feat: add a  ".into();
        app.execute_commit();
        assert_eq!(app.active_view, ActiveView::Diff);
        assert!(app.commit_message_input.is_empty());
        let log = std::process::Command::new("git")
            .current_dir(&app.git_engine.repo_path)
            .env_remove("GIT_DIR")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_WORK_TREE")
            .args(["log", "-1", "--format=%s"])
            .output()
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&log.stdout).trim(), "feat: add a");
    }

    #[test]
    fn test_execute_commit_failure_reports_error() {
        let mut app = create_test_app();
        app.commit_message_input = "msg".into();
        app.active_view = ActiveView::CommitModal;
        app.execute_commit();
        assert!(app
            .status_message
            .as_deref()
            .unwrap()
            .starts_with("Commit failed: "));
        assert_eq!(app.active_view, ActiveView::CommitModal);
        assert_eq!(app.commit_message_input, "msg");
    }
}
