use super::*;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCategory {
    NavigationLayout,
    DirectivesThreads,
    AgentSteering,
    PrRemote,
    AuditGit,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CommandDef {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub category: CommandCategory,
    pub args_summary: &'static str,
    pub description: &'static str,
}

pub static COMMAND_REGISTRY: &[CommandDef] = &[
    // Navigation & Layout
    CommandDef {
        name: "quit",
        aliases: &["q"],
        category: CommandCategory::NavigationLayout,
        args_summary: "",
        description: "Request safe quit",
    },
    CommandDef {
        name: "quit!",
        aliases: &["q!"],
        category: CommandCategory::NavigationLayout,
        args_summary: "",
        description: "Force quit immediately without saving",
    },
    CommandDef {
        name: "write",
        aliases: &["w"],
        category: CommandCategory::NavigationLayout,
        args_summary: "[path]",
        description: "Save review directives or write audit scorecard",
    },
    CommandDef {
        name: "exit",
        aliases: &["wq", "x"],
        category: CommandCategory::NavigationLayout,
        args_summary: "",
        description: "Save directives and exit",
    },
    CommandDef {
        name: "diff",
        aliases: &["rev", "range"],
        category: CommandCategory::NavigationLayout,
        args_summary: "[revision]",
        description: "Switch to diff view or review revision range",
    },
    CommandDef {
        name: "split",
        aliases: &["sp", "vsp", "vsplit"],
        category: CommandCategory::NavigationLayout,
        args_summary: "",
        description: "Toggle Unified vs Side-by-Side split diff view",
    },
    CommandDef {
        name: "drawer",
        aliases: &["tree", "sidebar"],
        category: CommandCategory::NavigationLayout,
        args_summary: "",
        description: "Toggle file tree sidebar drawer",
    },
    CommandDef {
        name: "find",
        aliases: &["f", "files"],
        category: CommandCategory::NavigationLayout,
        args_summary: "",
        description: "Open fuzzy file switcher modal",
    },
    CommandDef {
        name: "preview",
        aliases: &["md"],
        category: CommandCategory::NavigationLayout,
        args_summary: "",
        description: "Toggle rendered Markdown preview",
    },
    CommandDef {
        name: "reload",
        aliases: &["e", "edit"],
        category: CommandCategory::NavigationLayout,
        args_summary: "",
        description: "Reload diff from disk",
    },
    CommandDef {
        name: "next",
        aliases: &["n"],
        category: CommandCategory::NavigationLayout,
        args_summary: "",
        description: "Navigate to next file or audit issue",
    },
    CommandDef {
        name: "prev",
        aliases: &["p"],
        category: CommandCategory::NavigationLayout,
        args_summary: "",
        description: "Navigate to previous file or audit issue",
    },
    // Review Directives & Threads
    CommandDef {
        name: "comments",
        aliases: &["c", "directives", "todo"],
        category: CommandCategory::DirectivesThreads,
        args_summary: "[recheck|orphans]",
        description: "Open review directives drawer",
    },
    CommandDef {
        name: "reply",
        aliases: &["rep"],
        category: CommandCategory::DirectivesThreads,
        args_summary: "[text]",
        description: "Reply to comment or thread at cursor",
    },
    CommandDef {
        name: "resolve",
        aliases: &["res"],
        category: CommandCategory::DirectivesThreads,
        args_summary: "[all]",
        description: "Toggle resolved status on comment/thread (or resolve all)",
    },
    CommandDef {
        name: "unresolve",
        aliases: &[],
        category: CommandCategory::DirectivesThreads,
        args_summary: "[all]",
        description: "Mark comment/thread active (or unresolve all)",
    },
    CommandDef {
        name: "clear",
        aliases: &[],
        category: CommandCategory::DirectivesThreads,
        args_summary: "",
        description: "Clear all review directives",
    },
    CommandDef {
        name: "clear-resolved",
        aliases: &[],
        category: CommandCategory::DirectivesThreads,
        args_summary: "",
        description: "Remove only resolved review directives",
    },
    // Agent Steering & Execution
    CommandDef {
        name: "send",
        aliases: &["review", "push"],
        category: CommandCategory::AgentSteering,
        args_summary: "",
        description: "Compile batch directives and dispatch to agent sink",
    },
    CommandDef {
        name: "prompt",
        aliases: &["context"],
        category: CommandCategory::AgentSteering,
        args_summary: "",
        description: "Open agent prompt / gate context modal",
    },
    CommandDef {
        name: "approve",
        aliases: &["lgtm"],
        category: CommandCategory::AgentSteering,
        args_summary: "",
        description: "Approve waiting agent breakpoint gate",
    },
    CommandDef {
        name: "reject",
        aliases: &[],
        category: CommandCategory::AgentSteering,
        args_summary: "",
        description: "Revert hunk and dispatch prompt with rejection reason",
    },
    CommandDef {
        name: "reject-all",
        aliases: &["reject-gate"],
        category: CommandCategory::AgentSteering,
        args_summary: "",
        description: "Reject waiting agent breakpoint gate",
    },
    CommandDef {
        name: "pane",
        aliases: &["panes"],
        category: CommandCategory::AgentSteering,
        args_summary: "[id]",
        description: "View or set agent target pane ID",
    },
    CommandDef {
        name: "sink",
        aliases: &[],
        category: CommandCategory::AgentSteering,
        args_summary: "[name]",
        description: "Configure multiplexer agent sink",
    },
    CommandDef {
        name: "inject",
        aliases: &[],
        category: CommandCategory::AgentSteering,
        args_summary: "[all]",
        description: "Inject audit violation directive to agent",
    },
    // PR & Remote Lifecycle
    CommandDef {
        name: "pr",
        aliases: &[],
        category: CommandCategory::PrRemote,
        args_summary: "<num> | post [--yes]",
        description: "Review pull request by number or post comments",
    },
    CommandDef {
        name: "prs",
        aliases: &[],
        category: CommandCategory::PrRemote,
        args_summary: "[open|closed|all]",
        description: "Open pull request list picker",
    },
    CommandDef {
        name: "threads",
        aliases: &["thread"],
        category: CommandCategory::PrRemote,
        args_summary: "[comment|reply|resolve|unresolve|edit|discard|drafts|submit|refresh]",
        description: "Inspect live PR threads; draft and submit comments",
    },
    CommandDef {
        name: "checks",
        aliases: &[],
        category: CommandCategory::PrRemote,
        args_summary: "",
        description: "Refresh and inspect CI status checks",
    },
    // Audit, Security & Git Lifecycle
    CommandDef {
        name: "audit",
        aliases: &[],
        category: CommandCategory::AuditGit,
        args_summary: "",
        description: "Run full codebase security and architecture audit",
    },
    CommandDef {
        name: "scan",
        aliases: &["issues"],
        category: CommandCategory::AuditGit,
        args_summary: "[repo]",
        description: "Scan diff changes (or full repo) for guard alerts",
    },
    CommandDef {
        name: "risk",
        aliases: &[],
        category: CommandCategory::AuditGit,
        args_summary: "",
        description: "Run security and fraud risk scan",
    },
    CommandDef {
        name: "filter",
        aliases: &["sev"],
        category: CommandCategory::AuditGit,
        args_summary: "<crit|high|all>",
        description: "Filter audit alerts by severity level",
    },
    CommandDef {
        name: "file",
        aliases: &[],
        category: CommandCategory::AuditGit,
        args_summary: "[path]",
        description: "Filter audit alerts to specific file",
    },
    CommandDef {
        name: "commit",
        aliases: &["ci"],
        category: CommandCategory::AuditGit,
        args_summary: "[message]",
        description: "Commit staged hunks (or open Commit Forge)",
    },
    CommandDef {
        name: "worktrees",
        aliases: &["wt"],
        category: CommandCategory::AuditGit,
        args_summary: "[name|path]",
        description: "Open worktree switcher modal",
    },
    CommandDef {
        name: "hook",
        aliases: &[],
        category: CommandCategory::AuditGit,
        args_summary: "[install|uninstall|status]",
        description: "Inspect or manage Git pre-commit hook",
    },
    CommandDef {
        name: "editor",
        aliases: &[],
        category: CommandCategory::AuditGit,
        args_summary: "[name]",
        description: "Configure external editor command",
    },
    CommandDef {
        name: "export",
        aliases: &[],
        category: CommandCategory::AuditGit,
        args_summary: "[path]",
        description: "Export audit scorecard markdown report",
    },
    CommandDef {
        name: "hidden",
        aliases: &[],
        category: CommandCategory::AuditGit,
        args_summary: "",
        description: "Display baselined and suppressed findings",
    },
    CommandDef {
        name: "yank",
        aliases: &["y"],
        category: CommandCategory::AuditGit,
        args_summary: "",
        description: "Yank current code line to clipboard",
    },
    CommandDef {
        name: "ref",
        aliases: &["yr"],
        category: CommandCategory::AuditGit,
        args_summary: "",
        description: "Yank code reference with file and line to clipboard",
    },
    CommandDef {
        name: "help",
        aliases: &["h", "?"],
        category: CommandCategory::AuditGit,
        args_summary: "",
        description: "Open interactive documentation modal",
    },
];

#[allow(dead_code)]
pub fn find_command(name: &str) -> Option<&'static CommandDef> {
    COMMAND_REGISTRY
        .iter()
        .find(|cmd| cmd.name == name || cmd.aliases.contains(&name))
}

pub fn autocomplete_candidates(prefix: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    for cmd in COMMAND_REGISTRY {
        if prefix.is_empty() || cmd.name.starts_with(prefix) {
            candidates.push(cmd.name.to_string());
        }
        for &alias in cmd.aliases {
            if prefix.is_empty() || alias.starts_with(prefix) {
                candidates.push(alias.to_string());
            }
        }
    }
    candidates.sort();
    candidates.dedup();
    candidates
}

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
                        "Saved {} review directives to .lgtm/directives.jsonl",
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
            "md" | "preview" => {
                self.toggle_markdown_preview();
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
            "wt" | "worktrees" => {
                if let Some(target) = args.first() {
                    self.jump_to_worktree(target);
                } else {
                    self.open_worktree_picker();
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
            "r" => {
                self.status_message = Some(
                    "Ambiguous command: :r. Did you mean :reply, :reload, or :review? (Use :help for all commands)".into(),
                );
                self.active_view = ActiveView::Diff;
            }
            "send" | "push" | "review" => {
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
                    self.thread_command(&args);
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
                } else if self.thread_at_cursor().is_some() {
                    self.toggle_resolve_thread_at_cursor();
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
                } else if self.thread_at_cursor().is_some() {
                    self.toggle_resolve_thread_at_cursor();
                } else {
                    self.toggle_resolve_current_comment();
                }
                self.active_view = ActiveView::Diff;
            }
            "reply" | "rep" => {
                if !args.is_empty() {
                    let text = args.join(" ");
                    if let Some(t_idx) = self.thread_at_cursor() {
                        self.thread_command(&["reply", &(t_idx + 1).to_string(), &text]);
                    } else if let Some(idx) = self.comment_idx_at_cursor() {
                        self.comments[idx].replies.push(text);
                        let _ = self.comment_store.save_comments(&self.comments);
                        self.status_message = Some("Reply added to local comment".to_string());
                    } else {
                        self.status_message =
                            Some("No thread or comment at cursor to reply to".to_string());
                    }
                    self.active_view = ActiveView::Diff;
                } else if let Some(t_idx) = self.thread_at_cursor() {
                    self.start_thread_reply(t_idx);
                } else if let Some(idx) = self.comment_idx_at_cursor() {
                    self.start_reply_to_comment(idx);
                } else {
                    self.comment_input.clear();
                    self.active_view = ActiveView::CommentInput;
                }
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
                self.status_message = Some(format!(
                    "Unknown command: :{} (type :help for available commands)",
                    cmd_name
                ));
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
        assert_eq!(app.command.input, "worktrees");

        app.command.autocomplete_next();
        assert_eq!(app.command.input, "wq");

        app.command.autocomplete_next();
        assert_eq!(app.command.input, "write");

        app.command.autocomplete_next();
        assert_eq!(app.command.input, "wt");

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
        assert_eq!(app_all.command.input, "?");
        app_all.command.autocomplete_next();
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

    #[test]
    fn test_command_registry_find_and_autocomplete() {
        assert!(find_command("send").is_some());
        assert_eq!(find_command("review").unwrap().name, "send");
        assert_eq!(find_command("push").unwrap().name, "send");
        assert!(find_command("r").is_none());

        let p_matches = autocomplete_candidates("p");
        assert!(p_matches.contains(&"pane".to_string()));
        assert!(p_matches.contains(&"panes".to_string()));
        assert!(p_matches.contains(&"pr".to_string()));
        assert!(p_matches.contains(&"prs".to_string()));
        assert!(p_matches.contains(&"prompt".to_string()));
        assert!(p_matches.contains(&"preview".to_string()));
    }

    #[test]
    fn test_help_modal_documents_all_canonical_commands() {
        let help_src = include_str!("../ui/help.rs");
        for cmd in COMMAND_REGISTRY {
            let pattern = format!(":{}", cmd.name);
            assert!(
                help_src.contains(&pattern) || help_src.contains(cmd.name),
                "Canonical command '{}' missing from src/ui/help.rs",
                cmd.name
            );
        }
    }
}
