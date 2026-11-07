# Proposal

## Intent
Integrate static guard scanner alerts into `lgtm-mcp` so autonomous AI agents can optionally persist static analysis findings directly into `.lgtm/directives.jsonl` via `lgtm_security_scan(persist=true)`.

## Problem Statement
Autonomous AI coding agents interacting through the Model Context Protocol (MCP) need a unified mechanism to detect, query, and remediate static analysis findings alongside human review directives. Currently, `lgtm_security_scan` returns ephemeral JSON alerts without persisting them into `.lgtm/directives.jsonl`, requiring manual coordination or out-of-band directive creation.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `directives`: Clarify and fulfill `REQ-DIR-4` with MCP `lgtm_security_scan` schema support for `persist: bool`, deduplication against existing directives, and integration with `lgtm_list_directives` and `lgtm_directive_status`.

## Scope

### In-Scope
- **Affected crates**: `crates/lgtm-mcp` (primary), integration tests.
- **Tool Schema Extension**: Add optional `persist: bool` parameter to `lgtm_security_scan` in `crates/lgtm-mcp/src/lib.rs`.
- **Finding Persistence**: When `persist: true`, convert filtered active findings to `Directive` records using `Directive::from_guard_alert`.
- **Deduplication**: Prevent duplicate entries in `.lgtm/directives.jsonl` by skipping alerts matching unresolved directives on identical `(file_path, line, rule/title)`.
- **Cross-Platform Verification**: Verify path separators and file I/O work identically across Ubuntu, macOS, and Windows matrix without breaking existing tool behavior.

### Out-of-Scope (YAGNI)
- Modifying `crates/lgtm-core` conversion primitives or `Directive` schemas (`from_guard_alert` and `add_alert_directive` already suffice).
- Auto-resolving directives upon subsequent clean scans.
- Creating a separate MCP persistence tool.
- Network calls or remote forge sync (strictly local `.lgtm/directives.jsonl`).

## Assumptions
- **ASM-GEN-1**: `lgtm_core::findings::run_in` handles baseline suppression; only findings in `findings.active` are candidates for directive persistence.
- **ASM-GEN-2**: Existing review directives in `.lgtm/directives.jsonl` can be loaded via `DirectiveStore::load_directives` to perform in-memory deduplication before appending new ones.
- **ASM-GEN-3**: `persist` defaults to `false` when omitted, guaranteeing full backward compatibility for existing MCP callers.

## Success Metrics
- Calling `lgtm_security_scan` with `{"persist": true}` creates valid directives in `.lgtm/directives.jsonl`.
- Calling `lgtm_security_scan` repeatedly with `{"persist": true}` does not duplicate open directives.
- Stored directives immediately appear in `lgtm_list_directives` and `lgtm_directive_status`.
- 100% test pass rate across `make test` and `make lint`.

## Review Gate
- [x] **Gate 1 Approval**: Proposal approved by user.
