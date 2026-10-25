use super::web::{run, Rule};
use crate::guard::{Guard, GuardAlert, Severity};
use crate::model::FileDiff;
use lgtm_ast::Lang;

const RULES: &[Rule] = &[
    Rule {
        id: "go/sql-concat",
        severity: Severity::Critical,
        title: "SQL injection via string concatenation",
        description: "Executing SQL constructed via fmt.Sprintf or string concatenation",
        query: r#"((type_identifier) @hit (#match? @hit "^(Query|QueryRow|Exec)$"))"#,
        inside: &[],
    },
    Rule {
        id: "go/command-injection",
        severity: Severity::Critical,
        title: "Command injection (exec.Command with shell)",
        description: "Invoking 'sh -c' or 'bash -c' via exec.Command invites command injection vulnerabilities",
        query: r#"(call_expression function: (selector_expression operand: (identifier) @pkg field: (field_identifier) @hit) (#eq? @pkg "exec") (#eq? @hit "Command") arguments: (argument_list (interpreted_string_literal) @sh (#match? @sh "^\"(sh|bash|zsh)\"$")))"#,
        inside: &[],
    },
    Rule {
        id: "go/unhandled-error",
        severity: Severity::Warn,
        title: "Unhandled error assigned to blank identifier",
        description: "Assigning error returns to blank identifier '_' bypasses error handling",
        query: r#"(assignment_statement left: (expression_list (identifier) @hit (#eq? @hit "_")))"#,
        inside: &[],
    },
];

pub struct GoGuard;

impl Guard for GoGuard {
    fn scan_file(&self, file: &FileDiff) -> Vec<GuardAlert> {
        run(file, RULES, |l| matches!(l, Lang::Go))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ChangeType, DiffHunk, DiffLine, LineType};
    use std::path::PathBuf;

    fn file(path: &str, lines: &[(LineType, &str)]) -> FileDiff {
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
                lines: lines
                    .iter()
                    .enumerate()
                    .map(|(i, (t, c))| DiffLine {
                        line_type: *t,
                        content: c.to_string(),
                        old_line_num: None,
                        new_line_num: Some(i as u32 + 1),
                    })
                    .collect(),
                is_folded: false,
                folded_blocks: Vec::new(),
            }],
            is_folded: false,
            is_reviewed: false,
        }
    }

    fn ids(path: &str, code: &str) -> Vec<String> {
        let lines: Vec<_> = code.lines().map(|c| (LineType::Addition, c)).collect();
        GoGuard
            .scan_file(&file(path, &lines))
            .into_iter()
            .map(|a| a.rule_id)
            .collect()
    }

    #[test]
    fn test_go_rules_fire_correctly() {
        assert_eq!(
            ids(
                "test.go",
                "db.Query(fmt.Sprintf(\"SELECT * FROM users WHERE id = %s\", id))"
            ),
            vec!["go/sql-concat"]
        );
        assert_eq!(
            ids("test.go", "exec.Command(\"bash\", \"-c\", userCmd)"),
            vec!["go/command-injection"]
        );
        assert_eq!(
            ids("test.go", "_, err := os.Open(f)\n_ = doSomething()"),
            vec!["go/unhandled-error"]
        );

        // Clean code passes
        assert!(ids(
            "test.go",
            "db.Query(\"SELECT * FROM users WHERE id = $1\", id)"
        )
        .is_empty());
        assert!(ids("test.go", "exec.Command(\"ls\", \"-la\")").is_empty());
        assert!(ids("test.go", "res, err := doSomething()").is_empty());
    }
}
