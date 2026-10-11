# lgtm (Looks Good To Me)

[![CI](https://github.com/mgabs/lgtm/actions/workflows/ci.yml/badge.svg)](https://github.com/mgabs/lgtm/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/badge/coverage-95.6%25-brightgreen)](https://github.com/mgabs/lgtm)
[![Mutation Score](https://img.shields.io/badge/mutation_score-100%25-brightgreen)](https://github.com/mgabs/lgtm)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

> **Fast, real-time terminal diff reviewer, AI agent steering TUI & CI security scanner built in Rust.**
> *"Because code shouldn't ship until it actually looks good to you."*

`lgtm` is a keyboard-driven diff reviewer and security guardrail engine engineered for human-in-the-loop AI coding workflows and terminal velocity. It pairs Vim motion parity with live file-watcher updates, deep static analysis (complexity spikes, typo/hallucination checks, dead code, breaking API drift), AST/hunk folding, an interactive Commit Forge, GitHub PR review, and direct `#ref` injection into `tmux` agent panes or system clipboards.

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

## Features

- **Live Streaming Diffs**: Instant AST & streaming git patch parsing; automatically refreshes when files change on disk.
- **Vim Motion Engine**: Full `j`/`k`, `gg`/`G`, `{`/`}`, `]f`/`[f`, `]h`/`[h`, `]c`/`[c`, `]w`/`[w`, `]u`/`[u`, `zz`/`zt`/`zb`, `/` search, `*`/`#` symbol search, `y` yank, `PgUp`/`PgDn`.
- **Git Revision & Range Diffing**: Direct comparison of branch ranges (`lgtm main..feat`, `lgtm HEAD~3..HEAD`), commit hashes, or in-app via `:diff <range>`.
- **Native MCP Server Mode (`lgtm serve --mcp` / `lgtm mcp`)**: Standard JSON-RPC Model Context Protocol (MCP) server over stdio for autonomous AI agents (Claude Code, Cursor, Windsurf, Aider) to query diffs, run security scans, and manage review directives programmatically.
- **Quick Steering Directive Chords**: 1-key canned directive templates:
  - `ca`: ⚡ Test Coverage (`Add unit/regression tests covering these branches.`)
  - `cs`: 🔒 Security (`Ensure inputs are sanitized, parameterized, and auth checks enforced.`)
  - `cr`: 🧹 Refactor / DRY (`Simplify logic, reduce cognitive nesting, eliminate duplication.`)
  - `cp`: 🚀 Performance (`Optimize hot-path allocations and database query batching.`)
  - `cw`: ❓ Rationale / Why (`Explain rationale and verify architectural invariants.`)
- **Sticky Scope Header**: Real-time display of the enclosing function, class, or struct scope (`📍 Scope: fn ...`) in the diff title bar.
- **Side-by-Side Split View & Intra-Line Word Diffing**: Parallel dual-pane rendering with token/character-level highlight for intra-line modifications.
- **Interactive Hunk & Line Staging**: Stage (`s`), unstage (`u`), or discard (`x`) individual hunks and visual line selections (`V`) directly into the Git index.
- **Deep Static Analysis Guardrails**:
  - **Cognitive Complexity Spikes**: Flags functions where nested branching/loops increase cognitive complexity by $\ge 5$.
  - **Identifier Hallucination / Typo Guard**: Flags 1-2 character Levenshtein distance typos against repository symbol index.
  - **Dead Function Addition (YAGNI)**: Identifies newly added functions with 0 usages across the project.
  - **Breaking Public API Drift**: Detects changes to `pub` signatures, struct fields, or trait definitions.
  - **Security & Test Guard**: Flags SQLi, command injection, hardcoded secrets, TLS/auth bypass, and test neutering (deleted asserts, skipped tests, mock inflation).
- **Syntax & Hunk Folding**: Vim-style folding (`zc`, `zo`, `za`, `zM`, `zR`, `zm`, `zr`) to collapse or expand unchanged lines, clean files, or reviewed hunks.
- **Interactive Commit Forge (`C` / `:commit`)**: Craft conventional commits (`feat:`, `fix:`, `refactor:`, etc.) directly from staged hunks with automated diff-aware commit message synthesis.
- **Tabbed Interactive Help Modal (`?` / `h`)**: 7 discrete documentation tabs covering Overview & Modes, Motions, Directives, Staging & Forge, Folding, Guardrails, and Ex-Commands & CI.
- **Neovim-Style Ex Commands**: `:w`, `:q`, `:wq`, `:commit`, `:diff <rev>`, `:pr <num>`, `:prs`, `:pane`, `:files`, `:split`, `:risk`, `:<line_num>` with `Tab` / `Shift-Tab` wildmenu auto-completion.
- **GitHub PR Review Mode**: Review GitHub PRs directly (`lgtm pr <num>` or `:pr <num>`), write inline review directives, batch compile to `.lgtm/pr-<num>-review.md`, and submit via `gh pr review`.
- **Interactive Codebase Audit Mode (`lgtm scan repo` / `lgtm audit` / `:scan`)**: Dedicated dual-pane TUI reviewing repository-wide security and architectural issues without diff noise. Features 1-key severity filtering (`1`: Critical, `2`: High+, `3`: All), `$EDITOR` jumping (`e`/`o`), directive injection into agents (`Enter`), and instant report export (`w` / `:export`).
- **Headless CI & Security Gates**:
  - `lgtm check`: Pre-commit / CI gate that blocks builds on critical security vulnerabilities or complexity spikes.
  - `lgtm scan`: Generates full Markdown audit reports or SARIF 2.1.0 output for GitHub Security Code Scanning.
- **Zero-Crash Non-Git Fallback**: Graceful fallback and file diff scanning even outside git repositories.

---

## Installation

### Via Homebrew (macOS & Linux)
```bash
brew install mgabs/tap/lgtm
```
*Note: New releases automatically update `mgabs/homebrew-tap` with verified SHA-256 binary tarballs for Apple Silicon (`arm64`), Intel (`x86_64`), and Linux (`x86_64`).*

### From Source
```bash
cargo install --git https://github.com/mgabs/lgtm.git
# or clone & build release binary
cargo build --release
cp target/release/lgtm /usr/local/bin/lgtm
```

---

## Quick Start

```bash
# Launch interactive TUI on current working tree diff
lgtm

# Launch interactive Codebase Audit Mode (only issues, no diff view)
lgtm scan repo
# or: lgtm scan --tui / lgtm audit

# Review a specific GitHub PR
lgtm pr 42

# List and pick open PRs
lgtm prs

# Run CI security & quality gate (exit code 1 on critical security / test neutering)
lgtm check --strict

# Run full codebase audit and generate Markdown report
lgtm scan --output lgtm-report.md

# Headless SARIF 2.1.0 export for GitHub Actions Code Scanning
lgtm scan --format sarif --output lgtm-results.sarif

# Git passthrough (all unrecognized flags/subcommands route to git)
lgtm log -p
lgtm commit -m "feat: core diff engine"
```

---

## Keybindings Reference

### Navigation
| Key | Action |
| --- | --- |
| `j` / `k` / `Down` / `Up` | Move cursor down / up |
| `gg` / `G` | Jump to top / bottom of diff |
| `0` / `^` / `$` | Jump to start / first char / end of diff lines |
| `{` / `}` | Jump to previous / next paragraph (blank lines / hunk headers) |
| `Ctrl-d` / `Ctrl-u` | Scroll half-page down / up |
| `Ctrl-f` / `Ctrl-b` / `PgDn` / `PgUp` | Scroll full-page down / up |
| `]f` / `[f` or `]` / `[` | Jump to next / previous file |
| `]h` / `[h` | Jump to next / previous hunk |
| `]c` / `[c` | Jump to next / previous comment / directive |
| `]w` / `[w` | Jump to next / previous security / quality alert |
| `]u` / `[u` | Jump to next / previous unreviewed file |
| `zz` / `zt` / `zb` | Center / Top / Bottom cursor view |

### Search & Yank
| Key | Action |
| --- | --- |
| `/` | Interactive regex search |
| `n` / `N` | Next / previous search match |
| `*` / `#` | Search word under cursor forward / backward |
| `y` | Yank current code line to clipboard |
| `<Space>y` / `yr` | Yank reference `#ref file:line: code` to clipboard |

### Hunk & Code Folding
| Key | Action |
| --- | --- |
| `zc` | Close / fold current hunk |
| `zo` | Open / unfold current hunk |
| `za` | Toggle fold state of current hunk |
| `zM` / `zm` | Fold all hunks across file |
| `zR` / `zr` | Unfold all hunks across file |

### Actions, Review & Commit Forge
| Key | Action |
| --- | --- |
| `s` | Stage current hunk under cursor into git index |
| `u` | Unstage current hunk under cursor from git index |
| `S` | Stage entire current file |
| `U` | Unstage entire current file |
| `C` / `:commit` | Open interactive **Commit Forge** modal |
| `<Space>x` | Discard changes in current hunk |
| `X` | Discard all working tree changes for current file |
| `c` | Add inline review directive / comment on current line |
| `x` / `<Space>d` | Remove review directive on current line |
| `v` | Mark current file as reviewed (persisted in `.lgtm/state.json`) |
| `o` / `e` | Open current file and line in `$EDITOR` / `$VISUAL` (e.g. `nvim +line file`) |
| `Tab` | Toggle 35-column file drawer / sidebar |
| `d` | Toggle split-view / unified diff view |
| `p` | Select target `tmux` pane for agent steering |
| `Enter` | Inject reference `#ref file:line` into active `tmux` pane |
| `Ctrl-s` | Compile and export all review directives to agent / `.lgtm/review.md` |
| `?` / `h` | Toggle multi-tab help modal (`1`-`6` or `[`/`]` to switch tabs) |
| `q` | Quit |

---

## Ex Commands (`:cmds`)

Press `:` in diff view to enter command mode with `Tab` / `Shift-Tab` wildmenu completion:

- `:w` — Save comments / directives to `.lgtm/comments.jsonl`
- `:q` / `:q!` — Quit
- `:wq` / `:x` — Save directives and quit
- `:commit` / `:cforge` — Open Commit Forge modal
- `:pr <number>` — Load and review GitHub PR diff
- `:prs` — Open interactive PR picker
- `:f` / `:files` — Open fuzzy file finder
- `:pane` / `:panes` — Open `tmux` target pane selector
- `:sp` / `:split` — Toggle split / unified view
- `:tree` / `:drawer` — Toggle file tree drawer
- `:risk` / `:audit` — Re-scan current diff for security & test risks
- `:r` / `:review` — Compile directives into batch review file and blast to agent
- `:e` / `:reload` — Reload diff from disk
- `:clear` — Clear all review directives
- `:d` / `:del` — Delete directive on current line
- `:<line_number>` — Jump directly to line number (e.g. `:42`)

---

## CI/CD Integration & Quality Gates

### GitHub Actions Workflow
Add `lgtm check` and `lgtm scan` directly into your CI pipeline:

```yaml
name: LGTM Security & Quality Gate

on:
  pull_request:
    branches: [ main ]

jobs:
  lgtm-gate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Install lgtm
        run: cargo install --git https://github.com/mgabs/lgtm.git

      - name: Run CI Gate
        run: lgtm check --strict

      - name: Generate Security Scan Report
        run: lgtm scan --format sarif --output lgtm-results.sarif

      - name: Upload SARIF to GitHub Security Tab
        uses: github/codeql-action/upload-sarif@v3
        if: always()
        with:
          sarif_file: lgtm-results.sarif
```

---

## Testing & Quality Gates

`lgtm` enforces strict test coverage ($\ge 90\%$) and automated mutation testing ($\ge 90\%$ mutation score):

```bash
# Run unit & integration test suite (70+ tests)
make test

# Enforce code coverage (>=90% threshold gate)
make coverage

# Run automated mutation testing (12 mutant operators, >=90% kill gate)
make mutate

# Run full CI gate locally (fmt, lint, test, coverage, mutation, risk, sarif)
make check
```

---

## Architecture

```
lgtm/
├── src/
│   ├── main.rs               # CLI dispatcher, headless CI runner & subcommands
│   ├── app.rs                # State machine, key handlers & Vim motion engine
│   ├── ui/mod.rs             # Ratatui rendering, drawer, tabs, commit forge & modals
│   ├── model.rs              # Diff data models (DiffSnapshot, FileDiff, DiffHunk, DiffLine)
│   ├── git/mod.rs            # Streaming git diff engine + non-git directory fallback
│   ├── pr/mod.rs             # GitHub PR engine via `gh` CLI & unified diff parser
│   ├── scan.rs               # Full repository audit scanner
│   ├── guard/
│   │   ├── mod.rs            # Guard aggregator, severities & ignore directives
│   │   ├── security.rs       # Entropy secrets, OWASP sinks, auth bypass
│   │   ├── complexity.rs     # Cognitive complexity spike analyzer
│   │   ├── symbols.rs        # Identifier typo & symbol hallucination detector
│   │   ├── api_drift.rs      # Public API signature drift detector
│   │   ├── dead_code.rs      # Dead function / unused addition detector
│   │   ├── arch.rs           # Layer boundary, stray file & blast radius
│   │   ├── deps.rs           # Unpinned / typosquatted dependencies
│   │   └── duplicate.rs      # Duplicate helper anti-pattern
│   ├── comment/mod.rs        # Directive storage, PR metadata & batch review compiler
│   ├── turn/mod.rs           # Turn manager & reviewed file tracker (.lgtm/state.json)
│   ├── ipc/mod.rs            # Tmux pane injector & OSC 52 clipboard fallback
│   ├── watcher/mod.rs        # Filesystem watcher & debouncer
│   ├── mcp/mod.rs            # MCP JSON-RPC stdio server
│   ├── coverage/mod.rs       # LCOV parsing & delta coverage
│   ├── verify/mod.rs         # Inverted test verification
│   └── mutate/mod.rs         # Mutation testing engine (12 AST/binary mutant operators)
├── scripts/
│   ├── coverage.sh           # Automated coverage enforcer script
│   └── mutate.sh             # Automated mutation test runner
├── .github/workflows/
│   └── ci.yml                # Multi-OS CI pipeline with SARIF upload & release matrix
├── Makefile                  # All-in-one developer automation
└── Cargo.toml
```

---

## Documentation & Wiki

For full detailed guides on human-in-the-loop steering, MCP server configuration, AST folding, staging, and static guardrails:
👉 **[Read the Full LGTM Wiki & User Manual](docs/WIKI.md)**

---

## License

MIT
