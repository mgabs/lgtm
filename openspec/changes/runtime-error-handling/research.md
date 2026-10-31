# Research & Technical Tradeoffs

## 1. Intent & Ground Truth

The review identified two runtime error paths with misleading or incomplete recovery. In `src/main.rs`, an error returned by `event_loop.step(...)?` exits the UI loop before the terminal restoration calls run. In `src/app/external.rs`, the batch agent-delivery path ignores the sink's send result, then reports delivery and advances the turn. No existing capability spec covers terminal lifecycle cleanup or agent-sink delivery outcomes.

The change should preserve the original operation error, restore terminal state after UI setup on every return path, and report agent delivery as successful only when the sink accepts it. A failed delivery must not be silently treated as success.

## 2. Options Explored

1. **Explicit cleanup at each error site** — Add cleanup to the event-loop failure path and check the sink result. This is localized, but additional fallible exit paths can again bypass cleanup.
2. **Terminal restoration guard plus explicit delivery result handling** — Use a scoped guard for terminal restoration and handle sink failure through the existing user-facing error path. This centralizes cleanup while keeping delivery semantics explicit.
3. **Catch and suppress runtime errors** — Continue or report success after an error to avoid exiting the UI. This risks masking failures and leaving application state inconsistent.

## 3. Tradeoff Matrix

| Criteria | Option 1: Explicit cleanup | Option 2: Guard and checked result | Option 3: Suppress errors |
| :--- | :--- | :--- | :--- |
| **Complexity** | Low initially; grows with error paths | Low to moderate | Low |
| **New Dependencies** | None | None | None |
| **Performance & Latency** | Negligible | Negligible | Negligible |
| **Maintenance Burden** | Repeated cleanup logic | Centralized cleanup | Hidden failure recovery |
| **Blast Radius** | Narrow | Narrow | Broad and unpredictable |

## 4. Recommendation

Prefer a scoped terminal-restoration guard and explicit handling of the agent sink's send result. Preserve the existing failure rather than retrying or claiming delivery when acceptance is unknown.

## 5. Breaking Changes & Deprecations

None. No existing requirement in `openspec/specs/` specifies these terminal-cleanup or sink-delivery behaviors.

## Review Gate
- [ ] **Gate 0 Decision**: Human decides Go (name the option) or No-Go before the proposal is drafted.
