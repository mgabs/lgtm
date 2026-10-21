use super::*;

impl App {
    pub fn export_and_send_batch_review(&mut self) {
        if let Some(pr_num) = self.pr.current_number {
            let (prompt, file_path) =
                CommentStore::compile_pr_review_prompt(&self.comments, pr_num);
            let full_path = self.git_engine.repo_path.join(&file_path);
            let _ = std::fs::write(&full_path, &prompt);

            let mut notes = vec![format!("Saved to {}", file_path.display())];

            let forge = lgtm_core::forge::open(&self.git_engine.repo_path);
            if forge.available() {
                match forge.post_review(pr_num, "COMMENT", &prompt) {
                    Ok(_) => notes.push("Posted review to PR".to_string()),
                    Err(e) => notes.push(format!("review not posted: {}", e)),
                }
            }

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
            let _ = sink.send(&text);
            let label = sink.label();
            drop(sink);
            notes.push(format!("Delivered to {}", label));

            self.status_message = Some(notes.join(" | "));
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
        let _ = sink.send(&text);
        let label = sink.label();
        drop(sink);
        self.status_message = Some(format!(
            "Exported {} and delivered to {}",
            file_path.display(),
            label
        ));

        let _ = self.turn_manager.advance_turn(None);
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
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let path_str = file.display_name();

            let mut target_line: Option<u32> = None;
            let mut line_counter = 0;

            for hunk in &file.hunks {
                for line in &hunk.lines {
                    if line_counter == self.cursor_line_idx {
                        target_line = line.new_line_num.or(line.old_line_num);
                        break;
                    }
                    line_counter += 1;
                }
                if target_line.is_some() {
                    break;
                }
            }

            let matching_alerts: Vec<&crate::guard::GuardAlert> = self
                .alerts
                .iter()
                .filter(|a| {
                    (a.file_path == path_str || a.file_path == file.new_path.to_string_lossy())
                        && a.line_no == target_line
                })
                .collect();

            let matching_comment = target_line
                .and_then(|lno| {
                    self.comments.iter().find(|c| {
                        !c.orphaned
                            && (c.file_path == path_str
                                || c.file_path == file.new_path.to_string_lossy())
                            && c.target_line_no == lno
                    })
                })
                .or_else(|| {
                    if target_line.is_none() {
                        self.comments.iter().find(|c| {
                            !c.orphaned
                                && (c.file_path == path_str
                                    || c.file_path == file.new_path.to_string_lossy())
                        })
                    } else {
                        None
                    }
                });

            let mut directive_parts = Vec::new();
            for alert in &matching_alerts {
                directive_parts.push(format!("[{}] {}", alert.rule_id, alert.title));
            }
            if let Some(comment) = matching_comment {
                directive_parts.push(format!("DIRECTIVE: {}", comment.comment_text));
            }

            let payload = match target_line {
                Some(line_no) => {
                    if directive_parts.is_empty() {
                        format!("#ref {}:{}", path_str, line_no)
                    } else {
                        let mut msg = format!(
                            "#ref {}:{} {}",
                            path_str,
                            line_no,
                            directive_parts.join(" | ")
                        );
                        if let Some(comment) = matching_comment {
                            for reply in &comment.replies {
                                msg.push_str(&format!("\n↳ Reply: {}", reply));
                            }
                        }
                        msg
                    }
                }
                None => {
                    if directive_parts.is_empty() {
                        format!("#ref {}", path_str)
                    } else {
                        format!("#ref {} {}", path_str, directive_parts.join(" | "))
                    }
                }
            };

            let sink = self.agent_sink();
            let result = sink.send(&payload);
            let label = sink.label();
            drop(sink);
            self.status_message = Some(match result {
                Ok(()) => format!("Sent to {}: {}", label, payload),
                Err(e) => {
                    let _ = crate::ipc::sink::ClipboardSink.send(&payload);
                    format!("Send failed: {}", e)
                }
            });
        }
    }

    pub fn resolved_sink_kind(&self) -> crate::ipc::SinkKind {
        if let Some(kind) = self.agent_sink_kind {
            if kind != crate::ipc::SinkKind::Auto {
                return kind;
            }
        }
        if let Some(ref sink_str) = self.config.agent_sink {
            if let Ok(kind) = sink_str.parse() {
                if kind != crate::ipc::SinkKind::Auto {
                    return kind;
                }
            }
        }
        crate::ipc::IpcBridge::detect_sink()
    }

    pub fn agent_sink(&self) -> Box<dyn crate::ipc::sink::AgentSink + '_> {
        use crate::ipc::sink::*;
        let kind = self.resolved_sink_kind();
        match kind {
            crate::ipc::SinkKind::Tmux => match self.agent_pane.as_deref() {
                Some(pane) => Box::new(TmuxSink(pane)),
                None => Box::new(ClipboardSink),
            },
            crate::ipc::SinkKind::Zellij => Box::new(ZellijSink(self.agent_pane.as_deref())),
            crate::ipc::SinkKind::WezTerm => Box::new(WezTermSink(self.agent_pane.as_deref())),
            crate::ipc::SinkKind::Kitty => Box::new(KittySink(self.agent_pane.as_deref())),
            crate::ipc::SinkKind::Dummy => Box::new(DummySink),
            crate::ipc::SinkKind::Clipboard | crate::ipc::SinkKind::Auto => Box::new(ClipboardSink),
        }
    }

    pub fn open_in_editor(&mut self) {
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let path = self.git_engine.repo_path.join(&file.new_path);
            let mut line_no = 1;
            let mut line_counter = 0;
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    if line_counter == self.cursor_line_idx {
                        line_no = line.new_line_num.or(line.old_line_num).unwrap_or(1);
                        break;
                    }
                    line_counter += 1;
                }
            }

            let editor = self.resolved_editor();
            match launch_editor(&editor, &path, line_no) {
                Ok(_) => {
                    self.status_message = Some(format!(
                        "Opened {}:{} in {}",
                        file.display_name(),
                        line_no,
                        editor
                    ));
                }
                Err(err) => {
                    self.status_message = Some(format!(
                        "Failed to launch editor '{}': {} (use :editor <name>)",
                        editor, err
                    ));
                }
            }
        }
    }

    pub fn open_file_in_editor(&mut self, file_rel_path: &str, line_no: u32) {
        let path = self.git_engine.repo_path.join(file_rel_path);
        let editor = self.resolved_editor();

        match launch_editor(&editor, &path, line_no) {
            Ok(_) => {
                self.status_message = Some(format!(
                    "Opened {}:{} in {}",
                    file_rel_path, line_no, editor
                ));
            }
            Err(err) => {
                self.status_message = Some(format!(
                    "Failed to launch editor '{}': {} (use :editor <name>)",
                    editor, err
                ));
            }
        }
    }
}

fn launch_editor(editor: &str, path: &std::path::Path, line_no: u32) -> Result<(), std::io::Error> {
    let lower = editor.trim().to_lowercase();
    let is_gui = lower == "vscode"
        || lower == "code"
        || lower == "cursor"
        || lower == "subl"
        || lower == "sublime"
        || lower == "idea"
        || lower.starts_with("code ")
        || lower.starts_with("cursor ");

    if !is_gui {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
    }

    let status = match lower.as_str() {
        "vscode" | "code" => std::process::Command::new("code")
            .arg("-g")
            .arg(format!("{}:{}", path.display(), line_no))
            .status(),
        "cursor" => std::process::Command::new("cursor")
            .arg("-g")
            .arg(format!("{}:{}", path.display(), line_no))
            .status(),
        "subl" | "sublime" => std::process::Command::new("subl")
            .arg(format!("{}:{}", path.display(), line_no))
            .status(),
        "idea" => std::process::Command::new("idea")
            .arg("--line")
            .arg(line_no.to_string())
            .arg(path)
            .status(),
        _ => {
            let mut parts = editor.split_whitespace();
            let bin = parts.next().unwrap_or(editor);
            let mut cmd = std::process::Command::new(bin);
            for arg in parts {
                cmd.arg(arg);
            }
            if bin == "hx" || bin == "helix" {
                cmd.arg(format!("{}:{}", path.display(), line_no));
            } else {
                cmd.arg(format!("+{}", line_no));
                cmd.arg(path);
            }
            cmd.status()
        }
    };

    if !is_gui {
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen);
        let _ = crossterm::terminal::enable_raw_mode();
    }

    status.map(|_| ())
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
            let mut line_num = 1;
            let mut line_counter = 0;
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    if line_counter == self.cursor_line_idx {
                        line_num = line.new_line_num.or(line.old_line_num).unwrap_or(1);
                        break;
                    }
                    line_counter += 1;
                }
            }
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
            remote_id: None,
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
            remote_id: None,
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
}
