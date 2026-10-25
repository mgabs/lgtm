# Git Worktree Switcher Implementation Plan

> **Goal:** Enable reviewers to switch between Git worktrees inside `lgtm` via `:wt` / `:worktrees` modal and `:wt <name>` jump.

**Architecture:** 
- Core discovery in `crates/lgtm-core/src/git/` parsing `git worktree list --porcelain`.
- Interactive picker modal in `src/ui/modals/pickers.rs` (`ActiveView::WorktreePicker`).
- Command dispatch, tab completion, dirty state prompt, and engine re-initialization in `src/app/`.

**Spec:** [`specs/superpowers/specs/2026-10-25-worktree-switcher-design.md`](specs/superpowers/specs/2026-10-25-worktree-switcher-design.md)

---

### Task 1: Core Worktree Discovery (`lgtm-core`)

- [ ] **Step 1: Define `WorktreeInfo` struct**
  In `crates/lgtm-core/src/git/mod.rs` (or submodule):
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct WorktreeInfo {
      pub path: PathBuf,
      pub head: String,
      pub branch: Option<String>,
      pub is_bare: bool,
      pub is_current: bool,
  }
  ```
- [ ] **Step 2: Implement `GitEngine::list_worktrees()`**
  Execute `git worktree list --porcelain` and parse stdout into `Vec<WorktreeInfo>`. Flag `is_current` by comparing canonical paths against `self.repo_path`.
- [ ] **Step 3: Unit tests for worktree parsing**
  Add unit tests in `crates/lgtm-core/src/git/` verifying parsing with detached HEADs, main branch, and bare repositories.

---

### Task 2: Worktree App State & Command Dispatch

- [ ] **Step 1: Add `ActiveView::WorktreePicker`**
  Update `ActiveView` enum in `src/app/mod.rs`.
- [ ] **Step 2: Add `WorktreePickerState` to `App`**
  Track worktree list, filtered indices, and selected index.
- [ ] **Step 3: Implement `:wt` and `:worktrees` command**
  In `src/app/command.rs`:
  - `:wt` with no args: fetch worktrees, populate state, set `ActiveView::WorktreePicker`.
  - `:wt <arg>`: resolve worktree by path, branch, or basename. If valid, trigger `switch_worktree()`.
- [ ] **Step 4: Add tab completion for `:wt`**
  In `src/app/command.rs`, complete worktree branch names and directory basenames when typing `:wt `.

---

### Task 3: Worktree Switching & Dirty State Guard

- [ ] **Step 1: Implement `App::switch_worktree(path)`**
  - Reset PR mode (`app.pr.active_pr = None`) and `app.active_revision = None`.
  - Point `app.git_engine` and `app.comment_store` to new path.
  - Reset comment directives, reload diff snapshot via `app.reload_diff()`, reset cursor.
  - Rebind file watcher to new worktree root.
- [ ] **Step 2: Implement dirty directive confirmation prompt**
  If `app.comments.iter().any(|c| c.outcome.is_none())`, intercept switch with confirmation modal/prompt (`[s] Save`, `[d] Discard`, `[Esc] Cancel`).

---

### Task 4: UI Modal & Status Bar Badge

- [ ] **Step 1: Render Worktree Picker Modal**
  In `src/ui/modals/pickers.rs`, implement `render_worktree_picker`.
  Display path, branch, commit SHA, and active marker `*`.
- [ ] **Step 2: Add keyboard navigation for modal**
  Handle `j`/`k`, `Up`/`Down`, `Enter` to switch, `Esc`/`q` to close.
- [ ] **Step 3: Status bar worktree badge**
  In `src/ui/status.rs`, show `[wt: <name>]` indicating the current active worktree.

---

### Task 5: Verification & BDD Tests

- [ ] **Step 1: Run unit and clippy tests**
  `cargo test && cargo clippy --all-targets --all-features`
- [ ] **Step 2: Add integration BDD scenario**
  Verify switching worktrees reloads diffs and preserves directives safety.
