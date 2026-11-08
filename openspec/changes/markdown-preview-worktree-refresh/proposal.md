# Proposal

## Intent
Enable seamless real-time file updates in the TUI when viewing Markdown Preview and when operating within Git worktrees, ensuring that file additions, edits, deletions, and staged git changes automatically refresh the file change drawer and markdown preview without requiring manual `:reload` or view switches.

## Problem Statement
Reviewers reading markdown documents in Markdown Preview (`ActiveView::MarkdownPreview`) cannot see background file updates in the file drawer or rendered markdown because `apply_ready_local` restricts local refresh applications strictly to `ActiveView::Diff`. Additionally, when running inside a linked Git worktree or switching via `:wt`, file watcher events fail to observe target worktree modifications and ignore git index/HEAD updates located in the common git repository's `worktrees/<name>/` directory.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `markdown-preview`: Allow local diff snapshots and file drawer updates to apply while preview mode is active, invalidating cached rendered rows on document changes while preserving scroll and cursor state.
- `worktree-switcher`: Re-initialize the file watcher upon switching worktrees and watch both the working tree root and the worktree's git administrative directory (`worktrees/<name>/`).

## Scope

### In-Scope
- Permitting `apply_ready_local` to apply in `ActiveView::MarkdownPreview` when no visual selection modal is active.
- Invalidation of `MarkdownPreviewState.rendered_rows` when the active file's diff or content changes (`diff_revision`).
- Preserving or gracefully clamping preview cursor row and scroll offset when new diff snapshots arrive.
- Updating `apply_worktree_switch` to re-bind the active file watcher to the newly selected worktree directory.
- Ensuring `RepoWatcher` and `affects_diff` recognize changes to `.git/worktrees/<name>/index` and `HEAD`.
- Affected crates: `lgtm` (TUI app, refresh, input, ui), `lgtm-core` (watcher).

### Out-of-Scope (YAGNI)
- Non-goal: Live in-place editing of markdown inside the TUI preview (preview remains read-only; editing delegates to external `$EDITOR` via `e`/`o`).
- Non-goal: Polling-based watcher fallback for worktrees (must remain event-driven via `notify`).
- Non-goal: Multi-worktree simultaneous split views (exactly one worktree active at a time).

## Assumptions
- **ASM-GEN-1**: `git worktree` linked repositories point to a valid common git directory discoverable via `Repository::open` or gitdir file.
- **ASM-GEN-2**: Rebinding the `RepoWatcher` on worktree switch operates asynchronously without dropping or blocking UI event loop ticks.

## Success Metrics
- Modifying a markdown file while in Markdown Preview immediately updates both the file change list drawer and rendered preview contents within 50 ms.
- Staging or editing files in a linked Git worktree immediately triggers a diff snapshot refresh without requiring manual `:reload`.
- 100% mutation kill rate on core watcher logic and >= 95% line and branch test coverage.
- CI passes across Ubuntu, macOS, and Windows matrices.

## Review Gate
- [ ] **Gate 1 Approval**: Proposal reviewed and approved by human before drafting specs.
