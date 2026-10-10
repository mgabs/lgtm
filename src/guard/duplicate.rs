use crate::guard::{GuardAlert, Severity};
use crate::model::{FileDiff, LineType};

pub struct DuplicateHelperGuard;

impl DuplicateHelperGuard {
    pub fn scan_file(file: &FileDiff) -> Vec<GuardAlert> {
        let mut alerts = Vec::new();
        let path_str = file.display_name();

        for hunk in &file.hunks {
            for line in &hunk.lines {
                if line.line_type == LineType::Addition {
                    let text = line.content.trim();
                    let line_num = line.new_line_num;

                    // Detect suspect duplicate helper suffix patterns (e.g., fn format_date_2, helper_new, custom_slugify)
                    if is_duplicate_helper_name(text) {
                        alerts.push(GuardAlert {
                            severity: Severity::Warn,
                            title: "Suspicious Duplicate Helper Function".to_string(),
                            description: format!(
                                "Function declaration '{}' matches duplicate helper anti-pattern (check if utility already exists)",
                                text
                            ),
                            line_no: line_num,
                            file_path: path_str.clone(),
                        });
                    }
                }
            }
        }

        alerts
    }
}

fn is_duplicate_helper_name(text: &str) -> bool {
    let t = text.to_lowercase();
    let is_fn_def = t.contains("fn ")
        || t.starts_with("def ")
        || t.contains("function ")
        || (t.contains("const ") && t.contains(" = ("))
        || (t.contains("let ") && t.contains(" = ("))
        || t.starts_with("func ");

    if !is_fn_def {
        return false;
    }

    t.contains("_2(")
        || t.contains("_2 ")
        || t.contains("_v2(")
        || t.contains("_v2 ")
        || t.contains("_new(")
        || t.contains("_helper(")
        || t.contains("_util(")
        || t.contains("custom_")
        || t.contains("temp_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ChangeType, DiffHunk, DiffLine};
    use std::path::PathBuf;

    #[test]
    fn test_duplicate_helper_detection() {
        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/utils.rs"),
            change_type: ChangeType::Modified,
            additions: 2,
            deletions: 0,
            hunks: vec![DiffHunk {
                header: "@@ -1,2 +1,4 @@".to_string(),
                old_start: 1,
                old_lines: 2,
                new_start: 1,
                new_lines: 4,
                lines: vec![
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "pub fn format_date_v2(d: &str) -> String {".to_string(),
                        old_line_num: None,
                        new_line_num: Some(15),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "def custom_slugify(text):".to_string(),
                        old_line_num: None,
                        new_line_num: Some(30),
                    },
                ],
                is_folded: false,
            }],
            is_folded: false,
            is_reviewed: false,
        };

        let alerts = DuplicateHelperGuard::scan_file(&file);
        assert_eq!(alerts.len(), 2);
        assert!(alerts.iter().all(|a| a.title.contains("Duplicate Helper")));
    }
}
