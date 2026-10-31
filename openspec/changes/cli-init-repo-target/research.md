# Research & Technical Tradeoffs

## 1. Intent & Ground Truth

The review identified that the early `--init` handling in `src/main.rs` runs before normal CLI parsing and initializes `"."`, so it ignores a supplied `--repo <path>`. The parsed initialization path already has access to the selected repository. The existing `clean-commands-shortcuts` capability lists `init` as a command but does not define the repository-target behavior of `--init`.

The intended behavior is to honor an explicitly selected repository and retain the current-directory default when no repository is selected. This extends an existing CLI capability without changing unrelated command behavior.

## 2. Options Explored

1. **Handle `--init` after normal CLI parsing** — Use the parsed repository value in the existing initialization path. This avoids separate parsing logic.
2. **Manually parse `--repo` in the early `--init` path** — Keeps the early shortcut but duplicates CLI parsing and risks inconsistent option handling.
3. **Reject `--repo` with `--init`** — Avoids the mismatch but discards a repository selection users can reasonably expect to work.

## 3. Tradeoff Matrix

| Criteria | Option 1: Use parsed arguments | Option 2: Parse shortcut manually | Option 3: Reject combination |
| :--- | :--- | :--- | :--- |
| **Complexity** | Low | Moderate | Low |
| **New Dependencies** | None | None | None |
| **Performance & Latency** | Negligible | Negligible | Negligible |
| **Maintenance Burden** | One source of truth | Duplicate option semantics | Extra restriction to maintain |
| **Blast Radius** | Narrow | Narrow to moderate | User-visible behavior change |

## 4. Recommendation

Use the parsed CLI options for both initialization forms, preserving the existing default when `--repo` is absent. This is the smallest change and keeps repository selection consistent.

## 5. Breaking Changes & Deprecations

None. The change clarifies the existing `init` capability in `openspec/specs/clean-commands-shortcuts/spec.md`; it does not deprecate a command or option.

## Review Gate
- [ ] **Gate 0 Decision**: Human decides Go (name the option) or No-Go before the proposal is drafted.
