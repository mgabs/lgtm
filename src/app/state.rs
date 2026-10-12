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
    pub history: Vec<String>,
    pub history_idx: Option<usize>,
    pub completions: Vec<String>,
    pub completion_idx: Option<usize>,
    pub prefix: String,
}

impl CommandLine {
    pub fn autocomplete_next(&mut self) {
        let commands = [
            "audit",
            "c",
            "checks",
            "clear",
            "clear-resolved",
            "comments",
            "commit",
            "d",
            "del",
            "delete",
            "diff",
            "directives",
            "drawer",
            "e",
            "edit",
            "exit",
            "f",
            "files",
            "find",
            "h",
            "help",
            "hidden",
            "pane",
            "panes",
            "pr",
            "prs",
            "push",
            "q",
            "quit",
            "r",
            "ref",
            "reload",
            "res",
            "resolve",
            "review",
            "risk",
            "s",
            "scan",
            "send",
            "sidebar",
            "sp",
            "split",
            "submit",
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
            }
        } else if let Some(idx) = self.completion_idx {
            let next_idx = (idx + 1) % self.completions.len();
            self.completion_idx = Some(next_idx);
            self.input = self.completions[next_idx].clone();
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
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuditState {
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
    pub current_number: Option<u64>,
    pub list: Vec<crate::pr::PullRequestSummary>,
    pub selected_idx: usize,
}

impl PrState {
    pub fn new(current_number: Option<u64>) -> Self {
        Self {
            current_number,
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
