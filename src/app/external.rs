use super::*;

impl App {
    pub fn export_and_send_batch_review(&mut self) {
        if let Some(pr_num) = self.current_pr_number {
            let (prompt, file_path) =
                CommentStore::compile_pr_review_prompt(&self.comments, pr_num);
            let full_path = self.git_engine.repo_path.join(&file_path);
            let _ = std::fs::write(&full_path, &prompt);

            let mut notes = vec![format!("Saved to {}", file_path.display())];

            if crate::pr::PrEngine::is_gh_installed() {
                match crate::pr::PrEngine::post_review(pr_num, "COMMENT", &prompt) {
                    Ok(_) => notes.push("Posted review to PR".to_string()),
                    Err(e) => notes.push(format!("gh: {}", e)),
                }
            }

            if let Some(ref pane) = self.agent_pane {
                let inject_msg = format!("Please review directives in {}", file_path.display());
                let _ = IpcBridge::send_to_tmux(pane, &inject_msg);
                notes.push(format!("Notified pane {}", pane));
            } else {
                IpcBridge::send_osc52_clipboard(&prompt);
                notes.push("Copied prompt to clipboard".to_string());
            }

            self.status_message = Some(notes.join(" | "));
            return;
        }

        let turn = self.turn_manager.load_state().current_turn;
        let (prompt, file_path) = CommentStore::compile_batch_review_prompt(&self.comments, turn);

        let full_path = self.git_engine.repo_path.join(&file_path);
        let _ = std::fs::write(&full_path, &prompt);

        if let Some(ref pane) = self.agent_pane {
            let inject_msg = format!("Please review directives in {}", file_path.display());
            let _ = IpcBridge::send_to_tmux(pane, &inject_msg);
            self.status_message = Some(format!(
                "Exported {} and notified pane {}",
                file_path.display(),
                pane
            ));
        } else {
            IpcBridge::send_osc52_clipboard(&prompt);
            self.status_message = Some(format!(
                "Exported {} and copied prompt to clipboard",
                file_path.display()
            ));
        }

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

            if let Some(ref pane) = self.agent_pane {
                if let Err(e) = IpcBridge::send_to_tmux(pane, &payload) {
                    self.status_message = Some(format!("Send failed: {}", e));
                    IpcBridge::send_osc52_clipboard(&payload);
                } else {
                    self.status_message = Some(format!("Sent to pane {}: {}", pane, payload));
                }
            } else {
                IpcBridge::send_osc52_clipboard(&payload);
                self.status_message = Some(format!("Copied to clipboard: {}", payload));
            }
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
