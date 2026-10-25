# R1 Mechanical Split Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Split `src/app.rs`, `src/ui/mod.rs` and `src/main.rs` into focused submodules with zero behavior change.

**Architecture:** A one-off script moves functions verbatim (visibility widened to `pub(crate)`) into new files, then `cargo fmt` normalizes. No logic edits. The script is NOT committed.

**Tech Stack:** Rust 2021, Python 3 (one-off tool), cargo fmt/clippy/test.

**Spec:** `docs/superpowers/specs/2026-10-12-modular-breakdown-design.md`; roadmap item R1 in `docs/superpowers/roadmap.md`.

## Global Constraints

- Behavior-identical: `cargo test` shows 75 passed (trunk baseline); `check`/`scan` outputs match trunk byte for byte.
- `cargo clippy --all-targets --all-features -- -D warnings` clean; `cargo fmt --all --check` clean.
- No file-size cap (accepted: `app/input.rs` 1288, `app/motion.rs` 968, `ui/diff.rs` 911 lines).
- Splitter lives outside the repo; never commit it.
- Commits carry `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>`.

## File Structure

| New file | Holds |
|---|---|
| `src/app/mod.rs` | `App` struct, `new`, `handle_background_events`, `reload_diff` |
| `src/app/input.rs` | all `handle_*_key` |
| `src/app/command.rs` | autocomplete, `execute_command`, `jump_to_line` |
| `src/app/motion.rs` | cursor, paging, hunk/search/comment navigation |
| `src/app/{comments,staging,audit,pr,commit,external}.rs` | per-feature `impl App` methods |
| `src/app/test_util.rs` | `create_test_app` (`#[cfg(test)]`) |
| `src/ui/{header,diff,status,help,command,audit}.rs`, `src/ui/modals/{commit,pickers,comments}.rs` | `render_*` fns; `ui/mod.rs` keeps `render`, `centered_rect` |
| `src/cli.rs` | `Cli`, `Commands`, `AuditStartMode` (public fields) |
| `src/headless.rs` | `run_ci_audit`, report generators, `run_risk_audit`, `run_coverage_audit`, `run_init_config`, `run_git_passthrough` |
| `src/main.rs` | `main()`, `run_tui` |

Splitter: `/Users/mohammed.metawea/.copilot/session-state/bda74714-0eb4-4a87-8caf-6f5d455d04d6/files/split_r1.py` (usage: `python3 split_r1.py <repo_root>`). It also rewrites `src/mutate/mod.rs` labels (`module: "src/app.rs"` → `"src/app"`, `killing_test` → `app::<file>::tests::<name>`).

---

### Task 1: Baseline capture

**Files:** none modified; writes `/tmp/lgtm-baseline/*`.

- [ ] **Step 1: Clean branch**

```bash
git switch main && git switch -c refactor/r1-mechanical-split
git status --short   # only " D prd" may appear; leave it unstaged
```

- [ ] **Step 2: Record golden outputs from trunk**

```bash
cargo build --release
mkdir -p /tmp/lgtm-baseline
./target/release/lgtm --help > /tmp/lgtm-baseline/help.txt
./target/release/lgtm check > /tmp/lgtm-baseline/check.txt 2>&1; echo $? >> /tmp/lgtm-baseline/check.txt
./target/release/lgtm scan > /tmp/lgtm-baseline/scan.txt 2>&1; echo $? >> /tmp/lgtm-baseline/scan.txt
cargo test 2>&1 | grep -E "^test .* ok$" | sort > /tmp/lgtm-baseline/tests.txt
wc -l /tmp/lgtm-baseline/tests.txt   # expect 75
```

### Task 2: Run the splitter

**Files:** modifies/creates everything in File Structure; modifies `src/mutate/mod.rs`.

**Interfaces:** Produces `pub(crate)` methods on `App` and `pub(crate)` render fns; later R2 tasks rely on these paths.

- [ ] **Step 1: Run**

```bash
python3 /Users/mohammed.metawea/.copilot/session-state/bda74714-0eb4-4a87-8caf-6f5d455d04d6/files/split_r1.py "$PWD"
```
Expected: prints `split complete`, no stray-line warnings. An unmapped-function assertion means trunk changed since validation; add the function to the mapping in the script and rerun on a clean tree (`git checkout -- src`).

- [ ] **Step 2: Format, lint**

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```
Expected: no warnings (validated on a scratch copy at HEAD `9a4a7e8`).

- [ ] **Step 3: Test parity**

```bash
cargo test 2>&1 | grep -E "test result"      # 75 passed
cargo test 2>&1 | grep -E "^test .* ok$" | sed 's/app::[a-z_]*::tests/app::tests/' | sort | diff - <(sed 's/app::[a-z_]*::tests/app::tests/' /tmp/lgtm-baseline/tests.txt) && echo same
```
Expected: 75 passed; test names differ only by module path.

- [ ] **Step 4: Output parity**

```bash
cargo build --release
diff <(./target/release/lgtm --help) /tmp/lgtm-baseline/help.txt
diff <(./target/release/lgtm check 2>&1; echo $?) /tmp/lgtm-baseline/check.txt
diff <(./target/release/lgtm scan 2>&1; echo $?) /tmp/lgtm-baseline/scan.txt
```
Expected: no diff.

- [ ] **Step 5: Mutation and coverage gates**

```bash
scripts/coverage.sh && scripts/mutate.sh
```
Expected: both pass at ≥ 90%.

### Task 3: Docs and commit

**Files:** Modify `docs/WIKI.md:521-523`, `docs/superpowers/specs/2026-10-12-modular-breakdown-design.md`.

- [ ] **Step 1: Update module map**

Replace the `src/app.rs` row in `docs/WIKI.md` with rows for `src/app/` (TUI state machine split by concern), `src/ui/` (render fns), `src/cli.rs`, `src/headless.rs`. Then:

```bash
grep -rn "src/app.rs\|src/ui/mod.rs\|src/main.rs" README.md CONTRIBUTING.md docs/WIKI.md
```
Fix any remaining stale references.

- [ ] **Step 2: Correct the spec**

In the modular-breakdown spec: the mutate-path risk is cosmetic label drift (labels are strings, not file reads), and `clipboard_ipc.rs` is `external.rs`.

- [ ] **Step 3: Commit**

```bash
git add src docs
git commit -m "refactor: split app, ui and main into submodules (R1)

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```
Do not stage `prd`. Open the PR and wait for CI (fmt, clippy, tests, coverage, mutation, SARIF self-scan) before merging.

### Task 4: Cleanup

- [ ] `rm -rf /tmp/lgtm-r1 /tmp/lgtm-baseline`
