use super::*;

impl App {
    pub fn export_and_send_batch_review(&mut self) {
        self.export_and_send_batch_review_with(|sink, text| sink.send(text));
    }

    fn export_and_send_batch_review_with(
        &mut self,
        send: impl FnOnce(&crate::ipc::AgentSink<'_>, &str) -> anyhow::Result<()>,
    ) {
        if let Some(pr_num) = self.pr.current_number {
            if self.review_posting_prs.contains(&pr_num) {
                self.status_message = Some("A review for this PR is already being posted".into());
                return;
            }
            let (prompt, file_path) =
                CommentStore::compile_pr_review_prompt(&self.comments, pr_num);
            let full_path = self.git_engine.repo_path.join(&file_path);
            let _ = std::fs::write(&full_path, &prompt);

            let mut notes = vec![format!("Saved to {}", file_path.display())];

            let sink = self.agent_sink();
            let text = if self.agent_pane.is_some() {
                let open_comments: Vec<_> = self
                    .comments
                    .iter()
                    .filter(|c| {
                        !c.is_resolved && (c.pr_number == Some(pr_num) || c.pr_number.is_none())
                    })
                    .collect();
                if open_comments.is_empty() {
                    format!("Please review directives in {}", file_path.display())
                } else {
                    let mut s = format!("Please review directives in {}:\n", file_path.display());
                    for c in open_comments {
                        s.push_str(&format!(
                            "- {}:{} DIRECTIVE: {}\n",
                            c.file_path, c.target_line_no, c.comment_text
                        ));
                    }
                    s.trim_end().to_string()
                }
            } else {
                prompt.clone()
            };
            notes.push(match send(&sink, &text) {
                Ok(()) => format!("Delivered to {}", sink.label()),
                Err(error) => format!("Send failed: {error}; saved review retained"),
            });

            self.review_posting_prs.insert(pr_num);
            let repo = self.git_engine.repo_path.clone();
            let factory = self.forge_factory;
            let label = format!("{} | Posting review", notes.join(" | "));
            self.spawn_job(
                &label,
                move || {
                    let forge = factory(&repo);
                    if !forge.available() {
                        anyhow::bail!("provider tools are unavailable");
                    }
                    forge.post_review(pr_num, "COMMENT", &prompt)
                },
                move |app, result| {
                    app.review_posting_prs.remove(&pr_num);
                    if result.is_err() {
                        app.quit_after_jobs = false;
                    }
                    notes.push(match result {
                        Ok(()) => "Posted review to PR".into(),
                        Err(error) => format!("Review not posted: {error}; saved review retained"),
                    });
                    if app.pr.current_number == Some(pr_num) {
                        app.status_message = Some(notes.join(" | "));
                    }
                },
            );
            return;
        }

        let turn = self.turn_manager.load_state().current_turn;
        let (prompt, file_path) = CommentStore::compile_batch_review_prompt(&self.comments, turn);

        let full_path = self.git_engine.repo_path.join(&file_path);
        let _ = std::fs::write(&full_path, &prompt);

        let sink = self.agent_sink();
        let text = if self.agent_pane.is_some() {
            let open_comments: Vec<_> = self.comments.iter().filter(|c| !c.is_resolved).collect();
            if open_comments.is_empty() {
                format!("Please review directives in {}", file_path.display())
            } else {
                let mut s = format!("Please review directives in {}:\n", file_path.display());
                for c in open_comments {
                    s.push_str(&format!(
                        "- {}:{} DIRECTIVE: {}\n",
                        c.file_path, c.target_line_no, c.comment_text
                    ));
                }
                s.trim_end().to_string()
            }
        } else {
            prompt.clone()
        };
        if let Err(error) = send(&sink, &text) {
            self.status_message = Some(format!(
                "Exported {} | Send failed: {error}; saved review retained",
                file_path.display()
            ));
            return;
        }
        let label = sink.label();
        self.status_message = Some(format!(
            "Exported {} and delivered to {}",
            file_path.display(),
            label
        ));

        let _ = self.turn_manager.advance_turn(None);
        self.invalidate_pr_completion();
        self.invalidate_view_data();
    }

    pub fn run_test_mutation_probe(&mut self) {
        let test_alerts: Vec<_> = self
            .alerts
            .iter()
            .filter(|a| a.rule_id.starts_with("test/"))
            .collect();
        if test_alerts.is_empty() {
            self.status_message = Some(
                "✅ Test Authenticity Probe: No test neutering or vacuous assertions detected"
                    .to_string(),
            );
        } else {
            self.status_message = Some(format!(
                "⚠️ Test Authenticity Probe: Found {} test risk(s) (e.g. [{}])",
                test_alerts.len(),
                test_alerts[0].rule_id
            ));
        }
    }

    pub fn inject_reference_to_agent(&mut self) {
        let Some(file) = self.snapshot.files.get(self.selected_file_idx) else {
            return;
        };
        let path_str = file.display_name();
        let target_line = file
            .hunks
            .iter()
            .flat_map(|h| &h.lines)
            .nth(self.cursor_line_idx)
            .and_then(|l| l.new_line_num.or(l.old_line_num));

        let payload = self.build_reference_payload(file, &path_str, target_line);
        let sink = self.agent_sink();
        let result = sink.send(&payload);
        let label = sink.label();
        self.status_message = Some(match result {
            Ok(()) => format!("Sent to {}: {}", label, payload),
            Err(e) => {
                let _ = crate::ipc::sink::AgentSinkKind::Osc52.send(&payload); // lgtm:ignore
                format!("Send failed: {}", e)
            }
        });
    }

    fn build_reference_payload(
        &self,
        file: &crate::model::FileDiff,
        path_str: &str,
        target_line: Option<u32>,
    ) -> String {
        let new_path_str = file.new_path.to_string_lossy();
        let mut directive_parts: Vec<String> = self
            .alerts
            .iter()
            .filter(|a| {
                (a.file_path == path_str || a.file_path == new_path_str) && a.line_no == target_line
            })
            .map(|a| format!("[{}] {}", a.rule_id, a.title))
            .collect();

        let matching_comment = self.comments.iter().find(|c| {
            !c.orphaned
                && (c.file_path == path_str || c.file_path == new_path_str)
                && target_line.is_none_or(|lno| c.target_line_no == lno)
        });
        if let Some(comment) = matching_comment {
            directive_parts.push(format!("DIRECTIVE: {}", comment.comment_text));
        }

        let base = match target_line {
            Some(line_no) => format!("#ref {}:{}", path_str, line_no),
            None => format!("#ref {}", path_str),
        };
        if directive_parts.is_empty() {
            return base;
        }
        let mut msg = format!("{} {}", base, directive_parts.join(" | "));
        if let Some(comment) = matching_comment.filter(|_| target_line.is_some()) {
            for reply in &comment.replies {
                msg.push_str(&format!("\n↳ Reply: {}", reply));
            }
        }
        msg
    }

    pub fn resolved_sink_kind(&self) -> crate::ipc::SinkKind {
        self.agent_sink_kind
            .filter(|&k| k != crate::ipc::SinkKind::Auto)
            .or_else(|| {
                self.config
                    .agent_sink
                    .as_deref()?
                    .parse()
                    .ok()
                    .filter(|&k| k != crate::ipc::SinkKind::Auto)
            })
            .unwrap_or_else(crate::ipc::IpcBridge::detect_sink)
    }

    pub fn agent_sink(&self) -> crate::ipc::sink::AgentSinkKind<'_> {
        use crate::ipc::sink::AgentSinkKind;
        match (self.resolved_sink_kind(), self.agent_pane.as_deref()) {
            (crate::ipc::SinkKind::Tmux, Some(pane)) => AgentSinkKind::Tmux(pane),
            (crate::ipc::SinkKind::Zellij, p) => AgentSinkKind::Zellij(p),
            (crate::ipc::SinkKind::WezTerm, p) => AgentSinkKind::WezTerm(p),
            (crate::ipc::SinkKind::Kitty, p) => AgentSinkKind::Kitty(p),
            (crate::ipc::SinkKind::Dummy, _) => AgentSinkKind::Direct,
            _ => AgentSinkKind::Osc52,
        }
    }

    pub fn open_in_editor(&mut self) {
        let Some(file) = self.snapshot.files.get(self.selected_file_idx) else {
            return;
        };
        let path = self.git_engine.repo_path.join(&file.new_path);
        let line_no = file
            .hunks
            .iter()
            .flat_map(|h| &h.lines)
            .nth(self.cursor_line_idx)
            .and_then(|l| l.new_line_num.or(l.old_line_num))
            .unwrap_or(1);
        let display_name = file.display_name();
        let editor = self.resolved_editor();
        let is_cli = !is_gui_editor(&editor.trim().to_lowercase());
        self.invalidate_view_data();
        self.status_message = Some(match launch_editor(&editor, &path, line_no) {
            Ok(_) => format!("Opened {}:{} in {}", display_name, line_no, editor),
            Err(err) => format!(
                "Failed to launch editor '{}': {} (use :editor <name>)",
                editor, err
            ),
        });
        if is_cli {
            self.needs_terminal_clear = true;
        }
    }

    pub fn open_file_in_editor(&mut self, file_rel_path: &str, line_no: u32) {
        let path = self.git_engine.repo_path.join(file_rel_path);
        let editor = self.resolved_editor();
        let is_cli = !is_gui_editor(&editor.trim().to_lowercase());
        self.invalidate_view_data();
        self.status_message = Some(match launch_editor(&editor, &path, line_no) {
            Ok(_) => format!("Opened {}:{} in {}", file_rel_path, line_no, editor),
            Err(err) => format!(
                "Failed to launch editor '{}': {} (use :editor <name>)",
                editor, err
            ),
        });
        if is_cli {
            self.needs_terminal_clear = true;
        }
    }
}

fn is_gui_editor(lower: &str) -> bool {
    matches!(
        lower,
        "vscode" | "code" | "cursor" | "subl" | "sublime" | "idea"
    ) || lower.starts_with("code ")
        || lower.starts_with("cursor ")
}

#[cfg(not(test))]
fn launch_editor(editor: &str, path: &std::path::Path, line_no: u32) -> Result<(), std::io::Error> {
    use crossterm::terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    };
    let lower = editor.trim().to_lowercase();
    let is_gui = is_gui_editor(&lower);
    if !is_gui {
        let _ = disable_raw_mode(); // lgtm:ignore
        let _ = crossterm::execute!(std::io::stdout(), LeaveAlternateScreen); // lgtm:ignore
    }

    let loc = format!("{}:{}", path.display(), line_no);
    let status = match lower.as_str() {
        "vscode" | "code" => {
            let prog = lgtm_core::sys::resolve_program("code");
            std::process::Command::new(prog.as_ref())
                .args(["-g", &loc])
                .status()
        }
        "cursor" => {
            let prog = lgtm_core::sys::resolve_program("cursor");
            std::process::Command::new(prog.as_ref())
                .args(["-g", &loc])
                .status()
        }
        "subl" | "sublime" => {
            let prog = lgtm_core::sys::resolve_program("subl");
            std::process::Command::new(prog.as_ref()).arg(&loc).status()
        }
        "idea" => {
            let prog = lgtm_core::sys::resolve_program("idea");
            std::process::Command::new(prog.as_ref())
                .args(["--line", &line_no.to_string()])
                .arg(path)
                .status()
        }
        _ => spawn_cli_editor(editor, path, line_no, &loc),
    };

    if !is_gui {
        let _ = crossterm::execute!(std::io::stdout(), EnterAlternateScreen); // lgtm:ignore
        let _ = enable_raw_mode(); // lgtm:ignore
    }

    status.map(|_| ())
}

#[cfg(test)]
fn launch_editor(
    _editor: &str,
    _path: &std::path::Path,
    _line_no: u32,
) -> Result<(), std::io::Error> {
    Ok(())
}

#[cfg(not(test))]
fn spawn_cli_editor(
    editor: &str,
    path: &std::path::Path,
    line_no: u32,
    loc: &str,
) -> Result<std::process::ExitStatus, std::io::Error> {
    let mut parts = editor.split_whitespace();
    let bin = parts.next().unwrap_or(editor);
    let resolved = lgtm_core::sys::resolve_program(bin);
    let mut cmd = std::process::Command::new(resolved.as_ref());
    cmd.args(parts);
    if bin == "hx" || bin == "helix" {
        cmd.arg(loc);
    } else {
        cmd.arg(format!("+{}", line_no)).arg(path);
    }
    cmd.status()
}

impl App {
    pub fn yank_current_code_line(&mut self) {
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let mut line_counter = 0;
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    if line_counter == self.cursor_line_idx {
                        IpcBridge::send_osc52_clipboard(&line.content);
                        self.status_message = Some(format!("Yanked: \"{}\"", line.content.trim()));
                        return;
                    }
                    line_counter += 1;
                }
            }
        }
    }

    pub fn yank_reference_to_clipboard(&mut self) {
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let line_num = file
                .hunks
                .iter()
                .flat_map(|h| &h.lines)
                .nth(self.cursor_line_idx)
                .and_then(|l| l.new_line_num.or(l.old_line_num))
                .unwrap_or(1);
            let reference = format!("#ref {}:{}", file.display_name(), line_num);
            IpcBridge::send_osc52_clipboard(&reference);
            self.status_message = Some(format!("Copied to clipboard: {}", reference));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::test_util::create_test_app;
    use crate::comment::CommentAnchor;
    use crate::guard::{GuardAlert, Severity};
    use crate::model::*;
    use std::path::PathBuf;

    fn setup_app_with_line(line_no: u32) -> crate::app::App {
        let mut app = create_test_app();
        let hunk = DiffHunk {
            old_start: 1,
            old_lines: 1,
            new_start: line_no,
            new_lines: 1,
            header: "@@ -1,1 +1,1 @@".into(),
            lines: vec![DiffLine {
                line_type: LineType::Addition,
                content: "let x = 42;".into(),
                old_line_num: None,
                new_line_num: Some(line_no),
            }],
            is_folded: false,
            folded_blocks: Vec::new(),
        };
        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/main.rs"),
            change_type: ChangeType::Modified,
            additions: 1,
            deletions: 0,
            hunks: vec![hunk],
            is_folded: false,
            is_reviewed: false,
        };
        app.snapshot = DiffSnapshot::new(vec![file]);
        app.selected_file_idx = 0;
        app.cursor_line_idx = 0;
        app.agent_pane = None;
        app.agent_sink_kind = Some(crate::ipc::SinkKind::Clipboard);
        app
    }

    #[test]
    fn test_inject_reference_plain() {
        let mut app = setup_app_with_line(10);
        app.inject_reference_to_agent();
        assert_eq!(
            app.status_message.as_deref(),
            Some("Sent to clipboard (OSC 52): #ref src/main.rs:10")
        );
    }

    #[test]
    fn test_inject_reference_with_alert() {
        let mut app = setup_app_with_line(10);
        app.alerts.push(GuardAlert {
            rule_id: "security/sqli".into(),
            fingerprint: "fp1".into(),
            severity: Severity::Critical,
            title: "SQL Injection vulnerability".into(),
            description: "desc".into(),
            line_no: Some(10),
            file_path: "src/main.rs".into(),
        });
        app.inject_reference_to_agent();
        assert_eq!(
            app.status_message.as_deref(),
            Some("Sent to clipboard (OSC 52): #ref src/main.rs:10 [security/sqli] SQL Injection vulnerability")
        );
    }

    #[test]
    fn test_inject_reference_with_directive() {
        let mut app = setup_app_with_line(10);
        app.comments.push(CommentAnchor {
            id: lgtm_core::comment::new_id(),
            file_path: "src/main.rs".into(),
            target_line_no: 10,
            target_code: "let x = 42;".into(),
            context_before: vec![],
            context_after: vec![],
            comment_text: "Use constant here".into(),
            replies: vec![],
            created_at_turn: 1,
            is_resolved: false,
            pr_number: None,
            orphaned: false,
            track: Default::default(),
        });
        app.inject_reference_to_agent();
        assert_eq!(
            app.status_message.as_deref(),
            Some("Sent to clipboard (OSC 52): #ref src/main.rs:10 DIRECTIVE: Use constant here")
        );
    }

    #[test]
    fn test_inject_reference_with_alert_and_directive() {
        let mut app = setup_app_with_line(10);
        app.alerts.push(GuardAlert {
            rule_id: "security/sqli".into(),
            fingerprint: "fp1".into(),
            severity: Severity::Critical,
            title: "SQL Injection vulnerability".into(),
            description: "desc".into(),
            line_no: Some(10),
            file_path: "src/main.rs".into(),
        });
        app.comments.push(CommentAnchor {
            id: lgtm_core::comment::new_id(),
            file_path: "src/main.rs".into(),
            target_line_no: 10,
            target_code: "let x = 42;".into(),
            context_before: vec![],
            context_after: vec![],
            comment_text: "Use parameter binding".into(),
            replies: vec!["Checked".into()],
            created_at_turn: 1,
            is_resolved: false,
            pr_number: None,
            orphaned: false,
            track: Default::default(),
        });
        app.inject_reference_to_agent();
        assert_eq!(
            app.status_message.as_deref(),
            Some("Sent to clipboard (OSC 52): #ref src/main.rs:10 [security/sqli] SQL Injection vulnerability | DIRECTIVE: Use parameter binding\n↳ Reply: Checked")
        );
    }

    #[test]
    fn test_editor_resolution_precedence() {
        let mut app = setup_app_with_line(1);
        // Default
        assert_eq!(app.resolved_editor(), "nvim");

        // Config file override
        app.config.editor = Some("vscode".to_string());
        assert_eq!(app.resolved_editor(), "vscode");

        // Runtime :editor command override takes highest precedence
        app.editor_override = Some("helix".to_string());
        assert_eq!(app.resolved_editor(), "helix");
    }

    #[test]
    fn test_agent_sink_resolution_and_labels() {
        let mut app = setup_app_with_line(1);
        // Default clipboard when not inside any multiplexer
        app.agent_sink_kind = Some(crate::ipc::SinkKind::Clipboard);
        assert_eq!(app.agent_sink().label(), "clipboard (OSC 52)");

        // Zellij sink with active pane vs specific pane
        app.agent_sink_kind = Some(crate::ipc::SinkKind::Zellij);
        app.agent_pane = None;
        assert_eq!(app.agent_sink().label(), "zellij active pane");
        app.agent_pane = Some("1".to_string());
        assert_eq!(app.agent_sink().label(), "zellij pane 1");

        // WezTerm sink
        app.agent_sink_kind = Some(crate::ipc::SinkKind::WezTerm);
        app.agent_pane = None;
        assert_eq!(app.agent_sink().label(), "wezterm active pane");
        app.agent_pane = Some("4".to_string());
        assert_eq!(app.agent_sink().label(), "wezterm pane 4");

        // Kitty sink
        app.agent_sink_kind = Some(crate::ipc::SinkKind::Kitty);
        app.agent_pane = None;
        assert_eq!(app.agent_sink().label(), "kitty active window");
        app.agent_pane = Some("7".to_string());
        assert_eq!(app.agent_sink().label(), "kitty window 7");

        // Tmux sink
        app.agent_sink_kind = Some(crate::ipc::SinkKind::Tmux);
        app.agent_pane = Some("%2".to_string());
        assert_eq!(app.agent_sink().label(), "pane %2");

        // Dummy sink
        app.agent_sink_kind = Some(crate::ipc::SinkKind::Dummy);
        assert_eq!(app.agent_sink().label(), "dummy (/dev/null)");
    }

    #[test]
    fn test_cli_editor_flags_terminal_clear() {
        let mut app = setup_app_with_line(1);
        app.editor_override = Some("true".to_string());
        assert!(!app.needs_terminal_clear);

        app.open_in_editor();
        assert!(app.needs_terminal_clear);
    }

    #[test]
    fn test_gui_editor_does_not_flag_terminal_clear() {
        let mut app = setup_app_with_line(1);
        app.editor_override = Some("code".to_string());
        assert!(!app.needs_terminal_clear);

        app.open_in_editor();
        assert!(!app.needs_terminal_clear);
    }

    fn comment(text: &str, pr: Option<u64>) -> CommentAnchor {
        CommentAnchor {
            id: lgtm_core::comment::new_id(),
            file_path: "src/main.rs".into(),
            target_line_no: 10,
            target_code: "let x = 42;".into(),
            context_before: vec![],
            context_after: vec![],
            comment_text: text.into(),
            replies: vec![],
            created_at_turn: 1,
            is_resolved: false,
            pr_number: pr,
            orphaned: false,
            track: Default::default(),
        }
    }

    fn wait_for_post(app: &mut crate::app::App, pr: u64) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        while app.review_posting_prs.contains(&pr) {
            app.handle_background_events();
            assert!(std::time::Instant::now() < deadline, "post did not finish");
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    #[test]
    fn test_batch_review_export_writes_file_and_advances_turn() {
        for pane in [None, Some("%1".to_string())] {
            let mut app = setup_app_with_line(10);
            app.agent_sink_kind = Some(crate::ipc::SinkKind::Dummy);
            app.agent_pane = pane;
            app.comments.push(comment("rename this", None));
            let turn_before = app.turn_manager.load_state().current_turn;
            app.export_and_send_batch_review();
            let status = app.status_message.clone().unwrap();
            assert!(status.starts_with("Exported "), "{status}");
            assert!(
                status.ends_with("delivered to dummy (/dev/null)"),
                "{status}"
            );
            let exported = status
                .trim_start_matches("Exported ")
                .split(" and delivered")
                .next()
                .unwrap();
            let body = std::fs::read_to_string(app.git_engine.repo_path.join(exported)).unwrap();
            assert!(body.contains("rename this"));
            assert_eq!(app.turn_manager.load_state().current_turn, turn_before + 1);
        }
    }

    #[test]
    fn test_batch_review_send_failure_retains_export_and_turn() {
        let mut app = setup_app_with_line(10);
        app.comments.push(comment("retry this directive", None));
        let turn_before = app.turn_manager.load_state().current_turn;
        app.export_and_send_batch_review_with(|_, _| anyhow::bail!("agent unavailable"));
        let status = app.status_message.as_deref().unwrap();
        assert!(
            status.ends_with("Send failed: agent unavailable; saved review retained"),
            "{status}"
        );
        assert!(!status.contains("delivered"));
        let exported = status
            .trim_start_matches("Exported ")
            .split(" | ")
            .next()
            .unwrap();
        let body = std::fs::read_to_string(app.git_engine.repo_path.join(exported)).unwrap();
        assert!(body.contains("retry this directive"));
        assert_eq!(app.turn_manager.load_state().current_turn, turn_before);
    }

    #[test]
    fn test_pr_batch_review_send_failure_survives_post_result() {
        let mut app = setup_app_with_line(10);
        app.pr.current_number = Some(7);
        app.comments.push(comment("pr note", Some(7)));
        app.export_and_send_batch_review_with(|_, _| anyhow::bail!("agent unavailable"));
        let status = app.status_message.as_deref().unwrap();
        assert!(
            status.contains("Send failed: agent unavailable"),
            "{status}"
        );
        assert!(!status.contains("Delivered"));
        assert!(app.review_posting_prs.contains(&7));
        wait_for_post(&mut app, 7);
        let status = app.status_message.as_deref().unwrap();
        assert!(
            status.contains("Send failed: agent unavailable"),
            "{status}"
        );
        assert!(status.contains("Review not posted:"), "{status}");
        assert!(!status.contains("Delivered"));
    }

    #[test]
    fn test_pr_review_post_failure_reports_and_clears_in_flight() {
        let mut app = setup_app_with_line(10);
        app.agent_sink_kind = Some(crate::ipc::SinkKind::Dummy);
        app.pr.current_number = Some(7);
        app.comments.push(comment("pr note", Some(7)));
        app.comments.push(comment("resolved", None));
        app.comments[1].is_resolved = true;
        for pane in [None, Some("%1".to_string())] {
            app.agent_pane = pane;
            app.export_and_send_batch_review();
            assert!(app.review_posting_prs.contains(&7));
            assert!(app
                .status_message
                .as_deref()
                .unwrap()
                .ends_with("Posting review..."));

            // A second request while one is in flight is rejected.
            app.export_and_send_batch_review();
            assert_eq!(
                app.status_message.as_deref(),
                Some("A review for this PR is already being posted")
            );

            wait_for_post(&mut app, 7);
            let status = app.status_message.clone().unwrap();
            assert!(
                status.contains("Delivered to dummy (/dev/null)"),
                "{status}"
            );
            assert!(
                status.ends_with(
                    "Review not posted: provider tools are unavailable; saved review retained"
                ),
                "{status}"
            );
        }
    }

    #[test]
    fn test_pr_review_result_ignored_for_other_pr() {
        let mut app = setup_app_with_line(10);
        app.pr.current_number = Some(7);
        app.export_and_send_batch_review();
        app.pr.current_number = Some(8);
        app.status_message = None;
        wait_for_post(&mut app, 7);
        assert_eq!(app.status_message, None);
    }

    #[test]
    fn test_mutation_probe_reports_clean_and_risky() {
        let mut app = setup_app_with_line(1);
        app.run_test_mutation_probe();
        assert!(app
            .status_message
            .as_deref()
            .unwrap()
            .contains("No test neutering"));
        app.alerts.push(GuardAlert {
            rule_id: "test/vacuous".into(),
            fingerprint: "f".into(),
            severity: Severity::Critical,
            title: "t".into(),
            description: "d".into(),
            line_no: None,
            file_path: "a.rs".into(),
        });
        app.run_test_mutation_probe();
        assert_eq!(
            app.status_message.as_deref(),
            Some("⚠️ Test Authenticity Probe: Found 1 test risk(s) (e.g. [test/vacuous])")
        );
    }

    #[test]
    fn test_inject_reference_without_files_is_noop() {
        let mut app = create_test_app();
        app.status_message = None;
        app.inject_reference_to_agent();
        app.open_in_editor();
        app.yank_current_code_line();
        app.yank_reference_to_clipboard();
        assert_eq!(app.status_message, None);
        assert!(!app.needs_terminal_clear);
    }

    #[test]
    fn test_inject_reference_send_failure_reported() {
        let mut app = setup_app_with_line(10);
        app.agent_sink_kind = Some(crate::ipc::SinkKind::Tmux);
        app.agent_pane = Some("%nonexistent-pane-xyz".into());
        app.inject_reference_to_agent();
        let status = app.status_message.unwrap();
        assert!(status.starts_with("Send failed:"), "{status}");
    }

    #[test]
    fn test_resolved_sink_kind_from_config() {
        let mut app = setup_app_with_line(1);
        app.agent_sink_kind = None;
        app.config.agent_sink = Some("kitty".into());
        assert_eq!(app.resolved_sink_kind(), crate::ipc::SinkKind::Kitty);
        app.agent_sink_kind = Some(crate::ipc::SinkKind::Zellij);
        assert_eq!(app.resolved_sink_kind(), crate::ipc::SinkKind::Zellij);
    }

    #[test]
    fn test_open_file_in_editor_status_and_clear_flag() {
        let mut app = setup_app_with_line(1);
        app.editor_override = Some("nvim".into());
        app.open_file_in_editor("src/lib.rs", 12);
        assert_eq!(
            app.status_message.as_deref(),
            Some("Opened src/lib.rs:12 in nvim")
        );
        assert!(app.needs_terminal_clear);

        app.needs_terminal_clear = false;
        app.editor_override = Some("cursor ".into());
        app.open_file_in_editor("a.rs", 3);
        assert!(!app.needs_terminal_clear);
    }

    #[test]
    fn test_open_in_editor_reports_line_and_file() {
        let mut app = setup_app_with_line(42);
        app.editor_override = Some("subl".into());
        app.open_in_editor();
        assert_eq!(
            app.status_message.as_deref(),
            Some("Opened src/main.rs:42 in subl")
        );
    }

    #[test]
    fn test_yank_line_and_reference() {
        let mut app = setup_app_with_line(33);
        app.yank_current_code_line();
        assert_eq!(
            app.status_message.as_deref(),
            Some("Yanked: \"let x = 42;\"")
        );
        app.yank_reference_to_clipboard();
        assert_eq!(
            app.status_message.as_deref(),
            Some("Copied to clipboard: #ref src/main.rs:33")
        );

        // Cursor past the last line: yank is a no-op, reference falls back to line 1.
        app.status_message = None;
        app.cursor_line_idx = 5;
        app.yank_current_code_line();
        assert_eq!(app.status_message, None);
        app.yank_reference_to_clipboard();
        assert_eq!(
            app.status_message.as_deref(),
            Some("Copied to clipboard: #ref src/main.rs:1")
        );
    }

    #[test]
    fn test_is_gui_editor_matches_known_names_only() {
        assert!(super::is_gui_editor("code --wait"));
        assert!(super::is_gui_editor("idea"));
        assert!(!super::is_gui_editor("vim"));
        assert!(!super::is_gui_editor("codeium"));
    }
}
