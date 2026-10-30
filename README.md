# lgtm (Looks Good To Me)

[![CI](https://github.com/mgabs/lgtm/actions/workflows/ci.yml/badge.svg)](https://github.com/mgabs/lgtm/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

> **Fast, real-time terminal diff reviewer, AI agent steering TUI & CI security scanner built in Rust.**
> *"Because code shouldn't ship until it actually looks good to you."*

`lgtm` is a keyboard-driven diff reviewer and deterministic guardrail engine engineered for human-in-the-loop AI coding workflows and terminal velocity. It combines Vim motion parity with live file-watcher updates, deep static analysis (complexity spikes, typo/hallucination checks, dead code, breaking API drift), AST hunk folding, an interactive Commit Forge, GitHub & Azure DevOps PR review, and direct `#ref` injection into `tmux`, `zellij`, `wezterm`, `kitty` agent panes or clipboards.

```
┌── Files (35 cols) ─────────┬── Diff: src/guard/mod.rs ────────────────────────┐
│ [●] src/app.rs        +12 -4│ @@ -40,12 +40,24 @@                             │
│ [x] src/git/mod.rs     +8 -1│-    let query = format!("SELECT * FROM users    │
│ [!] src/guard/mod.rs  +45 -2│+    let query = sqlx::query!("SELECT * FROM ... │
│                             │ 🚨 [CRITICAL] SQL Injection vulnerability detected│
│                             │ ⚡ [COMPLEXITY] Function complexity jumped +8 (>5)│
│                             │ 💬 [DIRECTIVE] Use parameter binding here       │
└─────────────────────────────┴─────────────────────────────────────────────────┘
│ :e<Tab> [edit] [export] [errors]                               NORMAL | 12:4  │
└───────────────────────────────────────────────────────────────────────────────┘
```

---

## Quick Start

### Installation

```bash
# macOS & Linux (Homebrew)
brew install mgabs/tap/lgtm

# Windows, macOS & Linux (Cargo from GitHub)
cargo install --git https://github.com/mgabs/lgtm.git

# Windows (Pre-compiled zip)
# Download lgtm-v*-windows-x86_64.zip from GitHub Releases and extract to PATH.
```

### 30-Second Cheat Sheet

```bash
lgtm                  # Launch interactive TUI on current working tree diff
lgtm pr 42            # Review GitHub or Azure DevOps PR #42
lgtm pr list          # List open PRs (or --state closed / all; alias: lgtm prs)
lgtm audit            # Launch interactive Codebase Audit TUI (findings only; alias: lgtm scan repo)
lgtm check --strict   # Run pre-commit / CI security gate (exit 1 on block)
lgtm scan -o rep.sarif --format sarif  # Headless SARIF export for CI
lgtm mcp              # Run native Model Context Protocol (MCP) server (alias: lgtm serve --mcp)
```

---

## Core Capabilities

- **Vim Motion Parity & Dual-Pane Diffing**: Full `j`/`k`, `]f`/`[f` (files), `]h`/`[h` (hunks), `]c`/`[c` (directives), `]w`/`[w` (alerts), side-by-side split view (`d`), search (`/`, `*`, `#`), and AST-aware folding (`zc`, `zo`, `za`, `zM`).
- **Interactive Staging & Commit Forge**: Stage (`s`), unstage (`u`), or discard (`x`) individual hunks or visual line selections (`V`) straight to the Git index; craft conventional commits (`C` / `:commit`).
- **AI Agent Steering & Chords**: Drop inline directives (`c`), use 2-key chord templates (`ca` coverage, `cs` security, `cr` refactor, `cp` performance, `cw` why), and dispatch context-aware `#ref` links directly into agent multiplexer panes (`Enter`) or reject hunks (`X`).
- **Deep Static Analysis Guardrails**: Real-time detection of OWASP vulnerabilities, cognitive complexity spikes ($\ge 5$), hallucinated identifier typos, dead function additions (YAGNI), breaking API drift, and stealth test neutering.
- **Pull Request Review**: Review open/closed PRs from GitHub (`gh`/API/HTTPS fallback) and Azure DevOps (`az`/REST) with CI checks (`:checks`), remote discussion threads (`:threads`, markers `◆`/`◇`).
- **Native MCP Server**: JSON-RPC stdio server for Claude Code, Cursor, Windsurf, and Aider with live file-watcher resource subscriptions.
- **Headless CI & Security Gates**: `lgtm check` gates CI pipelines on critical findings; `lgtm hook` automates Git pre-commit verification; SARIF 2.1.0 and Markdown reporting.

---

## Documentation & Wiki

Comprehensive user guides, configuration specs, and reference manuals are maintained in the [**LGTM Wiki**](docs/WIKI.md):

- 📖 [**LGTM Wiki & User Manual**](docs/WIKI.md)
  - 🚀 [**Installation & Setup**](docs/WIKI.md#2-installation--setup) — Homebrew, Cargo, source compilation, release flags.
  - 🎮 [**Interactive TUI User Guide**](docs/WIKI.md#3-interactive-tui-user-guide)
    - [Motions & Navigation](docs/WIKI.md#motions--navigation) — 1-key navigation, file jumps, scrolling, searches.
    - [Interactive Hunk & Visual Line Staging](docs/WIKI.md#interactive-hunk--visual-line-staging) — Granular Git index staging and visual ranges (`V`).
    - [Syntax-Aware AST & Hunk Folding](docs/WIKI.md#syntax-aware-ast--hunk-folding) — Tree-sitter block and hunk folding (`zc`, `zo`, `zM`).
    - [Inline Directives & AI Agent Steering](docs/WIKI.md#inline-directives--ai-agent-steering) — `#ref` anchors, chords (`ca`, `cs`, `cr`), tmux/zellij/wezterm/kitty injection.
    - [Interactive Commit Forge](docs/WIKI.md#interactive-commit-forge) — Automated diff synthesis, conventional commit templates.
  - 🛡️ [**Deep Static Analysis & Guardrails**](docs/WIKI.md#4-deep-static-analysis--guardrails)
    - [Cognitive Complexity Spikes](docs/WIKI.md#cognitive-complexity-spikes) — Early-exit enforcement and nesting detection.
    - [Identifier Hallucination & Typo Guard](docs/WIKI.md#identifier-hallucination--typo-guard) — Levenshtein distance against repository symbols.
    - [Dead Function Addition (YAGNI)](docs/WIKI.md#dead-function-addition-yagni) — Unused new function addition detection.
    - [Breaking Public API Drift](docs/WIKI.md#breaking-public-api-drift) — `pub` signature modifications and interface stability.
    - [OWASP Security & Hardcoded Secrets](docs/WIKI.md#owasp-security--hardcoded-secrets) — SQLi, RCE, SSRF, Shannon entropy secret scanning ($H \ge 4.3$).
    - [Test Neutering Detection](docs/WIKI.md#test-neutering--assertion-drop-detection) — Dropped assertions, test skips, and tautological mocks.
  - 🔀 [**Git Revisions & Pull Request Reviews**](docs/WIKI.md#5-git-range-revision--pull-request-diffing)
    - [CLI & In-App Revision Diffing](docs/WIKI.md#cli-usage) — Branch comparisons, commit hashes, `:diff <rev>`.
    - [GitHub & Azure DevOps PR Reviews](docs/WIKI.md#pull-request-reviews-github--azure-devops) — Automatic remote detection, CLI/REST authentication.
    - [Open & Closed PR State Filtering](docs/WIKI.md#open-and-closed-prs) — Filtering by state, historical commit loading, anonymous HTTPS fallback.
    - [Discussion Threads & Gutter Markers](docs/WIKI.md#pr-threads-and-remote-discussions) — Remote discussions (`:threads`), markers `◆`/`◇`, `]t`/`[t` navigation, inline reply/resolve.
    - [Directive Outcomes & Remote Posting](docs/WIKI.md#directive-outcomes-and-remote-posting) — Tracking outcomes (`untouched`, `changed`, `gone`), batch posting.
    - [Durable Review State & Re-pinning](docs/WIKI.md#durable-review-state-and-comment-re-pinning) — Hunk marks (`M`), file review (`v`), orphan recovery.
  - 🤖 [**Model Context Protocol (MCP) Server**](docs/WIKI.md#6-model-context-protocol-mcp-server)
    - [Agent Client Setup](docs/WIKI.md#client-configuration) — Configuration snippets for Claude Code, Cursor, Windsurf, Aider.
    - [MCP Tools & Live Resources](docs/WIKI.md#available-mcp-tools) — Directives, diffs, security scans, claims, and live watcher subscriptions.
  - 🚦 [**Headless CI & Security Gates**](docs/WIKI.md#7-headless-ci--security-gates)
    - [Quality & Agent Gates](docs/WIKI.md#1-lgtm-check-ci-quality-gate) — `lgtm check`, `lgtm gate`, `lgtm scan`.
    - [Suppression, Thresholds & Baselines](docs/WIKI.md#suppression-thresholds--baselines) — Inline `lgtm:ignore`, `.lgtm-baseline.json`.
    - [Git Pre-commit Hook Integration](docs/WIKI.md#git-pre-commit-hook-integration) — `lgtm hook [install|status|uninstall]`.
    - [CI Pipeline Recipes](docs/WIKI.md#ci-recipes) — GitHub Actions composite action & workflows, GitLab CI, Azure Pipelines.
  - ⌨️ [**Ex-Commands Dictionary (`:cmds`)**](docs/WIKI.md#8-ex-commands-dictionary-cmds) — Full reference for command mode and wildmenu completion.
  - ⚙️ [**Configuration Reference (`.lgtm.toml`)**](docs/WIKI.md#9-configuration-reference-lgtmtoml) — Rule settings, thresholds, sinks, ignored paths.
  - 💻 [**CLI Command Reference**](docs/WIKI.md#10-cli-command-reference) — All subcommands and CLI options.
  - 📊 [**Diagnostics & Performance Debugging**](docs/WIKI.md#14-diagnostics-workload-benchmarking--performance-debugging) — JSONL timing logs, UI event budgets, benchmark suites.
  - 🧪 [**BDD Acceptance Scenario Catalog**](docs/bdd-scenarios.md) — 41 executable acceptance criteria and pass rate reports.

---

## Testing & Quality Gates

Run the test suite and acceptance scenarios:

```bash
# Unit, integration, and module behavior tests
make test

# Public API and application-launch BDD scenarios
cargo test --workspace --test bdd

# Automated BDD scenario catalog report
make bdd-report

# Repository quality gates (formatting, clippy, boundary checks)
make check
```

The [BDD scenario catalog](docs/bdd-scenarios.md) maps acceptance criteria to tests, including draft preservation, historical PR retrieval, posting recovery across restarts, ambiguous remote write reconciliation, real-terminal journeys, and GitHub/Azure request/response contracts.

---

## Architecture

```
lgtm/
├── src/                      # `lgtm` binary: TUI + CLI
│   ├── main.rs               # Entry point
│   ├── cli.rs, headless.rs   # CLI definitions & headless runners
│   ├── diagnostics.rs        # Opt-in JSONL timing & UI workload diagnostics
│   ├── event_loop.rs         # Bounded terminal/watcher/callback event loop
│   ├── app/                  # State machine & background coordinators
│   └── ui/                   # Ratatui rendering, viewport caches & modals
├── tests/bdd.rs              # Acceptance scenarios launching the application
├── crates/
│   ├── lgtm-ast/             # Tree-sitter AST queries & folding (TS/JS, Swift, Python, Go, Java)
│   ├── lgtm-core/            # Diff model, guards, scanner, forge (GitHub/Azure), review store
│   └── lgtm-mcp/             # MCP JSON-RPC stdio server & live resource subscriptions
├── scripts/                  # BDD reporting, boundary linter, tap updater
└── Makefile                  # Developer automation
```

---

## Donating & Humanitarian Support

LGTM is free and open-source. If you wish to express gratitude, please consider donating to humanitarian relief organizations providing food, water, medical aid, and shelter to families in Gaza and Palestine:

- **[UNRWA](https://www.unrwa.org)** — UN Relief and Works Agency for Palestine Refugees
- **[Palestine Children's Relief Fund (PCRF)](https://www.pcrf.net)**
- **[Palestine Red Crescent Society](https://www.palestinercs.org)**
- **[Medical Aid for Palestinians (MAP)](https://www.map.org.uk)**
- **[Anera](https://www.anera.org)**
- **[Médecins Sans Frontières (MSF)](https://www.msf.org)**
- **[International Committee of the Red Cross (ICRC)](https://www.icrc.org)**
- **[UNICEF](https://www.unicef.org)**
- **[World Food Programme (WFP)](https://www.wfp.org)**

---

## License

[MIT](LICENSE)
