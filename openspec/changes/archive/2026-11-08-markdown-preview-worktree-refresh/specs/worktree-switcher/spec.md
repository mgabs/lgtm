# Spec Delta

## MODIFIED Requirements

### Requirement: Single-Diff Isolation
Switching worktrees SHALL re-initialize the Git engine and active file watcher on the target worktree root and its linked git administrative directory (`worktrees/<name>/`), dropping and reloading diff snapshots, layout caches and file trees. Exactly one worktree's diff MUST be active at a time. In-flight background diff or audit workers MUST be cancelled before the repository path is rebound.

#### Scenario: Switch reloads state and rebinds watcher
- **WHEN** the user switches from the root worktree to `feat-x` via `:wt feat-x`
- **THEN** the diff shows only `feat-x` changes
- **AND** the active file watcher observes the `feat-x` root and its linked git directory

#### Scenario: Worktree git index and HEAD updates trigger diff reload
- **WHEN** files are staged or committed inside a linked git worktree
- **AND** the worktree's administrative index or HEAD changes in `.git/worktrees/<name>/`
- **THEN** the file watcher dispatches an update event
- **AND** the TUI reloads the active diff snapshot and file change list

## Review Gate
- [ ] **Gate 2 Approval**: Specifications reviewed and approved by human before technical design.
