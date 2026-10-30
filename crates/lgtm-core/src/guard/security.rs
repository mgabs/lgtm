use crate::guard::{GuardAlert, Severity};
use crate::model::{FileDiff, LineType};
use std::collections::HashMap;

pub struct AdvancedSecurityGuard;

impl AdvancedSecurityGuard {
    pub fn scan_file(file: &FileDiff) -> Vec<GuardAlert> {
        let mut alerts = Vec::new();
        let path_str = file.display_name();
        let path_lower = path_str.to_lowercase();
        let is_lockfile = path_lower.ends_with(".lock")
            || path_lower.ends_with("-lock.json")
            || path_lower.ends_with("-lock.yaml")
            || path_lower.ends_with(".lockb");

        for hunk in &file.hunks {
            for line in &hunk.lines {
                if line.line_type == LineType::Addition {
                    let text = line.content.trim();
                    let line_num = line.new_line_num;

                    // Skip lines explicitly ignored
                    if crate::guard::is_line_ignored(text) {
                        continue;
                    }

                    // 1. High-Entropy Secret Detection (for un-prefixed arbitrary tokens; skip lockfiles)
                    if !is_lockfile && !is_known_secret_prefix(text) {
                        if let Some(secret) = detect_high_entropy_string(text) {
                            alerts.push(GuardAlert {
                                severity: Severity::Critical,
                                rule_id: "sec/high-entropy-secret".to_string(),
                                fingerprint: String::new(),
                                title: "High-Entropy Secret / Token".to_string(),
                                description: format!(
                                    "Detected high-entropy candidate token: '{}'",
                                    truncate_secret(&secret)
                                ),
                                line_no: line_num,
                                file_path: path_str.clone(),
                            });
                        }
                    }

                    // 2. Path Traversal
                    if is_path_traversal(text) {
                        alerts.push(GuardAlert {
                            severity: Severity::Critical,
                            rule_id: "sec/path-traversal".to_string(),
                            fingerprint: String::new(),
                            title: "Path Traversal Risk".to_string(),
                            description:
                                "Unsanitized path concatenation with directory traversal (`../`)"
                                    .to_string(),
                            line_no: line_num,
                            file_path: path_str.clone(),
                        });
                    }

                    // 3. Server-Side Request Forgery (SSRF)
                    if is_ssrf_sink(text) {
                        alerts.push(GuardAlert {
                            severity: Severity::High,
                            rule_id: "sec/ssrf".to_string(),
                            fingerprint: String::new(),
                            title: "SSRF Risk (Unvalidated HTTP Request)".to_string(),
                            description:
                                "Dynamic HTTP request with unsanitized URL / host parameter"
                                    .to_string(),
                            line_no: line_num,
                            file_path: path_str.clone(),
                        });
                    }

                    // 4. Insecure Deserialization
                    if is_insecure_deserialization(text) {
                        alerts.push(GuardAlert {
                            severity: Severity::Critical,
                            rule_id: "sec/insecure-deserialization".to_string(),
                            fingerprint: String::new(),
                            title: "Insecure Deserialization".to_string(),
                            description:
                                "Unsafe object deserialization (pickle / unsafe yaml / unserialize)"
                                    .to_string(),
                            line_no: line_num,
                            file_path: path_str.clone(),
                        });
                    }

                    // 5. Weak Crypto / Auth Fallbacks
                    if is_hardcoded_auth_fallback(text) {
                        alerts.push(GuardAlert {
                            severity: Severity::High,
                            rule_id: "sec/auth-fallback".to_string(), fingerprint: String::new(), title: "Hardcoded Auth Fallback".to_string(),
                            description:
                                "Insecure fallback secret in environment lookup (e.g. `SECRET || 'default'`)"
                                    .to_string(),
                            line_no: line_num,
                            file_path: path_str.clone(),
                        });
                    }

                    if is_weak_cryptography(text) {
                        alerts.push(GuardAlert {
                            severity: Severity::Warn,
                            rule_id: "sec/weak-crypto".to_string(),
                            fingerprint: String::new(),
                            title: "Weak Cryptographic Primitive".to_string(),
                            description:
                                "Use of broken/weak cryptographic hash (MD5 / SHA1 / DES / RC4)"
                                    .to_string(),
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

pub fn shannon_entropy(s: &str) -> f64 {
    if s.is_empty() {
        return 0.0;
    }
    let mut counts: HashMap<char, usize> = HashMap::new();
    for c in s.chars() {
        *counts.entry(c).or_insert(0) += 1;
    }
    let len = s.len() as f64;
    let mut entropy = 0.0;
    for &count in counts.values() {
        let p = count as f64 / len;
        entropy -= p * p.log2();
    }
    entropy
}

fn detect_high_entropy_string(line: &str) -> Option<String> {
    if line.starts_with("//")
        || line.starts_with('#')
        || line.starts_with("/*")
        || crate::guard::is_line_ignored(line)
    {
        return None;
    }

    // Extract quoted strings
    let mut quotes = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let mut in_quote: Option<char> = None;
    let mut current = String::new();

    for &c in &chars {
        if let Some(q) = in_quote {
            if c == q {
                quotes.push(current.clone());
                current.clear();
                in_quote = None;
            } else if c != '\\' {
                current.push(c);
            }
        } else if c == '"' || c == '\'' || c == '`' {
            in_quote = Some(c);
        }
    }

    for candidate in quotes {
        let trimmed = candidate.trim();
        if trimmed.len() < 24 {
            continue;
        }
        // Exclude common safe patterns (URLs, import paths, SQL statements, CSS styles, base64 charsets)
        if trimmed.starts_with("http://")
            || trimmed.starts_with("https://")
            || trimmed.starts_with('/')
            || trimmed.starts_with("./")
            || trimmed.starts_with("../")
            || trimmed.contains(' ')
            || trimmed.contains('\n')
            || trimmed.contains("ABCDEFGHIJKLMNOPQRSTUVWXYZ")
            || trimmed.contains("${")
            || trimmed.contains("api-version=")
            || trimmed.ends_with(".tar.gz")
            || trimmed.ends_with(".zip")
            || trimmed.ends_with(".tgz")
        {
            continue;
        }

        let entropy = shannon_entropy(trimmed);
        // Base64-like high entropy > 4.3 or Hex-like > 3.2
        let is_hex = trimmed.chars().all(|c| c.is_ascii_hexdigit());
        if (is_hex && trimmed.len() >= 32 && entropy >= 3.2) || (!is_hex && entropy >= 4.3) {
            return Some(trimmed.to_string());
        }
    }

    None
}

fn truncate_secret(s: &str) -> String {
    if s.len() <= 12 {
        s.to_string()
    } else {
        format!("{}...{}", &s[..6], &s[s.len() - 4..])
    }
}

fn is_path_traversal(text: &str) -> bool {
    let t = text.to_lowercase();
    let has_traversal_token = t.contains("../")
        || t.contains("..\\")
        || t.contains("\"..\"")
        || t.contains("'..'")
        || t.contains("\"../")
        || t.contains("req.params")
        || t.contains("req.query");
    (t.contains("path.join(")
        || t.contains("fs.read")
        || t.contains("open(")
        || t.contains("std::fs::")
        || t.contains("file.read"))
        && has_traversal_token
}

fn is_ssrf_sink(text: &str) -> bool {
    let t = text.to_lowercase();
    (t.contains("fetch(")
        || t.contains("axios.get(")
        || t.contains("axios.post(")
        || t.contains("requests.get(")
        || t.contains("requests.post(")
        || t.contains("reqwest::get(")
        || t.contains("http.get("))
        && (t.contains("req.query")
            || t.contains("req.body")
            || t.contains("user_url")
            || t.contains("target_url")
            || t.contains("params[")
            || t.contains("request.args"))
}

fn is_insecure_deserialization(text: &str) -> bool {
    // lgtm:ignore
    let t = text.to_lowercase();
    t.contains("pickle.loads(") // lgtm:ignore
        || t.contains("pickle.load(") // lgtm:ignore
        || (t.contains("yaml.load(") && !t.contains("safeloader")) // lgtm:ignore
        || t.contains("marshal.load(") // lgtm:ignore
        || t.contains("unserialize(") // lgtm:ignore
}

fn is_hardcoded_auth_fallback(text: &str) -> bool {
    // lgtm:ignore
    let t = text.to_lowercase();
    (t.contains("jwt_secret")
        || t.contains("secret_key")
        || t.contains("api_key")
        || t.contains("auth_token"))
        && (t.contains("|| \"")
            || t.contains("|| '")
            || t.contains("?? \"")
            || t.contains("?? '")
            || t.contains("unwrap_or(\"")
            || t.contains("default=\"")) // lgtm:ignore
}

fn is_known_secret_prefix(text: &str) -> bool {
    // lgtm:ignore
    text.contains("sk-proj-") // lgtm:ignore
        || text.contains("ghp_") // lgtm:ignore
        || text.contains("xoxb-") // lgtm:ignore
        || (text.contains("AKIA") && text.len() >= 20) // lgtm:ignore
        || text.contains("BEGIN PRIVATE KEY") // lgtm:ignore
        || text.contains("BEGIN RSA PRIVATE KEY") // lgtm:ignore
}

fn is_weak_cryptography(text: &str) -> bool {
    // lgtm:ignore
    let t = text.to_lowercase();
    (t.contains("createhash('md5')") // lgtm:ignore
        || t.contains("createhash(\"md5\")") // lgtm:ignore
        || t.contains("createhash('sha1')") // lgtm:ignore
        || t.contains("createhash(\"sha1\")") // lgtm:ignore
        || t.contains("hashlib.md5(") // lgtm:ignore
        || t.contains("hashlib.sha1(") // lgtm:ignore
        || t.contains("crypto.createcipheriv('des'")) // lgtm:ignore
        && !t.contains("//")
}

impl super::Guard for AdvancedSecurityGuard {
    fn scan_file(&self, file: &FileDiff) -> Vec<GuardAlert> {
        Self::scan_file(file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ChangeType, DiffHunk, DiffLine, FileDiff, LineType};
    use std::path::PathBuf;

    #[test]
    fn test_entropy_calculator() {
        let low_entropy = "aaaaaaaaaaaaaaaaaaaa";
        let high_entropy = format!("{}{}{}", "aGv9$kL2!", "pQx9@mZ4#", "vN8*jB1%wE"); // lgtm:ignore
        assert!(shannon_entropy(low_entropy) < 1.0);
        assert!(shannon_entropy(&high_entropy) > 4.2);
    }

    #[test]
    fn test_advanced_security_detections() {
        let file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/server.ts"),
            change_type: ChangeType::Added,
            additions: 5,
            deletions: 0,
            hunks: vec![DiffHunk {
                header: "@@ -0,0 +1,5 @@".to_string(),
                old_start: 0,
                old_lines: 0,
                new_start: 1,
                new_lines: 5,
                lines: vec![
                    DiffLine {
                        line_type: LineType::Addition,
                        content: format!(
                            "const token = \"{}{}\";",
                            "aGv9kL2pQx9mZ4vN8", "jB1wE9qL3zX7cR0pT5yU"
                        ), // lgtm:ignore
                        old_line_num: None,
                        new_line_num: Some(1),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: format!(
                            "{}(\"{}/uploads\", req.query.file), 'utf8');",
                            "fs.readFile(path.join(", ".."
                        ), // lgtm:ignore
                        old_line_num: None,
                        new_line_num: Some(2),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "axios.get(req.query.target_url);".into(), // lgtm:ignore
                        old_line_num: None,
                        new_line_num: Some(3),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: format!("const data = {}(raw_input);", "pickle.loads"), // lgtm:ignore
                        old_line_num: None,
                        new_line_num: Some(4),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: format!(
                            "const secret = process.env.JWT_SECRET || '{}_secret';",
                            "default"
                        ), // lgtm:ignore
                        old_line_num: None,
                        new_line_num: Some(5),
                    },
                ],
                is_folded: false,
                folded_blocks: Vec::new(),
            }],
            is_folded: false,
            is_reviewed: false,
        };

        let alerts = AdvancedSecurityGuard::scan_file(&file);
        assert_eq!(alerts.len(), 5);
        assert!(alerts
            .iter()
            .any(|a| a.title.contains("High-Entropy Secret")));
        assert!(alerts.iter().any(|a| a.title.contains("Path Traversal")));
        assert!(alerts.iter().any(|a| a.title.contains("SSRF Risk")));
        assert!(alerts
            .iter()
            .any(|a| a.title.contains("Insecure Deserialization")));
        assert!(alerts
            .iter()
            .any(|a| a.title.contains("Hardcoded Auth Fallback")));
    }

    fn file_with(path: &str, lines: &[String]) -> FileDiff {
        let lines: Vec<DiffLine> = lines
            .iter()
            .enumerate()
            .map(|(i, l)| DiffLine {
                line_type: LineType::Addition,
                content: l.clone(),
                old_line_num: None,
                new_line_num: Some(i as u32 + 1),
            })
            .collect();
        FileDiff {
            old_path: None,
            new_path: PathBuf::from(path),
            change_type: ChangeType::Added,
            additions: lines.len(),
            deletions: 0,
            hunks: vec![DiffHunk {
                header: "@@".into(),
                old_start: 0,
                old_lines: 0,
                new_start: 1,
                new_lines: lines.len() as u32,
                lines,
                is_folded: false,
                folded_blocks: Vec::new(),
            }],
            is_folded: false,
            is_reviewed: false,
        }
    }

    #[test]
    fn weak_crypto_flagged_unless_commented_or_ignored() {
        let weak = ["h = hashlib.", "md5(data)"].concat();
        let commented = format!("{weak} // legacy");
        let ignored = format!("{weak} # lgtm:ignore");
        let alerts = AdvancedSecurityGuard::scan_file(&file_with(
            "a.py",
            &[weak.clone(), commented, ignored],
        ));
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].rule_id, "sec/weak-crypto");
        assert_eq!(alerts[0].line_no, Some(1));
        assert!(alerts[0].description.contains("MD5"));
    }

    #[test]
    fn lockfiles_skip_entropy_and_truncation_shortens_long_secrets() {
        let token = format!("\"{}{}\"", "aGv9kL2pQx9mZ4vN8", "jB1wE9qL3zX7cR0pT5yU");
        assert!(AdvancedSecurityGuard::scan_file(&file_with(
            "Cargo.lock",
            std::slice::from_ref(&token)
        ))
        .is_empty());
        let alerts = AdvancedSecurityGuard::scan_file(&file_with("a.js", &[token]));
        assert_eq!(alerts.len(), 1);
        assert!(alerts[0].description.contains("aGv9kL...T5yU"));
        assert_eq!(truncate_secret("short"), "short");
        assert_eq!(shannon_entropy(""), 0.0);
    }
}
