# Specification: Online Comments

## Capability: `comments`

Builds on the existing `PrDiscussion` trait (`threads`, `post_inline`, `post_general`, `reply`, `set_resolved`) in `forge/mod.rs`.

### 1. Requirements

#### REQ-OC-01 (Split PR comments from local directives)
- Local review directives (agent-steering channel: `:c`/`:directives` and `c` key, MCP `lgtm_list_directives`, watcher, tmux blast, `#ref`) are KEPT but move from `.lgtm/comments.jsonl` to `.lgtm/directives.jsonl`. A legacy `comments.jsonl`, if present, is read once on load, migrated to `directives.jsonl`, and removed.
- All PR-sync coupling is DELETED, not disabled: `pr_number`, `remote_id`, `orphaned`-vs-PR sync, `associate_directives`, `post::plan/post/reconcile/clear_uncertain`, the `lgtm pr post` CLI, `:post`, `:threads`-as-separate-from-drafts logic, directive-scoped remote `:reply`/`:resolve` paths, and their tests, BDD steps and docs.
- Verified by: `grep -rn "comments.jsonl"` returning only the one-time migration code/tests, `CHANGELOG.md` and `specs/archive/`; `grep -rn "remote_id\|associate_directives\|post::plan"` empty; clippy clean with no `dead_code` allowances added.
- Directives never carry or sync to PR state; PR threads never read or write the directive store.

#### REQ-OC-02 (Live thread listing)
- Opening the comments view or running `:refresh` MUST fetch threads live from the forge (`threads(pr)`), off the main loop.
- Each thread shows path, line, side, resolved badge, and all comments (author, body) in order.
- Gutter markers in the diff derive from the fetched threads only.
- With no PR/forge target, the view MUST show "Comments require a PR" and no create/reply/resolve actions.

#### REQ-OC-03 (Draft a thread)
- User can draft any number of new inline threads at the cursor line; side MUST be `LEFT` for deletions, `RIGHT` for additions/context. Drafting makes no network call.
- Empty/whitespace body MUST be rejected; lines outside a diff hunk MUST be rejected with a message naming file and line.
- A draft general PR conversation comment is also supported.

#### REQ-OC-04 (Draft a reply)
- User can draft multiple replies to focused threads (`thread_id` + target `comment_id`).

#### REQ-OC-05 (Draft resolve / unresolve)
- User can draft resolve/unresolve of a focused thread; the last draft per thread wins.
- Applying to an already-resolved (or unresolved) thread is an idempotent success.

#### REQ-OC-06 (Draft store)
- Drafts persist in `.lgtm/drafts/pr-<N>.json` (distinct from the deleted `comments.jsonl`), written atomically, survive restart, and are scoped per PR and forge repo.
- Drafts are visible in the comments view as pending items (badge `DRAFT`) alongside live threads; each can be edited or discarded individually (`:discard <id>`) or all at once.
- The store holds drafts only: never fetched remote threads.

#### REQ-OC-06b (Review and submit)
- `:submit` first shows the full batch (count by kind, file:line, body) for confirmation in the TUI; nothing is sent until confirmed.
- Submit applies items in order: new threads, replies, then resolves. On GitHub, inline threads are sent as a single review (`COMMENT` event); on Azure each item is sent individually.
- Each item succeeds or fails independently: a succeeded item is removed from the store immediately; a failed item stays with its error. Re-running submit sends only what remains.
- After submit, live threads are refreshed and a per-item result summary is shown.
- The target PR is the one loaded in the TUI.

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

### 5. TUI Command & Key Surface

#### REQ-OC-10
Local-directive commands/keys (`c`, `:c`, `:directives`, directive-scoped `:resolve/:unresolve/:reply` as today) are unchanged. PR comment work is namespaced under `:thread` so the two never collide; the old `:threads`, `:post`, and PR-aware paths of `:reply/:resolve/:unresolve` are removed.

| Command | Action |
|---|---|
| `:threads` | Open the single PR comments view (live threads + drafts) |
| `:thread comment [text]` | Draft a new thread at the cursor line (inline prompt if no text) |
| `:thread reply [text]` | Draft a reply to the thread at cursor / focused in the view |
| `:thread resolve` / `:thread unresolve` | Draft resolve / unresolve of the thread at cursor |
| `:thread submit` | Review and send the draft batch (REQ-OC-06b) |
| `:thread discard [id\|all]` | Discard one draft or all drafts for this PR |
| `:thread refresh` | Re-fetch live threads |

- Inside the view: `j/k` move, `Enter` expand, `c` comment, `r` reply, `x` resolve/unresolve, `e` edit draft, `d` discard draft, `s` submit, `Esc` close. Keys are scoped to the view; no global key is added or reassigned. Existing `]t/[t`-style navigation is kept.
- Without a loaded PR, `:thread *` shows "Comments require a PR".
- Help and palette list only these commands; removed commands are deleted, not aliased.
