use super::web::{run, Rule};
use crate::guard::{Guard, GuardAlert, Severity};
use crate::model::FileDiff;
use lgtm_ast::Lang;

const RULES: &[Rule] = &[
    Rule {
        id: "sec/sql-injection",
        severity: Severity::Critical,
        title: "SQL injection via dynamic string interpolation",
        description: "Executing SQL constructed via format! or string interpolation invites SQL injection",
        query: r#"((macro_invocation macro: (identifier) @m (#eq? @m "format") (token_tree (string_literal (string_content) @sql (#match? @sql "(?i)(SELECT|INSERT|UPDATE|DELETE).*\\{")))) @hit)"#,
        inside: &[],
    },
    Rule {
        id: "sec/command-injection",
        severity: Severity::Critical,
        title: "Command injection via shell invocation",
        description: "Invoking 'sh -c', 'bash -c', or shell interpreters with dynamic arguments invites command injection vulnerabilities",
        query: r#"((call_expression function: (field_expression value: (call_expression function: [(scoped_identifier name: (identifier) @n (#eq? @n "new")) (field_expression field: (field_identifier) @fn (#eq? @fn "new"))] arguments: (arguments (string_literal) @sh (#match? @sh "^\"(sh|bash|zsh|cmd|powershell)\"$"))) field: (field_identifier) @sh_arg (#match? @sh_arg "^(arg|args)$")) arguments: (arguments [(string_literal) @flag (array_expression (string_literal) @flag)] (#match? @flag "^\"(-c|/C|-Command)\"$"))) @hit)"#,
        inside: &[],
    },
];

pub struct RustGuard;

impl Guard for RustGuard {
    fn scan_file(&self, file: &FileDiff) -> Vec<GuardAlert> {
        run(file, RULES, |l| matches!(l, Lang::Rust))
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
            additions: lines
                .iter()
                .filter(|(t, _)| *t == LineType::Addition)
                .count(),
            deletions: 0,
            hunks: vec![DiffHunk {
                header: "@@".into(),
                old_start: 1,
                old_lines: 0,
                new_start: 1,
                new_lines: lines.len() as u32,
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
        RustGuard
            .scan_file(&file(path, &lines))
            .into_iter()
            .map(|a| a.rule_id)
            .collect()
    }

    #[test]
    fn test_rust_sql_injection() {
        assert_eq!(
            ids(
                "src/db.rs",
                r#"let q = format!("SELECT * FROM t WHERE id = '{}'", user);"#
            ),
            vec!["sec/sql-injection"]
        );
        assert_eq!(
            ids(
                "src/db.rs",
                r#"let q = format!("INSERT INTO users VALUES ({})", val);"#
            ),
            vec!["sec/sql-injection"]
        );
        assert_eq!(
            ids(
                "src/db.rs",
                r#"let q = format!("DELETE FROM users WHERE id = {}", id);"#
            ),
            vec!["sec/sql-injection"]
        );
        assert_eq!(
            ids(
                "src/db.rs",
                r#"let q = format!("UPDATE users SET name = '{:?}'", name);"#
            ),
            vec!["sec/sql-injection"]
        );

        // Safe format without SQL
        assert!(ids("src/db.rs", r#"let msg = format!("Hello, {}!", user);"#).is_empty());
        // Safe parameterized SQL
        assert!(ids(
            "src/db.rs",
            r#"let q = sqlx::query("SELECT * FROM t WHERE id = $1").bind(user);"#
        )
        .is_empty());
    }

    #[test]
    fn test_rust_command_injection() {
        assert_eq!(
            ids(
                "src/cmd.rs",
                r#"let out = Command::new("sh").arg("-c").arg(c).output();"#
            ),
            vec!["sec/command-injection"]
        );
        assert_eq!(
            ids(
                "src/cmd.rs",
                r#"std::process::Command::new("sh").arg("-c").arg(c);"#
            ),
            vec!["sec/command-injection"]
        );
        assert_eq!(
            ids(
                "src/cmd.rs",
                r#"Command::new("bash").arg("-c").arg(user_cmd);"#
            ),
            vec!["sec/command-injection"]
        );
        assert_eq!(
            ids(
                "src/cmd.rs",
                r#"Command::new("sh").args(["-c", user_cmd]);"#
            ),
            vec!["sec/command-injection"]
        );
        assert_eq!(
            ids(
                "src/cmd.rs",
                r#"Command::new("cmd").arg("/C").arg(user_cmd);"#
            ),
            vec!["sec/command-injection"]
        );
        assert_eq!(
            ids(
                "src/cmd.rs",
                r#"Command::new("powershell").arg("-Command").arg(user_cmd);"#
            ),
            vec!["sec/command-injection"]
        );

        // Safe command without shell -c
        assert!(ids("src/cmd.rs", r#"Command::new("git").arg("status");"#).is_empty());
        assert!(ids("src/cmd.rs", r#"Command::new("cargo").arg("build");"#).is_empty());
        // Safe CLI builder with -c flag
        assert!(ids(
            "src/cli.rs",
            r#"let app = Command::new("my-cli").arg("-c");"#
        )
        .is_empty());
    }
}
