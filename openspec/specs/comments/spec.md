# comments Specification

## Purpose

Separate local agent-steering review directives from forge PR discussion, and let reviewers draft, review and submit PR threads, replies and resolutions on GitHub and Azure DevOps with robust failure handling.

## Requirements

### Requirement: Directives Separated From PR Comments
Local review directives (`c`, `:c`, `:directives`, MCP `lgtm_list_directives`, watcher, tmux blast, `#ref`) SHALL be stored in `.lgtm/directives.jsonl`. A legacy `comments.jsonl` MUST be read once on load, migrated to `directives.jsonl`, and removed. Directives MUST NOT carry or sync PR thread state, and PR threads MUST NOT read or write the directive store.

#### Scenario: Legacy store migrated
- **WHEN** `.lgtm/comments.jsonl` exists on load
- **THEN** its content is moved to `.lgtm/directives.jsonl` and the legacy file is removed

#### Scenario: Stores stay isolated
- **WHEN** PR threads are fetched or drafted
- **THEN** the directive store is neither read nor written

### Requirement: Live Thread Listing
Opening the comments view or running `:refresh` SHALL fetch threads live from the forge off the main loop. Each thread MUST show path, line, side, resolved badge and all comments (author, body) in order. Diff gutter markers MUST derive from fetched threads only.

#### Scenario: Open comments view
- **WHEN** a PR is loaded and the comments view opens
- **THEN** live threads are listed with path, line, side, resolved badge and comments

#### Scenario: No PR target
- **WHEN** no PR or forge target exists
- **THEN** the view shows "Comments require a PR" and offers no create, reply or resolve actions

### Requirement: Draft a Thread
Users SHALL be able to draft any number of inline threads at the cursor line without any network call. The side MUST be `LEFT` for deletions and `RIGHT` for additions and context. Empty or whitespace bodies MUST be rejected, and lines outside a diff hunk MUST be rejected with a message naming file and line. A general PR conversation comment MUST also be draftable.

#### Scenario: Draft on a deleted line
- **WHEN** the cursor is on a deleted line and the user drafts a comment
- **THEN** the draft has side `LEFT` and no network call is made

#### Scenario: Rejected drafts
- **WHEN** the body is blank or the line is outside any hunk
- **THEN** the draft is rejected, naming the file and line in the second case

### Requirement: Draft a Reply
Users SHALL be able to draft multiple replies to focused threads, each identified by thread id and target comment id.

#### Scenario: Two replies
- **WHEN** the user drafts replies on two different threads
- **THEN** both drafts are stored with their thread and comment ids

### Requirement: Draft Resolve and Unresolve
Users SHALL be able to draft resolving or unresolving a focused thread; the last draft per thread MUST win. Applying a state the thread already has MUST succeed idempotently.

#### Scenario: Last draft wins
- **WHEN** the user drafts resolve and then unresolve for the same thread
- **THEN** only the unresolve draft remains

### Requirement: Persistent Draft Store
Drafts SHALL persist atomically in `.lgtm/drafts/pr-<N>.json`, survive restarts and be scoped per PR and forge repo. Drafts MUST appear in the comments view with a `DRAFT` badge and be editable or discardable individually (`:thread discard <id>`) or all at once. The store MUST hold drafts only, never fetched remote threads.

#### Scenario: Drafts survive restart
- **WHEN** drafts exist and the TUI restarts on the same PR
- **THEN** the drafts are shown as pending `DRAFT` items

### Requirement: Review and Submit Drafts
`:thread submit` SHALL first show the full batch (count by kind, file:line, body) and send nothing until confirmed. Items MUST be applied in order: new threads, replies, resolves. On GitHub inline threads MUST be sent as one `COMMENT` review; on Azure each item MUST be sent individually.

#### Scenario: Confirmation required
- **WHEN** the user runs `:thread submit`
- **THEN** the batch is displayed and nothing is sent until the user confirms

### Requirement: Independent Submit Outcomes
Each submitted item SHALL succeed or fail independently: a success MUST be removed from the store immediately, a failure MUST stay with its error, and re-running submit MUST send only what remains. Live threads MUST refresh afterwards with a per-item result summary.

#### Scenario: Partial failure
- **WHEN** one of three items fails during submit
- **THEN** two are removed from the store, the failed one remains with its error, and a rerun sends only that one

### Requirement: Robust Failure Handling
Every failure SHALL report operation, file:line (if any), reason and remediation, covering auth, HTTP 422 line mismatch, rate limit, network offline and thread not found. After a timeout or ambiguous outcome, submit MUST re-fetch threads and match (author, body, path, line, or reply body) before retrying so a retry never duplicates a comment. Draft bodies MUST NOT be lost, and failures MUST NOT be silently swallowed or downgraded to a different operation.

#### Scenario: Timeout does not duplicate
- **WHEN** a submit times out after the server accepted the comment
- **THEN** the retry finds the matching live comment and does not post it again

#### Scenario: Stale draft
- **WHEN** a draft's thread is gone or its line left the diff
- **THEN** the draft is marked `STALE` with the reason and is not sent until edited or discarded

### Requirement: Provider Parity
Live listing and the draft thread, reply and resolve behavior SHALL behave identically on GitHub (`side`) and Azure DevOps (`leftFileStart` versus `rightFileStart`). Unsupported capabilities MUST return an explicit error.

#### Scenario: Azure side mapping
- **WHEN** a `LEFT` thread is submitted to Azure DevOps
- **THEN** it is sent using the left-file position fields

### Requirement: MCP and Watcher Source Live Threads
The `lgtm-mcp` comment tools and the watcher SHALL source PR comments from live forge threads, not from local jsonl files.

#### Scenario: MCP lists live threads
- **WHEN** an MCP client requests PR comments
- **THEN** the response reflects threads fetched from the forge

### Requirement: Thread Command and Key Surface
PR comment work SHALL be namespaced under `:thread` (`comment`, `reply`, `resolve`, `unresolve`, `submit`, `discard [id|all]`, `refresh`), and `:threads` MUST open the single PR comments view. Local directive commands MUST be unchanged. In the view, `j/k`, `Enter`, `c`, `r`, `x`, `e`, `d`, `s` and `Esc` MUST be scoped to the view with no global key reassigned. Removed commands MUST NOT be aliased, and without a loaded PR `:thread *` MUST show "Comments require a PR".

#### Scenario: Draft via command
- **WHEN** a PR is loaded and the user runs `:thread comment looks wrong`
- **THEN** a draft thread with that body is created at the cursor line

#### Scenario: Removed command
- **WHEN** the user runs `:post`
- **THEN** it is reported as an unknown command

### Requirement: Comment Edge Cases and Non-Functional Constraints
A thread deleted remotely MUST report "thread not found" and refresh the list. The head SHA MUST be re-fetched at submit, with a 422 mapped to the failure message. Lineless threads MUST be listed under their path with reply and resolve still working. Over-limit bodies MUST be reported before sending. All forge calls MUST run off the UI thread and core comment logic MUST access the forge only through `PrDiscussion`.

#### Scenario: Thread deleted remotely
- **WHEN** a reply targets a thread deleted on the forge
- **THEN** "thread not found" is reported and the thread list refreshes

#### Scenario: Oversized body
- **WHEN** a draft body exceeds the provider limit
- **THEN** it is reported before anything is sent
