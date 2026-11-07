# Research & Technical Tradeoffs

## 1. Intent & Ground Truth
- **Problem**: LGTM currently conflates two systems under the word "comment":
  1. Local agent-steering directives stored in `.lgtm/directives.jsonl`, still referenced internally as `CommentStore`, `CommentAnchor`, and `self.comments`.
  2. Forge PR review discussions (GitHub / AzDO) that pass through a complex, heavy local draft queue (`.lgtm/drafts/pr-<N>.json`), requiring staging, draft locking, rank sorting, and a batch `:thread submit` modal.
- **Verified Codebase Facts**:
  - `crates/lgtm-core/src/forge/mod.rs` (`PrDiscussion`) already implements single-item methods: `post_inline`, `post_general`, `reply`, and `set_resolved`.
  - The local draft queue in `src/app/drafts.rs` (653 lines) and `crates/lgtm-core/src/discussion/` (store.rs, draft models) is an unnecessary staging layer over the existing single-item forge API.
  - The TUI already possesses an async forge channel architecture (`self.refresh_github_async`) to handle non-blocking remote calls off the main loop.
  - Existing spec `openspec/specs/comments/spec.md` specifies persistent drafts (`REQ-Draft Store`, `REQ-Review and Submit Drafts`) that contradict the goal of a clean, online-only forge tool.

## 2. Options Explored

### Option A: Pure Online Comments + Isolated Directives (Recommended)
- **Description**:
  1. Rename local steering types in `lgtm-core` and TUI (`CommentAnchor` → `Directive`, `CommentStore` → `DirectiveStore`, `self.comments` → `self.directives`).
  2. Remove `.lgtm/drafts/` store and draft staging logic completely.
  3. PR comment operations (`post_inline`, `reply`, `set_resolved`) dispatch directly to the forge via background channels.
  4. Failed operations retain typed input in an in-memory retry buffer, presenting an actionable status notice without disk persistence.
- **Pros**:
  - Eliminates ~1,000 lines of disk I/O, draft state machines, and batch submit code.
  - Instant UX: typing a comment or toggling a resolution immediately sends to GitHub / AzDO like web UI.
  - Complete domain clarity: "Directives" are local agent instructions; "Comments" are online forge discussions.
- **Cons**: Requires active PR and network connection to comment; offline comment queueing is dropped.

### Option B: Keep In-Memory Draft Staging, Drop Disk Persistence
- **Description**: Remove `.lgtm/drafts/` files from disk, but retain `self.drafts` in memory and require `:thread submit` to send all staged comments in a single batch review before exit.
- **Pros**: Retains review-batch submission on GitHub.
- **Cons**: Retains most draft queue complexity in memory, risks losing unsubmitted drafts on abrupt TUI exit or crash, and maintains the awkward `:thread submit` workflow.

### Option C: Only Rename Directives, Keep Disk Draft Queue
- **Description**: Disambiguate local directives naming, but leave `.lgtm/drafts/pr-<N>.json` and `:thread submit` untouched.
- **Pros**: Smaller code diff in forge discussion paths.
- **Cons**: Leaves the awkward draft queue and file persistence intact; fails to achieve the "online only tool" objective.

## 3. Tradeoff Matrix

| Criteria | Option A (Pure Online + Directives) | Option B (In-Memory Batch) | Option C (Rename Only) |
| :--- | :--- | :--- | :--- |
| **Complexity** | Lowest (deletes ~1,000 LOC) | Moderate (retains draft state) | High (retains full draft engine) |
| **New Dependencies** | None | None | None |
| **Performance & Latency** | Non-blocking async (<16ms frame) | Non-blocking async | Involves disk I/O per draft |
| **Maintenance Burden** | Minimal (no disk serialization) | Moderate | High (stale draft sync, locks) |
| **Blast Radius** | `src/app/`, `lgtm-core`, specs | `src/app/drafts.rs` | Local directive names only |

## 4. Recommendation
Option A is the cleanest, YAGNI-first architecture: completely decouple local agent directives from online forge comments, delete the complex draft queue, and make PR commenting an instant, direct online tool.

## 5. Breaking Changes & Deprecations
- **`openspec/specs/comments/spec.md`**:
  - `Draft a Thread`, `Draft a Reply`, `Draft Resolve and Unresolve`, `Persistent Draft Store`, `Review and Submit Drafts` are deprecated and replaced by immediate online execution requirements (`REQ-PR-PostInline`, `REQ-PR-Reply`, `REQ-PR-SetResolved`).
  - Local directive requirements are extracted to a dedicated `directives` capability.

## Review Gate
- [ ] **Gate 0 Decision**: Human decides Go (name the option) or No-Go before the proposal is drafted.
