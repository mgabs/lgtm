use crate::guard::{GuardAlert, Severity};
use crate::model::{DiffHunk, FileDiff, LineType};

pub struct ComplexityGuard;

impl ComplexityGuard {
    pub fn scan_file(file: &FileDiff) -> Vec<GuardAlert> {
        let mut alerts = Vec::new();
        let path_str = file.display_name();

        for hunk in &file.hunks {
            let ast = ast_complexity(file, hunk);
            let mut complexity = ast.unwrap_or(0);
            let mut nesting_level = 0;

            for line in hunk.lines.iter().filter(|_| ast.is_none()) {
                if line.line_type == LineType::Addition {
                    let text = line.content.trim();

                    // Adjust nesting
                    let open_braces = text.chars().filter(|&c| c == '{').count();
                    let close_braces = text.chars().filter(|&c| c == '}').count();

                    // Check conditionals and loops
                    let (is_control_flow, nesting_inc) = evaluate_cognitive_increment(text);
                    if is_control_flow {
                        complexity += 1 + nesting_level;
                    }

                    if open_braces > close_braces {
                        nesting_level += open_braces - close_braces;
                    } else if close_braces > open_braces {
                        nesting_level = nesting_level.saturating_sub(close_braces - open_braces);
                    }

                    if nesting_inc {
                        nesting_level += 1;
                    }
                }
            }

            let added_count = hunk
                .lines
                .iter()
                .filter(|l| l.line_type == LineType::Addition)
                .count();
            let high_density = (added_count > 100
                && (complexity as f64 / added_count as f64) >= 3.0
                && complexity >= 150)
                || (added_count > 60
                    && added_count <= 100
                    && (complexity as f64 / added_count as f64) >= 1.8
                    && complexity >= 40);
            let hunk_spike =
                added_count <= 60 && (complexity >= 15 || (complexity >= 8 && added_count <= 25));
            if hunk_spike || high_density {
                let line_num = hunk.lines.iter().find_map(|l| l.new_line_num);
                alerts.push(GuardAlert {
                    severity: Severity::High,
                    rule_id: "cx/complexity-spike".to_string(), fingerprint: String::new(), title: "Cognitive Complexity Spike".to_string(),
                    description: format!(
                        "Hunk introduces high cognitive complexity (+{} points with deeply nested logic)",
                        complexity
                    ),
                    line_no: line_num,
                    file_path: path_str.clone(),
                });
            }
        }

        alerts
    }
}

/// AST score for languages lgtm-ast parses; None means "use the text heuristic".
fn ast_complexity(file: &FileDiff, hunk: &DiffHunk) -> Option<usize> {
    let lang = lgtm_ast::lang_of(&file.new_path)?;
    // ponytail: hunk-local fragment parse, same ceiling as the web/swift packs
    let side: Vec<_> = hunk
        .lines
        .iter()
        .filter(|l| matches!(l.line_type, LineType::Addition | LineType::Context))
        .collect();
    let src = side
        .iter()
        .map(|l| l.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    lgtm_ast::cognitive(lang, &src, |row| {
        side.get(row)
            .is_some_and(|l| l.line_type == LineType::Addition)
    })
    .ok()
}

fn evaluate_cognitive_increment(text: &str) -> (bool, bool) {
    let t = text.to_lowercase();
    if t.starts_with("//") || t.starts_with('#') || t.starts_with("/*") {
        return (false, false);
    }

    let is_if = t.starts_with("if ") || t.starts_with("if(") || t.contains(" else if ");
    let is_loop = t.starts_with("for ")
        || t.starts_with("for(")
        || t.starts_with("while ")
        || t.starts_with("while(");
    let is_match = t.starts_with("match ") || t.starts_with("switch ");
    let is_catch = t.starts_with("catch ") || t.starts_with("catch(") || t.starts_with("except ");
    let is_binary_logic = (t.contains(" && ") || t.contains(" || ")) && !is_if;
    let is_ternary = t.contains(" ? ") && t.contains(" : ");

    let is_control = is_if || is_loop || is_match || is_catch || is_binary_logic || is_ternary;
    let nesting_inc = is_if || is_loop || is_match || is_catch;

    (is_control, nesting_inc)
}

impl super::Guard for ComplexityGuard {
    fn name(&self) -> &'static str {
        "complexity"
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
    fn test_cognitive_complexity_spike() {
        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/processor.rs"),
            change_type: ChangeType::Modified,
            additions: 12,
            deletions: 0,
            hunks: vec![DiffHunk {
                header: "@@ -1,0 +1,12 @@".to_string(),
                old_start: 1,
                old_lines: 0,
                new_start: 1,
                new_lines: 12,
                lines: vec![
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "if valid {".into(),
                        old_line_num: None,
                        new_line_num: Some(1),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "    for item in items {".into(),
                        old_line_num: None,
                        new_line_num: Some(2),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "        if item.active {".into(),
                        old_line_num: None,
                        new_line_num: Some(3),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "            while item.pending() {".into(),
                        old_line_num: None,
                        new_line_num: Some(4),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "                if item.retry() { process(); }".into(),
                        old_line_num: None,
                        new_line_num: Some(5),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "            }".into(),
                        old_line_num: None,
                        new_line_num: Some(6),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "        }".into(),
                        old_line_num: None,
                        new_line_num: Some(7),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "    }".into(),
                        old_line_num: None,
                        new_line_num: Some(8),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "}".into(),
                        old_line_num: None,
                        new_line_num: Some(9),
                    },
                ],
                is_folded: false,
            }],
            is_folded: false,
            is_reviewed: false,
        };

        let alerts = ComplexityGuard::scan_file(&file);
        assert_eq!(alerts.len(), 1);
        assert!(alerts[0].title.contains("Cognitive Complexity Spike"));
    }

    fn added(path: &str, code: &str) -> FileDiff {
        FileDiff {
            old_path: None,
            new_path: PathBuf::from(path),
            change_type: ChangeType::Modified,
            additions: 0,
            deletions: 0,
            hunks: vec![DiffHunk {
                header: "@@".into(),
                old_start: 1,
                old_lines: 0,
                new_start: 1,
                new_lines: 0,
                lines: code
                    .lines()
                    .enumerate()
                    .map(|(i, c)| DiffLine {
                        line_type: LineType::Addition,
                        content: c.to_string(),
                        old_line_num: None,
                        new_line_num: Some(i as u32 + 1),
                    })
                    .collect(),
                is_folded: false,
            }],
            is_folded: false,
            is_reviewed: false,
        }
    }

    #[test]
    fn ast_ignores_keywords_in_strings_and_flags_real_nesting() {
        let template = format!("const t = `\n{}`;", "if (a) {\n".repeat(10));
        assert!(ComplexityGuard::scan_file(&added("a.ts", &template)).is_empty());
        let mut real = String::from("function f() {\n");
        for i in 0..6 {
            real.push_str(&format!("{}if (c{i}) {{\n", "  ".repeat(i)));
        }
        real.push_str(&"}\n".repeat(7));
        assert_eq!(ComplexityGuard::scan_file(&added("a.ts", &real)).len(), 1);
        let swift = "func f() {\n  guard a else { return }\n}\n";
        assert!(ComplexityGuard::scan_file(&added("a.swift", swift)).is_empty());
    }
}
