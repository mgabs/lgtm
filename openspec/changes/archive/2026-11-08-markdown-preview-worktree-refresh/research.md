# Research & Technical Tradeoffs

## 1. Intent & Ground Truth
- **Problem**: When using Markdown Preview (`ActiveView::MarkdownPreview`), file modifications, additions, and deletions are not reflected in the file change drawer or the rendered preview. In Git worktrees (either started inside a worktree or switched via `:wt`), file updates and staged/committed changes fail to trigger live TUI refreshes.
- **Ground Truth Verified in Codebase**:
  1. `src/app/refresh.rs` (lines 51-55, 231-235): `local_ready_to_apply` and `apply_ready_local` reject background diff updates whenever `self.active_view != ActiveView::Diff`. When in `ActiveView::MarkdownPreview`, refreshes are rejected with `"Diff update ready; close this view or selection to apply"`, freezing the file change drawer.
  2. `src/ui/markdown.rs` (lines 24-27): `needs_rebuild` only checks `rendered_rows.is_empty()`, `cached_file_idx != Some(...)`, and `cached_width != width`. It ignores `diff_revision`. Even if a diff reloads, the rendered markdown preview never re-parses modified content unless the file selection or terminal width changes.
  3. `src/app/input.rs` (lines 980-997): `apply_worktree_switch` updates `git_engine` and `comment_store`, but does not re-point or recreate the file watcher for `target_path`. The watcher spawned in `src/main.rs:440` continues watching the previous directory.
  4. `crates/lgtm-core/src/watcher/mod.rs` (lines 14-27): `affects_diff` only inspects paths relative to `root`. In linked worktrees, `.git` is a gitdir pointer file and the actual index/HEAD live at `<common_dir>/worktrees/<name>/index` and `HEAD`. Changes to linked worktree indices are not recognized by `affects_diff` and are outside the worktree's filesystem tree.

## 2. Options Explored

### Option A: Unified View Invalidation & Dual-Path Worktree Watching (Recommended)
- **Description**:
  1. Extend `local_ready_to_apply()` and `apply_ready_local()` to permit applying diff updates while in `ActiveView::MarkdownPreview` (preserving markdown cursor position and selection).
  2. Invalidate `MarkdownPreviewState` (or track `cached_diff_revision`) on diff reload so modified markdown files re-render automatically.
  3. Enhance `RepoWatcher` / `GitEngine` to resolve both `workdir` and `git_dir` (handling linked worktree `worktrees/<name>/index` and `HEAD`).
  4. Allow `apply_worktree_switch` to rebind/re-spawn `RepoWatcher` on the new worktree path.
- **Pros**: Solves root cause for both markdown preview drawer updates and worktree live syncing without breaking pure-core boundary.
- **Cons**: Requires `App` to manage or re-spawn `RepoWatcher` channels and track gitdir paths.

### Option B: Markdown Preview Fallback with Manual Worktree Refresh
- **Description**:
  Allow diff updates in Markdown Preview only, leaving worktrees to rely on existing `:reload` command.
- **Pros**: Minimal code touched in `lgtm-core`.
- **Cons**: Violates existing spec (`worktree-switcher` Requirement: Single-Diff Isolation requires watcher to observe new path); leaves worktree live-reload broken.

### Option C: Global Event Bus with Periodic Git Status Polling
- **Description**:
  Replace filesystem notify watcher in worktrees with a 1-second `git status` polling thread.
- **Pros**: Avoids complex gitdir path resolution across git worktrees.
- **Cons**: High CPU/battery consumption on large repos; violates performance budgets (<1us keystroke, <16ms frame, silent background).

## 3. Tradeoff Matrix

| Criteria | Option A (Unified Invalidation + Dual-Path Watcher) | Option B (Markdown Preview Only) | Option C (Polling Fallback) |
| :--- | :--- | :--- | :--- |
| **Complexity** | Medium | Low | Medium |
| **New Dependencies** | None (`notify` / `notify-debouncer-mini` already in use) | None | None |
| **Performance & Latency** | High (event-driven, sub-millisecond dispatch) | High (for preview only) | Poor (repeated git process execution) |
| **Maintenance Burden** | Low (centralized watcher & view lifecycle) | Medium (inconsistent worktree UX) | High (polling timers, race conditions) |
| **Blast Radius** | Localized to `refresh.rs`, `ui/markdown.rs`, `watcher/mod.rs`, `app/worktree.rs` | Localized to `refresh.rs`, `ui/markdown.rs` | Broad (event loop, process spawning) |

## 4. Recommendation
Adopt Option A. Allow `ActiveView::MarkdownPreview` to receive diff updates cleanly, invalidate markdown cache on diff revision bump, and ensure `RepoWatcher` watches worktree gitdirs and updates on `:wt` switch.

## 5. Breaking Changes & Deprecations
- **REQ-PREVIEW-NAV**: Extend navigation/view state to preserve cursor row when diff reload arrives during preview.
- **REQ-WT-ISOLATION**: Enforce watcher re-initialization on target root and worktree gitdir tracking.
- No requirements removed or deprecated.

## Review Gate
- [ ] **Gate 0 Decision**: Human decides Go (name the option) or No-Go before the proposal is drafted.
