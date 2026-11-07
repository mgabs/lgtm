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
Opening the comments view or running `:refresh` SHALL fetch threads live from the forge off the main loop. Each thread MUST show path, line, side, resolved badge, outdated badge, and all comments (author, body) in order. Diff gutter markers MUST derive from fetched threads, displaying `◆` for open threads, `◇` for resolved threads, and `~` for outdated threads whose target lines changed in the PR diff.

#### Scenario: Open comments view
- **WHEN** a PR is loaded and the comments view opens
- **THEN** live threads are listed with path, line, side, resolved badge, outdated badge, and comments

#### Scenario: Outdated comment marker in diff gutter
- **WHEN** a remote thread targets a line modified or deleted by a newer commit in the PR
- **THEN** the diff gutter renders an outdated marker `~` rather than an active open marker

#### Scenario: No PR target
- **WHEN** no PR or forge target exists
- **THEN** the view shows "Comments require a PR" and offers no create, reply, or resolve actions

### Requirement: Robust Failure Handling
Every failure SHALL report operation, file:line (if any), reason and remediation, covering auth, HTTP 422 line mismatch, rate limit, network offline and thread not found. After a timeout or ambiguous outcome, submit MUST re-fetch threads and match (author, body, path, line, or reply body) before retrying so a retry never duplicates a comment. Draft bodies MUST NOT be lost, and failures MUST NOT be silently swallowed or downgraded to a different operation.

#### Scenario: Timeout does not duplicate
- **WHEN** a submit times out after the server accepted the comment
- **THEN** the retry finds the matching live comment and does not post it again

#### Scenario: Stale draft
- **WHEN** a draft's thread is gone or its line left the diff
- **THEN** the draft is marked `STALE` with the reason and is not sent until edited or discarded

### Requirement: Provider Parity
Live listing, outdated thread detection, reply, and resolve behavior SHALL behave identically across GitHub and Azure DevOps. GitHub threads SHALL source outdated status from `isOutdated`; Azure DevOps threads SHALL reconcile positions against the current diff snapshot. Unsupported capabilities MUST return an explicit error.

#### Scenario: GitHub outdated thread parsing
- **WHEN** a GitHub review thread GraphQL response contains `isOutdated: true`
- **THEN** the thread model sets `outdated = true`

#### Scenario: Azure DevOps outdated thread reconciliation
- **WHEN** an Azure DevOps thread targets a line not present in the current diff snapshot hunks
- **THEN** the thread model sets `outdated = true`

#### Scenario: Azure side mapping
- **WHEN** a `LEFT` thread is submitted to Azure DevOps
- **THEN** it is sent using the left-file position fields

### Requirement: MCP and Watcher Source Live Threads
The `lgtm-mcp` comment tools and the watcher SHALL source PR comments from live forge threads, not from local jsonl files.

#### Scenario: MCP lists live threads
- **WHEN** an MCP client requests PR comments
- **THEN** the response reflects threads fetched from the forge

### Requirement: Thread Command and Key Surface
PR commands SHALL use `:thread` (`comment`, `reply`, `resolve`, `unresolve`, `retry`, `refresh`, `outdated`) and `:threads [open|outdated|all]`. Bare `:reply`, `:resolve`, and `:unresolve` MUST affect only local directives. Draft commands (`drafts`, `submit`, `discard`, `edit`) MUST be unknown. In `:threads`, `Tab` and `o` MUST cycle `All`, `Open`, `Outdated`; the dead `s` submit key MUST be removed.

#### Scenario: Command domains stay separate
- **WHEN** a local directive and remote thread share the cursor line and the user runs bare `:reply`, `:resolve`, or `:unresolve`
- **THEN** only the local directive is changed; remote thread actions require the explicit `:thread` namespace

#### Scenario: Draft via command
- **WHEN** a PR is loaded and the user runs `:thread comment looks wrong`
- **THEN** the comment is posted directly to the remote forge without creating a local disk draft

#### Scenario: Removed command
- **WHEN** the user runs `:post`
- **THEN** it is reported as an unknown command

#### Scenario: Filter threads by outdated status
- **WHEN** the user runs `:threads outdated`
- **THEN** only threads with `outdated == true` are displayed in the list, including resolved outdated threads

#### Scenario: Cycle thread status filters
- **WHEN** the user presses `Tab` or `o` in the thread modal
- **THEN** the filter advances from `All` to `Open` to `Outdated` and back to `All`
- **AND** `Open` includes only unresolved, non-outdated threads, while `Outdated` includes all outdated threads

#### Scenario: Unknown removed command
- **WHEN** the user runs `:thread submit` or `:thread drafts`
- **THEN** the command is rejected as unknown with usage instructions

#### Scenario: Navigation to outdated thread line
- **WHEN** the user presses `Enter` on an outdated thread whose line is absent from the diff
- **THEN** cursor navigates to the file header and status bar displays "Line gone from current diff (outdated)"

### Requirement: Comment Edge Cases and Non-Functional Constraints
A thread deleted remotely MUST report "thread not found" and refresh the list. The head SHA MUST be re-fetched at submit, with a 422 mapped to the failure message. Lineless threads MUST be listed under their path with reply and resolve still working. Over-limit bodies MUST be reported before sending. All forge calls MUST run off the UI thread and core comment logic MUST access the forge only through `PrDiscussion`.

#### Scenario: Thread deleted remotely
- **WHEN** a reply targets a thread deleted on the forge
- **THEN** "thread not found" is reported and the thread list refreshes

#### Scenario: Oversized body
- **WHEN** a draft body exceeds the provider limit
- **THEN** it is reported before anything is sent

### Requirement: REQ-COM-1 Direct Online Thread Creation
When viewing a pull request diff, users SHALL be able to create inline review comments on any diff hunk line. The system MUST dispatch the comment directly to the remote forge API asynchronously without creating local disk drafts.

#### Scenario: Post inline thread online
- **WHEN** the user creates a comment on a diff line in an active PR
- **THEN** the comment is posted directly to GitHub or Azure DevOps
- **AND** the new thread appears in the view upon successful response

### Requirement: REQ-COM-2 Direct Online Thread Reply
Users SHALL be able to reply to focused PR discussion threads. The reply MUST be sent directly to the forge API asynchronously without entering a local draft queue.

#### Scenario: Reply to active thread
- **WHEN** the user submits a reply on an existing thread
- **THEN** the reply is dispatched to the forge and appended to the live thread

### Requirement: REQ-COM-3 Direct Online Thread Resolution
Users SHALL be able to toggle thread resolution (resolve or unresolve) directly against the forge API. The resolved state MUST update immediately in the TUI upon forge confirmation.

#### Scenario: Resolve thread online
- **WHEN** the user resolves a focused thread
- **THEN** the resolution status is updated directly on the forge and reflected in the TUI

### Requirement: REQ-COM-4 Non-Blocking Async Execution
All forge comment operations (posting, replying, resolving, and fetching) MUST execute off the main event loop, maintaining a responsive UI frame budget (< 16ms).

#### Scenario: UI responsiveness during comment post
- **WHEN** a comment post is awaiting a remote forge HTTP response
- **THEN** the TUI remains responsive to scrolling and key navigation with a progress status message

### Requirement: REQ-COM-5 In-Memory Failure Retention
When a forge comment operation fails due to network outage, rate limit, or authentication error, the system MUST retain the typed comment body in an in-memory retry buffer and report an actionable status error. The failure MUST NOT write error drafts to disk.

#### Scenario: Failed post retains input buffer
- **WHEN** a comment post fails due to network disconnection
- **THEN** the error is reported in the status bar
- **AND** reopening the comment prompt pre-fills the unsent text for retry

### Requirement: REQ-COM-6 Outdated Thread Indication and Filtering
The system MUST provide dedicated visual indicators for outdated PR threads across diff gutters and the thread modal, and SHALL allow reviewers to jump directly between outdated threads in the diff view using `]T` and `[T`.

#### Scenario: Jump to next outdated thread
- **WHEN** the user presses `]T` in the diff view
- **THEN** the cursor jumps to the next line containing an unresolved outdated thread marker in the current file

#### Scenario: Jump to previous outdated thread
- **WHEN** the user presses `[T` in the diff view
- **THEN** the cursor jumps to the previous line containing an unresolved outdated thread marker in the current file

#### Scenario: Outdated thread modal badge
- **WHEN** an outdated thread is selected in the thread modal
- **THEN** its status displays `[⚠ Outdated]`

### Requirement: NFR-COM-1 Thread Filter and Gutter Budget
Reconciling thread outdated state against the diff snapshot and rendering gutter markers MUST execute in under 1 millisecond on diff reload to preserve the 16ms frame rendering budget.

#### Scenario: Outdated check maintains frame budget
- **WHEN** a PR diff with 100 discussion threads and 1,000 diff lines is measured in a release build
- **THEN** in-memory reconciliation and gutter marker mapping complete within 1ms
- **AND** the median of seven samples after warm-up is measured by `outdated_thread_performance_budgets`, with setup and network excluded as recorded in `docs/bdd-scenarios.md`
