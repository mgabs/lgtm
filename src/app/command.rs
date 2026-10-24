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
            self.handle_line_jump(line_num);
            return;
        }

        let mut parts = trimmed.split_whitespace();
        let cmd_name = parts.next().unwrap_or("");
        let args: Vec<&str> = parts.collect();

        match cmd_name {
            "q" | "quit" => self.request_quit(),
            "q!" | "quit!" => self.should_quit = true,
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
                self.request_quit();
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
            "reject" => {
                self.reject_hunk_and_reprompt();
            }
            "approve" | "lgtm" => {
                self.approve_gate();
            }
            "reject-all" | "reject-gate" => {
                self.reject_gate();
            }
            "prompt" | "context" => {
                self.open_agent_prompt();
            }
            "c" | "comments" | "directives" | "todo" if args.first() == Some(&"recheck") => {
                self.recheck_directives();
            }
            "c" | "comments" | "directives" | "todo" if args.first() == Some(&"orphans") => {
                self.orphans_command(&args[1..]);
            }
            "c" | "comments" | "directives" | "todo" => {
                self.open_comment_list();
            }
            "prs" => {
                match crate::pr::PullRequestState::parse(args.first().copied().unwrap_or("open")) {
                    Ok(state) => self.open_pr_picker_state(state),
                    Err(_) => self.status_message = Some("Usage: :prs [open|closed|all]".into()),
                }
            }
            "diff" | "rev" | "range" => {
                if let Some(spec) = args.first() {
                    self.load_local_revision((*spec).to_string());
                    self.active_view = ActiveView::Diff;
                } else {
                    self.active_view = ActiveView::Diff;
                    self.status_message = Some("Returned to diff view".into());
                }
            }
            "pr" => {
                if args.first() == Some(&"post") {
                    let confirmed = args.contains(&"--yes");
                    self.post_pr_comments_async(confirmed);
                    return;
                }
                if let Some(arg) = args.first() {
                    if let Ok(pr_num) = arg.parse::<u64>() {
                        self.load_pr_async(pr_num);
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
                } else {
                    let kind = self.resolved_sink_kind();
                    match IpcBridge::list_panes(kind) {
                        Ok(panes) if !panes.is_empty() => {
                            self.pane_picker.list = panes;
                            self.pane_picker.selected_idx = 0;
                            self.active_view = ActiveView::PanePicker;
                        }
                        _ => {
                            self.status_message =
                                Some(format!("No active targets detected for {} sink", kind));
                            self.active_view = ActiveView::Diff;
                        }
                    }
                }
            }
            "sink" => {
                if let Some(name) = args.first() {
                    match name.parse::<crate::ipc::SinkKind>() {
                        Ok(kind) => {
                            self.agent_sink_kind = Some(kind);
                            self.status_message =
                                Some(format!("Configured agent steering sink: {}", kind));
                        }
                        Err(err) => {
                            self.status_message = Some(err.to_string());
                        }
                    }
                } else {
                    let current = self.resolved_sink_kind();
                    self.status_message = Some(format!(
                        "Current agent sink: {} (configure with :sink <tmux|zellij|wezterm|kitty|clipboard|dummy>)",
                        current
                    ));
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
                if args.is_empty() || args.first() == Some(&"view") {
                    self.open_threads_modal();
                } else {
                    self.thread_action_async(&args);
                }
            }
            "threads" => {
                self.refresh_github_async(true);
                self.open_threads_modal();
            }
            "checks" => {
                self.refresh_github_async(false);
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
                self.trigger_risk_scan();
            }
            "editor" => {
                if let Some(name) = args.first() {
                    self.editor_override = Some(name.to_string());
                    self.status_message = Some(format!("Configured editor: {}", name));
                } else {
                    let current = self.resolved_editor();
                    self.status_message = Some(format!(
                        "Current editor: {} (set with :editor <name>)",
                        current
                    ));
                }
            }
            "reload" | "e" | "edit" => {
                self.reload_diff();
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
            "hook" => {
                let action = args.first().copied().unwrap_or("status");
                self.status_message = Some(Self::run_hook_action(&self.git_engine, action));
                self.active_view = ActiveView::Diff;
            }
            _ => {
                self.status_message = Some(format!("Unknown command: :{}", cmd_name));
                self.active_view = ActiveView::Diff;
            }
        }
    }

    fn run_hook_action(git_engine: &crate::git::GitEngine, action: &str) -> String {
        if action == "install" {
            return git_engine
                .install_pre_commit_hook()
                .map(|p| format!("Installed pre-commit hook at {}", p.display()))
                .unwrap_or_else(|e| format!("Failed to install pre-commit hook: {}", e));
        }
        if action == "uninstall" {
            return git_engine
                .uninstall_pre_commit_hook()
                .map(|removed| {
                    if removed {
                        "Uninstalled pre-commit hook"
                    } else {
                        "No LGTM pre-commit hook found to uninstall"
                    }
                    .to_string()
                })
                .unwrap_or_else(|e| format!("Failed to uninstall pre-commit hook: {}", e));
        }
        if action == "status" {
            return git_engine
                .pre_commit_hook_status()
                .map(|active| {
                    if active {
                        "Pre-commit hook is installed and active"
                    } else {
                        "Pre-commit hook is not installed"
                    }
                    .to_string()
                })
                .unwrap_or_else(|e| format!("Failed to check hook status: {}", e));
        }
        format!(
            "Unknown hook action '{}'. Usage: :hook [install|uninstall|status]",
            action
        )
    }

    /// Moves the cursor to the diff line closest to `target_line_no`; returns the line landed on.
    pub(crate) fn jump_to_line(&mut self, target_line_no: u32) -> Option<u32> {
        let f = self.snapshot.files.get(self.selected_file_idx)?;
        let mut best: Option<(u32, usize, u32)> = None;
        for (counter, line) in f.hunks.iter().flat_map(|h| &h.lines).enumerate() {
            if let Some(n) = line.new_line_num.or(line.old_line_num) {
                let dist = n.abs_diff(target_line_no);
                if best.is_none_or(|(d, _, _)| dist < d) {
                    best = Some((dist, counter, n));
                }
            }
        }
        let (_, idx, n) = best?;
        self.cursor_line_idx = idx;
        Some(n)
    }

    fn handle_line_jump(&mut self, line_num: u32) {
        self.active_view = ActiveView::Diff;
        if line_num == 0 {
            return;
        }
        self.status_message = Some(match self.jump_to_line(line_num) {
            Some(n) if n == line_num => format!("Jumped to line {}", line_num),
            Some(n) => format!(
                "Line {} not in diff; jumped to nearest line {}",
                line_num, n
            ),
            None => "No lines in the active file".to_string(),
        });
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

        app.execute_command(":editor vscode");
        assert_eq!(app.resolved_editor(), "vscode");
        assert_eq!(
            app.status_message,
            Some("Configured editor: vscode".to_string())
        );

        app.execute_command(":editor");
        assert!(app.status_message.as_ref().unwrap().contains("vscode"));

        app.execute_command(":sink zellij");
        assert_eq!(app.agent_sink_kind, Some(crate::ipc::SinkKind::Zellij));
        assert_eq!(
            app.status_message,
            Some("Configured agent steering sink: zellij".to_string())
        );

        app.execute_command(":sink");
        assert!(app.status_message.as_ref().unwrap().contains("zellij"));

        app.execute_command(":sink /dev/null");
        assert_eq!(app.agent_sink_kind, Some(crate::ipc::SinkKind::Dummy));
        assert_eq!(
            app.status_message,
            Some("Configured agent steering sink: dummy".to_string())
        );
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
            .contains("No lines in the active file"));
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
        assert_eq!(
            app_e.command.completions,
            vec!["e", "edit", "editor", "exit", "export"]
        );
        assert_eq!(app_e.command.input, "e");
        app_e.command.autocomplete_next();
        assert_eq!(app_e.command.input, "edit");
        app_e.command.autocomplete_next();
        assert_eq!(app_e.command.input, "editor");
        app_e.command.autocomplete_next();
        assert_eq!(app_e.command.input, "exit");

        // Test :<tab> (empty input shows all commands)
        let mut app_all = create_test_app();
        app_all.command.input = "".to_string();
        app_all.command.autocomplete_next();
        assert!(app_all.command.completions.len() > 30);
        assert_eq!(app_all.command.input, "approve");
        app_all.command.autocomplete_next();
        assert_eq!(app_all.command.input, "audit");
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
            folded_blocks: Vec::new(),
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
                    id: lgtm_core::comment::new_id(),
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
            id: lgtm_core::comment::new_id(),
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
                folded_blocks: Vec::new(),
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
            id: lgtm_core::comment::new_id(),
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

        crate::app::test_util::wait_for_audit(&mut app);

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
        crate::app::test_util::wait_for_audit(&mut app);
        assert!(report_file.exists());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_thread_modal_command() {
        let mut app = create_test_app();
        app.pr.current_number = Some(7);
        app.threads = vec![lgtm_core::github::Thread {
            id: "1".into(),
            path: "a.rs".into(),
            line: Some(4),
            resolved: false,
            comments: vec![],
        }];
        app.execute_command(":thread");
        assert_eq!(app.active_view, ActiveView::ThreadView);

        app.active_view = ActiveView::Diff;
        app.execute_command(":thread view");
        assert_eq!(app.active_view, ActiveView::ThreadView);
    }

    #[test]
    fn test_execute_command_hook() {
        let mut app = create_test_app();
        // create_test_app uses a temp dir without git repo initially
        app.execute_command("hook status");
        assert!(app
            .status_message
            .as_ref()
            .unwrap()
            .contains("Failed to check hook status"));

        // Now initialize a real git repository in a temp dir
        let tmp = std::env::temp_dir().join(format!(
            "lgtm-hook-cmd-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        lgtm_core::git2::Repository::init(&tmp).unwrap();
        app.git_engine = crate::git::GitEngine::new(&tmp);

        app.execute_command("hook status");
        assert_eq!(
            app.status_message.as_deref(),
            Some("Pre-commit hook is not installed")
        );

        app.execute_command("hook install");
        assert!(app
            .status_message
            .as_ref()
            .unwrap()
            .contains("Installed pre-commit hook"));

        app.execute_command("hook status");
        assert_eq!(
            app.status_message.as_deref(),
            Some("Pre-commit hook is installed and active")
        );

        app.execute_command("hook uninstall");
        assert_eq!(
            app.status_message.as_deref(),
            Some("Uninstalled pre-commit hook")
        );

        app.execute_command("hook uninstall");
        assert_eq!(
            app.status_message.as_deref(),
            Some("No LGTM pre-commit hook found to uninstall")
        );

        app.execute_command("hook unknown");
        assert!(app
            .status_message
            .as_ref()
            .unwrap()
            .contains("Unknown hook action"));

        let _ = std::fs::remove_dir_all(&tmp); // lgtm:ignore
    }
}
