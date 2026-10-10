pub mod deps;
pub mod duplicate;

use crate::model::{FileDiff, LineType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Warn,     // 🟡
    High,     // 🟠
    Critical, // 🔴
}

impl Severity {
    pub fn badge(&self) -> &'static str {
        match self {
            Severity::Critical => "🔴 CRITICAL",
            Severity::High => "🟠 HIGH",
            Severity::Warn => "🟡 WARN",
        }
    }

    #[allow(dead_code)]
    pub fn short_badge(&self) -> &'static str {
        match self {
            Severity::Critical => "🔴",
            Severity::High => "🟠",
            Severity::Warn => "🟡",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardAlert {
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub line_no: Option<u32>,
    pub file_path: String,
}

pub struct SecurityAndTestGuard;

impl SecurityAndTestGuard {
    pub fn scan_file(file: &FileDiff) -> Vec<GuardAlert> {
        let mut alerts = Vec::new();
        // Dependency & Duplicate Helper Guards
        alerts.extend(deps::DependencyGuard::scan_file(file));
        alerts.extend(duplicate::DuplicateHelperGuard::scan_file(file));

        let path_str = file.display_name();
        let is_test_file = is_test_path(&path_str);

        let mut additions_assert_count = 0;
        let mut deletions_assert_count = 0;

        for hunk in &file.hunks {
            for line in &hunk.lines {
                let text = line.content.trim();
                let line_num = line.new_line_num.or(line.old_line_num);

                match line.line_type {
                    LineType::Addition => {
                        // Count assertions
                        if is_test_file && is_assertion_line(text) {
                            additions_assert_count += 1;
                        }

                        // 1. Critical Injections & Exploits
                        if is_sql_injection(text) {
                            alerts.push(GuardAlert {
                                severity: Severity::Critical,
                                title: "SQL Injection Risk".to_string(),
                                description:
                                    "Raw string interpolation or concatenation in SQL query"
                                        .to_string(),
                                line_no: line_num,
                                file_path: path_str.clone(),
                            });
                        }
                        if is_command_injection(text) {
                            alerts.push(GuardAlert {
                                severity: Severity::Critical,
                                title: "Command Injection Risk".to_string(),
                                description: "Unsafe shell execution (shell=True / os.system / child_process.exec)".to_string(),
                                line_no: line_num,
                                file_path: path_str.clone(),
                            });
                        }

                        // 2. Hardcoded Secrets
                        if is_secret_token(text) {
                            alerts.push(GuardAlert {
                                severity: Severity::Critical,
                                title: "Hardcoded Secret".to_string(),
                                description: "Possible API key / high-entropy token in source code"
                                    .to_string(),
                                line_no: line_num,
                                file_path: path_str.clone(),
                            });
                        }

                        // 3. Security Deactivations
                        if is_disabled_tls(text) {
                            alerts.push(GuardAlert {
                                severity: Severity::High,
                                title: "TLS Verification Disabled".to_string(),
                                description: "Disabled TLS/SSL verification (verify=False / rejectUnauthorized: false)".to_string(),
                                line_no: line_num,
                                file_path: path_str.clone(),
                            });
                        }
                        if is_wildcard_cors(text) {
                            alerts.push(GuardAlert {
                                severity: Severity::High,
                                title: "Wildcard CORS with Credentials".to_string(),
                                description: "Permissive wildcard CORS origin configuration"
                                    .to_string(),
                                line_no: line_num,
                                file_path: path_str.clone(),
                            });
                        }
                        if is_permissive_chmod(text) {
                            alerts.push(GuardAlert {
                                severity: Severity::High,
                                title: "Permissive File Permissions (777)".to_string(),
                                description: "Excessive file permissions (chmod 777 / 0o777)"
                                    .to_string(),
                                line_no: line_num,
                                file_path: path_str.clone(),
                            });
                        }

                        // 4. Test Fraud & Bypasses
                        if is_test_file {
                            if is_test_skip(text) {
                                alerts.push(GuardAlert {
                                    severity: Severity::High,
                                    title: "Test Skipped / Ignored".to_string(),
                                    description: "Agent added @skip / #[ignore] decorator to bypass test suite".to_string(),
                                    line_no: line_num,
                                    file_path: path_str.clone(),
                                });
                            }
                            if is_tautological_assertion(text) {
                                alerts.push(GuardAlert {
                                    severity: Severity::High,
                                    title: "Tautological / Vacuous Assertion".to_string(),
                                    description:
                                        "Assertion always evaluates to true (assert True / x == x)"
                                            .to_string(),
                                    line_no: line_num,
                                    file_path: path_str.clone(),
                                });
                            }
                        }

                        // 5. Bad Practices
                        if is_swallowed_exception(text) {
                            alerts.push(GuardAlert {
                                severity: Severity::Warn,
                                title: "Swallowed Exception".to_string(),
                                description: "Empty catch/except block suppressing errors without logging/rethrow".to_string(),
                                line_no: line_num,
                                file_path: path_str.clone(),
                            });
                        }
                        if is_debug_residue(text) {
                            alerts.push(GuardAlert {
                                severity: Severity::Warn,
                                title: "Debug Residue".to_string(),
                                description: "Lingering debug statement (console.log / dbg! / print / debugger)".to_string(),
                                line_no: line_num,
                                file_path: path_str.clone(),
                            });
                        }
                    }
                    LineType::Deletion if is_test_file && is_assertion_line(text) => {
                        deletions_assert_count += 1;
                    }
                    _ => {}
                }
            }
        }

        // Check for net assert drop in test files
        if is_test_file
            && deletions_assert_count > additions_assert_count
            && deletions_assert_count > 0
        {
            alerts.push(GuardAlert {
                severity: Severity::High,
                title: "Assert Drop (Test Neutering)".to_string(),
                description: format!(
                    "Net reduction of assertions (-{} deleted, +{} added)",
                    deletions_assert_count, additions_assert_count
                ),
                line_no: None,
                file_path: path_str,
            });
        }

        alerts
    }

    pub fn scan_snapshot(files: &[FileDiff]) -> Vec<GuardAlert> {
        let mut all_alerts = Vec::new();
        let mut total_logic_lines = 0;
        let mut total_assert_additions = 0;

        for file in files {
            let alerts = Self::scan_file(file);
            all_alerts.extend(alerts);

            let path_str = file.display_name();
            let is_test = is_test_path(&path_str);

            for hunk in &file.hunks {
                for line in &hunk.lines {
                    if line.line_type == LineType::Addition {
                        let text = line.content.trim();
                        if !text.is_empty()
                            && !text.starts_with("//")
                            && !text.starts_with('#')
                            && text != "{"
                            && text != "}"
                        {
                            if is_test {
                                if is_assertion_line(text) {
                                    total_assert_additions += 1;
                                }
                            } else {
                                total_logic_lines += 1;
                            }
                        }
                    }
                }
            }
        }

        // Assertion Density Gate: alert if substantial new logic is added without any assertions
        if total_logic_lines >= 30 && total_assert_additions == 0 {
            all_alerts.push(GuardAlert {
                severity: Severity::High,
                title: "Zero Assertions for New Logic".to_string(),
                description: format!(
                    "Added {} lines of production code with 0 new test assertions",
                    total_logic_lines
                ),
                line_no: None,
                file_path: "diff".to_string(),
            });
        }

        all_alerts
    }
}

fn is_test_path(path: &str) -> bool {
    let p = path.to_lowercase();
    p.contains("test")
        || p.contains("spec")
        || p.starts_with("tests/")
        || p.ends_with("_test.rs")
        || p.ends_with(".test.ts")
        || p.ends_with(".test.js")
}

fn is_assertion_line(text: &str) -> bool {
    let t = text.to_lowercase();
    t.contains("assert")
        || t.contains("expect(")
        || t.contains("should.")
        || t.contains("t.equal")
        || t.contains("assert_eq!")
}

fn is_sql_injection(text: &str) -> bool {
    let t = text.to_lowercase();
    (t.contains("execute(") || t.contains("query(") || t.contains("raw("))
        && (t.contains("f\"")
            || t.contains("f'")
            || t.contains("${")
            || t.contains(" + ")
            || t.contains(".format("))
        && (t.contains("select ")
            || t.contains("insert ")
            || t.contains("update ")
            || t.contains("delete ")
            || t.contains("where "))
}

fn is_command_injection(text: &str) -> bool {
    let t = text.to_lowercase();
    t.contains("shell=true")
        || t.contains("os.system(")
        || t.contains("child_process.exec(")
        || t.contains("exec.command(\"sh\"")
}

fn is_secret_token(text: &str) -> bool {
    text.contains("sk-proj-")
        || text.contains("ghp_")
        || text.contains("xoxb-")
        || (text.contains("AKIA") && text.len() >= 20)
        || text.contains("BEGIN PRIVATE KEY")
        || text.contains("BEGIN RSA PRIVATE KEY")
}

fn is_disabled_tls(text: &str) -> bool {
    let t = text.to_lowercase();
    t.contains("verify=false")
        || t.contains("rejectunauthorized: false")
        || t.contains("insecureskipverify: true")
        || t.contains("--insecure")
}

fn is_wildcard_cors(text: &str) -> bool {
    let t = text.to_lowercase();
    (t.contains("cors") || t.contains("allow_origins") || t.contains("access-control-allow-origin"))
        && (t.contains("'*'") || t.contains("\"*\"") || t.contains("origin: true"))
}

fn is_permissive_chmod(text: &str) -> bool {
    text.contains("777") && (text.contains("chmod") || text.contains("0o777"))
}

fn is_test_skip(text: &str) -> bool {
    let t = text.to_lowercase();
    t.contains("@pytest.mark.skip")
        || t.contains("#[ignore]")
        || t.contains("it.skip")
        || t.contains("test.skip")
        || t.contains("describe.skip")
        || t.contains("xit(")
}

fn is_tautological_assertion(text: &str) -> bool {
    let t = text.to_lowercase();
    t.contains("assert true")
        || t.contains("assert 1 == 1")
        || t.contains("expect(true).tobe(true)")
        || t.contains("assert_eq!(true, true)")
}

fn is_swallowed_exception(text: &str) -> bool {
    let t = text.to_lowercase();
    t == "except: pass"
        || t == "except exception: pass"
        || t == "except: ... "
        || t.contains("catch (e) {}")
        || t.contains("catch (err) {}")
        || t.contains("catch {}")
}

fn is_debug_residue(text: &str) -> bool {
    let t = text.trim();
    t.starts_with("console.log(")
        || t.starts_with("dbg!(")
        || t.starts_with("print(")
        || t == "debugger;"
        || t.contains("pdb.set_trace()")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ChangeType, DiffHunk, DiffLine, FileDiff, LineType};
    use std::path::PathBuf;

    #[test]
    fn test_security_alert_detection() {
        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/api/auth.py"),
            change_type: ChangeType::Added,
            additions: 4,
            deletions: 0,
            hunks: vec![DiffHunk {
                header: "@@ -0,0 +1,4 @@".to_string(),
                old_start: 0,
                old_lines: 0,
                new_start: 1,
                new_lines: 4,
                lines: vec![
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "cursor.execute(f\"SELECT * FROM users WHERE id = {user_id}\")"
                            .to_string(),
                        old_line_num: None,
                        new_line_num: Some(1),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "api_key = 'sk-proj-9876543210abcdef9876543210'".to_string(),
                        old_line_num: None,
                        new_line_num: Some(2),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "requests.get(url, verify=False)".to_string(),
                        old_line_num: None,
                        new_line_num: Some(3),
                    },
                ],
            }],
            is_folded: false,
            is_reviewed: false,
        };

        let alerts = SecurityAndTestGuard::scan_file(&file);
        assert_eq!(alerts.len(), 3);
        assert!(alerts.iter().any(|a| a.title.contains("SQL Injection")));
        assert!(alerts.iter().any(|a| a.title.contains("Hardcoded Secret")));
        assert!(alerts.iter().any(|a| a.title.contains("TLS Verification")));
    }

    #[test]
    fn test_test_neutering_detection() {
        let test_file = FileDiff {
            old_path: Some(PathBuf::from("tests/test_auth.py")),
            new_path: PathBuf::from("tests/test_auth.py"),
            change_type: ChangeType::Modified,
            additions: 1,
            deletions: 3,
            hunks: vec![DiffHunk {
                header: "@@ -1,4 +1,2 @@".to_string(),
                old_start: 1,
                old_lines: 4,
                new_start: 1,
                new_lines: 2,
                lines: vec![
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "@pytest.mark.skip".to_string(),
                        old_line_num: None,
                        new_line_num: Some(1),
                    },
                    DiffLine {
                        line_type: LineType::Deletion,
                        content: "assert user.is_authenticated == True".to_string(),
                        old_line_num: Some(2),
                        new_line_num: None,
                    },
                    DiffLine {
                        line_type: LineType::Deletion,
                        content: "assert user.role == 'admin'".to_string(),
                        old_line_num: Some(3),
                        new_line_num: None,
                    },
                ],
            }],
            is_folded: false,
            is_reviewed: false,
        };

        let alerts = SecurityAndTestGuard::scan_file(&test_file);
        assert!(alerts.iter().any(|a| a.title.contains("Test Skipped")));
        assert!(alerts.iter().any(|a| a.title.contains("Assert Drop")));
    }

    #[test]
    fn test_all_vulnerability_patterns() {
        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("tests/audit_test.py"),
            change_type: ChangeType::Added,
            additions: 7,
            deletions: 0,
            hunks: vec![DiffHunk {
                header: "@@ -0,0 +1,7 @@".to_string(),
                old_start: 0,
                old_lines: 0,
                new_start: 1,
                new_lines: 7,
                lines: vec![
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "os.system(f'rm -rf {path}')".to_string(),
                        old_line_num: None,
                        new_line_num: Some(1),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "allow_origins = ['*']".to_string(),
                        old_line_num: None,
                        new_line_num: Some(2),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "os.chmod(path, 0o777)".to_string(),
                        old_line_num: None,
                        new_line_num: Some(3),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "assert True".to_string(),
                        old_line_num: None,
                        new_line_num: Some(4),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "except: pass".to_string(),
                        old_line_num: None,
                        new_line_num: Some(5),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "console.log(token)".to_string(),
                        old_line_num: None,
                        new_line_num: Some(6),
                    },
                ],
            }],
            is_folded: false,
            is_reviewed: false,
        };

        let alerts = SecurityAndTestGuard::scan_file(&file);
        assert!(alerts.iter().any(|a| a.title.contains("Command Injection")));
        assert!(alerts.iter().any(|a| a.title.contains("Wildcard CORS")));
        assert!(alerts
            .iter()
            .any(|a| a.title.contains("Permissive File Permissions")));
        assert!(alerts.iter().any(|a| a.title.contains("Tautological")));
        assert!(alerts
            .iter()
            .any(|a| a.title.contains("Swallowed Exception")));
        assert!(alerts.iter().any(|a| a.title.contains("Debug Residue")));

        let snapshot_alerts = SecurityAndTestGuard::scan_snapshot(&[file]);
        assert_eq!(snapshot_alerts.len(), alerts.len());
    }

    #[test]
    fn test_snapshot_assertion_density_gate() {
        let mut lines = Vec::new();
        for i in 1..=35 {
            lines.push(DiffLine {
                line_type: LineType::Addition,
                content: format!("let compute_step_{} = process_data({});", i, i),
                old_line_num: None,
                new_line_num: Some(i),
            });
        }

        let prod_file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/engine.rs"),
            change_type: ChangeType::Added,
            additions: 35,
            deletions: 0,
            hunks: vec![DiffHunk {
                header: "@@ -0,0 +1,35 @@".to_string(),
                old_start: 0,
                old_lines: 0,
                new_start: 1,
                new_lines: 35,
                lines,
            }],
            is_folded: false,
            is_reviewed: false,
        };

        let alerts = SecurityAndTestGuard::scan_snapshot(&[prod_file]);
        assert!(alerts
            .iter()
            .any(|a| a.title.contains("Zero Assertions for New Logic")));
    }

    #[test]
    fn test_severity_formatting() {
        assert_eq!(Severity::Critical.badge(), "🔴 CRITICAL");
        assert_eq!(Severity::High.badge(), "🟠 HIGH");
        assert_eq!(Severity::Warn.badge(), "🟡 WARN");

        assert_eq!(Severity::Critical.short_badge(), "🔴");
        assert_eq!(Severity::High.short_badge(), "🟠");
        assert_eq!(Severity::Warn.short_badge(), "🟡");
    }
}
