# Research & Technical Tradeoffs

## 1. Intent & Ground Truth

The review identified that `lgtm_add_directive` accepts a JSON `line` value as `u64` and converts it to the directive's `u32` line field with an unchecked cast in `crates/lgtm-mcp/src/lib.rs`. Values above `u32::MAX` can therefore wrap to a different line and be persisted. The `comments` capability mentions MCP thread sourcing but does not specify argument validation for adding a local directive; there is no dedicated MCP input-validation spec.

The required behavior is to reject values that cannot be represented before saving a directive, and to preserve the exact line for representable inputs. This research does not expand scope to other input constraints such as whether line zero is valid.

## 2. Options Explored

1. **Checked conversion at the storage boundary** — Convert with a checked integer conversion and return an argument error before persistence if out of range. This is minimal and makes the failure explicit.
2. **Constrain the MCP tool schema to the destination integer width** — Advertise a narrower maximum to clients, while retaining server-side validation for malformed requests. This may help clients but does not replace runtime validation.
3. **Clamp or wrap oversized values** — Continue with a transformed line. This can attach a directive to an unintended location and is not acceptable.

## 3. Tradeoff Matrix

| Criteria | Option 1: Checked conversion | Option 2: Schema and validation | Option 3: Clamp or wrap |
| :--- | :--- | :--- | :--- |
| **Complexity** | Low | Moderate | Low |
| **New Dependencies** | None | None | None |
| **Performance & Latency** | Negligible | Negligible | Negligible |
| **Maintenance Burden** | Low | Schema and handler must stay aligned | Hidden data corruption risk |
| **Blast Radius** | Narrow; invalid inputs fail explicitly | Narrow; clearer client contract | Can silently mis-anchor directives |

## 4. Recommendation

Use checked conversion and return an explicit argument error before any directive is persisted. Consider documenting the range in the MCP tool schema as part of the same contract, but never rely on client-side validation alone.

## 5. Breaking Changes & Deprecations

None. Existing specs do not define the numeric range for this MCP directive argument.

## Review Gate
- [ ] **Gate 0 Decision**: Human decides Go (name the option) or No-Go before the proposal is drafted.
