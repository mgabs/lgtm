use crate::guard::{GuardAlert, Severity};
use crate::model::{FileDiff, LineType};

pub struct FragilityGuard;

impl FragilityGuard {
    pub fn scan_file(file: &FileDiff) -> Vec<GuardAlert> {
        let mut alerts = Vec::new();
        let path_str = file.display_name();
        let path_lower = path_str.to_lowercase();

        // Skip test files from fragility checks
        if crate::guard::is_test_path(&path_str) {
            return alerts;
        }

        let is_rust = path_lower.ends_with(".rs");
        let is_python = path_lower.ends_with(".py") || path_lower.ends_with(".pyi");
        let is_js_ts = path_lower.ends_with(".ts")
            || path_lower.ends_with(".tsx")
            || path_lower.ends_with(".js")
            || path_lower.ends_with(".jsx");

        let mut in_render_or_hot_func = false;
        let mut in_test_mod = false;

        for hunk in &file.hunks {
            for line in &hunk.lines {
                let text = line.content.trim();
                let line_num = line.new_line_num;

                if text.contains("mod test")
                    || text.contains("mod tests")
                    || text.contains("#[cfg(test)]")
                    || text.starts_with("class Test")
                    || text.starts_with("describe(")
                {
                    in_test_mod = true;
                }

                // Track if we are inside a render/draw/tick function
                if text.starts_with("pub fn render_")
                    || text.starts_with("fn render_")
                    || text.starts_with("pub(crate) fn render_")
                    || text.starts_with("def render_")
                    || text.starts_with("function render")
                    || text.starts_with("const render")
                {
                    in_render_or_hot_func = true;
                }

                if in_test_mod {
                    continue;
                }

                if line.line_type == LineType::Addition {
                    if crate::guard::is_line_ignored(text) {
                        continue;
                    }

                    // 1. Silent Error Swallowing
                    if is_rust {
                        if is_rust_silent_error(text) {
                            alerts.push(GuardAlert {
                                severity: Severity::Warn,
                                rule_id: "fragility/silent-error".to_string(),
                                fingerprint: String::new(),
                                title: "Silent Error Swallowing".to_string(),
                                description: format!(
                                    "Fallible operation result is silently swallowed without logging or propagation: '{}'",
                                    text
                                ),
                                line_no: line_num,
                                file_path: path_str.clone(),
                            });
                        }
                    } else if is_python {
                        if is_python_silent_error(text) {
                            alerts.push(GuardAlert {
                                severity: Severity::Warn,
                                rule_id: "fragility/silent-error".to_string(),
                                fingerprint: String::new(),
                                title: "Silent Exception Swallowing".to_string(),
                                description: format!(
                                    "Bare except block suppresses errors without logging or rethrow: '{}'",
                                    text
                                ),
                                line_no: line_num,
                                file_path: path_str.clone(),
                            });
                        }
                    } else if is_js_ts && is_js_silent_error(text) {
                        alerts.push(GuardAlert {
                            severity: Severity::Warn,
                            rule_id: "fragility/silent-error".to_string(),
                            fingerprint: String::new(),
                            title: "Silent Catch Swallowing".to_string(),
                            description: format!(
                                "Empty catch block suppresses errors without logging or rethrow: '{}'",
                                text
                            ),
                            line_no: line_num,
                            file_path: path_str.clone(),
                        });
                    }

                    // 2. Synchronous I/O in Hot Render / Tick Loops
                    if in_render_or_hot_func && is_sync_io_hazard(text) {
                        alerts.push(GuardAlert {
                            severity: Severity::High,
                            rule_id: "fragility/render-sync-io".to_string(),
                            fingerprint: String::new(),
                            title: "Blocking I/O in UI Render Loop".to_string(),
                            description: format!(
                                "Synchronous file/network I/O invoked inside UI render context: '{}'",
                                text
                            ),
                            line_no: line_num,
                            file_path: path_str.clone(),
                        });
                    }
                }
            }
        }

        alerts
    }
}

fn strip_strings_and_comments(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    let mut in_str = false;
    let mut escape = false;

    while let Some(c) = chars.next() {
        if in_str {
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_str = false;
                out.push('"');
            }
        } else if c == '"' {
            in_str = true;
            out.push('"');
        } else if c == '/' && chars.peek() == Some(&'/') {
            break;
        } else {
            out.push(c);
        }
    }
    out
}

fn is_rust_silent_error(text: &str) -> bool {
    let stripped = strip_strings_and_comments(text);
    let t = stripped.trim();
    if t.is_empty() || t.starts_with("/*") || t.starts_with('*') {
        return false;
    }

    // Flag `.ok();` expressions
    if t.ends_with(".ok();") || t.contains(".ok();") {
        return true;
    }

    // Flag `let _ = ` when not explicitly a known allowlisted idiom
    if t.starts_with("let _ = ") {
        let is_allowlisted = t.contains("writeln!")
            || t.contains("write!")
            || t.contains(".send(")
            || t.contains("Sender::")
            || t.contains("mpsc");
        return !is_allowlisted;
    }

    // Empty Err match arm: `Err(_) => {}`
    if t.starts_with("Err(_) => {}") || t.starts_with("Err(_) => ()") {
        return true;
    }

    false
}

fn is_python_silent_error(text: &str) -> bool {
    text == "except:"
        || text.starts_with("except:")
        || text == "except Exception: pass"
        || text.starts_with("except Exception:") && text.ends_with("pass")
}

fn is_js_silent_error(text: &str) -> bool {
    text.starts_with("catch") && text.ends_with("{}")
        || text.contains("catch (") && text.contains(") {}")
}

fn is_sync_io_hazard(text: &str) -> bool {
    text.contains("std::fs::read")
        || text.contains("std::fs::write")
        || text.contains("File::open")
        || text.contains("fs::read_to_string")
        || text.contains("readFileSync")
        || text.contains("writeFileSync")
        || text.contains("open(") && (text.contains("'r'") || text.contains("\"r\""))
}

impl super::Guard for FragilityGuard {
    fn name(&self) -> &'static str {
        "fragility"
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

    fn make_diff(file_name: &str, line_content: &str) -> FileDiff {
        FileDiff {
            old_path: None,
            new_path: PathBuf::from(file_name),
            change_type: ChangeType::Modified,
            additions: 1,
            deletions: 0,
            hunks: vec![DiffHunk {
                header: "@@ -1,1 +1,2 @@".to_string(),
                old_start: 1,
                old_lines: 1,
                new_start: 1,
                new_lines: 2,
                lines: vec![DiffLine {
                    line_type: LineType::Addition,
                    content: line_content.to_string(),
                    old_line_num: None,
                    new_line_num: Some(2),
                }],
                is_folded: false,
            }],
            is_folded: false,
            is_reviewed: false,
        }
    }

    #[test]
    fn test_flags_silent_ok_in_rust() {
        let diff = make_diff("src/service.rs", "stream.flush().ok();");
        let alerts = FragilityGuard::scan_file(&diff);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].rule_id, "fragility/silent-error");
    }

    #[test]
    fn test_allows_writeln_in_rust() {
        let diff = make_diff("src/service.rs", "let _ = writeln!(buffer, \"ok\");");
        let alerts = FragilityGuard::scan_file(&diff);
        assert!(alerts.is_empty());
    }

    #[test]
    fn test_flags_bare_except_in_python() {
        let diff = make_diff("api/handler.py", "except Exception: pass");
        let alerts = FragilityGuard::scan_file(&diff);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].rule_id, "fragility/silent-error");
    }

    #[test]
    fn test_flags_sync_io_in_render_loop() {
        let diff = FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/ui/view.rs"),
            change_type: ChangeType::Modified,
            additions: 2,
            deletions: 0,
            hunks: vec![DiffHunk {
                header: "@@ -1,1 +1,3 @@".to_string(),
                old_start: 1,
                old_lines: 1,
                new_start: 1,
                new_lines: 3,
                lines: vec![
                    DiffLine {
                        line_type: LineType::Context,
                        content: "pub fn render_view(frame: &mut Frame) {".to_string(),
                        old_line_num: Some(1),
                        new_line_num: Some(1),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "    let data = std::fs::read_to_string(\"config.json\");"
                            .to_string(),
                        old_line_num: None,
                        new_line_num: Some(2),
                    },
                ],
                is_folded: false,
            }],
            is_folded: false,
            is_reviewed: false,
        };
        let alerts = FragilityGuard::scan_file(&diff);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].rule_id, "fragility/render-sync-io");
    }
}
