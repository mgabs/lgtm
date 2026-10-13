use crate::guard::{GuardAlert, SecurityAndTestGuard, Severity};
use crate::model::{ChangeType, DiffHunk, DiffLine, FileDiff, LineType};
use anyhow::Result;
use std::path::{Path, PathBuf};

pub struct FullRepoScanner;

impl FullRepoScanner {
    /// Full scan with config and baseline applied: only new findings are returned.
    pub fn scan_repository(
        root_dir: &Path,
        cfg: &crate::config::LgtmConfig,
    ) -> Result<(Vec<FileDiff>, Vec<GuardAlert>)> {
        let (files, found) = Self::scan_with_baseline(root_dir, cfg)?;
        Ok((files, found.active))
    }

    /// Like `scan_repository` but keeps existing/stale baseline partitions.
    pub fn scan_with_baseline(
        root_dir: &Path,
        cfg: &crate::config::LgtmConfig,
    ) -> Result<(Vec<FileDiff>, crate::findings::Findings)> {
        let (files, alerts) = Self::scan_unbaselined(root_dir, cfg)?;
        let found = crate::findings::Findings {
            active: alerts,
            ..Default::default()
        };
        Ok((files, crate::baseline::apply(root_dir, cfg, found, true)?))
    }

    /// Config pipeline only; this is what `baseline update` records.
    pub fn scan_unbaselined(
        root_dir: &Path,
        cfg: &crate::config::LgtmConfig,
    ) -> Result<(Vec<FileDiff>, Vec<GuardAlert>)> {
        let mut files = Vec::new();
        let mut all_alerts = Vec::new();

        let source_files = collect_source_files(root_dir);

        for path in source_files {
            if let Ok(content) = std::fs::read_to_string(&path) {
                let rel_path = path.strip_prefix(root_dir).unwrap_or(&path).to_path_buf();
                let mut in_test_block = false;
                let lines: Vec<DiffLine> = content
                    .lines()
                    .enumerate()
                    .map(|(idx, line)| {
                        let trimmed = line.trim();
                        if trimmed.contains("#[cfg(test)]") {
                            in_test_block = true;
                        }
                        let content_str = if in_test_block && !trimmed.contains("lgtm:ignore") {
                            format!("{} // lgtm:ignore", line)
                        } else {
                            line.to_string()
                        };
                        DiffLine {
                            line_type: LineType::Addition,
                            content: content_str,
                            old_line_num: None,
                            new_line_num: Some((idx + 1) as u32),
                        }
                    })
                    .collect();

                let hunk = DiffHunk {
                    header: format!("@@ -0,0 +1,{} @@", lines.len()),
                    old_start: 0,
                    old_lines: 0,
                    new_start: 1,
                    new_lines: lines.len() as u32,
                    lines,
                    is_folded: false,
                };

                let file_diff = FileDiff {
                    old_path: None,
                    new_path: rel_path,
                    change_type: ChangeType::Added,
                    additions: content.lines().count(),
                    deletions: 0,
                    hunks: vec![hunk],
                    is_folded: false,
                    is_reviewed: false,
                };

                let alerts = SecurityAndTestGuard::scan_file(&file_diff);
                all_alerts.extend(alerts);
                files.push(file_diff);
            }
        }

        crate::guard::assign_fingerprints(&mut all_alerts, &files);
        let mut all_alerts = crate::findings::apply(all_alerts, &files, cfg).active;
        // Deduplicate alerts
        all_alerts.sort_by(|a, b| {
            b.severity
                .cmp(&a.severity)
                .then_with(|| a.file_path.cmp(&b.file_path))
                .then_with(|| a.line_no.cmp(&b.line_no))
        });
        all_alerts.dedup();

        Ok((files, all_alerts))
    }

    pub fn generate_markdown_report(
        root_dir: &Path,
        files: &[FileDiff],
        alerts: &[GuardAlert],
    ) -> String {
        let mut out = String::new();

        let critical_count = alerts
            .iter()
            .filter(|a| a.severity == Severity::Critical)
            .count();
        let high_count = alerts
            .iter()
            .filter(|a| a.severity == Severity::High)
            .count();
        let warn_count = alerts
            .iter()
            .filter(|a| a.severity == Severity::Warn)
            .count();

        let grade = if critical_count > 0 {
            "F (Critical Risks Detected)"
        } else if high_count > 2 {
            "C (High Severity Issues)"
        } else if high_count > 0 || warn_count > 5 {
            "B (Acceptable with Warnings)"
        } else {
            "A (Clean & Secure)"
        };

        let total_lines: usize = files.iter().map(|f| f.additions).sum();

        out.push_str("# 🛡️ LGTM Codebase Security & Architecture Audit Report\n\n");
        out.push_str(&format!(
            "* **Target Repository:** `{}`\n",
            root_dir.display()
        ));
        out.push_str(&format!(
            "* **Scope:** {} source files ({} total lines of code)\n",
            files.len(),
            total_lines
        ));
        out.push_str(&format!("* **Health Grade:** **{}**\n\n", grade));

        out.push_str("## 📊 Executive Summary\n\n");
        out.push_str("| Metric | Count | Status |\n");
        out.push_str("| :--- | :--- | :--- |\n");
        out.push_str(&format!(
            "| 🔴 **Critical Vulnerabilities** | **{}** | {} |\n",
            critical_count,
            if critical_count == 0 {
                "✅ Clean"
            } else {
                "🚨 Immediate Fix Required"
            }
        ));
        out.push_str(&format!(
            "| 🟠 **High Risks & Architectural Leaks** | **{}** | {} |\n",
            high_count,
            if high_count == 0 {
                "✅ Clean"
            } else {
                "⚠️ Needs Review"
            }
        ));
        out.push_str(&format!(
            "| 🟡 **Warnings & Code Smells** | **{}** | ℹ️ Tracked |\n\n",
            warn_count
        ));

        if alerts.is_empty() {
            out.push_str("> ✨ **Repository is Clean!** No security vulnerabilities, test neutering, hardcoded secrets, or architectural boundary violations detected.\n\n");
        } else {
            out.push_str("## 🔍 Detailed Findings\n\n");
            out.push_str("| Severity | Location | Category | Details |\n");
            out.push_str("| :---: | :--- | :--- | :--- |\n");

            for alert in alerts {
                let loc = match alert.line_no {
                    Some(lno) => format!("`{}:{}`", alert.file_path, lno),
                    None => format!("`{}`", alert.file_path),
                };
                out.push_str(&format!(
                    "| {} | {} | **{}** | {} |\n",
                    alert.severity.badge(),
                    loc,
                    alert.title,
                    alert.description
                ));
            }
            out.push('\n');

            out.push_str("## 🤖 Agent Remediation Directives\n\n");
            out.push_str("Copy and paste the block below into your coding agent (Claude Code, Cursor, Aider) to fix all findings:\n\n");
            out.push_str("```markdown\n");
            out.push_str("# Directives for Codebase Security & Architecture Fixes\n\n");
            for (idx, alert) in alerts.iter().enumerate() {
                let loc = match alert.line_no {
                    Some(lno) => format!("{}:{}", alert.file_path, lno),
                    None => alert.file_path.clone(),
                };
                out.push_str(&format!(
                    "{}. [{}] **{}**: {}\n",
                    idx + 1,
                    loc,
                    alert.title,
                    alert.description
                ));
            }
            out.push_str("\nRefactor each issue carefully, ensuring test assertions remain strong and secrets are externalized to environment variables.\n");
            out.push_str("```\n\n");
        }

        out.push_str("---\n*Generated by [LGTM](https://github.com/mgabs/lgtm) Real-Time Diff Reviewer & Agent Steering Engine.*\n");
        out
    }
}

fn collect_source_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_files_recursive(dir, &mut files);
    files
}

fn collect_files_recursive(dir: &Path, acc: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden directories and build artifacts
            if name.starts_with('.') && name != ".env" {
                continue;
            }
            if name == "target"
                || name == "node_modules"
                || name == "dist"
                || name == "build"
                || name == "vendor"
                || name == "tmp"
                || name == "fixtures"
                || name == "testdata"
            {
                continue;
            }

            if path.is_dir() {
                collect_files_recursive(&path, acc);
            } else if path.is_file() && is_source_file(&name) {
                acc.push(path);
            }
        }
    }
}

fn is_source_file(name: &str) -> bool {
    name.ends_with(".rs")
        || name.ends_with(".ts")
        || name.ends_with(".tsx")
        || name.ends_with(".js")
        || name.ends_with(".jsx")
        || name.ends_with(".py")
        || name.ends_with(".go")
        || name.ends_with(".java")
        || name.ends_with(".kt")
        || name.ends_with(".c")
        || name.ends_with(".cpp")
        || name.ends_with(".h")
        || name.ends_with(".toml")
        || name.ends_with(".json")
        || name.ends_with(".yaml")
        || name.ends_with(".yml")
        || name.starts_with(".env")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_source_file() {
        assert!(is_source_file("main.rs"));
        assert!(is_source_file("server.ts"));
        assert!(is_source_file(".env.production"));
        assert!(!is_source_file("image.png"));
        assert!(!is_source_file("binary.exe"));
    }
}
