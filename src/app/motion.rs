use super::*;

impl App {
    pub fn jump_to_file_and_line(&mut self, file_path: &str, target_line: u32) {
        let clean_path = file_path.trim_start_matches("./");
        if let Some(idx) = self.snapshot.files.iter().position(|f| {
            let dname = f.display_name();
            let clean_dname = dname.trim_start_matches("./");
            clean_dname == clean_path
                || clean_dname.ends_with(clean_path)
                || clean_path.ends_with(clean_dname)
        }) {
            self.selected_file_idx = idx;
            if let Some(f) = self.snapshot.files.get_mut(idx) {
                f.is_folded = false;
            }
            self.cursor_line_idx = 0;
            let mut line_counter = 0;
            let mut closest_idx = None;
            let mut min_diff = u32::MAX;

            if let Some(f) = self.snapshot.files.get(idx) {
                for hunk in &f.hunks {
                    for line in &hunk.lines {
                        let l_no = line.new_line_num.or(line.old_line_num);

                        if let Some(line_num) = l_no {
                            if line_num == target_line {
                                self.cursor_line_idx = line_counter;
                                self.diff_scroll_offset = line_counter.saturating_sub(10);
                                self.status_message =
                                    Some(format!("Jumped to {}:{}", clean_path, target_line));
                                return;
                            }
                            let diff = (line_num as i64 - target_line as i64).unsigned_abs() as u32;
                            if diff < min_diff {
                                min_diff = diff;
                                closest_idx = Some(line_counter);
                            }
                        }
                        line_counter += 1;
                    }
                }
            }

            if let Some(c_idx) = closest_idx {
                self.cursor_line_idx = c_idx;
                self.diff_scroll_offset = c_idx.saturating_sub(10);
            }
            self.status_message = Some(format!("Jumped to {}:{}", clean_path, target_line));
        } else {
            self.status_message = Some(format!("File '{}' is not in the active diff", clean_path));
        }
    }

    pub(crate) fn update_file_picker_filter(&mut self) {
        let names: Vec<String> = self
            .snapshot
            .files
            .iter()
            .map(|f| f.display_name())
            .collect();
        self.file_picker.refilter(&names);
    }

    pub fn next_file(&mut self) {
        if self.snapshot.files.is_empty() {
            self.status_message = Some("No changed files in diff".to_string());
            return;
        }
        let total = self.snapshot.files.len();
        if self.selected_file_idx + 1 < total {
            self.selected_file_idx += 1;
            self.diff_scroll_offset = 0;
            self.cursor_line_idx = 0;
            if let Some(file) = self.snapshot.files.get_mut(self.selected_file_idx) {
                file.is_folded = false;
            }
            let name = self.snapshot.files[self.selected_file_idx].display_name();
            self.status_message = Some(format!(
                "File [{}/{}]: {}",
                self.selected_file_idx + 1,
                total,
                name
            ));
        } else {
            let name = self.snapshot.files[self.selected_file_idx].display_name();
            self.status_message = Some(format!(
                "Already at last file [{}/{}]: {}",
                total, total, name
            ));
        }
    }

    pub fn prev_file(&mut self) {
        if self.snapshot.files.is_empty() {
            self.status_message = Some("No changed files in diff".to_string());
            return;
        }
        let total = self.snapshot.files.len();
        if self.selected_file_idx > 0 {
            self.selected_file_idx -= 1;
            self.diff_scroll_offset = 0;
            self.cursor_line_idx = 0;
            if let Some(file) = self.snapshot.files.get_mut(self.selected_file_idx) {
                file.is_folded = false;
            }
            let name = self.snapshot.files[self.selected_file_idx].display_name();
            self.status_message = Some(format!(
                "File [{}/{}]: {}",
                self.selected_file_idx + 1,
                total,
                name
            ));
        } else {
            let name = self.snapshot.files[0].display_name();
            self.status_message = Some(format!("Already at first file [1/{}]: {}", total, name));
        }
    }

    pub fn next_alert(&mut self) {
        if self.alerts.is_empty() {
            self.status_message = Some("No warnings detected".to_string());
            return;
        }
        self.selected_alert_idx = (self.selected_alert_idx + 1) % self.alerts.len();
        self.jump_to_selected_alert();
    }

    pub fn prev_alert(&mut self) {
        if self.alerts.is_empty() {
            self.status_message = Some("No warnings detected".to_string());
            return;
        }
        if self.selected_alert_idx == 0 {
            self.selected_alert_idx = self.alerts.len() - 1;
        } else {
            self.selected_alert_idx -= 1;
        }
        self.jump_to_selected_alert();
    }

    pub(crate) fn jump_to_selected_alert(&mut self) {
        let alert = &self.alerts[self.selected_alert_idx];
        let badge = alert.severity.badge();
        let title = alert.title.clone();
        let file_path = alert.file_path.clone();
        let target_line = alert.line_no;

        if let Some(idx) = self
            .snapshot
            .files
            .iter()
            .position(|f| f.display_name() == file_path)
        {
            self.selected_file_idx = idx;
            self.cursor_line_idx = 0;
            if let Some(lno) = target_line {
                let mut line_counter = 0;
                if let Some(f) = self.snapshot.files.get(idx) {
                    for hunk in &f.hunks {
                        for line in &hunk.lines {
                            let line_num = line.new_line_num.or(line.old_line_num);
                            if line_num == Some(lno) {
                                self.cursor_line_idx = line_counter;
                                break;
                            }
                            line_counter += 1;
                        }
                    }
                }
            }
        }
        self.status_message = Some(format!("{}: {}", badge, title));
    }

    pub fn cursor_down(&mut self) {
        let total = self.current_file_line_count();
        if total > 0 && self.cursor_line_idx + 1 < total {
            self.cursor_line_idx += 1;
        }
    }

    pub fn cursor_up(&mut self) {
        if self.cursor_line_idx > 0 {
            self.cursor_line_idx -= 1;
        }
    }

    pub fn current_file_line_count(&self) -> usize {
        self.snapshot
            .files
            .get(self.selected_file_idx)
            .map(|f| f.hunks.iter().map(|h| h.lines.len()).sum())
            .unwrap_or(0)
    }

    pub fn cursor_to_top(&mut self) {
        self.cursor_line_idx = 0;
        self.diff_scroll_offset = 0;
    }

    pub fn cursor_to_bottom(&mut self) {
        let total = self.current_file_line_count();
        if total > 0 {
            self.cursor_line_idx = total - 1;
        }
    }

    pub fn page_down(&mut self, amount: usize) {
        let total = self.current_file_line_count();
        if total > 0 {
            self.cursor_line_idx = (self.cursor_line_idx + amount).min(total - 1);
        }
    }

    pub fn page_up(&mut self, amount: usize) {
        if self.cursor_line_idx >= amount {
            self.cursor_line_idx -= amount;
        } else {
            self.cursor_line_idx = 0;
        }
    }

    pub fn center_cursor(&mut self) {
        self.diff_scroll_offset = self.cursor_line_idx.saturating_sub(15);
    }

    pub fn cursor_to_top_of_view(&mut self) {
        self.diff_scroll_offset = self.cursor_line_idx;
    }

    pub fn cursor_to_bottom_of_view(&mut self) {
        self.diff_scroll_offset = self.cursor_line_idx.saturating_sub(30);
    }

    pub fn next_unreviewed_file(&mut self) {
        if self.snapshot.files.is_empty() {
            return;
        }
        let total = self.snapshot.files.len();
        for i in (self.selected_file_idx + 1)..total {
            if !self.snapshot.files[i].is_reviewed {
                self.selected_file_idx = i;
                self.diff_scroll_offset = 0;
                self.cursor_line_idx = 0;
                self.snapshot.files[i].is_folded = false;
                let name = self.snapshot.files[i].display_name();
                self.status_message =
                    Some(format!("Unreviewed file [{}/{}]: {}", i + 1, total, name));
                return;
            }
        }
        for i in 0..self.selected_file_idx {
            if !self.snapshot.files[i].is_reviewed {
                self.selected_file_idx = i;
                self.diff_scroll_offset = 0;
                self.cursor_line_idx = 0;
                self.snapshot.files[i].is_folded = false;
                let name = self.snapshot.files[i].display_name();
                self.status_message =
                    Some(format!("Unreviewed file [{}/{}]: {}", i + 1, total, name));
                return;
            }
        }
        self.status_message = Some("All files marked reviewed! 🎉".to_string());
    }

    pub fn prev_unreviewed_file(&mut self) {
        if self.snapshot.files.is_empty() {
            return;
        }
        let total = self.snapshot.files.len();
        for i in (0..self.selected_file_idx).rev() {
            if !self.snapshot.files[i].is_reviewed {
                self.selected_file_idx = i;
                self.diff_scroll_offset = 0;
                self.cursor_line_idx = 0;
                self.snapshot.files[i].is_folded = false;
                let name = self.snapshot.files[i].display_name();
                self.status_message =
                    Some(format!("Unreviewed file [{}/{}]: {}", i + 1, total, name));
                return;
            }
        }
        for i in ((self.selected_file_idx + 1)..total).rev() {
            if !self.snapshot.files[i].is_reviewed {
                self.selected_file_idx = i;
                self.diff_scroll_offset = 0;
                self.cursor_line_idx = 0;
                self.snapshot.files[i].is_folded = false;
                let name = self.snapshot.files[i].display_name();
                self.status_message =
                    Some(format!("Unreviewed file [{}/{}]: {}", i + 1, total, name));
                return;
            }
        }
        self.status_message = Some("All files marked reviewed! 🎉".to_string());
    }

    pub fn next_hunk(&mut self) {
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let mut line_offset = 0;
            let mut target_cursor = None;
            for hunk in &file.hunks {
                if line_offset > self.cursor_line_idx {
                    target_cursor = Some(line_offset);
                    break;
                }
                line_offset += hunk.lines.len();
            }

            if let Some(target) = target_cursor {
                self.cursor_line_idx = target;
            } else if self.selected_file_idx + 1 < self.snapshot.files.len() {
                self.next_file();
            }
        }
    }

    pub fn prev_hunk(&mut self) {
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let mut hunk_starts = Vec::new();
            let mut line_offset = 0;
            for hunk in &file.hunks {
                hunk_starts.push(line_offset);
                line_offset += hunk.lines.len();
            }

            let mut target = None;
            for &start in hunk_starts.iter().rev() {
                if start < self.cursor_line_idx {
                    target = Some(start);
                    break;
                }
            }

            if let Some(t) = target {
                self.cursor_line_idx = t;
            } else if self.selected_file_idx > 0 {
                self.prev_file();
                self.cursor_to_bottom();
            }
        }
    }

    pub fn next_paragraph(&mut self) {
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let mut lines: Vec<&str> = Vec::new();
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    lines.push(&line.content);
                }
            }
            if lines.is_empty() {
                return;
            }
            let mut target = None;
            let mut found_non_empty = false;
            for (i, content) in lines.iter().enumerate().skip(self.cursor_line_idx + 1) {
                let is_blank = content.trim().is_empty();
                if !is_blank {
                    found_non_empty = true;
                } else if found_non_empty && is_blank {
                    target = Some(i);
                    break;
                }
            }
            if let Some(t) = target {
                self.cursor_line_idx = t;
            } else if !lines.is_empty() {
                self.cursor_line_idx = lines.len() - 1;
            }
        }
    }

    pub fn prev_paragraph(&mut self) {
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let mut lines: Vec<&str> = Vec::new();
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    lines.push(&line.content);
                }
            }
            if lines.is_empty() {
                return;
            }
            let mut target = None;
            let mut found_non_empty = false;
            for i in (0..self.cursor_line_idx).rev() {
                let is_blank = lines[i].trim().is_empty();
                if !is_blank {
                    found_non_empty = true;
                } else if found_non_empty && is_blank {
                    target = Some(i);
                    break;
                }
            }
            if let Some(t) = target {
                self.cursor_line_idx = t;
            } else {
                self.cursor_line_idx = 0;
            }
        }
    }

    pub fn next_comment(&mut self) {
        if self.comments.is_empty() {
            self.status_message = Some("No comments saved".to_string());
            return;
        }
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let path_str = file.display_name();
            let mut line_counter = 0;
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    let lno = line.new_line_num.or(line.old_line_num);
                    if line_counter > self.cursor_line_idx {
                        if let Some(num) = lno {
                            if self
                                .comments
                                .iter()
                                .any(|c| c.file_path == path_str && c.target_line_no == num)
                            {
                                self.cursor_line_idx = line_counter;
                                self.status_message =
                                    Some(format!("Comment at {}:{}", path_str, num));
                                return;
                            }
                        }
                    }
                    line_counter += 1;
                }
            }
        }
        self.status_message = Some("End of comments in file".to_string());
    }

    pub fn prev_comment(&mut self) {
        if self.comments.is_empty() {
            self.status_message = Some("No comments saved".to_string());
            return;
        }
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let path_str = file.display_name();
            let mut match_line = None;
            let mut line_counter = 0;
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    let lno = line.new_line_num.or(line.old_line_num);
                    if line_counter < self.cursor_line_idx {
                        if let Some(num) = lno {
                            if self
                                .comments
                                .iter()
                                .any(|c| c.file_path == path_str && c.target_line_no == num)
                            {
                                match_line = Some(line_counter);
                            }
                        }
                    }
                    line_counter += 1;
                }
            }
            if let Some(idx) = match_line {
                self.cursor_line_idx = idx;
                self.status_message = Some("Jumped to previous comment".to_string());
            }
        }
    }

    /// `]t` / `[t`: jump to the next or previous line with a remote thread in the current file.
    pub fn jump_thread(&mut self, forward: bool) {
        if self.threads.is_empty() {
            self.status_message = Some("No threads; run :threads first".to_string());
            return;
        }
        let Some(file) = self.snapshot.files.get(self.selected_file_idx) else {
            return;
        };
        let path = file.display_name();
        let rows = file
            .hunks
            .iter()
            .flat_map(|h| &h.lines)
            .enumerate()
            .filter(|(_, l)| {
                l.new_line_num.is_some_and(|n| {
                    lgtm_core::github::thread_marker(&self.threads, &path, n).is_some()
                })
            })
            .map(|(i, _)| i);
        let cur = self.cursor_line_idx;
        let hit = if forward {
            rows.filter(|&i| i > cur).min()
        } else {
            rows.filter(|&i| i < cur).max()
        };
        match hit {
            Some(i) => self.cursor_line_idx = i,
            None => self.status_message = Some("No more threads in this file".to_string()),
        }
    }

    pub fn execute_search(&mut self, query: &str) {
        self.search_matches.clear();
        let q = query.to_lowercase();
        if q.is_empty() {
            return;
        }

        for (f_idx, file) in self.snapshot.files.iter().enumerate() {
            let mut line_counter = 0;
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    if line.content.to_lowercase().contains(&q) {
                        self.search_matches.push((f_idx, line_counter));
                    }
                    line_counter += 1;
                }
            }
        }

        if !self.search_matches.is_empty() {
            self.search_match_idx = 0;
            let (f_idx, l_idx) = self.search_matches[0];
            self.selected_file_idx = f_idx;
            self.cursor_line_idx = l_idx;
            self.status_message = Some(format!(
                "Match 1/{} for '{}'",
                self.search_matches.len(),
                query
            ));
        } else {
            self.status_message = Some(format!("Pattern not found: '{}'", query));
        }
    }

    pub fn next_search_match(&mut self) {
        if self.search_matches.is_empty() {
            if !self.search_query.is_empty() {
                self.execute_search(&self.search_query.clone());
            }
            if self.search_matches.is_empty() {
                self.status_message =
                    Some("No search pattern active (press / to search)".to_string());
                return;
            }
        }
        self.search_match_idx = (self.search_match_idx + 1) % self.search_matches.len();
        let (f_idx, l_idx) = self.search_matches[self.search_match_idx];
        self.selected_file_idx = f_idx;
        self.cursor_line_idx = l_idx;
        self.status_message = Some(format!(
            "Match {}/{} for '{}'",
            self.search_match_idx + 1,
            self.search_matches.len(),
            self.search_query
        ));
    }

    pub fn prev_search_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        if self.search_match_idx == 0 {
            self.search_match_idx = self.search_matches.len() - 1;
        } else {
            self.search_match_idx -= 1;
        }
        let (f_idx, l_idx) = self.search_matches[self.search_match_idx];
        self.selected_file_idx = f_idx;
        self.cursor_line_idx = l_idx;
        self.status_message = Some(format!(
            "Match {}/{} for '{}'",
            self.search_match_idx + 1,
            self.search_matches.len(),
            self.search_query
        ));
    }

    pub fn search_symbol_under_cursor(&mut self, forward: bool) {
        let mut target_word = None;
        if let Some(file) = self.snapshot.files.get(self.selected_file_idx) {
            let mut line_counter = 0;
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    if line_counter == self.cursor_line_idx {
                        let text = line.content.trim();
                        let words: Vec<&str> = text
                            .split(|c: char| !c.is_alphanumeric() && c != '_')
                            .filter(|w| !w.is_empty())
                            .collect();
                        target_word = words.first().map(|&s| s.to_string());
                        break;
                    }
                    line_counter += 1;
                }
                if target_word.is_some() {
                    break;
                }
            }
        }

        if let Some(word) = target_word {
            self.search_query = word.clone();
            self.execute_search(&word);
            if !forward {
                self.prev_search_match();
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
    fn test_paragraph_jumps_and_page_scroll() {
        use crate::model::{ChangeType, DiffHunk, DiffLine, FileDiff, LineType};
        let mut app = create_test_app();
        let lines = vec![
            DiffLine {
                line_type: LineType::Context,
                content: "fn main() {".into(),
                old_line_num: Some(1),
                new_line_num: Some(1),
            },
            DiffLine {
                line_type: LineType::Addition,
                content: "    let x = 1;".into(),
                old_line_num: None,
                new_line_num: Some(2),
            },
            DiffLine {
                line_type: LineType::Context,
                content: "".into(),
                old_line_num: Some(2),
                new_line_num: Some(3),
            }, // blank paragraph 1
            DiffLine {
                line_type: LineType::Addition,
                content: "    let y = 2;".into(),
                old_line_num: None,
                new_line_num: Some(4),
            },
            DiffLine {
                line_type: LineType::Context,
                content: "".into(),
                old_line_num: Some(3),
                new_line_num: Some(5),
            }, // blank paragraph 2
            DiffLine {
                line_type: LineType::Context,
                content: "}".into(),
                old_line_num: Some(4),
                new_line_num: Some(6),
            },
        ];
        let hunk = DiffHunk {
            header: "@@ -1,4 +1,6 @@".into(),
            old_start: 1,
            old_lines: 4,
            new_start: 1,
            new_lines: 6,
            lines,
            is_folded: false,
        };
        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("test.rs"),
            change_type: ChangeType::Modified,
            additions: 2,
            deletions: 0,
            hunks: vec![hunk],
            is_folded: false,
            is_reviewed: false,
        };
        app.snapshot = DiffSnapshot::new(vec![file]);

        app.cursor_line_idx = 0;
        app.next_paragraph();
        assert_eq!(app.cursor_line_idx, 2);

        app.next_paragraph();
        assert_eq!(app.cursor_line_idx, 4);

        app.prev_paragraph();
        assert_eq!(app.cursor_line_idx, 2);

        app.prev_paragraph();
        assert_eq!(app.cursor_line_idx, 0);

        app.page_down(3);
        assert_eq!(app.cursor_line_idx, 3);
        app.page_up(2);
        assert_eq!(app.cursor_line_idx, 1);
    }

    #[test]
    fn thread_jump_moves_between_marked_lines() {
        use crate::model::{ChangeType, DiffHunk, DiffLine, FileDiff, LineType};
        let mut app = create_test_app();
        let lines = (1..=6)
            .map(|n| DiffLine {
                line_type: LineType::Context,
                content: "x".into(),
                old_line_num: Some(n),
                new_line_num: Some(n),
            })
            .collect();
        let hunk = DiffHunk {
            header: "@@".into(),
            old_start: 1,
            old_lines: 6,
            new_start: 1,
            new_lines: 6,
            lines,
            is_folded: false,
        };
        app.snapshot = DiffSnapshot::new(vec![FileDiff {
            old_path: None,
            new_path: PathBuf::from("t.rs"),
            change_type: ChangeType::Modified,
            additions: 0,
            deletions: 0,
            hunks: vec![hunk],
            is_folded: false,
            is_reviewed: false,
        }]);
        let th = |line| lgtm_core::github::Thread {
            id: "1".into(),
            path: "t.rs".into(),
            line: Some(line),
            resolved: false,
            comments: vec![],
        };
        app.jump_thread(true);
        assert!(app.status_message.as_deref().unwrap().contains(":threads"));
        app.threads = vec![th(2), th(5)];
        app.cursor_line_idx = 0;
        app.jump_thread(true);
        assert_eq!(app.cursor_line_idx, 1);
        app.jump_thread(true);
        assert_eq!(app.cursor_line_idx, 4);
        app.jump_thread(false);
        assert_eq!(app.cursor_line_idx, 1);

        // Inline reply & resolve from marker
        assert_eq!(app.thread_at_cursor(), Some(0));
        let key_r = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE);
        app.handle_diff_key(key_r);
        assert_eq!(app.active_view, ActiveView::CommandLine);
        assert_eq!(app.command.input, "thread reply 1 ");
    }

    #[test]
    fn test_unreviewed_file_navigation() {
        use crate::model::{ChangeType, FileDiff};
        let mut app = create_test_app();
        let f1 = FileDiff {
            old_path: None,
            new_path: PathBuf::from("a.rs"),
            change_type: ChangeType::Modified,
            additions: 1,
            deletions: 0,
            hunks: vec![],
            is_folded: false,
            is_reviewed: true,
        };
        let f2 = FileDiff {
            old_path: None,
            new_path: PathBuf::from("b.rs"),
            change_type: ChangeType::Modified,
            additions: 1,
            deletions: 0,
            hunks: vec![],
            is_folded: false,
            is_reviewed: false,
        };
        let f3 = FileDiff {
            old_path: None,
            new_path: PathBuf::from("c.rs"),
            change_type: ChangeType::Modified,
            additions: 1,
            deletions: 0,
            hunks: vec![],
            is_folded: false,
            is_reviewed: true,
        };
        let f4 = FileDiff {
            old_path: None,
            new_path: PathBuf::from("d.rs"),
            change_type: ChangeType::Modified,
            additions: 1,
            deletions: 0,
            hunks: vec![],
            is_folded: false,
            is_reviewed: false,
        };

        app.snapshot = DiffSnapshot::new(vec![f1, f2, f3, f4]);
        app.selected_file_idx = 0;

        app.next_unreviewed_file();
        assert_eq!(app.selected_file_idx, 1);

        app.next_unreviewed_file();
        assert_eq!(app.selected_file_idx, 3);

        app.prev_unreviewed_file();
        assert_eq!(app.selected_file_idx, 1);
    }

    #[test]
    fn test_view_centering() {
        let mut app = create_test_app();
        app.cursor_line_idx = 50;
        app.center_cursor();
        assert_eq!(app.diff_scroll_offset, 35);

        app.cursor_to_top_of_view();
        assert_eq!(app.diff_scroll_offset, 50);

        app.cursor_to_bottom_of_view();
        assert_eq!(app.diff_scroll_offset, 20);
    }

    #[test]
    fn test_search_and_symbol_under_cursor() {
        use crate::model::{ChangeType, DiffHunk, DiffLine, FileDiff, LineType};
        let mut app = create_test_app();
        let lines = vec![
            DiffLine {
                line_type: LineType::Context,
                content: "fn calculate_total() {".into(),
                old_line_num: Some(1),
                new_line_num: Some(1),
            },
            DiffLine {
                line_type: LineType::Addition,
                content: "    let total = 100;".into(),
                old_line_num: None,
                new_line_num: Some(2),
            },
            DiffLine {
                line_type: LineType::Addition,
                content: "    println!(\"{}\", total);".into(),
                old_line_num: None,
                new_line_num: Some(3),
            },
            DiffLine {
                line_type: LineType::Context,
                content: "}".into(),
                old_line_num: Some(2),
                new_line_num: Some(4),
            },
        ];
        let hunk = DiffHunk {
            header: "@@ -1,2 +1,4 @@".into(),
            old_start: 1,
            old_lines: 2,
            new_start: 1,
            new_lines: 4,
            lines,
            is_folded: false,
        };
        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("calc.rs"),
            change_type: ChangeType::Modified,
            additions: 2,
            deletions: 0,
            hunks: vec![hunk],
            is_folded: false,
            is_reviewed: false,
        };
        app.snapshot = DiffSnapshot::new(vec![file]);

        app.cursor_line_idx = 1; // "    let total = 100;"
        app.search_symbol_under_cursor(true);
        assert_eq!(app.search_query, "let");

        app.execute_search("total");
        assert_eq!(app.search_matches.len(), 3);

        app.next_search_match();
        assert_eq!(app.cursor_line_idx, 1);

        app.next_search_match();
        assert_eq!(app.cursor_line_idx, 2);

        app.prev_search_match();
        assert_eq!(app.cursor_line_idx, 1);
    }

    #[test]
    fn test_hunk_and_alert_and_file_jumps() {
        use crate::guard::{GuardAlert, Severity};
        use crate::model::{ChangeType, DiffHunk, DiffLine, FileDiff, LineType};
        let mut app = create_test_app();

        let h1 = DiffHunk {
            header: "@@ -1,1 +1,2 @@".into(),
            old_start: 1,
            old_lines: 1,
            new_start: 1,
            new_lines: 2,
            lines: vec![
                DiffLine {
                    line_type: LineType::Addition,
                    content: "a".into(),
                    old_line_num: None,
                    new_line_num: Some(1),
                },
                DiffLine {
                    line_type: LineType::Addition,
                    content: "b".into(),
                    old_line_num: None,
                    new_line_num: Some(2),
                },
            ],
            is_folded: false,
        };
        let h2 = DiffHunk {
            header: "@@ -10,1 +10,2 @@".into(),
            old_start: 10,
            old_lines: 1,
            new_start: 10,
            new_lines: 2,
            lines: vec![
                DiffLine {
                    line_type: LineType::Addition,
                    content: "c".into(),
                    old_line_num: None,
                    new_line_num: Some(10),
                },
                DiffLine {
                    line_type: LineType::Addition,
                    content: "d".into(),
                    old_line_num: None,
                    new_line_num: Some(11),
                },
            ],
            is_folded: false,
        };

        let f1 = FileDiff {
            old_path: None,
            new_path: PathBuf::from("f1.rs"),
            change_type: ChangeType::Modified,
            additions: 4,
            deletions: 0,
            hunks: vec![h1, h2],
            is_folded: false,
            is_reviewed: false,
        };
        let f2 = FileDiff {
            old_path: None,
            new_path: PathBuf::from("f2.rs"),
            change_type: ChangeType::Modified,
            additions: 0,
            deletions: 0,
            hunks: vec![],
            is_folded: false,
            is_reviewed: false,
        };

        app.snapshot = DiffSnapshot::new(vec![f1, f2]);
        app.alerts = vec![
            GuardAlert {
                severity: Severity::Critical,
                rule_id: "test/fixture".to_string(),
                fingerprint: String::new(),
                title: "Alert 1".into(),
                description: "d1".into(),
                line_no: Some(1),
                file_path: "f1.rs".into(),
            },
            GuardAlert {
                severity: Severity::Warn,
                rule_id: "test/fixture".to_string(),
                fingerprint: String::new(),
                title: "Alert 2".into(),
                description: "d2".into(),
                line_no: Some(10),
                file_path: "f1.rs".into(),
            },
        ];

        // Hunk hopping
        app.cursor_line_idx = 0;
        app.next_hunk();
        assert_eq!(app.cursor_line_idx, 2);

        app.prev_hunk();
        assert_eq!(app.cursor_line_idx, 0);

        // Alert hopping
        app.next_alert();
        assert_eq!(app.selected_alert_idx, 1);
        app.prev_alert();
        assert_eq!(app.selected_alert_idx, 0);

        // File hopping
        app.next_file();
        assert_eq!(app.selected_file_idx, 1);
        app.prev_file();
        assert_eq!(app.selected_file_idx, 0);

        // Review toggle
        app.toggle_review_current_file();
        assert!(app.snapshot.files[0].is_reviewed);
    }

    #[test]
    fn test_fuzzy_file_picker_filtering() {
        use crate::model::{ChangeType, FileDiff};
        let mut app = create_test_app();
        let f1 = FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/app.rs"),
            change_type: ChangeType::Modified,
            additions: 1,
            deletions: 0,
            hunks: vec![],
            is_folded: false,
            is_reviewed: false,
        };
        let f2 = FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/ui/mod.rs"),
            change_type: ChangeType::Modified,
            additions: 1,
            deletions: 0,
            hunks: vec![],
            is_folded: false,
            is_reviewed: false,
        };
        let f3 = FileDiff {
            old_path: None,
            new_path: PathBuf::from("tests/guard_test.rs"),
            change_type: ChangeType::Added,
            additions: 1,
            deletions: 0,
            hunks: vec![],
            is_folded: false,
            is_reviewed: false,
        };
        app.snapshot = DiffSnapshot::new(vec![f1, f2, f3]);

        app.file_picker.query = "ui".to_string();
        app.update_file_picker_filter();
        assert_eq!(app.file_picker.filtered_indices.len(), 1);
        assert_eq!(app.file_picker.filtered_indices[0], 1); // src/ui/mod.rs

        app.file_picker.query = "xyz123nonexistent".to_string();
        app.update_file_picker_filter();
        assert_eq!(app.file_picker.filtered_indices.len(), 0);

        app.file_picker.query = "".to_string();
        app.update_file_picker_filter();
        assert_eq!(app.file_picker.filtered_indices.len(), 3);
    }

    #[test]
    fn test_bracket_chord_and_file_navigation() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut app = create_test_app();
        app.snapshot.files = vec![
            crate::model::FileDiff {
                old_path: None,
                new_path: std::path::PathBuf::from("file1.rs"),
                change_type: crate::model::ChangeType::Added,
                additions: 1,
                deletions: 0,
                hunks: Vec::new(),
                is_folded: false,
                is_reviewed: false,
            },
            crate::model::FileDiff {
                old_path: None,
                new_path: std::path::PathBuf::from("file2.rs"),
                change_type: crate::model::ChangeType::Added,
                additions: 1,
                deletions: 0,
                hunks: Vec::new(),
                is_folded: true,
                is_reviewed: false,
            },
            crate::model::FileDiff {
                old_path: None,
                new_path: std::path::PathBuf::from("file3.rs"),
                change_type: crate::model::ChangeType::Added,
                additions: 1,
                deletions: 0,
                hunks: Vec::new(),
                is_folded: false,
                is_reviewed: false,
            },
        ];
        app.selected_file_idx = 0;

        let key_close_bracket = KeyEvent::new(KeyCode::Char(']'), KeyModifiers::NONE);
        let key_open_bracket = KeyEvent::new(KeyCode::Char('['), KeyModifiers::NONE);
        let key_f = KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE);

        // Chord ']f' moves to file 1 and un-folds it
        app.handle_diff_key(key_close_bracket);
        assert_eq!(app.pending_leader, Some(']'));
        app.handle_diff_key(key_f);
        assert_eq!(app.selected_file_idx, 1);
        assert!(!app.snapshot.files[1].is_folded);
        assert_eq!(app.status_message.as_deref(), Some("File [2/3]: file2.rs"));

        // Chained ']]' moves to file 2
        app.handle_diff_key(key_close_bracket);
        app.handle_diff_key(key_close_bracket);
        assert_eq!(app.selected_file_idx, 2);
        assert_eq!(app.status_message.as_deref(), Some("File [3/3]: file3.rs"));

        // Chaining ']' while already at last file gives hard stop warning
        app.handle_diff_key(key_close_bracket);
        assert_eq!(app.selected_file_idx, 2);
        assert_eq!(
            app.status_message.as_deref(),
            Some("Already at last file [3/3]: file3.rs")
        );

        // Reverse direction: '[' moves back to file 1
        app.handle_diff_key(key_open_bracket);
        assert_eq!(app.selected_file_idx, 1);
        assert_eq!(app.status_message.as_deref(), Some("File [2/3]: file2.rs"));

        // Chaining '[' moves to file 0
        app.handle_diff_key(key_open_bracket);
        assert_eq!(app.selected_file_idx, 0);
        assert_eq!(app.status_message.as_deref(), Some("File [1/3]: file1.rs"));

        // At first file, hard stop with informative status message
        app.handle_diff_key(key_open_bracket);
        assert_eq!(app.selected_file_idx, 0);
        assert_eq!(
            app.status_message.as_deref(),
            Some("Already at first file [1/3]: file1.rs")
        );
    }
}
