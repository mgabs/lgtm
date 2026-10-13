use super::*;

impl App {
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
            "c" | "comments" | "directives" | "todo" if args.first() == Some(&"recheck") => {
                self.recheck_directives();
            }
            "c" | "comments" | "directives" | "todo" if args.first() == Some(&"orphans") => {
                self.orphans_command(&args[1..]);
            }
            "c" | "comments" | "directives" | "todo" => {
                self.comment_list_selected_idx = 0;
                self.active_view = ActiveView::CommentList;
            }
            "prs" => {
                self.open_pr_picker();
            }
            "diff" | "rev" | "range" => {
                if let Some(spec) = args.first() {
                    let expanded = self.git_engine.expand_branch_spec(spec);
                    let rev_spec = &expanded.as_str();
                    match self.git_engine.compute_revision_diff(rev_spec) {
                        Ok(snap) => {
                            self.active_revision = Some(rev_spec.to_string());
                            self.pr.current_number = None;
                            self.snapshot = snap;
                            self.alerts = self.scan(&self.snapshot.files);
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
                if args.first() == Some(&"post") {
                    let confirmed = args.contains(&"--yes");
                    let forge = lgtm_core::forge::open(&self.git_engine.repo_path);
                    self.post_pr_comments(&*forge, confirmed);
                    return;
                }
                if let Some(arg) = args.first() {
                    if let Ok(pr_num) = arg.parse::<u64>() {
                        match crate::diff_source::DiffSource::snapshot(
                            &crate::diff_source::PullRequestSource {
                                pr: pr_num,
                                repo: self.git_engine.repo_path.clone(),
                            },
                        ) {
                            Ok(snap) => {
                                self.pr.current_number = Some(pr_num);
                                self.snapshot = snap;
                                self.alerts = self.scan(&self.snapshot.files);
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
                    self.pane_picker.list = panes;
                    self.pane_picker.selected_idx = 0;
                    self.active_view = ActiveView::PanePicker;
                } else {
                    self.status_message = Some("No tmux session running".to_string());
                    self.active_view = ActiveView::Diff;
                }
            }
            "f" | "files" | "find" => {
                self.active_view = ActiveView::FilePicker;
                self.file_picker.query.clear();
                self.file_picker.selected_idx = 0;
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
                    if total > 0 && self.audit.selected_idx + 1 < total {
                        self.audit.selected_idx += 1;
                        self.audit.inspector_scroll = 0;
                        self.sync_audit_selected_file();
                    }
                } else {
                    self.next_file();
                }
            }
            "prev" | "p" => {
                if self.active_view == ActiveView::AuditReport {
                    if self.audit.selected_idx > 0 {
                        self.audit.selected_idx -= 1;
                        self.audit.inspector_scroll = 0;
                        self.sync_audit_selected_file();
                    }
                } else {
                    self.prev_file();
                }
            }
            "filter" | "sev" => match args.first().copied().unwrap_or("") {
                "1" | "crit" | "critical" => {
                    self.audit.filter_severity = Some(crate::guard::Severity::Critical);
                    self.audit.selected_idx = 0;
                    self.sync_audit_selected_file();
                    self.status_message = Some("Filter: 🔴 Critical issues only".into());
                }
                "2" | "high" => {
                    self.audit.filter_severity = Some(crate::guard::Severity::High);
                    self.audit.selected_idx = 0;
                    self.sync_audit_selected_file();
                    self.status_message = Some("Filter: 🟠 High & 🔴 Critical issues".into());
                }
                "3" | "all" => {
                    self.audit.filter_severity = None;
                    self.audit.selected_idx = 0;
                    self.sync_audit_selected_file();
                    self.status_message = Some("Filter: All issues".into());
                }
                _ => {
                    self.status_message = Some("Usage: :filter <1|2|3|crit|high|all>".into());
                }
            },
            "file" => {
                if let Some(target_file) = args.first() {
                    self.audit.file_filter = Some(target_file.to_string());
                    self.audit.selected_idx = 0;
                    self.status_message =
                        Some(format!("Filtered audit issues to: {}", target_file));
                } else {
                    self.audit.file_filter = None;
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
                    let sink = self.agent_sink();
                    let _ = sink.send(&payload);
                    let label = sink.label();
                    drop(sink);
                    self.status_message =
                        Some(format!("Sent {} issues to {}", filtered.len(), label));
                } else if let Some(alert) = filtered.get(self.audit.selected_idx) {
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
            "audit" => {
                self.trigger_repo_scan();
            }
            "thread" => {
                let forge = lgtm_core::forge::open(&self.git_engine.repo_path);
                self.thread_action(&*forge, &args);
            }
            "checks" | "threads" => {
                self.refresh_github_async(cmd_name == "threads");
            }
            "hidden" => {
                let h = self.hidden.borrow();
                let list: Vec<String> = h
                    .suppressed
                    .iter()
                    .map(|a| ("suppressed", a))
                    .chain(h.existing.iter().map(|a| ("baselined", a)))
                    .take(5)
                    .map(|(k, a)| {
                        format!(
                            "{} {} {}:{}",
                            k,
                            a.rule_id,
                            a.file_path,
                            a.line_no.unwrap_or(0)
                        )
                    })
                    .collect();
                self.status_message = Some(if list.is_empty() {
                    "No suppressed or baselined findings".into()
                } else {
                    format!(
                        "{} suppressed, {} baselined: {}",
                        h.suppressed.len(),
                        h.existing.len(),
                        list.join("; ")
                    )
                });
            }
            "scan" | "issues" => {
                if args.first() == Some(&"repo") {
                    self.trigger_repo_scan();
                } else {
                    self.trigger_diff_scan();
                }
            }
            "risk" => {
                self.alerts = self.scan(&self.snapshot.files);
                self.status_message = Some(format!(
                    "Scanned {} files — {} security & test alerts found",
                    self.snapshot.files.len(),
                    self.alerts.len()
                ));
                self.active_view = ActiveView::Diff;
            }
            "reload" | "e" | "edit" => {
                if let Some(pr_num) = self.pr.current_number {
                    if let Ok(snap) = crate::diff_source::DiffSource::snapshot(
                        &crate::diff_source::PullRequestSource {
                            pr: pr_num,
                            repo: self.git_engine.repo_path.clone(),
                        },
                    ) {
                        self.snapshot = snap;
                        self.alerts = self.scan(&self.snapshot.files);
                        self.status_message = Some(format!("Reloaded PR #{}", pr_num));
                    }
                } else if self.git_engine.is_git_repo() {
                    if let Ok(snap) = self.git_engine.compute_diff_snapshot() {
                        self.snapshot = snap;
                        self.alerts = self.scan(&self.snapshot.files);
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
        app.command.input = "w".to_string();
        app.command.autocomplete_next();
        assert!(!app.command.completions.is_empty());
        assert_eq!(app.command.input, "w");

        app.command.autocomplete_next();
        assert_eq!(app.command.input, "write");

        app.command.autocomplete_next();
        assert_eq!(app.command.input, "wq");

        app.command.autocomplete_prev();
        assert_eq!(app.command.input, "write");

        // Test :e<tab>
        let mut app_e = create_test_app();
        app_e.command.input = "e".to_string();
        app_e.command.autocomplete_next();
        assert_eq!(app_e.command.completions, vec!["e", "edit", "exit"]);
        assert_eq!(app_e.command.input, "e");
        app_e.command.autocomplete_next();
        assert_eq!(app_e.command.input, "edit");
        app_e.command.autocomplete_next();
        assert_eq!(app_e.command.input, "exit");

        // Test :<tab> (empty input shows all commands)
        let mut app_all = create_test_app();
        app_all.command.input = "".to_string();
        app_all.command.autocomplete_next();
        assert!(app_all.command.completions.len() > 30);
        assert_eq!(app_all.command.input, "audit");
        app_all.command.autocomplete_next();
        assert_eq!(app_all.command.input, "c");
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
    fn test_github_refresh_uses_client_and_needs_a_pr() {
        use lgtm_core::github::{Bucket, Check, FakeClient, Thread};
        let mut app = create_test_app();
        app.refresh_github(&FakeClient::default(), false);
        assert_eq!(
            app.status_message.as_deref(),
            Some("Open a PR first (:pr <n>)")
        );
        app.pr.current_number = Some(3);
        let fake = FakeClient {
            checks: vec![
                Check {
                    name: "a".into(),
                    bucket: Bucket::Pass,
                },
                Check {
                    name: "b".into(),
                    bucket: Bucket::Fail,
                },
            ],
            threads: vec![
                Thread {
                    id: "1".into(),
                    path: "a.rs".into(),
                    line: Some(4),
                    resolved: false,
                    comments: vec![],
                },
                Thread {
                    id: "2".into(),
                    path: "b.rs".into(),
                    line: None,
                    resolved: true,
                    comments: vec![],
                },
            ],
            error: None,
            ..Default::default()
        };
        app.refresh_github(&fake, false);
        assert_eq!(app.ci.as_ref().map(|c| (c.passed, c.failed)), Some((1, 1)));
        assert_eq!(
            app.status_message.as_deref(),
            Some("CI: 1 passed, 1 failed, 0 pending")
        );
        app.refresh_github(&fake, true);
        assert_eq!(app.threads.len(), 2);
        assert_eq!(
            app.status_message.as_deref(),
            Some("2 threads, 1 open: a.rs:4")
        );
        let bad = FakeClient {
            error: Some("gh missing".into()),
            ..Default::default()
        };
        app.refresh_github(&bad, true);
        assert_eq!(app.status_message.as_deref(), Some("gh missing"));
        assert_eq!(app.threads.len(), 2);
    }

    #[test]
    fn test_pr_post_previews_then_posts_with_confirmation() {
        use lgtm_core::github::FakeClient;
        let mut app = create_test_app();
        app.post_pr_comments(&FakeClient::default(), true);
        assert_eq!(
            app.status_message.as_deref(),
            Some("Open a PR first (:pr <n>)")
        );
        app.pr.current_number = Some(4);
        app.post_pr_comments(&FakeClient::default(), true);
        assert_eq!(
            app.status_message.as_deref(),
            Some("No unposted comments for this PR")
        );
        let mut c =
            app.comments
                .first()
                .cloned()
                .unwrap_or_else(|| lgtm_core::comment::CommentAnchor {
                    file_path: "nowhere.rs".into(),
                    target_line_no: 1,
                    target_code: String::new(),
                    context_before: vec![],
                    context_after: vec![],
                    comment_text: "hi".into(),
                    replies: vec![],
                    created_at_turn: 1,
                    is_resolved: false,
                    pr_number: None,
                    remote_id: None,
                    orphaned: false,
                    track: Default::default(),
                });
        c.pr_number = Some(4);
        c.remote_id = None;
        c.is_resolved = false;
        app.comments = vec![c];
        let fake = FakeClient::default();
        app.post_pr_comments(&fake, false);
        assert!(app
            .status_message
            .as_deref()
            .unwrap()
            .starts_with("Would post"));
        assert!(fake.posted.borrow().is_empty());
        app.post_pr_comments(&fake, true);
        assert_eq!(app.status_message.as_deref(), Some("1 of 1 posted"));
        assert_eq!(fake.posted.borrow().len(), 1);
        assert!(app.comments[0].remote_id.is_some());
    }

    #[test]
    fn test_thread_actions_preview_then_send() {
        use lgtm_core::github::{FakeClient, Thread, ThreadComment};
        let mut app = create_test_app();
        let fake = FakeClient::default();
        app.thread_action(&fake, &["resolve", "1"]);
        assert_eq!(
            app.status_message.as_deref(),
            Some("Open a PR first (:pr <n>)")
        );
        app.pr.current_number = Some(2);
        app.thread_action(&fake, &["resolve"]);
        assert!(app.status_message.as_deref().unwrap().starts_with("Usage"));
        app.thread_action(&fake, &["resolve", "1"]);
        assert_eq!(
            app.status_message.as_deref(),
            Some("No such thread; run :threads first")
        );
        app.threads = vec![Thread {
            id: "T1".into(),
            path: "a.rs".into(),
            line: Some(3),
            resolved: false,
            comments: vec![ThreadComment {
                id: 9,
                author: "bob".into(),
                body: "x".into(),
            }],
        }];
        app.thread_action(&fake, &["resolve", "1"]);
        assert_eq!(
            app.status_message.as_deref(),
            Some("Would resolve thread on a.rs:3; add --yes to send")
        );
        app.thread_action(&fake, &["reply", "1"]);
        assert_eq!(app.status_message.as_deref(), Some("Reply text is empty"));
        app.thread_action(&fake, &["nuke", "1"]);
        assert_eq!(
            app.status_message.as_deref(),
            Some("Unknown thread action 'nuke'")
        );
        assert!(fake.posted.borrow().is_empty());
        app.thread_action(&fake, &["reply", "1", "thanks", "all", "--yes"]);
        assert_eq!(app.status_message.as_deref(), Some("Reply posted"));
        app.thread_action(&fake, &["resolve", "1", "--yes"]);
        assert!(app.threads[0].resolved);
        app.thread_action(&fake, &["unresolve", "1", "--yes"]);
        assert!(!app.threads[0].resolved);
        assert_eq!(
            *fake.posted.borrow(),
            vec!["reply:9:thanks all", "resolve:T1:true", "resolve:T1:false"]
        );
        let bad = FakeClient {
            fail_post_after: Some(0),
            ..Default::default()
        };
        app.thread_action(&bad, &["resolve", "1", "--yes"]);
        assert_eq!(app.status_message.as_deref(), Some("rate limited"));
        assert!(!app.threads[0].resolved);
    }

    #[test]
    fn background_jobs_apply_on_the_ui_thread() {
        let mut app = create_test_app();
        app.spawn_job(
            "Working",
            || 41 + 1,
            |app, n| {
                app.status_message = Some(format!("got {n}"));
            },
        );
        assert_eq!(app.status_message.as_deref(), Some("Working..."));
        for _ in 0..100 {
            app.handle_background_events();
            if app.status_message.as_deref() == Some("got 42") {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        panic!("job result never applied");
    }

    #[test]
    fn orphans_list_repin_and_drop() {
        use crate::model::{ChangeType, DiffHunk, DiffLine, FileDiff, LineType};
        let mut app = create_test_app();
        app.execute_command("directives orphans");
        assert_eq!(app.status_message.as_deref(), Some("No orphaned comments"));
        let mk = |orphaned| lgtm_core::comment::CommentAnchor {
            file_path: "old.rs".into(),
            target_line_no: 9,
            target_code: "x".into(),
            context_before: vec![],
            context_after: vec![],
            comment_text: "x".into(),
            replies: vec![],
            created_at_turn: 0,
            is_resolved: false,
            pr_number: None,
            remote_id: None,
            orphaned,
            track: Default::default(),
        };
        app.comments = vec![mk(true), mk(false), mk(true)];
        app.execute_command("directives orphans");
        assert_eq!(
            app.status_message.as_deref(),
            Some("2 orphaned: 1) old.rs:9, 2) old.rs:9")
        );
        app.snapshot = DiffSnapshot::new(vec![FileDiff {
            old_path: None,
            new_path: "new.rs".into(),
            change_type: ChangeType::Modified,
            additions: 1,
            deletions: 0,
            hunks: vec![DiffHunk {
                header: "@@".into(),
                old_start: 1,
                old_lines: 1,
                new_start: 1,
                new_lines: 1,
                lines: vec![DiffLine {
                    line_type: LineType::Addition,
                    content: "y".into(),
                    old_line_num: None,
                    new_line_num: Some(3),
                }],
                is_folded: false,
            }],
            is_folded: false,
            is_reviewed: false,
        }]);
        app.selected_file_idx = 0;
        app.cursor_line_idx = 0;
        app.execute_command("directives orphans repin 1");
        assert_eq!(app.status_message.as_deref(), Some("Re-pinned to new.rs:3"));
        assert_eq!(
            (app.comments[0].orphaned, app.comments[0].target_line_no),
            (false, 3)
        );
        app.execute_command("directives orphans drop");
        assert!(app
            .status_message
            .as_deref()
            .unwrap()
            .starts_with("Would delete 1"));
        assert_eq!(app.comments.len(), 3);
        app.execute_command("directives orphans drop --yes");
        assert_eq!(app.comments.len(), 2);
    }

    #[test]
    fn test_directives_recheck_reports_outcomes() {
        let mut app = create_test_app();
        app.execute_command("directives recheck");
        assert_eq!(
            app.status_message.as_deref(),
            Some("Directives: 0 untouched, 0 changed, 0 gone, 0 orphaned")
        );
        app.comments.push(lgtm_core::comment::CommentAnchor {
            file_path: "gone.rs".into(),
            target_line_no: 1,
            target_code: String::new(),
            context_before: vec![],
            context_after: vec![],
            comment_text: "x".into(),
            replies: vec![],
            created_at_turn: 0,
            is_resolved: false,
            pr_number: None,
            remote_id: None,
            orphaned: false,
            track: lgtm_core::outcome::Tracking {
                hunk_hash: Some("abc".into()),
                ..Default::default()
            },
        });
        app.execute_command("directives recheck");
        assert_eq!(
            app.status_message.as_deref(),
            Some("Directives: 0 untouched, 0 changed, 1 gone, 0 orphaned")
        );
        assert_eq!(app.active_view, ActiveView::Diff);
    }

    #[test]
    fn test_hidden_command_lists_suppressed_and_baselined() {
        let mut app = create_test_app();
        app.execute_command("hidden");
        assert_eq!(
            app.status_message.as_deref(),
            Some("No suppressed or baselined findings")
        );
        let mk = |rule: &str| crate::guard::GuardAlert {
            rule_id: rule.into(),
            fingerprint: String::new(),
            severity: crate::guard::Severity::Warn,
            title: "t".into(),
            description: "d".into(),
            line_no: Some(3),
            file_path: "a.rs".into(),
        };
        *app.hidden.borrow_mut() = Hidden {
            suppressed: vec![mk("sec/a")],
            existing: vec![mk("dead/b")],
        };
        app.execute_command("hidden");
        let msg = app.status_message.unwrap();
        assert!(msg.starts_with("1 suppressed, 1 baselined"));
        assert!(msg.contains("suppressed sec/a a.rs:3"));
        assert!(msg.contains("baselined dead/b a.rs:3"));
    }

    #[test]
    fn test_execute_command_scan_and_export() {
        let mut app = create_test_app();
        let tmp = std::env::temp_dir().join(format!("lgtm_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp);
        app.audit.root_dir = tmp.clone();

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
            app.audit.filter_severity,
            Some(crate::guard::Severity::Critical)
        );

        // :file test.rs filters to target file
        app.execute_command("file test.rs");
        assert_eq!(app.audit.file_filter.as_deref(), Some("test.rs"));

        // :file clears filter
        app.execute_command("file");
        assert_eq!(app.audit.file_filter, None);

        // Export report
        let report_file = tmp.join("custom-report.md");
        app.execute_command(&format!("w {}", report_file.display()));
        assert!(report_file.exists());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
