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
- **Vim Motion Engine**: Full `j`/`k`, `gg`/`G`, `{`/`}`, `]f`/`[f`, `]h`/`[h`, `]c`/`[c`, `]t`/`[t` (PR threads), `]w`/`[w`, `]u`/`[u`, `zz`/`zt`/`zb`, `/` search, `*`/`#` symbol search, `y` yank, `PgUp`/`PgDn`.
- **Git Revision & Range Diffing**: Direct comparison of branch ranges (`lgtm main..feat`, `lgtm HEAD~3..HEAD`), commit hashes, or in-app via `:diff <range>`.
- **Human-in-the-Loop Breakpoint Gate (`lgtm gate`)**: Agent pre-tool / pre-commit hook halting autonomous execution until reviewed and approved via `A` / `:approve` or `:reject-all` in TUI.
- **Reject Hunk & Re-Prompt (`X` / `:reject`)**: 1-key action discarding flawed hunk from working copy and sending prompt with directive/alert context directly to the agent's pane.
- **Context-Aware Directive Injection (`Enter`)**: Passes `#ref file:line [rule_id] <alert> | DIRECTIVE: <text>` to target agent pane or system clipboard, or bare line ref if no alert/directive exists.
- **Agent Session / Prompt Correlation (`P` / `:prompt`)**: Split modal displaying agent prompt context and instructions next to diffs from `.lgtm/gate.json`, `.lgtm/prompt.txt`, or commit history.
- **Test Authenticity & Mutation Check (`T` / `lgtm verify-tests`)**: Automated tautological test verification (`assert_eq!(x, x)`, `toBe(x)`, `assertEquals(x, x)`) and in-app mutation testing.
- **Native MCP Server & Subscriptions (`lgtm serve --mcp` / `lgtm mcp`)**: Standard JSON-RPC MCP server with live resource read/subscribe (`lgtm://directives`, `lgtm://diff`, `lgtm://findings`, `lgtm://review-state`) and directive claim tracking.
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
  - **Security & Test Guard**: Flags SQLi, command injection, hardcoded secrets, TLS/auth bypass, and test neutering (deleted asserts, skipped tests, mock inflation, self-assertions).
- **Syntax & Hunk Folding**: Vim-style folding (`zc`, `zo`, `za`, `zM`, `zR`, `zm`, `zr`) to collapse or expand unchanged lines, clean files, or reviewed hunks.
- **Interactive Commit Forge (`C` / `:commit`)**: Craft conventional commits (`feat:`, `fix:`, `refactor:`, etc.) directly from staged hunks with automated diff-aware commit message synthesis.
- **Tabbed Interactive Help Modal (`?` / `h`)**: 7 discrete documentation tabs covering Overview & Modes, Motions, Directives, Staging & Forge, Folding, Guardrails, and Ex-Commands & CI.
- **Neovim-Style Ex Commands**: `:w`, `:q`, `:wq`, `:commit`, `:diff <rev>`, `:pr <num>`, `:prs`, `:pane`, `:files`, `:split`, `:risk`, `:approve`, `:reject`, `:prompt`, `:<line_num>` with `Tab` / `Shift-Tab` wildmenu auto-completion.
- **PR Review Mode**: Review GitHub PRs (`gh`) or Azure DevOps PRs (`az`: list, diff, policies, threads, post, reply, resolve, vote) with `lgtm pr <num>` or `:pr <num>`, write inline review directives, batch compile to `.lgtm/pr-<num>-review.md`, and submit via `gh pr review` on GitHub.
- **CI step**: `lgtm ci --rev origin/main...HEAD` gates a committed range (exit 0 pass, 1 blocking findings, 2 tool error); a composite GitHub Action (`mgabs/lgtm`) adds verified install, SARIF upload and a job summary.
- **Orphaned comments**: comments re-pin when code moves and are flagged `⚠ Orphaned` when their code is gone, never deleted.
- **Interactive Codebase Audit Mode (`lgtm scan repo` / `lgtm audit` / `:scan`)**: Dedicated dual-pane TUI reviewing repository-wide security and architectural issues without diff noise. Features 1-key severity filtering (`1`: Critical, `2`: High+, `3`: All), `$EDITOR` jumping (`e`/`o`), directive injection into agents (`Enter`), and instant report export (`w` / `:export`).
- **Headless CI & Security Gates**:
  - `lgtm check`: Pre-commit / CI gate that blocks builds on critical security vulnerabilities or complexity spikes.
  - `lgtm gate`: Blocks autonomous agents until human approves or rejects via TUI.
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
| `X` / `:reject` | Reject & revert hunk under cursor; re-prompt agent with directive/alert context |
| `A` / `:approve` | Approve waiting agent breakpoint gate (`.lgtm/gate.json`) |
| `P` / `:prompt` | View agent prompt context modal from gate, `.lgtm/prompt.txt`, or commit history |
| `T` | Run mutation verification probe across test files |
| `c` | Add inline review directive / comment on current line |
| `x` / `<Space>d` | Remove review directive on current line |
| `v` | Mark current file as reviewed (persisted in `.lgtm/state.json`) |
| `o` / `e` | Open current file and line in `$EDITOR` / `$VISUAL` (e.g. `nvim +line file`) |
| `Tab` | Toggle 35-column file drawer / sidebar |
| `d` | Toggle split-view / unified diff view |
| `p` | Select target `tmux` pane for agent steering |
| `Enter` | Inject reference `#ref file:line [rule] <alert> | DIRECTIVE: <text>` into active `tmux` pane |
| `Ctrl-s` | Compile and export all review directives to agent / `.lgtm/review.md` |
| `?` / `h` | Toggle multi-tab help modal (`1`-`7` or `[`/`]` to switch tabs) |
| `q` | Quit |

---

## Ex Commands (`:cmds`)

Press `:` in diff view to enter command mode with `Tab` / `Shift-Tab` wildmenu completion:

- `:w` — Save comments / directives to `.lgtm/comments.jsonl`
- `:q` / `:q!` — Quit
- `:wq` / `:x` — Save directives and quit
- `:approve` / `:lgtm` — Approve pending agent breakpoint gate
- `:reject` — Reject current hunk and dispatch prompt to agent
- `:reject-all` — Reject waiting agent breakpoint gate
- `:prompt` / `:context` — Open agent prompt and instruction context modal
- `:commit` / `:cforge` — Open Commit Forge modal
- `:pr <number>` — Load and review a GitHub or Azure DevOps PR diff
- `:prs` — Open interactive PR picker
- `:checks` / `:threads` — PR CI status / list PR threads (gutter `◆`/`◇`, `]t`/`[t`)
- `:directives orphans` — List orphaned comments; `repin <n>` or `drop --yes`
- `:f` / `:files` — Open fuzzy file finder
- `:pane` / `:panes` — Open `tmux` target pane selector
- `:sp` / `:split` — Toggle split / unified view
- `:tree` / `:drawer` — Toggle file tree drawer
- `:risk` — Re-scan current diff for security & test risks
- `:audit` — Audit the whole codebase (not just the diff)
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
├── src/                      # `lgtm` binary: TUI + CLI
│   ├── main.rs               # CLI dispatcher
│   ├── cli.rs, headless.rs   # CLI definitions, headless runners & subcommands
│   ├── app/                  # State machine, key handlers & Vim motion engine
│   └── ui/                   # Ratatui rendering, drawer, tabs, commit forge & modals
├── crates/
│   ├── lgtm-ast/src/         # tree-sitter parsing and queries
│   ├── lgtm-core/src/        # UI-free engine
│   │   ├── model.rs          # Diff data models
│   │   ├── git/, pr/         # Git diff engine; PR providers (`forge/`: gh, az)
│   │   ├── scan.rs           # Full repository audit scanner
│   │   ├── guard/            # Security, complexity, symbols, API drift, dead code, arch, deps, duplicate
│   │   ├── comment/, turn/   # Directives; reviewed-file tracker (.lgtm/state.json)
│   │   ├── ipc/              # Tmux injector, OSC 52 clipboard, AgentSink trait
│   │   ├── diff_source.rs    # DiffSource trait
│   │   ├── watcher/          # Filesystem watcher & debouncer
│   │   └── coverage/, verify/, mutate/  # Quality gates
│   └── lgtm-mcp/src/lib.rs   # MCP JSON-RPC stdio server
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

For full detailed guides on human-in-the-loop steering, MCP server configuration, hunk folding, staging, and static guardrails:
👉 **[Read the Full LGTM Wiki & User Manual](docs/WIKI.md)**

---

## Donating & Sponsorship

LGTM is free and always will be. If you want to show appreciation for this project, please give instead to humanitarian relief for Gaza and Palestine. These organisations deliver food, water, shelter, and medical care on the ground:

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

MIT

