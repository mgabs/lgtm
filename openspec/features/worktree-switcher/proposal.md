# Feature Proposal: Git Worktree Switcher

## Intent
Enable reviewers to inspect and switch between multiple Git worktrees (`.worktrees/*` and root repo) directly within `lgtm` via command-driven selection (`:wt`, `:worktrees`), maintaining strict single-diff isolation and prompting to save or discard pending review directives before switching.

## Scope

### In-Scope
- **Worktree Discovery**: Parse active worktrees via `git worktree list --porcelain`.
- **Command & Interactive Picker**:
  - `:wt` or `:worktrees` opens an interactive modal list showing worktree paths, checked-out branches, and HEAD commit hashes.
  - `:wt <name|path>` directly jumps to a specific worktree.
- **Single-Diff Invariant**:
  - Strictly switch `GitEngine` repo root, file watcher, and diff snapshots to the target worktree.
  - Exactly one diff is visible at any time; zero cross-worktree diff merging.
- **Dirty State Guard**:
  - If unsaved review directives or uncommitted comments exist in the current worktree, prompt the user with a confirmation modal: `[Save/Export]` vs `[Discard]` before switching.
- **Visual Status Badge**:
  - Display active worktree name in header/status bar (e.g. `[wt: feat-markdown-preview]`).

### Out-of-Scope (YAGNI)
- Creating, removing, or locking git worktrees from within the TUI (manage worktrees via CLI/scripts).
- Concurrent split-pane diffing of two worktrees simultaneously.
- Cross-worktree batch review exporting.

## Problem Statement
When orchestrating multiple concurrent AI coding agents, each agent works in an isolated Git worktree (`.worktrees/feat-*`). Reviewers currently have to shut down `lgtm` and reopen it in a different directory to review another agent's work. This causes friction during human review gates and breaks workflow continuity.

## Goals, Non-Goals, and Anti-Goals
- **Goal**: Seamless, keyboard-driven worktree context switching in under 100ms.
- **Non-Goal**: Re-implementing git worktree administration commands in `lgtm`.
- **Anti-Goal**: Displaying a multiplexed or unified diff containing hunks from multiple worktrees simultaneously, which causes cognitive overload and attribution confusion.

## Success Metrics
- **Context Switch Latency**: Switching between worktrees reloads diffs and file trees in < 100ms for repositories with < 10,000 files.
- **Data Protection**: 100% of unsaved directives prompt for save/discard, preventing accidental directive loss.
- **Clarity**: 100% visual clarity on which worktree is currently active.
