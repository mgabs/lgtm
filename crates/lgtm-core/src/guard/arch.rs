use crate::guard::{GuardAlert, Severity};
use crate::model::{ChangeType, FileDiff, LineType};
use std::collections::HashSet;

pub struct ArchitectureGuard;

impl ArchitectureGuard {
    pub fn scan_file(file: &FileDiff) -> Vec<GuardAlert> {
        let mut alerts = Vec::new();
        let path_str = file.display_name();

        // 1. Stray File / Environment Contamination
        if file.change_type == ChangeType::Added {
            if let Some(reason) = is_stray_or_sensitive_file(&path_str) {
                alerts.push(GuardAlert {
                    severity: Severity::Critical,
                    rule_id: "arch/stray-file".to_string(),
                    fingerprint: String::new(),
                    title: "Stray / Sensitive File Leak".to_string(),
                    description: format!(
                        "Agent created sensitive or temporary file: {} ({})",
                        path_str, reason
                    ),
                    line_no: None,
                    file_path: path_str.clone(),
                });
            }
        }

        // 2. Layer Boundary & Scope Inflation rules across line additions
        for hunk in &file.hunks {
            for line in &hunk.lines {
                if line.line_type == LineType::Addition {
                    let text = line.content.trim();
                    let line_num = line.new_line_num;

                    if crate::guard::is_line_ignored(text) {
                        continue;
                    }

                    // Layer Boundary Violations
                    if let Some(violation) = check_layer_boundary_violation(&path_str, text) {
                        alerts.push(GuardAlert {
                            severity: Severity::High,
                            rule_id: "arch/layering".to_string(),
                            fingerprint: String::new(),
                            title: "Architecture Layer Boundary Violation".to_string(),
                            description: violation,
                            line_no: line_num,
                            file_path: path_str.clone(),
                        });
                    }

                    // Scope Inflation (YAGNI Creep)
                    if is_scope_inflation(text, &path_str) {
                        alerts.push(GuardAlert {
                            severity: Severity::Warn,
                            rule_id: "arch/visibility-inflation".to_string(),
                            fingerprint: String::new(),
                            title: "Visibility Scope Inflation".to_string(),
                            description:
                                "Unnecessary `pub` / `export` exposure on internal module items"
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

    pub fn scan_snapshot(files: &[FileDiff]) -> Vec<GuardAlert> {
        let mut alerts = Vec::new();

        // Blast Radius & Architectural Sprawl
        let mut touched_domains = HashSet::new();
        let mut total_files = 0;

        for file in files {
            let path_str = file.display_name();
            total_files += 1;

            if let Some(domain) = extract_top_level_domain(&path_str) {
                touched_domains.insert(domain);
            }
        }

        // If diff touches >= 4 distinct top-level modules and >= 6 files
        if touched_domains.len() >= 4 && total_files >= 6 {
            let mut domain_list: Vec<String> = touched_domains.into_iter().collect();
            domain_list.sort();
            alerts.push(GuardAlert {
                severity: Severity::High,
                rule_id: "arch/blast-radius".to_string(),
                fingerprint: String::new(),
                title: "High Blast Radius Sprawl".to_string(),
                description: format!(
                    "Diff spans {} separate architectural subtrees ({}) across {} files",
                    domain_list.len(),
                    domain_list.join(", "),
                    total_files
                ),
                line_no: None,
                file_path: "diff".to_string(),
            });
        }

        alerts
    }
}

fn extract_top_level_domain(path: &str) -> Option<String> {
    let normalized = path.trim_start_matches("./");
    let lower = normalized.to_lowercase();
    if lower.ends_with(".md")
        || lower.ends_with(".txt")
        || lower.ends_with(".lock")
        || lower == "cargo.toml"
        || lower == "package.json"
        || lower.starts_with("docs/")
        || lower.starts_with(".github/")
    {
        return None;
    }

    let parts: Vec<&str> = normalized.split('/').collect();
    if parts.len() >= 3 && parts[0] == "crates" {
        Some(format!("{}/{}", parts[0], parts[1]))
    } else if parts.len() >= 2
        && (parts[0] == "src" || parts[0] == "lib" || parts[0] == "pkg" || parts[0] == "packages")
    {
        Some(parts[1].to_string())
    } else if !parts.is_empty() {
        Some(parts[0].to_string())
    } else {
        None
    }
}

fn is_stray_or_sensitive_file(path: &str) -> Option<&'static str> {
    let p = path.to_lowercase();
    if p.ends_with(".env") || p.contains(".env.") || p.ends_with(".env.local") {
        Some("Environment / secret credentials")
    } else if p.ends_with(".pem") || p.ends_with(".key") || p.ends_with("id_rsa") {
        Some("Private cryptographic key")
    } else if p.ends_with(".ds_store") || p.ends_with("thumbs.db") {
        Some("OS garbage metadata file")
    } else if p.ends_with(".log") || p.ends_with("debug.json") || p.ends_with("dump.rdb") {
        Some("Debug / runtime log dump")
    } else if (p.starts_with("test_") || p.starts_with("temp_") || p.starts_with("tmp_"))
        && !p.contains('/')
    {
        Some("Stray root-level scratch script")
    } else {
        None
    }
}

fn check_layer_boundary_violation(file_path: &str, line_text: &str) -> Option<String> {
    let p = file_path.to_lowercase();
    let text = line_text.to_lowercase();

    let is_import = text.starts_with("import ")
        || text.starts_with("use ")
        || text.starts_with("from ")
        || text.contains("require(");

    if !is_import {
        return None;
    }

    // Rule 1: UI / Presentation layer importing Database / Persistence directly
    if (p.contains("/ui/")
        || p.contains("/views/")
        || p.contains("/frontend/")
        || p.contains("/components/"))
        && (text.contains("db::")
            || text.contains("repository")
            || text.contains("models")
            || text.contains("schema")
            || text.contains("typeorm")
            || text.contains("prisma")
            || text.contains("database"))
    {
        return Some(format!(
            "Presentation layer ('{}') directly coupling to database/persistence model ('{}')",
            file_path, line_text
        ));
    }

    // Rule 2: Domain core importing Transport / API Controllers directly
    if (p.contains("/domain/") || p.contains("/core/") || p.contains("/models/"))
        && (text.contains("controllers")
            || text.contains("routes")
            || text.contains("fastapi")
            || text.contains("express")
            || text.contains("axum")
            || text.contains("actix_web"))
    {
        return Some(format!(
            "Domain core ('{}') coupling to transport/web framework ('{}')",
            file_path, line_text
        ));
    }

    None
}

fn is_scope_inflation(line_text: &str, file_path: &str) -> bool {
    let t = line_text.trim();
    let p = file_path.to_lowercase();

    // Ignore test files or top-level lib/main
    if p.contains("test")
        || p.ends_with("lib.rs")
        || p.ends_with("index.ts")
        || p.ends_with("main.rs")
    {
        return false;
    }

    // Check for pub / export additions on struct fields or internal helpers
    (t.starts_with("pub fn ")
        || t.starts_with("pub struct ")
        || t.starts_with("export function ")
        || t.starts_with("export const "))
        && (p.contains("/internal/") || p.contains("/impl/") || p.contains("/private/"))
}

impl super::Guard for ArchitectureGuard {
    fn scan_file(&self, file: &FileDiff) -> Vec<GuardAlert> {
        Self::scan_file(file)
    }
    fn scan_snapshot(&self, files: &[FileDiff]) -> Vec<GuardAlert> {
        Self::scan_snapshot(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ChangeType, DiffHunk, DiffLine, FileDiff, LineType};
    use std::path::PathBuf;

    #[test]
    fn test_stray_file_detection() {
        let env_file = FileDiff {
            old_path: None,
            new_path: PathBuf::from(".env.production"),
            change_type: ChangeType::Added,
            additions: 1,
            deletions: 0,
            hunks: vec![],
            is_folded: false,
            is_reviewed: false,
        };

        let alerts = ArchitectureGuard::scan_file(&env_file);
        assert_eq!(alerts.len(), 1);
        assert!(alerts[0].title.contains("Stray / Sensitive File"));
    }

    #[test]
    fn test_layer_boundary_violation() {
        let ui_file = FileDiff {
            old_path: None,
            new_path: PathBuf::from("src/ui/dashboard.tsx"),
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
                    content: "import { DatabasePool } from '../db/connection';".to_string(),
                    old_line_num: None,
                    new_line_num: Some(1),
                }],
                is_folded: false,
                folded_blocks: Vec::new(),
            }],
            is_folded: false,
            is_reviewed: false,
        };

        let alerts = ArchitectureGuard::scan_file(&ui_file);
        assert_eq!(alerts.len(), 1);
        assert!(alerts[0].title.contains("Layer Boundary Violation"));
    }

    #[test]
    fn test_blast_radius_sprawl() {
        let files = vec![
            FileDiff {
                old_path: None,
                new_path: PathBuf::from("src/auth/login.rs"),
                change_type: ChangeType::Modified,
                additions: 1,
                deletions: 0,
                hunks: vec![],
                is_folded: false,
                is_reviewed: false,
            },
            FileDiff {
                old_path: None,
                new_path: PathBuf::from("src/billing/invoice.rs"),
                change_type: ChangeType::Modified,
                additions: 1,
                deletions: 0,
                hunks: vec![],
                is_folded: false,
                is_reviewed: false,
            },
            FileDiff {
                old_path: None,
                new_path: PathBuf::from("src/ui/table.rs"),
                change_type: ChangeType::Modified,
                additions: 1,
                deletions: 0,
                hunks: vec![],
                is_folded: false,
                is_reviewed: false,
            },
            FileDiff {
                old_path: None,
                new_path: PathBuf::from("src/db/migrate.rs"),
                change_type: ChangeType::Modified,
                additions: 1,
                deletions: 0,
                hunks: vec![],
                is_folded: false,
                is_reviewed: false,
            },
            FileDiff {
                old_path: None,
                new_path: PathBuf::from("src/analytics/tracker.rs"),
                change_type: ChangeType::Modified,
                additions: 1,
                deletions: 0,
                hunks: vec![],
                is_folded: false,
                is_reviewed: false,
            },
            FileDiff {
                old_path: None,
                new_path: PathBuf::from("src/infra/k8s.rs"),
                change_type: ChangeType::Modified,
                additions: 1,
                deletions: 0,
                hunks: vec![],
                is_folded: false,
                is_reviewed: false,
            },
        ];

        let alerts = ArchitectureGuard::scan_snapshot(&files);
        assert_eq!(alerts.len(), 1);
        assert!(alerts[0].title.contains("High Blast Radius Sprawl"));
    }
}
