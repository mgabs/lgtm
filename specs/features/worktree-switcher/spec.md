# Specification: Git Worktree Switcher

## Capability: `worktree-switcher`

### 1. Requirements

- **REQ-WT-01 (Worktree Discovery)**:
  - `GitEngine::list_worktrees()` MUST execute `git worktree list --porcelain` in the repository root and parse stdout into structured `WorktreeInfo` records.
  - Each `WorktreeInfo` MUST contain:
    - `path`: Canonical `PathBuf` of the worktree directory.
    - `head`: String representing HEAD commit SHA (abbreviated or full).
    - `branch`: Optional string containing checked-out branch name (e.g. `refs/heads/feature-x` normalized to `feature-x`).
    - `is_bare`: Boolean indicating if worktree is bare repository.
    - `is_current`: Boolean indicating if worktree matches current working directory / repo root.

- **REQ-WT-02 (Interactive Picker Modal)**:
  - Ex-command `:wt` or `:worktrees` with no arguments MUST open `ActiveView::WorktreePicker`.
  - The modal MUST display all discovered worktrees with:
    - Active status badge (`*` or checkmark for current worktree).
    - Branch name or `(detached HEAD @ <sha>)`.
    - Relative or canonical path.
    - Commit hash and commit subject line when available.
  - Navigation controls:
    - `j` / `Down`: Move selection down (clamped to list length).
    - `k` / `Up`: Move selection up (clamped to 0).
    - `Enter`: Select worktree and initiate context switch.
    - `Esc` / `q`: Close picker modal without switching.

- **REQ-WT-03 (Direct Command Jump & Autocompletion)**:
  - Ex-command `:wt <target>` MUST attempt direct resolution:
    1. Match exact branch name.
    2. Match worktree directory basename (e.g. `feat-markdown-preview`).
    3. Match canonical path prefix/exact path.
  - If a single match is resolved, initiate context switch immediately.
  - If no match or multiple ambiguous matches exist, display error in status bar and keep picker open.
  - Typing `:wt <Tab>` in command mode MUST auto-complete available worktree names and directory basenames.

- **REQ-WT-04 (Single-Diff Isolation)**:
  - Switching worktrees MUST re-initialize `GitEngine` with target worktree root path.
  - File watcher (`DebouncedWatcher`) MUST be pointed to the new worktree path.
  - Existing diff snapshots, row layout caches, and file trees MUST be dropped and reloaded for the target worktree.
  - Exactly ONE worktree diff is active at any time; no cross-worktree diff merging.

- **REQ-WT-05 (Dirty Directives Guard)**:
  - When switching worktrees, if uncommitted directives or comments exist in `.lgtm/comments.jsonl`:
    - Display confirmation prompt: `Pending directives exist: [S]ave & Switch, [D]iscard & Switch, [C]ancel`.
    - `s` / `S`: Save current directives to disk, then switch.
    - `d` / `D`: Discard pending directives, then switch.
    - `c` / `C` / `Esc`: Abort switch; remain in current worktree.

- **REQ-WT-06 (Visual Header Badge)**:
  - When active worktree is not root repository (or in any worktree), header/status bar MUST display active worktree badge: `[wt: <branch|name>]`.

---

### 2. Non-Functional Requirements & Performance Budgets

- **NFR-WT-01 (Context Switch Latency)**:
  - Worktree context switch (engine re-init, diff load, file tree build) MUST complete in $< 100\text{ ms}$ for repositories with $< 10,000$ files.
- **NFR-WT-02 (Zero-Allocation Navigation)**:
  - Navigating worktree picker (`j`/`k`) MUST NOT re-query filesystem or execute git subprocesses.
- **NFR-WT-03 (Thread Safety)**:
  - Worktree switch MUST cancel any inflight background diff/audit workers before rebinding repository path.

---

### 3. Assumptions

- **ASM-WT-01**: Git binary ($\ge 2.20$) supporting `git worktree list --porcelain` is available in `PATH`.
- **ASM-WT-02**: Worktrees are located on locally accessible filesystems.
- **ASM-WT-03**: Worktree creation, deletion, or locking is handled out-of-band by user or orchestration scripts (Ponytail / YAGNI rule).

---

### 4. Edge Cases & Errors

- **EDG-WT-01 (Missing Directory)**:
  - If a worktree listed by `git` was deleted from disk, display warning `Worktree path missing: <path>` and prune from picker.
- **EDG-WT-02 (Current Worktree Selected)**:
  - Selecting the currently active worktree closes the picker with no reload.
- **EDG-WT-03 (Bare Repository Root)**:
  - Bare root worktree is displayed as `(bare)` and disabled for direct diff review unless checking out a branch worktree.

---

### 5. Behavioral Scenarios (Given / When / Then)

#### Scenario 5.1: Open Worktree Picker
- **Given** repository has root worktree on `main` and worktree `.worktrees/feat-x` on `feat-x`
- **When** user types `:wt` and presses `Enter`
- **Then** modal `WorktreePicker` opens showing 2 entries with `main` marked active

#### Scenario 5.2: Direct Jump via Branch Name
- **Given** worktree `.worktrees/feat-x` on branch `feat-x` exists
- **When** user executes `:wt feat-x`
- **Then** application switches context to `.worktrees/feat-x` and diff reloads

#### Scenario 5.3: Dirty Directive Guard
- **Given** user has 2 unsaved review directives in current worktree
- **When** user selects another worktree in picker
- **Then** prompt `[S]ave / [D]iscard / [C]ancel` appears before switching
