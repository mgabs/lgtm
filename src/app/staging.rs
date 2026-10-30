use super::*;

impl App {
    fn ensure_working_tree_modifiable(&mut self) -> bool {
        if self.pr.current_number.is_some() || self.active_revision.is_some() {
            self.status_message =
                Some("Staging and discarding are only available on working tree diffs".to_string());
            return false;
        }
        if !self.git_engine.is_git_repo() {
            self.status_message = Some("Not a git repository".to_string());
            return false;
        }
        true
    }

    pub fn toggle_stage_current_file(&mut self) {
        if !self.ensure_working_tree_modifiable() {
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
        if !self.ensure_working_tree_modifiable() {
            return;
        }
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let path = file.display_name();
            let res = std::process::Command::new("git")
                .current_dir(&self.git_engine.repo_path)
                .args(["restore", "--", &path])
                .output();

            match res {
                Ok(out) if out.status.success() => {
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
        if !self.ensure_working_tree_modifiable() {
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
        if !self.ensure_working_tree_modifiable() {
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
        if !self.ensure_working_tree_modifiable() {
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
        if !self.ensure_working_tree_modifiable() {
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
        if !self.ensure_working_tree_modifiable() {
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

    pub fn detect_blocks_in_hunk(
        hunk: &DiffHunk,
        lang: lgtm_ast::Lang,
    ) -> Vec<lgtm_core::model::BlockRange> {
        let src = hunk
            .lines
            .iter()
            .map(|l| l.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        if let Ok(blocks) = lgtm_ast::fold_blocks(lang, &src) {
            blocks
                .into_iter()
                .filter_map(|b| {
                    let start_ln = hunk
                        .lines
                        .get(b.start_row)
                        .and_then(|l| l.new_line_num)
                        .unwrap_or(hunk.new_start + b.start_row as u32);
                    let end_ln = hunk
                        .lines
                        .get(b.end_row)
                        .and_then(|l| l.new_line_num)
                        .unwrap_or(hunk.new_start + b.end_row as u32);
                    if end_ln > start_ln {
                        Some(lgtm_core::model::BlockRange {
                            start_line: start_ln,
                            end_line: end_ln,
                        })
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn toggle_fold_at_cursor(&mut self) {
        let cursor = self.cursor_line_idx;
        let file_path = self
            .snapshot
            .files
            .get(self.selected_file_idx)
            .map(|f| f.new_path.clone());
        let lang = file_path.as_deref().and_then(lgtm_ast::lang_of);

        if let Some(file) = self.snapshot.files.get_mut(self.selected_file_idx) {
            let mut line_offset = 0;
            for hunk in file.hunks.iter_mut() {
                let hunk_len = hunk.lines.len();
                if cursor >= line_offset && cursor < line_offset + hunk_len {
                    let rel_idx = cursor - line_offset;
                    if hunk.is_folded {
                        hunk.is_folded = false;
                        self.status_message = Some("Unfolded hunk".to_string());
                        self.save_marks();
                        return;
                    }

                    if let Some(folded) = Self::is_line_in_folded_block(hunk, rel_idx) {
                        hunk.folded_blocks.retain(|b| *b != folded);
                        self.status_message = Some(format!(
                            "Unfolded AST block (L{}-L{})",
                            folded.start_line, folded.end_line
                        ));
                        self.save_marks();
                        return;
                    }

                    if let Some(l) = lang {
                        let blocks = Self::detect_blocks_in_hunk(hunk, l);
                        let cur_ln = hunk
                            .lines
                            .get(rel_idx)
                            .and_then(|l| l.new_line_num)
                            .unwrap_or(hunk.new_start + rel_idx as u32);
                        let mut candidates: Vec<_> = blocks
                            .into_iter()
                            .filter(|b| {
                                cur_ln >= b.start_line
                                    && cur_ln <= b.end_line
                                    && !hunk.folded_blocks.contains(b)
                            })
                            .collect();
                        candidates.sort_by_key(|b| b.end_line - b.start_line);
                        if let Some(best) = candidates.first() {
                            hunk.folded_blocks.push(*best);
                            self.status_message = Some(format!(
                                "Folded AST block (L{}-L{})",
                                best.start_line, best.end_line
                            ));
                            self.save_marks();
                            return;
                        }
                    }

                    hunk.is_folded = !hunk.is_folded;
                    self.status_message = Some(if hunk.is_folded {
                        "Folded hunk".to_string()
                    } else {
                        "Unfolded hunk".to_string()
                    });
                    self.save_marks();
                    return;
                }
                line_offset += hunk_len;
            }
        }
    }

    pub fn fold_current_hunk(&mut self) {
        let cursor = self.cursor_line_idx;
        let file_path = self
            .snapshot
            .files
            .get(self.selected_file_idx)
            .map(|f| f.new_path.clone());
        let lang = file_path.as_deref().and_then(lgtm_ast::lang_of);

        if let Some(file) = self.snapshot.files.get_mut(self.selected_file_idx) {
            let mut line_offset = 0;
            for hunk in file.hunks.iter_mut() {
                let hunk_len = hunk.lines.len();
                if cursor >= line_offset && cursor < line_offset + hunk_len {
                    let rel_idx = cursor - line_offset;
                    if let Some(l) = lang {
                        let blocks = Self::detect_blocks_in_hunk(hunk, l);
                        let cur_ln = hunk
                            .lines
                            .get(rel_idx)
                            .and_then(|l| l.new_line_num)
                            .unwrap_or(hunk.new_start + rel_idx as u32);
                        let mut candidates: Vec<_> = blocks
                            .into_iter()
                            .filter(|b| {
                                cur_ln >= b.start_line
                                    && cur_ln <= b.end_line
                                    && !hunk.folded_blocks.contains(b)
                            })
                            .collect();
                        candidates.sort_by_key(|b| b.end_line - b.start_line);
                        if let Some(best) = candidates.first() {
                            hunk.folded_blocks.push(*best);
                            self.status_message = Some(format!(
                                "Folded AST block (L{}-L{})",
                                best.start_line, best.end_line
                            ));
                            self.save_marks();
                            return;
                        }
                    }
                    hunk.is_folded = true;
                    self.status_message = Some("Folded hunk".to_string());
                    self.save_marks();
                    return;
                }
                line_offset += hunk_len;
            }
            if let Some(last_hunk) = file.hunks.last_mut() {
                last_hunk.is_folded = true;
                self.status_message = Some("Folded hunk".to_string());
                self.save_marks();
            }
        }
    }

    pub fn unfold_current_hunk(&mut self) {
        let cursor = self.cursor_line_idx;
        if let Some(file) = self.snapshot.files.get_mut(self.selected_file_idx) {
            let mut line_offset = 0;
            for hunk in file.hunks.iter_mut() {
                let hunk_len = hunk.lines.len();
                if cursor >= line_offset && cursor < line_offset + hunk_len {
                    let rel_idx = cursor - line_offset;
                    if let Some(folded) = Self::is_line_in_folded_block(hunk, rel_idx) {
                        hunk.folded_blocks.retain(|b| *b != folded);
                        self.status_message = Some(format!(
                            "Unfolded AST block (L{}-L{})",
                            folded.start_line, folded.end_line
                        ));
                        self.save_marks();
                        return;
                    }
                    hunk.is_folded = false;
                    hunk.folded_blocks.clear();
                    self.status_message = Some("Unfolded hunk".to_string());
                    self.save_marks();
                    return;
                }
                line_offset += hunk_len;
            }
            if let Some(last_hunk) = file.hunks.last_mut() {
                last_hunk.is_folded = false;
                last_hunk.folded_blocks.clear();
                self.status_message = Some("Unfolded hunk".to_string());
                self.save_marks();
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
        self.save_marks();
    }

    pub fn unfold_all_hunks(&mut self) {
        for file in &mut self.snapshot.files {
            for hunk in &mut file.hunks {
                hunk.is_folded = false;
                hunk.folded_blocks.clear();
            }
        }
        self.status_message = Some("Unfolded all hunks in diff".to_string());
        self.save_marks();
    }

    pub fn staged_files_count(&self) -> usize {
        view_data::read_staged_count(&self.git_engine.repo_path)
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
        if !self.ensure_working_tree_modifiable() {
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
        if !self.ensure_working_tree_modifiable() {
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
        if !self.ensure_working_tree_modifiable() {
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
    use super::*;
    use crate::app::test_util::create_test_app;
    use crate::model::*;
    use std::path::PathBuf;

    #[test]
    fn test_current_hunk_info_and_staging() {
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
            folded_blocks: Vec::new(),
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
            folded_blocks: Vec::new(),
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
            folded_blocks: Vec::new(),
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
            folded_blocks: Vec::new(),
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
    fn test_ast_block_folding_and_visible_rows_motion() {
        let mut app = create_test_app();
        let code = [
            "fn compute(a: i32) -> i32 {", // 0 (L1)
            "    let b = a * 2;",          // 1 (L2)
            "    if b > 10 {",             // 2 (L3)
            "        let c = b + 1;",      // 3 (L4)
            "        c * 2",               // 4 (L5)
            "    } else {",                // 5 (L6)
            "        0",                   // 6 (L7)
            "    }",                       // 7 (L8)
            "}",                           // 8 (L9)
        ];
        let lines: Vec<DiffLine> = code
            .iter()
            .enumerate()
            .map(|(i, &c)| DiffLine {
                line_type: LineType::Addition,
                content: c.into(),
                old_line_num: None,
                new_line_num: Some((i + 1) as u32),
            })
            .collect();
        let hunk = DiffHunk {
            header: "@@ -0,0 +1,9 @@".into(),
            old_start: 0,
            old_lines: 0,
            new_start: 1,
            new_lines: 9,
            lines,
            is_folded: false,
            folded_blocks: Vec::new(),
        };
        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/compute.rs"),
            change_type: ChangeType::Added,
            additions: 9,
            deletions: 0,
            hunks: vec![hunk],
            is_folded: false,
            is_reviewed: false,
        };
        app.snapshot = DiffSnapshot::new(vec![file]);
        app.selected_file_idx = 0;

        // Initial visible lines: all 9 lines
        assert_eq!(*app.visible_line_indices(), (0..9).collect::<Vec<_>>());

        // Position cursor on line 3 ("if b > 10 {") -> toggle_fold_at_cursor
        app.cursor_line_idx = 2;
        app.toggle_fold_at_cursor();

        // Innermost AST block (then-block: L3-L6) is now folded!
        let hunk = &app.snapshot.files[0].hunks[0];
        assert_eq!(hunk.folded_blocks.len(), 1);
        assert_eq!(hunk.folded_blocks[0].start_line, 3);
        assert_eq!(hunk.folded_blocks[0].end_line, 6);

        // Visible lines should now have 0, 1, 2 (fold marker), 6, 7, 8
        let visible = app.visible_line_indices();
        assert_eq!(*visible, vec![0, 1, 2, 6, 7, 8]);

        // Motion: moving down from fold marker (idx 2) skips lines 3-5 and lands on 6
        app.cursor_line_idx = 2;
        app.cursor_down();
        assert_eq!(app.cursor_line_idx, 6);

        // Motion: moving up from line 6 lands on fold marker 2
        app.cursor_up();
        assert_eq!(app.cursor_line_idx, 2);

        // Unfold at cursor: toggling again unfolds the block
        app.toggle_fold_at_cursor();
        assert!(app.snapshot.files[0].hunks[0].folded_blocks.is_empty());
        assert_eq!(*app.visible_line_indices(), (0..9).collect::<Vec<_>>());

        // Position cursor on line 0 ("fn compute...") -> toggle_fold_at_cursor folds entire function
        app.cursor_line_idx = 0;
        app.toggle_fold_at_cursor();
        assert_eq!(app.snapshot.files[0].hunks[0].folded_blocks.len(), 1);
        assert_eq!(
            app.snapshot.files[0].hunks[0].folded_blocks[0].start_line,
            1
        );
        assert_eq!(app.snapshot.files[0].hunks[0].folded_blocks[0].end_line, 9);
        assert_eq!(*app.visible_line_indices(), vec![0]);

        // Unfold function
        app.toggle_fold_at_cursor();
        assert!(app.snapshot.files[0].hunks[0].folded_blocks.is_empty());
        assert_eq!(*app.visible_line_indices(), (0..9).collect::<Vec<_>>());
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
            folded_blocks: Vec::new(),
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
            id: lgtm_core::comment::new_id(),
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
            orphaned: false,
            track: Default::default(),
        });
        let (line_no, prompt) = app.hunk_rejection_prompt(&file, &hunk);
        assert_eq!(line_no, 1);
        assert!(prompt.contains("DIRECTIVE: Use constant instead of magic number"));
        assert!(prompt.contains("Reverted hunk in src/main.rs:1"));
    }

    fn git_out(app: &App, args: &[&str]) -> String {
        let out = std::process::Command::new("git")
            .current_dir(&app.git_engine.repo_path)
            .env_remove("GIT_DIR")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_PREFIX")
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    const ORIGINAL: &str = "l1\nl2\nl3\nl4\nl5\nl6\nl7\nl8\nl9\nl10\nl11\nl12\nl13\nl14\nl15\nl16\nl17\nl18\nl19\nl20\n";

    fn modified() -> String {
        ORIGINAL.replace("l2\n", "L2\n").replace("l19\n", "L19\n")
    }

    /// Repo with a committed 20-line a.txt, two separate edits in the working tree, diff loaded.
    fn app_with_two_hunks() -> App {
        let mut app = create_test_app();
        let root = app.git_engine.repo_path.clone();
        std::fs::write(root.join(".git-placeholder"), "").unwrap();
        git_out(&app, &["init", "-q"]);
        std::fs::write(root.join(".git/info/exclude"), ".lgtm/\n.git-placeholder\n").unwrap();
        std::fs::write(root.join("a.txt"), ORIGINAL).unwrap();
        git_out(&app, &["add", "a.txt"]);
        git_out(
            &app,
            &[
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "commit",
                "-q",
                "-m",
                "init",
            ],
        );
        std::fs::write(root.join("a.txt"), modified()).unwrap();
        reload(&mut app);
        assert_eq!(app.snapshot.files.len(), 1);
        assert_eq!(app.snapshot.files[0].hunks.len(), 2);
        app
    }

    fn reload(app: &mut App) {
        app.reload_diff();
        crate::app::test_util::wait_for_local_refresh(app);
    }

    fn disk(app: &App) -> String {
        std::fs::read_to_string(app.git_engine.repo_path.join("a.txt")).unwrap()
    }

    fn second_hunk_cursor(app: &App) -> usize {
        app.snapshot.files[0].hunks[0].lines.len()
    }

    #[test]
    fn test_mutations_blocked_for_pr_and_non_repo() {
        let mut app = create_test_app();
        app.snapshot = DiffSnapshot::new(vec![FileDiff {
            old_path: None,
            new_path: PathBuf::from("a.txt"),
            change_type: ChangeType::Modified,
            additions: 0,
            deletions: 0,
            hunks: vec![],
            is_folded: false,
            is_reviewed: false,
        }]);
        let ops: [fn(&mut App); 11] = [
            App::toggle_stage_current_file,
            App::discard_current_file_changes,
            App::stage_current_hunk,
            App::unstage_current_hunk,
            App::discard_current_hunk,
            App::reject_hunk_and_reprompt,
            App::unstage_current_file,
            App::stage_selected_lines,
            App::unstage_selected_lines,
            App::discard_selected_lines,
            |a| {
                a.active_revision = Some("HEAD".into());
                a.stage_current_hunk()
            },
        ];
        for op in ops {
            app.status_message = None;
            app.pr.current_number = Some(3);
            op(&mut app);
            assert_eq!(
                app.status_message.as_deref(),
                Some("Staging and discarding are only available on working tree diffs")
            );
        }
        app.pr.current_number = None;
        app.active_revision = None;
        app.status_message = None;
        app.toggle_stage_current_file();
        assert_eq!(app.status_message.as_deref(), Some("Not a git repository"));
    }

    #[test]
    fn test_stage_and_unstage_whole_file() {
        let mut app = app_with_two_hunks();
        assert_eq!(app.staged_files_count(), 0);
        app.toggle_stage_current_file();
        assert_eq!(app.staged_files_count(), 1);
        assert!(git_out(&app, &["diff", "--cached"]).contains("+L19"));

        app.unstage_current_file();
        assert!(!git_out(&app, &["diff", "--cached"]).contains("+L2"));
    }

    #[test]
    fn test_stage_and_unstage_hunk_touches_only_current_hunk() {
        let mut app = app_with_two_hunks();
        app.cursor_line_idx = second_hunk_cursor(&app);
        app.stage_current_hunk();
        let staged = git_out(&app, &["diff", "--cached"]);
        assert!(staged.contains("+L19"), "{staged}");
        assert!(!staged.contains("+L2\n"), "{staged}");

        reload(&mut app);
        app.cursor_line_idx = 0;
        app.stage_current_hunk();
        assert!(git_out(&app, &["diff", "--cached"]).contains("+L2\n"));

        reload(&mut app);
        let all = git_out(&app, &["diff", "--cached"]);
        assert!(all.contains("+L19") && all.contains("+L2\n"));
    }

    #[test]
    fn test_discard_hunk_reverts_only_that_hunk() {
        let mut app = app_with_two_hunks();
        app.cursor_line_idx = second_hunk_cursor(&app);
        app.discard_current_hunk();
        assert_eq!(disk(&app), ORIGINAL.replace("l2\n", "L2\n"));
    }

    #[test]
    fn test_reject_hunk_reverts_and_sends_prompt() {
        let mut app = app_with_two_hunks();
        app.cursor_line_idx = 0;
        app.reject_hunk_and_reprompt();
        assert_eq!(disk(&app), ORIGINAL.replace("l19\n", "L19\n"));
    }

    #[test]
    fn test_discard_file_restores_committed_content() {
        let mut app = app_with_two_hunks();
        app.discard_current_file_changes();
        assert_eq!(disk(&app), ORIGINAL);
    }

    #[test]
    fn test_line_level_stage_unstage_discard() {
        let mut app = app_with_two_hunks();
        let idx = app.snapshot.files[0].hunks[0]
            .lines
            .iter()
            .position(|l| l.content == "L2")
            .unwrap();
        app.cursor_line_idx = idx;
        app.visual_anchor_idx = Some(idx - 1);
        app.visual_mode = true;
        app.stage_selected_lines();
        assert!(!app.visual_mode);
        assert!(app.visual_anchor_idx.is_none());
        let staged = git_out(&app, &["diff", "--cached"]);
        assert!(
            staged.contains("+L2\n") && !staged.contains("+L19"),
            "{staged}"
        );

        reload(&mut app);
        let idx = app.snapshot.files[0].hunks[0]
            .lines
            .iter()
            .position(|l| l.content == "L2")
            .unwrap();
        app.cursor_line_idx = idx;
        app.visual_anchor_idx = Some(idx - 1);
        app.visual_mode = true;
        app.unstage_selected_lines();
        assert!(!app.visual_mode);
        assert_eq!(git_out(&app, &["diff", "--cached"]), "");

        reload(&mut app);
        let idx = app.snapshot.files[0].hunks[0]
            .lines
            .iter()
            .position(|l| l.content == "L2")
            .unwrap();
        app.cursor_line_idx = idx;
        app.visual_anchor_idx = Some(idx - 1);
        app.visual_mode = true;
        app.discard_selected_lines();
        assert!(!app.visual_mode);
        assert_eq!(disk(&app), ORIGINAL.replace("l19\n", "L19\n"));
    }

    #[test]
    fn test_stale_snapshot_reports_failures() {
        let mut app = app_with_two_hunks();
        std::fs::write(
            app.git_engine.repo_path.join("a.txt"),
            "completely different\n",
        )
        .unwrap();
        git_out(
            &app,
            &[
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "commit",
                "-q",
                "-am",
                "diverge",
            ],
        );
        app.cursor_line_idx = 0;

        type Case = (fn(&mut App), &'static str);
        let cases: [Case; 7] = [
            (App::stage_current_hunk, "Failed to stage hunk"),
            (App::unstage_current_hunk, "Failed to unstage hunk"),
            (App::discard_current_hunk, "Failed to discard hunk"),
            (App::reject_hunk_and_reprompt, "Failed to revert hunk"),
            (
                |a| {
                    a.toggle_visual_mode();
                    a.stage_selected_lines()
                },
                "Failed to stage lines",
            ),
            (|a| a.unstage_selected_lines(), "Failed to unstage lines"),
            (|a| a.discard_selected_lines(), "Failed to discard lines"),
        ];
        for (op, expected) in cases {
            app.status_message = None;
            op(&mut app);
            let status = app.status_message.clone().unwrap_or_default();
            assert!(status.starts_with(expected), "{expected}: {status}");
        }
        assert_eq!(disk(&app), "completely different\n");
    }

    #[test]
    fn test_file_level_failures_reported() {
        let mut app = app_with_two_hunks();
        app.snapshot.files[0].new_path = PathBuf::from("missing.txt");
        app.snapshot.files[0].old_path = None;
        app.toggle_stage_current_file();
        assert!(app
            .status_message
            .as_deref()
            .unwrap()
            .starts_with("Failed to stage missing.txt: "));
        app.discard_current_file_changes();
        assert_eq!(
            app.status_message.as_deref(),
            Some("Failed to restore missing.txt")
        );
    }

    fn rust_file_app() -> App {
        let mut app = create_test_app();
        let code = [
            "fn compute(a: i32) -> i32 {",
            "    let b = a * 2;",
            "    if b > 10 {",
            "        let c = b + 1;",
            "        c * 2",
            "    } else {",
            "        0",
            "    }",
            "}",
        ];
        let lines = code
            .iter()
            .enumerate()
            .map(|(i, &c)| DiffLine {
                line_type: LineType::Addition,
                content: c.into(),
                old_line_num: None,
                new_line_num: Some(i as u32 + 1),
            })
            .collect();
        app.snapshot = DiffSnapshot::new(vec![FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/compute.rs"),
            change_type: ChangeType::Added,
            additions: 9,
            deletions: 0,
            hunks: vec![DiffHunk {
                header: "@@ -0,0 +1,9 @@".into(),
                old_start: 0,
                old_lines: 0,
                new_start: 1,
                new_lines: 9,
                lines,
                is_folded: false,
                folded_blocks: Vec::new(),
            }],
            is_folded: false,
            is_reviewed: false,
        }]);
        app
    }

    #[test]
    fn test_fold_current_hunk_prefers_innermost_ast_block_then_unfolds() {
        let mut app = rust_file_app();
        app.cursor_line_idx = 3;
        app.fold_current_hunk();
        assert_eq!(
            app.status_message.as_deref(),
            Some("Folded AST block (L3-L6)")
        );
        let blocks = &app.snapshot.files[0].hunks[0].folded_blocks;
        assert_eq!((blocks[0].start_line, blocks[0].end_line), (3, 6));

        app.cursor_line_idx = 0;
        app.fold_current_hunk();
        assert_eq!(
            app.status_message.as_deref(),
            Some("Folded AST block (L1-L9)")
        );

        app.unfold_current_hunk();
        assert_eq!(
            app.status_message.as_deref(),
            Some("Unfolded AST block (L1-L9)")
        );
        assert_eq!(app.snapshot.files[0].hunks[0].folded_blocks.len(), 1);

        app.cursor_line_idx = 99;
        app.unfold_current_hunk();
        assert_eq!(app.status_message.as_deref(), Some("Unfolded hunk"));
        assert!(app.snapshot.files[0].hunks[0].folded_blocks.is_empty());
    }

    #[test]
    fn test_fold_falls_back_to_whole_hunk_without_blocks() {
        let mut app = rust_file_app();
        app.snapshot.files[0].new_path = PathBuf::from("notes.unknownext");
        app.cursor_line_idx = 1;
        app.toggle_fold_at_cursor();
        assert_eq!(app.status_message.as_deref(), Some("Folded hunk"));
        assert!(app.snapshot.files[0].hunks[0].is_folded);
        app.toggle_fold_at_cursor();
        assert_eq!(app.status_message.as_deref(), Some("Unfolded hunk"));
        assert!(!app.snapshot.files[0].hunks[0].is_folded);

        // Cursor past the end folds / unfolds the last hunk.
        app.cursor_line_idx = 50;
        app.fold_current_hunk();
        assert!(app.snapshot.files[0].hunks[0].is_folded);
        app.unfold_current_hunk();
        assert!(!app.snapshot.files[0].hunks[0].is_folded);
    }

    #[test]
    fn test_toggle_fold_unfolds_hunk_and_blocks_with_lang() {
        let mut app = rust_file_app();
        app.snapshot.files[0].hunks[0].is_folded = true;
        app.cursor_line_idx = 0;
        app.toggle_fold_at_cursor();
        assert_eq!(app.status_message.as_deref(), Some("Unfolded hunk"));
        assert!(!app.snapshot.files[0].hunks[0].is_folded);

        // A line outside any block (a closing brace row already inside L1-L9) still folds the outer one.
        app.cursor_line_idx = 8;
        app.toggle_fold_at_cursor();
        assert_eq!(
            app.status_message.as_deref(),
            Some("Folded AST block (L1-L9)")
        );
        app.toggle_fold_at_cursor();
        assert_eq!(
            app.status_message.as_deref(),
            Some("Unfolded AST block (L1-L9)")
        );
    }

    #[test]
    fn test_selected_lines_span_and_empty() {
        let mut app = rust_file_app();
        app.cursor_line_idx = 4;
        app.visual_anchor_idx = Some(2);
        let (_, _, idx) = app.selected_line_indices_in_current_hunk().unwrap();
        assert_eq!(idx, vec![2, 3, 4]);

        app.visual_anchor_idx = None;
        let (_, _, idx) = app.selected_line_indices_in_current_hunk().unwrap();
        assert_eq!(idx, vec![4]);

        app.cursor_line_idx = 30;
        assert!(app.selected_line_indices_in_current_hunk().is_none());
        app.snapshot = DiffSnapshot::new(vec![]);
        assert!(app.current_hunk_info().is_none());
    }

    #[test]
    fn test_rejection_prompt_combines_alert_comment_and_replies() {
        use crate::comment::CommentAnchor;
        let mut app = rust_file_app();
        let file = app.snapshot.files[0].clone();
        let hunk = file.hunks[0].clone();
        let (line, prompt) = app.hunk_rejection_prompt(&file, &hunk);
        assert_eq!(line, 1);
        assert_eq!(
            prompt,
            "Reverted hunk in src/compute.rs:1, reason: does not meet requirements. Please fix and re-implement."
        );

        app.alerts.push(crate::guard::GuardAlert {
            rule_id: "r/x".into(),
            fingerprint: "f".into(),
            severity: crate::guard::Severity::High,
            title: "Bad".into(),
            description: "d".into(),
            line_no: Some(3),
            file_path: "src/compute.rs".into(),
        });
        app.comments.push(CommentAnchor {
            id: lgtm_core::comment::new_id(),
            file_path: "src/compute.rs".into(),
            target_line_no: 4,
            target_code: String::new(),
            context_before: vec![],
            context_after: vec![],
            comment_text: "why".into(),
            replies: vec!["because".into()],
            created_at_turn: 1,
            is_resolved: false,
            pr_number: None,
            orphaned: false,
            track: Default::default(),
        });
        let (line, prompt) = app.hunk_rejection_prompt(&file, &hunk);
        assert_eq!(line, 4);
        assert_eq!(
            prompt,
            "Reverted hunk in src/compute.rs:4, reason: [r/x] Bad | DIRECTIVE: why | ↳ Reply: because. Please fix and re-implement."
        );
    }

    #[test]
    fn test_detect_blocks_ignores_single_line_blocks() {
        let mk = |c: &str, n: u32| DiffLine {
            line_type: LineType::Addition,
            content: c.into(),
            old_line_num: None,
            new_line_num: Some(n),
        };
        let hunk = DiffHunk {
            header: String::new(),
            old_start: 0,
            old_lines: 0,
            new_start: 1,
            new_lines: 1,
            lines: vec![mk("fn f() {}", 1)],
            is_folded: false,
            folded_blocks: Vec::new(),
        };
        assert!(App::detect_blocks_in_hunk(&hunk, lgtm_ast::Lang::Rust).is_empty());
    }
}
