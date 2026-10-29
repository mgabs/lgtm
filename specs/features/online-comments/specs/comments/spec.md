# Specification: Online Comments

## Capability: `comments`

Builds on the existing `PrDiscussion` trait (`threads`, `post_inline`, `post_general`, `reply`, `set_resolved`) in `forge/mod.rs`.

### 1. Requirements

#### REQ-OC-01 (Removal of local comment store)
- `.lgtm/comments.jsonl` MUST NOT be read or written; `comment` store, `CommentAnchor` persistence, PR association/binding, and `post::plan`/preview pipeline MUST be deleted.
- `:c` and `:threads` MUST collapse into one `:comments` view backed only by forge threads.
- Agent directives (`#ref file:line`, `.lgtm/reviews/`, MCP review inbox) are unaffected.

#### REQ-OC-02 (Live thread listing)
- Opening the comments view or running `:refresh` MUST fetch threads live from the forge (`threads(pr)`), off the main loop.
- Each thread shows path, line, side, resolved badge, and all comments (author, body) in order.
- Gutter markers in the diff derive from the fetched threads only.
- With no PR/forge target, the view MUST show "Comments require a PR" and no create/reply/resolve actions.

#### REQ-OC-03 (Open thread)
- User can open a new inline thread at the cursor line; side MUST be `LEFT` for deletions, `RIGHT` for additions/context.
- Submit calls `post_inline(pr, head_sha, path, line, side, body)` immediately; on success the thread list is refreshed and the new thread focused.
- Empty/whitespace body MUST be rejected locally without a request.
- Lines outside a diff hunk MUST be rejected locally with a message naming file and line.
- A general PR conversation comment is available via `post_general`.

#### REQ-OC-04 (Reply)
- User can reply to a focused thread; calls `reply(pr, thread_id, comment_id, body)` immediately, then refreshes that thread via `thread(pr, id)`.
- CLI: `lgtm pr reply <THREAD_ID> <TEXT> [--pr N]`.

#### REQ-OC-05 (Resolve / unresolve)
- User can resolve and unresolve a focused thread via `set_resolved`; badge updates after the thread refresh.
- Resolving an already-resolved thread (or the reverse) is an idempotent success.
- CLI: `lgtm pr resolve|unresolve <THREAD_ID> [--pr N]`.

#### REQ-OC-06 (CLI)
- `lgtm pr comment <PATH>:<LINE> [--side left|right] <TEXT> [--pr N]` opens a thread.
- Operations execute immediately (no `--yes`, no dry-run); exit `0` on success, `1` on forge failure, `2` on usage/ambiguous PR.
- Omitted `--pr` resolves via the forge's branch association; ambiguity exits `2`.

#### REQ-OC-07 (Robust failures)
- Every failure reports operation, file:line (if any), reason, and remediation, covering: auth, HTTP 422 line mismatch, rate limit, network/offline, thread not found.
- On timeout/ambiguous outcome, the client MUST re-fetch threads and check for the body (same author) before any retry, so a retry never duplicates a comment.
- The user's typed body MUST be preserved in the prompt after a failure.
- Failures are never silently swallowed or downgraded to a different operation (e.g. inline → general).

#### REQ-OC-08 (Provider parity)
- REQ-OC-02..05 hold identically on GitHub and Azure DevOps (`leftFileStart` vs `rightFileStart` on Azure; `side` on GitHub). Unsupported capabilities return an explicit error.

#### REQ-OC-09 (MCP / watcher)
- `lgtm-mcp` comment tools and the watcher MUST source comments from live forge threads, not the jsonl.

### 2. Non-Functional Requirements

- NFR-OC-01: All forge calls run off the UI thread; the event loop never blocks (frame < 16ms, keystroke < 1µs).
- NFR-OC-02: Thread fetch for a 200-thread PR renders within 1s of response; per-thread refresh avoids refetching all threads where the provider supports it.
- NFR-OC-03: No hot-path allocations added to diff rendering; markers use a prebuilt `(path,line)` index.
- NFR-OC-04: `lgtm_core` comment logic stays free of direct I/O; forge access only through `PrDiscussion`.
- NFR-OC-05: Coverage ≥ 95%, 100% mutation kill on new/changed core logic.

### 3. Assumptions
- ASM-OC-01: A PR and authenticated forge CLI/token are present; no offline queue (per ASM-GEN-1).
- ASM-OC-02: Existing local `comments.jsonl` content is discarded; a one-line notice is shown if the file exists.
- ASM-OC-03: Duplicate detection uses (author, body, path, line) from fresh threads.
- ASM-OC-04: Refresh is manual (`:refresh`) plus after every mutation; no background polling.

### 4. Edge Cases & Errors
- Thread deleted remotely between list and reply → "thread not found", list refreshed.
- PR head moved after view → `head_sha` re-fetched at submit; if the line is no longer in the diff, 422 mapped to REQ-OC-07 message.
- Outdated/lineless threads (`line: None`) are listed under their path, reply/resolve still work.
- Multi-line body, unicode, very long body (> provider limit) → validated/reported before sending.
- Forge returns resolved-by-others thread → shown resolved; unresolve allowed.
