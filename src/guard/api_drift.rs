use crate::guard::{GuardAlert, Severity};
use crate::model::{FileDiff, LineType};

pub struct ApiDriftGuard;

impl ApiDriftGuard {
    pub fn scan_file(file: &FileDiff) -> Vec<GuardAlert> {
        let mut alerts = Vec::new();
        let path_str = file.display_name();

        // Skip test files or internal implementation directories
        if path_str.contains("test")
            || path_str.contains("/internal/")
            || path_str.contains("/private/")
        {
            return alerts;
        }

        for hunk in &file.hunks {
            let mut deleted_signatures = Vec::new();
            let mut added_signatures = Vec::new();

            for line in &hunk.lines {
                let text = line.content.trim();
                match line.line_type {
                    LineType::Deletion if is_public_function_signature(text) => {
                        deleted_signatures.push(text.to_string());
                    }
                    LineType::Addition
                        if !crate::guard::is_line_ignored(text)
                            && is_public_function_signature(text) =>
                    {
                        added_signatures.push((text.to_string(), line.new_line_num));
                    }
                    _ => {}
                }
            }

            // If a public function was modified in this hunk
            for (added, line_num) in &added_signatures {
                let fn_name = extract_function_name(added);
                if let Some(name) = fn_name {
                    for del in &deleted_signatures {
                        let del_fn_name = extract_function_name(del);
                        if del_fn_name == Some(name) && added != del {
                            alerts.push(GuardAlert {
                                severity: Severity::High,
                                title: "Breaking Public API Signature Change".to_string(),
                                description: format!(
                                    "Modified public function signature for `{}`: was `{}` -> now `{}`",
                                    name, del, added
                                ),
                                line_no: *line_num,
                                file_path: path_str.clone(),
                            });
                        }
                    }
                }
            }
        }

        alerts
    }
}

fn is_public_function_signature(text: &str) -> bool {
    let t = text.trim();
    (t.starts_with("pub fn ")
        || t.starts_with("pub async fn ")
        || t.starts_with("export function ")
        || t.starts_with("export const ")
        || t.starts_with("public ")
        || t.starts_with("def "))
        && t.contains('(')
}

fn extract_function_name(sig: &str) -> Option<&str> {
    let before_paren = sig.split('(').next()?;
    let tokens: Vec<&str> = before_paren.split_whitespace().collect();
    tokens.last().copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ChangeType, DiffHunk, DiffLine, FileDiff, LineType};
    use std::path::PathBuf;

    #[test]
    fn test_breaking_api_change_detection() {
        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/api.rs"),
            change_type: ChangeType::Modified,
            additions: 1,
            deletions: 1,
            hunks: vec![DiffHunk {
                header: "@@ -10,1 +10,1 @@".to_string(),
                old_start: 10,
                old_lines: 1,
                new_start: 10,
                new_lines: 1,
                lines: vec![
                    DiffLine {
                        line_type: LineType::Deletion,
                        content: "pub fn authenticate(user: &str) -> bool {".into(),
                        old_line_num: Some(10),
                        new_line_num: None,
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "pub fn authenticate(user: &str, token: &str, strict: bool) -> Result<bool> {".into(),
                        old_line_num: None,
                        new_line_num: Some(10),
                    },
                ],
                is_folded: false,
            }],
            is_folded: false,
            is_reviewed: false,
        };

        let alerts = ApiDriftGuard::scan_file(&file);
        assert_eq!(alerts.len(), 1);
        assert!(alerts[0].title.contains("Breaking Public API Signature"));
    }
}
