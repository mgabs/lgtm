use super::*;

impl App {
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
                if self.active_view == ActiveView::AuditReport {
                    let target_path = args.first().map(std::path::Path::new);
                    let _ = self.export_audit_report(target_path);
                } else {
                    let _ = self.comment_store.save_comments(&self.comments);
                    self.status_message = Some(format!(
                        "Saved {} review directives to .lgtm/comments.jsonl",
                        self.comments.len()
                    ));
                    self.active_view = ActiveView::Diff;
                }
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
            "diff" | "rev" | "range" => {
                if let Some(rev_spec) = args.first() {
                    match self.git_engine.compute_revision_diff(rev_spec) {
                        Ok(snap) => {
                            self.active_revision = Some(rev_spec.to_string());
                            self.current_pr_number = None;
                            self.snapshot = snap;
                            self.alerts = SecurityAndTestGuard::scan_snapshot(&self.snapshot.files);
                            self.selected_file_idx = 0;
                            self.cursor_line_idx = 0;
                            self.diff_scroll_offset = 0;
                            self.status_message = Some(format!(
                                "Loaded revision '{}' ({} files changed, {} alerts)",
                                rev_spec,
                                self.snapshot.files.len(),
                                self.alerts.len()
                            ));
                            self.active_view = ActiveView::Diff;
                        }
                        Err(e) => {
                            self.status_message =
                                Some(format!("Failed to load revision '{}': {}", rev_spec, e));
                            self.active_view = ActiveView::Diff;
                        }
                    }
                } else {
                    self.active_view = ActiveView::Diff;
                    self.status_message = Some("Returned to diff view".into());
                }
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
            "r" | "review" => {
                self.export_and_send_batch_review();
                self.active_view = ActiveView::Diff;
            }
            "send" | "push" => {
                self.export_and_send_batch_review();
                self.active_view = ActiveView::Diff;
            }
            "export" => {
                let target_path = args.first().map(std::path::Path::new);
                let _ = self.export_audit_report(target_path);
            }
            "next" | "n" => {
                if self.active_view == ActiveView::AuditReport {
                    let total = self.filtered_audit_alerts().len();
                    if total > 0 && self.audit_selected_idx + 1 < total {
                        self.audit_selected_idx += 1;
                        self.audit_inspector_scroll = 0;
                        self.sync_audit_selected_file();
                    }
                } else {
                    self.next_file();
                }
            }
            "prev" | "p" => {
                if self.active_view == ActiveView::AuditReport {
                    if self.audit_selected_idx > 0 {
                        self.audit_selected_idx -= 1;
                        self.audit_inspector_scroll = 0;
                        self.sync_audit_selected_file();
                    }
                } else {
                    self.prev_file();
                }
            }
            "filter" | "sev" => match args.first().copied().unwrap_or("") {
                "1" | "crit" | "critical" => {
                    self.audit_filter_severity = Some(crate::guard::Severity::Critical);
                    self.audit_selected_idx = 0;
                    self.sync_audit_selected_file();
                    self.status_message = Some("Filter: 🔴 Critical issues only".into());
                }
                "2" | "high" => {
                    self.audit_filter_severity = Some(crate::guard::Severity::High);
                    self.audit_selected_idx = 0;
                    self.sync_audit_selected_file();
                    self.status_message = Some("Filter: 🟠 High & 🔴 Critical issues".into());
                }
                "3" | "all" => {
                    self.audit_filter_severity = None;
                    self.audit_selected_idx = 0;
                    self.sync_audit_selected_file();
                    self.status_message = Some("Filter: All issues".into());
                }
                _ => {
                    self.status_message = Some("Usage: :filter <1|2|3|crit|high|all>".into());
                }
            },
            "file" => {
                if let Some(target_file) = args.first() {
                    self.audit_file_filter = Some(target_file.to_string());
                    self.audit_selected_idx = 0;
                    self.status_message =
                        Some(format!("Filtered audit issues to: {}", target_file));
                } else {
                    self.audit_file_filter = None;
                    self.status_message = Some("Cleared file filter: showing all issues".into());
                }
            }
            "inject" => {
                let filtered = self.filtered_audit_alerts();
                if filtered.is_empty() {
                    self.status_message = Some("No issues to inject".into());
                } else if args.first() == Some(&"all") {
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
                    if let Some(ref pane) = self.agent_pane {
                        let _ = IpcBridge::send_to_tmux(pane, &payload);
                        self.status_message = Some(format!(
                            "Injected {} issues to pane {}",
                            filtered.len(),
                            pane
                        ));
                    } else {
                        IpcBridge::send_osc52_clipboard(&payload);
                        self.status_message = Some(format!(
                            "Copied {} issues to clipboard (OSC 52)",
                            filtered.len()
                        ));
                    }
                } else if let Some(alert) = filtered.get(self.audit_selected_idx) {
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
                    if let Some(ref pane) = self.agent_pane {
                        let _ = IpcBridge::send_to_tmux(pane, &directive);
                        self.status_message = Some(format!("Injected directive to pane {}", pane));
                    } else {
                        IpcBridge::send_osc52_clipboard(&directive);
                        self.status_message = Some("Copied directive to clipboard (OSC 52)".into());
                    }
                }
            }
            "audit" => {
                self.trigger_repo_scan();
            }
            "scan" | "issues" => {
                if args.first() == Some(&"repo") {
                    self.trigger_repo_scan();
                } else {
                    self.trigger_diff_scan();
                }
            }
            "risk" => {
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
    fn test_execute_command_scan_and_export() {
        let mut app = create_test_app();
        let tmp = std::env::temp_dir().join(format!("lgtm_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        app.audit_root_dir = tmp.clone();

        // :scan scans changes and switches to AuditReport
        app.execute_command("scan");
        assert_eq!(app.active_view, ActiveView::AuditReport);

        // :diff returns to Diff
        app.execute_command("diff");
        assert_eq!(app.active_view, ActiveView::Diff);

        // :audit audits full repo and switches to AuditReport
        app.execute_command("audit");
        assert_eq!(app.active_view, ActiveView::AuditReport);

        // :filter 1 filters to Critical
        app.execute_command("filter 1");
        assert_eq!(
            app.audit_filter_severity,
            Some(crate::guard::Severity::Critical)
        );

        // :file test.rs filters to target file
        app.execute_command("file test.rs");
        assert_eq!(app.audit_file_filter.as_deref(), Some("test.rs"));

        // :file clears filter
        app.execute_command("file");
        assert_eq!(app.audit_file_filter, None);

        // Export report
        let report_file = tmp.join("custom-report.md");
        app.execute_command(&format!("w {}", report_file.display()));
        assert!(report_file.exists());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
