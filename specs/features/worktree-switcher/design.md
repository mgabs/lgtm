# Git Worktree Switcher (Design Document)

- **Date:** 2026-10-25
- **Status:** Draft / Ready for Builder Review
- **Feature:** Barebones Git Worktree Switcher (`:wt`, `:worktrees`)

---

## 1. Intent & Problem Statement

When orchestrating multiple concurrent AI coding agents, each agent operates in an isolated Git worktree (e.g., `.worktrees/feat-*` or siblings). Currently, reviewing an agent's work requires quitting `lgtm` and relaunching it from another directory.

The Worktree Switcher provides seamless, keyboard-driven context switching between Git worktrees directly inside the `lgtm` TUI without restarting the application or losing uncommitted review directives.

---

## 2. Goals, Non-Goals, and Anti-Goals

### Goals
- **Fast context switching**: Switch active repository root and reload diffs/file-tree in < 100ms.
- **Data safety**: Guard pending review directives with a prompt (Save / Discard / Cancel) before switching worktrees.
- **Clear visibility**: Always display the active worktree name in the status bar / header (`[wt: <name>]`).
- **Interactive & direct CLI navigation**: Support both interactive picker modal (`:wt` / `:worktrees`) and direct command jumps (`:wt <name|path>`) with tab completion.

### Non-Goals (YAGNI)
- No worktree lifecycle management (creating, removing, moving, locking worktrees) inside the TUI; manage worktrees via git CLI or external agent scripts.
- No fuzzy-scored agent discovery heuristics or metadata scraping.
- No cross-worktree batch export or multiplexed multi-worktree diff views.

### Anti-Goals
- Never merge or interleave diff hunks across multiple worktrees. Exactly one worktree diff is active and rendered at any time.

---

## 3. Architecture & User Experience

### 3.1 Worktree Discovery (`lgtm_core::git`)
- Add `WorktreeInfo` struct:
  ```rust
  pub struct WorktreeInfo {
      pub path: PathBuf,
      pub head: String,
      pub branch: Option<String>,
      pub is_bare: bool,
      pub is_current: bool,
  }
  ```
- Method on `GitEngine`: `list_worktrees(&self) -> Result<Vec<WorktreeInfo>>`
- Implementation: Shell out to `git worktree list --porcelain` in the repository root, parse blocks separated by double newlines or lines starting with `worktree `, `HEAD `, `branch `.

### 3.2 Command Navigation & Autocompletion
- `:wt` or `:worktrees`:
  - Opens `ActiveView::WorktreePicker`.
  - Fetches worktree list synchronously or cached from engine.
- `:wt <name|path>`:
  - Finds matching worktree by branch name, directory basename, or exact path.
  - Switches immediately if clean; prompts if directives are dirty.
- Autocomplete in `:wt <tab>`:
  - Completes available worktree directory names or branch names.

### 3.3 Interactive Modal (`ActiveView::WorktreePicker`)
- Centered popup modal styled consistently with `PrPicker` and `FilePicker`.
- Lists worktrees with:
  - Branch name (e.g. `feat-login`) or `(detached HEAD)`
  - Relative or canonical path
  - Short commit hash
  - Indicator for current active worktree (e.g., `*` or `(current)`)
- Keybindings:
  - `j` / `Down`: Move selection down
  - `k` / `Up`: Move selection up
  - `Enter`: Switch to selected worktree
  - `Esc` / `q`: Close modal without switching

### 3.4 Dirty State & PR Reset Guard
- **Pending Directives Definition**:
  - A worktree has pending review state if `app.comments.iter().any(|c| c.outcome.is_none())` or comment draft input is open.
  - Prompt options:
    - `[s] Export/Save and switch` -> saves/exports pending directives and proceeds.
    - `[d] Discard and switch` -> discards unsaved/unexported directives in memory and proceeds without modifying remote/agent sinks.
    - `[Esc/c] Cancel` -> cancels switch, stays on current worktree.
- **PR & Revision Reset**:
  - Switching worktree immediately exits PR mode (`app.pr.active_pr = None`) and clears `app.active_revision = None`.
  - Always loads the target worktree's local working directory diff against its own HEAD.

### 3.5 Worktree Resolution & Bare Handling
- **Bare Worktrees**: Filtered out from interactive picker. If targeted by direct `:wt <path>`, halts with error: `Cannot switch to bare worktree`.
- **Precedence for `:wt <arg>`**:
  1. Exact path match (`target == worktree.path`)
  2. Branch name match (`Some(target) == worktree.branch`)
  3. Directory basename match (`target == worktree.path.file_name()`)
  If ambiguous, reports `Ambiguous worktree target: <arg>. Use :wt to pick.`

### 3.5 Switching Pipeline (`switch_worktree(path)`)
When switching to target worktree path:
1. Update `app.git_engine = GitEngine::new(target_path)`.
2. Update `app.comment_store = CommentStore::new(target_path)`.
3. Clear in-memory directives (`app.comments.clear()`) and load target worktree's directives if existing.
4. Update file watcher to watch new root path.
5. Recompute diff snapshot via `app.reload_diff()`.
6. Reset cursor and file tree selection to top.
7. Update active worktree label.

### 3.6 Status Bar Indicator
- When inside a secondary worktree or any worktree, display active worktree in status bar: `[wt: <basename>]`.

---

## 4. Testing Strategy
- Unit tests in `lgtm-core` verifying `list_worktrees` parsing against porcelain output and real temporary git repos with worktrees.
- CLI / TUI tests for `:wt` command handling and switching mechanics.
- BDD test for dirty state guard preventing directive loss during worktree switch.
