# Specification: Clean Commands and Shortcuts

## Capability: `clean-commands-shortcuts`

### 1. Requirements

#### REQ-CCS-01 (Canonical CLI Command Taxonomy & Deprecations)
- The CLI command suite MUST be consolidated around 9 canonical commands.
- Deprecated subcommands MUST continue to execute identically to their canonical replacements but MUST print a deprecation warning to `stderr`.

| Canonical Command | Arguments & Flags | Deprecated / Secondary Aliases | Deprecation Warning Notice | Description |
| :--- | :--- | :--- | :--- | :--- |
| `lgtm [REVISION]` | `[REVISION]`, `--pr <N>`, `-p/--agent-pane`, `--agent-sink`, `-r/--repo` | `lgtm diff [REV]`, `lgtm status [REV]` | `Warning: 'lgtm status' is deprecated and will be removed in a future release. Use 'lgtm' or 'lgtm diff'.` | Launch interactive diff reviewer TUI (default entrypoint). |
| `lgtm check` | `-r/--repo`, `-f/--format <markdown\|json\|sarif>`, `-o/--output <PATH>`, `--strict`, `--show-suppressed`, `--no-user-config`, `--rev <REV>` | `lgtm ci` | `Warning: 'lgtm ci' is deprecated. Use 'lgtm check --no-user-config'.` | Run headless CI/pre-commit check on working tree or revision diff. |
| `lgtm audit` | `-r/--repo` | `lgtm scan --tui`, `lgtm scan repo` | `Warning: 'lgtm scan --tui' is deprecated. Use 'lgtm audit'.` | Launch interactive TUI Codebase Audit mode (all repository issues). |
| `lgtm scan` | `-r/--repo`, `-o/--output <PATH>`, `-f/--format <markdown\|json\|sarif>` | — | — | Run headless full repository security and architectural scan to file or stdout. |
| `lgtm pr` | `[NUMBER]`, `list [--state]`, `post [PR] [--yes]`, `reply <PR> <THREAD> <BODY>`, `resolve <PR> <THREAD> [--unresolve]` | `lgtm prs` (redirects to `lgtm pr list`) | `Warning: 'lgtm prs' is deprecated. Use 'lgtm pr list'.` | Pull request inspection and remote thread operations. |
| `lgtm review` | `export [TARGET]`, `import <FILE>`, `-r/--repo` | — | — | Export or import review bundles and directive state. |
| `lgtm mcp` | `-r/--repo` | `lgtm serve --mcp` | `Warning: 'lgtm serve --mcp' is deprecated. Use 'lgtm mcp'.` | Launch Model Context Protocol stdio server for AI agents. |
| `lgtm gate` | `-r/--repo`, `-t/--timeout <SEC>`, `-m/--message <MSG>` | — | — | Human-in-the-loop review approval breakpoint gate for autonomous agents. |
| `lgtm hook` | `<install\|uninstall\|status>`, `-r/--repo` | — | — | Manage Git pre-commit hooks. |
| `lgtm config` | `--show`, `-r/--repo`, `--no-user-config` | — | — | Inspect effective configuration values. |
| `lgtm init` | `-r/--repo` | — | — | Initialize `.lgtm.toml` starter configuration. |
| `lgtm baseline`| `<update>`, `--prune`, `-r/--repo` | — | — | Manage findings baseline (`.lgtm-baseline.json`). |

##### Deprecation Rules:
1. When a deprecated alias is invoked, the command executes the canonical path without failing.
2. The warning message is written to `stderr` with format: `⚠️  Notice: 'lgtm <alias>' is deprecated and will be removed in v0.5.0. Use 'lgtm <canonical>' instead.`
3. Exit codes and stdout outputs are strictly preserved for CI script compatibility.

---

#### REQ-CCS-02 (Canonical TUI Ex Command Taxonomy & Argument Specs)
- All TUI `:` Ex commands MUST be registered in a single static command registry.
- Commands MUST belong to one of 5 functional categories.
- Command names MUST be unique; aliases MUST route directly to the canonical definition.
- The ambiguous alias `:r` for review batch dispatch MUST BE REMOVED to eliminate collision with normal mode `r` (reply).

| Category | Canonical Command | Aliases | Arguments | Description & Side Effects |
| :--- | :--- | :--- | :--- | :--- |
| **1. Navigation & Layout** | `:quit` | `:q` | None | Request safe exit (prompts if unsaved comments). |
| | `:quit!` | `:q!` | None | Force quit immediately without saving. |
| | `:write` | `:w` | `[path]` | Save review directives to `.lgtm/comments.jsonl` (or write audit report if in Audit view). |
| | `:exit` | `:wq`, `:x` | None | Save directives and request quit. |
| | `:diff` | `:rev`, `:range` | `[revision]` | Switch to diff view, or load revision/range (e.g. `:diff HEAD~1`). |
| | `:split` | `:sp`, `:vsp`, `:vsplit` | None | Toggle Side-by-Side vs Unified diff mode. |
| | `:drawer` | `:tree`, `:sidebar` | None | Toggle file tree drawer sidebar. |
| | `:find` | `:f`, `:files` | None | Open fuzzy file picker modal. |
| | `:preview` | `:md` | None | Toggle rendered Markdown preview for markdown diffs. |
| | `:reload` | `:e`, `:edit` | None | Reload working tree git diff from disk. |
| | `:<line>` | — | None | Jump cursor to line number (e.g. `:42`). |
| **2. Directives & Threads** | `:comments` | `:c`, `:directives`, `:todo` | `[recheck\|orphans]` | Open directives drawer (or run directive diagnostics). |
| | `:reply` | `:rep` | `[text]` | Reply to thread or directive under cursor. |
| | `:resolve` | `:res` | `[all]` | Toggle resolved state on cursor item (or `:resolve all`). |
| | `:unresolve` | — | `[all]` | Mark cursor item or all directives as active/unresolved. |
| | `:clear` | — | None | Clear all review directives. |
| | `:clear-resolved` | — | None | Remove only resolved review directives. |
| **3. Agent Steering** | `:send` | `:review`, `:push` | None | Compile directives into batch prompt and send to active agent sink. |
| | `:prompt` | `:context` | None | Open agent prompt / gate context modal. |
| | `:approve` | `:lgtm` | None | Approve waiting agent breakpoint gate. |
| | `:reject` | — | None | Revert current hunk and re-prompt agent with reason. |
| | `:reject-all` | `:reject-gate` | None | Reject waiting agent gate entirely. |
| | `:pane` | `:panes` | `[id]` | View or set target multiplexer pane ID (e.g. `:pane %2`). |
| | `:sink` | — | `[name]` | View or set agent steering sink (`tmux`, `zellij`, `wezterm`, `kitty`, `clipboard`, `auto`). |
| | `:inject` | — | `[all]` | Inject selected audit violation (or `all`) to agent sink. |
| **4. PR & Remote** | `:pr` | — | `<num> \| post [--yes]` | Load PR diff by number, or post pending directives to PR. |
| | `:prs` | — | `[open\|closed\|all]` | Open remote PR picker (default: `open`). |
| | `:threads` | `:thread` | `[view]` | Refresh and open remote PR thread discussion modal. |
| | `:checks` | — | None | Fetch latest CI status checks for active PR. |
| **5. Audit & Git Lifecycle** | `:audit` | — | None | Run whole codebase security/architecture audit in TUI. |
| | `:scan` | `:issues` | `[repo]` | Scan diff issues (or `:scan repo` for full codebase). |
| | `:risk` | — | None | Run security and fraud risk scan. |
| | `:filter` | `:sev` | `<crit\|high\|all\|1\|2\|3>` | Filter audit alerts by severity level. |
| | `:commit` | `:ci` | `[message]` | Open Commit Forge modal (or commit directly with `[message]`). |
| | `:worktrees` | `:wt` | `[name\|path]` | Open worktree switcher modal (or jump directly to target). |
| | `:hook` | — | `<install\|uninstall\|status>` | Inspect or manage Git pre-commit hook. |
| | `:editor` | — | `[name]` | View or configure external editor command (`nvim`, `code`, etc.). |
| | `:export` | — | `[path]` | Export audit report to markdown (default: `lgtm-report.md`). |
| | `:hidden` | — | None | Display summary of baselined or suppressed findings. |
| | `:help` | `:h`, `:?` | None | Open interactive documentation modal. |

##### Error Handling & Status Feedback:
- Unrecognized command: `self.status_message = Some(format!("Unknown command: :{} (type :help for command list)", cmd_name));`
- Invalid arguments: print specific usage string (e.g. `Usage: :filter <crit|high|all>`).

---

#### REQ-CCS-03 (Canonical TUI Keyboard Shortcuts Taxonomy)
- Keyboard shortcuts MUST follow strict state boundaries: Normal Diff Mode, Visual Mode, Leader Chords, and Modal States.
- Every single key MUST be deterministic and reachable.

##### 1. Normal Diff Mode (Single Key Actions)
- **Motions**:
  - `j` / `k` or `↓` / `↑`: Move cursor down / up by line.
  - `h` / `l` or `←` / `→`: Pan column left / right (or cycle panes in Audit view).
  - `gg` / `G`: Jump to top / bottom of current diff.
  - `Ctrl-d` / `Ctrl-u`: Scroll half-page down / up (15 lines).
  - `Ctrl-f` / `Ctrl-b` or `PgDn` / `PgUp`: Scroll full page down / up (25 lines).
  - `}` / `{`: Jump to next / previous paragraph / hunk boundary.
  - `0` / `$`: Jump to first / last line of current file diff.
- **Git Staging (Symmetric Hierarchy)**:
  - `s`: Stage current hunk (or visual line selection) to Git index.
  - `S`: Stage entire current file to Git index.
  - `u`: Unstage current hunk (or visual line selection) from Git index.
  - `U`: Unstage entire current file from Git index.
  - `x`: Discard changes in current hunk (or delete directive if on comment line).
  - `X`: Discard ALL changes in current file (`git restore <file>`) — **RESTORED & UNBLOCKED**.
  - `V`: Toggle visual line selection mode.
  - `C`: Open interactive Commit Forge modal for staged changes.
- **Review Directives & Agent Steering**:
  - `c`: Open comment input box to anchor inline directive at cursor line.
  - `r`: Thread a reply to comment or PR thread at cursor (or open new directive if none).
  - `R`: Toggle resolved status of comment or PR thread at cursor.
  - `A`: Approve agent breakpoint gate (`.lgtm/gate.json`).
  - `P`: Open agent prompt / gate context modal.
  - `T`: Summarize test mutation / test-risk alerts.
  - `Enter`: Inject reference anchor `#ref file:line [rule]` to target agent multiplexer pane.
  - `y`: Yank raw current line content to clipboard.
- **Layout & Modal Triggers**:
  - `Tab`: Toggle file drawer sidebar.
  - `d`: Toggle Unified vs Side-by-Side split diff view.
  - `m`: Toggle rendered Markdown preview for markdown files.
  - `M`: Toggle review checkmark for current hunk.
  - `v`: Toggle review checkmark for current file (persisted in `.lgtm/state.json`).
  - `o` / `e`: Open file at current line in `$EDITOR`.
  - `f` / `Ctrl-p`: Open fuzzy file switcher modal.
  - `t`: Open remote PR threads discussion modal.
  - `p`: Open multiplexer pane picker modal.
  - `:`: Open Ex command line with Wildmenu completion.
  - `/`: Open interactive regex diff search.
  - `n` / `N`: Jump to next / previous search match.
  - `*` / `#`: Search word under cursor forward / backward.
  - `?` or `h`: Open Help Modal.
  - `q`: Request quit.

##### 2. Leader `<Space>` Chords
Leader chords provide quick mnemonic access to high-value actions without leaving the home row:
- `<Space> c`: Anchor inline review directive.
- `<Space> l`: Open directives drawer list (`:comments`).
- `<Space> r`: Toggle resolve status on cursor comment/thread.
- `<Space> d`: Delete review directive anchored on current line.
- `<Space> s`: Compile batch review directives and dispatch to agent sink (`:send`).
- `<Space> x`: Reject current hunk and re-prompt agent with reason (`reject_hunk_and_reprompt`).
- `<Space> y`: Yank reference anchor `#ref file:line` to clipboard via OSC 52.
- `<Space> t`: Open remote PR threads modal.
- `<Space> p`: Open multiplexer target pane picker.
- `<Space> P`: Open remote PR picker modal.
- `<Space> w`: Jump to next security alert / guard warning.

##### 3. Vim Multi-Key Prefixes
- **Bracket Jumps (`]` and `[`)**:
  - `]f` / `[f` (or chained `]` / `[`): Next / Previous changed file.
  - `]h` / `[h`: Next / Previous diff hunk.
  - `]c` / `[c`: Next / Previous review directive / comment.
  - `]t` / `[t`: Next / Previous remote PR thread.
  - `]w` / `[w`: Next / Previous security alert / guard warning.
  - `]u` / `[u`: Next / Previous unreviewed file.
- **Folding Chords (`z` prefix)**:
  - `zc`: Fold current hunk.
  - `zo`: Unfold current hunk.
  - `za`: Toggle fold on current hunk.
  - `zM` / `zm`: Fold ALL hunks in current file.
  - `zR` / `zr`: Unfold ALL hunks in current file.
  - `zz`: Center viewport on cursor line.
  - `zt`: Scroll viewport so cursor line is at top.
  - `zb`: Scroll viewport so cursor line is at bottom.
- **Canned Directive Chords (`c` prefix)**:
  - `ca`: Add `⚡ [COVERAGE] Add unit/regression tests covering these branches.`
  - `cs`: Add `🔒 [SECURITY] Ensure inputs are sanitized, parameterized, and auth checks enforced.`
  - `cr`: Add `🧹 [REFACTOR] Simplify logic, reduce cognitive nesting, eliminate duplication.`
  - `cp`: Add `🚀 [PERF] Optimize hot-path allocations and database query batching.`
  - `cw`: Add `❓ [WHY] Explain rationale and verify architectural invariants.`
  - `cc` or `c <Enter>`: Open blank directive prompt.

---

#### REQ-CCS-04 (Reconciliation of Shadowed & Conflicting Shortcuts)

##### Issue 1: `KeyCode::Char('X')` Shadowing Bug
- **Root Cause**: `src/app/input.rs:264` had:
  ```rust
  (KeyModifiers::SHIFT, KeyCode::Char('X')) | (KeyModifiers::NONE, KeyCode::Char('X')) => {
      self.reject_hunk_and_reprompt();
  }
  ```
  This unconditionally swallowed `X` before line 405 (`(_, KeyCode::Char('X')) => self.discard_current_file_changes()`), making line 405 100% dead.
- **Resolution**:
  1. Remove lines 264–267.
  2. Retain line 405: `X` strictly invokes `self.discard_current_file_changes()`, restoring symmetric Git staging operations (`s`/`S`, `u`/`U`, `x`/`X`).
  3. Bind `reject_hunk_and_reprompt()` to `<Space>x` in the leader match block, and to Ex command `:reject`.

##### Issue 2: Collision of `:r` vs `r`
- **Root Cause**: In diff mode, `r` opens comment reply. In Ex mode, `:r` dispatched a batch review send to the agent, creating severe user surprises when expecting `:reload` or `:reply`.
- **Resolution**:
  1. Canonical command for agent batch review dispatch is `:send` (with aliases `:review` and `:push`).
  2. Remove alias `:r` from batch review dispatch.
  3. Ex command `:reply` (alias `:rep`) handles thread replies.
  4. Ex command `:reload` (alias `:e`, `:edit`) reloads the diff from disk.
  5. Leader `<Space>s` triggers batch review send; leader `<Space>r` toggles comment resolution.

##### Issue 3: Disambiguation of `m` and `M`
- **Resolution**:
  1. `m`: Toggles markdown rendered preview in diff view (`toggle_markdown_preview`); toggles inspector maximization in Codebase Audit view (`audit.maximized_inspector`).
  2. `M`: Toggles review status on current hunk (`toggle_review_current_hunk`).
  3. `v`: Toggles review status on current file (`toggle_review_current_file`).
  4. `V`: Toggles visual line selection mode (`toggle_visual_mode`).

---

#### REQ-CCS-05 (Help Modal Synchronization Across All 7 Tabs)
- All 7 tabs in `src/ui/help.rs` MUST be audited and updated to match the canonical specifications above:
  - **Tab 1 (Overview & Modes)**: Update mode summary; remove obsolete alias mentions.
  - **Tab 2 (Motions & Navigation)**: Document `j/k`, `gg/G`, `Ctrl-d/u/f/b`, `}/ {`, `0/$`, bracket jumps (`]f`, `]h`, `]c`, `]t`, `]w`, `]u`).
  - **Tab 3 (Review & Agent Steering)**: Document `c`, `r`, `R`, `<Space>s` (:send), `<Space>x` (:reject), `A` (:approve), `P` (:prompt), `Enter`, `y`, `yr`.
  - **Tab 4 (Staging & Commit Forge)**: Explicitly document the symmetric hierarchy:
    - `s` (stage hunk) vs `S` (stage file)
    - `u` (unstage hunk) vs `U` (unstage file)
    - `x` (discard hunk) vs `X` (discard file — git restore)
    - `C` / `:commit` (Commit Forge)
  - **Tab 5 (Folding)**: Document `zc`, `zo`, `za`, `zM`, `zR`, `zz`, `zt`, `zb`.
  - **Tab 6 (Guards & Static Analysis)**: Retain security guardrail reference and audit actions.
  - **Tab 7 (Ex-Commands & Headless CI)**: Complete reference of canonical Ex commands grouped by category; accurate headless CLI reference (`lgtm check`, `lgtm audit`, `lgtm scan`).
