use clap::Parser;
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

    /// Review a specific GitHub pull request by number (e.g. --pr 16)
    #[arg(long = "pr")]
    pub pr: Option<u64>,

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
    /// Initialize .lgtm/config.toml starter configuration
    Init {
        /// Target repository path
        #[arg(short, long, default_value = ".")]
        repo: Option<PathBuf>,
    },
    /// Review a specific GitHub Pull Request
    Pr {
        /// Pull Request number
        number: u64,
    },
    /// List open Pull Requests
    Prs,
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
    /// Audit test coverage across all modules or strictly on changed lines in diff
    Coverage {
        /// Minimum coverage percentage required to pass
        #[arg(short, long, default_value_t = 90.0)]
        min: f64,
        /// Calculate and enforce delta coverage strictly on added lines in diff
        #[arg(long)]
        delta: bool,
        /// Optional path to LCOV tracefile (e.g. lcov.info)
        #[arg(long)]
        lcov: Option<PathBuf>,
    },
    /// Run automated mutation testing suite and enforce mutation score threshold (default: 90%)
    Mutate {
        /// Minimum mutation score percentage required to pass
        #[arg(short, long, default_value_t = 90.0)]
        min: f64,
    },
    /// Run inverted test verification to detect fraudulent or tautological tests
    VerifyTests,
    /// Launch interactive diff reviewer TUI (default)
    Diff,
    /// Launch interactive diff reviewer TUI
    Status,
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
    /// Unrecognized git command passthrough
    #[command(external_subcommand)]
    External(Vec<String>),
}
