use crate::guard::{GuardAlert, Severity};
use crate::model::{FileDiff, LineType};
use std::collections::HashMap;

pub struct AdvancedSecurityGuard;

impl AdvancedSecurityGuard {
    pub fn scan_file(file: &FileDiff) -> Vec<GuardAlert> {
        let mut alerts = Vec::new();
        let path_str = file.display_name();

        for hunk in &file.hunks {
            for line in &hunk.lines {
                if line.line_type == LineType::Addition {
                    let text = line.content.trim();
                    let line_num = line.new_line_num;

                    // 1. High-Entropy Secret Detection (for un-prefixed arbitrary tokens)
                    if !is_known_secret_prefix(text) {
                        if let Some(secret) = detect_high_entropy_string(text) {
                            alerts.push(GuardAlert {
                                severity: Severity::Critical,
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
                            title: "Hardcoded Auth Fallback".to_string(),
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
    if line.starts_with("//") || line.starts_with('#') || line.starts_with("/*") {
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
        // Exclude common safe patterns (URLs, import paths, SQL statements, CSS styles)
        if trimmed.starts_with("http://")
            || trimmed.starts_with("https://")
            || trimmed.starts_with('/')
            || trimmed.starts_with("./")
            || trimmed.starts_with("../")
            || trimmed.contains(' ')
            || trimmed.contains('\n')
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
    (t.contains("path.join(")
        || t.contains("fs.read")
        || t.contains("open(")
        || t.contains("std::fs::")
        || t.contains("file.read"))
        && (t.contains("..") || t.contains("req.params") || t.contains("req.query"))
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
    let t = text.to_lowercase();
    t.contains("pickle.loads(")
        || t.contains("pickle.load(")
        || (t.contains("yaml.load(") && !t.contains("safeloader"))
        || t.contains("marshal.load(")
        || t.contains("unserialize(")
}

fn is_hardcoded_auth_fallback(text: &str) -> bool {
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
            || t.contains("default=\""))
}

fn is_known_secret_prefix(text: &str) -> bool {
    text.contains("sk-proj-")
        || text.contains("ghp_")
        || text.contains("xoxb-")
        || (text.contains("AKIA") && text.len() >= 20)
        || text.contains("BEGIN PRIVATE KEY")
        || text.contains("BEGIN RSA PRIVATE KEY")
}

fn is_weak_cryptography(text: &str) -> bool {
    let t = text.to_lowercase();
    (t.contains("createhash('md5')")
        || t.contains("createhash(\"md5\")")
        || t.contains("createhash('sha1')")
        || t.contains("createhash(\"sha1\")")
        || t.contains("hashlib.md5(")
        || t.contains("hashlib.sha1(")
        || t.contains("crypto.createcipheriv('des'"))
        && !t.contains("//")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ChangeType, DiffHunk, DiffLine, FileDiff, LineType};
    use std::path::PathBuf;

    #[test]
    fn test_entropy_calculator() {
        let low_entropy = "aaaaaaaaaaaaaaaaaaaa";
        let high_entropy = "aGv9$kL2!pQx9@mZ4#vN8*jB1%wE";
        assert!(shannon_entropy(low_entropy) < 1.0);
        assert!(shannon_entropy(high_entropy) > 4.2);
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
                        content: "const token = \"aGv9kL2pQx9mZ4vN8jB1wE9qL3zX7cR0pT5yU\";".into(),
                        old_line_num: None,
                        new_line_num: Some(1),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "fs.readFile(path.join('/uploads', req.query.file), 'utf8');"
                            .into(),
                        old_line_num: None,
                        new_line_num: Some(2),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "axios.get(req.query.target_url);".into(),
                        old_line_num: None,
                        new_line_num: Some(3),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "const data = pickle.loads(raw_input);".into(),
                        old_line_num: None,
                        new_line_num: Some(4),
                    },
                    DiffLine {
                        line_type: LineType::Addition,
                        content: "const secret = process.env.JWT_SECRET || 'default_secret';"
                            .into(),
                        old_line_num: None,
                        new_line_num: Some(5),
                    },
                ],
                is_folded: false,
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
}
