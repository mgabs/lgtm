use crate::guard::{GuardAlert, Severity};
use crate::model::{DiffHunk, DiffLine, FileDiff, LineType};
use std::collections::HashSet;

pub struct SymbolGuard;

impl SymbolGuard {
    pub fn scan_file(file: &FileDiff) -> Vec<GuardAlert> {
        let mut alerts = Vec::new();
        let path_str = file.display_name();

        let ext = file
            .new_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        if ext == "md"
            || ext == "txt"
            || ext == "json"
            || ext == "toml"
            || ext == "yaml"
            || ext == "yml"
        {
            return alerts;
        }

        let mut existing_tokens: HashSet<String> = HashSet::new();
        let mut added_tokens: Vec<(String, Option<u32>)> = Vec::new();

        let ast_lang = lgtm_ast::lang_of(&file.new_path);
        for hunk in file.hunks.iter().filter(|_| ast_lang.is_some()) {
            if let Some(lang) = ast_lang {
                ast_tokens(lang, hunk, &mut existing_tokens, &mut added_tokens);
            }
        }

        for hunk in file.hunks.iter().filter(|_| ast_lang.is_none()) {
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

                if added.eq_ignore_ascii_case(existing)
                    || is_naming_convention_variant(&added, existing)
                {
                    continue;
                }

                // Check distance
                let dist = levenshtein_distance(&added.to_lowercase(), &existing.to_lowercase());

                if dist == 1 {
                    alerts.push(GuardAlert {
                        severity: Severity::Warn,
                        rule_id: "typo/identifier".to_string(),
                        fingerprint: String::new(),
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

pub fn is_naming_convention_variant(added: &str, existing: &str) -> bool {
    let norm_added = added.replace('_', "").to_lowercase();
    let norm_existing = existing.replace('_', "").to_lowercase();
    if norm_added == norm_existing {
        return true;
    }

    // Stem-safe pluralization check: require length >= 5 and avoid words ending with 'ss' (e.g. process, class, address)
    let is_valid_stem = |w: &str| w.len() >= 5 && !w.ends_with("ss");
    if is_valid_stem(&norm_added) && norm_added.strip_suffix('s') == Some(&norm_existing) {
        return true;
    }
    if is_valid_stem(&norm_existing) && norm_existing.strip_suffix('s') == Some(&norm_added) {
        return true;
    }

    false
}

/// Identifiers from parsed hunk fragments: old side feeds `existing`, additions feed `added`.
fn ast_tokens(
    lang: lgtm_ast::Lang,
    hunk: &DiffHunk,
    existing: &mut HashSet<String>,
    added: &mut Vec<(String, Option<u32>)>,
) {
    let keep = |t: &str| t.len() >= 4 && !is_keyword(t);
    let join = |side: &[&DiffLine]| {
        side.iter()
            .map(|l| l.content.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    };
    let new_side: Vec<_> = hunk
        .lines
        .iter()
        .filter(|l| matches!(l.line_type, LineType::Addition | LineType::Context))
        .collect();
    let old_side: Vec<_> = hunk
        .lines
        .iter()
        .filter(|l| matches!(l.line_type, LineType::Deletion | LineType::Context))
        .collect();
    for (name, _) in lgtm_ast::identifiers(lang, &join(&old_side)).unwrap_or_default() {
        if keep(&name) {
            existing.insert(name);
        }
    }
    for (name, row) in lgtm_ast::identifiers(lang, &join(&new_side)).unwrap_or_default() {
        let Some(l) = new_side.get(row) else { continue };
        if !keep(&name) {
            continue;
        }
        if l.line_type == LineType::Context {
            existing.insert(name);
        } else if !crate::guard::is_line_ignored(&l.content) {
            added.push((name, l.new_line_num));
        }
    }
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
                        content: "validate(user_tokn);".into(),
                        old_line_num: None,
                        new_line_num: Some(2),
                    },
                ],
                is_folded: false,
                folded_blocks: Vec::new(),
            }],
            is_folded: false,
            is_reviewed: false,
        };

        let alerts = SymbolGuard::scan_file(&file);
        assert_eq!(alerts.len(), 1);
        assert!(alerts[0].title.contains("Identifier Typo / Hallucination"));
    }

    #[test]
    fn ast_typo_ignores_strings_and_comments() {
        let mk = |add: &str| FileDiff {
            old_path: None,
            new_path: PathBuf::from("a.ts"),
            change_type: ChangeType::Modified,
            additions: 1,
            deletions: 0,
            hunks: vec![DiffHunk {
                header: "@@".into(),
                old_start: 1,
                old_lines: 1,
                new_start: 1,
                new_lines: 2,
                lines: vec![
                    DiffLine {
                        line_type: LineType::Context,
                        content: "const userToken = get();".into(),
                        old_line_num: Some(1),
                        new_line_num: Some(1),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: add.into(),
                        old_line_num: None,
                        new_line_num: Some(2),
                    },
                ],
                is_folded: false,
                folded_blocks: Vec::new(),
            }],
            is_folded: false,
            is_reviewed: false,
        };
        let hit = SymbolGuard::scan_file(&mk("validate(userTokn);"));
        assert_eq!((hit.len(), hit[0].line_no), (1, Some(2)));
        assert!(SymbolGuard::scan_file(&mk("log(\"userTokn\"); // userTokn")).is_empty());
    }

    #[test]
    fn test_naming_convention_variant() {
        // Casing normalization
        assert!(is_naming_convention_variant(
            "CommentInput",
            "comment_input"
        ));
        assert!(is_naming_convention_variant("active_view", "ActiveView"));
        assert!(is_naming_convention_variant("userName", "user_name"));

        // Pluralization / singular equivalence
        assert!(is_naming_convention_variant("directives", "directive"));
        assert!(is_naming_convention_variant("directive", "directives"));
        assert!(is_naming_convention_variant("comments", "comment"));

        // Stem safeguards on words ending in 'ss' (must NOT be treated as plural variant)
        assert!(!is_naming_convention_variant("proces", "process"));
        assert!(!is_naming_convention_variant("clas", "class"));
        assert!(!is_naming_convention_variant("addres", "address"));

        // Real typos
        assert!(!is_naming_convention_variant("token", "tokn"));
    }

    #[test]
    fn levenshtein_handles_empty_sides() {
        assert_eq!(levenshtein_distance("", "abc"), 3);
        assert_eq!(levenshtein_distance("abcd", ""), 4);
    }

    #[test]
    fn plain_text_typos_skip_ignored_lines_and_headers() {
        let line = |line_type, content: &str, n| DiffLine {
            line_type,
            content: content.into(),
            old_line_num: None,
            new_line_num: Some(n),
        };
        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("notes.zzz"),
            change_type: ChangeType::Modified,
            additions: 2,
            deletions: 0,
            hunks: vec![DiffHunk {
                header: "@@".into(),
                old_start: 1,
                old_lines: 1,
                new_start: 1,
                new_lines: 3,
                lines: vec![
                    line(LineType::HunkHeader, "@@ user_token", 0),
                    line(LineType::Context, "user_token", 1),
                    line(LineType::Addition, "user_tokn # lgtm:ignore", 2),
                    line(LineType::Addition, "user_tokn", 3),
                ],
                is_folded: false,
                folded_blocks: Vec::new(),
            }],
            is_folded: false,
            is_reviewed: false,
        };
        let alerts = SymbolGuard::scan_file(&file);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].line_no, Some(3));
    }
}
