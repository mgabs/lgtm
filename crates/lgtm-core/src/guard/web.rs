use crate::guard::{Guard, GuardAlert, Severity};
use crate::model::{FileDiff, LineType};

const DANGEROUS_HTML: &str =
    r#"(jsx_attribute (property_identifier) @n (#eq? @n "dangerouslySetInnerHTML")) @hit"#;

pub struct WebGuard;

impl Guard for WebGuard {
    fn name(&self) -> &'static str {
        "web"
    }

    fn scan_file(&self, file: &FileDiff) -> Vec<GuardAlert> {
        let Some(lang) = lgtm_ast::lang_of(&file.new_path) else {
            return Vec::new();
        };
        let path = file.display_name();
        let mut alerts = Vec::new();
        for hunk in &file.hunks {
            // ponytail: parses each hunk's new-side lines as a fragment; full-file parse if context-sensitive rules need it
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
            let Ok(rows) = lgtm_ast::hits(lang, &src, DANGEROUS_HTML) else {
                continue;
            };
            for row in rows {
                let Some(l) = side.get(row).filter(|l| l.line_type == LineType::Addition) else {
                    continue;
                };
                alerts.push(GuardAlert {
                    rule_id: "web/dangerous-html".into(),
                    fingerprint: String::new(),
                    severity: Severity::High,
                    title: "dangerouslySetInnerHTML".into(),
                    description: "Raw HTML injected into the DOM; sanitize or render as text"
                        .into(),
                    line_no: l.new_line_num,
                    file_path: path.clone(),
                });
            }
        }
        alerts
    }
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
            }],
            is_folded: false,
            is_reviewed: false,
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
