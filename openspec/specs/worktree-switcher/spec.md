# worktree-switcher Specification

## Purpose

Let reviewers list and switch between Git worktrees from inside the TUI, showing exactly one worktree's diff at a time.

## Requirements

### Requirement: Worktree Discovery
The engine SHALL run `git worktree list --porcelain` in the repository root and parse it into records containing the canonical path, HEAD SHA, optional branch name (`refs/heads/` stripped), a bare flag, and a current flag.

#### Scenario: Parse worktrees
- **WHEN** the repository has a root worktree on `main` and a linked worktree on `feat-x`
- **THEN** two records are returned with branches `main` and `feat-x` and only the root marked current

### Requirement: Interactive Worktree Picker
`:wt` or `:worktrees` without arguments SHALL open a picker listing every worktree with a current-worktree badge, branch or `(detached HEAD @ <sha>)`, path, and commit hash and subject when available. `j`/`Down` and `k`/`Up` MUST move the selection (clamped), `Enter` MUST switch, and `Esc`/`q` MUST close without switching. Navigation MUST NOT re-query the filesystem or run git.

#### Scenario: Open the picker
- **WHEN** the repository has `main` and `.worktrees/feat-x` and the user runs `:wt`
- **THEN** the picker shows 2 entries with `main` marked active

#### Scenario: Close without switching
- **WHEN** the picker is open and `Esc` is pressed
- **THEN** the picker closes and the current worktree is unchanged

### Requirement: Direct Jump and Autocompletion
`:wt <target>` SHALL resolve the target by exact branch name, then worktree directory basename, then exact or prefix path, and switch immediately on a single match. With no match or an ambiguous match it MUST show an error in the status bar. `Tab` MUST complete worktree names and directory basenames.

#### Scenario: Jump by branch
- **WHEN** worktree `.worktrees/feat-x` on branch `feat-x` exists and the user runs `:wt feat-x`
- **THEN** the application switches to it and the diff reloads

#### Scenario: Unknown target
- **WHEN** the user runs `:wt no-such-branch`
- **THEN** the status bar shows `Worktree not found: no-such-branch`

### Requirement: Single-Diff Isolation
Switching worktrees SHALL re-initialize the Git engine and file watcher on the target root and drop and reload diff snapshots, layout caches and file trees. Exactly one worktree's diff MUST be active at a time. In-flight background diff or audit workers MUST be cancelled before the repository path is rebound.

#### Scenario: Switch reloads state
- **WHEN** the user switches from the root worktree to `feat-x`
- **THEN** the diff shows only `feat-x` changes and the watcher observes the new path

### Requirement: Pending Directives Guard
Switching while unsaved review directives exist SHALL prompt `Pending directives exist: [S]ave & Switch, [D]iscard & Switch, [C]ancel`. `s` MUST save then switch, `d` MUST discard then switch, and `c` or `Esc` MUST abort and stay.

#### Scenario: Guard prompt
- **WHEN** the user has 2 unsaved directives and selects another worktree
- **THEN** the prompt appears before switching
- **WHEN** `c` is pressed
- **THEN** the switch is aborted

### Requirement: Worktree Header Badge
When the active worktree is a linked worktree, the header or status bar SHALL display `[wt: <branch|name>]`.

#### Scenario: Badge in a linked worktree
- **WHEN** the active worktree is `feat-x`
- **THEN** the header shows `[wt: feat-x]`

### Requirement: Worktree Edge Cases
If a listed worktree is missing on disk, the picker SHALL warn `Worktree path missing: <path>` and omit it. Selecting the current worktree MUST close the picker without reloading. A bare root MUST be shown as `(bare)` and MUST NOT be switchable.

#### Scenario: Select current worktree
- **WHEN** the user selects the already-active worktree
- **THEN** the picker closes and no reload occurs

### Requirement: Worktree Switch Latency
A worktree switch (engine re-init, diff load, file tree build) MUST complete in under 100 ms for repositories with fewer than 10,000 files.

#### Scenario: Small repository switch
- **WHEN** switching in a repository with fewer than 10,000 files
- **THEN** the switch completes in under 100 ms
