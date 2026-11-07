# Proposal

## Intent
Expose obsolete and outdated PR review threads across GitHub and Azure DevOps using native forge metadata and in-memory diff reconciliation, provide clear TUI indicators and modal filters, simplify the `:thread` command surface, and completely purge vestigial disk-draft queue code.

## Problem Statement
When a PR receives new commits (e.g. PR 81), comments targeting modified or deleted code lines are either presented as active or disappear silently from diff gutters without indicating that their target code changed. Furthermore, the codebase retains over 500 lines of dead draft-queue infrastructure (`DraftSet`, `ForgeAction`, stubbed commands, dead keybindings) from the superseded offline draft architecture.

## Affected Crates & CI Impact
- **Affected Crates**: `lgtm-core` (data models, queries, dead code removal) and `lgtm` (TUI modal, diff gutter, command handlers).
- **CI Impact**: Zero impact on OS matrix; changes are purely in-memory data structures and TUI rendering on Ubuntu, macOS, and Windows.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `comments`: Track `outdated` thread status from GitHub GraphQL and in-memory diff hunk reconciliation, render TUI gutter and modal status badges, add modal filter cycling, simplify `:thread` command namespace, and eliminate dead draft queue models.

## Scope

### In-Scope
- Query `isOutdated` in GitHub GraphQL `THREADS_QUERY` and map into `lgtm_core::github::Thread.outdated`.
- Reconcile Azure DevOps thread positions against the active in-memory `DiffSnapshot` to flag threads on modified/deleted code as `outdated`.
- Display outdated thread markers in diff gutter (`~` / Yellow) and status badges in `:threads` modal (`[⚠ Outdated]`).
- Add modal filter cycling (`Tab` / `o`) between `All`, `Open`, and `Outdated` threads.
- Prevent invalid cursor jumps on outdated threads when the original line has left the diff.
- Clean `:thread` command surface: remove obsolete draft stubs (`:thread drafts|submit|discard|edit`), add `:threads [open|outdated|all]`, and keep bare `:reply`, `:resolve`, and `:unresolve` local to directives; remote thread actions require `:thread`.
- Remove dead keybinding `'s'` (`submit`) from `ThreadView`.
- Purge dead code: delete `crates/lgtm-core/src/discussion/`, `App.drafts`, `App.submit_confirm`, `App.thread_pending_label`, and `ForgeAction`.

### Out-of-Scope (YAGNI)
- No local disk caching, SQLite persistence, or offline draft queueing for PR comments.
- No background daemons, webhooks, or high-frequency polling loops.
- No modifications to local agent directive persistence (`.lgtm/directives.jsonl`).
- No retroactive modification of historical commits on the remote forge.

## Assumptions
- **ASM-GEN-1**: GitHub's GraphQL `PullRequestReviewThread.isOutdated` field is authoritative for GitHub PRs.
- **ASM-GEN-2**: Reconciling `(path, line)` against in-memory `DiffSnapshot` hunks reliably identifies obsolete thread locations on Azure DevOps without multi-iteration REST roundtrips.
- **ASM-GEN-3**: Direct online dispatch with in-memory retry buffer completely supersedes local disk drafts.

## Success Metrics
- PR review threads on modified lines display `[⚠ Outdated]` in `:threads` modal and `~` in diff gutter.
- Reviewers can filter directly to outdated threads using `Tab` / `o` in `:threads` modal.
- >500 lines of dead draft-queue code deleted from `crates/lgtm-core/src/discussion/` and `src/app/`.
- All tests pass with 100% mutation kill rate and maintained hot-path budgets (<1µs keystroke, <16ms frame).

## Review Gate
- [x] **Gate 1 Approval**: Proposal reviewed and approved by human.
- [x] **Gate 2 Approval**: Specifications reviewed and approved by human.
