# lgtm (Looks Good To Me)

[![CI](https://github.com/mgabs/lgtm/actions/workflows/ci.yml/badge.svg)](https://github.com/mgabs/lgtm/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

> **Fast, real-time terminal diff reviewer, AI agent steering TUI & CI security scanner built in Rust.**
> *"Because code shouldn't ship until it actually looks good to you."*

`lgtm` is a keyboard-driven diff reviewer and security guardrail engine engineered for human-in-the-loop AI coding workflows and terminal velocity. It pairs Vim motion parity with live file-watcher updates, deep static analysis (complexity spikes, typo/hallucination checks, dead code, breaking API drift), AST/hunk folding, an interactive Commit Forge, GitHub PR review, and direct `#ref` injection into `tmux`, `zellij`, `wezterm`, `kitty` agent panes or system clipboards.

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
- **Native MCP Server Mode (`lgtm serve --mcp` / `lgtm mcp`)**: Standard JSON-RPC Model Context Protocol (MCP) server over stdio for autonomous AI agents (Claude Code, Cursor, Windsurf, Aider) to query diffs, run security scans, and manage review directives programmatically, including recording claims that a directive was handled (verified against the diff, never auto-resolved).
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
  - **Security & Test Guard**: Flags SQLi, command injection, hardcoded secrets, TLS/auth bypass, and test neutering (deleted asserts, skipped tests, mock inflation, self-assertions). Supported AST languages: Rust, TypeScript/JavaScript, Python, Swift, Go, and Java.
- **Syntax & Hunk Folding**: Vim-style folding (`zc`, `zo`, `za`, `zM`, `zR`, `zm`, `zr`) to collapse or expand unchanged lines, clean files, or reviewed hunks.
- **Interactive Commit Forge (`C` / `:commit`)**: Craft conventional commits (`feat:`, `fix:`, `refactor:`, etc.) directly from staged hunks with automated diff-aware commit message synthesis.
- **Tabbed Interactive Help Modal (`?` / `h`)**: 7 discrete documentation tabs covering Overview & Modes, Motions, Directives, Staging & Forge, Folding, Guardrails, and Ex-Commands & CI.
- **Neovim-Style Ex Commands**: `:w`, `:q`, `:wq`, `:commit`, `:diff <rev>`, `:pr <num>`, `:prs`, `:pane`, `:files`, `:split`, `:risk`, `:approve`, `:reject`, `:prompt`, `:<line_num>` with `Tab` / `Shift-Tab` wildmenu auto-completion.
- **PR Review Mode**: Review GitHub PRs (Git, API, or `gh`) or Azure DevOps PRs (`az`: list, diff, policies, threads, post, reply, resolve, vote) with `lgtm pr <num>` or `:pr <num>`, write inline review directives, batch compile to `.lgtm/pr-<num>-review.md`, and submit via `gh pr review` on GitHub.
- **CI step**: `lgtm ci --rev origin/main...HEAD` gates a committed range (exit 0 pass, 1 blocking findings, 2 tool error); a composite GitHub Action (`mgabs/lgtm`) adds verified install, SARIF upload and a job summary.
- **Orphaned comments**: comments re-pin when code moves and are flagged `⚠ Orphaned` when their code is gone, never deleted.
- **Interactive Codebase Audit Mode (`lgtm scan repo` / `lgtm audit` / `:scan`)**: Dedicated dual-pane TUI reviewing repository-wide security and architectural issues without diff noise. Features 1-key severity filtering (`1`: Critical, `2`: High+, `3`: All), `$EDITOR` jumping (`e`/`o`), directive injection into agents (`Enter`), and Markdown report export (`w` / `:export`).
- **Headless CI & Security Gates**:
  - `lgtm check`: Pre-commit / CI gate that blocks builds on critical security vulnerabilities or complexity spikes.
  - `lgtm hook [install|uninstall|status]`: Automates setup of `.git/hooks/pre-commit` to gate commits locally.
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

# Review an open or closed GitHub or Azure DevOps PR
lgtm pr 42

# List open PRs (default), closed PRs, or all states
lgtm prs
lgtm prs --state closed
lgtm prs --state all

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

## Open and closed PRs

List PRs with `lgtm prs --state open`, `--state closed`, or `--state all`.
Inside the TUI, use `:prs open`, `:prs closed`, or `:prs all`; the picker also supports
`1` (open), `2` (closed), and `3` (all). Each entry shows its provider status.
The state filter applies to the returned listing; provider page limits still apply.

The closed view includes merged and closed GitHub PRs, and completed and abandoned
Azure PRs. See the provider references for [GitHub listing](https://cli.github.com/manual/gh_pr_list)
and [Azure statuses](https://learn.microsoft.com/en-us/cli/azure/repos/pr).
Any PR can also be opened directly with `lgtm pr <number>` or `:pr <number>`.

Azure loading reads fresh PR metadata first through REST, reusing the cached authentication
token with a CLI fallback. It uses recorded source and target commits before probing Git
refs, so historical PRs avoid attempts at merge refs or branches that may have been deleted. If those objects
are already local, no additional fetch is needed. The server must still retain the commits
for a new fetch to succeed. A closed PR without recorded commits fails clearly rather than
comparing unrelated current branches. GitHub falls back to the stored PR patch from the API
or GitHub CLI, which also avoids guessing the current target branch for historical reviews.
Authentication and network failures still stop without trying additional Git fetches.

For a recognized `github.com` origin, public PR listings and stored patches can
fall back to anonymous HTTPS when `gh` is missing or asks for login and no
`GITHUB_TOKEN`/`GH_TOKEN` is available. An authenticated `gh` result is reused;
the fallback makes one GET request with no authorization header. Private
repositories, discussion threads, and writes still require authentication.
Git fetch authentication failures, unrelated CLI failures, rejected API tokens,
and failed anonymous requests do not trigger further fallback attempts. Anonymous
reads have lower GitHub rate limits; a denied, missing, or rate-limited resource
returns an error. Enterprise hosts and SSH aliases do not use this GitHub.com
fallback.

---

## Debugging slow PR reviews

Enable timing diagnostics when launching the rebuilt binary:

```bash
lgtm --debug pr 42

# Choose a different log path
lgtm --debug --debug-log /tmp/lgtm-review.jsonl pr 42
```

By default, `--debug` writes `lgtm-debug.jsonl` in the system temporary directory
(`/tmp` on most systems). Reproduce the slow action, then inspect the JSONL file. Each operation has a `start` and
`end` record matched by `pid` and `id`. End records include `elapsed_ms` and `outcome`;
a start without an end points to an operation still waiting. Logging is disabled unless
`--debug` is supplied; setting `LGTM_DEBUG_LOG` alone has no effect. Use a log outside the repository (as above), or inside `.lgtm/`,
to avoid treating log writes as workspace edits. The parent directory must exist.
Arguments, command output, comment text, source code, and credentials are never logged.

UI diagnostics include `ui.draw` (including terminal output), `ui.render`, `ui.input`,
`ui.background_events`, `ui.callback`, and `ui.callback_wait`. The last operation measures
the wait between a completed background job and its UI callback; `counts.applied=0` means
the callback was dropped. `ui.pr_apply`, `review.local_snapshot`, `review.reanchor`,
`review.outcomes`, `review.pr_directives`, and `review.load_marks` isolate refresh/completion
work. Local refreshes additionally expose `ui.local_request` (with `counts.coalesced`),
`ui.local_discard` for superseded completions, and `ui.local_apply` for the final UI update.
Counts appear on the corresponding end record. Background-event records also report
`counts.watchers_remaining` and `counts.callbacks_remaining` when work is deferred.

The terminal loop draws at startup and after input, resize/focus changes, applicable
watcher changes, or background results. It checks for background work at most every
16 ms while idle, without redrawing. Each iteration processes up to 32 terminal events,
32 watcher messages, and eight callbacks, with a separate 4 ms budget for each batch.
A running handler cannot be interrupted; an individual command or completion apply can
still exceed that budget (tracked separately in the performance TODOs). Input stays on the UI thread
so terminal editors retain keyboard access. If the file watcher cannot start, external
status is refreshed once a second; use `:reload` to refresh file changes.

Repository audits (`:audit`, `:scan repo`), changed-file scans (`:scan`, `:issues`), and
`:risk` run in background jobs. Previous audit results remain available during scanning;
repository audits show processed-file progress in the header. Repeated requests keep one
running scan and the latest queued request. Review-target, snapshot, or workspace changes
discard affected results; request a new audit after a cancellation. A completion keeps
newer filters and drafts and does not reopen a view you left. Diff scanning still copies
the current snapshot on the UI thread before starting its worker.

Report export also runs in the background. It exports the currently displayed audit,
or scans the repository first if no audit result exists. The status bar confirms the
completed write; normal quit waits for it. A failed export keeps the app open and reports
the error. Export does not replace the current audit results.

Audit diagnostics include `ui.audit_request` (enqueue/coalescing), `ui.audit_snapshot`
(the diff snapshot copy), `ui.audit_progress` (numeric completed/total file counts),
`ui.audit_apply`, and `ui.audit_discard`. Worker spans are `audit.repository`,
`audit.changes`, and `audit.export`. Source paths and finding text are excluded.

Rendering uses prepared repository availability, turn/gate state, staged counts, prompt
context, and audit snippets. A silent background worker loads missing data, keeps only the
latest view demand, and discards results after invalidation. Watcher changes, reloads,
review/audit updates, focus, and local gate/turn changes refresh the relevant context.
Opening the prompt dialog refreshes its contents. Opening the commit dialog retains one
staged-count validation; typing in it uses the cached count, which also follows index changes.
The inspector caches up to 32 snippets of at most seven lines, including missing-file
results. It checks file/line/root and invalidation state before displaying a cached snippet.
Drafts and view selection are preserved when reads complete.

`ui.view_request` reports which prepared data was requested using numeric flags;
`ui.view_prepare` measures worker reads and `ui.view_apply` reports `counts.applied=0`
for stale results. `ui.staged_prepare` measures staged-count queries and reports the numeric
file count. Source and prompt contents are excluded from diagnostics.

Diff rendering also exposes `ui.rows_prepare`, `ui.layout_prepare`,
`ui.annotations_prepare`, and `ui.words_prepare` for cache preparation, plus
`ui.diff_viewport` and `ui.drawer_viewport` for visible-row work. Navigation reuses the
selected file's layout and folded-row mapping; word differences are prepared only for
visible pairs and retained across navigation, folding, annotations, and resizing.
Content, fold, annotation, and width changes invalidate their affected caches. The first
layout preparation still visits the selected file; later draws style and render its
viewport. Annotation change detection still scans annotation data each frame.

`ui.frame` records cumulative frames/idle frames, input/event batch sizes, and background
activity. An idle frame has no terminal events or background activity since the previous
draw; the startup frame is excluded. Use the frame start timestamps to measure redraw
frequency. `ui.input_to_draw` measures from reading the oldest key in a batch to completing
the next draw, not from the key's arrival at the operating system. Only records with
`counts.presented=1` represent completed draws; quitting without another draw leaves zero.
Render context includes numeric file, changed-line, selected-line, comment, thread, alert,
and viewport counts. Keys, view contents, and paths are not included. Diagnostics perform
no timing clock reads or context allocations when disabled.

Compare offline UI workloads with:

```bash
cargo test --bin lgtm ui_workload_benchmark -- --ignored --nocapture

# Also record the instrumented operations (use a log outside the repository)
LGTM_UI_PERF_LOG=/private/tmp/lgtm-ui-performance.jsonl \
cargo test --bin lgtm ui_workload_benchmark -- --ignored --nocapture
```

The benchmark compares 100 lines, 10,000 lines, 1,000 comments, and a 2,000-file working
tree at 120x40 using `TestBackend`. It reports initial view-data preparation separately
(`view_prepare_ms`, including waiting for its callback), then first-draw time, warm draw
median/p95, input and input-to-draw p95,
and separate refresh-request and refresh-completion medians. The request metric measures
enqueueing work; completion includes the background diff/scan and applying its result.
Each scenario runs 21 no-input draws (including startup), 21
input/draw batches, and three refreshes; refreshes use real local files separately from
the synthetic review snapshot. It excludes terminal output and event polling. Use the same
build profile for comparisons; enabled diagnostics add measurement overhead. The benchmark
is opt-in and imposes no wall-clock pass/fail threshold.

Run the offline Azure performance regression tests with:

```bash
cargo test -p lgtm-core forge::azure::performance_tests -- --nocapture
```

These use real local Git fetches and mocked metadata/authentication. They enforce request
budgets across separate forge instances, without flaky wall-clock thresholds:

| Historical PR load | Token requests | Fresh metadata requests | Git fetches | Merge-ref probes | CLI metadata calls |
| --- | --- | --- | --- | --- | --- |
| First load into an empty checkout | 1 | 1 | 1 | 0 | 0 |
| Each of 10 reloads | 0 | 1 | 0 | 0 | 0 |

The active-PR fallback test also verifies one missing merge-ref probe across repeated
loads, followed by a new probe when the five-minute cache expires. Printed offline timings
measure the local harness; they do not predict live network speed.

To measure a real Azure PR, opt into the ignored benchmark from this source checkout:

```bash
LGTM_PERF_REPO=/absolute/path/to/azure-checkout \
LGTM_PERF_PR=19673 LGTM_PERF_RUNS=10 \
cargo test -p lgtm-core live_azure_pr_load_benchmark -- --ignored --nocapture
```

It uses your existing login and fetches PR objects into that checkout. It prints elapsed
milliseconds and request counts for each load, then the reload median and p95. The first
sample starts with empty process caches but retains existing Git objects; it is not a
guaranteed cold network fetch. Compare the same PR and checkout conditions when evaluating
changes. This benchmark does not run during normal tests.

GitHub has equivalent offline request-budget tests:

```bash
cargo test -p lgtm-core forge::github::performance_tests -- --nocapture
```

They exercise an actual merge-ref fetch from a local Git remote, stored-patch API and CLI
fallbacks, and failures that must stop without trying another transport. When the merge ref
exists, each load fetches it once. When the merge ref is missing, it is cached for five
minutes per repository, remote, and PR (matching Azure), so reloads skip repeated failing
merge-ref fetches and issue only one stored-patch request per load.
These tests make those costs explicit without imposing network timing thresholds.

Run the opt-in live GitHub benchmark with:

```bash
LGTM_PERF_REPO=/absolute/path/to/github-checkout \
LGTM_PERF_PR=19 LGTM_PERF_RUNS=10 \
cargo test -p lgtm-core live_github_pr_load_benchmark -- --ignored --nocapture
```

It reports per-load merge fetches, other fetches, API/CLI patch requests, elapsed time, and
reload median/p95. Git fetching uses the checkout's existing credentials. Stored-patch
loading uses `GITHUB_TOKEN`/`GH_TOKEN` when set, otherwise `gh`; public GitHub.com patches can use anonymous HTTPS if `gh` is unavailable or needs login. Existing
Git objects are retained, so the first sample is not guaranteed to fetch cold objects.

- `pr.anonymous_read`: a public GitHub HTTPS fallback attempt; logs timing without repository URLs.
- `pr.diff`, `git.fetch_merge_ref`, `git.fetch_head_ref`, `git.fetch_branches`, `git.fetch_commits`, `pr.fetch_metadata`, `pr.build_diff`: fetching and building a PR diff, including fallback attempts.
- `review.scan` / `review.reload`: guard scanning and explicit diff reloads.
- `comments.add_comment` / `comments.save_comments`: local directive creation, replies, and resolution saves.
- `pr.post_batch`, `pr.head_sha`, `pr.post_inline`, `pr.post_general`, `pr.post_throttle`: remote posting, persistence, and intentional spacing between requests.
- `pr.threads`, `pr.thread`, `pr.reply`, `pr.resolve` / `pr.unresolve`: reading and updating remote discussions.
- `command.curl`, `command.gh`, `command.az`: underlying provider commands. `failure` records include a safe `category`, `exit_code`, and `http_status` when available. No raw provider error output is saved.

`c` creates a local directive; `:pr post` previews unposted directives and
`:pr post --yes` sends them. Local directive replies and resolution are saved locally.
Use `:threads` to load remote discussions, `:thread reply <n> <text> --yes` to reply,
and `:thread resolve <n> --yes` to resolve a remote discussion. Thread updates run in
the background, in order within each discussion. Duplicate pending actions are ignored;
different discussions can update independently. A successful reply appears immediately,
then only that discussion is refreshed. A failed reply remains available as a draft when
you press `r` again. A refresh failure does not resubmit a successful write.

Opening/reloading a PR, posting directive batches, and submitting compiled reviews also
run in the background. The current diff stays visible until the new PR is ready. An older
load cannot replace a newer selection; a failed load preserves the current review and
shows a retry command. If a draft or modal is open when loading finishes, the switch waits
until that view is closed so comments keep their original review target. Workspace and comment edits do not refetch an open PR; use
`:reload` to fetch its latest diff.

PR workers also prepare directive anchors, advisory outcomes, reviewed-file flags, and saved
folds. Before switching views, the UI checks the repository/load generation, the exact target
PR directive inputs (including edits, replies, resolution, and posting receipts), and the
marks/turn revision. Changed inputs trigger another preparation of the fetched snapshot,
without fetching or scanning the PR again. Modals and visual selections defer application.
Unrelated directives remain untouched. Only the UI saves the merged current directives;
preparation workers do not write directive files. Posting still waits for each successful
receipt save before its next remote write. A failed completion save remains visible.

The preparation spans are `review.pr_prepare`, `review.pr_turn`, `review.pr_reanchor`,
`review.pr_outcomes`, `review.load_marks`, and `review.pr_mark_matching`.
`ui.pr_prepare_request` measures input capture, `ui.pr_reprepare` records a retry, and
`ui.pr_apply` includes the remaining UI merge and `review.persist_pr_directives` write.
That write remains synchronous to retain the existing save-acknowledgement boundary.

Measure the offline PR completion fixture (1,000 directives, 2,000 lines) with:

```bash
cargo test --bin lgtm pr_completion_workload_benchmark -- --ignored --nocapture

# Optional phase diagnostics; no source or directive contents are logged
LGTM_PR_PERF_LOG=/private/tmp/lgtm-pr-completion.jsonl \
cargo test --bin lgtm pr_completion_workload_benchmark -- --ignored --nocapture
```

It reports input capture/queue time, background preparation, UI completion (including its
save), and TestBackend input-to-draw p95 while the previous review remains visible.
It uses no network or provider credentials and excludes terminal output and first rendering
of the new review. Timings are fixture observations, not latency guarantees.

Local startup, working-tree/revision reloads, and refreshes after staging or committing also
compute their diff and findings in the background. One local refresh runs at a time; further
requests replace a single queued request. Already-received workspace edits invalidate an
older completion before it can be applied, and switching review targets cancels obsolete
local results. `:reload` uses the current source, including revision mode and PR mode.
Existing snapshots stay visible until the new result can be applied. Drafts, reply editors,
modals, and visual selections defer the update until closed. Completion reconciles current
directives rather than replacing them with a worker's older copy. Errors retain the current
review. Local-refresh re-anchoring, outcome checks, and review-mark loading still run during
the final UI update; PR loads perform those preparation phases in their worker.

Each successful batch post is saved before the next request, so a retry skips comments
already sent. New directives and local replies made during posting are preserved. Existing
comment files receive stable local ids automatically. Normal quit waits for pending PR
writes and their saves; failures keep the review open so the error can be inspected.

For Azure PRs with incomplete active metadata, a missing merge ref can fall back to branch
fetching. Missing refs are remembered for five minutes per repository, remote, and PR across
loads; diagnostics mark skipped probes as `pr.merge_ref_cache_hit`. Authentication, access, network,
and timeout failures stop immediately instead of repeating fetches through other routes.
GitHub writes choose a single transport; uncertain failures are reported without an
automatic duplicate attempt. Curl requests have a 10-second connection timeout and a
30-second total timeout. Diagnostics remain disabled unless `--debug` is supplied.

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
| `]f` / `[f` (or `]]` / `[[`) | Jump to next / previous changed file (chainable) |
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
| `T` | Summarize test-risk alerts in the current review |
| `c` | Add inline review directive on current line (or comment on finding in `:audit`) |
| `r` | Reply to review directive at cursor |
| `R` / `<Space>r` | Toggle directive resolved state (`✓ RESOLVED` vs `💬 OPEN`) |
| `<Space>d` | Delete directive on current line |
| `x` | Discard changes in current hunk (or delete directive if on comment line) |
| `<Space>l` / `:c` | Open Review Directives list drawer |
| `v` | Mark current file as reviewed (persisted in `.lgtm/state.json`) |
| `o` / `e` | Open active file at cursor line in external editor (default: `nvim`, configurable: `vscode`, `vim`, etc.) |
| `Tab` | Toggle 35-column file drawer / sidebar |
| `d` | Toggle split-view / unified diff view |
| `p` | Select target agent pane (`tmux`, `zellij`, `wezterm`, `kitty`) for agent steering |
| `Enter` | Inject reference `#ref file:line [rule] <alert> | DIRECTIVE: <text>` into active agent pane |
| `Ctrl-s` | Compile and export all review directives to agent / `.lgtm/review.md` |
| `?` / `h` | Toggle multi-tab help modal (`1`-`7` or `[`/`]` to switch tabs) |
| `q` | Quit |

---

## Ex Commands (`:cmds`)

Press `:` in diff view to enter command mode with `Tab` / `Shift-Tab` wildmenu completion:

- `:w` — Save comments / directives to `.lgtm/comments.jsonl` (or export report in Audit mode)
- `:q` / `:q!` — Quit
- `:wq` / `:x` / `:exit` — Save directives and quit
- `:approve` / `:lgtm` — Approve pending agent breakpoint gate
- `:reject` — Reject current hunk and dispatch prompt to agent
- `:reject-all` — Reject waiting agent breakpoint gate
- `:prompt` / `:context` — Open agent prompt and instruction context modal
- `:commit [msg]` / `:ci` — Open Commit Forge modal or commit staged changes
- `:pr <number>` — Load and review a GitHub or Azure DevOps PR diff
- `:prs [open|closed|all]` — Open the PR picker with a state filter (default: open); use `1`/`2`/`3` in the picker to switch filters
- `:checks` / `:threads` — PR CI status / list PR threads (gutter `◆`/`◇`, `]t`/`[t`)
- `:c` / `:directives` — Open review directives drawer (`:directives recheck`, `:directives orphans`)
- `:clear` — Clear all review directives
- `:clear-resolved` — Purge all resolved directives while preserving active ones
- `:resolve [all]` / `:unresolve [all]` — Toggle directive resolved state
- `:f` / `:files` / `:find` — Open fuzzy file finder
- `:pane` / `:panes` — Open target pane selector for active terminal multiplexer
- `:sink [name]` — View or configure active agent steering sink (`tmux`, `zellij`, `wezterm`, `kitty`, `clipboard`, `auto`)
- `:sp` / `:split` / `:vsp` — Toggle split / unified view
- `:tree` / `:drawer` / `:sidebar` — Toggle file tree drawer
- `:risk` — Re-scan current diff for security & test risks
- `:editor [name]` — View or configure active editor (`nvim`, `vim`, `vi`, `vscode`, `code`, `cursor`, `helix`)
- `:scan [repo]` — Scan diff for issues (or `:scan repo` for full codebase)
- `:audit` — Audit the whole codebase; `:issues` scans current changes
- `:filter <1|2|3|crit|high|all>` / `:sev` — Filter audit issues by severity
- `:inject [all]` — Inject violation remediation directive(s) into agent pane
- `:export [path]` — Export audit scorecard report to Markdown
- `:r` / `:review` / `:send` / `:push` — Compile directives into batch review file and blast to agent
- `:e` / `:edit` / `:reload` — Reload diff from disk
- `:hook [install|uninstall|status]` — Manage Git pre-commit hook directly from the TUI
- `:y` / `:yank` — Yank current code line
- `:yr` / `:ref` — Yank `#ref file:line` to clipboard
- `:<line_number>` — Jump directly to line number (e.g. `:42`)

---

## Agent Setup & MCP Client Configuration

`lgtm` embeds a Model Context Protocol (MCP) server over standard I/O for AI agents:

### Claude Code (`~/.claude/claude_desktop_config.json` or `.mcp.json`)
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

### Cursor (`.cursor/mcp.json`)
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

### Windsurf (`~/.codeium/windsurf/mcp_config.json`)
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

### Aider (`.aider.conf.yml`)
```yaml
lint-cmd: "lgtm check --strict"
test-cmd: "cargo test"
```

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

Run the workspace suite and the acceptance scenarios with:

```bash
# Unit, integration, and module behavior tests
make test

# Public API and application-launch BDD scenarios
cargo test --workspace --test bdd

# Automated BDD scenario catalog report
make bdd-report

# Repository quality gates
make check
```

The [BDD scenario catalog](docs/bdd-scenarios.md) maps acceptance criteria to tests,
including draft preservation, historical PR retrieval, posting recovery across restarts,
ambiguous remote write reconciliation, real-terminal journeys, and
GitHub/Azure request/response contracts. See [CONTRIBUTING.md](CONTRIBUTING.md#behavior-scenarios-bdd)
for test placement and the [scenario catalog](docs/bdd-scenarios.md)
for completed acceptance milestones.

Automated BDD reporting is available via `make bdd-report` (or `./scripts/bdd-report.sh`),
verifying 100% automation and pass rate across all 41 catalog criteria.

The in-app `T` key summarizes current test-risk alerts.

---

## Architecture

```
lgtm/
├── src/                      # `lgtm` binary: TUI + CLI
│   ├── main.rs               # CLI dispatcher
│   ├── cli.rs, headless.rs   # CLI definitions, headless runners & subcommands
│   ├── diagnostics.rs        # Opt-in JSONL timing & UI workload diagnostics
│   ├── event_loop.rs         # Bounded terminal/watcher/callback event loop
│   ├── app/                  # State machine, background coordinators & Vim motion engine
│   └── ui/                   # Ratatui rendering, viewport caches, commit forge & modals
├── tests/bdd.rs              # Acceptance scenarios launching the application
├── crates/
│   ├── lgtm-ast/src/         # tree-sitter parsing and queries (TS/JS, Swift, Python, Go, Java)
│   ├── lgtm-core/src/        # UI-free engine
│   │   ├── model.rs          # Diff data models
│   │   ├── debug.rs          # Structured JSONL timing recorder
│   │   ├── git/, pr/, forge/ # Git diff engine; PR providers (`forge/`: GitHub, Azure)
│   │   ├── scan.rs           # Full repository audit scanner
│   │   ├── guard/            # Security, complexity, symbols, API drift, dead code, arch, deps, duplicate
│   │   ├── findings/, baseline/ # Suppression/severity pipeline & baseline partitioning
│   │   ├── review/, outcome/ # Durable hunk/fold marks & directive outcome tracking
│   │   ├── comment/, turn/   # Directives; reviewed-file tracker (.lgtm/state.json)
│   │   ├── ipc/              # Multiplexer sinks (tmux, zellij, wezterm, kitty), OSC 52 clipboard
│   │   ├── diff_source.rs    # DiffSource trait
│   │   └── watcher/          # Filesystem watcher & debouncer
│   └── lgtm-mcp/src/lib.rs   # MCP JSON-RPC stdio server
├── scripts/
│   ├── bdd-report.sh         # Automated BDD scenario catalog report
│   ├── bdd-report.py         # Python reporting engine
│   ├── check-boundaries.sh   # Crate boundary lint
│   └── update-tap.sh         # Homebrew tap updater
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

