pub mod api_drift;
pub mod arch;
pub mod complexity;
pub mod dead_code;
pub mod deps;
pub mod duplicate;
pub mod security;
pub mod symbols;

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
    pub rule_id: String,
    pub fingerprint: String,
    pub severity: Severity,
    pub title: String,
    pub description: String,
    pub line_no: Option<u32>,
    pub file_path: String,
}

pub trait Guard {
    #[allow(dead_code)] // consumed by B1 config (rule enable/disable)
    fn name(&self) -> &'static str;
    fn scan_file(&self, _file: &FileDiff) -> Vec<GuardAlert> {
        Vec::new()
    }
    fn scan_snapshot(&self, _files: &[FileDiff]) -> Vec<GuardAlert> {
        Vec::new()
    }
}

pub fn registry() -> &'static [&'static dyn Guard] {
    &[
        &deps::DependencyGuard,
        &duplicate::DuplicateHelperGuard,
        &security::AdvancedSecurityGuard,
        &arch::ArchitectureGuard,
        &complexity::ComplexityGuard,
        &symbols::SymbolGuard,
        &api_drift::ApiDriftGuard,
        &dead_code::DeadCodeGuard,
    ]
}

pub struct SecurityAndTestGuard;

impl SecurityAndTestGuard {
    pub fn scan_file(file: &FileDiff) -> Vec<GuardAlert> {
        let mut alerts = Vec::new();
        for g in registry() {
            alerts.extend(g.scan_file(file));
        }

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
                        // Skip lines explicitly ignored by user or test directives
                        if is_line_ignored(text) {
                            continue;
                        }

                        // Count assertions
                        if is_test_file && is_assertion_line(text) {
                            additions_assert_count += 1;
                        }

                        // 1. Critical Injections & Exploits
                        if is_sql_injection(text) {
                            alerts.push(GuardAlert {
                                severity: Severity::Critical,
                                rule_id: "sec/sql-injection".to_string(),
                                fingerprint: String::new(),
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
                                rule_id: "sec/command-injection".to_string(), fingerprint: String::new(), title: "Command Injection Risk".to_string(),
                                description: "Unsafe shell execution (shell=True / os.system / child_process.exec)".to_string(), // lgtm:ignore
                                line_no: line_num,
                                file_path: path_str.clone(),
                            });
                        }

                        // 2. Hardcoded Secrets
                        if is_secret_token(text) {
                            alerts.push(GuardAlert {
                                severity: Severity::Critical,
                                rule_id: "sec/hardcoded-secret".to_string(),
                                fingerprint: String::new(),
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
                                rule_id: "sec/tls-bypass".to_string(), fingerprint: String::new(), title: "TLS Verification Disabled".to_string(),
                                description: "Disabled TLS/SSL verification (verify=False / rejectUnauthorized: false)".to_string(), // lgtm:ignore
                                line_no: line_num,
                                file_path: path_str.clone(),
                            });
                        }
                        if is_wildcard_cors(text) {
                            alerts.push(GuardAlert {
                                severity: Severity::High,
                                rule_id: "sec/wildcard-cors".to_string(),
                                fingerprint: String::new(),
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
                                rule_id: "sec/permissive-chmod".to_string(),
                                fingerprint: String::new(),
                                title: "Permissive File Permissions (777)".to_string(),
                                description: "Excessive file permissions (chmod 777 / 0o777)" // lgtm:ignore
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
                                    rule_id: "test/skipped".to_string(), fingerprint: String::new(), title: "Test Skipped / Ignored".to_string(),
                                    description: "Agent added @skip / #[ignore] decorator to bypass test suite".to_string(),
                                    line_no: line_num,
                                    file_path: path_str.clone(),
                                });
                            }
                            if is_tautological_assertion(text) {
                                alerts.push(GuardAlert {
                                    severity: Severity::High,
                                    rule_id: "test/tautological-assert".to_string(),
                                    fingerprint: String::new(),
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
                                rule_id: "test/swallowed-exception".to_string(), fingerprint: String::new(), title: "Swallowed Exception".to_string(),
                                description: "Empty catch/except block suppressing errors without logging/rethrow".to_string(),
                                line_no: line_num,
                                file_path: path_str.clone(),
                            });
                        }
                        if is_debug_residue(text) {
                            alerts.push(GuardAlert {
                                severity: Severity::Warn,
                                rule_id: "quality/debug-residue".to_string(), fingerprint: String::new(), title: "Debug Residue".to_string(),
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
                rule_id: "test/neutered-assert".to_string(),
                fingerprint: String::new(),
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
                rule_id: "test/zero-assertions".to_string(),
                fingerprint: String::new(),
                title: "Zero Assertions for New Logic".to_string(),
                description: format!(
                    "Added {} lines of production code with 0 new test assertions",
                    total_logic_lines
                ),
                line_no: None,
                file_path: "diff".to_string(),
            });
        }

        // Snapshot-level guards (blast radius, dead code)
        for g in registry() {
            all_alerts.extend(g.scan_snapshot(files));
        }

        assign_fingerprints(&mut all_alerts, files);
        all_alerts
    }
}

fn fnv1a(s: &str) -> u64 {
    s.bytes().fold(0xcbf29ce484222325, |h, b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    })
}

/// Stable id from rule, file, whitespace-normalized flagged line and occurrence index (no line number).
pub fn assign_fingerprints(alerts: &mut [GuardAlert], files: &[FileDiff]) {
    let mut seen: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for a in alerts.iter_mut() {
        let text = files
            .iter()
            .filter(|f| f.display_name() == a.file_path)
            .flat_map(|f| f.hunks.iter().flat_map(|h| h.lines.iter()))
            .find(|l| a.line_no.is_some() && l.new_line_num == a.line_no)
            .map(|l| l.content.as_str())
            .unwrap_or(&a.title);
        let norm = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let key = format!("{}\0{}\0{}", a.rule_id, a.file_path, norm);
        let n = seen.entry(key.clone()).and_modify(|c| *c += 1).or_insert(0);
        a.fingerprint = format!("{:016x}", fnv1a(&format!("{key}\0{n}")));
    }
}

pub fn is_line_ignored(text: &str) -> bool {
    let t = text.trim();
    t.contains("lgtm:ignore")
        || t.contains("lgtm-ignore")
        || t.contains("nolint:lgtm")
        || t.contains("lgtm[ignore]")
        || t.contains("lgtm_ignore")
}

pub fn is_test_path(path: &str) -> bool {
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
    // lgtm:ignore
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
    // lgtm:ignore
    let t = text.to_lowercase();
    t.contains("shell=true") // lgtm:ignore
        || t.contains("os.system(") // lgtm:ignore
        || t.contains("child_process.exec(") // lgtm:ignore
        || t.contains("exec.command(\"sh\"") // lgtm:ignore
}

fn is_secret_token(text: &str) -> bool {
    // lgtm:ignore
    text.contains("sk-proj-") // lgtm:ignore
        || text.contains("ghp_") // lgtm:ignore
        || text.contains("xoxb-") // lgtm:ignore
        || (text.contains("AKIA") && text.len() >= 20) // lgtm:ignore
        || text.contains("BEGIN PRIVATE KEY") // lgtm:ignore
        || text.contains("BEGIN RSA PRIVATE KEY") // lgtm:ignore
}

fn is_disabled_tls(text: &str) -> bool {
    // lgtm:ignore
    let t = text.to_lowercase();
    t.contains("verify=false") // lgtm:ignore
        || t.contains("rejectunauthorized: false") // lgtm:ignore
        || t.contains("insecureskipverify: true") // lgtm:ignore
        || t.contains("--insecure") // lgtm:ignore
}

fn is_wildcard_cors(text: &str) -> bool {
    // lgtm:ignore
    let t = text.to_lowercase();
    (t.contains("cors") || t.contains("allow_origins") || t.contains("access-control-allow-origin"))
        && (t.contains("'*'") || t.contains("\"*\"") || t.contains("origin: true"))
    // lgtm:ignore
}

fn is_permissive_chmod(text: &str) -> bool {
    // lgtm:ignore
    text.contains("777") && (text.contains("chmod") || text.contains("0o777")) // lgtm:ignore
}

fn is_test_skip(text: &str) -> bool {
    // lgtm:ignore
    let t = text.to_lowercase();
    t.contains("@pytest.mark.skip") // lgtm:ignore
        || t.contains("#[ignore]") // lgtm:ignore
        || t.contains("it.skip") // lgtm:ignore
        || t.contains("test.skip") // lgtm:ignore
        || t.contains("describe.skip") // lgtm:ignore
        || t.contains("xit(") // lgtm:ignore
}

fn is_tautological_assertion(text: &str) -> bool {
    // lgtm:ignore
    let t = text.to_lowercase();
    t.contains("assert true") // lgtm:ignore
        || t.contains("assert 1 == 1") // lgtm:ignore
        || t.contains("expect(true).tobe(true)") // lgtm:ignore
        || t.contains("assert_eq!(true, true)") // lgtm:ignore
}

fn is_swallowed_exception(text: &str) -> bool {
    // lgtm:ignore
    let t = text.to_lowercase();
    t == "except: pass" // lgtm:ignore
        || t == "except exception: pass" // lgtm:ignore
        || t == "except: ... " // lgtm:ignore
        || t.contains("catch (e) {}") // lgtm:ignore
        || t.contains("catch (err) {}") // lgtm:ignore
        || t.contains("catch {}") // lgtm:ignore
}

fn is_debug_residue(text: &str) -> bool {
    // lgtm:ignore
    let t = text.trim();
    t.starts_with("console.log(") // lgtm:ignore
        || t.starts_with("dbg!(") // lgtm:ignore
        || t.starts_with("print(") // lgtm:ignore
        || t == "debugger;" // lgtm:ignore
        || t.contains("pdb.set_trace()") // lgtm:ignore
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ChangeType, DiffHunk, DiffLine, FileDiff, LineType};
    use std::path::PathBuf;

    #[test]
    fn registry_order_is_stable() {
        let names: Vec<&str> = registry().iter().map(|g| g.name()).collect();
        assert_eq!(
            names,
            [
                "deps",
                "duplicate",
                "security",
                "arch",
                "complexity",
                "symbols",
                "api_drift",
                "dead_code"
            ]
        );
    }

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
                        content: format!(
                            "api_key = '{}{}'",
                            "sk-proj-", "9876543210abcdef9876543210"
                        ), // lgtm:ignore
                        old_line_num: None,
                        new_line_num: Some(2),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: format!("requests.get(url, {}{})", "verify=", "False"), // lgtm:ignore
                        old_line_num: None,
                        new_line_num: Some(3),
                    },
                ],
                is_folded: false,
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
                        content: format!("@pytest.mark.{}", "skip"), // lgtm:ignore
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
                is_folded: false,
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
                        content: format!("{}(f'rm -rf {{path}}')", "os.system"), // lgtm:ignore
                        old_line_num: None,
                        new_line_num: Some(1),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: format!("allow_origins = ['{}']", "*"), // lgtm:ignore
                        old_line_num: None,
                        new_line_num: Some(2),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: format!("os.chmod(path, {})", "0o777"), // lgtm:ignore
                        old_line_num: None,
                        new_line_num: Some(3),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: format!("assert {}", "True"), // lgtm:ignore
                        old_line_num: None,
                        new_line_num: Some(4),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: format!("except: {}", "pass"), // lgtm:ignore
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
                is_folded: false,
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
                is_folded: false,
            }],
            is_folded: false,
            is_reviewed: false,
        };

        let alerts = SecurityAndTestGuard::scan_snapshot(&[prod_file]);
        assert!(alerts
            .iter()
            .any(|a| a.title.contains("Zero Assertions for New Logic")));
    }

    fn tls_file(lines: &[(&str, u32)]) -> FileDiff {
        FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/net.py"),
            change_type: ChangeType::Added,
            hunks: vec![DiffHunk {
                lines: lines
                    .iter()
                    .map(|(c, n)| DiffLine {
                        line_type: LineType::Addition,
                        content: format!("{}{}", c, ["verify=", "False"].concat()), // lgtm:ignore
                        old_line_num: None,
                        new_line_num: Some(*n),
                    })
                    .collect(),
                ..Default::default()
            }],
            additions: 0,
            deletions: 0,
            is_folded: false,
            is_reviewed: false,
        }
    }

    #[test]
    fn test_fingerprints_stable_across_moves_and_distinct_for_duplicates() {
        let a = SecurityAndTestGuard::scan_snapshot(&[tls_file(&[("requests.get(u, ", 3)])]);
        let b = SecurityAndTestGuard::scan_snapshot(&[tls_file(&[("requests.get(u,   ", 40)])]);
        assert_eq!(a[0].rule_id, "sec/tls-bypass");
        assert_eq!(a[0].fingerprint.len(), 16);
        assert_eq!(a[0].fingerprint, b[0].fingerprint);

        let d = SecurityAndTestGuard::scan_snapshot(&[tls_file(&[
            ("requests.get(u, ", 3),
            ("requests.get(u, ", 4),
        ])]);
        assert_eq!(d.len(), 2);
        assert_ne!(d[0].fingerprint, d[1].fingerprint);

        let e = SecurityAndTestGuard::scan_snapshot(&[tls_file(&[("requests.post(u, ", 3)])]);
        assert_ne!(a[0].fingerprint, e[0].fingerprint);
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
