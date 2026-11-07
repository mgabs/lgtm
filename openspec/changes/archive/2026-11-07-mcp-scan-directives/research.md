# Research & Technical Tradeoffs

## 1. Intent & Ground Truth
- **Problem**: Autonomous AI agents interacting via MCP need a unified path to discover and remediate static analysis findings alongside human review directives using existing `lgtm_list_directives` and `lgtm_directive_status` tools (Issue #79).
- **Verified Codebase Facts**:
  - `Directive::from_guard_alert` and `DirectiveStore::add_alert_directive` are already implemented and tested in `crates/lgtm-core/src/directive/mod.rs`.
  - `self.active_findings()` in `crates/lgtm-mcp/src/lib.rs` executes `lgtm_core::findings::run_in`, which automatically filters baseline-suppressed findings into `findings.existing` / `findings.suppressed`.
  - `lgtm_security_scan` MCP tool currently takes `{ "strict": bool }` and returns JSON formatted alerts without persisting.
  - `openspec/specs/directives/spec.md` already specifies `REQ-DIR-4: Static Scan Findings Stored as Directives` and scenario `Guard scan populates directives`.
- **Verified Constraints**:
  - `lgtm-core` remains free of I/O, network, or external API dependencies.
  - Storage is strictly local in `.lgtm/directives.jsonl`.
  - Directives must retain accurate line and file coordinates for `#ref file:line` anchors.

## 2. Options Explored

### Option A: Parameter on `lgtm_security_scan` (`persist: bool`)
- **Description**: Add an optional `persist: bool` parameter to the existing `lgtm_security_scan` tool schema. When `persist: true`, convert filtered active alerts via `Directive::from_guard_alert` and append non-duplicate alerts to `DirectiveStore`.
- **Pros**: Minimal tool schema sprawl; single tool call for agents to scan and record directives; backward compatible (defaults to false).
- **Cons**: Overloads `lgtm_security_scan` with write side-effects when `persist: true`.

### Option B: Dedicated Persistence Tool (`lgtm_persist_security_scan`)
- **Description**: Introduce a distinct MCP tool dedicated solely to running the scan and saving directives.
- **Pros**: Strict separation of read vs write tool semantics.
- **Cons**: Duplicates scan logic and parameter definitions (`strict`), increases LLM tool selection overhead.

## 3. Tradeoff Matrix

| Criteria | Option A (`persist` flag) | Option B (dedicated tool) |
| :--- | :--- | :--- |
| **Complexity** | Low — ~20 lines in `lgtm-mcp` | Medium — duplicate tool schema and handler |
| **New Dependencies** | None | None |
| **Performance & Latency** | Identical (one AST/diff scan pass) | Identical |
| **Maintenance Burden** | Low — single tool to maintain | Higher — two related tools |
| **Blast Radius** | Localized to `crates/lgtm-mcp/src/lib.rs` | Localized to `crates/lgtm-mcp/src/lib.rs` |

## 4. Recommendation
Implement Option A: Add `persist: bool` to `lgtm_security_scan` with in-memory deduplication against unresolved directives in `.lgtm/directives.jsonl`. It strictly honors YAGNI and reuses existing `lgtm-core` conversion primitives.

## 5. Breaking Changes & Deprecations
None. `persist` defaults to `false`, maintaining full backward compatibility for existing callers of `lgtm_security_scan`. Fulfills existing specification requirement `REQ-DIR-4`.

## Review Gate
- [x] **Gate 0 Decision**: Option A (persist flag) approved by user.
