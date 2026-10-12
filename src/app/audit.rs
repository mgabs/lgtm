use super::*;

impl App {
    pub fn filtered_audit_alerts(&self) -> Vec<&GuardAlert> {
        self.audit
            .alerts
            .iter()
            .filter(|alert| {
                if let Some(ref target_file) = self.audit.file_filter {
                    if &alert.file_path != target_file {
                        return false;
                    }
                }
                if let Some(sev) = self.audit.filter_severity {
                    alert.severity >= sev
                } else {
                    true
                }
            })
            .collect()
    }

    pub fn trigger_repo_scan(&mut self) {
        self.status_message =
            Some("Auditing full repository for security & architectural issues...".to_string());
        match crate::scan::FullRepoScanner::scan_repository(&self.audit.root_dir) {
            Ok((files, alerts)) => {
                let count = alerts.len();
                self.audit.files = files;
                self.audit.alerts = alerts;
                self.audit.selected_idx = 0;
                self.audit.filter_severity = None;
                self.audit.file_filter = None;
                self.audit.inspector_scroll = 0;
                self.audit.focus = AuditPaneFocus::Issues;
                self.sync_audit_selected_file();
                self.active_view = ActiveView::AuditReport;
                self.status_message = Some(format!(
                    "Audit scan complete: {} issues found across repository (press 'w' to export report)",
                    count
                ));
            }
            Err(e) => {
                self.status_message = Some(format!("Audit scan failed: {}", e));
            }
        }
    }

    pub fn trigger_diff_scan(&mut self) {
        self.status_message =
            Some("Scanning changed files for security & architectural issues...".to_string());
        self.alerts = crate::guard::SecurityAndTestGuard::scan_snapshot(&self.snapshot.files);
        self.audit.files = self.snapshot.files.clone();
        self.audit.alerts = self.alerts.clone();
        self.audit.selected_idx = 0;
        self.audit.filter_severity = None;
        self.audit.file_filter = None;
        self.audit.inspector_scroll = 0;
        self.audit.focus = AuditPaneFocus::Issues;
        self.sync_audit_selected_file();
        self.active_view = ActiveView::AuditReport;
        self.status_message = Some(format!(
            "Scan complete: {} issues found in changes across {} files (press 'w' to export report)",
            self.audit.alerts.len(),
            self.audit.files.len()
        ));
    }

    pub fn sync_audit_selected_file(&mut self) {
        let filtered = self.filtered_audit_alerts();
        if let Some(alert) = filtered.get(self.audit.selected_idx) {
            if let Some(pos) = self
                .audit
                .files
                .iter()
                .position(|f| f.display_name() == alert.file_path)
            {
                self.selected_file_idx = pos;
            }
        }
    }

    pub fn audit_next_file(&mut self) {
        let filtered = self.filtered_audit_alerts();
        let total = filtered.len();
        if total == 0 {
            return;
        }
        if let Some(current_alert) = filtered.get(self.audit.selected_idx) {
            let current_path = current_alert.file_path.clone();
            if let Some(next_idx) = (self.audit.selected_idx + 1..total)
                .find(|&i| filtered[i].file_path != current_path)
            {
                self.audit.selected_idx = next_idx;
                self.sync_audit_selected_file();
                return;
            }
            if let Some(wrap_idx) =
                (0..self.audit.selected_idx).find(|&i| filtered[i].file_path != current_path)
            {
                self.audit.selected_idx = wrap_idx;
                self.sync_audit_selected_file();
            }
        }
    }

    pub fn audit_prev_file(&mut self) {
        let filtered = self.filtered_audit_alerts();
        let total = filtered.len();
        if total == 0 {
            return;
        }
        if let Some(current_alert) = filtered.get(self.audit.selected_idx) {
            let current_path = current_alert.file_path.clone();
            let prev_diff_idx = (0..self.audit.selected_idx)
                .rev()
                .find(|&i| filtered[i].file_path != current_path);
            if let Some(idx) = prev_diff_idx {
                let target_path = filtered[idx].file_path.clone();
                let first_of_target = (0..=idx)
                    .find(|&i| filtered[i].file_path == target_path)
                    .unwrap_or(idx);
                self.audit.selected_idx = first_of_target;
                self.sync_audit_selected_file();
                return;
            }
            let last_diff_idx = (self.audit.selected_idx + 1..total)
                .rev()
                .find(|&i| filtered[i].file_path != current_path);
            if let Some(idx) = last_diff_idx {
                let target_path = filtered[idx].file_path.clone();
                let first_of_target = (0..=idx)
                    .find(|&i| filtered[i].file_path == target_path)
                    .unwrap_or(idx);
                self.audit.selected_idx = first_of_target;
                self.sync_audit_selected_file();
            }
        }
    }

    pub fn export_audit_report(
        &mut self,
        target_path: Option<&std::path::Path>,
    ) -> Result<PathBuf, anyhow::Error> {
        let path = target_path
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| self.audit.root_dir.join("lgtm-report.md"));

        if self.audit.alerts.is_empty() && self.audit.files.is_empty() {
            let (files, alerts) =
                crate::scan::FullRepoScanner::scan_repository(&self.audit.root_dir)?;
            self.audit.files = files;
            self.audit.alerts = alerts;
        }

        let report_content = crate::scan::FullRepoScanner::generate_markdown_report(
            &self.audit.root_dir,
            &self.audit.files,
            &self.audit.alerts,
        );

        std::fs::write(&path, report_content)?;
        self.status_message = Some(format!("✨ Audit report exported to: {}", path.display()));
        Ok(path)
    }

    pub fn cycle_audit_focus(&mut self, forward: bool) {
        if self.audit.maximized_inspector {
            return;
        }
        self.audit.focus = match (self.audit.focus, forward) {
            (AuditPaneFocus::Files, true) => AuditPaneFocus::Issues,
            (AuditPaneFocus::Issues, true) => AuditPaneFocus::Inspector,
            (AuditPaneFocus::Inspector, true) => {
                if self.show_file_drawer {
                    AuditPaneFocus::Files
                } else {
                    AuditPaneFocus::Issues
                }
            }
            (AuditPaneFocus::Inspector, false) => AuditPaneFocus::Issues,
            (AuditPaneFocus::Issues, false) => {
                if self.show_file_drawer {
                    AuditPaneFocus::Files
                } else {
                    AuditPaneFocus::Inspector
                }
            }
            (AuditPaneFocus::Files, false) => AuditPaneFocus::Inspector,
        };
        let target_name = match self.audit.focus {
            AuditPaneFocus::Files => "Files Drawer",
            AuditPaneFocus::Issues => "Issues List",
            AuditPaneFocus::Inspector => "Issue Inspector",
        };
        self.status_message = Some(format!("Focus: {}", target_name));
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
    fn test_audit_report_navigation_and_filtering() {
        use crate::guard::{GuardAlert, Severity};
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut app = create_test_app();
        app.active_view = ActiveView::AuditReport;
        app.audit.alerts = vec![
            GuardAlert {
                severity: Severity::Critical,
                rule_id: "test/fixture".to_string(),
                fingerprint: String::new(),
                title: "Secret Key".into(),
                description: "Leak detected".into(),
                line_no: Some(10),
                file_path: "src/auth.rs".into(),
            },
            GuardAlert {
                severity: Severity::High,
                rule_id: "test/fixture".to_string(),
                fingerprint: String::new(),
                title: "Inverted Assert".into(),
                description: "Fraudulent test".into(),
                line_no: Some(25),
                file_path: "tests/auth_test.rs".into(),
            },
            GuardAlert {
                severity: Severity::Warn,
                rule_id: "test/fixture".to_string(),
                fingerprint: String::new(),
                title: "High Complexity".into(),
                description: "Cyclomatic 25".into(),
                line_no: Some(50),
                file_path: "src/engine.rs".into(),
            },
        ];
        app.audit.files = vec![
            FileDiff {
                old_path: None,
                new_path: PathBuf::from("src/auth.rs"),
                change_type: ChangeType::Modified,
                additions: 1,
                deletions: 0,
                hunks: Vec::new(),
                is_folded: false,
                is_reviewed: false,
            },
            FileDiff {
                old_path: None,
                new_path: PathBuf::from("tests/auth_test.rs"),
                change_type: ChangeType::Modified,
                additions: 1,
                deletions: 0,
                hunks: Vec::new(),
                is_folded: false,
                is_reviewed: false,
            },
            FileDiff {
                old_path: None,
                new_path: PathBuf::from("src/engine.rs"),
                change_type: ChangeType::Modified,
                additions: 1,
                deletions: 0,
                hunks: Vec::new(),
                is_folded: false,
                is_reviewed: false,
            },
        ];

        assert_eq!(app.filtered_audit_alerts().len(), 3);

        // Key 'j' navigates down
        app.handle_audit_report_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        assert_eq!(app.audit.selected_idx, 1);

        // Key 'k' navigates up
        app.handle_audit_report_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
        assert_eq!(app.audit.selected_idx, 0);

        // Key '1' filters to Critical only
        app.handle_audit_report_key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE));
        assert_eq!(app.audit.filter_severity, Some(Severity::Critical));
        assert_eq!(app.filtered_audit_alerts().len(), 1);

        // Key '2' filters to High and Critical
        app.handle_audit_report_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE));
        assert_eq!(app.audit.filter_severity, Some(Severity::High));
        assert_eq!(app.filtered_audit_alerts().len(), 2);

        // Key '3' resets filter to all
        app.handle_audit_report_key(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE));
        assert_eq!(app.audit.filter_severity, None);
        assert_eq!(app.filtered_audit_alerts().len(), 3);

        // Sidebar toggle with 'b'
        let initial_drawer = app.show_file_drawer;
        app.handle_audit_report_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));
        assert_eq!(app.show_file_drawer, !initial_drawer);
        app.handle_audit_report_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));
        assert_eq!(app.show_file_drawer, initial_drawer);

        // Tab cycles focus between Files, Issues, and Inspector
        assert_eq!(app.audit.focus, AuditPaneFocus::Issues);
        app.handle_audit_report_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.audit.focus, AuditPaneFocus::Inspector);
        app.handle_audit_report_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.audit.focus, AuditPaneFocus::Files);
        app.handle_audit_report_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.audit.focus, AuditPaneFocus::Issues);

        // Inspector maximization with 'Z'
        assert!(!app.audit.maximized_inspector);
        app.handle_audit_report_key(KeyEvent::new(KeyCode::Char('Z'), KeyModifiers::NONE));
        assert!(app.audit.maximized_inspector);
        assert_eq!(app.audit.focus, AuditPaneFocus::Inspector);
        // Esc exits maximized view
        app.handle_audit_report_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!app.audit.maximized_inspector);

        // File switching with ']' and '['
        assert_eq!(app.audit.selected_idx, 0); // src/auth.rs
        app.handle_audit_report_key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE));
        assert_eq!(app.audit.selected_idx, 1); // tests/auth_test.rs
        app.handle_audit_report_key(KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE));
        assert_eq!(app.audit.selected_idx, 2); // src/engine.rs
        app.handle_audit_report_key(KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE));
        assert_eq!(app.audit.selected_idx, 1); // tests/auth_test.rs

        // File isolation filter in Files pane
        app.audit.focus = AuditPaneFocus::Files;
        app.selected_file_idx = 0; // src/auth.rs
        app.handle_audit_report_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        assert_eq!(app.audit.file_filter.as_deref(), Some("src/auth.rs"));
        assert_eq!(app.filtered_audit_alerts().len(), 1);

        // Reset filter with 'a'
        app.handle_audit_report_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        assert_eq!(app.audit.file_filter, None);
        assert_eq!(app.filtered_audit_alerts().len(), 3);

        // Next and previous issue navigation with 'w' / 'W' and 'n' / 'N'
        assert_eq!(app.audit.selected_idx, 0);
        app.handle_audit_report_key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::NONE));
        assert_eq!(app.audit.selected_idx, 1);
        app.handle_audit_report_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));
        assert_eq!(app.audit.selected_idx, 2);
        app.handle_audit_report_key(KeyEvent::new(KeyCode::Char('W'), KeyModifiers::NONE));
        assert_eq!(app.audit.selected_idx, 1);
        app.handle_audit_report_key(KeyEvent::new(KeyCode::Char('N'), KeyModifiers::NONE));
        assert_eq!(app.audit.selected_idx, 0);

        // Yank remediation directive via 'y'
        app.handle_audit_report_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        assert!(app
            .status_message
            .as_ref()
            .unwrap()
            .contains("Copied remediation directive"));

        // Help modal with '?'
        app.handle_audit_report_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
        assert_eq!(app.active_view, ActiveView::Help);
        app.active_view = ActiveView::AuditReport;

        // Esc returns to diff view
        app.handle_audit_report_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.active_view, ActiveView::Diff);
    }
}
