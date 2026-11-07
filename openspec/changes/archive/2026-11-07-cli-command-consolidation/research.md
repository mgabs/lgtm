# Research & Technical Tradeoffs

## 1. Intent & Ground Truth
- **Problem**: `lgtm --help` exposes 17 subcommands with semantic duplication (`serve`/`mcp`, `prs`/`pr`, `diff`/`status`/`review`, `risk`/`check`/`scan`/`audit`/`ci`), deprecated stubs, and redundant top-level flags (`--init`, `--pr`).
- **Verified Facts**:
  - `serve` (`src/cli.rs:226`, `src/main.rs:176`) is a deprecated alias for `lgtm mcp` printing a stderr notice; still referenced in `README.md:50` and `docs/WIKI.md:78`.
  - `status` (`src/cli.rs:215`, `src/main.rs:434`) and `diff` (`src/cli.rs:210`) both launch diff TUI; bare `lgtm` and `lgtm <rev>` already launch diff TUI via external git subcommand parsing (`src/main.rs:189`).
  - `ci` (`src/cli.rs:163`, `src/main.rs:272`) delegates to `run_check` with `no_user_config: true` after printing a deprecation notice; used in `Makefile:53`.
  - `prs` (`src/cli.rs:110`, `src/main.rs:250`) delegates to `list_prs_cli` after printing a deprecation notice; duplicates `pr list`.
  - `risk` (`src/cli.rs:71`, `src/main.rs:127`) runs diff findings and prints terminal color text; overlaps `check` which produces markdown/json/sarif reports.
  - Top-level `--init` (`src/cli.rs:46`, `src/main.rs:102`) and `--pr` (`src/cli.rs:30`, `src/main.rs:442`) duplicate subcommands `init` and `pr`.

## 2. Options Explored

### Option A: Aggressive Consolidation (Strict Canonical CLI)
- Remove all deprecated commands (`serve`, `status`, `ci`, `prs`).
- Remove duplicated top-level flags (`--init`, `--pr`).
- Fold `risk` into `check` (add `--format text` or auto-detect TTY).
- Restrict `review` to export/import only (`review` without subcommand fails or requires action).
- Drop explicit `diff` subcommand in favor of bare `lgtm [REVISION]`.
- **Pros**: Minimal surface area (9-10 subcommands), zero dead code, perfectly clean `--help`.
- **Cons**: Breaking change for external scripts invoking `lgtm ci` or `lgtm serve`.

### Option B: Phased Consolidation (Clean Help, Hard Removal of Stale Aliases, Keep Aliases for Common Shortcuts)
- Remove `serve` (and `--mcp` flag) and `prs` immediately.
- Drop `status` (already deprecated) and update docs/tests.
- Keep `check` as canonical, keep `ci` as a hidden alias or replace `Makefile` with `lgtm check --no-user-config`.
- Keep `risk` as a human-friendly shortcut for working-tree diff audit; keep `diff` as an explicit subcommand alias for bare `lgtm`.
- Remove top-level flags `--init` and `--pr` with migration warnings or drop immediately.
- **Pros**: Balanced ergonomics, preserves high-frequency developer muscle memory (`lgtm risk`, `lgtm diff`).
- **Cons**: Slightly larger subcommand count (11-12 subcommands).

### Option C: Minimal Pruning (Issue #83 Strict Minimum)
- Only remove `serve` (and `--mcp`), update docs and tests.
- Leave `risk`, `check`, `scan`, `audit`, `ci`, `diff`, `status`, `prs`, and top-level flags untouched or merely document them.
- **Pros**: Zero friction with existing workflows.
- **Cons**: Fails to resolve the 17-command sprawl and confusion identified in Issue #83.

## 3. Tradeoff Matrix

| Criteria | Option A (Strict Canonical) | Option B (Balanced Ergonomics) | Option C (Minimal Pruning) |
| :--- | :--- | :--- | :--- |
| **Complexity** | Low (code removed) | Low | Moderate (keeps legacy baggage) |
| **New Dependencies** | None | None | None |
| **Performance & Latency** | Neutral / micro-improvement | Neutral | Neutral |
| **Maintenance Burden** | Lowest | Low | High (maintaining multiple paths) |
| **Blast Radius** | Medium (breaking CLI changes) | Low-Medium | Lowest |

## 4. Recommendation
Adopt Option A (or Option B keeping `risk` as a dedicated convenience alias). Complete removal of dead aliases (`serve`, `prs`, `status`, `ci`) and top-level flags (`--init`, `--pr`) aligns with ponytail YAGNI discipline and eliminates duplicate code paths.

## 5. Breaking Changes & Deprecations
- `clean-commands-shortcuts`:
  - `REQ-CLI-1`: `--init` flag removed in favor of `lgtm init [--repo <path>]`.
  - Deprecated CLI Aliases requirement modified: `serve`, `prs`, `status`, and `ci` completely removed rather than printing deprecation notices.
  - Canonical CLI Commands requirement updated to reflect final consolidated list.

## Review Gate
- [x] **Gate 0 Decision**: Option A
