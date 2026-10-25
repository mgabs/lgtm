# LGTM (Looks Good To Me) — Comprehensive Wiki & User Manual

> **Real-Time Terminal Diff Reviewer, AI Agent Steering Platform & Deterministic Security Guardrail Engine.**
> *"Because code shouldn't ship until it actually looks good to you."*

---

## Table of Contents

1. [Purpose & Architectural Philosophy](#1-purpose--architectural-philosophy)
2. [Installation & Setup](#2-installation--setup)
3. [Interactive TUI User Guide](#3-interactive-tui-user-guide)
   - [Motions & Navigation](#motions--navigation)
   - [Interactive Hunk & Visual Line Staging](#interactive-hunk--visual-line-staging)
   - [Syntax-Aware AST & Hunk Folding](#syntax-aware-ast--hunk-folding)
   - [Inline Directives & AI Agent Steering](#inline-directives--ai-agent-steering)
   - [Interactive Commit Forge](#interactive-commit-forge)
4. [Deep Static Analysis & Guardrails](#4-deep-static-analysis--guardrails)
   - [Cognitive Complexity Spikes](#cognitive-complexity-spikes)
   - [Identifier Hallucination & Typo Guard](#identifier-hallucination--typo-guard)
   - [Dead Function Addition (YAGNI)](#dead-function-addition-yagni)
   - [Breaking Public API Drift](#breaking-public-api-drift)
   - [OWASP Security & Hardcoded Secrets](#owasp-security--hardcoded-secrets)
   - [Test Neutering & Assertion Drop Detection](#test-neutering--assertion-drop-detection)
5. [Git Range & Revision Diffing](#5-git-range--revision-diffing)
6. [Model Context Protocol (MCP) Server](#6-model-context-protocol-mcp-server)
7. [Headless CI & Security Gates](#7-headless-ci--security-gates)
8. [Ex-Commands Dictionary (`:cmds`)](#8-ex-commands-dictionary-cmds)
9. [Configuration Reference (`.lgtm.toml`)](#9-configuration-reference-lgtmtoml)
10. [CLI Command Reference](#10-cli-command-reference)
11. [Additional Guardrails](#11-additional-guardrails)
12. [Quality Tooling (`make` targets)](#12-quality-tooling-make-targets)
13. [Source Module Map](#13-source-module-map)

---

## 1. Purpose & Architectural Philosophy

### Why LGTM Exists
Modern software development has fundamentally shifted with the advent of autonomous AI coding agents (Claude Code, Cursor Composer, Windsurf, Aider, GitHub Copilot). While agents generate code at superhuman speeds, they introduce novel, insidious failure modes that traditional code review tools (like browser PR interfaces or standard `git diff`) fail to capture in real time:

1. **Stealth Test Neutering**: Agents quietly delete assertions, add `@pytest.mark.skip`, or introduce tautological tests (`assert True`) when encountering test failures.
2. **Blast Radius & Architectural Sprawl**: Agents modify 8+ unrelated files, import backend database sinks into UI layers, or add redundant helper functions that already exist elsewhere in the codebase.
3. **Identifier Typo Hallucinations**: Agents invent slightly misspelled variants of existing variables or methods (e.g. `user_token` when the project uses `usertoken`), leading to runtime `AttributeError` / `NameError` bugs.
4. **Cognitive Complexity Spikes**: Agents generate deeply nested conditionals (5+ indent levels) instead of clean, early-exit guard clauses.
5. **Accidental Credential & Secret Leaks**: Agents hardcode fallback API tokens, private keys, or `.env` entries during fast prototyping.

### The Human-in-the-Loop Steering Paradigm
`lgtm` bridges the speed of terminal-native developers and autonomous agents. Instead of waiting for a completed PR to review in a slow browser, `lgtm` runs locally alongside your agent in `tmux` or terminal split:
- **Instant AST feedback**: As the agent saves files on disk, `lgtm` immediately analyzes the diff and surfaces guardrail alerts.
- **Micro-steering via `#ref` Anchors**: Select any line, press `c` to drop a directive, and press `Enter` to inject the reference directly into the agent's active `tmux` pane.
- **Granular Staging**: Stage verified hunks (`s`), fold clean boilerplate (`zc`), discard suspicious changes (`x`), and synthesize conventional commits (`C`) without context-switching.

```
┌───────────────────────────────────────────────────────────────────────────────┐
│                           LGTM SYSTEM ARCHITECTURE                            │
└───────────────────────────────────────────────────────────────────────────────┘
                                       │
            ┌──────────────────────────┴──────────────────────────┐
            ▼                                                     ▼
   [ Interactive TUI ]                                   [ Headless CI / MCP ]
  - Ratatui / Crossterm                                 - `lgtm check` (Gate)
  - 1-Key Motions (j/k, ], [)                           - `lgtm scan` (Audit MD)
  - Hunk Staging & Visual Mode                          - `lgtm serve --mcp` (Stdio)
  - Interactive Commit Forge                            - SARIF 2.1.0 Export
            │                                                     │
            └──────────────────────────┬──────────────────────────┘
                                       │
                                       ▼
                     [ Deterministic Guardrail Engine ]
                    ├── Shannon Entropy (Secrets: H >= 4.3)
                    ├── OWASP Security Sniffer (SQLi, RCE, SSRF)
                    ├── Cognitive Complexity Calculator (>= +5 spike)
                    ├── Symbol Table Levenshtein Matcher (Typo guard)
                    ├── YAGNI Dead Function Tracker (0 usages)
                    ├── Public API Drift Detector (pub signatures)
                    └── Test Neutering & Inverted Test Verifier
```

---

## 2. Installation & Setup

### Via Homebrew (macOS & Linux)
```bash
brew install mgabs/tap/lgtm
```

> **Note on Homebrew Tap Updates**:
> LGTM's CI/CD pipeline automatically updates the formula in `mgabs/homebrew-tap` with verified SHA-256 checksums on each release. Binaries are available for macOS (`arm64`, `x86_64`) and Linux (`x86_64`).

### From Source (Cargo)
```bash
# Install directly from GitHub
cargo install --git https://github.com/mgabs/lgtm.git

# Or clone and build optimized release binary
git clone https://github.com/mgabs/lgtm.git
cd lgtm
make release
cp target/release/lgtm /usr/local/bin/lgtm
```

### Requirements
- **OS**: macOS, Linux, Windows (via WSL2 or Windows Terminal)
- **Git**: Git 2.20+ installed
- **PR features**: GitHub uses Git/API/`gh`; public listings and stored patches can fall back to anonymous HTTPS via `curl`. Private reads, discussions, and writes require authentication. Azure uses `az` with the `azure-devops` extension.
- **Terminal**: Modern UTF-8 terminal with 256-color or TrueColor support (Ghostty, iTerm2, Alacritty, Kitty, WezTerm, Tmux)

---

## 3. Interactive TUI User Guide

Launch `lgtm` inside any repository:
```bash
lgtm
```

### Motions & Navigation

| Key | Action | Description |
|---|---|---|
| `j` / `k` or `↓` / `↑` | Line Cursor | Move cursor down / up across diff lines |
| `]` / `[` | **File Navigation** | **Single tap immediately jumps to Next / Previous changed file** |
| `]h` / `[h` | Hunk Navigation | Jump to next / previous diff hunk |
| `]w` / `[w` or `w` / `W` | Alert Jump | Jump to next / previous security / quality guardrail alert |
| `]c` / `[c` | Directive Jump | Jump to next / previous inline review comment |
| `]t` / `[t` | Thread Jump | Jump to next / previous remote PR thread line in the file (after `:threads`) |
| `gg` / `G` | Top / Bottom | Jump to top / bottom of current file diff |
| `Ctrl-d` / `Ctrl-u` | Half-Page | Scroll half-page down / up (15 lines) |
| `Ctrl-f` / `Ctrl-b` | Full-Page | Scroll full page down / up (25 lines) |
| `}` / `{` | Paragraph | Jump to next / previous blank line / section header |
| `/` | Regex Search | Interactive regex search across all diff lines (`n` / `N` to cycle) |
| `*` / `#` | Word Search | Search symbol under cursor forward / backward |
| `zz` / `zt` / `zb` | Viewport | Center cursor / align top / align bottom |
| `f` / `Ctrl-p` | Fuzzy Finder | Open fuzzy file switcher popup to jump to any file |
| `Tab` | Drawer Sidebar | Toggle 35-column changed file list on the left |
| `d` | Split View Toggle | Toggle between Unified Diff and Side-by-Side Dual Pane Diff |
| `o` / `e` | Open in Editor | Open current file and line in configured editor (default: `nvim`, configurable: `:editor <name>` or config `editor = "vscode"`) |
| `?` / `h` | **Help Modal** | Open 7-tab interactive documentation modal |
| `q` | Quit | Exit `lgtm` |

---

### Interactive Hunk & Visual Line Staging

`lgtm` includes a complete staging engine directly connected to your Git index:

- **Stage Hunk (`s`)**: Stages the entire diff hunk under the cursor into the git index.
- **Unstage Hunk (`u`)**: Unstages the hunk under the cursor from the git index.
- **Stage File (`S`)**: Stages all hunks in the current file into the git index.
- **Unstage File (`U`)**: Unstages all hunks in the current file from the git index.
- **Discard Hunk (`x` / `<Space>x`)**: Restores working tree file to index state for current hunk.
- **Discard File (`X`)**: Discards all uncommitted changes in current file (`git restore`).
- **Visual Line Selection Mode (`V`)**:
  1. Press `V` on any diff line to enter Visual Mode.
  2. Move cursor with `j` / `k` to highlight an arbitrary range of lines.
  3. Press `s` to stage only highlighted lines into git.
  4. Press `u` to unstage only highlighted lines.
  5. Press `x` to discard only highlighted lines.
  6. Press `Esc` or `V` to exit visual mode.

---

### Syntax-Aware AST & Hunk Folding

When reviewing large pull requests or AI-generated changes, noise files (lockfiles, generated schema, boilerplate) distract from core logic. Use Vim-style folding with AST block awareness to collapse and isolate diffs:

| Fold Key | Action |
|---|---|
| `za` | Toggle AST block fold (function, class, inner loop/block) or hunk under cursor |
| `zc` | Close (fold) innermost AST block or hunk under cursor |
| `zo` | Open (unfold) AST block or hunk under cursor |
| `zM` / `zm` | Fold **ALL** hunks and blocks across the diff |
| `zR` / `zr` | Unfold **ALL** hunks and blocks across the diff |

Folds use a visible-rows indirection for cursor motion (`j`, `k`, `G`, `gg`, `Ctrl-d`, `Ctrl-u`), ensuring smooth navigation that hops directly over folded ranges without getting stuck. Block folds persist across sessions in the review store (`.lgtm/reviews/<target>.json`).

---

### Inline Directives & AI Agent Steering

`lgtm` enables precise human-in-the-loop steering by attaching anchored directives directly to code lines:

#### 1. Adding Directives
- Press `c` on any line to open the directive input modal.
- Type your feedback (e.g. `Use parameter binding here rather than string concatenation`) and press `Enter`.
- Directives are immediately persisted to `.lgtm/comments.jsonl`.

#### 2. Quick Steering Chords (2-Key Templates)
To minimize typing during fast pairing sessions, use instant directive chords:
- **`ca`**: ⚡ Test Coverage (`Add unit/regression tests covering these branches.`)
- **`cs`**: 🔒 Security (`Ensure inputs are sanitized, parameterized, and auth checks enforced.`)
- **`cr`**: 🧹 Refactor / DRY (`Simplify logic, reduce cognitive nesting, eliminate duplication.`)
- **`cp`**: 🚀 Performance (`Optimize hot-path allocations and database query batching.`)
- **`cw`**: ❓ Rationale / Why (`Explain rationale and verify architectural invariants.`)

#### 3. Injecting Directives to AI Agent Panes
- **`Enter`**: Context-aware agent injection. If the cursor line has an active security/quality alert or human directive, formats and dispatches `#ref <file>:<line> [<rule_id>] <alert> | DIRECTIVE: <text>`. If neither is present, sends the clean `#ref <file>:<line>`.
- **`X` / `:reject`**: 1-key reject & re-prompt. Reverts the current hunk from the working tree and dispatches `REVERTED hunk in <file>:<line>, reason: <directive/alert>` to your agent's pane or clipboard.
- **`Ctrl-s` / `:review`**: Compiles all active directives into a structured batch prompt and blasts it into the active agent pane (`tmux`, `zellij`, `wezterm`, `kitty`) or copies it to clipboard via OSC 52.
- **`p` / `<Space>p`**: Opens pane selector modal for current terminal multiplexer to choose which pane receives steering messages.
- **`<Space>y` / `yr`**: Yanks `#ref <file>:<line>` to system clipboard.
- **`v` / `R`**: Marks a directive as resolved (`✓ RESOLVED`).

#### 4. Human-in-the-Loop Breakpoint Gate (`lgtm gate`)
Agents often run tools in loops, modify multiple files, and produce diffs without pausing for human verification.
- **Agent Hook**: Agents call `lgtm gate [--timeout 300] [--message "Reviewing core auth logic"]` before running destructive tools or committing.
- This creates `.lgtm/gate.json` and halts the agent until a human acts.
- **TUI Indicator**: The header displays a bright `🛑 GATE WAITING (A/:approve)` indicator.
- **Approval (`A` / `:approve` / `:lgtm`)**: Writes `"status": "approved"` to the gate file, allowing the agent to proceed.
- **Rejection (`:reject-all`)**: Writes `"status": "rejected"`, signaling the agent to halt or course-correct.

#### 5. Agent Prompt & Session Correlation (`P` / `:prompt`)
- Press **`P`** (or **`:prompt`** / **`:context`**) to open a dedicated modal displaying the exact instructions the agent was executing.
- Sources checked automatically:
  1. Active `.lgtm/gate.json` prompt message.
  2. Workspace `.lgtm/prompt.txt` session log.
  3. Git repository HEAD commit message.
- Press `Esc`, `q`, or `P` to return to the diff.

#### 6. In-App Test Risk Summary (`T`)
- Press **`T`** to summarize `test/` alerts already present in the current review.
- Test guards identify tautological assertions, constant truth evaluations, and deleted assertions. This key reports those findings; it does not execute tests or mutate source.

---

### Interactive Commit Forge

Once you have staged your verified hunks (`s` / `S`), press **`C`** (or type **`:commit`**) to open the **Commit Forge**:

1. `lgtm` inspects your staged diff and **auto-synthesizes Conventional Commit titles**:
   - `feat(guard): add cognitive complexity analyzer`
   - `fix(git): resolve hunk index offset calculation`
   - `refactor(ui): extract status line paragraph widget`
2. Edit the title or add an optional extended commit body.
3. Press `Enter` to commit directly to git.
4. `lgtm` immediately refreshes the working tree diff snapshot.

---

## 4. Deep Static Analysis & Guardrails

`lgtm` runs deterministic static analysis and AST inspections across diff snapshots in real time:

### Cognitive Complexity Spikes
- **What it checks**: Measures nested `if`, `match`, `for`, `while`, and boolean chains (`&&`, `||`) added in new hunks.
- **Trigger**: Warns if a function's cognitive complexity increases by $\ge +5$ points.
- **Remediation**: Extract sub-functions or refactor into early return guard clauses.

### AST-based web rules
Identifier typo detection (`typo/identifier`) reads real identifiers from the AST, so strings and comments are ignored. Cognitive complexity (`cx/complexity-spike`) also uses the AST for TS/JS/Swift hunks (branch cost = 1 + nesting, `else if` adds no depth); other languages keep the text heuristic.

`lgtm-ast` parses JS/TS/JSX/TSX with tree-sitter and guards query syntax nodes instead of text. Only matches on added lines are reported. Context-aware rules (like `"use client"` leaks) leverage whole-file directive detection. Rules (added lines only):

| Rule | Severity | Flags |
|---|---|---|
| `web/dangerous-html` | High | `dangerouslySetInnerHTML` |
| `web/inner-html` | High | `innerHTML` / `outerHTML` assignment |
| `web/eval` | Critical | `eval(...)` |
| `next/public-secret` | Critical | `process.env.NEXT_PUBLIC_*` with SECRET/TOKEN/PASSWORD/PRIVATE/API_KEY |
| `web/hook-conditional` | High | `useX()` inside if/loop/ternary (within one function) |
| `web/ts-escape` | Warn | `@ts-ignore` / `@ts-nocheck` |
| `next/client-server-leak` | High | Server-only modules or private env read in `"use client"` files |
| `next/action-unvalidated` | High | Next.js Server Action (`"use server"`) without validation |

Swift (`.swift`, same engine):

| Rule | Severity | Flags |
|---|---|---|
| `swift/force-unwrap` | Warn | `x!` |
| `swift/try-bang` | High | `try!` |
| `swift/force-cast` | Warn | `as!` |
| `swift/weak-crypto` | High | `CC_MD5`, `CC_SHA1`, `Insecure.MD5/SHA1` |
| `swift/secret-in-userdefaults` | High | `UserDefaults ... forKey: "…token/password/secret/apikey…"` |
| `test/xctskip` | Warn | `XCTSkip` |
| `swift/empty-catch` | Warn | `catch { }` |

Python (`.py` / `.pyi`, same engine):

| Rule | Severity | Flags |
|---|---|---|
| `python/eval-exec` | Critical | Dynamic code execution (`eval(...)`, `exec(...)`) |
| `python/shell-injection` | Critical | Subprocess invocation with `shell = True` |
| `python/unsafe-pickle` | Critical | Unsafe deserialization via `pickle.load` or `pickle.loads` |
| `python/bare-except` | Warn | Bare `except:` catching system exits and `KeyboardInterrupt` |

Go (`.go`, same engine):

| Rule | Severity | Flags |
|---|---|---|
| `go/sql-concat` | Critical | SQL query execution with `fmt.Sprintf` / concatenation |
| `go/command-injection` | Critical | `exec.Command` invoked with shell binary (`sh`, `bash`, `zsh`) |
| `go/unhandled-error` | Warn | Ignored error assigned to blank identifier (`_`) |

Java (`.java`, same engine):

| Rule | Severity | Flags |
|---|---|---|
| `java/sql-injection` | Critical | Raw SQL in `executeQuery` / `executeUpdate` instead of PreparedStatement |
| `java/command-injection` | Critical | Invoking `Runtime.getRuntime().exec(...)` with raw external commands |
| `java/unsafe-deserialization` | High | Remote code execution hazard via `ObjectInputStream.readObject()` |
| `java/empty-catch` | Warn | Empty catch block swallowing exceptions without logging or rethrow |

### Identifier Hallucination & Typo Guard
- **What it checks**: Extracts project-wide symbol identifiers (variable names, functions, structs, methods).
- **Trigger**: Flags new identifiers that have a Levenshtein edit distance of $1 \le d \le 2$ from existing symbols (e.g. agent typed `user_token` when the project defines `usertoken`).
- **Remediation**: Verify symbol spelling against existing codebase conventions.

### Dead Function Addition (YAGNI)
- **What it checks**: Scans newly declared helper functions, private methods, and utility structs.
- **Trigger**: Flags any added non-public function that has 0 references/call sites across the repository.
- **Remediation**: Eliminate dead code or wire up the intended call site.

### Breaking Public API Drift
- **What it checks**: Monitors exported signatures (`pub fn`, `export function`, public trait methods).
- **Trigger**: Flags changes to argument counts, parameter types, or return signatures that break backward compatibility.
- **Remediation**: Maintain deprecation shims or update all downstream callers.

### OWASP Security & Hardcoded Secrets
- **Shannon Entropy Secrets**: Sniffs high-entropy strings ($H \ge 4.3$), OpenAI tokens (`sk-proj-...`), GitHub tokens (`ghp_...`), AWS keys (`AKIA...`), and RSA private keys.
- **Injection Sinks**: Detects raw SQL interpolation (`SELECT ... {user_input}`), command execution (`os.system`, `exec`, `spawn`), path traversal (`../`), and SSRF sinks.
- **Insecure Defaults**: Flags CORS `*`, `verify=False` in HTTP clients, and disabled authentication decorators.

### Test Neutering & Assertion Drop Detection
- **Assertion Drop**: Detects net loss of `assert`, `expect()`, or `assertEquals` statements inside test files (`test_*.py`, `*_test.go`, `*.spec.ts`, `tests/*.rs`).
- **Test Skips**: Flags `@pytest.mark.skip`, `t.Skip()`, or commented-out test blocks.
- **Tautological Tests**: Flags meaningless tests like `assert True` or `expect(true).toBe(true)`.

---

## 5. Git Range & Revision Diffing

`lgtm` supports arbitrary Git revision ranges from the CLI and inside the TUI:

### CLI Usage
```bash
# Compare two branches
lgtm main..feature-auth

# Compare three-dot merge base
lgtm main...feature-auth

# Review the last 3 commits
lgtm HEAD~3..HEAD

# Review a specific commit hash
lgtm 0c20203

# Review a GitHub Pull Request
lgtm pr 42
```

### In-App Revision Switching
Type `:` in diff view to change comparison targets dynamically:
- `:diff main..feature`
- `:diff main` (a branch name): what the current branch changed since it diverged from `main` (same as `main...HEAD`); the diff is audited and the alert count shown. Uncommitted changes are not included.
- `:rev HEAD~2..HEAD`
- `:pr 104`
- `:reload` (or `:e`) to re-scan current target from disk

---

## 6. Model Context Protocol (MCP) Server

`lgtm` provides a native JSON-RPC Model Context Protocol (MCP) server over standard I/O (`stdio`). This allows AI agents (Claude Code, Cursor, Windsurf, Aider) to query diffs, run security audits, and read review directives programmatically.

### Starting the Server
```bash
lgtm serve --mcp
# or shorthand
lgtm mcp
```

### Configuration with Claude Code / Cursor / Windsurf / Aider

#### Claude Code (`~/.claude/claude_desktop_config.json` or project `.mcp.json`)
```json
{
  "mcpServers": {
    "lgtm": {
      "command": "lgtm",
      "args": ["mcp", "--repo", "."]
    }
  }
}
```

#### Cursor (`.cursor/mcp.json`)
```json
{
  "mcpServers": {
    "lgtm": {
      "command": "lgtm",
      "args": ["serve", "--mcp"]
    }
  }
}
```

#### Windsurf (`~/.codeium/windsurf/mcp_config.json`)
```json
{
  "mcpServers": {
    "lgtm": {
      "command": "lgtm",
      "args": ["mcp"]
    }
  }
}
```

#### Aider (`.aider.conf.yml` or CLI)
```yaml
# In .aider.conf.yml:
lint-cmd: "lgtm check --strict"
test-cmd: "cargo test"
```
Or start Aider in one tmux/zellij/wezterm pane and `lgtm` in the other for real-time steering via `Enter` or chords (`ca`, `cs`, `cr`, `cp`, `cw`).

### Available MCP Tools

| MCP Tool | Description | Parameters |
|---|---|---|
| `lgtm_diff` | Get structured diff snapshot with file-by-file stats | `rev_spec` (optional string, e.g. `main..feat`) |
| `lgtm_security_scan` | Run static analysis guardrails for OWASP leaks, complexity spikes, and test neutering | `strict` (boolean, flag only Critical/High) |
| `lgtm_list_directives` | Read all human review directives from `.lgtm/comments.jsonl` | *(None)* |
| `lgtm_add_directive` | Anchor a review directive to a specific file and line | `file` (string), `line` (integer), `content` (string) |
| `lgtm_directive_status` | Directives with 1-based `id`, outcome, claim and verdict | *(None)* |
| `lgtm_review_state` | Reviewed hunk count and directive counts | *(None)* |
| `lgtm_scan_changes` | Findings new since the previous call | *(None)* |
| `lgtm_resolve_directive` | Record a claim only; never resolves | `id` (integer), `note` (string) |

**Claims:** an agent calls `lgtm_resolve_directive` to say it handled a directive. lgtm only stores the claim and checks it against the diff (shown as a verdict such as `claimed but not changed` in the directive list). Only the human resolves. MCP writes are limited to directives and claims; it never stages, commits, posts or edits code. `lgtm_scan_changes` keeps its seen fingerprints in `.lgtm/last_scan.json`.

### MCP Live Resources & Subscriptions
Agents can discover and subscribe to real-time review resources via standard MCP protocol (`resources/list`, `resources/read`, `resources/subscribe`, `resources/unsubscribe`). A background filesystem watcher automatically emits `notifications/resources/updated` whenever repository files or directives change, debouncing events and serializing messages safely under a locked stdout:

| Resource URI | MIME Type | Description |
|---|---|---|
| `lgtm://directives` | `application/json` | Active and claimed human review directives with line anchors and verdicts |
| `lgtm://diff` | `text/plain` | Current repository git diff snapshot |
| `lgtm://findings` | `application/json` | Security alerts, complexity spikes, and test fraud detections across working tree |
| `lgtm://review-state` | `application/json` | Reviewed hunk counts, progress percentage, and review marks |

---

## 7. Headless CI & Security Gates

Integrate `lgtm` into pre-commit hooks, Makefiles, and GitHub Actions workflows to prevent broken or insecure agent code from merging:

### 1. `lgtm check` (CI Quality Gate)
Exits with code `1` if Critical or High severity security alerts, complexity spikes, or neutered tests are detected:
```bash
lgtm check --strict
```

### 2. `lgtm gate` (Human-in-the-Loop Agent Gate)
Halts an autonomous agent until a reviewer approves or rejects changes via TUI:
```bash
# Agent runs this before tool execution or committing
lgtm gate --timeout 300 --message "Review generated auth middleware"
```
Exit codes: `0` (Approved), `1` (Rejected or timeout), `2` (Tool error).

### 3. `lgtm scan` & Interactive Codebase Audit Mode
`:audit` and `:scan repo` audit the **whole codebase**, not just the diff; plain `:scan` and `:risk` cover changed files only.

Recursively audits all source files in the repository for security vulnerabilities, architectural violations, and code quality issues:

```bash
# Launch interactive in-app Codebase Audit TUI (shows only issues, no diff view)
lgtm scan repo
lgtm scan --tui
lgtm audit

# Headless terminal summary with scorecard grade (Grade A/B/C/F)
lgtm scan

# Headless Markdown audit report export
lgtm scan --output lgtm-report.md

# Generate SARIF 2.1.0 report for GitHub Security tab
lgtm scan --format sarif --output lgtm-results.sarif
```

Config (`.lgtm.toml`) is applied by `check`, `ci`, `scan`, `risk` and the TUI through one pipeline in `lgtm-core::findings`: suppress (inline + `[ignore]`), then `[severity]` overrides. Suppression forms:

- `lgtm:ignore` (and variants) hides every rule on that line.
- `lgtm:ignore(sec/tls-bypass, dead/*)` hides only the listed rules.
- `[ignore] paths` and `[ignore.rules]` globs (`*` within a segment, `**` across) hide by file.
- `[severity] "rule-or-prefix/*" = critical|high|warning|info|off`: exact IDs beat prefixes, `info` maps to `warning`, `off` drops the finding.
- `thresholds.complexity_increase` hides complexity spikes below that many points (guards report from 8).

Suppressed counts go to stderr; `check`/`ci --show-suppressed` lists them. `ci` ignores the user config file; `--no-user-config` or `LGTM_NO_USER_CONFIG=1` ignores it elsewhere.

Baselines: `lgtm baseline update [--prune]` runs a full scan and records `{rule_id, file, fingerprint}` entries, sorted, in `[baseline] file` (default `.lgtm-baseline.json`). `check`, `ci`, `scan`, `risk` and the TUI then treat matching findings as existing: they are counted on stderr and never affect the exit code. `scan` also reports stale entries (no matching finding); `--prune` removes them. Critical findings are never baselined unless `[baseline] allow_critical = true`. Fingerprints ignore line numbers, so moving code keeps its baseline entry; Editing the flagged line makes it new again.

Remote PR calls: `lgtm_core::github::GithubClient` hides `gh` (Azure DevOps uses `az`, see below); `:checks` shows CI pass/fail/pending in the PR header and `:threads` lists open review threads in the status line and marks their lines in the diff gutter (`◆` open, `◇` resolved). Both run only on demand; missing credentials give a clear message and local features keep working. See the [README diagnostics guide](../README.md#debugging-slow-pr-reviews) for opt-in timing logs.

Posting: `:pr post` previews how many comments would go inline (line is in the diff) or into one combined general comment; `:pr post --yes` sends them, saving each comment's `remote_id` right away so a failed run resumes without duplicates ("N of M posted"). Only unresolved comments made while that PR was open are posted. Not yet: re-anchoring on a changed PR head, orphan skipping, and a `↑ #id` marker in the UI.

Public GitHub listings and stored patches support anonymous HTTPS fallback for a
recognized `github.com` origin when no API token is set and `gh` is unavailable
or needs login. Successful CLI reads are reused. Each fallback sends one GET
without credentials; failures report missing/private resources, access restrictions,
or public rate limits without retrying. Invalid API tokens, Git-fetch auth errors,
network failures, enterprise hosts, and SSH aliases do not activate this fallback.
Discussion threads and writes still require credentials.

PR loading and directive preparation run in the background while the current review stays visible.
Closing drafts, modals, and visual selections allows the prepared review to appear. If its
comments, marks, or turn changed during loading, preparation runs again against the fetched
snapshot without another fetch or scan. The UI merges and saves current directives, preserving
new edits, replies, and posting receipts. Local refresh reconciliation and this final save
still run on the UI thread. Use `:reload` to fetch the current PR again.

Threads: after `:threads`, `:thread reply <n> <text>`, `:thread resolve <n>` and `:thread unresolve <n>` (n = 1-based position in the list) preview what they would do; add `--yes` to send. Nothing is replied to or resolved without it, and a 403 or auth failure is shown as is, never retried.

Directive outcomes (`lgtm_core::outcome`): each comment stores the hash of its hunk when written. On every diff reload, open directives from earlier turns are classified as `untouched` (same hunk), `changed` (file still in the diff, hunk differs), `gone` (file left the diff) or `orphaned` (no baseline, e.g. older comments). The result is saved on the comment. Agent claims are stored apart (`claim`) and never change the outcome or resolve anything; "claimed but not changed" is flagged. In the directive list (`:c`) each open directive shows its verdict (`[changed]`, `[claimed but not changed]` in red, ...) and the title shows the per-turn counts; `:directives recheck` recomputes on demand and prints the counts. Resolving stays a human action: `v`/Space in the list. No outcome filter yet.

Review state (`lgtm_core::review`): in the TUI, `M` marks or unmarks the current hunk and `v` the whole file; marks persist per target (`pr-N`, revision or `working`) and survive edits above the hunk. The file list shows `✓n/m` for partly reviewed files. `lgtm review export [target]` prints the JSON and `lgtm review import <file>` merges it (hunks by hash, comments by file+code+text; local wins on conflicts, which are listed on stderr). Storage: `.lgtm/reviews/<target>.json` holds hunk marks (keyed by a whitespace-normalized hunk content hash) and the review-store comment model with `reanchor`; files with a newer `version` are refused.

Comments and directives still live in `.lgtm/comments.jsonl` (ids stay stable for MCP). On every diff reload they are re-pinned: the commented code plus its neighbours is searched in the same file and the nearest match wins. If it is gone from a file still in the diff, the comment is flagged `orphaned` (`⚠ Orphaned` in `:c`, a count in the header, no inline marker) and never deleted automatically; it recovers if the code returns. `:directives orphans` lists them, `:directives orphans repin <n>` moves orphan n to the cursor line, and `:directives orphans drop` previews deleting them all (`--yes` to confirm). Comments on files outside the current diff, or on deleted lines, are left as they are.

In the TUI, suppressed and baselined findings never appear in the alert list or block anything. The header shows a dim `(+N suppressed, M baselined :hidden)` counter, and `:hidden` lists the first five in the status line.

Every finding carries a kebab-case `rule_id` (e.g. `sec/tls-bypass`) and a `fingerprint` (rule + file + whitespace-normalized line + occurrence index, independent of line number). JSON exposes both; SARIF uses `ruleId` and `partialFingerprints["lgtmFingerprint/v1"]`.

#### Interactive Audit Mode Keybindings
When in Codebase Audit Mode (`ActiveView::AuditReport`):
- `j` / `k` (or `Up` / `Down`): Navigate issues list.
- `1`: Filter to `[🔴 CRIT]` issues only.
- `2`: Filter to `[🟠 HIGH]` and `[🔴 CRIT]` issues.
- `3` / `a`: Show all issues (`Critical`, `High`, `Warn`).
- `Enter`: Inject violation remediation directive directly to agent `tmux` pane / OSC 52 clipboard.
- `e` / `o`: Open violation file and line number directly in configured editor (default: `nvim`).
- `w` / `E`: Export full repository audit report to `lgtm-report.md`.
- `Esc`: Return to diff review mode.
- `:w [path]`: Export report to a custom Markdown path.
- `:diff`: Return to Diff mode.

### GitHub Actions Workflow Example
Create `.github/workflows/lgtm-gate.yml`:

```yaml
name: LGTM Security & Quality Gate

on:
  pull_request:
    branches: [ main ]

jobs:
  security-gate:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout Code
        uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Install LGTM
        run: cargo install --git https://github.com/mgabs/lgtm.git

      - name: Run LGTM Pre-Merge Gate
        run: lgtm check --strict

      - name: Generate Security SARIF
        if: always()
        run: lgtm scan --format sarif --output lgtm-results.sarif

      - name: Upload to GitHub Code Scanning
        uses: github/codeql-action/upload-sarif@v3
        if: always()
        with:
          sarif_file: lgtm-results.sarif
```

---

### CI recipes

Gate a pull request on the committed range (needs full history: `fetch-depth: 0`). Exit codes: 0 pass, 1 blocking findings, 2 tool error.

**GitHub Action** (verified install, SARIF upload, job summary):

```yaml
permissions: { contents: read, security-events: write }
steps:
  - uses: actions/checkout@v4
    with: { fetch-depth: 0 }
  - uses: mgabs/lgtm@v1.0.0
    with: { version: "1.0.0" }
```

Inputs: `version` (required), `base`, `strict`, `upload-sarif`. Outputs: `exit-code`, `new-findings`.

**GitLab CI** (any runner: install the release tarball, then run):

```bash
lgtm ci --rev "origin/${TARGET_BRANCH}...HEAD" -f markdown
```

**Azure Pipelines** (needs `fetchDepth: 0`; findings appear as `##vso[task.logissue]` annotations, auto-selected when `TF_BUILD` is set or with `-f azure`):

```yaml
steps:
  - checkout: self
    fetchDepth: 0
  - script: |
      # install the lgtm release tarball, then:
      lgtm ci --rev "origin/$(System.PullRequest.TargetBranch)...HEAD" -f azure
    displayName: lgtm
```

**pre-commit hook** (`.git/hooks/pre-commit`, checks the working tree):

```bash
# Manage pre-commit hook automatically:
lgtm hook install     # writes .git/hooks/pre-commit and chmod +x
lgtm hook status      # verifies hook presence and active state
lgtm hook uninstall   # cleanly removes hook
```

### Azure DevOps pull requests

lgtm picks the provider from the `origin` URL (`dev.azure.com`, `ssh.dev.azure.com`, `*.visualstudio.com` are Azure DevOps; other remotes use the GitHub provider). Azure uses the `az` CLI with the `azure-devops` extension: `az extension add --name azure-devops`, then `az login` (or `az devops login`). The REST path reuses a short-lived token in memory and passes it to curl through a private temporary file.

| Works | Not yet (planned) |
|---|---|
| `:prs open/closed/all`, `lgtm prs --state open/closed/all`: list PRs by state (default open) | |
| `:pr <n>`, `lgtm pr <n>`: reads fresh metadata through REST with cached authentication and CLI fallback; uses recorded commits before probing refs, including completed/abandoned PRs | |
| `:checks`: branch policies as pass / fail / pending | |
| `:threads`: PR threads (via `az devops invoke`; system and deleted comments skipped; `fixed`, `closed`, `wontFix`, `byDesign` count as resolved) | |
| `:pr post --yes`: inline comments become threads on the new-side line, the rest one general thread; `:thread reply`, `resolve` (status `fixed`), `unresolve` (`active`) | |
| Review submit: `APPROVE` sets vote `approve`, `REQUEST_CHANGES` sets `wait-for-author`, `COMMENT` only comments (body posted as a thread) | |

After `:threads`, diff lines with remote discussion get a gutter marker (`◆` open, `◇` resolved) on both GitHub and Azure DevOps. Markers cover threads attached to a new-side line; general threads show only in the status line. Jump between thread lines with `]t`/`[t`, and trigger inline reply or toggle resolution directly on the marked line with `r` or `R`.

Speed: `:prs`, `:checks` and `:threads` run in the background (the status line shows "Loading..."), and Azure reads call the REST API with `curl` using one `az account get-access-token` per session (about 30 minutes, memory only; the token goes to curl through a private temp file, never the command line). Results are cached for 5 minutes and any write (post, reply, resolve, vote) clears the cache. If `curl` or the token fails, lgtm falls back to `az repos` / `az devops invoke` (slower, same result).

Errors name the fix: missing `az`, missing extension, or not logged in. The organization and project come from the git remote (`--detect true`). Unverified against a live Azure DevOps project.

## 8. Ex-Commands Dictionary (`:cmds`)

Press `:` in diff view to enter Ex-Command Mode with `Tab` / `Shift-Tab` wildmenu auto-completion:

| Command | Aliases | Description |
|---|---|---|
| `:audit` | — | Audit the whole codebase (issues only, no diff) |
| `:scan [repo]` | `:issues` | Scan changed files; `:scan repo` audits the whole codebase |
| `:export [path]` | — | Export audit scorecard report to Markdown (default: `lgtm-report.md`) |
| `:w [path]` | `:write` | In Audit mode: exports report to disk. In Diff mode: saves review directives to `.lgtm/comments.jsonl` |
| `:diff [range]` | `:rev`, `:range` | Return to Diff mode (or load a range, e.g. `:diff main..feat`; a bare branch name means `<branch>...HEAD`, audited on load) |
| `:q` | `:quit` | Quit `lgtm` (`:q!` force quit without saving) |
| `:wq` | `:x`, `:exit` | Save directives and quit |
| `:commit [msg]` | `:ci` | Open Commit Forge modal or commit staged changes with message |
| `:pr <number>` | — | Review an open or closed GitHub/Azure DevOps PR in the background |
| `:prs [open\|closed\|all]` | — | Open PR picker; `1`/`2`/`3` select open/closed/all |
| `:f` | `:files`, `:find` | Open fuzzy file switcher popup |
| `:pane [id]` | `:panes` | Select or set target agent pane ID (`tmux`, `zellij`, `wezterm`, `kitty`) |
| `:sink [name]` | — | View or set active agent steering sink (`tmux`, `zellij`, `wezterm`, `kitty`, `clipboard`, `auto`) |
| `:sp` | `:split`, `:vsp` | Toggle Unified Diff vs Side-by-Side Dual Pane Diff |
| `:tree` | `:drawer`, `:sidebar` | Toggle file tree sidebar |
| `:c` | `:comments`, `:directives`, `:todo` | Open review directives management drawer |
| `:c recheck` / `:c orphans [repin <n> \| drop [--yes]]` | `:directives` | Reclassify outcomes; list, re-pin or drop orphaned comments |
| `:clear` | — | Clear all review directives |
| `:clear-resolved` | — | Purge all resolved directives while preserving active ones |
| `:resolve [all]` | `:res` | Mark directive on current line (or all directives) as resolved |
| `:unresolve [all]` | — | Mark directive on current line (or all directives) as active |
| `:checks` / `:threads` | — | CI status in the header / list PR threads and mark their lines (`]t`/`[t` jump) |
| `:thread reply <n> <text>` / `resolve <n>` / `unresolve <n>` | — | Preview; add `--yes` to send |
| `:r` | `:review`, `:send`, `:push` | Compile directives batch prompt and blast to target agent pane |
| `:risk` | — | Re-scan current diff for security, test fraud, and complexity risks |
| `:reload` | `:e`, `:edit` | Reload diff snapshot from working tree or GitHub PR |
| `:filter <1\|2\|3\|crit\|high\|all>` | `:sev` | Filter audit issues by severity |
| `:inject [all]` | — | Inject violation remediation directive(s) into agent pane |
| `:approve` | `:lgtm` | Approve pending agent breakpoint gate (`.lgtm/gate.json`) |
| `:reject` | — | Reject & revert hunk under cursor and re-prompt agent with reason |
| `:reject-all` | `:reject-gate` | Reject waiting agent breakpoint gate |
| `:prompt` | `:context` | Open agent prompt context modal from gate, prompt log, or commit |
| `:editor [name]` | — | View or configure active editor (`nvim`, `vim`, `vi`, `vscode`, `code`, `cursor`, `helix`) |
| `:hook [install\|uninstall\|status]` | — | Manage Git pre-commit hook directly from the TUI |
| `:y` | `:yank` | Yank current code line to clipboard |
| `:yr` | `:ref` | Yank `#ref file:line` to clipboard |
| `:<number>` | — | Jump directly to line number (e.g. `:42` or `:150`) |

---

## 9. Configuration Reference (`.lgtm.toml`)

Run `lgtm init` (or `lgtm --init`) to create a starter configuration file at `.lgtm.toml`:

```toml
# LGTM Configuration File (.lgtm.toml)
# Documentation: https://github.com/mgabs/lgtm

editor = "nvim"
agent_sink = "tmux"

[thresholds]
complexity_line_delta = 15
complexity_nesting_depth = 4
blast_radius_files = 8

[ignore]
paths = [
    "package-lock.json",
    "Cargo.lock",
    "yarn.lock",
    "**/*.min.js",
    "target/**",
    "dist/**"
]

[severity]
# Override rule severity ("critical", "high", "warn", or "off"), e.g.:
# "arch/blast-radius" = "off"

[baseline]
path = ".lgtm-baseline.json"
```

## 10. CLI Command Reference

| Command | Description |
|---|---|
| `lgtm` / `diff` / `status` / `review` | Launch interactive diff reviewer TUI (default) |
| `lgtm gate [--timeout <s>] [-m <msg>]` | Human-in-the-loop breakpoint gate; halts until approved/rejected in TUI |
| `lgtm hook [install\|uninstall\|status]` | Install, uninstall, or inspect the LGTM Git pre-commit hook |
| `lgtm risk` | Audit current diff for security risks, test neutering, fraud |
| `lgtm init` / `--init` | Create `.lgtm.toml` |
| `lgtm pr <n>` / `--pr <n>` | Review a PR (GitHub via Git/API/`gh`, Azure DevOps via `az`) |
| `lgtm prs --state open/closed/all` | List and pick PRs by state (default open) |
| `lgtm check [--strict] [-f markdown\|json\|sarif] [-o file]` | Headless gate; fails on Critical/High (`--strict`: Warn too) |
| `lgtm config [--show] [--no-user-config]` | Print effective `.lgtm.toml` (repo) + `~/.config/lgtm/config.toml` (user) values with sources; precedence repo > user > defaults, tables merge, lists replace, unknown keys warn |
| `lgtm ci [--strict] [--rev <spec>] [-f ...] [-o file]` | Headless diff audit; sticky PR comment, JSON or SARIF. `--rev origin/main...HEAD` checks a committed range. Exit 0 pass, 1 blocking findings, 2 tool error (also on `check`) |
| `lgtm scan [repo] [-t] [-f markdown\|json] [-o file]` | Full repository audit (default output `lgtm-report.md`) |
| `lgtm audit` | Interactive repository audit TUI |
| `lgtm mcp` / `lgtm serve --mcp` | MCP stdio server |
| `lgtm log\|commit\|add\|...` | Git passthrough |

Global options: `-r/--repo <path>`, `-p/--agent-pane <id>`, `--agent-sink <kind>`, `--pr <n>`, `-d/--debounce <ms>` (default 300), `--debug`, `--debug-log <path>`, `--init`.

---

## 11. Additional Guardrails

Beyond section 4, these guards run on every diff / scan:

- **Architecture (`guard/arch.rs`)**: stray/sensitive file leaks (Critical), layer boundary violations (High), `pub` scope inflation (Warn), blast radius across many files.
- **Dependencies (`guard/deps.rs`)**: unpinned/wildcard deps (High) and suspicious/typosquatted packages (Critical) in `Cargo.toml`, `package.json`, `pyproject.toml`, `requirements.txt`, `go.mod`.
- **Duplicate helpers (`guard/duplicate.rs`)**: names like `format_date_2` suggesting an existing utility was reimplemented (Warn).

### Suppressing alerts
Add any of `lgtm:ignore`, `lgtm-ignore`, `nolint:lgtm`, `lgtm[ignore]`, `lgtm_ignore` on a line. Code inside `#[cfg(test)]` is auto-ignored during `lgtm scan`.

---

## 12. Quality Tooling (`make` targets)

| Target | Purpose |
|---|---|
| `make build` / `release` | Debug / LTO release binary |
| `make test` | Unit + integration tests |
| `make bdd-report` | Automated BDD scenario catalog report (`./scripts/bdd-report.sh`) |
| `make lint` / `fmt` / `check-fmt` | Clippy (`-D warnings`) / rustfmt |
| `make check` | `check-fmt` + `lint` + boundary check + `test` |
| `make ci` | `check` + SARIF export |
| `make tap` | `scripts/update-tap.sh` — update Homebrew formula |

### Behavior scenarios and coverage

Public API acceptance scenarios live in `crates/lgtm-core/tests/bdd.rs`; scenarios
that launch the application live in `tests/bdd.rs`. Both use ordinary Rust tests
with Given/When/Then comments. Run them with:

```sh
cargo test --workspace --test bdd
```

Private UI and provider behavior tests stay beside their modules and run with
`cargo test --workspace`. The [scenario catalog](bdd-scenarios.md) maps both types
and distinguishes offline fixtures from terminal and live-provider verification.
The [Q0–Q6 roadmap](../specs/superpowers/roadmap.md#q--bdd-acceptance-gaps-added-2026-10-23)
tracks the acceptance milestones (all complete).

Run the automated BDD catalog coverage report:

```sh
make bdd-report
# or ./scripts/bdd-report.sh
```

It maps all 41 agreed criteria to executable tests and evaluates pass/fail status,
automation coverage, verified coverage, and pass rate.

---

## 13. Source Module Map

| Path (under `crates/lgtm-core/src/` unless noted) | Role |
|---|---|
| `src/main.rs`, `src/event_loop.rs`, `src/diagnostics.rs` | Entry point, bounded terminal/watcher/callback event loop, and UI workload diagnostics |
| Root `tests/bdd.rs`, `crates/lgtm-core/tests/bdd.rs` | Application-launch and public API acceptance scenarios |
| `crates/lgtm-core/tests/live_provider.rs` | Opt-in live GitHub & Azure DevOps provider verification |
| `scripts/bdd-report.py`, `scripts/bdd-report.sh` | Automated BDD scenario catalog coverage and verification reporting |
| `src/cli.rs`, `src/headless.rs` | CLI definitions; headless runners (`ci`, `risk`, `review`, `init`) |
| `src/app/` | TUI state machine and background coordinators (`refresh`, `view_data`, `rows`, `audit/jobs`, `pr/completion`) |
| `src/ui/` | Ratatui rendering with viewport/layout caches (`diff/cache.rs`, `diff/viewport.rs`) and `modals/` |
| `crates/lgtm-ast/src/lib.rs` | Tree-sitter AST parsing, queries, and block folding (TS/JS, Swift, Python, Go, Java) |
| `model.rs`, `debug.rs` | `DiffSnapshot`, `FileDiff`, `DiffHunk`, `DiffLine`; structured JSONL timing recorder |
| `scan.rs` | Full repo scanner and report generator |
| `git/`, `pr/`, `forge/` | Git diff engine; PR providers (`forge`: GitHub via Git/API/`gh`, Azure DevOps via `az`, `failure.rs`) |
| `guard/` | Static analysis guards behind the `Guard` trait and `registry()` (see sections 4, 11) |
| `config/`, `findings/`, `baseline/` | `.lgtm.toml` loader, suppression/severity pipeline, and baseline partitioning |
| `comment/`, `turn/`, `review/`, `outcome/` | Directives, reviewed-file state, durable hunk/fold marks, and directive outcome tracking |
| `ipc/` | Multiplexer sinks (`tmux`, `zellij`, `wezterm`, `kitty`), OSC 52 clipboard; `AgentSink` enum (`sink.rs`) |
| `watcher/` | Debounced filesystem watcher |
| `crates/lgtm-mcp/src/lib.rs` (crate root) | MCP server and live resource watcher notifications |

---

## Contributing & Support

See [CONTRIBUTING.md](../CONTRIBUTING.md).

- **Repository**: [https://github.com/mgabs/lgtm](https://github.com/mgabs/lgtm)
- **Issue Tracker**: [https://github.com/mgabs/lgtm/issues](https://github.com/mgabs/lgtm/issues)
- **Discussions**: [https://github.com/mgabs/lgtm/discussions](https://github.com/mgabs/lgtm/discussions)
- **License**: MIT
