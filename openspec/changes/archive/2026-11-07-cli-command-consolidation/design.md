# Architecture & Technical Design

## 1. Overview & Invariants
- **Consolidated Entrypoint**: Bare `lgtm` and `lgtm [REVISION]` remain the primary interactive diff review entrypoint.
- **Strict Canonical Surface**: Exactly 11 canonical subcommands (`Gate`, `Config`, `Init`, `Pr`, `Check`, `Scan`, `Audit`, `Baseline`, `Review`, `Mcp`, `Hook`) plus `External` for git passthrough.
- **Invariant**: Deprecated aliases (`serve`, `prs`, `status`, `ci`, `diff`, `risk`) and duplicate top-level flags (`--init`, `--pr`) do not parse; Clap rejects them with standard error feedback.
- **Invariant**: `lgtm review` requires an explicit action (`export` or `import`); it cannot silently launch the TUI.

## 2. Domain Models, State & Interface Contracts

### CLI Schema (`src/cli.rs`)
```rust
#[derive(Parser, Debug)]
#[command(
    name = "lgtm",
    author,
    version,
    about = "Real-Time Terminal Diff Reviewer & Agent Steering Engine with Vim motions",
    allow_external_subcommands = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(short, long, default_value = ".")]
    pub repo: PathBuf,

    #[arg(short = 'p', long = "agent-pane")]
    pub agent_pane: Option<String>,

    #[arg(long = "agent-sink")]
    pub agent_sink: Option<String>,

    #[arg(long, global = true)]
    pub debug: bool,

    #[arg(long, global = true, requires = "debug", value_name = "PATH")]
    pub debug_log: Option<PathBuf>,

    #[arg(short, long, default_value_t = 300)]
    pub debounce: u64,
}

#[derive(clap::Subcommand, Debug)]
pub enum Commands {
    Gate { ... },
    Config { ... },
    Init {
        #[arg(short, long, default_value = ".")]
        repo: Option<PathBuf>,
    },
    Pr {
        #[command(subcommand)]
        action: Option<PrSubcommand>,
        #[arg(value_name = "NUMBER")]
        number: Option<u64>,
    },
    Check { ... },
    Scan { ... },
    Audit { ... },
    Baseline { ... },
    Review {
        #[arg(value_parser = ["export", "import"])]
        action: String,
        arg: Option<String>,
        #[arg(short, long)]
        repo: Option<PathBuf>,
    },
    Mcp { ... },
    Hook { ... },
    #[command(external_subcommand)]
    External(Vec<String>),
}
```

## 3. Architecture Diagram & Data Flow

```mermaid
flowchart TD
    A["CLI Invocation (argv)"] --> B["Clap Parser (Cli::try_parse)"]
    B -->|lgtm [REVISION]| C["Interactive TUI (run_tui)"]
    B -->|lgtm init| D["run_init_config"]
    B -->|lgtm check| E["run_check / run_ci_audit"]
    B -->|lgtm pr NUMBER/list| F["run_tui(pr) or list_prs_cli"]
    B -->|lgtm audit| G["run_tui(FullRepo)"]
    B -->|lgtm scan| H["FullRepoScanner"]
    B -->|lgtm mcp| I["McpServer::run_stdio"]
    B -->|lgtm review exp/imp| J["headless::run_review"]
    B -->|lgtm hook / baseline / config / gate| K["Headless Command Handlers"]
    B -->|serve / ci / status / prs / --init / --pr| L["Clap Error (Unknown Command)"]
```

## 4. Boundary Rules & Budgets
- **Pure Core vs. I/O Shell**: `lgtm-core` remains 100% pure logic with zero CLI or Clap dependencies. All argument parsing changes are isolated to `src/cli.rs` and top-level routing in `src/main.rs`.
- **Latency & Resource Budgets**: CLI parsing is a cold-start path; `Cli::try_parse` completes in <5ms. TUI hot-path keystroke budget (<1µs) and rendering budget (<16ms) are completely untouched.

## 5. Testing Strategy
1. **Unit Tests (`src/cli.rs`)**:
   - Verify `lgtm mcp` parses and `lgtm serve` is rejected.
   - Verify `lgtm init` parses with `--repo` and `lgtm --init` is rejected.
   - Verify `lgtm pr list` parses and `lgtm prs` is rejected.
   - Verify `lgtm check` parses and `lgtm ci` is rejected.
   - Verify `lgtm review` without action fails parsing.
2. **BDD Scenarios (`tests/bdd.rs`)**:
   - Update `bdd_cli_010` to invoke `lgtm init --repo <path>` and `lgtm init`.
   - Update `docs/bdd-scenarios.md` entry for `bdd_cli_010`.
3. **Quality Gates**:
   - `make check` (fmt check + clippy + test).
   - `make coverage` enforcing >= 90% workspace line coverage.

## Review Gate
- [x] **Gate 3 Approval**: Architecture & design reviewed and approved by human before task breakdown.
