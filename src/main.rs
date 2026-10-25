mod app;
use lgtm_core::{comment, git, guard, ipc, model, pr, scan, turn, watcher};
use lgtm_mcp as mcp;
mod ui;

use anyhow::Result;
use app::App;
use clap::Parser;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use watcher::RepoWatcher;

mod cli;
mod diagnostics;
mod event_loop;
mod headless;

use cli::{AuditStartMode, Cli, Commands};
use headless::{run_ci_audit, run_git_passthrough, run_init_config, run_risk_audit};

fn run_check(
    target: PathBuf,
    format: &str,
    output: Option<&Path>,
    strict: bool,
    show_suppressed: bool,
    no_user_config: bool,
    rev: Option<&str>,
) -> ExitCode {
    let repo_path = std::fs::canonicalize(&target).unwrap_or(target);
    match run_ci_audit(
        &repo_path,
        format,
        output,
        strict,
        show_suppressed,
        no_user_config,
        rev,
    ) {
        Ok(code) => ExitCode::from(code as u8),
        Err(e) => {
            eprintln!("Error running check audit: {}", e);
            ExitCode::from(2)
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();

    // Check for direct --init flag
    if args.iter().any(|a| a == "--init") {
        let repo_path = PathBuf::from(".");
        if let Err(e) = run_init_config(&repo_path) {
            eprintln!("Error initializing config: {}", e);
            return ExitCode::from(1);
        }
        return ExitCode::SUCCESS;
    }

    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e) => {
            // Check if user is invoking an external git command like `lgtm log -p`
            if args.len() > 1 && !args[1].starts_with('-') {
                let git_subcmd = &args[1];
                let is_known_git = [
                    "add", "branch", "checkout", "clone", "commit", "fetch", "log", "merge",
                    "pull", "push", "rebase", "reset", "restore", "revert", "show", "stash",
                    "switch", "tag", "remote", "blame",
                ]
                .contains(&git_subcmd.as_str());

                if is_known_git {
                    return match run_git_passthrough(&args[1..]) {
                        Ok(code) => ExitCode::from(code as u8),
                        Err(err) => {
                            eprintln!("Failed to execute git command: {}", err);
                            ExitCode::from(1)
                        }
                    };
                }
            }
            e.exit();
        }
    };

    if cli.debug {
        let path = cli
            .debug_log
            .as_ref()
            .cloned()
            .unwrap_or_else(|| std::env::temp_dir().join("lgtm-debug.jsonl"));
        if let Err(error) = lgtm_core::debug::init(&path) {
            eprintln!("Could not open debug log {}: {error}", path.display());
            return ExitCode::from(2);
        }
    }

    if cli.init {
        let repo_path = std::fs::canonicalize(&cli.repo).unwrap_or(cli.repo);
        if let Err(e) = run_init_config(&repo_path) {
            eprintln!("Error initializing config: {}", e);
            return ExitCode::from(1);
        }
        return ExitCode::SUCCESS;
    }

    match cli.command {
        Some(Commands::Gate {
            repo,
            timeout,
            message,
        }) => {
            let target = repo.unwrap_or(cli.repo);
            let repo_path = std::fs::canonicalize(&target).unwrap_or(target);
            match headless::run_gate(&repo_path, timeout, message.as_deref()) {
                Ok(code) => ExitCode::from(code as u8),
                Err(e) => {
                    eprintln!("Error running gate: {}", e);
                    ExitCode::from(2)
                }
            }
        }
        Some(Commands::Risk { repo }) => {
            let target = repo.unwrap_or(cli.repo);
            let repo_path = std::fs::canonicalize(&target).unwrap_or(target);
            match run_risk_audit(&repo_path) {
                Ok(code) => ExitCode::from(code as u8),
                Err(e) => {
                    eprintln!("Error running risk audit: {}", e);
                    ExitCode::from(1)
                }
            }
        }
        Some(Commands::Config {
            repo,
            no_user_config,
            ..
        }) => {
            let target = repo.unwrap_or(cli.repo);
            let repo_path = std::fs::canonicalize(&target).unwrap_or(target);
            let user = lgtm_core::config::user_config_path(no_user_config);
            match lgtm_core::config::LgtmConfig::load(&repo_path, user.as_deref()) {
                Ok(c) => {
                    print!("{}", c.show());
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("Error loading config: {e}");
                    ExitCode::from(1)
                }
            }
        }
        Some(Commands::Init { repo }) => {
            let target = repo.unwrap_or(cli.repo);
            let repo_path = std::fs::canonicalize(&target).unwrap_or(target);
            if let Err(e) = run_init_config(&repo_path) {
                eprintln!("Error initializing config: {}", e);
                return ExitCode::from(1);
            }
            ExitCode::SUCCESS
        }
        Some(Commands::Mcp { repo }) => {
            let target = repo.unwrap_or(cli.repo);
            let repo_path = std::fs::canonicalize(&target).unwrap_or(target);
            let server = mcp::McpServer::new(&repo_path);
            if let Err(e) = server.run_stdio() {
                eprintln!("MCP server error: {}", e);
                return ExitCode::from(1);
            }
            ExitCode::SUCCESS
        }
        Some(Commands::Serve { mcp, repo }) => {
            let target = repo.unwrap_or(cli.repo);
            let repo_path = std::fs::canonicalize(&target).unwrap_or(target);
            if mcp {
                let server = mcp::McpServer::new(&repo_path);
                if let Err(e) = server.run_stdio() {
                    eprintln!("MCP server error: {}", e);
                    return ExitCode::from(1);
                }
            }
            ExitCode::SUCCESS
        }
        Some(Commands::External(ext_args)) => {
            let first = ext_args.first().map(|s| s.as_str()).unwrap_or("");
            let repo_path = std::fs::canonicalize(&cli.repo).unwrap_or(cli.repo.clone());
            let git_engine = git::GitEngine::new(&repo_path);
            let expanded = git_engine.expand_branch_spec(first);
            let is_branch_or_rev = first.contains("..")
                || first.starts_with("HEAD~")
                || first.starts_with("HEAD^")
                || expanded != first;

            if is_branch_or_rev {
                if let Err(e) = run_tui(
                    repo_path,
                    cli.agent_pane,
                    cli.agent_sink,
                    cli.debounce,
                    None,
                    Some(first.to_string()),
                    None,
                ) {
                    eprintln!("Error reviewing revision {}: {}", first, e);
                    return ExitCode::from(1);
                }
                ExitCode::SUCCESS
            } else {
                match run_git_passthrough(&ext_args) {
                    Ok(code) => ExitCode::from(code as u8),
                    Err(e) => {
                        eprintln!("Failed to execute git command: {}", e);
                        ExitCode::from(1)
                    }
                }
            }
        }
        Some(Commands::Pr { number }) => {
            let repo_path = std::fs::canonicalize(&cli.repo).unwrap_or(cli.repo);
            if let Err(e) = run_tui(
                repo_path,
                cli.agent_pane,
                cli.agent_sink,
                cli.debounce,
                Some(number),
                None,
                None,
            ) {
                eprintln!("Error reviewing PR #{}: {}", number, e);
                return ExitCode::from(1);
            }
            ExitCode::SUCCESS
        }
        Some(Commands::Prs { state }) => match lgtm_core::forge::open(
            &std::fs::canonicalize(&cli.repo).unwrap_or(cli.repo.clone()),
        )
        .list_prs_state(pr::PullRequestState::parse(&state).expect("validated PR state"))
        {
            Ok(prs) => {
                println!("📋 {} pull requests ({} total):\n", state, prs.len());
                for pr in prs {
                    println!(
                            "  \x1b[1;34m#{}\x1b[0m {} \x1b[33mby @{}\x1b[0m (\x1b[32m+{}\x1b[0m/\x1b[31m-{}\x1b[0m) [\x1b[36m{}\x1b[0m] ({})",
                            pr.number, pr.title, pr.author.login, pr.additions, pr.deletions, pr.head_ref_name, pr.state
                        );
                }
                println!("\nRun `lgtm pr <num>` to review a PR.");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("Error listing PRs: {}", e);
                ExitCode::from(1)
            }
        },
        Some(Commands::Check {
            repo,
            format,
            output,
            strict,
            show_suppressed,
            no_user_config,
            rev,
        }) => run_check(
            repo.unwrap_or(cli.repo),
            &format,
            output.as_deref(),
            strict,
            show_suppressed,
            no_user_config,
            rev.as_deref(),
        ),
        // CI ignores the user-level config so results match across machines
        Some(Commands::Ci {
            repo,
            format,
            output,
            strict,
            show_suppressed,
            rev,
            ..
        }) => run_check(
            repo.unwrap_or(cli.repo),
            &format,
            output.as_deref(),
            strict,
            show_suppressed,
            true,
            rev.as_deref(),
        ),
        Some(Commands::Baseline { prune, repo, .. }) => {
            let target = repo.unwrap_or(cli.repo);
            let repo_path = std::fs::canonicalize(&target).unwrap_or(target);
            match headless::run_baseline_update(&repo_path, prune) {
                Ok(code) => ExitCode::from(code as u8),
                Err(e) => {
                    eprintln!("Error updating baseline: {}", e);
                    ExitCode::from(1)
                }
            }
        }
        Some(Commands::Review {
            action: Some(action),
            arg,
            repo,
        }) => {
            let target = repo.unwrap_or(cli.repo);
            let repo_path = std::fs::canonicalize(&target).unwrap_or(target);
            match headless::run_review(&repo_path, &action, arg.as_deref()) {
                Ok(code) => ExitCode::from(code as u8),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    ExitCode::from(1)
                }
            }
        }
        Some(Commands::Audit { repo }) => {
            let target = repo.unwrap_or(cli.repo);
            let repo_path = std::fs::canonicalize(&target).unwrap_or(target);
            if let Err(e) = run_tui(
                repo_path,
                cli.agent_pane,
                cli.agent_sink,
                cli.debounce,
                None,
                None,
                Some(AuditStartMode::FullRepo),
            ) {
                eprintln!("Error running audit TUI: {}", e);
                return ExitCode::from(1);
            }
            ExitCode::SUCCESS
        }
        Some(Commands::Scan {
            repo,
            output,
            format,
            tui,
            target,
        }) => {
            let mut resolved_repo = repo;
            let mut open_tui = tui;
            let mut full_repo = false;

            if let Some(ref t) = target {
                if t == "repo" {
                    open_tui = true;
                    full_repo = true;
                } else if resolved_repo.is_none() {
                    resolved_repo = Some(PathBuf::from(t));
                }
            }

            let repo_target = resolved_repo.unwrap_or(cli.repo);
            let repo_path = std::fs::canonicalize(&repo_target).unwrap_or(repo_target);

            if open_tui {
                let start_mode = if full_repo {
                    AuditStartMode::FullRepo
                } else {
                    AuditStartMode::DiffChanges
                };
                if let Err(e) = run_tui(
                    repo_path,
                    cli.agent_pane,
                    cli.agent_sink,
                    cli.debounce,
                    None,
                    None,
                    Some(start_mode),
                ) {
                    eprintln!("Error running scan TUI: {}", e);
                    return ExitCode::from(1);
                }
                return ExitCode::SUCCESS;
            }

            println!("🔍 Running full codebase security and architecture audit...\n");
            let cfg = match headless::load_config(&repo_path, false) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error loading config: {}", e);
                    return ExitCode::from(1);
                }
            };
            match scan::FullRepoScanner::scan_with_baseline(&repo_path, &cfg) {
                Ok((files, found)) => {
                    headless::report_suppressed(&found, false);
                    let alerts = found.active;
                    let output_text = if format.eq_ignore_ascii_case("json") {
                        serde_json::to_string_pretty(&alerts).unwrap_or_default()
                    } else {
                        scan::FullRepoScanner::generate_markdown_report(&repo_path, &files, &alerts)
                    };

                    if let Some(path) = output {
                        if let Err(e) = std::fs::write(&path, &output_text) {
                            eprintln!("Failed to write report to {}: {}", path.display(), e);
                            return ExitCode::from(1);
                        }
                        println!("✨ Full scan report written to: {}", path.display());
                    } else {
                        println!("{}", output_text);
                    }

                    let has_critical = alerts
                        .iter()
                        .any(|a| a.severity == guard::Severity::Critical);
                    if has_critical {
                        ExitCode::from(1)
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("Error running repository scan: {}", e);
                    ExitCode::from(1)
                }
            }
        }
        Some(Commands::Hook { action, repo }) => {
            let target = repo.unwrap_or(cli.repo);
            let repo_path = std::fs::canonicalize(&target).unwrap_or(target);
            match headless::run_hook_command(&repo_path, &action) {
                Ok(code) => ExitCode::from(code as u8),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    ExitCode::from(1)
                }
            }
        }
        Some(Commands::Diff { revision }) | Some(Commands::Status { revision }) => {
            let repo_path = std::fs::canonicalize(&cli.repo).unwrap_or(cli.repo);
            if let Err(e) = run_tui(
                repo_path,
                cli.agent_pane,
                cli.agent_sink,
                cli.debounce,
                cli.pr,
                revision,
                None,
            ) {
                eprintln!("Error running lgtm: {}", e);
                return ExitCode::from(1);
            }
            ExitCode::SUCCESS
        }
        Some(Commands::Review { .. }) | None => {
            let repo_path = std::fs::canonicalize(&cli.repo).unwrap_or(cli.repo);
            if let Err(e) = run_tui(
                repo_path,
                cli.agent_pane,
                cli.agent_sink,
                cli.debounce,
                cli.pr,
                None,
                None,
            ) {
                eprintln!("Error running lgtm: {}", e);
                return ExitCode::from(1);
            }
            ExitCode::SUCCESS
        }
    }
}

fn run_tui(
    repo_path: PathBuf,
    agent_pane: Option<String>,
    agent_sink: Option<String>,
    debounce: u64,
    pr_number: Option<u64>,
    revision: Option<String>,
    start_audit: Option<AuditStartMode>,
) -> Result<()> {
    // Setup watcher channel
    let (watcher_tx, watcher_rx) = crossbeam_channel::unbounded();
    let _watcher = match RepoWatcher::spawn(&repo_path, watcher_tx, debounce) {
        Ok(w) => Some(w),
        Err(e) => {
            eprintln!(
                "Warning: Failed to start file watcher: {}. Status updates use polling; use :reload for file changes.",
                e
            );
            None
        }
    };

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Initialize application state (handles non-git directory, PR mode, and revision diffs cleanly)
    let mut app = App::new(repo_path, watcher_rx, agent_pane, pr_number, revision);
    if let Some(sink_str) = agent_sink {
        if let Ok(kind) = sink_str.parse() {
            app.agent_sink_kind = Some(kind);
        }
    }

    match start_audit {
        Some(AuditStartMode::FullRepo) => {
            app.trigger_repo_scan();
        }
        Some(AuditStartMode::DiffChanges) => {
            app.open_initial_diff_audit();
        }
        None => {}
    }

    let mut event_loop = event_loop::UiLoop::default();
    if _watcher.is_none() {
        event_loop.enable_polling_fallback();
    }
    let mut input = event_loop::TerminalInput;
    let mut diagnostics = lgtm_core::debug::enabled().then(diagnostics::UiLoop::default);
    while !app.should_quit {
        event_loop.step(&mut app, &mut input, &mut diagnostics, |app| {
            if app.needs_terminal_clear {
                terminal.clear()?;
                app.needs_terminal_clear = false;
            }
            lgtm_core::debug::measure("ui.draw", || terminal.draw(|f| ui::render(f, app)))?;
            Ok(())
        })?;
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
