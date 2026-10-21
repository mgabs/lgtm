use super::web::{run, Rule};
use crate::guard::{Guard, GuardAlert, Severity};
use crate::model::FileDiff;
use lgtm_ast::Lang;

const RULES: &[Rule] = &[
    Rule {
        id: "java/sql-injection",
        severity: Severity::Critical,
        title: "SQL injection via unparameterized JDBC query",
        description:
            "Invoking executeQuery or executeUpdate with raw SQL instead of PreparedStatement",
        query: r#"((method_invocation name: (identifier) @hit (#match? @hit "^(executeQuery|executeUpdate|executeLargeUpdate)$")))"#,
        inside: &[],
    },
    Rule {
        id: "java/command-injection",
        severity: Severity::Critical,
        title: "Command injection via Runtime.exec",
        description: "Invoking external processes via Runtime.exec invites command injection vulnerabilities",
        query: r#"((method_invocation name: (identifier) @hit (#eq? @hit "exec")))"#,
        inside: &[],
    },
    Rule {
        id: "java/unsafe-deserialization",
        severity: Severity::High,
        title: "Unsafe ObjectInputStream deserialization",
        description:
            "Calling ObjectInputStream.readObject() on untrusted streams permits remote code execution",
        query: r#"((method_invocation name: (identifier) @hit (#eq? @hit "readObject")))"#,
        inside: &[],
    },
    Rule {
        id: "java/empty-catch",
        severity: Severity::Warn,
        title: "Empty catch block suppresses exceptions",
        description:
            "Empty catch block swallows exceptions without logging, reporting, or rethrow",
        query: r#"((catch_clause body: (block) @hit))"#,
        inside: &[],
    },
];

pub struct JavaGuard;

impl Guard for JavaGuard {
    fn name(&self) -> &'static str {
        "java"
    }

    fn scan_file(&self, file: &FileDiff) -> Vec<GuardAlert> {
        run(file, RULES, |l| matches!(l, Lang::Java))
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
                header: "@@ -1,1 +1,4 @@".into(),
                old_start: 1,
                old_lines: 1,
                new_start: 1,
                new_lines: lines.len() as u32,
                lines: lines
                    .iter()
                    .enumerate()
                    .map(|(i, (t, c))| DiffLine {
                        line_type: *t,
                        content: (*c).into(),
                        old_line_num: None,
                        new_line_num: Some((i + 1) as u32),
                    })
                    .collect(),
                is_folded: false,
                folded_blocks: Vec::new(),
            }],
            is_folded: false,
            is_reviewed: false,
        }
    }

    #[test]
    fn test_java_rules_fire_correctly() {
        let bad = [
            (LineType::Addition, "class Vuln {"),
            (
                LineType::Addition,
                "  void run(String cmd, String q) throws Exception {",
            ),
            (LineType::Addition, "    Runtime.getRuntime().exec(cmd);"),
            (LineType::Addition, "    stmt.executeQuery(q);"),
            (LineType::Addition, "    Object o = in.readObject();"),
            (
                LineType::Addition,
                "    try { doWork(); } catch (Exception e) {}",
            ),
            (LineType::Addition, "  }"),
            (LineType::Addition, "}"),
        ];
        let alerts = JavaGuard.scan_file(&file("OrderService.java", &bad));
        let ids: Vec<_> = alerts.iter().map(|a| a.rule_id.as_str()).collect();
        assert!(ids.contains(&"java/command-injection"));
        assert!(ids.contains(&"java/sql-injection"));
        assert!(ids.contains(&"java/unsafe-deserialization"));
        assert!(ids.contains(&"java/empty-catch"));
    }

    #[test]
    fn test_java_rules_ignore_clean_code() {
        let good = [
            (LineType::Addition, "class Clean {"),
            (LineType::Addition, "  void safe() {"),
            (LineType::Addition, "    int x = 42;"),
            (LineType::Addition, "    logger.info(\"safe\");"),
            (LineType::Addition, "  }"),
            (LineType::Addition, "}"),
        ];
        let alerts = JavaGuard.scan_file(&file("CleanService.java", &good));
        assert!(alerts.is_empty());
    }
}
