use super::web::{run, Rule};
use crate::guard::{Guard, GuardAlert, Severity};
use crate::model::FileDiff;
use lgtm_ast::Lang;

const RULES: &[Rule] = &[
    Rule {
        id: "python/eval-exec",
        severity: Severity::Critical,
        title: "Dynamic code execution (eval / exec)",
        description: "eval() and exec() execute arbitrary code dynamically on untrusted input",
        query: r#"(call function: (identifier) @hit (#match? @hit "^(eval|exec)$"))"#,
        inside: &[],
    },
    Rule {
        id: "python/shell-injection",
        severity: Severity::Critical,
        title: "Subprocess shell injection (shell=True)", // lgtm:ignore
        description: "Executing subprocesses with shell=True invites command injection vulnerabilities", // lgtm:ignore
        query: r#"(keyword_argument name: (identifier) @n (#eq? @n "shell") value: (true) @hit)"#,
        inside: &[],
    },
    Rule {
        id: "python/unsafe-pickle",
        severity: Severity::Critical,
        title: "Insecure deserialization (pickle.loads)",
        description: "pickle deserialization executes arbitrary bytecode; use JSON or safe formats",
        query: r#"(call function: (attribute object: (identifier) @obj (#eq? @obj "pickle") attribute: (identifier) @hit (#match? @hit "^(load|loads)$")))"#,
        inside: &[],
    },
    Rule {
        id: "python/bare-except",
        severity: Severity::Warn,
        title: "Bare except clause",
        description: "A bare 'except:' catches KeyboardInterrupt and system exits; catch Exception or specific errors",
        query: r#"((except_clause) @hit (#match? @hit "^except\\s*:"))"#,
        inside: &[],
    },
];

pub struct PythonGuard;

impl Guard for PythonGuard {
    fn name(&self) -> &'static str {
        "python"
    }

    fn scan_file(&self, file: &FileDiff) -> Vec<GuardAlert> {
        run(file, RULES, |l| matches!(l, Lang::Python))
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
        PythonGuard
            .scan_file(&file(path, &lines))
            .into_iter()
            .map(|a| a.rule_id)
            .collect()
    }

    #[test]
    fn test_python_rules_fire_correctly() {
        assert_eq!(ids("test.py", "eval(user_input)"), vec!["python/eval-exec"]);
        assert_eq!(ids("test.py", "exec(user_input)"), vec!["python/eval-exec"]);
        assert_eq!(
            ids("test.py", "subprocess.run(cmd, shell=True)"), // lgtm:ignore
            vec!["python/shell-injection"]
        );
        assert_eq!(
            ids("test.py", "pickle.loads(payload)"), // lgtm:ignore
            vec!["python/unsafe-pickle"]
        );
        assert_eq!(
            ids("test.py", "try:\n    do_something()\nexcept:\n    pass"),
            vec!["python/bare-except"]
        );

        // Clean code passes
        assert!(ids("test.py", "subprocess.run(['ls', '-la'], shell=False)").is_empty());
        assert!(ids("test.py", "json.loads(payload)").is_empty());
        assert!(ids(
            "test.py",
            "try:\n    do_something()\nexcept Exception:\n    pass"
        )
        .is_empty());
    }
}
