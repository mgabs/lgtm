use crate::guard::{GuardAlert, Severity};
use crate::model::{FileDiff, LineType};
use std::collections::HashSet;

pub struct SymbolGuard;

impl SymbolGuard {
    pub fn scan_file(file: &FileDiff) -> Vec<GuardAlert> {
        let mut alerts = Vec::new();
        let path_str = file.display_name();

        let mut existing_tokens: HashSet<String> = HashSet::new();
        let mut added_tokens: Vec<(String, Option<u32>)> = Vec::new();

        for hunk in &file.hunks {
            for line in &hunk.lines {
                let tokens = extract_identifiers(&line.content);
                match line.line_type {
                    LineType::Context | LineType::Deletion => {
                        for token in tokens {
                            existing_tokens.insert(token);
                        }
                    }
                    LineType::Addition => {
                        if crate::guard::is_line_ignored(&line.content) {
                            continue;
                        }
                        let line_num = line.new_line_num;
                        for token in tokens {
                            added_tokens.push((token, line_num));
                        }
                    }
                    _ => {}
                }
            }
        }

        // Compare added tokens against existing tokens
        for (added, line_num) in added_tokens {
            if added.len() < 5 || existing_tokens.contains(&added) {
                continue;
            }

            for existing in &existing_tokens {
                if existing.len() < 5 || added == *existing {
                    continue;
                }

                if added.eq_ignore_ascii_case(existing) {
                    continue;
                }

                // Check distance
                let dist = levenshtein_distance(&added.to_lowercase(), &existing.to_lowercase());
                let normalized_match = added.len() >= 8
                    && added.replace('_', "").to_lowercase()
                        == existing.replace('_', "").to_lowercase();

                if dist == 1 || normalized_match {
                    alerts.push(GuardAlert {
                        severity: Severity::Warn,
                        title: "Probable Identifier Typo / Hallucination".to_string(),
                        description: format!(
                            "Identifier '{}' differs by only 1 edit from existing identifier '{}'",
                            added, existing
                        ),
                        line_no: line_num,
                        file_path: path_str.clone(),
                    });
                    break;
                }
            }
        }

        alerts
    }
}

pub fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let v1: Vec<char> = s1.chars().collect();
    let v2: Vec<char> = s2.chars().collect();
    let len1 = v1.len();
    let len2 = v2.len();

    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

    let mut prev_row: Vec<usize> = (0..=len2).collect();
    let mut curr_row: Vec<usize> = vec![0; len2 + 1];

    for (i, &c1) in v1.iter().enumerate() {
        curr_row[0] = i + 1;
        for (j, &c2) in v2.iter().enumerate() {
            let cost = if c1 == c2 { 0 } else { 1 };
            curr_row[j + 1] = std::cmp::min(
                std::cmp::min(curr_row[j] + 1, prev_row[j + 1] + 1),
                prev_row[j] + cost,
            );
        }
        prev_row.copy_from_slice(&curr_row);
    }

    prev_row[len2]
}

fn extract_identifiers(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    for c in text.chars() {
        if c.is_alphanumeric() || c == '_' {
            current.push(c);
        } else {
            if current.len() >= 4 && !is_keyword(&current) {
                tokens.push(current.clone());
            }
            current.clear();
        }
    }
    if current.len() >= 4 && !is_keyword(&current) {
        tokens.push(current);
    }

    tokens
}

fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "function"
            | "return"
            | "const"
            | "struct"
            | "import"
            | "export"
            | "pub"
            | "class"
            | "while"
            | "match"
            | "switch"
            | "default"
            | "async"
            | "await"
            | "string"
            | "number"
            | "boolean"
            | "usize"
            | "isize"
            | "option"
            | "result"
    )
}

impl super::Guard for SymbolGuard {
    fn name(&self) -> &'static str {
        "symbols"
    }
    fn scan_file(&self, file: &FileDiff) -> Vec<GuardAlert> {
        Self::scan_file(file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ChangeType, DiffHunk, DiffLine, FileDiff, LineType};
    use std::path::PathBuf;

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("user_id", "userid"), 1);
        assert_eq!(levenshtein_distance("account_token", "account_tokn"), 1);
        assert_eq!(levenshtein_distance("apple", "banana"), 5);
    }

    #[test]
    fn test_identifier_typo_guard() {
        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/session.rs"),
            change_type: ChangeType::Modified,
            additions: 1,
            deletions: 0,
            hunks: vec![DiffHunk {
                header: "@@ -1,2 +1,3 @@".to_string(),
                old_start: 1,
                old_lines: 2,
                new_start: 1,
                new_lines: 3,
                lines: vec![
                    DiffLine {
                        line_type: LineType::Context,
                        content: "let user_token = session.get_token();".into(),
                        old_line_num: Some(1),
                        new_line_num: Some(1),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "validate(usertoken);".into(),
                        old_line_num: None,
                        new_line_num: Some(2),
                    },
                ],
                is_folded: false,
            }],
            is_folded: false,
            is_reviewed: false,
        };

        let alerts = SymbolGuard::scan_file(&file);
        assert_eq!(alerts.len(), 1);
        assert!(alerts[0].title.contains("Identifier Typo / Hallucination"));
    }
}
