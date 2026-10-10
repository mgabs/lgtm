# lgtm

> **Fast, real-time terminal diff reviewer & AI agent steering TUI built in Rust.**

`lgtm` is a keyboard-driven diff reviewer designed for human-in-the-loop AI workflows and terminal velocity. It combines Vim motion parity, live file-watcher reloading, AST & regex security guards, GitHub PR review, and seamless `#ref` injection into `tmux` agent panes or system clipboards.

---

## Features

- **Live Streaming Diffs**: Instant AST & streaming git patch parsing; automatically refreshes when files change on disk.
- **Vim Motion Engine**: Full `j`/`k`, `gg`/`G`, `{`/`}`, `]f`/`[f`, `]h`/`[h`, `]c`/`[c`, `]w`/`[w`, `]u`/`[u`, `zz`/`zt`/`zb`, `/` search, `*`/`#` symbol search, `y` yank, `PgUp`/`PgDn`.
- **Neovim-Style Ex Commands**: `:w`, `:q`, `:wq`, `:pr <num>`, `:prs`, `:pane`, `:files`, `:split`, `:risk`, `:<line_num>` with `Tab` / `Shift-Tab` wildmenu auto-completion.
- **Security & Test Guard**: Static analysis engine flagging SQLi, command injection, secret leakage, TLS bypass, disabled auth/CORS, and test neutering (deleted asserts, skipped tests, mock inflation).
- **GitHub PR Review Mode**: Review GitHub PRs directly (`lgtm pr <num>` or `:pr <num>`), write inline review directives, batch compile to `.lgtm/pr-<num>-review.md`, and submit via `gh pr review`.
- **Agent Steering & Directives**: Add inline comments (`c`), delete comments (`x` / `:d`), yank exact file references (`<Space>y` or `yr`), and blast directives to your agent's `tmux` pane (`Enter` or `Ctrl-s`).
- **Headless CI & Security Audits**: `lgtm risk` and `lgtm ci` output Markdown sticky tables, JSON, or SARIF 2.1.0 with automatic GitHub Actions annotations.
- **Zero-Crash Non-Git Fallback**: Graceful fallback and file diff scanning even outside git repositories.

---

## Installation & Build

```bash
cargo build --release
cp target/release/lgtm /usr/local/bin/lgtm
```

---

## Quick Start

```bash
# Launch interactive TUI on current working tree diff
lgtm

# Review a specific GitHub PR
lgtm pr 42

# List and pick open PRs
lgtm prs

# Headless security & fraud risk scan (exit 1 on critical/high)
lgtm risk

# Headless CI report (SARIF format for GitHub Security tab)
lgtm ci --format sarif --out lgtm-results.sarif

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
| `]f` / `[f` | Jump to next / previous file |
| `]h` / `[h` | Jump to next / previous hunk |
| `]c` / `[c` | Jump to next / previous comment / directive |
| `]w` / `[w` | Jump to next / previous security / test alert |
| `]u` / `[u` | Jump to next / previous unreviewed file |
| `zz` / `zt` / `zb` | Center / Top / Bottom cursor view |

### Search & Yank
| Key | Action |
| --- | --- |
| `/` | Interactive regex search |
| `n` / `N` | Next / previous search match |
| `*` / `#` | Search word under cursor forward / backward |
| `y` | Yank current code line to clipboard |
| `<Space>y` | Yank reference `#ref file:line: code` to clipboard |

### Actions & Review
| Key | Action |
| --- | --- |
| `c` | Add inline review directive / comment on current line |
| `x` / `<Space>d` | Remove review directive on current line |
| `C` / `:c` | Open directive / comment modal list (`d` to delete, `Enter` to jump) |
| `v` | Mark current file as reviewed (persisted in `.lgtm/state.json`) |
| `s` | Stage / unstage current file in git index |
| `X` | Discard working tree changes for current file (with confirmation) |
| `o` / `e` | Open current file and line in `$EDITOR` / `$VISUAL` (e.g. `nvim +line file`) |
| `Tab` | Toggle 35-column file drawer / sidebar |
| `Space` | Toggle split-view / unified diff view |
| `p` | Select target `tmux` pane for agent steering |
| `Enter` | Inject reference `#ref file:line` into active `tmux` pane |
| `Ctrl-s` | Compile and export all review directives to agent / `.lgtm/review.md` |
| `?` / `h` | Toggle help modal |
| `q` | Quit |

---

## Ex Commands (`:cmds`)

Press `:` in diff view to enter command mode with `Tab` / `Shift-Tab` wildmenu completion:

- `:w` — Save comments / directives to `.lgtm/comments.jsonl`
- `:q` / `:q!` — Quit
- `:wq` / `:x` — Save directives and quit
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

## Testing & Quality Gates

`lgtm` enforces strict test coverage ($\ge 90\%$) and automated mutation testing ($\ge 90\%$ mutation score):

```bash
# Run unit & integration test suite (39 tests)
make test

# Enforce code coverage (>=90% threshold gate)
make coverage

# Run automated mutation testing (12 mutant operators, >=90% kill gate)
make mutate

# Run full CI gate locally (fmt, lint, test, coverage, mutation, risk, sarif)
make check
```

---

## Makefile Targets

| Target | Description |
| --- | --- |
| `make build` | Compile debug binary |
| `make release` | Compile optimized release binary |
| `make test` | Run all unit & integration tests |
| `make coverage` | Generate llvm-cov summary & enforce $\ge 90\%$ line coverage |
| `make mutate` | Run mutation testing engine & enforce $\ge 90\%$ mutation kill score |
| `make lint` | Run `cargo clippy -- -D warnings` |
| `make fmt` | Check code formatting with `cargo fmt` |
| `make check` | Run complete test, lint, coverage, and mutation test suite |
| `make risk` | Run headless security and test fraud audit on git diff |
| `make ci` | Generate SARIF 2.1.0 security report (`lgtm-results.sarif`) |
| `make install` | Install binary into `~/.cargo/bin` |
| `make clean` | Clean build artifacts and coverage files |

---

## Architecture

```
lgtm/
├── src/
│   ├── main.rs         # CLI dispatcher, headless CI runner & subcommands
│   ├── app.rs          # State machine, key handlers & Vim motion engine
│   ├── ui/mod.rs       # Ratatui rendering, drawer, inline alerts & modals
│   ├── model.rs        # Diff data models (DiffSnapshot, FileDiff, DiffHunk, DiffLine)
│   ├── git/mod.rs      # Streaming git diff engine + non-git directory fallback
│   ├── pr/mod.rs       # GitHub PR engine via `gh` CLI & unified diff parser
│   ├── guard/mod.rs    # AST & regex security scanner & test fraud detector
│   ├── comment/mod.rs  # Directive storage, PR metadata & batch review compiler
│   ├── turn/mod.rs     # Turn manager & reviewed file tracker (.lgtm/state.json)
│   ├── ipc/mod.rs      # Tmux pane injector & OSC 52 clipboard fallback
│   └── mutate/mod.rs   # Mutation testing engine (12 AST/binary mutant operators)
├── scripts/
│   ├── coverage.sh     # Automated coverage enforcer script
│   └── mutate.sh       # Automated mutation test runner
├── .github/workflows/
│   └── ci.yml          # Multi-OS CI pipeline with SARIF upload & release matrix
├── Makefile            # All-in-one developer automation
└── Cargo.toml
```

---

## Roadmap & Upcoming Features

- [ ] **Interactive Hunk & Line Staging**: Stage (`s`), unstage (`u`), or discard (`x`) individual hunks and visual line selections (`V`) directly into the Git index.
- [ ] **Side-by-Side Split View & Intra-Line Word Diffing**: Parallel dual-pane rendering with token/character-level highlight for intra-line modifications.
- [ ] **Tree-Sitter Syntax Highlighting & Sticky Scope**: Multi-language syntax coloring and sticky header displaying the enclosing function/class/struct context.
- [ ] **MCP Server Mode (`lgtm serve --mcp`)**: Native Model Context Protocol (MCP) server over stdio for autonomous AI agents to query diffs, security alerts, and review directives programmatically.
- [ ] **Arbitrary Revision & Git Range Diffing**: Direct comparison of branch ranges (`lgtm main..feature`), commit revisions (`lgtm HEAD~3..HEAD`), and 3-way merge conflict resolution.
- [ ] **Quick Steering Directives**: 1-key canned directive templates (`ca`: test coverage, `cs`: security guard, `cr`: refactor/DRY, `cp`: performance).

---

## License

MIT


