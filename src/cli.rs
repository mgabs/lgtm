use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "lgtm",
    author,
    version,
    about = "Real-Time Terminal Diff Reviewer & Agent Steering Engine with Vim motions",
    allow_external_subcommands = true
)]
pub struct Cli {
    /// Subcommand or git passthrough (e.g. risk, init, pr, diff, status, log, commit)
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Target repository path (default: current directory)
    #[arg(short, long, default_value = ".")]
    pub repo: PathBuf,

    /// Explicit target multiplexer pane ID (e.g. %1, %2)
    #[arg(short = 'p', long = "agent-pane")]
    pub agent_pane: Option<String>,

    /// Multiplexer agent steering sink: tmux, zellij, wezterm, kitty, clipboard, dummy (default: auto)
    #[arg(long = "agent-sink")]
    pub agent_sink: Option<String>,

    /// Review an open or closed GitHub or Azure DevOps PR by number
    #[arg(long = "pr")]
    pub pr: Option<u64>,

    /// Enable timing diagnostics in the system temporary directory (lgtm-debug.jsonl)
    #[arg(long, global = true)]
    pub debug: bool,

    /// Override the timing log path (requires --debug)
    #[arg(long, global = true, requires = "debug", value_name = "PATH")]
    pub debug_log: Option<PathBuf>,

    /// Watcher debounce interval in milliseconds
    #[arg(short, long, default_value_t = 300)]
    pub debounce: u64,

    /// Initialize configuration file
    #[arg(long = "init")]
    pub init: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditStartMode {
    FullRepo,
    DiffChanges,
}

#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    /// Wait for human review approval in TUI before agent proceeds
    Gate {
        /// Target repository path
        #[arg(short, long, default_value = ".")]
        repo: Option<PathBuf>,
        /// Timeout in seconds to wait for approval (default: 300)
        #[arg(long, default_value_t = 300)]
        timeout: u64,
        /// Optional message / prompt describing the agent's changes
        #[arg(short, long)]
        message: Option<String>,
    },
    /// Audit current diff for security risks, test neutering, and fraud
    Risk {
        /// Target repository path
        #[arg(short, long, default_value = ".")]
        repo: Option<PathBuf>,
    },
    /// Show effective `.lgtm.toml` configuration with value sources
    Config {
        /// Print effective values (default action)
        #[arg(long)]
        show: bool,
        /// Target repository path
        #[arg(short, long)]
        repo: Option<PathBuf>,
        /// Ignore ~/.config/lgtm/config.toml
        #[arg(long)]
        no_user_config: bool,
    },
    /// Initialize .lgtm.toml starter configuration
    Init {
        /// Target repository path
        #[arg(short, long, default_value = ".")]
        repo: Option<PathBuf>,
    },
    /// Review an open or closed GitHub or Azure DevOps pull request
    ///
    /// Public GitHub.com patches can fall back to anonymous HTTPS when gh is missing
    /// or needs login and no API token is set. Private access and writes need authentication.
    Pr {
        #[command(subcommand)]
        action: Option<PrSubcommand>,

        /// Pull Request number
        #[arg(value_name = "NUMBER")]
        number: Option<u64>,
    },
    /// List pull requests by state (closed includes merged/completed and abandoned)
    ///
    /// Public GitHub.com listings can fall back to anonymous HTTPS when gh is missing
    /// or needs login and no API token is set. Anonymous requests have lower rate limits.
    Prs {
        #[arg(long, default_value = "open", value_parser = ["open", "closed", "all"])]
        state: String,
    },
    /// Run headless CI / pre-commit check (fails on Critical/High risks)
    Check {
        /// Target repository path
        #[arg(short, long)]
        repo: Option<PathBuf>,
        /// Output format: markdown, json, or sarif
        #[arg(short, long, default_value = "markdown")]
        format: String,
        /// Optional output file to write the report to
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Fail on warning severity as well as critical/high
        #[arg(long)]
        strict: bool,
        /// List suppressed findings on stderr
        #[arg(long)]
        show_suppressed: bool,
        /// Ignore ~/.config/lgtm/config.toml
        #[arg(long)]
        no_user_config: bool,
        /// Check a revision diff instead of the working tree (e.g. origin/main...HEAD)
        #[arg(long)]
        rev: Option<String>,
    },
    /// Run full repository security and architecture scan with markdown report
    Scan {
        /// Target repository path
        #[arg(short, long)]
        repo: Option<PathBuf>,
        /// Optional output file to write the report to (default: lgtm-report.md)
        #[arg(short, long, default_value = "lgtm-report.md")]
        output: Option<PathBuf>,
        /// Output format: markdown or json
        #[arg(short, long, default_value = "markdown")]
        format: String,
        /// Open interactive TUI audit mode to inspect repository issues
        #[arg(short = 't', long = "tui")]
        tui: bool,
        /// Optional target (e.g. `lgtm scan repo`)
        #[arg(value_name = "TARGET")]
        target: Option<String>,
    },
    /// Interactive repository audit mode (shows only repository issues, no diff view)
    Audit {
        /// Target repository path
        #[arg(short, long)]
        repo: Option<PathBuf>,
    },
    /// Run headless CI mode: audit diff and produce sticky PR comment markdown or JSON/SARIF report
    Ci {
        /// Target repository path
        #[arg(short, long)]
        repo: Option<PathBuf>,
        /// Output format: markdown, json, or sarif
        #[arg(short, long, default_value = "markdown")]
        format: String,
        /// Optional output file to write the report to
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Fail on warning severity as well as critical/high
        #[arg(long)]
        strict: bool,
        /// List suppressed findings on stderr
        #[arg(long)]
        show_suppressed: bool,
        /// Ignore ~/.config/lgtm/config.toml
        #[arg(long)]
        no_user_config: bool,
        /// Check a revision diff instead of the working tree (e.g. origin/main...HEAD)
        #[arg(long)]
        rev: Option<String>,
    },
    /// Manage the findings baseline (`.lgtm-baseline.json`)
    Baseline {
        /// Action to run
        #[arg(value_parser = ["update"])]
        action: String,
        /// Drop baseline entries that no longer match a finding
        #[arg(long)]
        prune: bool,
        /// Target repository path
        #[arg(short, long)]
        repo: Option<PathBuf>,
    },
    /// Launch the TUI, or share review state: `review export [target]` prints JSON, `review import <file>` merges it
    Review {
        /// Optional action; without one the TUI starts
        #[arg(value_parser = ["export", "import"])]
        action: Option<String>,
        /// Export: review target (default `working`). Import: JSON file to merge
        arg: Option<String>,
        /// Target repository path
        #[arg(short, long)]
        repo: Option<PathBuf>,
    },
    /// Launch interactive diff reviewer TUI (default)
    Diff {
        /// Optional branch or revision to review (e.g. main, feat..HEAD)
        revision: Option<String>,
    },
    /// Launch interactive diff reviewer TUI
    Status {
        /// Optional branch or revision to review (e.g. main, feat..HEAD)
        revision: Option<String>,
    },
    /// Launch Model Context Protocol (MCP) server over stdio for AI coding agents
    Mcp {
        /// Target repository path
        #[arg(short, long, default_value = ".")]
        repo: Option<PathBuf>,
    },
    /// Run background server (e.g. lgtm serve --mcp)
    Serve {
        /// Run in MCP stdio mode
        #[arg(long)]
        mcp: bool,
        /// Target repository path
        #[arg(short, long, default_value = ".")]
        repo: Option<PathBuf>,
    },
    /// Manage Git pre-commit hooks (install, uninstall, status)
    Hook {
        /// Hook action: install, uninstall, or status
        #[arg(value_parser = ["install", "uninstall", "status"])]
        action: String,
        /// Target repository path
        #[arg(short, long)]
        repo: Option<PathBuf>,
    },
    /// Unrecognized git command passthrough
    #[command(external_subcommand)]
    External(Vec<String>),
}

#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum PrSubcommand {
    /// Post pending local comments to the PR
    Post {
        /// Pull request number (auto-detected from branch/comments if omitted)
        #[arg(value_name = "PR")]
        pr: Option<u64>,

        /// Confirm posting without prompt
        #[arg(long)]
        yes: bool,
    },
    /// Reply to an existing review thread
    Reply {
        /// Pull request number
        pr: u64,
        /// Review thread id
        thread: String,
        /// Comment reply message
        body: String,
    },
    /// Resolve or unresolve a review thread
    Resolve {
        /// Pull request number
        pr: u64,
        /// Review thread id
        thread: String,
        /// Mark as unresolved instead of resolved
        #[arg(long)]
        unresolve: bool,
    },
    /// List pull requests by state (open, closed, all)
    List {
        #[arg(long, default_value = "open", value_parser = ["open", "closed", "all"])]
        state: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_is_opt_in_and_global() {
        let cli = Cli::try_parse_from(["lgtm", "pr", "42"]).unwrap();
        assert!(!cli.debug);
        assert!(cli.debug_log.is_none());
        for args in [
            vec!["lgtm", "--debug", "pr", "42"],
            vec!["lgtm", "pr", "42", "--debug"],
        ] {
            assert!(Cli::try_parse_from(args).unwrap().debug);
        }
        let cli = Cli::try_parse_from([
            "lgtm",
            "pr",
            "42",
            "--debug",
            "--debug-log",
            "/tmp/review.jsonl",
        ])
        .unwrap();
        assert_eq!(cli.debug_log.unwrap(), PathBuf::from("/tmp/review.jsonl"));
        assert!(Cli::try_parse_from(["lgtm", "--debug-log", "/tmp/review.jsonl"]).is_err());
    }
    #[test]
    fn pr_listing_accepts_open_closed_and_all() {
        for state in ["open", "closed", "all"] {
            let cli = Cli::try_parse_from(["lgtm", "prs", "--state", state]).unwrap();
            assert!(
                matches!(cli.command, Some(Commands::Prs { state: actual }) if actual == state)
            );
        }
        assert!(Cli::try_parse_from(["lgtm", "prs", "--state", "invalid"]).is_err());
    }

    #[test]
    fn pr_commands_parse_correctly() {
        let cli = Cli::try_parse_from(["lgtm", "pr", "42"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Pr {
                action: None,
                number: Some(42)
            })
        ));

        let cli = Cli::try_parse_from(["lgtm", "pr", "post"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Pr {
                action: Some(PrSubcommand::Post {
                    pr: None,
                    yes: false
                }),
                number: None
            })
        ));

        let cli = Cli::try_parse_from(["lgtm", "pr", "post", "42", "--yes"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Pr {
                action: Some(PrSubcommand::Post {
                    pr: Some(42),
                    yes: true
                }),
                number: None
            })
        ));

        let cli = Cli::try_parse_from(["lgtm", "pr", "reply", "42", "T1", "looks good"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Pr {
                action: Some(PrSubcommand::Reply {
                    pr: 42,
                    ref thread,
                    ref body
                }),
                number: None
            }) if thread == "T1" && body == "looks good"
        ));

        let cli =
            Cli::try_parse_from(["lgtm", "pr", "resolve", "42", "T1", "--unresolve"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Pr {
                action: Some(PrSubcommand::Resolve {
                    pr: 42,
                    ref thread,
                    unresolve: true
                }),
                number: None
            }) if thread == "T1"
        ));
        let cli = Cli::try_parse_from(["lgtm", "pr", "list", "--state", "closed"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Pr {
                action: Some(PrSubcommand::List { ref state }),
                number: None
            }) if state == "closed"
        ));
    }

    #[test]
    fn test_cli_deprecated_commands_parse() {
        let cli = Cli::try_parse_from(["lgtm", "ci", "--strict"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Ci { strict: true, .. })
        ));

        let cli = Cli::try_parse_from(["lgtm", "status"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Status { revision: None })
        ));

        let cli = Cli::try_parse_from(["lgtm", "serve", "--mcp"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Serve { mcp: true, .. })
        ));
    }
}
