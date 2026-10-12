mod app;
use lgtm_core::{
    comment, coverage, diff_source, git, guard, ipc, model, mutate, pr, scan, turn, verify, watcher,
};
use lgtm_mcp as mcp;
mod ui;

use anyhow::Result;
use app::App;
use clap::Parser;
use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;
use watcher::RepoWatcher;

mod cli;
mod headless;

use cli::{AuditStartMode, Cli, Commands};
use headless::{
    run_ci_audit, run_coverage_audit, run_git_passthrough, run_init_config, run_risk_audit,
};

fn run_check(
    target: PathBuf,
    format: &str,
    output: Option<&Path>,
    strict: bool,
    show_suppressed: bool,
    no_user_config: bool,
) -> ExitCode {
    let repo_path = std::fs::canonicalize(&target).unwrap_or(target);
    match run_ci_audit(
        &repo_path,
        format,
        output,
        strict,
        show_suppressed,
        no_user_config,
    ) {
        Ok(code) => ExitCode::from(code as u8),
        Err(e) => {
            eprintln!("Error running check audit: {}", e);
            ExitCode::from(1)
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

    if cli.init {
        let repo_path = std::fs::canonicalize(&cli.repo).unwrap_or(cli.repo);
        if let Err(e) = run_init_config(&repo_path) {
            eprintln!("Error initializing config: {}", e);
            return ExitCode::from(1);
        }
        return ExitCode::SUCCESS;
    }

    match cli.command {
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
            if first.contains("..") || first.starts_with("HEAD~") || first.starts_with("HEAD^") {
                let repo_path = std::fs::canonicalize(&cli.repo).unwrap_or(cli.repo);
                if let Err(e) = run_tui(
                    repo_path,
                    cli.agent_pane,
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
        Some(Commands::Prs) => match pr::PrEngine::list_pull_requests() {
            Ok(prs) => {
                println!("📋 Open Pull Requests ({} total):\n", prs.len());
                for pr in prs {
                    println!(
                            "  \x1b[1;34m#{}\x1b[0m {} \x1b[33mby @{}\x1b[0m (\x1b[32m+{}\x1b[0m/\x1b[31m-{}\x1b[0m) [\x1b[36m{}\x1b[0m]",
                            pr.number, pr.title, pr.author.login, pr.additions, pr.deletions, pr.head_ref_name
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
        }) => run_check(
            repo.unwrap_or(cli.repo),
            &format,
            output.as_deref(),
            strict,
            show_suppressed,
            no_user_config,
        ),
        // CI ignores the user-level config so results match across machines
        Some(Commands::Ci {
            repo,
            format,
            output,
            strict,
            show_suppressed,
            ..
        }) => run_check(
            repo.unwrap_or(cli.repo),
            &format,
            output.as_deref(),
            strict,
            show_suppressed,
            true,
        ),
        Some(Commands::Audit { repo }) => {
            let target = repo.unwrap_or(cli.repo);
            let repo_path = std::fs::canonicalize(&target).unwrap_or(target);
            if let Err(e) = run_tui(
                repo_path,
                cli.agent_pane,
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
            match scan::FullRepoScanner::scan_repository(&repo_path, &cfg) {
                Ok((files, alerts)) => {
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
        Some(Commands::Coverage { min, delta, lcov }) => match run_coverage_audit(min, delta, lcov)
        {
            Ok(code) => ExitCode::from(code as u8),
            Err(e) => {
                eprintln!("Error auditing test coverage: {}", e);
                ExitCode::from(1)
            }
        },
        Some(Commands::Mutate { min }) => match mutate::MutationTester::run_mutation_audit(min) {
            Ok(code) => ExitCode::from(code as u8),
            Err(e) => {
                eprintln!("Error running mutation audit: {}", e);
                ExitCode::from(1)
            }
        },
        Some(Commands::VerifyTests) => {
            println!("🔍 Running LGTM Inverted Test Verification Engine...\n");
            let repo_path = std::path::PathBuf::from(".");
            let git_engine = git::GitEngine::new(repo_path);
            match git_engine.compute_diff_snapshot() {
                Ok(diff) => match verify::InvertedTestVerifier::verify(&diff) {
                    Ok(res) => {
                        println!("{}", res.details);
                        if res.is_fraudulent {
                            ExitCode::from(1)
                        } else {
                            ExitCode::SUCCESS
                        }
                    }
                    Err(e) => {
                        eprintln!("Error running inverted test verification: {}", e);
                        ExitCode::from(1)
                    }
                },
                Err(e) => {
                    eprintln!("Error computing diff snapshot: {}", e);
                    ExitCode::from(1)
                }
            }
        }
        Some(Commands::Diff) | Some(Commands::Status) | Some(Commands::Review) | None => {
            let repo_path = std::fs::canonicalize(&cli.repo).unwrap_or(cli.repo);
            if let Err(e) = run_tui(repo_path, cli.agent_pane, cli.debounce, cli.pr, None, None) {
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
                "Warning: Failed to start file watcher: {}. Polling fallback will be used.",
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

    match start_audit {
        Some(AuditStartMode::FullRepo) => {
            app.trigger_repo_scan();
        }
        Some(AuditStartMode::DiffChanges) => {
            app.trigger_diff_scan();
        }
        None => {}
    }

    // Main event loop
    let tick_rate = Duration::from_millis(100);
    while !app.should_quit {
        // Drain any filesystem events
        app.handle_background_events();

        // Render UI
        terminal.draw(|f| ui::render(f, &mut app))?;

        // Poll user keyboard events
        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == crossterm::event::KeyEventKind::Press {
                    app.handle_key(key);
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
