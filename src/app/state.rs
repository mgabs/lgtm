use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

#[derive(Debug, Default, Clone)]
pub struct FilePicker {
    pub query: String,
    pub selected_idx: usize,
    pub filtered_indices: Vec<usize>,
}

impl FilePicker {
    pub fn refilter(&mut self, names: &[String]) {
        if self.query.is_empty() {
            self.filtered_indices = (0..names.len()).collect();
        } else {
            let matcher = SkimMatcherV2::default();
            let mut scored: Vec<(i64, usize)> = names
                .iter()
                .enumerate()
                .filter_map(|(i, n)| matcher.fuzzy_match(n, &self.query).map(|s| (s, i)))
                .collect();
            scored.sort_by_key(|a| std::cmp::Reverse(a.0));
            self.filtered_indices = scored.into_iter().map(|(_, i)| i).collect();
        }
        if self.selected_idx >= self.filtered_indices.len() {
            self.selected_idx = 0;
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct PanePicker {
    pub list: Vec<crate::ipc::TmuxPane>,
    pub selected_idx: usize,
}

#[derive(Debug, Default, Clone)]
pub struct WorktreePickerState {
    pub list: Vec<lgtm_core::git::WorktreeInfo>,
    pub selected_idx: usize,
    pub pending_target: Option<std::path::PathBuf>,
}

#[derive(Debug, Default, Clone)]
pub struct CommandLine {
    pub input: String,
    pub cursor: usize,
    pub history: Vec<String>,
    pub history_idx: Option<usize>,
    pub completions: Vec<String>,
    pub completion_idx: Option<usize>,
    pub prefix: String,
}

impl CommandLine {
    pub fn reset(&mut self) {
        self.input.clear();
        self.cursor = 0;
        self.history_idx = None;
        self.completions.clear();
        self.completion_idx = None;
        self.prefix.clear();
    }

    pub fn set_text(&mut self, text: String) {
        self.input = text;
        self.cursor = self.input.chars().count();
    }

    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_right(&mut self) {
        if self.cursor < self.input.chars().count() {
            self.cursor += 1;
        }
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.input.chars().count();
    }

    pub fn insert_char(&mut self, c: char) {
        let byte_offset = self
            .input
            .char_indices()
            .nth(self.cursor)
            .map(|(idx, _)| idx)
            .unwrap_or(self.input.len());
        self.input.insert(byte_offset, c);
        self.cursor += 1;
    }

    pub fn delete_backward(&mut self) -> bool {
        if self.cursor == 0 {
            return !self.input.is_empty();
        }
        let prev_idx = self.cursor - 1;
        if let Some((start, ch)) = self.input.char_indices().nth(prev_idx) {
            let end = start + ch.len_utf8();
            self.input.drain(start..end);
            self.cursor = prev_idx;
        }
        true
    }

    pub fn delete_forward(&mut self) {
        if let Some((start, ch)) = self.input.char_indices().nth(self.cursor) {
            let end = start + ch.len_utf8();
            self.input.drain(start..end);
        }
    }

    pub fn delete_to_start(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let byte_offset = self
            .input
            .char_indices()
            .nth(self.cursor)
            .map(|(idx, _)| idx)
            .unwrap_or(self.input.len());
        self.input.drain(..byte_offset);
        self.cursor = 0;
    }

    pub fn delete_to_end(&mut self) {
        let byte_offset = self
            .input
            .char_indices()
            .nth(self.cursor)
            .map(|(idx, _)| idx)
            .unwrap_or(self.input.len());
        self.input.truncate(byte_offset);
    }

    pub fn delete_word_backward(&mut self) {
        self.cursor = delete_word_backward_at(&mut self.input, self.cursor);
    }

    #[allow(dead_code)]
    pub fn move_word_backward(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let (cursor, byte_pos) = if self.input.is_ascii() {
            let len = self.input.len();
            let c = self.cursor.min(len);
            (c, c)
        } else {
            let total = self.input.chars().count();
            let c = self.cursor.min(total);
            let b = self
                .input
                .char_indices()
                .nth(c)
                .map(|(idx, _)| idx)
                .unwrap_or(self.input.len());
            (c, b)
        };
        if cursor == 0 {
            self.cursor = 0;
            return;
        }
        let prefix = &self.input[..byte_pos];
        let mut chars_skipped = 0;
        let mut rev = prefix.chars().rev().peekable();
        while let Some(&c) = rev.peek() {
            if c.is_whitespace() {
                rev.next();
                chars_skipped += 1;
            } else {
                break;
            }
        }
        if let Some(&c) = rev.peek() {
            let is_word = c.is_alphanumeric() || c == '_';
            while let Some(&prev) = rev.peek() {
                if prev.is_whitespace() {
                    break;
                }
                let prev_is_word = prev.is_alphanumeric() || prev == '_';
                if prev_is_word != is_word {
                    break;
                }
                rev.next();
                chars_skipped += 1;
            }
        }
        self.cursor = cursor - chars_skipped;
    }

    #[allow(dead_code)]
    pub fn move_word_forward(&mut self) {
        let (cursor, byte_pos, total_chars) = if self.input.is_ascii() {
            let len = self.input.len();
            let c = self.cursor.min(len);
            (c, c, len)
        } else {
            let total = self.input.chars().count();
            let c = self.cursor.min(total);
            let b = self
                .input
                .char_indices()
                .nth(c)
                .map(|(idx, _)| idx)
                .unwrap_or(self.input.len());
            (c, b, total)
        };
        if cursor >= total_chars {
            self.cursor = total_chars;
            return;
        }
        let suffix = &self.input[byte_pos..];
        let mut chars_forward = 0;
        let mut iter = suffix.chars().peekable();
        while let Some(&c) = iter.peek() {
            if c.is_whitespace() {
                iter.next();
                chars_forward += 1;
            } else {
                break;
            }
        }
        if let Some(&c) = iter.peek() {
            let is_word = c.is_alphanumeric() || c == '_';
            while let Some(&next) = iter.peek() {
                if next.is_whitespace() {
                    break;
                }
                let next_is_word = next.is_alphanumeric() || next == '_';
                if next_is_word != is_word {
                    break;
                }
                iter.next();
                chars_forward += 1;
            }
        }
        self.cursor = (cursor + chars_forward).min(total_chars);
    }

    #[allow(dead_code)]
    pub fn delete_word_forward(&mut self) {
        let char_indices: Vec<(usize, char)> = self.input.char_indices().collect();
        let char_count = char_indices.len();
        let cursor = self.cursor.min(char_count);
        if cursor >= char_count {
            return;
        }
        let mut idx = cursor;
        while idx < char_count && char_indices[idx].1.is_whitespace() {
            idx += 1;
        }
        if idx < char_count {
            let is_word = char_indices[idx].1.is_alphanumeric() || char_indices[idx].1 == '_';
            while idx < char_count {
                let ch = char_indices[idx].1;
                if ch.is_whitespace() {
                    break;
                }
                let ch_is_word = ch.is_alphanumeric() || ch == '_';
                if ch_is_word != is_word {
                    break;
                }
                idx += 1;
            }
        }
        if idx > cursor {
            let byte_start = char_indices[cursor].0;
            let byte_end = if idx < char_count {
                char_indices[idx].0
            } else {
                self.input.len()
            };
            self.input.drain(byte_start..byte_end);
        }
    }

    pub fn autocomplete_next(&mut self) {
        if self.completions.is_empty() {
            let current = self.input.trim_start_matches(':').trim();
            self.prefix = current.to_string();
            let matches = super::command::autocomplete_candidates(current);

            if !matches.is_empty() {
                self.completions = matches;
                self.completion_idx = Some(0);
                self.input = self.completions[0].clone();
                self.cursor = self.input.chars().count();
            }
        } else if let Some(idx) = self.completion_idx {
            let next_idx = (idx + 1) % self.completions.len();
            self.completion_idx = Some(next_idx);
            self.input = self.completions[next_idx].clone();
            self.cursor = self.input.chars().count();
        }
    }

    pub fn autocomplete_prev(&mut self) {
        if !self.completions.is_empty() {
            if let Some(idx) = self.completion_idx {
                let prev_idx = if idx == 0 {
                    self.completions.len() - 1
                } else {
                    idx - 1
                };
                self.completion_idx = Some(prev_idx);
                self.input = self.completions[prev_idx].clone();
                self.cursor = self.input.chars().count();
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuditState {
    pub has_result: bool,
    pub progress: Option<String>,
    pub alerts: Vec<crate::guard::GuardAlert>,
    pub files: Vec<crate::model::FileDiff>,
    pub selected_idx: usize,
    pub filter_severity: Option<crate::guard::Severity>,
    pub root_dir: std::path::PathBuf,
    pub focus: super::AuditPaneFocus,
    pub file_filter: Option<String>,
    pub inspector_scroll: usize,
    pub maximized_inspector: bool,
}

impl AuditState {
    pub fn new(root_dir: std::path::PathBuf) -> Self {
        Self {
            has_result: false,
            progress: None,
            alerts: Vec::new(),
            files: Vec::new(),
            selected_idx: 0,
            filter_severity: None,
            root_dir,
            focus: super::AuditPaneFocus::Issues,
            file_filter: None,
            inspector_scroll: 0,
            maximized_inspector: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PrState {
    pub filter: crate::pr::PullRequestState,
    pub list_generation: u64,
    pub current_number: Option<u64>,
    pub list: Vec<crate::pr::PullRequestSummary>,
    pub selected_idx: usize,
}

impl PrState {
    pub fn new(current_number: Option<u64>) -> Self {
        Self {
            current_number,
            filter: Default::default(),
            list_generation: 0,
            list: Vec::new(),
            selected_idx: 0,
        }
    }
}

/// Deletes backward from character `cursor` in `input` to previous word boundary.
/// Returns updated character index.
pub fn delete_word_backward_at(input: &mut String, cursor: usize) -> usize {
    let char_indices: Vec<(usize, char)> = input.char_indices().collect();
    let char_count = char_indices.len();
    let cursor = cursor.min(char_count);
    if cursor == 0 {
        return 0;
    }

    let mut idx = cursor;
    while idx > 0 && char_indices[idx - 1].1.is_whitespace() {
        idx -= 1;
    }

    if idx > 0 {
        let is_word = char_indices[idx - 1].1.is_alphanumeric() || char_indices[idx - 1].1 == '_';
        while idx > 0 {
            let prev = char_indices[idx - 1].1;
            if prev.is_whitespace() {
                break;
            }
            let prev_is_word = prev.is_alphanumeric() || prev == '_';
            if prev_is_word != is_word {
                break;
            }
            idx -= 1;
        }
    }

    let byte_start = if idx < char_count {
        char_indices[idx].0
    } else {
        input.len()
    };
    let byte_end = if cursor < char_count {
        char_indices[cursor].0
    } else {
        input.len()
    };

    input.drain(byte_start..byte_end);
    idx
}

/// Deletes backward from end of string `s` to previous word boundary.
#[allow(dead_code)]
pub fn delete_word_backward_str(s: &mut String) {
    let char_count = s.chars().count();
    delete_word_backward_at(s, char_count);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_line_default_is_empty() {
        let c = CommandLine::default();
        assert!(c.input.is_empty() && c.history.is_empty() && c.completions.is_empty());
        assert_eq!((c.history_idx, c.completion_idx), (None, None));
        assert_eq!(c.cursor, 0);
    }

    #[test]
    fn command_line_movement_and_clamping() {
        let mut c = CommandLine::default();
        c.set_text("pr 12".into());
        assert_eq!(c.cursor, 5);

        c.move_left();
        c.move_left();
        assert_eq!(c.cursor, 3);

        c.move_home();
        assert_eq!(c.cursor, 0);

        c.move_left(); // clamping at 0
        assert_eq!(c.cursor, 0);

        c.move_right();
        c.move_right();
        assert_eq!(c.cursor, 2);

        c.move_end();
        assert_eq!(c.cursor, 5);

        c.move_right(); // clamping at len
        assert_eq!(c.cursor, 5);
    }

    #[test]
    fn command_line_char_insertion_ascii_and_utf8() {
        let mut c = CommandLine::default();
        c.set_text("audi".into());
        c.insert_char('t');
        assert_eq!(c.input, "audit");
        assert_eq!(c.cursor, 5);

        c.move_left();
        c.move_left();
        c.move_left();
        assert_eq!(c.cursor, 2);
        c.insert_char('x');
        assert_eq!(c.input, "auxdit");
        assert_eq!(c.cursor, 3);

        // UTF-8 multi-byte: '🦀' is 4 bytes
        c.insert_char('🦀');
        assert_eq!(c.input, "aux🦀dit");
        assert_eq!(c.cursor, 4);

        c.move_left();
        assert_eq!(c.cursor, 3);
        c.insert_char('🔥');
        assert_eq!(c.input, "aux🔥🦀dit");
        assert_eq!(c.cursor, 4);
    }

    #[test]
    fn command_line_deletion_backward_and_forward() {
        let mut c = CommandLine::default();
        // Empty backspace -> returns false
        assert!(!c.delete_backward());

        c.set_text("help".into());
        assert_eq!(c.cursor, 4);

        // Cursor at 0 with text -> returns true, noop
        c.move_home();
        assert_eq!(c.cursor, 0);
        assert!(c.delete_backward());
        assert_eq!(c.input, "help");
        assert_eq!(c.cursor, 0);

        // Move to index 2 (between e and l)
        c.move_right();
        c.move_right();
        assert_eq!(c.cursor, 2);
        assert!(c.delete_backward());
        assert_eq!(c.input, "hlp");
        assert_eq!(c.cursor, 1);

        // Delete forward (deletes 'l')
        c.delete_forward();
        assert_eq!(c.input, "hp");
        assert_eq!(c.cursor, 1);

        // Delete forward at end of string -> noop
        c.move_end();
        assert_eq!(c.cursor, 2);
        c.delete_forward();
        assert_eq!(c.input, "hp");
        assert_eq!(c.cursor, 2);
    }

    #[test]
    fn command_line_delete_to_start_and_end() {
        let mut c = CommandLine::default();
        c.set_text("diff main".into());
        c.cursor = 4;

        // Delete to start
        c.delete_to_start();
        assert_eq!(c.input, " main");
        assert_eq!(c.cursor, 0);

        // Delete to end
        c.set_text("diff main".into());
        c.cursor = 4;
        c.delete_to_end();
        assert_eq!(c.input, "diff");
        assert_eq!(c.cursor, 4);

        // Delete to start from end clears entire line
        c.set_text("scan repo".into());
        c.delete_to_start();
        assert_eq!(c.input, "");
        assert_eq!(c.cursor, 0);
    }

    #[test]
    fn command_line_delete_word_backward_scenarios() {
        let mut c = CommandLine::default();

        // Scenario 2.3: "pr 12" -> "pr " -> ""
        c.set_text("pr 12".into());
        c.delete_word_backward();
        assert_eq!(c.input, "pr ");
        assert_eq!(c.cursor, 3);
        c.delete_word_backward();
        assert_eq!(c.input, "");
        assert_eq!(c.cursor, 0);

        // Scenario 2.4: Trailing spaces "audit   " -> ""
        c.set_text("audit   ".into());
        c.delete_word_backward();
        assert_eq!(c.input, "");
        assert_eq!(c.cursor, 0);

        // Punctuation boundaries "foo-bar" -> "foo-" -> "foo"
        c.set_text("foo-bar".into());
        c.delete_word_backward();
        assert_eq!(c.input, "foo-");
        assert_eq!(c.cursor, 4);
        c.delete_word_backward();
        assert_eq!(c.input, "foo");
        assert_eq!(c.cursor, 3);

        // Multi-punctuation "::audit" -> ""
        c.set_text("::audit".into());
        c.delete_word_backward();
        assert_eq!(c.input, "::");
        assert_eq!(c.cursor, 2);
        c.delete_word_backward();
        assert_eq!(c.input, "");
        assert_eq!(c.cursor, 0);

        // Empty string
        c.delete_word_backward();
        assert_eq!(c.input, "");
        assert_eq!(c.cursor, 0);
    }

    #[test]
    fn command_line_autocomplete_cursor_sync() {
        let mut c = CommandLine::default();
        c.set_text("au".into());
        c.autocomplete_next();
        assert_eq!(c.input, "audit");
        assert_eq!(c.cursor, 5);

        c.autocomplete_prev();
        assert_eq!(c.cursor, c.input.chars().count());
    }

    #[test]
    fn command_line_sub_microsecond_keystroke_benchmark() {
        let mut c = CommandLine::default();
        c.set_text("a".repeat(100));

        let start = std::time::Instant::now();
        const ITERS: u32 = 10_000;
        for _ in 0..ITERS {
            c.move_left();
            c.move_right();
        }
        let elapsed = start.elapsed();
        let per_op = elapsed.as_nanos() as f64 / (ITERS as f64 * 2.0);
        // Budget: < 1000ns (< 1µs) per keystroke operation
        assert!(
            per_op < 1000.0,
            "Keystroke operation took {}ns, expected < 1000ns",
            per_op
        );
    }

    #[test]
    fn file_picker_refilter_fuzzy_and_resets_selection() {
        let names: Vec<String> = ["src/main.rs", "src/app/mod.rs", "README.md"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mut p = FilePicker::default();
        p.refilter(&names);
        assert_eq!(p.filtered_indices, vec![0, 1, 2]);
        p.query = "readme".into();
        p.selected_idx = 2;
        p.refilter(&names);
        assert_eq!(p.filtered_indices, vec![2]);
        assert_eq!(p.selected_idx, 0);
    }

    #[test]
    fn audit_state_new_starts_on_issues_pane() {
        let a = AuditState::new(std::path::PathBuf::from("/repo"));
        assert!(a.alerts.is_empty() && a.files.is_empty());
        assert_eq!(a.selected_idx, 0);
        assert_eq!(a.focus, super::super::AuditPaneFocus::Issues);
        assert!(a.filter_severity.is_none() && a.file_filter.is_none() && !a.maximized_inspector);
    }

    #[test]
    fn pr_state_new_keeps_explicit_number() {
        let p = PrState::new(Some(42));
        assert_eq!(p.current_number, Some(42));
        assert!(p.list.is_empty());
        assert_eq!(p.selected_idx, 0);
    }

    #[test]
    fn test_delete_word_backward_at_vectors() {
        let cases = vec![
            ("", 0, "", 0, "empty string"),
            ("hello", 0, "hello", 0, "cursor at start"),
            ("audit", 5, "", 0, "single word end"),
            ("pr 12", 5, "pr ", 3, "word with space end"),
            ("pr ", 3, "", 0, "word after space deleted"),
            (
                "audit   ",
                8,
                "",
                0,
                "trailing whitespace stripped with word",
            ),
            ("   ", 3, "", 0, "pure whitespace"),
            ("\t\n  ", 4, "", 0, "mixed whitespace"),
            (
                "foo-bar",
                7,
                "foo-",
                4,
                "punctuation delimiter word backward",
            ),
            ("foo-", 4, "foo", 3, "punctuation token backward"),
            ("::audit", 7, "::", 2, "multi-punctuation prefix"),
            (
                "foo_bar",
                7,
                "",
                0,
                "underscore treated as word constituent",
            ),
            (
                "foo.bar_baz",
                11,
                "foo.",
                4,
                "mixed punctuation and identifier",
            ),
            (
                "cargo test --release",
                11,
                "cargo --release",
                6,
                "mid-string deletion with trailing space",
            ),
            ("hello world", 8, "hello rld", 6, "mid-word deletion"),
            ("abc", 99, "", 0, "out-of-bounds cursor clamping"),
            (
                "hello 🦀",
                8,
                "hello ",
                6,
                "UTF-8 emoji multi-byte boundary",
            ),
            ("Привет мир", 10, "Привет ", 7, "UTF-8 Cyrillic word"),
            ("hello 世界", 8, "hello ", 6, "UTF-8 CJK wide characters"),
            (
                "café au lait",
                12,
                "café au ",
                8,
                "UTF-8 accented characters",
            ),
            (
                "🦀🔥",
                2,
                "",
                0,
                "consecutive emojis deleted as symbol token",
            ),
        ];

        for (input, cursor, exp_str, exp_cur, desc) in cases {
            let mut s = input.to_string();
            let new_cur = delete_word_backward_at(&mut s, cursor);
            assert_eq!(s, exp_str, "String mismatch on case: {desc}");
            assert_eq!(new_cur, exp_cur, "Cursor mismatch on case: {desc}");
        }
    }

    #[test]
    fn test_delete_word_backward_str_vectors() {
        let cases = vec![
            ("", "", "empty string"),
            ("audit", "", "single word"),
            ("foo bar", "foo ", "two words"),
            ("audit   ", "", "trailing spaces"),
            ("foo-bar", "foo-", "hyphenated token"),
            ("foo-", "foo", "hyphen token"),
            ("::audit", "::", "double colon"),
            ("🦀 🔥", "🦀 ", "UTF-8 emoji tokens"),
        ];

        for (input, exp_str, desc) in cases {
            let mut s = input.to_string();
            delete_word_backward_str(&mut s);
            assert_eq!(s, exp_str, "Mismatch on delete_word_backward_str: {desc}");
        }
    }

    #[test]
    fn test_command_line_word_motion_backward_forward() {
        let mut c = CommandLine::default();

        // Empty string motion
        c.move_word_backward();
        assert_eq!(c.cursor, 0);
        c.move_word_forward();
        assert_eq!(c.cursor, 0);

        // All whitespace motion
        c.set_text("     ".into());
        c.cursor = 5;
        c.move_word_backward();
        assert_eq!(c.cursor, 0);
        c.move_word_forward();
        assert_eq!(c.cursor, 5);

        // Normal words: "audit issues"
        c.set_text("audit issues".into());
        c.cursor = 0;
        c.move_word_forward();
        assert_eq!(c.cursor, 5); // after "audit"
        c.move_word_forward();
        assert_eq!(c.cursor, 12); // after "issues"
        c.move_word_forward();
        assert_eq!(c.cursor, 12); // clamped at len

        c.move_word_backward();
        assert_eq!(c.cursor, 6); // before "issues"
        c.move_word_backward();
        assert_eq!(c.cursor, 0); // before "audit"
        c.move_word_backward();
        assert_eq!(c.cursor, 0); // clamped at 0

        // Punctuation and underscore tokens: "foo_bar-baz.rs"
        c.set_text("foo_bar-baz.rs".into());
        c.cursor = 0;
        c.move_word_forward();
        assert_eq!(c.cursor, 7); // after "foo_bar"
        c.move_word_forward();
        assert_eq!(c.cursor, 8); // after "-"
        c.move_word_forward();
        assert_eq!(c.cursor, 11); // after "baz"
        c.move_word_forward();
        assert_eq!(c.cursor, 12); // after "."
        c.move_word_forward();
        assert_eq!(c.cursor, 14); // after "rs"

        c.move_word_backward();
        assert_eq!(c.cursor, 12); // before "rs"
        c.move_word_backward();
        assert_eq!(c.cursor, 11); // before "."
        c.move_word_backward();
        assert_eq!(c.cursor, 8); // before "baz"
        c.move_word_backward();
        assert_eq!(c.cursor, 7); // before "-"
        c.move_word_backward();
        assert_eq!(c.cursor, 0); // before "foo_bar"

        // Cursor out of bounds clamping
        c.cursor = 999;
        c.move_word_backward();
        assert_eq!(c.cursor, 12);
        c.cursor = 999;
        c.move_word_forward();
        assert_eq!(c.cursor, 14);

        // UTF-8 emojis: "🦀🦀 aux 🔥"
        c.set_text("🦀🦀 aux 🔥".into());
        c.cursor = 0;
        c.move_word_forward();
        assert_eq!(c.cursor, 2); // after "🦀🦀"
        c.move_word_forward();
        assert_eq!(c.cursor, 6); // after "aux"
        c.move_word_forward();
        assert_eq!(c.cursor, 8); // after "🔥"

        c.move_word_backward();
        assert_eq!(c.cursor, 7); // before "🔥"
        c.move_word_backward();
        assert_eq!(c.cursor, 3); // before "aux"
        c.move_word_backward();
        assert_eq!(c.cursor, 0); // before "🦀🦀"
    }

    #[test]
    fn test_command_line_delete_word_forward_scenarios() {
        let mut c = CommandLine::default();

        // Empty string
        c.delete_word_forward();
        assert_eq!(c.input, "");
        assert_eq!(c.cursor, 0);

        // At end of input
        c.set_text("audit".into());
        c.cursor = 5;
        c.delete_word_forward();
        assert_eq!(c.input, "audit");
        assert_eq!(c.cursor, 5);

        // Out of bounds cursor
        c.cursor = 100;
        c.delete_word_forward();
        assert_eq!(c.input, "audit");

        // Forward word delete from index 0: "audit issues" -> " issues"
        c.set_text("audit issues".into());
        c.cursor = 0;
        c.delete_word_forward();
        assert_eq!(c.input, " issues");
        assert_eq!(c.cursor, 0);

        // Next forward word delete: whitespace + word " issues" -> ""
        c.delete_word_forward();
        assert_eq!(c.input, "");
        assert_eq!(c.cursor, 0);

        // Mid-string forward delete: "git status -s" at cursor 4 ("status -s")
        c.set_text("git status -s".into());
        c.cursor = 4;
        c.delete_word_forward();
        assert_eq!(c.input, "git  -s");
        assert_eq!(c.cursor, 4);

        // Mid-word forward delete: "audit issues" at cursor 2 ("dit")
        c.set_text("audit issues".into());
        c.cursor = 2;
        c.delete_word_forward();
        assert_eq!(c.input, "au issues");
        assert_eq!(c.cursor, 2);

        // Punctuation token forward delete: "foo-bar" at 0 -> "-bar" -> "bar" -> ""
        c.set_text("foo-bar".into());
        c.cursor = 0;
        c.delete_word_forward();
        assert_eq!(c.input, "-bar");
        assert_eq!(c.cursor, 0);
        c.delete_word_forward();
        assert_eq!(c.input, "bar");
        assert_eq!(c.cursor, 0);
        c.delete_word_forward();
        assert_eq!(c.input, "");
        assert_eq!(c.cursor, 0);

        // All whitespace forward delete: "   " at 0 -> ""
        c.set_text("   ".into());
        c.cursor = 0;
        c.delete_word_forward();
        assert_eq!(c.input, "");
        assert_eq!(c.cursor, 0);

        // UTF-8 forward delete
        c.set_text("🦀 rocket 🔥 fire".into());
        c.cursor = 0;
        c.delete_word_forward();
        assert_eq!(c.input, " rocket 🔥 fire");
        assert_eq!(c.cursor, 0);
    }

    #[test]
    fn test_command_line_delete_word_backward_all_whitespace() {
        let mut c = CommandLine::default();
        c.set_text("   ".into());
        c.cursor = 3;
        c.delete_word_backward();
        assert_eq!(c.input, "");
        assert_eq!(c.cursor, 0);

        c.set_text("\t\n  ".into());
        c.cursor = 4;
        c.delete_word_backward();
        assert_eq!(c.input, "");
        assert_eq!(c.cursor, 0);
    }

    #[test]
    fn word_operations_move_between_word_boundaries() {
        let mut c = CommandLine::default();
        c.set_text("word1 word2".into());
        c.cursor = 0;

        c.move_word_forward();
        assert_eq!(c.cursor, 5);
        c.move_word_backward();
        assert_eq!(c.cursor, 0);
    }

    #[cfg(all(feature = "perf-budgets", not(debug_assertions)))]
    #[test]
    fn test_performance_budgets_nfr_word_operations() {
        use std::hint::black_box;
        use std::time::Instant;

        let input = "word1 word2 word3 word4 word5 word6 word7 word8 word9 word10".repeat(5);
        let mut command = CommandLine::default();
        command.set_text(input);

        for _ in 0..100 {
            command.cursor = 0;
            command.move_word_forward();
            command.move_word_backward();
        }

        let mut samples = Vec::with_capacity(7);
        for _ in 0..7 {
            let start = Instant::now();
            for _ in 0..1000 {
                command.cursor = 0;
                command.move_word_forward();
                command.move_word_backward();
            }
            samples.push(start.elapsed().as_nanos() / 2000);
            black_box(command.cursor);
        }
        samples.sort_unstable();
        let median_ns = samples[3];
        println!(
            "Benchmark [word motion]: median {}ns/op, samples: {:?}",
            median_ns, samples
        );
        assert!(
            median_ns < 1000,
            "Word motion took {}ns, expected < 1000ns",
            median_ns
        );
    }
}
