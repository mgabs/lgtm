# R4 Cargo Workspace Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Turn the single crate into a workspace: `lgtm-core` (TUI-free domain), `lgtm-mcp` (MCP server), `lgtm` (TUI + CLI, binary name unchanged).

**Architecture:** The root package `lgtm` stays at the repo root (keeps `cargo install --git`, the tap flow and CI paths working). Domain modules move with `git mv` to `crates/lgtm-core/src/`, `mcp` to `crates/lgtm-mcp/src/lib.rs`. The binary re-imports them at its crate root (`use lgtm_core::{git, guard, ...};`) so `crate::guard::X` paths in `app`/`ui`/`headless` keep compiling untouched.

**Tech Stack:** Cargo workspaces, `[workspace.package]` and `[workspace.dependencies]`.

**Spec:** `specs/superpowers/specs/2026-10-12-modular-breakdown-design.md` stage 4; roadmap item R4.

## Global Constraints

- Behavior-identical: all tests pass (`cargo test --workspace`, 82 total across crates); `check`, `check --format sarif`, `check --format json`, `scan .` match an R3 build (output and exit codes).
- Binary name `lgtm`, version `0.7.0` unchanged; `grep '^version =' Cargo.toml | head -1` still yields the version (CI and `scripts/update-tap.sh` depend on it).
- `lgtm-core` and `lgtm-mcp` must not depend on `ratatui` or `crossterm`.
- No new third-party dependencies; versions are shared through `[workspace.dependencies]`.
- fmt, clippy `-D warnings` (with `--workspace`), `scripts/coverage.sh`, `scripts/mutate.sh` (>= 90%) green.
- Commits carry `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>`.

## File Structure

- `Cargo.toml` (root): `[workspace]`, `[workspace.package]`, `[workspace.dependencies]`, package `lgtm`.
- `crates/lgtm-core/{Cargo.toml,src/lib.rs}` + moved: `model.rs`, `git/`, `guard/`, `scan.rs`, `pr/`, `comment/`, `coverage/`, `verify/`, `mutate/`, `turn/`, `ipc/`, `watcher/`, `diff_source.rs`.
- `crates/lgtm-mcp/{Cargo.toml,src/lib.rs}` (from `src/mcp/mod.rs`).
- Root `src/`: `main.rs`, `cli.rs`, `headless.rs`, `app/`, `ui/`.

---

### Task 1: Baseline

- [ ] `git status --short` clean; `mkdir -p /tmp/lgtm-r3 && git archive HEAD | tar -x -C /tmp/lgtm-r3 && (cd /tmp/lgtm-r3 && cargo build --release)`.
- [ ] `cargo test 2>&1 | grep "test result"` -> `82 passed`.

### Task 2: Create the workspace and move the code

- [ ] **Step 1: Move files**

```bash
mkdir -p crates/lgtm-core/src crates/lgtm-mcp/src
for m in model.rs git guard scan.rs pr comment coverage verify mutate turn ipc watcher diff_source.rs; do git mv src/$m crates/lgtm-core/src/$m; done
git mv src/mcp/mod.rs crates/lgtm-mcp/src/lib.rs && rmdir src/mcp 2>/dev/null || true
```
- [ ] **Step 2: Root `Cargo.toml`**

```toml
[workspace]
members = ["crates/lgtm-core", "crates/lgtm-mcp"]

[workspace.package]
version = "0.7.0"
edition = "2021"
authors = ["lgtm contributors"]

[workspace.dependencies]
anyhow = "1.0.95"
serde = { version = "1.0.217", features = ["derive"] }
serde_json = "1.0.134"
crossbeam-channel = "0.5.13"
git2 = { version = "0.20.4", features = ["vendored-openssl", "vendored-libgit2"] }
notify = "6.1.1"
notify-debouncer-mini = "0.4.1"
lgtm-core = { path = "crates/lgtm-core" }
lgtm-mcp = { path = "crates/lgtm-mcp" }

[package]
name = "lgtm"
version.workspace = true
edition.workspace = true
description = "Real-Time Terminal Diff Reviewer & Agent Steering Engine"
authors.workspace = true

[dependencies]
lgtm-core.workspace = true
lgtm-mcp.workspace = true
ratatui = "0.29.0"
crossterm = { version = "0.28.1", features = ["event-stream"] }
similar = { version = "2.6.0", features = ["inline"] }
fuzzy-matcher = "0.3.7"
crossbeam-channel.workspace = true
serde_json.workspace = true
clap = { version = "4.5.23", features = ["derive"] }
anyhow.workspace = true

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
```
- [ ] **Step 3: `crates/lgtm-core/Cargo.toml`**

```toml
[package]
name = "lgtm-core"
version.workspace = true
edition.workspace = true
description = "TUI-free core of lgtm: diff model, guards, git/PR engines, scanning"
authors.workspace = true

[dependencies]
anyhow.workspace = true
serde.workspace = true
serde_json.workspace = true
crossbeam-channel.workspace = true
git2.workspace = true
notify.workspace = true
notify-debouncer-mini.workspace = true
```
`crates/lgtm-mcp/Cargo.toml`: same header (`name = "lgtm-mcp"`, description "MCP JSON-RPC stdio server for lgtm"), dependencies `lgtm-core.workspace`, `anyhow`, `serde`, `serde_json` (all `.workspace = true`).
- [ ] **Step 4: `crates/lgtm-core/src/lib.rs`**

```rust
pub mod coverage;
pub mod comment;
pub mod diff_source;
pub mod git;
pub mod guard;
pub mod ipc;
pub mod model;
pub mod mutate;
pub mod pr;
pub mod scan;
pub mod turn;
pub mod verify;
pub mod watcher;
```
At the top of `crates/lgtm-mcp/src/lib.rs` replace `use crate::{comment,git,guard,model}::...` imports with `use lgtm_core::git::GitEngine;` etc. (same names, `crate::` -> `lgtm_core::`).
- [ ] **Step 5: Bin crate root.** In `src/main.rs` replace the `mod coverage; mod diff_source; ... mod watcher;` lines for moved modules with:

```rust
use lgtm_core::{coverage, diff_source, git, guard, ipc, model, mutate, pr, scan, turn, verify, watcher};
use lgtm_mcp as mcp;
```
Keep `mod app; mod ui; mod cli; mod headless;`.
- [ ] **Step 6: Build, fix visibility.** `cargo build --workspace`. Expect errors for items that were `pub(crate)`/private but used across the new crate boundary; make exactly those items `pub`. Remove unused imports/deps that clippy reports.
- [ ] **Step 7: Subprocess test runs.** Change every `.args(["test", "--", "--nocapture"])` in `src/headless.rs` (coverage, delta fallback) and `crates/lgtm-core/src/verify/mod.rs` to `.args(["test", "--workspace", "--", "--nocapture"])`.
- [ ] **Step 8:** `cargo fmt --all && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace 2>&1 | grep "test result"` -> sum of `passed` is 82.
- [ ] **Step 9: Golden parity**

```bash
cargo build --release
for c in "check" "check --format sarif" "check --format json" "scan ."; do
  diff <(/tmp/lgtm-r3/target/release/lgtm $c 2>&1; echo $?) <(./target/release/lgtm $c 2>&1; echo $?) >/dev/null && echo "$c same" || echo "$c DIFF"
done
```
- [ ] **Step 10: Commit** `refactor: split into lgtm-core, lgtm-mcp and lgtm workspace crates` (with trailer).

### Task 3: Scripts, Makefile, CI

- [ ] `scripts/coverage.sh` and `scripts/mutate.sh`: `cargo test -- --nocapture` -> `cargo test --workspace -- --nocapture`.
- [ ] `Makefile`: `test` -> `cargo test --workspace -- --nocapture`; `lint` -> `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `install` stays `cargo install --path . --force`.
- [ ] `.github/workflows/ci.yml`: clippy step adds `--workspace`; test step `cargo test --workspace --verbose -- --nocapture`. Release packaging (`target/release/lgtm`, version grep) is unchanged.
- [ ] Verify: `scripts/coverage.sh && scripts/mutate.sh` pass; `cargo install --path . --root /tmp/lgtm-install --force` succeeds and `/tmp/lgtm-install/bin/lgtm --help` runs; `grep '^version =' Cargo.toml | head -1 | cut -d '"' -f2` prints `0.7.0`.
- [ ] Commit `build: run workspace-wide tests and lints` (with trailer).

### Task 4: Docs and cleanup

- [ ] `docs/WIKI.md` module map and `README.md` project tree: show `crates/lgtm-core`, `crates/lgtm-mcp`, root `src/` (app, ui, cli, headless).
- [ ] Mark R4 ✅ in `specs/superpowers/roadmap.md`; note the tap flow (`scripts/update-tap.sh` reads the root version) was checked locally, the remote tap run remains a manual step.
- [ ] Commit `docs: record R4 completion`; `rm -rf /tmp/lgtm-r3 /tmp/lgtm-install`.
