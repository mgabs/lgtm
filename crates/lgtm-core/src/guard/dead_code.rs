use crate::guard::{GuardAlert, Severity};
use crate::model::{FileDiff, LineType};

fn is_test_attr(text: &str) -> bool {
    text.starts_with("#[test") || text.starts_with("#[tokio::test") || text.starts_with("#[rstest")
}

fn is_test_module_boundary(text: &str) -> bool {
    text.starts_with("#[cfg(test)]") || text.starts_with("mod test")
}

fn is_test_or_entry_file(path_str: &str) -> bool {
    path_str.contains("test")
        || path_str.contains("benchmark")
        || path_str.contains("regression")
        || path_str.ends_with("lib.rs")
        || path_str.ends_with("main.rs")
}

fn process_hunk_lines(
    hunk: &crate::model::DiffHunk,
    path_str: &str,
    all_diff_text: &mut String,
    added: &mut Vec<(String, String, Option<u32>)>,
    in_test_module: &mut bool,
) {
    if hunk.header.contains("test") {
        *in_test_module = true;
    }
    let mut prev_is_test_attr = false;
    for line in &hunk.lines {
        let text = line.content.trim();
        if is_test_module_boundary(text) {
            *in_test_module = true;
        }
        if text.is_empty() {
            continue;
        }
        let is_test = is_test_attr(text);
        if line.line_type == LineType::Addition && !crate::guard::is_line_ignored(text) {
            all_diff_text.push_str(text);
            all_diff_text.push('\n');
            if let Some(fn_name) = extract_private_function_definition(text) {
                if !*in_test_module && !prev_is_test_attr {
                    added.push((fn_name, path_str.to_string(), line.new_line_num));
                }
            }
        }
        prev_is_test_attr = is_test;
    }
}

fn collect_file_private_functions(
    file: &FileDiff,
    all_diff_text: &mut String,
    added: &mut Vec<(String, String, Option<u32>)>,
) {
    let path_str = file.display_name();
    if is_test_or_entry_file(&path_str) {
        return;
    }
    let mut in_test_module = false;
    for hunk in &file.hunks {
        process_hunk_lines(hunk, &path_str, all_diff_text, added, &mut in_test_module);
    }
}

pub struct DeadCodeGuard;

impl DeadCodeGuard {
    pub fn scan_snapshot(files: &[FileDiff]) -> Vec<GuardAlert> {
        let mut alerts = Vec::new();

        // 1. Collect all newly added private/internal function names
        let mut added_private_functions: Vec<(String, String, Option<u32>)> = Vec::new();
        let mut all_diff_text = String::new();

        for file in files {
            collect_file_private_functions(file, &mut all_diff_text, &mut added_private_functions);
        }

        // 2. Count references across all added lines in the diff
        for (fn_name, file_path, line_num) in added_private_functions {
            let call_pattern = format!("{}(", fn_name);
            let call_pattern_space = format!("{} (", fn_name);
            let occurrences = all_diff_text.matches(&call_pattern).count()
                + all_diff_text.matches(&call_pattern_space).count();

            // If it only occurs once (the definition itself), it is unused / dead code in this diff
            if occurrences <= 1 && fn_name.len() >= 4 {
                alerts.push(GuardAlert {
                    severity: Severity::Warn,
                    rule_id: "dead/unused-function".to_string(),
                    fingerprint: String::new(),
                    title: "Unreferenced / Dead Function Added".to_string(),
                    description: format!(
                        "Agent introduced private function `{}` with 0 call sites across this diff",
                        fn_name
                    ),
                    line_no: line_num,
                    file_path,
                });
            }
        }

        alerts
    }
}

fn extract_private_function_definition(text: &str) -> Option<String> {
    let t = text.trim();
    if t.starts_with("pub ")
        || t.starts_with("export ")
        || t.starts_with("//")
        || t.starts_with('#')
    {
        return None;
    }

    if (t.starts_with("fn ") || t.starts_with("def ") || t.starts_with("function "))
        && t.contains('(')
    {
        let before_paren = t.split('(').next()?;
        let tokens: Vec<&str> = before_paren.split_whitespace().collect();
        if let Some(&name) = tokens.last() {
            if !name.starts_with('_')
                && !name.starts_with("test_")
                && !name.starts_with("test")
                && !name.starts_with("bdd_")
                && name != "new"
                && name != "main"
            {
                return Some(name.to_string());
            }
        }
    }

    None
}

impl super::Guard for DeadCodeGuard {
    fn scan_snapshot(&self, files: &[FileDiff]) -> Vec<GuardAlert> {
        Self::scan_snapshot(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ChangeType, DiffHunk, DiffLine, FileDiff, LineType};
    use std::path::PathBuf;

    #[test]
    fn test_dead_code_guard() {
        let files = vec![FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/engine.rs"),
            change_type: ChangeType::Modified,
            additions: 3,
            deletions: 0,
            hunks: vec![DiffHunk {
                header: "@@ -1,0 +1,3 @@".to_string(),
                old_start: 1,
                old_lines: 0,
                new_start: 1,
                new_lines: 3,
                lines: vec![
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "fn unused_internal_helper() {".into(),
                        old_line_num: None,
                        new_line_num: Some(1),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "    println!(\"nobody calls me\");".into(),
                        old_line_num: None,
                        new_line_num: Some(2),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "}".into(),
                        old_line_num: None,
                        new_line_num: Some(3),
                    },
                ],
                is_folded: false,
                folded_blocks: Vec::new(),
            }],
            is_folded: false,
            is_reviewed: false,
        }];

        let alerts = DeadCodeGuard::scan_snapshot(&files);
        assert_eq!(alerts.len(), 1);
        assert!(alerts[0].title.contains("Unreferenced / Dead Function"));
    }
}
