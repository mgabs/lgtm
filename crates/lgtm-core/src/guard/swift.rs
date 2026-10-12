use super::web::{run, Rule};
use crate::guard::{Guard, GuardAlert, Severity};
use crate::model::FileDiff;
use lgtm_ast::Lang;

const RULES: &[Rule] = &[
    Rule {
        id: "swift/force-unwrap",
        severity: Severity::Warn,
        title: "Force unwrap (!)",
        description: "Crashes at runtime when nil; use guard let / if let / ??",
        query: r#"(postfix_expression operation: (bang)) @hit"#,
        inside: &[],
    },
    Rule {
        id: "swift/try-bang",
        severity: Severity::High,
        title: "try!",
        description: "Crashes on any thrown error; use do/catch or try?",
        query: r#"((try_operator) @hit (#match? @hit "^try\\s*!"))"#,
        inside: &[],
    },
    Rule {
        id: "swift/force-cast",
        severity: Severity::Warn,
        title: "Force cast (as!)",
        description: "Crashes when the cast fails; use as? with a guard",
        query: r#"((as_operator) @hit (#match? @hit "^as\\s*!"))"#,
        inside: &[],
    },
    Rule {
        id: "swift/weak-crypto",
        severity: Severity::High,
        title: "Weak cryptographic primitive",
        description: "MD5 / SHA-1 are broken for security use; use SHA-256 or better",
        query: r#"
            ((simple_identifier) @hit (#match? @hit "^CC_(MD5|SHA1)$"))
            (navigation_suffix suffix: (simple_identifier) @hit (#match? @hit "^(MD5|SHA1)$"))
        "#,
        inside: &[],
    },
    Rule {
        id: "swift/secret-in-userdefaults",
        severity: Severity::High,
        title: "Secret stored in UserDefaults",
        description: "UserDefaults is unencrypted plist storage; use the Keychain",
        query: r#"((call_expression
            (call_suffix (value_arguments (value_argument
                name: (value_argument_label (simple_identifier) @l (#eq? @l "forKey"))
                value: (line_string_literal text: (line_str_text) @hit
                    (#match? @hit "(?i)(token|password|secret|api_?key)")))))) @c
            (#match? @c "UserDefaults"))"#,
        inside: &[],
    },
    Rule {
        id: "test/xctskip",
        severity: Severity::Warn,
        title: "Test skipped (XCTSkip)",
        description: "Skipping a test hides a failure instead of fixing it",
        query: r#"((simple_identifier) @hit (#eq? @hit "XCTSkip"))"#,
        inside: &[],
    },
    Rule {
        id: "swift/empty-catch",
        severity: Severity::Warn,
        title: "Empty catch block",
        description: "Swallows errors silently; handle or log them",
        query: r#"((catch_block) @hit (#match? @hit "(?s)^catch[^{]*\\{\\s*\\}$"))"#,
        inside: &[],
    },
];

pub struct SwiftGuard;

impl Guard for SwiftGuard {
    fn name(&self) -> &'static str {
        "swift"
    }

    fn scan_file(&self, file: &FileDiff) -> Vec<GuardAlert> {
        run(file, RULES, |l| l == Lang::Swift)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ChangeType, DiffHunk, DiffLine, LineType};
    use std::path::PathBuf;

    fn ids(path: &str, code: &str) -> Vec<String> {
        let f = FileDiff {
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
        };
        SwiftGuard
            .scan_file(&f)
            .into_iter()
            .map(|a| a.rule_id)
            .collect()
    }

    #[test]
    fn rules_fire_on_bad_code_and_not_on_good() {
        let cases = [
            ("let a = x!", "swift/force-unwrap"),
            ("let b = try! foo()", "swift/try-bang"),
            ("let c = v as! String", "swift/force-cast"),
            ("CC_MD5(p, 1, q)", "swift/weak-crypto"),
            ("let h = Insecure.SHA1.hash(data: d)", "swift/weak-crypto"),
            (
                "UserDefaults.standard.set(t, forKey: \"authToken\")",
                "swift/secret-in-userdefaults",
            ),
            ("throw XCTSkip(\"no\")", "test/xctskip"),
            ("do { try f() } catch { }", "swift/empty-catch"),
        ];
        for (code, id) in cases {
            assert_eq!(ids("a.swift", code), [id], "{code}");
        }
        for code in [
            "let a = x ?? 0",
            "let b = try? foo()",
            "let c = v as? String",
            "let h = SHA256.hash(data: d)",
            "UserDefaults.standard.set(1, forKey: \"theme\")",
            "store.set(t, forKey: \"authToken\")",
            "do { try f() } catch { log(error) }",
        ] {
            assert!(ids("a.swift", code).is_empty(), "{code}");
        }
        assert!(ids("a.ts", "let a = x!").is_empty());
    }
}
