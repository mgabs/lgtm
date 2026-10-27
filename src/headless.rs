use super::*;
use std::time::Duration;

/// Load `.lgtm.toml` (+ user file unless disabled), echoing warnings to stderr.
pub(crate) fn load_config(
    repo_path: &Path,
    no_user_config: bool,
) -> Result<lgtm_core::config::LgtmConfig> {
    let user = lgtm_core::config::user_config_path(no_user_config);
    let cfg = lgtm_core::config::LgtmConfig::load(repo_path, user.as_deref())?;
    for w in &cfg.warnings {
        eprintln!("warning: {}", w);
    }
    Ok(cfg)
}

pub(crate) fn report_suppressed(f: &lgtm_core::findings::Findings, show: bool) {
    if !f.existing.is_empty() {
        eprintln!("{} existing finding(s) in baseline", f.existing.len());
    }
    if !f.stale.is_empty() {
        eprintln!(
            "{} stale baseline entr{} (run `lgtm baseline update --prune`)",
            f.stale.len(),
            if f.stale.len() == 1 { "y" } else { "ies" }
        );
    }
    if f.suppressed.is_empty() {
        return;
    }
    eprintln!("{} finding(s) suppressed", f.suppressed.len());
    if show {
        for a in &f.suppressed {
            eprintln!(
                "  [{}] {}:{} {}",
                a.rule_id,
                a.file_path,
                a.line_no.unwrap_or(0),
                a.title
            );
        }
    }
}

pub(crate) fn run_ci_audit(
    repo_path: &Path,
    format: &str,
    output_file: Option<&Path>,
    strict: bool,
    show_suppressed: bool,
    no_user_config: bool,
    rev: Option<&str>,
) -> Result<i32> {
    let cfg = load_config(repo_path, no_user_config)?;
    let git_engine = git::GitEngine::new(repo_path);
    let snapshot = if git_engine.is_git_repo() {
        match rev {
            Some(spec) => git_engine.compute_revision_diff(spec)?,
            None => git_engine.compute_diff_snapshot()?,
        }
    } else {
        model::DiffSnapshot::default()
    };

    let found = lgtm_core::findings::run_in(repo_path, &snapshot.files, &cfg)?;
    report_suppressed(&found, show_suppressed);
    let alerts = found.active;
    let critical_count = alerts
        .iter()
        .filter(|a| a.severity == guard::Severity::Critical)
        .count();
    let high_count = alerts
        .iter()
        .filter(|a| a.severity == guard::Severity::High)
        .count();
    let warn_count = alerts
        .iter()
        .filter(|a| a.severity == guard::Severity::Warn)
        .count();
    let has_blocking = critical_count > 0 || high_count > 0 || (strict && warn_count > 0);

    let azure = std::env::var("TF_BUILD").is_ok()
        || format.eq_ignore_ascii_case("azure")
        || format.eq_ignore_ascii_case("vso");
    let github = !azure
        && (std::env::var("GITHUB_ACTIONS").is_ok()
            || format.eq_ignore_ascii_case("github")
            || format.eq_ignore_ascii_case("annotations"));
    if azure || github {
        for alert in &alerts {
            println!("{}", annotation(alert, azure));
        }
    }

    let output_text = match format.to_lowercase().as_str() {
        "json" => serde_json::to_string_pretty(&alerts)?,
        "sarif" => generate_sarif_report(&alerts)?,
        "github" | "annotations" => generate_ci_markdown_report(&snapshot, &alerts, has_blocking),
        _ => generate_ci_markdown_report(&snapshot, &alerts, has_blocking),
    };

    if let Some(path) = output_file {
        std::fs::write(path, &output_text)?;
        println!("✨ CI report written to {}", path.display());
    } else if !format.eq_ignore_ascii_case("annotations") {
        println!("{}", output_text);
    }

    if has_blocking {
        Ok(1)
    } else {
        Ok(0)
    }
}

/// One CI log annotation: GitHub `::error` or Azure Pipelines `##vso[task.logissue]`.
fn annotation(alert: &guard::GuardAlert, azure: bool) -> String {
    let error = matches!(
        alert.severity,
        guard::Severity::Critical | guard::Severity::High
    );
    let line = alert.line_no.unwrap_or(1);
    if azure {
        let esc = |t: &str| {
            t.replace('%', "%AZP25")
                .replace('\r', "%0D")
                .replace('\n', "%0A")
                .replace(';', "%3B")
                .replace(']', "%5D")
        };
        return format!(
            "##vso[task.logissue type={};sourcepath={};linenumber={line};columnnumber=1;code={}]{}: {}",
            if error { "error" } else { "warning" },
            esc(&alert.file_path),
            esc(&alert.rule_id),
            esc(&alert.title),
            esc(&alert.description)
        );
    }
    let level = if error { "error" } else { "warning" };
    format!(
        "::{level} file={},line={line},title={}::{}",
        alert.file_path, alert.title, alert.description
    )
}

pub(crate) fn generate_ci_markdown_report(
    snapshot: &model::DiffSnapshot,
    alerts: &[guard::GuardAlert],
    has_blocking: bool,
) -> String {
    let mut out = String::new();
    let status_badge = if has_blocking {
        "❌ **LGTM Risk Audit: FAILED**"
    } else if !alerts.is_empty() {
        "⚠️ **LGTM Risk Audit: PASSED WITH WARNINGS**"
    } else {
        "✅ **LGTM Risk Audit: PASSED**"
    };

    let total_add = snapshot.total_additions;
    let total_del = snapshot.total_deletions;
    let files_count = snapshot.files.len();

    out.push_str(&format!("## {}\n\n", status_badge));
    out.push_str(&format!(
        "📊 **Diff Summary:** {} file(s) changed (+{}/-{}) | **Alerts:** {} detected\n\n",
        files_count,
        total_add,
        total_del,
        alerts.len()
    ));

    if alerts.is_empty() {
        out.push_str("> ✨ No security vulnerabilities, test neutering, or hardcoded secrets detected in this diff.\n\n");
    } else {
        out.push_str("| Severity | Location | Risk Title | Details |\n");
        out.push_str("|:---:|:---|:---|:---|\n");

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

        // Agent Remediation Prompt block
        out.push_str("<details>\n<summary>🤖 <b>Agent Remediation Prompt (Copy & Paste to Agent)</b></summary>\n\n");
        out.push_str("```markdown\n");
        out.push_str("Please address the following code security and test integrity issues found in the latest diff:\n\n");
        for (i, alert) in alerts.iter().enumerate() {
            let loc = match alert.line_no {
                Some(lno) => format!("{}:{}", alert.file_path, lno),
                None => alert.file_path.clone(),
            };
            out.push_str(&format!(
                "{}. [{}] **{}** ({:?}): {}\n",
                i + 1,
                loc,
                alert.title,
                alert.severity,
                alert.description
            ));
        }
        out.push_str("\nEnsure all SQL/command inputs are sanitized, assertions are intact, and secrets are removed.\n");
        out.push_str("```\n\n</details>\n\n");
    }

    out.push_str("---\n*Generated automatically by [LGTM](https://github.com/mgabs/lgtm) Real-Time Diff Reviewer & Agent Steering Engine.*\n");
    out
}

pub(crate) fn generate_sarif_report(alerts: &[guard::GuardAlert]) -> Result<String> {
    let runs = serde_json::json!([{
        "tool": {
            "driver": {
                "name": "lgtm",
                "informationUri": "https://github.com/mgabs/lgtm",
                "rules": []
            }
        },
        "results": alerts.iter().map(|a| {
            let level = match a.severity {
                guard::Severity::Critical | guard::Severity::High => "error",
                guard::Severity::Warn => "warning",
            };
            serde_json::json!({
                "ruleId": a.rule_id,
                "partialFingerprints": { "lgtmFingerprint/v1": a.fingerprint },
                "level": level,
                "message": { "text": a.description },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": a.file_path },
                        "region": {
                            "startLine": a.line_no.unwrap_or(1)
                        }
                    }
                }]
            })
        }).collect::<Vec<_>>()
    }]);

    let sarif = serde_json::json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": runs
    });

    Ok(serde_json::to_string_pretty(&sarif)?)
}

pub(crate) fn run_risk_audit(repo_path: &Path) -> Result<i32> {
    let git_engine = git::GitEngine::new(repo_path);
    if !git_engine.is_git_repo() {
        eprintln!(
            "Error: Directory '{}' is not a git repository.",
            repo_path.display()
        );
        return Ok(1);
    }

    let snapshot = git_engine.compute_diff_snapshot()?;
    let found =
        lgtm_core::findings::run_in(repo_path, &snapshot.files, &load_config(repo_path, false)?)?;
    report_suppressed(&found, false);
    let alerts = found.active;

    if alerts.is_empty() {
        println!("✨ LGTM Risk Audit: No security risks or test fraud detected (0 warnings across {} files).", snapshot.files.len());
        return Ok(0);
    }

    println!(
        "🔍 LGTM Risk Audit: Found {} warning(s) in current diff:\n",
        alerts.len()
    );
    let mut has_failure = false;
    for alert in &alerts {
        let loc = match alert.line_no {
            Some(lno) => format!("{}:{}", alert.file_path, lno),
            None => alert.file_path.clone(),
        };

        let badge = match alert.severity {
            guard::Severity::Critical => {
                has_failure = true;
                "\x1b[1;31m🔴 CRITICAL\x1b[0m"
            }
            guard::Severity::High => {
                has_failure = true;
                "\x1b[1;33m🟠 HIGH\x1b[0m"
            }
            guard::Severity::Warn => "\x1b[1;33m🟡 WARN\x1b[0m",
        };

        println!("  {} {} — {}", badge, loc, alert.title);
        println!("     {}", alert.description);
    }

    if has_failure {
        eprintln!("\n❌ Risk audit failed with Critical/High severity warnings.");
        Ok(1)
    } else {
        println!("\n⚠️ Risk audit passed with non-blocking warnings.");
        Ok(0)
    }
}

pub(crate) fn run_init_config(repo_path: &Path) -> Result<()> {
    let config_file = repo_path.join(lgtm_core::config::REPO_FILE);
    if config_file.exists() {
        println!("Config already exists at {}", config_file.display());
        return Ok(());
    }

    let template = r#"# LGTM Configuration File (.lgtm.toml)
# Documentation: https://github.com/mgabs/lgtm

# editor = "nvim"
# agent_sink = "tmux"

[thresholds]
complexity_increase = 5

[ignore]
paths = ["package-lock.json", "Cargo.lock", "yarn.lock", "**/*.min.js"]

[ignore.rules]
# "sec/hardcoded-secret" = ["tests/fixtures/**"]

[severity]
# "code/complexity-increase" = "warning"
# "test/assertion-removed" = "high"

[baseline]
file = ".lgtm-baseline.json"
allow_critical = false
"#;

    std::fs::write(&config_file, template)?;
    println!(
        "✨ Initialized LGTM starter configuration at {}",
        config_file.display()
    );
    Ok(())
}

pub(crate) fn run_git_passthrough(args: &[String]) -> Result<i32> {
    let status = std::process::Command::new("git").args(args).status()?;
    Ok(status.code().unwrap_or(1))
}

/// `review export [target]` prints the state JSON; `review import <file>` merges it into its target.
pub(crate) fn run_review(repo_path: &Path, action: &str, arg: Option<&str>) -> Result<i32> {
    use lgtm_core::comment::CommentStore;
    use lgtm_core::review::{merge, target_pr, ReviewComment, ReviewState, ReviewStore, Status};

    let comment_store = CommentStore::new(repo_path);
    if action == "export" {
        let target = arg.unwrap_or("working");
        let pr = target_pr(target);
        let mut st = ReviewStore::open(repo_path, target)?.load()?;
        let live_comments: Vec<ReviewComment> = comment_store
            .load_comments()
            .into_iter()
            .filter(|c| c.pr_number == pr)
            .map(|a| ReviewComment::new(a, &[]))
            .collect();
        let _ = merge(
            &mut st,
            ReviewState {
                version: lgtm_core::review::VERSION,
                target: target.to_string(),
                comments: live_comments,
                ..Default::default()
            },
        );
        println!("{}", serde_json::to_string_pretty(&st)?);
        return Ok(0);
    }
    let file = arg.ok_or_else(|| anyhow::anyhow!("usage: lgtm review import <file>"))?;
    let other: ReviewState = serde_json::from_str(&std::fs::read_to_string(file)?)?;
    if other.version > lgtm_core::review::VERSION {
        anyhow::bail!(
            "{} was written by a newer lgtm (version {})",
            file,
            other.version
        );
    }
    let target = other.target.clone();
    let pr = target_pr(&target);
    let store = ReviewStore::open(repo_path, &target)?;
    let mut base = store.load()?;
    let all_comments = comment_store.load_comments();
    let mut unrelated_comments = Vec::new();
    let mut target_comments = Vec::new();
    for c in all_comments {
        if c.pr_number == pr {
            target_comments.push(ReviewComment::new(c, &[]));
        } else {
            unrelated_comments.push(c);
        }
    }
    let _ = merge(
        &mut base,
        ReviewState {
            version: lgtm_core::review::VERSION,
            target: target.clone(),
            comments: target_comments,
            ..Default::default()
        },
    );
    let conflicts = merge(&mut base, other);
    store.save(&base)?;

    for rc in &base.comments {
        let mut anchor = rc.anchor.clone();
        anchor.is_resolved = rc.status == Status::Resolved;
        anchor.orphaned = rc.status == Status::Orphaned;
        if anchor.id.is_empty() {
            anchor.id = lgtm_core::comment::new_id();
        }
        anchor.pr_number = pr;
        unrelated_comments.push(anchor);
    }
    comment_store.save_comments(&unrelated_comments)?;

    for c in &conflicts {
        eprintln!("conflict: {}", c);
    }
    println!(
        "Imported into '{}': {} hunk marks, {} comments, {} conflicts",
        target,
        base.hunks.len(),
        base.comments.len(),
        conflicts.len()
    );
    Ok(0)
}

pub(crate) fn run_pr_post(repo_path: &Path, pr_opt: Option<u64>, yes: bool) -> Result<i32> {
    use lgtm_core::comment::CommentStore;
    use lgtm_core::github::post;
    use std::time::Duration;

    let forge = lgtm_core::forge::open(repo_path);
    let store = CommentStore::new(repo_path);
    let mut comments = store.load_comments();

    let pr = match pr_opt {
        Some(n) => n,
        None => {
            // Try detecting from comments first if there's only one target PR
            let pr_from_comments: std::collections::HashSet<u64> =
                comments.iter().filter_map(|c| c.pr_number).collect();
            if pr_from_comments.len() == 1 {
                *pr_from_comments.iter().next().unwrap()
            } else {
                // Try finding an open PR for the current branch
                let current_branch =
                    lgtm_core::git2::Repository::open(repo_path)
                        .ok()
                        .and_then(|r| {
                            let h = r.head().ok()?;
                            if h.is_branch() {
                                h.shorthand().map(String::from)
                            } else {
                                None
                            }
                        });

                let matched_pr = if let Some(ref branch) = current_branch {
                    forge.list_prs().ok().and_then(|list| {
                        list.into_iter()
                            .find(|p| &p.head_ref_name == branch)
                            .map(|p| p.number)
                    })
                } else {
                    None
                };

                match matched_pr {
                    Some(n) => n,
                    None => {
                        anyhow::bail!("PR number required: specify `lgtm pr post <PR>` or switch to a branch with an open PR.");
                    }
                }
            }
        }
    };

    println!("Checking diff and directives for PR #{pr}...");
    let diff = forge.pr_diff(pr)?;

    let associated = post::associate_directives(&mut comments, &diff.files, pr);
    if associated > 0 {
        store.save_comments(&comments)?;
        println!("Associated {associated} local directive(s) with PR #{pr}");
    }

    let plan = post::plan(&comments, &diff.files, pr);
    if plan.total() == 0 {
        if plan.skipped.is_empty() {
            println!("No pending comments to post for PR #{pr}.");
        } else {
            println!("No eligible comments to post for PR #{pr}.");
            for s in &plan.skipped {
                let c = &comments[s.index];
                println!(
                    "  - Skipped {}:{}: {}",
                    c.file_path, c.target_line_no, s.reason
                );
            }
        }
        return Ok(0);
    }

    if !yes {
        println!(
            "\nDry Run: {} comment(s) planned for PR #{pr}:",
            plan.total()
        );
        if !plan.inline.is_empty() {
            println!("\nInline Comments ({}):", plan.inline.len());
            for &idx in &plan.inline {
                let c = &comments[idx];
                println!(
                    "  + {}:{} [{:?}] {}",
                    c.file_path,
                    c.target_line_no,
                    c.side,
                    c.comment_text.lines().next().unwrap_or("")
                );
            }
        }
        if !plan.general.is_empty() {
            println!("\nGeneral Comments outside diff ({}):", plan.general.len());
            for &idx in &plan.general {
                let c = &comments[idx];
                println!(
                    "  ~ {}:{} {}",
                    c.file_path,
                    c.target_line_no,
                    c.comment_text.lines().next().unwrap_or("")
                );
            }
        }
        if !plan.skipped.is_empty() {
            println!("\nSkipped Comments ({}):", plan.skipped.len());
            for s in &plan.skipped {
                let c = &comments[s.index];
                println!("  - {}:{} - {}", c.file_path, c.target_line_no, s.reason);
            }
        }
        println!("\nRun `lgtm pr post {} --yes` to post these comments.", pr);
        return Ok(0);
    }

    println!("\nPosting {} comment(s) to PR #{pr}...", plan.total());
    let report = post::post(
        forge.as_ref(),
        pr,
        &mut comments,
        &plan,
        Duration::from_millis(150),
        |cs| store.save_comments(cs),
    );

    println!("{}", report.summary());
    if let Some(err) = report.error {
        eprintln!("Error posting comments: {}", err);
        return Ok(1);
    }

    Ok(0)
}

pub(crate) fn run_pr_reply(repo_path: &Path, pr: u64, thread_id: &str, body: &str) -> Result<i32> {
    let forge = lgtm_core::forge::open(repo_path);
    let thread = forge.thread(pr, thread_id)?;
    let comment_id = thread
        .and_then(|t| t.comments.first().map(|c| c.id))
        .unwrap_or(0);
    let reply_id = forge.reply(pr, thread_id, comment_id, body)?;
    println!("Reply posted successfully (id: {reply_id})");
    Ok(0)
}

pub(crate) fn run_pr_resolve(
    repo_path: &Path,
    pr: u64,
    thread_id: &str,
    unresolve: bool,
) -> Result<i32> {
    let forge = lgtm_core::forge::open(repo_path);
    forge.set_resolved(pr, thread_id, !unresolve)?;
    if unresolve {
        println!("Thread {thread_id} reopened (unresolved)");
    } else {
        println!("Thread {thread_id} resolved");
    }
    Ok(0)
}

pub(crate) fn run_baseline_update(repo_path: &Path, prune: bool) -> Result<i32> {
    let cfg = load_config(repo_path, false)?;
    let (_, alerts) = scan::FullRepoScanner::scan_unbaselined(repo_path, &cfg)?;
    let path = lgtm_core::baseline::path_for(repo_path, &cfg);
    let mut bl = lgtm_core::baseline::Baseline::load(&path)?;
    bl.update(&alerts, cfg.baseline.allow_critical, prune);
    bl.save(&path)?;
    println!(
        "Baseline written to {} ({} entries)",
        path.display(),
        bl.entries.len()
    );
    Ok(0)
}

pub(crate) fn run_hook_command(repo_path: &Path, action: &str) -> Result<i32> {
    let engine = git::GitEngine::new(repo_path);
    if !engine.is_git_repo() {
        eprintln!("lgtm hook: Not a git repository");
        return Ok(1);
    }
    match action {
        "install" => {
            let path = engine.install_pre_commit_hook()?;
            println!("Installed LGTM pre-commit hook at {}", path.display());
            Ok(0)
        }
        "uninstall" => {
            let removed = engine.uninstall_pre_commit_hook()?;
            if removed {
                println!("Uninstalled LGTM pre-commit hook");
            } else {
                println!("No LGTM pre-commit hook found to uninstall");
            }
            Ok(0)
        }
        "status" => {
            let installed = engine.pre_commit_hook_status()?;
            if installed {
                println!("LGTM pre-commit hook is installed and active");
            } else {
                println!("LGTM pre-commit hook is not installed");
            }
            Ok(0)
        }
        _ => anyhow::bail!("Unknown hook action: {}", action),
    }
}

pub(crate) fn run_gate(repo_path: &Path, timeout_secs: u64, message: Option<&str>) -> Result<i32> {
    let git = git::GitEngine::new(repo_path);
    if !git.is_git_repo() {
        eprintln!("lgtm gate: Not a git repository");
        return Ok(1);
    }

    let snapshot = git.compute_diff_snapshot()?;
    if snapshot.files.is_empty() {
        println!("✅ lgtm gate: Working tree clean, no unreviewed changes.");
        return Ok(0);
    }

    let lgtm_dir = repo_path.join(".lgtm");
    let _ = std::fs::create_dir_all(&lgtm_dir); // lgtm:ignore
    let gate_file = lgtm_dir.join("gate.json");

    let gate_state = serde_json::json!({
        "status": "waiting",
        "timestamp": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        "timeout": timeout_secs,
        "message": message.unwrap_or("Agent generated changes awaiting human review"),
        "files_changed": snapshot.files.len(),
    });

    std::fs::write(&gate_file, serde_json::to_string_pretty(&gate_state)?)?;

    println!(
        "🛑 lgtm gate: {} changed file(s) awaiting review.",
        snapshot.files.len()
    );
    if let Some(msg) = message {
        println!("   Prompt: {}", msg);
    }
    println!("   Waiting for approval in TUI (press 'A' or run ':approve' / ':reject-all')...");

    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(timeout_secs);

    loop {
        if start.elapsed() > timeout {
            let _ = std::fs::remove_file(&gate_file); // lgtm:ignore
            eprintln!(
                "⏱️ lgtm gate timed out after {}s waiting for review.",
                timeout_secs
            );
            return Ok(1);
        }

        std::thread::sleep(Duration::from_millis(200));

        if let Ok(content) = std::fs::read_to_string(&gate_file) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                match val.get("status").and_then(|s| s.as_str()) {
                    Some("approved") => {
                        let feedback = val
                            .get("feedback")
                            .and_then(|f| f.as_str())
                            .map(|s| s.to_string());
                        let _ = std::fs::remove_file(&gate_file); // lgtm:ignore
                        if let Some(fb) = feedback {
                            println!("✅ lgtm gate: Changes approved by human reviewer with directives:\n{}", fb);
                        } else {
                            println!("✅ lgtm gate: Changes approved by human reviewer!");
                        }
                        return Ok(0);
                    }
                    Some("rejected") => {
                        let feedback = val
                            .get("feedback")
                            .and_then(|f| f.as_str())
                            .map(|s| s.to_string());
                        let _ = std::fs::remove_file(&gate_file); // lgtm:ignore
                        if let Some(fb) = feedback {
                            eprintln!("❌ lgtm gate: Changes rejected by human reviewer:\n{}", fb);
                        } else {
                            eprintln!("❌ lgtm gate: Changes rejected by human reviewer.");
                        }
                        return Ok(1);
                    }
                    _ => {}
                }
            }
        } else {
            // File was deleted manually
            println!("ℹ️ lgtm gate: Gate file cleared, resuming.");
            return Ok(0);
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(unused_imports)]
    use super::*;

    fn test_dir(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("lgtm-test-{}-{}", name, unique));
        let _ = std::fs::create_dir_all(&path);
        path
    }

    #[test]
    fn ci_rev_checks_committed_diff_and_errors_on_bad_rev() {
        let dir = test_dir("ci-rev");
        let git = |args: &[&str]| {
            let mut cmd = std::process::Command::new("git");
            cmd.args(["-c", "user.email=a@b", "-c", "user.name=a"])
                .args(args)
                .current_dir(&dir);
            for var in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_PREFIX"] {
                cmd.env_remove(var);
            }
            let ok = cmd.output().unwrap().status.success();
            assert!(ok, "git {args:?}");
        };
        git(&["init", "-q"]);
        std::fs::write(dir.join("a.py"), "a = 1\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "-qm", "i"]);
        std::fs::write(dir.join("b.py"), "key = \"AKIAABCDEFGHIJKLMNOP\"\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "-qm", "x"]);
        let out = dir.join("r.json");
        let code = run_ci_audit(
            &dir,
            "json",
            Some(&out),
            false,
            false,
            true,
            Some("HEAD~1..HEAD"),
        )
        .unwrap();
        assert_eq!(code, 1);
        assert!(std::fs::read_to_string(&out).unwrap().contains("rule_id"));
        let clean = run_ci_audit(&dir, "json", Some(&out), false, false, true, None).unwrap();
        assert_eq!(clean, 0);
        assert!(run_ci_audit(&dir, "json", None, false, false, true, Some("nope..bad")).is_err());
    }

    #[test]
    fn test_ci_markdown_report_generation() {
        let snapshot = model::DiffSnapshot {
            files: vec![],
            total_additions: 10,
            total_deletions: 5,
        };
        let alerts = vec![guard::GuardAlert {
            severity: guard::Severity::Critical,
            rule_id: "test/fixture".to_string(),
            fingerprint: String::new(),
            title: "SQL Injection".into(),
            description: "Raw SQL query".into(),
            line_no: Some(42),
            file_path: "src/db.rs".into(),
        }];

        let report_failed = generate_ci_markdown_report(&snapshot, &alerts, true);
        assert!(report_failed.contains("LGTM Risk Audit: FAILED"));
        assert!(report_failed.contains("SQL Injection"));
        assert!(report_failed.contains("src/db.rs:42"));
        assert!(report_failed.contains("Agent Remediation Prompt"));

        let report_passed = generate_ci_markdown_report(&snapshot, &[], false);
        assert!(report_passed.contains("LGTM Risk Audit: PASSED"));
        assert!(report_passed.contains("No security vulnerabilities"));
    }

    #[test]
    fn annotations_for_github_and_azure() {
        let alert = guard::GuardAlert {
            severity: guard::Severity::Warn,
            rule_id: "r/x".into(),
            fingerprint: String::new(),
            title: "T".into(),
            description: "a;b]\nc 100%".into(),
            line_no: None,
            file_path: "src/a.rs".into(),
        };
        assert_eq!(
            annotation(&alert, false),
            "::warning file=src/a.rs,line=1,title=T::a;b]\nc 100%"
        );
        assert_eq!(
            annotation(&alert, true),
            "##vso[task.logissue type=warning;sourcepath=src/a.rs;linenumber=1;columnnumber=1;code=r/x]T: a%3Bb%5D%0Ac 100%AZP25"
        );
    }

    #[test]
    fn test_sarif_report_generation() {
        let alerts = vec![
            guard::GuardAlert {
                severity: guard::Severity::High,
                rule_id: "sec/tls-bypass".to_string(),
                fingerprint: String::new(),
                title: "TLS Verification Disabled".into(),
                description: format!("verify={}", "False"), // lgtm:ignore
                line_no: Some(12),
                file_path: "src/net.py".into(),
            },
            guard::GuardAlert {
                severity: guard::Severity::Warn,
                rule_id: "test/swallowed-exception".to_string(),
                fingerprint: String::new(),
                title: "Swallowed Exception".into(),
                description: "empty catch".into(),
                line_no: None,
                file_path: "src/util.ts".into(),
            },
        ];

        let sarif_json = generate_sarif_report(&alerts).unwrap();
        assert!(sarif_json.contains("2.1.0"));
        assert!(sarif_json.contains("\"ruleId\": \"sec/tls-bypass\""));
        assert!(sarif_json.contains("test/swallowed-exception"));
        assert!(sarif_json.contains("lgtmFingerprint/v1"));
    }

    #[test]
    fn test_run_init_config() {
        let tmp = test_dir("init");
        run_init_config(&tmp).unwrap();
        let config_file = tmp.join(lgtm_core::config::REPO_FILE);
        assert!(config_file.exists());
        let cfg = lgtm_core::config::LgtmConfig::load(&tmp, None).unwrap();
        assert!(cfg.warnings.is_empty(), "warnings: {:?}", cfg.warnings);
        assert_eq!(cfg.thresholds.complexity_increase, 5);
        assert!(cfg.ignore.paths.contains(&"Cargo.lock".to_string()));
    }

    #[test]
    fn test_baseline_update_roundtrip() {
        let tmp = test_dir("baseline");
        std::fs::write(
            tmp.join("a.py"),
            format!("requests.get(u, {})\n", ["verify=", "False"].concat()), // lgtm:ignore
        )
        .unwrap();
        assert_eq!(run_baseline_update(&tmp, false).unwrap(), 0);
        let bl = lgtm_core::baseline::Baseline::load(&tmp.join(".lgtm-baseline.json")).unwrap();
        assert_eq!(bl.entries.len(), 1);
        assert_eq!(bl.entries[0].rule_id, "sec/tls-bypass");
        std::fs::remove_file(tmp.join("a.py")).unwrap();
        assert_eq!(run_baseline_update(&tmp, true).unwrap(), 0);
        let bl = lgtm_core::baseline::Baseline::load(&tmp.join(".lgtm-baseline.json")).unwrap();
        assert!(bl.entries.is_empty());
    }

    #[test]
    fn review_export_import_round_trip() {
        let tmp = std::env::temp_dir().join(format!("lgtm-rev-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let st = r#"{"version":1,"target":"feat","hunks":[{"file":"a","content_hash":"h","last_line":1,"reviewed_at":"t"}],"comments":[{"file_path":"a","target_line_no":1,"target_code":"let x = 1;","context_before":[],"context_after":[],"comment_text":"fix x","created_at_turn":1,"is_resolved":false}]}"#;
        let f = tmp.join("in.json");
        std::fs::write(&f, st).unwrap();
        assert_eq!(run_review(&tmp, "import", f.to_str()).unwrap(), 0);
        assert_eq!(run_review(&tmp, "export", Some("feat")).unwrap(), 0);
        let back = lgtm_core::review::ReviewStore::open(&tmp, "feat")
            .unwrap()
            .load()
            .unwrap();
        assert_eq!(back.hunks.len(), 1);
        assert_eq!(back.comments.len(), 1);
        let synced = lgtm_core::comment::CommentStore::new(&tmp).load_comments();
        assert_eq!(synced.len(), 1);
        assert_eq!(synced[0].comment_text, "fix x");
        assert!(run_review(&tmp, "import", None).is_err());
        std::fs::write(&f, r#"{"version":9,"target":"feat"}"#).unwrap();
        assert!(run_review(&tmp, "import", f.to_str()).is_err());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_headless_hook_command() {
        let tmp = std::env::temp_dir().join(format!("lgtm-hook-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp); // lgtm:ignore
        lgtm_core::git2::Repository::init(&tmp).unwrap();

        assert_eq!(run_hook_command(&tmp, "status").unwrap(), 0);
        assert_eq!(run_hook_command(&tmp, "install").unwrap(), 0);
        assert_eq!(run_hook_command(&tmp, "status").unwrap(), 0);
        assert_eq!(run_hook_command(&tmp, "uninstall").unwrap(), 0);
        assert_eq!(run_hook_command(&tmp, "status").unwrap(), 0);

        let _ = std::fs::remove_dir_all(&tmp); // lgtm:ignore
    }
}
