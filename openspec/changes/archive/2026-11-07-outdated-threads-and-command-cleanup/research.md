# Research & Technical Tradeoffs

## 1. Intent & Ground Truth
- **Problem**:
  1. On PRs with subsequent commits (such as PR 81), comments targeting modified/deleted code lines appear active or disappear from diff gutters without indicating they are obsolete/outdated.
  2. GitHub GraphQL exposes `isOutdated: Boolean!` on `PullRequestReviewThread`, but `THREADS_QUERY` omits this field, and `Thread` struct has no `outdated` indicator.
  3. Azure DevOps threads are iteration-based, but `AzureForge::parse_threads` does not correlate threads with the live diff snapshot.
  4. Commands and shortcuts conflate local agent directives (`:c`, `:directives`) and remote PR threads (`:threads`, `:thread`), with dead draft-queue stubs (`:thread drafts|submit|discard|edit`) and dead TUI shortcuts (`'s'` in ThreadView).
  5. Vestigial draft queue code (`crates/lgtm-core/src/discussion/`, `App.drafts`, `App.submit_confirm`, `App.thread_pending_label`, `ForgeAction`) remains in the codebase despite online-only migration.
- **Verified Codebase Facts**:
  - `crates/lgtm-core/src/github/mod.rs:213`: `THREADS_QUERY` requests `id isResolved path line comments...`, omitting `isOutdated`.
  - `crates/lgtm-core/src/github/mod.rs:47`: `Thread` struct only tracks `id`, `path`, `line`, `resolved`, `comments`.
  - `src/ui/modals/threads.rs:56-140`: Only formats `● Open` and `✓ Resolved`. No outdated badge or filter.
  - `src/app/input.rs:785`: Key `'s'` in `ThreadView` dispatches dead `submit` command.
  - `crates/lgtm-core/src/discussion/mod.rs` (435 LOC): `DraftSet`, `Draft`, `DraftKind`, and `mark_stale` are unused dead code.
  - `src/app/drafts.rs:7`: `#[allow(dead_code)] enum ForgeAction` is unused.

## 2. Options Explored

### Option A: Native `isOutdated` + In-Memory Diff Verification + Dead Code Purge (Recommended)
- **Description**:
  1. Add `isOutdated` to GitHub GraphQL query and `pub outdated: bool` to `lgtm_core::github::Thread`.
  2. For Azure DevOps (and fallback), verify thread `(path, line)` against current in-memory `DiffSnapshot`. If absent from current hunks, mark `outdated = true`.
  3. Update TUI: Gutter shows `~` (yellow) for outdated comments, modal shows `[⚠ Outdated]` badge, and `Tab`/`o` cycles `All` → `Open` → `Outdated`.
  4. Separate command domains: Clean `:thread` / `:threads` for remote forge discussion; `:directives` / `:c` for local agent directives. Remove ambiguous polymorphic fallback in `:reply` and `:resolve`.
  5. Purge dead code: Delete `crates/lgtm-core/src/discussion/`, `App.drafts`, `App.submit_confirm`, `App.thread_pending_label`, `ForgeAction`, and stubbed draft commands.
- **Pros**:
  - Authoritative accuracy on GitHub and reliable fallback on Azure DevOps with zero disk I/O.
  - Eliminates >500 lines of dead code and confusing command fallbacks.
  - Instantly exposes outdated comments to reviewer with dedicated modal filter.
- **Cons**: Minor change to `Thread` struct and command help docs.

### Option B: Server-Only Flag (GitHub Only)
- **Description**: Only add `isOutdated` to GitHub GraphQL query. Azure DevOps threads remain unflagged. Keep dead code stubs for backwards compatibility.
- **Pros**: Minimal touch on Azure forge code.
- **Cons**: Leaves Azure DevOps with broken parity; retains dead draft code and confusing command fallbacks.

### Option C: Pure Client-Side Line Matching (Ignore Forge Metadata)
- **Description**: Compute `outdated` purely by testing if thread line exists in current `DiffSnapshot`, ignoring GitHub's `isOutdated` field.
- **Pros**: Same logic across all providers.
- **Cons**: Less accurate on GitHub (GitHub's diff algorithm accounts for shifted lines across rebased commits that client regex/hunk matching may miss).

## 3. Tradeoff Matrix

| Criteria | Option A (Native + In-Memory Fallback + Purge) | Option B (GitHub Server Only) | Option C (Pure Client Match) |
| :--- | :--- | :--- | :--- |
| **Accuracy** | Highest (authoritative on GitHub + hunk-verified on AzDO) | High for GitHub, Zero for AzDO | Medium (may misclassify shifted lines) |
| **Provider Parity** | Parity maintained across GitHub & AzDO | Broken (GitHub only) | Identical |
| **Code Cleanliness** | Highest (removes 500+ LOC dead draft code) | Poor (keeps dead stubs) | Poor (keeps dead stubs) |
| **Performance & Latency** | Instant (in-memory, <16ms frame) | Instant | Instant |
| **Blast Radius** | Moderate (`lgtm-core`, `src/app/`, `src/ui/`) | Small | Small |

## 4. Recommendation
Option A is the cleanest, YAGNI-first architecture: query GitHub's native `isOutdated`, use in-memory diff hunk verification for Azure DevOps, add TUI indicators (`~` gutter, `Tab`/`o` filter), cleanly separate remote `:thread` from local `:directives`, and delete dead draft queue remnants.

## 5. Breaking Changes & Deprecations
- **Modified Requirements**:
  - `Requirement: Provider Parity` in `openspec/specs/comments/spec.md`: must include normalized `outdated` thread state parity.
  - `Requirement: Thread Command and Key Surface` in `openspec/specs/comments/spec.md`: remove dead `:thread drafts|submit|discard|edit` stubs, add `:threads [open|outdated|all]`, remove `'s'` submit key from `ThreadView`.
- **Deprecated / Removed Code**:
  - `lgtm_core::discussion` module completely removed.

## Review Gate
- [x] **Gate 0 Decision**: Go (Option A) approved by human.
