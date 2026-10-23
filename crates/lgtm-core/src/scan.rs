use crate::guard::{GuardAlert, SecurityAndTestGuard, Severity};
use crate::model::{ChangeType, DiffHunk, DiffLine, FileDiff, LineType};
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

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
        Self::scan_with_progress(root_dir, cfg, &|_, _| true)
    }

    /// Reports processed source files. Returning false cancels remaining work.
    /// The observer may run on multiple scanner workers and must be thread-safe.
    pub fn scan_with_progress(
        root_dir: &Path,
        cfg: &crate::config::LgtmConfig,
        progress: &(dyn Fn(usize, usize) -> bool + Sync),
    ) -> Result<(Vec<FileDiff>, crate::findings::Findings)> {
        let (files, alerts) = Self::scan_unbaselined_progress(root_dir, cfg, progress)?;
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
        Self::scan_unbaselined_progress(root_dir, cfg, &|_, _| true)
    }

    fn scan_unbaselined_progress(
        root_dir: &Path,
        cfg: &crate::config::LgtmConfig,
        progress: &(dyn Fn(usize, usize) -> bool + Sync),
    ) -> Result<(Vec<FileDiff>, Vec<GuardAlert>)> {
        let mut source_files = collect_source_files(root_dir);
        source_files.sort();
        source_files.dedup();

        anyhow::ensure!(progress(0, source_files.len()), "audit cancelled");
        if source_files.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }

        let num_threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .min(16);

        let completed = AtomicUsize::new(0);
        let cancelled = AtomicBool::new(false);
        let completed = &completed;
        let cancelled = &cancelled;
        let total = source_files.len();
        let batch_size = source_files.len().div_ceil(num_threads).max(1);
        let batches = source_files.chunks(batch_size); // lgtm:ignore
        let results: Vec<(Vec<FileDiff>, Vec<GuardAlert>)> = std::thread::scope(|s| {
            let mut handles = Vec::new();
            for batch in batches {
                handles.push(s.spawn(move || {
                    let mut thread_files = Vec::with_capacity(batch.len());
                    let mut thread_alerts = Vec::new();
                    for path in batch {
                        if cancelled.load(Ordering::Relaxed)
                            || !progress(completed.load(Ordering::Relaxed), total)
                        {
                            cancelled.store(true, Ordering::Relaxed);
                            break;
                        }
                        if let Some((diff, alerts)) = process_single_file(root_dir, path) {
                            thread_alerts.extend(alerts);
                            thread_files.push(diff);
                        }
                        let count = completed.fetch_add(1, Ordering::Relaxed) + 1;
                        if !progress(count, total) {
                            cancelled.store(true, Ordering::Relaxed);
                            break;
                        }
                    }
                    (thread_files, thread_alerts)
                }));
            }
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });

        anyhow::ensure!(!cancelled.load(Ordering::Relaxed), "audit cancelled");
        let mut files = Vec::with_capacity(source_files.len());
        let mut all_alerts = Vec::new();
        for (thread_files, thread_alerts) in results {
            files.extend(thread_files);
            all_alerts.extend(thread_alerts);
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

fn process_single_file(root_dir: &Path, path: &Path) -> Option<(FileDiff, Vec<GuardAlert>)> {
    let content = std::fs::read_to_string(path).ok()?;
    let rel_path = path.strip_prefix(root_dir).unwrap_or(path).to_path_buf();
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
        folded_blocks: Vec::new(),
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
    Some((file_diff, alerts))
}

fn collect_source_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    // Prefer git index & status discovery when available: fast and respects .gitignore
    if let Ok(repo) = git2::Repository::discover(dir) {
        if let Some(workdir) = repo.workdir() {
            if let Ok(index) = repo.index() {
                for entry in index.iter() {
                    if let Ok(path_str) = std::str::from_utf8(&entry.path) {
                        let full_path = workdir.join(path_str);
                        if full_path.starts_with(dir) {
                            if let Some(name) = full_path.file_name().and_then(|n| n.to_str()) {
                                if is_source_file(name) && !is_ignored_file_name(name) {
                                    if let Ok(meta) = std::fs::symlink_metadata(&full_path) {
                                        if meta.is_file() && meta.len() <= 500_000 {
                                            files.push(full_path);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let mut status_opts = git2::StatusOptions::new();
            status_opts.include_untracked(true);
            status_opts.recurse_untracked_dirs(true);
            if let Ok(statuses) = repo.statuses(Some(&mut status_opts)) {
                for entry in statuses.iter() {
                    let status = entry.status();
                    if status.contains(git2::Status::WT_NEW) {
                        if let Some(path_str) = entry.path() {
                            let full_path = workdir.join(path_str);
                            if full_path.starts_with(dir) {
                                if let Some(name) = full_path.file_name().and_then(|n| n.to_str()) {
                                    if is_source_file(name) && !is_ignored_file_name(name) {
                                        if let Ok(meta) = std::fs::symlink_metadata(&full_path) {
                                            if meta.is_file() && meta.len() <= 500_000 {
                                                files.push(full_path);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if !files.is_empty() {
                return files;
            }
        }
    }

    // Fallback to recursive filesystem traversal
    collect_files_recursive(dir, &mut files);
    files
}

fn is_ignored_directory(name: &str) -> bool {
    matches!(
        name,
        "target"
            | "node_modules"
            | "dist"
            | "build"
            | "vendor"
            | "tmp"
            | "fixtures"
            | "testdata"
            | "venv"
            | ".venv"
            | "env"
            | "__pycache__"
            | "site-packages"
            | "Pods"
            | "Carthage"
            | "coverage"
            | "bin"
            | "obj"
            | "pkg"
            | "out"
            | ".next"
            | ".nuxt"
            | ".turbo"
            | ".cache"
            | ".cargo"
            | ".git"
    )
}

fn is_ignored_file_name(name: &str) -> bool {
    matches!(
        name,
        "Cargo.lock"
            | "package-lock.json"
            | "yarn.lock"
            | "pnpm-lock.yaml"
            | "composer.lock"
            | "Pipfile.lock"
            | "poetry.lock"
    ) || name.ends_with(".min.js")
        || name.ends_with(".min.css")
        || name.ends_with(".bundle.js")
        || name.ends_with(".bundle.css")
        || name.ends_with(".map")
        || name.ends_with(".sarif")
}

fn collect_files_recursive(dir: &Path, acc: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden directories (except .env files)
            if name.starts_with('.') && name != ".env" {
                continue;
            }
            if is_ignored_directory(&name) {
                continue;
            }

            // Never follow symlinks to avoid circular recursion or traversing outside workspace
            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            if file_type.is_symlink() {
                continue;
            }

            if file_type.is_dir() {
                collect_files_recursive(&path, acc);
            } else if file_type.is_file() && is_source_file(&name) && !is_ignored_file_name(&name) {
                if let Ok(meta) = entry.metadata() {
                    // Skip massive generated or minified source files exceeding 500KB
                    if meta.len() <= 500_000 {
                        acc.push(path);
                    }
                }
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
    use tempfile::tempdir;

    #[test]
    fn progress_scan_preserves_findings_and_reports_completion() {
        let tmp = tempdir().unwrap();
        for name in ["one.rs", "two.rs", "three.rs"] {
            std::fs::write(tmp.path().join(name), "pub fn sample() {}\n").unwrap();
        }
        let cfg = crate::config::LgtmConfig::default();
        let (expected_files, expected_findings) =
            FullRepoScanner::scan_with_baseline(tmp.path(), &cfg).unwrap();
        let completed = AtomicUsize::new(0);
        let (files, findings) =
            FullRepoScanner::scan_with_progress(tmp.path(), &cfg, &|done, total| {
                assert_eq!(total, 3);
                assert!(done <= total);
                completed.fetch_max(done, Ordering::SeqCst);
                true
            })
            .unwrap();
        assert_eq!(completed.load(Ordering::SeqCst), 3);
        assert_eq!(
            FullRepoScanner::generate_markdown_report(tmp.path(), &files, &findings.active),
            FullRepoScanner::generate_markdown_report(
                tmp.path(),
                &expected_files,
                &expected_findings.active
            )
        );
    }

    #[test]
    fn progress_observer_can_cancel_before_any_file_or_between_files() {
        let tmp = tempdir().unwrap();
        for index in 0..32 {
            std::fs::write(
                tmp.path().join(format!("sample{index}.rs")),
                "pub fn sample() {}\n",
            )
            .unwrap();
        }
        let cfg = crate::config::LgtmConfig::default();
        let completed = AtomicUsize::new(0);
        assert!(FullRepoScanner::scan_with_progress(tmp.path(), &cfg, &|_, _| false).is_err());
        let result = FullRepoScanner::scan_with_progress(tmp.path(), &cfg, &|done, _| {
            completed.fetch_max(done, Ordering::SeqCst);
            done == 0
        });
        assert!(result.is_err());
        assert!((1..32).contains(&completed.load(Ordering::SeqCst)));
    }

    #[test]
    fn test_is_source_file() {
        assert!(is_source_file("main.rs"));
        assert!(is_source_file("server.ts"));
        assert!(is_source_file(".env.production"));
        assert!(!is_source_file("image.png"));
        assert!(!is_source_file("binary.exe"));
    }

    #[test]
    fn test_is_ignored_directory_and_files() {
        assert!(is_ignored_directory("node_modules"));
        assert!(is_ignored_directory(".venv"));
        assert!(is_ignored_directory("target"));
        assert!(!is_ignored_directory("src"));

        assert!(is_ignored_file_name("Cargo.lock"));
        assert!(is_ignored_file_name("package-lock.json"));
        assert!(is_ignored_file_name("app.min.js"));
        assert!(is_ignored_file_name("bundle.map"));
        assert!(!is_ignored_file_name("main.rs"));
    }

    #[test]
    fn test_collect_files_avoids_symlink_loops() {
        let tmp = tempdir().unwrap();
        let root = tmp.path();

        let src = root.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("lib.rs"), "pub fn foo() {}").unwrap();

        // Create a circular directory symlink: src/loop -> src
        #[cfg(unix)]
        {
            let link = src.join("loop");
            let _ = std::os::unix::fs::symlink(&src, link); // lgtm:ignore
        }

        let mut files = Vec::new();
        collect_files_recursive(root, &mut files);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_name().unwrap(), "lib.rs");
    }
}
