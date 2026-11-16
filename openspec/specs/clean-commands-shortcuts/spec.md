# clean-commands-shortcuts Specification

## Purpose

A small, consistent CLI command set, a single TUI Ex-command registry, and deterministic keyboard shortcuts with no shadowed or colliding keys, documented accurately in the help modal.

## Requirements

### Requirement: Canonical CLI Commands
The CLI SHALL expose only these canonical commands: `lgtm [REVISION]` (interactive diff TUI, default entrypoint), `check` (headless diff audit), `audit` (interactive repository issues TUI), `scan` (headless full-repo security & architecture report), `pr` (`[NUMBER]` and `list [--state]`), `review` (`export`/`import`), `mcp` (stdio Model Context Protocol server), `gate` (agent approval barrier), `hook` (git hook manager), `config` (configuration inspector), `init` (configuration initializer), and `baseline` (findings baseline manager).

#### Scenario: Default entrypoint
- **WHEN** the user runs `lgtm` or `lgtm HEAD~1`
- **THEN** the interactive diff reviewer launches on the working tree or that revision

#### Scenario: Headless check
- **WHEN** the user runs `lgtm check --format sarif --output r.sarif`
- **THEN** a SARIF report is written and the exit code reflects the gate result

#### Scenario: Review command requires subcommand
- **WHEN** the user runs `lgtm review` without `export` or `import`
- **THEN** the CLI exits with an argument error and shows command usage

### Requirement: Single Ex-Command Registry
All TUI `:` commands SHALL be registered in one static registry, each in exactly one of five categories (Navigation & Layout, Directives & Threads, Agent Steering, PR & Remote, Audit & Git Lifecycle). Command names MUST be unique and aliases MUST route to the canonical definition. The `:r` alias for batch review dispatch MUST NOT exist.

#### Scenario: Alias routing
- **WHEN** the user runs `:q`, `:wq` or `:sp`
- **THEN** each behaves as `:quit`, `:exit` or `:split` respectively

#### Scenario: Removed ambiguous alias
- **WHEN** the user runs `:r`
- **THEN** it is not accepted as the batch review dispatch

### Requirement: Ex-Command Error Feedback
An unrecognized command SHALL show `Unknown command: :<name> (type :help for command list)`, and invalid arguments MUST show a usage string such as `Usage: :filter <crit|high|all>`.

#### Scenario: Unknown command
- **WHEN** the user runs `:bogus`
- **THEN** the status bar shows `Unknown command: :bogus (type :help for command list)`

#### Scenario: Bad argument
- **WHEN** the user runs `:filter nonsense`
- **THEN** the status bar shows the `:filter` usage string

### Requirement: Normal-Mode Keyboard Shortcuts
Normal diff mode SHALL provide deterministic single-key actions: motions (`j/k`, `h/l`, `gg/G`, `Ctrl-d/u` half page, `Ctrl-f/b` and `PgDn/PgUp` full page, `{`/`}`, `0/$`), staging (`s/S`, `u/U`, `x/X`, `V`, `C`), review and steering (`c`, `r`, `R`, `A`, `P`, `T`, `Enter`, `y`) and layout and modal triggers (`Tab`, `d`, `m`, `M`, `v`, `o`/`e`, `f`/`Ctrl-p`, `t`, `p`, `:`, `/`, `n/N`, `*`/`#`, `?`/`h`, `q`).

#### Scenario: Symmetric staging
- **WHEN** the cursor is in a hunk and `s` is pressed
- **THEN** the hunk is staged
- **WHEN** `S` is pressed
- **THEN** the whole file is staged

#### Scenario: Discard hunk vs file
- **WHEN** `x` is pressed on a hunk
- **THEN** only that hunk's changes are discarded
- **WHEN** `X` is pressed
- **THEN** all changes in the current file are restored from Git

### Requirement: Leader and Prefix Chords
`<Space>` leader chords SHALL provide `c` anchor directive, `l` directives list, `r` toggle resolve, `d` delete directive, `s` send batch, `x` reject hunk and re-prompt, `y` yank `#ref`, `t` PR threads, `p` pane picker, `P` PR picker and `w` next alert. Bracket jumps (`]`/`[` with `f`, `h`, `c`, `t`, `w`, `u`), folding chords (`zc`, `zo`, `za`, `zM`, `zR`, `zz`, `zt`, `zb`) and canned directive chords (`ca`, `cs`, `cr`, `cp`, `cw`, `cc`) MUST be supported.

#### Scenario: Reject via leader
- **WHEN** `<Space>` then `x` is pressed on a hunk
- **THEN** the hunk is reverted and the agent is re-prompted with the reason

#### Scenario: Canned directive
- **WHEN** `cs` is pressed
- **THEN** a security directive is added at the cursor line

### Requirement: Shortcut Collision Resolution
`X` SHALL only discard the current file's changes and MUST NOT be shadowed by hunk rejection, which is bound to `<Space>x` and `:reject`. `m` MUST toggle Markdown preview (or the inspector maximization in Audit view), `M` MUST toggle the current hunk's reviewed mark, `v` the current file's reviewed mark and `V` visual line selection. Batch review dispatch MUST be `:send` (aliases `:review`, `:push`).

#### Scenario: X is not shadowed
- **WHEN** `X` is pressed in diff mode
- **THEN** the file is restored and no agent re-prompt is sent

#### Scenario: m versus M
- **WHEN** `m` is pressed on a Markdown file
- **THEN** preview toggles
- **WHEN** `M` is pressed
- **THEN** the current hunk's reviewed mark toggles

### Requirement: Help Modal Matches Behavior
The help modal's seven tabs (Overview & Modes, Motions & Navigation, Review & Agent Steering, Staging & Commit Forge, Folding, Guards & Static Analysis, Ex-Commands & Headless CI) SHALL document the canonical commands, shortcuts and symmetric staging hierarchy above, with no obsolete aliases.

#### Scenario: Staging tab
- **WHEN** the Staging & Commit Forge tab is opened
- **THEN** it lists `s/S`, `u/U`, `x/X` as hunk/file pairs and `C` / `:commit`

### Requirement: REQ-CLI-1 Initialization Repository Target
The `init` subcommand MUST initialize the repository selected by `--repo` when supplied, and MUST initialize the current working directory when `--repo` is omitted. Top-level `--init` flag SHALL NOT be accepted.

#### Scenario: Initialize selected repository
- **WHEN** the user runs `lgtm init --repo <path>`
- **THEN** initialization targets `<path>` and writes starter configuration there

#### Scenario: Initialize default repository
- **WHEN** the user runs `lgtm init` without `--repo`
- **THEN** initialization targets the current directory

#### Scenario: Top-level init flag rejected
- **WHEN** the user runs `lgtm --init`
- **THEN** argument parsing fails with an unrecognized option error

### Requirement: REQ-GATE-1 Tick Unticked Review Gate Checkbox on Approve
When the reviewer invokes `:approve`, `:lgtm`, or `A` in `Diff` or `MarkdownPreview` view on a local working-tree file whose cursor line is an unticked `- [ ]` item inside a `## Review Gate` section, the system MUST replace `[ ]` with `[x]` on disk, reload the diff, and NOT modify `.lgtm/gate.json`.

#### Scenario: Tick Review Gate checkbox in Diff view
- **WHEN** the reviewer views a local working-tree diff of a markdown file with cursor on `- [ ] **Gate 1 Approval**: ...` under `## Review Gate` and presses `A` or runs `:approve`
- **THEN** `[ ]` on that line is replaced with `[x]` on disk, the diff reloads, and `.lgtm/gate.json` remains unchanged

#### Scenario: Tick Review Gate checkbox in MarkdownPreview view
- **WHEN** the reviewer views a local working-tree markdown file in `MarkdownPreview` with cursor on an unticked `- [ ]` line under `## Review Gate` and presses `A` or runs `:lgtm`
- **THEN** `[ ]` on that line is replaced with `[x]` on disk, the preview reloads, and `.lgtm/gate.json` remains unchanged

#### Scenario: Preserve CRLF line endings when ticking
- **WHEN** the target markdown file uses CRLF (`\r\n`) line endings
- **THEN** ticking `[ ]` to `[x]` preserves all CRLF line endings in the file

### Requirement: REQ-GATE-2 Two-Press Gate Approval
If the cursor is not on an unticked `- [ ]` line inside a `## Review Gate` section of a local working-tree file, invoking `:approve`, `:lgtm`, or `A` MUST fall through to `approve_gate()` and approve `.lgtm/gate.json` if present.

#### Scenario: Second press after ticking approves .lgtm/gate.json
- **WHEN** the reviewer has ticked the Review Gate checkbox on the first press (line is now `- [x]`) and presses `A` or runs `:approve` a second time while `.lgtm/gate.json` is waiting
- **THEN** `.lgtm/gate.json` is updated with `"status": "approved"` and the status bar reports gate approval

#### Scenario: Non-gate checkbox is not ticked
- **WHEN** the cursor is on `- [ ]` outside a `## Review Gate` section (e.g., under `## Tasks`)
- **THEN** the file on disk is not modified and the command falls through to `approve_gate()`

### Requirement: REQ-GATE-3 Guard Against Historical or Deleted Targets
The system MUST NOT edit files on disk when viewing a PR diff, a historical revision diff, or a deleted diff line.

#### Scenario: Historical revision or PR diff falls through
- **WHEN** the reviewer views a PR diff or `:diff <rev>` and invokes `:approve` on a `## Review Gate` `- [ ]` line
- **THEN** no working-tree file is modified and the command falls through to `approve_gate()`

#### Scenario: Deleted line falls through
- **WHEN** the cursor is on a deleted `- [ ]` line in Diff or MarkdownPreview view
- **THEN** no working-tree file is modified and the command falls through to `approve_gate()`

### Requirement: NFR-GATE-1 Keystroke Responsiveness
Checking whether the cursor line is an unticked Review Gate checkbox MUST NOT run on per-frame render paths and MUST execute only on explicit `:approve`, `:lgtm`, or `A` invocation.

#### Scenario: Zero per-frame overhead
- **WHEN** rendering frames in Diff or MarkdownPreview view without pressing `A` or running `:approve`
- **THEN** no disk reads or Review Gate parsing occur during rendering
