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
        if self.cursor == 0 {
            return;
        }
        let chars: Vec<char> = self.input.chars().collect();
        let mut idx = self.cursor;
        while idx > 0 && chars[idx - 1].is_whitespace() {
            idx -= 1;
        }
        if idx > 0 {
            let is_word = chars[idx - 1].is_alphanumeric() || chars[idx - 1] == '_';
            while idx > 0 {
                let prev = chars[idx - 1];
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
        let byte_start = self
            .input
            .char_indices()
            .nth(idx)
            .map(|(i, _)| i)
            .unwrap_or(self.input.len());
        let byte_end = self
            .input
            .char_indices()
            .nth(self.cursor)
            .map(|(i, _)| i)
            .unwrap_or(self.input.len());
        self.input.drain(byte_start..byte_end);
        self.cursor = idx;
    }

    pub fn autocomplete_next(&mut self) {
        let commands = [
            "approve",
            "audit",
            "c",
            "checks",
            "ci",
            "clear",
            "clear-resolved",
            "comments",
            "commit",
            "context",
            "diff",
            "directives",
            "drawer",
            "e",
            "edit",
            "editor",
            "exit",
            "export",
            "f",
            "file",
            "files",
            "filter",
            "find",
            "h",
            "help",
            "hidden",
            "hook",
            "inject",
            "issues",
            "lgtm",
            "md",
            "n",
            "next",
            "p",
            "pane",
            "panes",
            "pr",
            "preview",
            "prompt",
            "prs",
            "push",
            "q",
            "quit",
            "r",
            "ref",
            "reject",
            "reject-all",
            "reload",
            "res",
            "resolve",
            "review",
            "risk",
            "scan",
            "send",
            "sev",
            "sidebar",
            "sink",
            "sp",
            "split",
            "thread",
            "threads",
            "todo",
            "tree",
            "unresolve",
            "vsp",
            "vsplit",
            "w",
            "write",
            "wq",
            "x",
            "yank",
            "yr",
        ];

        if self.completions.is_empty() {
            let current = self.input.trim_start_matches(':').trim();
            self.prefix = current.to_string();
            let matches: Vec<String> = commands
                .iter()
                .filter(|&&cmd| current.is_empty() || cmd.starts_with(current))
                .map(|&s| s.to_string())
                .collect();

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
}
