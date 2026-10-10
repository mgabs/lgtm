use crate::guard::{GuardAlert, Severity};
use crate::model::{FileDiff, LineType};

pub struct DeadCodeGuard;

impl DeadCodeGuard {
    pub fn scan_snapshot(files: &[FileDiff]) -> Vec<GuardAlert> {
        let mut alerts = Vec::new();

        // 1. Collect all newly added private/internal function names
        let mut added_private_functions: Vec<(String, String, Option<u32>)> = Vec::new();
        let mut all_diff_text = String::new();

        for file in files {
            let path_str = file.display_name();
            if path_str.contains("test")
                || path_str.ends_with("lib.rs")
                || path_str.ends_with("main.rs")
            {
                continue;
            }

            for hunk in &file.hunks {
                for line in &hunk.lines {
                    if line.line_type == LineType::Addition {
                        let text = line.content.trim();
                        all_diff_text.push_str(text);
                        all_diff_text.push('\n');

                        if let Some(fn_name) = extract_private_function_definition(text) {
                            added_private_functions.push((
                                fn_name,
                                path_str.clone(),
                                line.new_line_num,
                            ));
                        }
                    }
                }
            }
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
                && name != "new"
                && name != "main"
            {
                return Some(name.to_string());
            }
        }
    }

    None
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
            }],
            is_folded: false,
            is_reviewed: false,
        }];

        let alerts = DeadCodeGuard::scan_snapshot(&files);
        assert_eq!(alerts.len(), 1);
        assert!(alerts[0].title.contains("Unreferenced / Dead Function"));
    }
}
