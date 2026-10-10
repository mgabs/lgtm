use crate::guard::{GuardAlert, Severity};
use crate::model::{FileDiff, LineType};

pub struct DependencyGuard;

impl DependencyGuard {
    pub fn scan_file(file: &FileDiff) -> Vec<GuardAlert> {
        let mut alerts = Vec::new();
        let path_str = file.display_name();

        let is_manifest = path_str.ends_with("Cargo.toml")
            || path_str.ends_with("package.json")
            || path_str.ends_with("pyproject.toml")
            || path_str.ends_with("requirements.txt")
            || path_str.ends_with("go.mod");

        if !is_manifest {
            return alerts;
        }

        for hunk in &file.hunks {
            for line in &hunk.lines {
                if line.line_type == LineType::Addition {
                    let text = line.content.trim();
                    let line_num = line.new_line_num;

                    if crate::guard::is_line_ignored(text) {
                        continue;
                    }

                    // Unpinned or wildcard dependencies
                    if is_unpinned_wildcard(text, &path_str) {
                        alerts.push(GuardAlert {
                            severity: Severity::High,
                            title: "Unpinned / Wildcard Dependency".to_string(),
                            description: format!(
                                "Agent added unpinned wildcard dependency '{}' (supply chain risk)",
                                text
                            ),
                            line_no: line_num,
                            file_path: path_str.clone(),
                        });
                    }

                    // Suspicious or hallucinated package patterns
                    if is_suspicious_package(text) {
                        alerts.push(GuardAlert {
                            severity: Severity::Critical,
                            title: "Suspicious / Typosquatted Dependency".to_string(),
                            description: format!(
                                "Detected potentially hallucinated / risky package name: '{}'",
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

fn is_unpinned_wildcard(text: &str, path: &str) -> bool {
    let t = text.to_lowercase();
    if path.ends_with("Cargo.toml") {
        t.contains("= \"*\"") || t.contains("= \"^0.0\"") || t.contains("= \">")
    } else if path.ends_with("package.json") {
        t.contains("\"*\"") || t.contains("\">") || t.contains("\"latest\"")
    } else if path.ends_with("requirements.txt") || path.ends_with("pyproject.toml") {
        t.contains(">=0.0") || t.contains("*") || (t.contains("==") && t.contains("latest"))
    } else {
        false
    }
}

fn is_suspicious_package(text: &str) -> bool {
    let t = text.to_lowercase();
    // Common hallucinated or typosquatted package name artifacts
    t.contains("react-native-crypto-js")
        || t.contains("crossenv")
        || t.contains("colors-2")
        || t.contains("fake-helper")
        || t.contains("test-lib-mock")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ChangeType, DiffHunk, DiffLine};
    use std::path::PathBuf;

    #[test]
    fn test_unpinned_dependency_scan() {
        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("package.json"),
            change_type: ChangeType::Modified,
            additions: 2,
            deletions: 0,
            hunks: vec![DiffHunk {
                header: "@@ -10,2 +10,4 @@".to_string(),
                old_start: 10,
                old_lines: 2,
                new_start: 10,
                new_lines: 4,
                lines: vec![
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "\"unpinned-pkg\": \"*\"".to_string(),
                        old_line_num: None,
                        new_line_num: Some(12),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "\"fake-helper\": \"1.0.0\"".to_string(),
                        old_line_num: None,
                        new_line_num: Some(13),
                    },
                ],
                is_folded: false,
            }],
            is_folded: false,
            is_reviewed: false,
        };

        let alerts = DependencyGuard::scan_file(&file);
        assert_eq!(alerts.len(), 2);
        assert!(alerts
            .iter()
            .any(|a| a.title.contains("Unpinned / Wildcard")));
        assert!(alerts
            .iter()
            .any(|a| a.title.contains("Suspicious / Typosquatted")));
    }
}
