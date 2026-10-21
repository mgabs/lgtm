use crate::guard::{Guard, GuardAlert, Severity};
use crate::model::{FileDiff, LineType};
use lgtm_ast::Lang;

pub(super) struct Rule {
    pub(super) id: &'static str,
    pub(super) severity: Severity,
    pub(super) title: &'static str,
    pub(super) description: &'static str,
    pub(super) query: &'static str,
    pub(super) inside: &'static [&'static str],
}

const RULES: &[Rule] = &[
    Rule {
        id: "web/dangerous-html",
        severity: Severity::High,
        title: "dangerouslySetInnerHTML",
        description: "Raw HTML injected into the DOM; sanitize or render as text",
        query: r#"(jsx_attribute (property_identifier) @n (#eq? @n "dangerouslySetInnerHTML")) @hit"#,
        inside: &[],
    },
    Rule {
        id: "web/inner-html",
        severity: Severity::High,
        title: "innerHTML / outerHTML assignment",
        description: "Assigning HTML strings to the DOM enables XSS; use textContent or sanitize",
        query: r#"(assignment_expression left: (member_expression property: (property_identifier) @hit (#match? @hit "^(inner|outer)HTML$")))"#,
        inside: &[],
    },
    Rule {
        id: "web/eval",
        severity: Severity::Critical,
        title: "eval()",
        description: "Dynamic code execution on possibly untrusted input",
        query: r#"(call_expression function: (identifier) @hit (#eq? @hit "eval"))"#,
        inside: &[],
    },
    Rule {
        id: "next/public-secret",
        severity: Severity::Critical,
        title: "Secret exposed via NEXT_PUBLIC_ env var",
        description: "NEXT_PUBLIC_ variables are inlined into the client bundle",
        query: r#"(member_expression property: (property_identifier) @hit (#match? @hit "^NEXT_PUBLIC_.*(SECRET|TOKEN|PASSWORD|PRIVATE|API_?KEY)"))"#,
        inside: &[],
    },
    Rule {
        id: "web/hook-conditional",
        severity: Severity::High,
        title: "React hook called conditionally",
        description:
            "Hooks must run in the same order every render; move it above the condition or loop",
        query: r#"(call_expression function: (identifier) @hit (#match? @hit "^use[A-Z]"))"#,
        inside: &[
            "if_statement",
            "for_statement",
            "for_in_statement",
            "while_statement",
            "do_statement",
            "ternary_expression",
        ],
    },
    Rule {
        id: "web/ts-escape",
        severity: Severity::Warn,
        title: "TypeScript check bypassed",
        description: "@ts-ignore / @ts-nocheck hides type errors instead of fixing them",
        query: r#"((comment) @hit (#match? @hit "@ts-(ignore|nocheck)"))"#,
        inside: &[],
    },
    Rule {
        id: "next/client-server-leak",
        severity: Severity::High,
        title: "Server-only dependency in 'use client' file",
        description: "Client component imports server-only module or reads non-public process.env",
        query: r#"
            (import_statement source: (string (string_fragment) @hit (#match? @hit "^(server-only|fs|child_process|crypto|path|dns|net)$")))
            (member_expression
                object: (member_expression object: (identifier) @obj (#eq? @obj "process") property: (property_identifier) @p (#eq? @p "env"))
                property: (property_identifier) @hit (#not-match? @hit "^NEXT_PUBLIC_"))
        "#,
        inside: &[],
    },
    Rule {
        id: "next/action-unvalidated",
        severity: Severity::High,
        title: "Unvalidated Next.js Server Action",
        description:
            "'use server' function receives input without schema validation or session/auth check",
        query: r#"(statement_block (expression_statement (string (string_fragment) @hit (#eq? @hit "use server"))))"#,
        inside: &[],
    },
];

pub struct WebGuard;

impl Guard for WebGuard {
    fn name(&self) -> &'static str {
        "web"
    }

    fn scan_file(&self, file: &FileDiff) -> Vec<GuardAlert> {
        run(file, RULES, |l| matches!(l, Lang::Ts | Lang::Tsx))
    }
}

/// Shared by the AST guard packs: runs `rules` over the added lines of files whose language passes `accept`.
pub(super) fn run(file: &FileDiff, rules: &[Rule], accept: fn(Lang) -> bool) -> Vec<GuardAlert> {
    let Some(lang) = lgtm_ast::lang_of(&file.new_path).filter(|l| accept(*l)) else {
        return Vec::new();
    };
    let path = file.display_name();
    let mut alerts = Vec::new();

    // Context-sensitive whole-file parse for Next.js "use client" leak rule
    let is_use_client_file = file.hunks.iter().any(|h| {
        h.lines.iter().any(|l| {
            matches!(l.line_type, LineType::Addition | LineType::Context)
                && l.content.contains("\"use client\"")
        })
    });

    for hunk in &file.hunks {
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
        for r in rules {
            // "next/client-server-leak" only applies when the file is declared "use client"
            if r.id == "next/client-server-leak" && !is_use_client_file {
                continue;
            }
            let Ok(rows) = lgtm_ast::hits_within(lang, &src, r.query, r.inside) else {
                continue;
            };
            for row in rows {
                let Some(l) = side.get(row).filter(|l| l.line_type == LineType::Addition) else {
                    continue;
                };
                alerts.push(GuardAlert {
                    rule_id: r.id.into(),
                    fingerprint: String::new(),
                    severity: r.severity,
                    title: r.title.into(),
                    description: r.description.into(),
                    line_no: l.new_line_num,
                    file_path: path.clone(),
                });
            }
        }
    }
    alerts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ChangeType, DiffHunk, DiffLine};
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
        WebGuard
            .scan_file(&file(path, &lines))
            .into_iter()
            .map(|a| a.rule_id)
            .collect()
    }

    #[test]
    fn rules_fire_on_bad_code_and_not_on_good() {
        let cases = [
            (
                "a.tsx",
                "<div dangerouslySetInnerHTML={{ __html: h }} />",
                "web/dangerous-html",
            ),
            ("a.ts", "el.innerHTML = html;", "web/inner-html"),
            ("a.ts", "eval(code);", "web/eval"),
            (
                "a.ts",
                "const k = process.env.NEXT_PUBLIC_STRIPE_SECRET;",
                "next/public-secret",
            ),
            (
                "a.tsx",
                "function A() {\n  if (x) {\n    useState(0);\n  }\n}",
                "web/hook-conditional",
            ),
            ("a.ts", "// @ts-ignore\nfoo();", "web/ts-escape"),
            (
                "a.tsx",
                "\"use client\";\nimport { server } from \"server-only\";",
                "next/client-server-leak",
            ),
            (
                "a.ts",
                "async function updateUser() {\n  \"use server\";\n  db.users.update();\n}",
                "next/action-unvalidated",
            ),
        ];
        for (path, code, id) in cases {
            assert_eq!(ids(path, code), [id], "{id}");
        }
        for (path, code) in [
            ("a.tsx", "<p>{text}</p>"),
            ("a.ts", "el.textContent = t;"),
            ("a.ts", "const u = process.env.NEXT_PUBLIC_API_URL;"),
            ("a.tsx", "function A() {\n  const [a] = useState(0);\n}"),
            ("a.ts", "// regular comment"),
        ] {
            assert!(ids(path, code).is_empty(), "{code}");
        }
    }

    #[test]
    fn flags_only_added_dangerous_html() {
        use LineType::*;
        let jsx = "<div dangerouslySetInnerHTML={{ __html: h }} />";
        let f = file("a.tsx", &[(Context, "const a = 1;"), (Addition, jsx)]);
        let a = WebGuard.scan_file(&f);
        assert_eq!((a.len(), a[0].line_no), (1, Some(2)));
        assert_eq!(a[0].rule_id, "web/dangerous-html");
        // pre-existing (context) usage is not this change's fault
        assert!(WebGuard
            .scan_file(&file("a.tsx", &[(Context, jsx)]))
            .is_empty());
        // wrong language and clean code stay silent
        assert!(WebGuard
            .scan_file(&file("a.rs", &[(Addition, jsx)]))
            .is_empty());
        assert!(WebGuard
            .scan_file(&file("a.tsx", &[(Addition, "<p>{text}</p>")]))
            .is_empty());
    }
}
