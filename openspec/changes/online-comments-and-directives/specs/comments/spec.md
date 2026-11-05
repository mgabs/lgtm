# Spec Delta

## REMOVED Requirements

### Requirement: Draft a Thread
**Reason**: Local draft staging is superseded by direct online thread posting against the forge.
**Migration**: Use immediate online thread creation (`REQ-COM-1`).

### Requirement: Draft a Reply
**Reason**: Local draft staging is superseded by direct online thread replies.
**Migration**: Use immediate online thread reply (`REQ-COM-2`).

### Requirement: Draft Resolve and Unresolve
**Reason**: Local draft staging is superseded by direct online thread resolution.
**Migration**: Use immediate online thread resolution (`REQ-COM-3`).

### Requirement: Persistent Draft Store
**Reason**: The `.lgtm/drafts/` disk queue is eliminated to make comments a pure online tool without local file storage.
**Migration**: PR discussions live exclusively on the remote forge.

### Requirement: Review and Submit Drafts
**Reason**: Batch staging and `:thread submit` confirmation are removed in favor of instant execution.
**Migration**: Thread actions are posted directly to the forge API on user confirmation.

### Requirement: Independent Submit Outcomes
**Reason**: The batch submit queue is eliminated.
**Migration**: Actions are evaluated and reported individually upon dispatch.

## ADDED Requirements

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

## Review Gate
- [ ] **Gate 2 Approval**: Specifications reviewed and approved by human before technical design.
