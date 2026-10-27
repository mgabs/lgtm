# Feature Proposal: Consolidated Comments Workflow

## Intent
Unify local review directives and remote PR review threads across the TUI and CLI headless workflows into a cohesive, dependable engine for planning, previewing, posting, replying to, and resolving code review comments on GitHub and Azure DevOps.

## Scope

### In-Scope
- **CLI & Headless Execution**:
  - Fix CLI parsing so `lgtm pr post [--yes]` and `lgtm pr <NUMBER>` work without conflict.
  - Support headless planning preview (`lgtm pr post`) and confirmed posting (`lgtm pr post --yes`) with deterministic exit codes.
  - Add headless PR comment operations: `lgtm pr reply` and `lgtm pr resolve`.
- **Automatic PR Association**:
  - Automatically detect and bind unattached local review directives (`pr_number: None`) to the active target PR when their target files match the PR diff.
  - Persist association to `.lgtm/comments.jsonl` immediately upon binding.
- **Accurate Diff Line & Side Targeting**:
  - Persist diff side (`CommentSide::Left` vs `CommentSide::Right`) in `CommentAnchor`.
  - Accurately tag deleted lines as `LEFT` (`old_line_num`) and added/context lines as `RIGHT` (`new_line_num`).
  - Pass correct diff side to GitHub API (`side: "LEFT" | "RIGHT"`) and Azure DevOps REST (`leftFileStart` vs `rightFileStart`).
  - Hunk boundary validation to guarantee comments fall within valid patch boundaries.
- **Consolidated TUI & CLI Commands**:
  - Unify `:resolve` and `:reply` commands so users don't have to distinguish between local directive vs remote thread.
  - Consolidate local review directives and remote PR threads in the comments modal view with clear status badges.
- **Actionable Error Reporting & Feedback**:
  - Provide granular, actionable diagnostic reasons when comments are planned as general, skipped, or rejected by provider APIs (e.g., HTTP 422 line mismatches, auth errors, ambiguous timeouts).

### Out-of-Scope (YAGNI)
- Full-text markdown comment editor inside the TUI (minimal inline prompt is sufficient).
- PR creation or branching workflows (manage PRs via `gh`, `az`, or web interface).
- Multi-repo or cross-fork comment bridging (single git repository scope).
- Live bidirectional polling websocket connections for comment streaming.

## Problem Statement
Reviewers currently face critical friction points when transitioning between local code inspection and remote PR reviews:
1. Running `lgtm pr post [--yes]` from the CLI crashes with parsing errors because `lgtm pr` strictly expects a numeric PR argument.
2. Directives created during initial local review before opening or loading a PR are assigned `pr_number: None` and silently ignored by `post::plan`, leaving users unaware that their review comments were omitted.
3. Review comments created on deleted lines fail or get rejected by forge APIs (HTTP 422) because line side (`LEFT` vs `RIGHT`) is not tracked, and deleted lines are omitted from diff boundary checks.
4. TUI commands diverge confusingly between `:c` (local directives) and `:threads` (remote PR threads), forcing reviewers to mentally juggle two separate lifecycles for what is fundamentally one conversation.
5. Forge errors provide vague "failed" summaries with no indication of which comment was rejected or how to correct the placement.

## Goals, Non-Goals, and Anti-Goals

### Goals
- **Seamless Transition**: Move frictionlessly from local offline code steering to remote PR publishing without losing a single directive.
- **Zero-Failure Line Targeting**: Guarantee 100% accurate line and side tracking for added, modified, and deleted lines across both GitHub and Azure DevOps.
- **Dual Execution Parity**: Every review comment action (plan, post, reply, resolve) can be run identically from either the interactive TUI or headless CI/CLI scripts.
- **Actionable Visibility**: Never silently drop or fail a comment without reporting the specific file, line, reason, and remediation step.

### Non-Goals (YAGNI)
- Implementing custom markdown rich-text rendering widgets for comments.
- Building an autonomous AI bot that posts PR replies without explicit human confirmation (`--yes`).
- Creating a proprietary comments backend database outside of `.lgtm/comments.jsonl`.

### Anti-Goals
- Never post unconfirmed comments to a remote forge without explicit review (`--yes` flag or interactive confirmation).
- Never corrupt or overwrite remote review thread histories by duplicating already-posted local directives.

## Success Metrics
- **Zero Dropped Comments**: 100% of unattached local directives on modified files are correctly associated and posted when a PR is targeted.
- **Zero 422 Side Errors**: 0% API rejection rate due to deleted lines being posted as `RIGHT` side on GitHub or Azure DevOps.
- **Unified Action Latency**: Command resolution (`:resolve`, `:reply`) completes locally in < 5ms and kicks off async remote synchronization immediately.
