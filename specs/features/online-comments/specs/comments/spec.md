# Specification: Online Comments

## Capability: `comments`

Builds on the existing `PrDiscussion` trait (`threads`, `post_inline`, `post_general`, `reply`, `set_resolved`) in `forge/mod.rs`.

### 1. Requirements

#### REQ-OC-01 (Removal of local comment store)
- `.lgtm/comments.jsonl` MUST NOT be read or written; `comment` store, `CommentAnchor` persistence, PR association/binding, and `post::plan`/preview pipeline MUST be deleted.
- `:c` and `:threads` MUST collapse into one `:comments` view backed only by forge threads.
- All `comments.jsonl` dead code MUST be deleted, not left disabled: the `comment` module and its types, jsonl (de)serialization, watcher hooks, MCP/review/`github::post` references, related tests and BDD steps, CLI/TUI commands, docs, `.gitignore` entries, and the tracked `.lgtm/comments.jsonl`. Verified by `grep -rn "comments.jsonl"` returning nothing outside `CHANGELOG.md`/`specs/archive/`, and a clean `cargo clippy` with no `dead_code` allowances added.
- Agent directives (`#ref file:line`, `.lgtm/reviews/`, MCP review inbox) are unaffected.

#### REQ-OC-02 (Live thread listing)
- Opening the comments view or running `:refresh` MUST fetch threads live from the forge (`threads(pr)`), off the main loop.
- Each thread shows path, line, side, resolved badge, and all comments (author, body) in order.
- Gutter markers in the diff derive from the fetched threads only.
- With no PR/forge target, the view MUST show "Comments require a PR" and no create/reply/resolve actions.

#### REQ-OC-03 (Draft a thread)
- User can draft any number of new inline threads at the cursor line; side MUST be `LEFT` for deletions, `RIGHT` for additions/context. Drafting makes no network call.
- Empty/whitespace body MUST be rejected; lines outside a diff hunk MUST be rejected with a message naming file and line.
- A draft general PR conversation comment is also supported.
- CLI: `lgtm pr comment <PATH>:<LINE> [--side left|right] <TEXT> [--pr N]` adds a draft.

#### REQ-OC-04 (Draft a reply)
- User can draft multiple replies to focused threads (`thread_id` + target `comment_id`). CLI: `lgtm pr reply <THREAD_ID> <TEXT> [--pr N]`.

#### REQ-OC-05 (Draft resolve / unresolve)
- User can draft resolve/unresolve of a focused thread; the last draft per thread wins. CLI: `lgtm pr resolve|unresolve <THREAD_ID> [--pr N]`.
- Applying to an already-resolved (or unresolved) thread is an idempotent success.

#### REQ-OC-06 (Draft store)
- Drafts persist in `.lgtm/drafts/pr-<N>.json` (distinct from the deleted `comments.jsonl`), written atomically, survive restart, and are scoped per PR and forge repo.
- Drafts are visible in the comments view as pending items (badge `DRAFT`) alongside live threads; each can be edited or discarded individually (`:discard <id>`) or all at once.
- The store holds drafts only: never fetched remote threads.

#### REQ-OC-06b (Review and submit)
- `:submit` / `lgtm pr submit [--pr N]` first shows the full batch (count by kind, file:line, body) for confirmation in the TUI; CLI requires `--yes`, otherwise prints the batch and exits `0` without sending.
- Submit applies items in order: new threads, replies, then resolves. On GitHub, inline threads are sent as a single review (`COMMENT` event); on Azure each item is sent individually.
- Each item succeeds or fails independently: a succeeded item is removed from the store immediately; a failed item stays with its error. Re-running submit sends only what remains.
- After submit, live threads are refreshed. Exit `0` all applied, `1` any failed (partial listed), `2` usage/ambiguous PR.
- Omitted `--pr` resolves via the forge's branch association; ambiguity exits `2`.

#### REQ-OC-07 (Robust failures)
- Every failure reports operation, file:line (if any), reason, and remediation, covering: auth, HTTP 422 line mismatch, rate limit, network/offline, thread not found.
- On timeout/ambiguous outcome, submit MUST re-fetch threads and match (same author, body, path, line, or reply body) before retrying that item, so a retry never duplicates a comment.
- Draft bodies are never lost on failure; failures are never silently swallowed or downgraded to a different operation (e.g. inline → general).
- A draft whose thread is gone or whose line left the diff is marked `STALE` with the reason and is not sent until edited or discarded.

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
- ASM-OC-01: A PR and authenticated forge CLI/token are present to list and submit; drafting itself works offline.
- ASM-OC-02: Existing local `comments.jsonl` content is discarded (not migrated); a one-line notice is shown if the file exists.
- ASM-OC-03: Duplicate detection uses (author, body, path, line) from fresh threads.
- ASM-OC-04: Refresh is manual (`:refresh`) plus after submit; no background polling.
- ASM-OC-05: `.lgtm/drafts/` is gitignored; GitHub inline drafts are batched in one review, but are NOT stored as a server-side pending review (identical behaviour on both forges).

### 4. Edge Cases & Errors
- Thread deleted remotely between list and reply → "thread not found", list refreshed.
- PR head moved after view → `head_sha` re-fetched at submit; if the line is no longer in the diff, 422 mapped to REQ-OC-07 message.
- Outdated/lineless threads (`line: None`) are listed under their path, reply/resolve still work.
- Multi-line body, unicode, very long body (> provider limit) → validated/reported before sending.
- Forge returns resolved-by-others thread → shown resolved; unresolve allowed.
