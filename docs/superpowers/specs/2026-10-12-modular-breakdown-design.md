# Modular Breakdown of lgtm — Design

Date: 2026-10-12 · Status: draft for review · Baseline: `main` @ a90d5b0 (v0.7.0)

## Goal

Break up the god files (`src/app.rs` 4,668 lines, `src/ui/mod.rs` 3,120, `src/main.rs` 1,089), introduce internal trait seams, and expose a reusable, TUI-free core as a Cargo workspace. No behavior changes and no new features (new features get separate specs).

## Decisions (from brainstorming)

1. Driver: maintainability and a reusable core, staged (maintainability first).
2. `App` split: mechanical `impl App` split by feature first, then state extraction only for clean seams.
3. Internal traits (`Guard`, `DiffSource`, `AgentSink`) are in scope. No dynamic or external plugin system.
4. Cargo workspace is in scope; the `lib.rs`-only intermediate step is dropped (core becomes a crate directly).
5. Async is in scope for `mcp` and PR only, last, behind a go/no-go spike. Core stays sync.

## Findings that shape the design

- `model` depends on nothing; `guard`, `git`, `pr`, `scan`, `coverage`, `verify`, `mutate`, `mcp` depend only on `model` and each other. `turn`, `comment`, `ipc`, `watcher` have no internal deps.
- No domain module imports ratatui/crossterm; only `app` and `ui` do.
- `src/mutate/mod.rs` hard-codes source paths (e.g. `"src/app.rs"`, `"src/ui/mod.rs"`). Moving files breaks these; update in the same PR as each move.
- Gate: fmt, clippy `-D warnings`, tests, coverage >= 90% (`scripts/coverage.sh`), mutation >= 90% (`scripts/mutate.sh`, runs `lgtm mutate`), SARIF self-scan.

## Stages (one PR each, cut from trunk, merged before the next; never stacked)

### 1 — Mechanical split (no signature changes)
- `app.rs` -> `app/`: `mod.rs` (struct, `new`), `input.rs`, `command.rs`, `motion.rs`, `staging.rs`, `comments.rs`, `audit.rs`, `pr.rs`, `commit.rs`, `clipboard_ipc.rs`, `test_util.rs`. Tests move beside the code they cover.
- `ui/mod.rs` -> `ui/`: `diff.rs`, `header.rs`, `status.rs`, `help.rs`, `audit.rs`, `modals/` (commit, pickers, comments).
- `main.rs` -> `main.rs` + `cli.rs` + `headless.rs` (check, scan, ci).
- Update hard-coded paths in `mutate` and any path assumptions in `scripts/`.
- Verify: test count unchanged; `lgtm --help` and `lgtm check` output identical to trunk; full gate green.

### 2 — State extraction
Order, one commit each: `CommandLine`, `Picker` (shared by file/pane pickers), `AuditState`, `PrState`, `CommitForge`.
- Sub-structs own fields and pure logic; methods needing cross-struct context stay on `App` as thin orchestrators.
- `handle_*_key` stays in `app/input.rs` and delegates.
- `ui` reads sub-structs through `&App`.
- Diff/cursor/search/visual/fold state stays on `App`.
- Add unit tests per struct where the mutation score would otherwise drop.

### 3 — Internal traits (same crate)
- `Guard`: one impl per existing guard (`arch`, `security`, `complexity`, `api_drift`, `deps`, `dead_code`, `duplicate`, `symbols`), run through a registry.
- `DiffSource`: git, PR, plain files.
- `AgentSink`: tmux, clipboard.
- Verify behavior parity with golden outputs of `lgtm check` and `lgtm scan` (markdown and SARIF) on a fixed fixture repo, captured before the stage.

### 4 — Cargo workspace
- Crates: `lgtm-core` (model, git, guard, scan, pr, comment, coverage, verify, mutate, turn, ipc, watcher), `lgtm-mcp`, `lgtm` (TUI + CLI; binary name unchanged).
- Update: `scripts/coverage.sh`, `scripts/mutate.sh`, CI workflow, release job, `scripts/update-tap.sh`, `Cargo.lock` handling, `mutate` hard-coded paths.
- Verify `cargo install --git` and the tap flow before merging.

### 5 — Boundary enforcement and docs
- CI step failing if `lgtm-core`/`lgtm-mcp` reference `ratatui` or `crossterm`.
- Update `docs/WIKI.md`, `CONTRIBUTING.md` module map (also updated incrementally in every stage PR).

### 6 — Async for `mcp` and PR (separate spec; go/no-go spike)
- tokio confined to `lgtm-mcp` and the PR client; core stays sync.
- Spike decides whether concurrency/non-blocking PR fetch justifies the dependency, binary size and test churn. If not, the stage is dropped (not deferred).

## Out of scope
Dynamic/external plugin loading, behavior changes, new features, `ViewState` extraction.

## Risks
| Risk | Mitigation |
|---|---|
| Mutation gate path breakage | Update `mutate` paths in the same PR; run `scripts/mutate.sh` locally before pushing |
| Coverage/release script path assumptions (esp. stage 4) | Audit scripts per stage; verify install and tap flow |
| Trait refactor changes behavior subtly | Golden outputs captured before stage 3 |
| Borrow-checker friction in stage 2 | Keep multi-struct methods on `App` |
| Async changes runtime semantics | Isolated to stage 6 with go/no-go spike |
| Docs drift | Docs updated in the same PR as each stage |
| CI not enforced as required check | Manual human step: confirm branch protection on `main` (unverified from checkout) |
