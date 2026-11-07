# directives Specification

## Purpose
Provide a pure, local-only directive engine for AI agent steering and code reviews, fully isolated from network calls and forge PR discussions.

## Requirements

### Requirement: REQ-DIR-1 Local Directive Storage and Isolation
The system SHALL store local review directives exclusively in `.lgtm/directives.jsonl` using `DirectiveStore` and `Directive` models. Directives MUST NOT interact with or depend on remote git forges or PR review discussions.

#### Scenario: Directives loaded offline
- **WHEN** the user creates or lists review directives without network connectivity
- **THEN** directives are read and written to `.lgtm/directives.jsonl` without errors or network calls

### Requirement: REQ-DIR-2 Directive Creation and Management
Users SHALL be able to attach review directives to diff lines or files using the `c` keybinding, `:c`, or `:directives`. Directives MUST support text notes, anchor coordinates, and resolution states for local review workflows.

#### Scenario: Add directive on cursor line
- **WHEN** the cursor is positioned on a diff line and the user presses `c`
- **THEN** a directive input modal opens and saving writes the directive to `.lgtm/directives.jsonl`
- **AND** a gutter marker is displayed on that line

### Requirement: REQ-DIR-3 Agent and MCP Export
The system SHALL expose all stored directives to AI agents through the native MCP server and CLI review commands, formatted with exact `#ref file:line` anchors.

#### Scenario: MCP client queries directives
- **WHEN** an AI agent queries directives via `lgtm_list_directives`
- **THEN** the stored directives from `.lgtm/directives.jsonl` are returned with file anchors

### Requirement: REQ-DIR-4 Static Scan Findings Stored as Directives
The system SHALL support converting static guard scanner alerts (`GuardAlert`) into review directives stored in `.lgtm/directives.jsonl`. The MCP `lgtm_security_scan` tool SHALL accept an optional `persist` boolean parameter defaulting to false. When true, active findings MUST be converted into directives and appended to `.lgtm/directives.jsonl` without duplicating existing open directives. Baseline-suppressed findings MUST NOT generate directives.

#### Scenario: Guard scan populates directives via MCP
- **WHEN** an MCP agent invokes `lgtm_security_scan` with `persist` set to true
- **THEN** active guard alerts are persisted to `.lgtm/directives.jsonl`
- **AND** the directives appear in `lgtm_list_directives` and `lgtm_directive_status`

#### Scenario: Baseline-suppressed findings do not generate directives
- **WHEN** static scans detect alerts that match repository baseline suppressions
- **THEN** no directives are created for those suppressed alerts

#### Scenario: Idempotent scan persistence
- **WHEN** `lgtm_security_scan` is invoked multiple times with `persist` set to true without code changes
- **THEN** open directives for existing alerts are not duplicated in `.lgtm/directives.jsonl`
