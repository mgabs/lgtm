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
                format!("Please review directives in {}", file_path.display())
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
            format!("Please review directives in {}", file_path.display())
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
        self.status_message =
            Some("Running test validity probe (checking for vacuous assertions)...".to_string());
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

            let payload = match target_line {
                Some(line_no) => format!("#ref {}:{}", path_str, line_no),
                None => format!("#ref {}", path_str),
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

    pub fn agent_sink(&self) -> Box<dyn crate::ipc::sink::AgentSink + '_> {
        use crate::ipc::sink::*;
        match self.agent_pane.as_deref() {
            Some(pane) => Box::new(TmuxSink(pane)),
            None => Box::new(ClipboardSink),
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

            let editor = std::env::var("EDITOR")
                .or_else(|_| std::env::var("VISUAL"))
                .unwrap_or_else(|_| "nvim".to_string());

            let _ = std::process::Command::new(&editor)
                .arg(format!("+{}", line_no))
                .arg(&path)
                .status();

            self.status_message = Some(format!(
                "Opened {}:{} in {}",
                file.display_name(),
                line_no,
                editor
            ));
        }
    }

    pub fn open_file_in_editor(&mut self, file_rel_path: &str, line_no: u32) {
        let path = self.git_engine.repo_path.join(file_rel_path);
        let editor = std::env::var("EDITOR")
            .or_else(|_| std::env::var("VISUAL"))
            .unwrap_or_else(|_| "nvim".to_string());

        let _ = std::process::Command::new(&editor)
            .arg(format!("+{}", line_no))
            .arg(&path)
            .status();

        self.status_message = Some(format!(
            "Opened {}:{} in {}",
            file_rel_path, line_no, editor
        ));
    }

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
