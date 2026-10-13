# Agent Feedback Loop, Richer MCP and Agent Sinks — Design (Project A)

Date: 2026-10-12 · Status: draft for review · Baseline: `main` @ a90d5b0 (v0.7.0)
Depends on: Project C stage 1 (review state + re-anchoring); refactor stages 3 (`AgentSink` trait) and 4 (`lgtm-mcp` crate); Project B fingerprints for `lgtm_scan_changes` (fallback described below).

## Goal

Close the loop between a human reviewer and an AI agent: after a directive is sent, show whether the code responded; give agents richer MCP tools and live resources; support more terminal multiplexers.

## Decisions (from brainstorming)

1. Outcome detection is diff-based and automatic, never auto-resolving; a human confirms resolution with one key.
2. Agent-reported claims (via MCP) are recorded and cross-checked against the diff; a claim never resolves a directive.
3. Scope: richer MCP tools, MCP resources + subscriptions, and more agent sinks (Zellij, WezTerm, kitty).
4. Notifications use a watcher thread and a locked stdout writer; no async runtime required. If refactor stage 6 (async) goes ahead, the loop can move to tokio without changing contracts.

## Current state (verified)

- `src/mcp/mod.rs`: stdio JSON-RPC, `tools` capability only, 4 tools (`lgtm_diff`, `lgtm_security_scan`, `lgtm_list_directives`, `lgtm_add_directive`), synchronous request handling, no resources, prompts or notifications.
- `src/ipc/mod.rs`: tmux pane listing/detection, `send_to_tmux`, OSC52 clipboard. One-way.
- `src/turn/mod.rs`: `TurnState`, `advance_turn(tree_oid)`.
- `CommentAnchor.is_resolved` is set manually.

## Design

### 1. Directive lifecycle and outcomes
- On each turn advance, re-anchoring (Project C) sets an observed outcome per open directive: `untouched` (hunk byte-identical), `changed` (hunk content changed/moved with edits), `gone` (code or file removed), `orphaned` (no match).
- `resolved` is set only by the human (one key) or by confirming an agent claim.
- Claims: `lgtm_resolve_directive(id, note)` records `claimed { note, turn }`. Display: claimed + changed/gone = "claimed, code changed"; claimed + untouched = "claimed but not changed" (flagged). A claim never sets `resolved`.
- Per-turn summary (counts by outcome) in the status bar and directive list, with filters. `:directives recheck` recomputes on demand.
- Stored in `.lgtm/reviews/<target>.json` (Project C model).

### 2. MCP surface
- New tools: `lgtm_directive_status`, `lgtm_review_state`, `lgtm_scan_changes` (findings new since last turn; uses B's fingerprints, else turn-over-turn diff of finding text+location), `lgtm_resolve_directive` (write: claim only). Existing four tools unchanged; outputs gain `rule_id` and directive id.
- Resources: `lgtm://diff`, `lgtm://directives`, `lgtm://findings`, `lgtm://review-state` with `resources/list|read|subscribe`; capability `resources: { subscribe: true, listChanged: true }`; `notifications/resources/updated` on change, debounced and coalesced per URI. Sources: filesystem watcher and `.lgtm/` state writes.
- Server shape: request loop on main thread, watcher thread feeding a channel, notifier writing under a single stdout lock so messages never interleave.
- Safety: MCP writes limited to directives and claims. Never stage, commit, post to GitHub, edit code, or resolve.
- Additive only: clients using the existing tools keep working.

### 3. Agent sinks (`AgentSink` trait)
- Methods: `list_targets()`, `detect()`, `send(target, text)`.
- Implementations: tmux (existing), Zellij, WezTerm, kitty; OSC52 clipboard remains the fallback.
- Detection order: tmux, Zellij (`ZELLIJ`), WezTerm (`WEZTERM_PANE`), kitty (`KITTY_WINDOW_ID`), clipboard. `--agent-sink <name>` overrides.
- Each sink uses a small command runner; tests use a fake runner and assert exact commands. Real-terminal behavior gets a manual smoke checklist (not verifiable in CI).
- The exact CLI syntax for Zellij, WezTerm and kitty must be checked against current documentation during implementation.

## Testing
- Unit: outcome classification, claim/outcome matrix (including claimed-but-unchanged), per-turn summary.
- MCP: request/response tests per tool, subscription flow, debounce/coalescing, stdout non-interleaving concurrency test; existing MCP tests untouched and passing.
- Sinks: fake-runner command assertions; manual smoke checklist.
- Coverage >= 90%, mutation >= 90%.

## Delivery (one PR each, from trunk, merged before the next)
1. Outcome tracking + per-turn summary (after Project C stage 1).
2. TUI: outcome display, filters, `:directives recheck`, confirm-resolve key.
3. New MCP tools incl. claims (after refactor stage 4, in `lgtm-mcp`).
4. Resources, subscriptions, notifications.
5. Agent sinks: Zellij, WezTerm, kitty, `--agent-sink`, detection.
6. Docs incl. MCP client setup snippets.

## Out of scope
MCP writes beyond directives/claims, auto-resolving, agent-to-agent coordination, the async migration itself.

## Risks
| Risk | Mitigation |
|---|---|
| "Changed" mistaken for "fixed" | Never auto-resolve; claim-vs-diff cross-check; human confirms |
| Notification/response interleaving on stdout | Single locked writer; concurrency test |
| Notification storms | Debounce and per-URI coalescing |
| Multiplexer CLI syntax drift | Verify against current docs; fake-runner tests; manual smoke checklist |
| Dependency on B/C/refactor ordering | Explicit stage dependencies; fingerprint fallback for `lgtm_scan_changes` |
| Mutation gate path breakage when files move | Follow the refactor spec's path-update rule |
