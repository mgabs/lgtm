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
}
