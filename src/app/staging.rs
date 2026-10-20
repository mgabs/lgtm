use super::*;

impl App {
    pub fn toggle_stage_current_file(&mut self) {
        if !self.git_engine.is_git_repo() {
            self.status_message = Some("Not a git repository".to_string());
            return;
        }
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let path = file.display_name();
            match self.git_engine.stage_file(std::path::Path::new(&path)) {
                Ok(_) => {
                    self.status_message = Some(format!("Staged {}", path));
                    self.reload_diff();
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to stage {}: {}", path, e));
                }
            }
        }
    }

    pub fn discard_current_file_changes(&mut self) {
        if !self.git_engine.is_git_repo() {
            return;
        }
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let path = file.display_name();
            let res = std::process::Command::new("git")
                .current_dir(&self.git_engine.repo_path)
                .args(["restore", &path])
                .status();

            match res {
                Ok(s) if s.success() => {
                    self.status_message = Some(format!("Discarded changes in {}", path));
                    self.reload_diff();
                }
                _ => {
                    self.status_message = Some(format!("Failed to restore {}", path));
                }
            }
        }
    }

    pub fn current_hunk_info(&self) -> Option<(&FileDiff, &DiffHunk, usize)> {
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let mut line_offset = 0;
            for (h_idx, hunk) in file.hunks.iter().enumerate() {
                let hunk_len = hunk.lines.len();
                if self.cursor_line_idx >= line_offset
                    && self.cursor_line_idx < line_offset + hunk_len
                {
                    return Some((file, hunk, h_idx));
                }
                line_offset += hunk_len;
            }
            if let Some(last_hunk) = file.hunks.last() {
                return Some((file, last_hunk, file.hunks.len().saturating_sub(1)));
            }
        }
        None
    }

    pub fn stage_current_hunk(&mut self) {
        if !self.git_engine.is_git_repo() {
            self.status_message = Some("Not a git repository".to_string());
            return;
        }
        if let Some((file, hunk, h_idx)) = self.current_hunk_info() {
            let file_clone = file.clone();
            let hunk_clone = hunk.clone();
            let file_name = file_clone.display_name();
            match self.git_engine.stage_hunk(&file_clone, &hunk_clone) {
                Ok(_) => {
                    self.status_message =
                        Some(format!("✅ Staged hunk {} in {}", h_idx + 1, file_name));
                    self.reload_diff();
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to stage hunk: {}", e));
                }
            }
        }
    }

    pub fn unstage_current_hunk(&mut self) {
        if !self.git_engine.is_git_repo() {
            self.status_message = Some("Not a git repository".to_string());
            return;
        }
        if let Some((file, hunk, h_idx)) = self.current_hunk_info() {
            let file_clone = file.clone();
            let hunk_clone = hunk.clone();
            let file_name = file_clone.display_name();
            match self.git_engine.unstage_hunk(&file_clone, &hunk_clone) {
                Ok(_) => {
                    self.status_message =
                        Some(format!("↩️ Unstaged hunk {} in {}", h_idx + 1, file_name));
                    self.reload_diff();
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to unstage hunk: {}", e));
                }
            }
        }
    }

    pub fn discard_current_hunk(&mut self) {
        if !self.git_engine.is_git_repo() {
            self.status_message = Some("Not a git repository".to_string());
            return;
        }
        if let Some((file, hunk, h_idx)) = self.current_hunk_info() {
            let file_clone = file.clone();
            let hunk_clone = hunk.clone();
            let file_name = file_clone.display_name();
            match self.git_engine.discard_hunk(&file_clone, &hunk_clone) {
                Ok(_) => {
                    self.status_message =
                        Some(format!("🗑️ Discarded hunk {} in {}", h_idx + 1, file_name));
                    self.reload_diff();
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to discard hunk: {}", e));
                }
            }
        }
    }

    pub fn hunk_rejection_prompt(&self, file: &FileDiff, hunk: &DiffHunk) -> (u32, String) {
        let file_name = file.display_name();
        let matching_comment = self.comments.iter().find(|c| {
            !c.orphaned
                && (c.file_path == file_name || c.file_path == file.new_path.to_string_lossy())
                && c.target_line_no >= hunk.new_start
                && c.target_line_no < hunk.new_start + hunk.new_lines.max(1)
        });

        let matching_alert = self.alerts.iter().find(|a| {
            (a.file_path == file_name || a.file_path == file.new_path.to_string_lossy())
                && a.line_no.is_some_and(|l| {
                    l >= hunk.new_start && l < hunk.new_start + hunk.new_lines.max(1)
                })
        });

        let mut directive_parts = Vec::new();
        if let Some(a) = matching_alert {
            directive_parts.push(format!("[{}] {}", a.rule_id, a.title));
        }
        if let Some(c) = matching_comment {
            directive_parts.push(format!("DIRECTIVE: {}", c.comment_text));
            for reply in &c.replies {
                directive_parts.push(format!("↳ Reply: {}", reply));
            }
        }

        let reason = if directive_parts.is_empty() {
            "does not meet requirements".to_string()
        } else {
            directive_parts.join(" | ")
        };

        let line_no = matching_comment
            .map(|c| c.target_line_no)
            .unwrap_or(hunk.new_start);

        let prompt = format!(
            "Reverted hunk in {}:{}, reason: {}. Please fix and re-implement.",
            file_name, line_no, reason
        );
        (line_no, prompt)
    }

    pub fn reject_hunk_and_reprompt(&mut self) {
        if !self.git_engine.is_git_repo() {
            self.status_message = Some("Not a git repository".to_string());
            return;
        }
        if let Some((file, hunk, _h_idx)) = self.current_hunk_info() {
            let file_clone = file.clone();
            let hunk_clone = hunk.clone();
            let (_line_no, prompt) = self.hunk_rejection_prompt(&file_clone, &hunk_clone);

            match self.git_engine.discard_hunk(&file_clone, &hunk_clone) {
                Ok(_) => {
                    let sink = self.agent_sink();
                    let _ = sink.send(&prompt);
                    let label = sink.label();
                    drop(sink);
                    self.status_message = Some(format!(
                        "❌ Reverted hunk & re-prompted {}: {}",
                        label, prompt
                    ));
                    self.reload_diff();
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to revert hunk: {}", e));
                }
            }
        }
    }

    pub fn unstage_current_file(&mut self) {
        if !self.git_engine.is_git_repo() {
            self.status_message = Some("Not a git repository".to_string());
            return;
        }
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let path = file.display_name();
            match self.git_engine.unstage_file(std::path::Path::new(&path)) {
                Ok(_) => {
                    self.status_message = Some(format!("↩️ Unstaged file {}", path));
                    self.reload_diff();
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to unstage file {}: {}", path, e));
                }
            }
        }
    }

    pub fn fold_current_hunk(&mut self) {
        if let Some(file) = self.snapshot.files.get_mut(self.selected_file_idx) {
            let mut line_offset = 0;
            for hunk in file.hunks.iter_mut() {
                let hunk_len = hunk.lines.len();
                if self.cursor_line_idx >= line_offset
                    && self.cursor_line_idx < line_offset + hunk_len
                {
                    hunk.is_folded = true;
                    self.status_message = Some("Folded hunk".to_string());
                    return;
                }
                line_offset += hunk_len;
            }
            if let Some(last_hunk) = file.hunks.last_mut() {
                last_hunk.is_folded = true;
                self.status_message = Some("Folded hunk".to_string());
            }
        }
    }

    pub fn unfold_current_hunk(&mut self) {
        if let Some(file) = self.snapshot.files.get_mut(self.selected_file_idx) {
            let mut line_offset = 0;
            for hunk in file.hunks.iter_mut() {
                let hunk_len = hunk.lines.len();
                if self.cursor_line_idx >= line_offset
                    && self.cursor_line_idx < line_offset + hunk_len
                {
                    hunk.is_folded = false;
                    self.status_message = Some("Unfolded hunk".to_string());
                    return;
                }
                line_offset += hunk_len;
            }
            if let Some(last_hunk) = file.hunks.last_mut() {
                last_hunk.is_folded = false;
                self.status_message = Some("Unfolded hunk".to_string());
            }
        }
    }

    pub fn fold_all_hunks(&mut self) {
        for file in &mut self.snapshot.files {
            for hunk in &mut file.hunks {
                hunk.is_folded = true;
            }
        }
        self.status_message = Some("Folded all hunks in diff".to_string());
    }

    pub fn unfold_all_hunks(&mut self) {
        for file in &mut self.snapshot.files {
            for hunk in &mut file.hunks {
                hunk.is_folded = false;
            }
        }
        self.status_message = Some("Unfolded all hunks in diff".to_string());
    }

    pub fn staged_files_count(&self) -> usize {
        if let Ok(repo) = self.git_engine.open_repo() {
            if let Ok(statuses) = repo.statuses(None) {
                return statuses
                    .iter()
                    .filter(|s| {
                        let st = s.status();
                        st.is_index_new()
                            || st.is_index_modified()
                            || st.is_index_deleted()
                            || st.is_index_renamed()
                            || st.is_index_typechange()
                    })
                    .count();
            }
        }
        0
    }

    pub fn toggle_visual_mode(&mut self) {
        self.visual_mode = !self.visual_mode;
        if self.visual_mode {
            self.visual_anchor_idx = Some(self.cursor_line_idx);
            self.status_message = Some(
                "-- VISUAL LINE SELECTION (s: stage, u: unstage, x: discard, Esc: cancel) --"
                    .to_string(),
            );
        } else {
            self.visual_anchor_idx = None;
            self.status_message = Some("Visual mode exited".to_string());
        }
    }

    pub fn is_in_visual_selection(&self, line_idx: usize) -> bool {
        if !self.visual_mode {
            return false;
        }
        if let Some(anchor) = self.visual_anchor_idx {
            let min = anchor.min(self.cursor_line_idx);
            let max = anchor.max(self.cursor_line_idx);
            line_idx >= min && line_idx <= max
        } else {
            false
        }
    }

    pub fn selected_line_indices_in_current_hunk(
        &self,
    ) -> Option<(&FileDiff, &DiffHunk, Vec<usize>)> {
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let anchor = self.visual_anchor_idx.unwrap_or(self.cursor_line_idx);
            let sel_min = anchor.min(self.cursor_line_idx);
            let sel_max = anchor.max(self.cursor_line_idx);

            let mut line_offset = 0;
            for hunk in &file.hunks {
                let hunk_len = hunk.lines.len();
                let hunk_end = line_offset + hunk_len;

                // Check overlap
                if sel_max >= line_offset && sel_min < hunk_end {
                    let mut hunk_selected_indices = Vec::new();
                    for (h_line_idx, _) in hunk.lines.iter().enumerate() {
                        let global_line_idx = line_offset + h_line_idx;
                        if global_line_idx >= sel_min && global_line_idx <= sel_max {
                            hunk_selected_indices.push(h_line_idx);
                        }
                    }
                    if !hunk_selected_indices.is_empty() {
                        return Some((file, hunk, hunk_selected_indices));
                    }
                }
                line_offset += hunk_len;
            }
        }
        None
    }

    pub fn stage_selected_lines(&mut self) {
        if !self.git_engine.is_git_repo() {
            return;
        }
        if let Some((file, hunk, indices)) = self.selected_line_indices_in_current_hunk() {
            let file_clone = file.clone();
            let hunk_clone = hunk.clone();
            let count = indices.len();
            match self
                .git_engine
                .stage_lines(&file_clone, &hunk_clone, &indices)
            {
                Ok(_) => {
                    self.visual_mode = false;
                    self.visual_anchor_idx = None;
                    self.status_message = Some(format!("✅ Staged {} selected lines", count));
                    self.reload_diff();
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to stage lines: {}", e));
                }
            }
        }
    }

    pub fn unstage_selected_lines(&mut self) {
        if !self.git_engine.is_git_repo() {
            return;
        }
        if let Some((file, hunk, indices)) = self.selected_line_indices_in_current_hunk() {
            let file_clone = file.clone();
            let hunk_clone = hunk.clone();
            let count = indices.len();
            match self
                .git_engine
                .unstage_lines(&file_clone, &hunk_clone, &indices)
            {
                Ok(_) => {
                    self.visual_mode = false;
                    self.visual_anchor_idx = None;
                    self.status_message = Some(format!("↩️ Unstaged {} selected lines", count));
                    self.reload_diff();
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to unstage lines: {}", e));
                }
            }
        }
    }

    pub fn discard_selected_lines(&mut self) {
        if !self.git_engine.is_git_repo() {
            return;
        }
        if let Some((file, hunk, indices)) = self.selected_line_indices_in_current_hunk() {
            let file_clone = file.clone();
            let hunk_clone = hunk.clone();
            let count = indices.len();
            match self
                .git_engine
                .discard_lines(&file_clone, &hunk_clone, &indices)
            {
                Ok(_) => {
                    self.visual_mode = false;
                    self.visual_anchor_idx = None;
                    self.status_message = Some(format!("🗑️ Discarded {} selected lines", count));
                    self.reload_diff();
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to discard lines: {}", e));
                }
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
    fn test_current_hunk_info_and_staging() {
        use crate::model::{ChangeType, DiffHunk, DiffLine, FileDiff, LineType};
        let mut app = create_test_app();
        let hunk1 = DiffHunk {
            header: "@@ -1,2 +1,3 @@".into(),
            old_start: 1,
            old_lines: 2,
            new_start: 1,
            new_lines: 3,
            lines: vec![
                DiffLine {
                    line_type: LineType::Context,
                    content: "fn main() {".into(),
                    old_line_num: Some(1),
                    new_line_num: Some(1),
                },
                DiffLine {
                    line_type: LineType::Addition,
                    content: "    println!(\"hello\");".into(),
                    old_line_num: None,
                    new_line_num: Some(2),
                },
            ],
            is_folded: false,
        };
        let hunk2 = DiffHunk {
            header: "@@ -10,2 +10,3 @@".into(),
            old_start: 10,
            old_lines: 2,
            new_start: 10,
            new_lines: 3,
            lines: vec![DiffLine {
                line_type: LineType::Addition,
                content: "fn helper() {}".into(),
                old_line_num: None,
                new_line_num: Some(10),
            }],
            is_folded: false,
        };

        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/main.rs"),
            change_type: ChangeType::Modified,
            additions: 2,
            deletions: 0,
            hunks: vec![hunk1, hunk2],
            is_folded: false,
            is_reviewed: false,
        };
        app.snapshot = DiffSnapshot::new(vec![file]);

        app.cursor_line_idx = 1;
        let (f, h, h_idx) = app.current_hunk_info().unwrap();
        assert_eq!(f.display_name(), "src/main.rs");
        assert_eq!(h.header, "@@ -1,2 +1,3 @@");
        assert_eq!(h_idx, 0);

        app.cursor_line_idx = 2;
        let (_, h2, h2_idx) = app.current_hunk_info().unwrap();
        assert_eq!(h2.header, "@@ -10,2 +10,3 @@");
        assert_eq!(h2_idx, 1);
    }

    #[test]
    fn test_app_visual_mode_selection() {
        let (_tx, rx) = crossbeam_channel::unbounded();
        let mut app = App::new(PathBuf::from("."), rx, None, None, None);

        assert!(!app.visual_mode);
        app.toggle_visual_mode();
        assert!(app.visual_mode);
        assert_eq!(app.visual_anchor_idx, Some(0));

        app.cursor_line_idx = 3;
        assert!(app.is_in_visual_selection(0));
        assert!(app.is_in_visual_selection(2));
        assert!(app.is_in_visual_selection(3));
        assert!(!app.is_in_visual_selection(4));

        app.toggle_visual_mode();
        assert!(!app.visual_mode);
        assert!(!app.is_in_visual_selection(2));
    }

    #[test]
    fn test_hunk_folding_and_unfolding() {
        let mut app = create_test_app();
        let hunk1 = DiffHunk {
            header: "@@ -1,3 +1,3 @@".into(),
            old_start: 1,
            old_lines: 3,
            new_start: 1,
            new_lines: 3,
            lines: vec![DiffLine {
                line_type: LineType::Addition,
                content: "fn one() {}".into(),
                old_line_num: None,
                new_line_num: Some(1),
            }],
            is_folded: false,
        };
        let hunk2 = DiffHunk {
            header: "@@ -10,3 +10,3 @@".into(),
            old_start: 10,
            old_lines: 3,
            new_start: 10,
            new_lines: 3,
            lines: vec![DiffLine {
                line_type: LineType::Addition,
                content: "fn two() {}".into(),
                old_line_num: None,
                new_line_num: Some(10),
            }],
            is_folded: false,
        };

        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/fold_test.rs"),
            change_type: ChangeType::Modified,
            additions: 2,
            deletions: 0,
            hunks: vec![hunk1, hunk2],
            is_folded: false,
            is_reviewed: false,
        };

        app.snapshot = DiffSnapshot::new(vec![file]);

        // Fold first hunk
        app.cursor_line_idx = 0;
        app.fold_current_hunk();
        assert!(app.snapshot.files[0].hunks[0].is_folded);
        assert!(!app.snapshot.files[0].hunks[1].is_folded);

        // Unfold first hunk
        app.unfold_current_hunk();
        assert!(!app.snapshot.files[0].hunks[0].is_folded);

        // Fold all hunks
        app.fold_all_hunks();
        assert!(app.snapshot.files[0].hunks[0].is_folded);
        assert!(app.snapshot.files[0].hunks[1].is_folded);

        // Unfold all hunks
        app.unfold_all_hunks();
        assert!(!app.snapshot.files[0].hunks[0].is_folded);
        assert!(!app.snapshot.files[0].hunks[1].is_folded);
    }

    #[test]
    fn test_reject_hunk_and_reprompt_includes_directive() {
        use crate::comment::CommentAnchor;
        let mut app = create_test_app();
        app.git_engine = lgtm_core::git::GitEngine::new(".");
        let hunk = DiffHunk {
            header: "@@ -1,3 +1,3 @@".into(),
            old_start: 1,
            old_lines: 3,
            new_start: 1,
            new_lines: 3,
            lines: vec![DiffLine {
                line_type: LineType::Addition,
                content: "let x = 10;".into(),
                old_line_num: None,
                new_line_num: Some(1),
            }],
            is_folded: false,
        };
        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/main.rs"),
            change_type: ChangeType::Modified,
            additions: 1,
            deletions: 0,
            hunks: vec![hunk.clone()],
            is_folded: false,
            is_reviewed: false,
        };
        app.snapshot = DiffSnapshot::new(vec![file.clone()]);
        app.comments.push(CommentAnchor {
            file_path: "src/main.rs".into(),
            target_line_no: 1,
            target_code: "let x = 10;".into(),
            context_before: vec![],
            context_after: vec![],
            comment_text: "Use constant instead of magic number".into(),
            replies: vec![],
            created_at_turn: 1,
            is_resolved: false,
            pr_number: None,
            remote_id: None,
            orphaned: false,
            track: Default::default(),
        });
        let (line_no, prompt) = app.hunk_rejection_prompt(&file, &hunk);
        assert_eq!(line_no, 1);
        assert!(prompt.contains("DIRECTIVE: Use constant instead of magic number"));
        assert!(prompt.contains("Reverted hunk in src/main.rs:1"));
    }
}
