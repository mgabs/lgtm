# Spec Delta

## MODIFIED Requirements

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

## Review Gate
- [x] **Gate 2 Approval**: Specifications approved by user.
