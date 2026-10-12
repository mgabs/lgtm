# R3 Internal Traits Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Introduce `Guard`, `DiffSource` and `AgentSink` traits (same crate) so later work (B1 config, A5 sinks) adds implementations instead of editing call sites.

**Architecture:** `Guard` gets a static registry that `SecurityAndTestGuard` iterates (alert order preserved). `DiffSource` unifies working tree / revision / PR loading behind `App::diff_source()`. `AgentSink` unifies tmux / clipboard delivery behind `App::agent_sink()`. Concrete guard/engine types and their inherent functions stay, so existing tests keep working.

**Tech Stack:** Rust 2021, `anyhow`, cargo fmt/clippy/test.

**Spec:** `docs/superpowers/specs/2026-10-12-modular-breakdown-design.md` stage 3; roadmap item R3.

## Global Constraints

- Starts from R2 (branch `rel.0.8`). One commit per trait.
- Golden parity: `lgtm check` and `lgtm scan` (markdown) output and exit codes identical to an R2 build run in the same working tree; `check --format sarif` and `--format json` too.
- All existing tests pass unchanged; new tests only add.
- `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `scripts/coverage.sh` and `scripts/mutate.sh` (both >= 90%) stay green.
- No new dependencies. No `async`.
- Only intentional user-visible change: agent-delivery status wording (Task 4). No test asserts on it.
- Commits carry `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>`.

## File Structure

- Modify `src/guard/mod.rs`: `Guard` trait, `registry()`, loops in `scan_file`/`scan_snapshot`.
- Modify `src/guard/{deps,duplicate,security,arch,complexity,symbols,api_drift,dead_code}.rs`: one `impl Guard` each.
- Create `src/diff_source.rs`: `DiffSource` + three impls. Register in `src/main.rs` (`mod diff_source;`).
- Create `src/ipc/sink.rs`: `AgentSink`, `TmuxSink`, `ClipboardSink`; `pub mod sink;` in `src/ipc/mod.rs`.
- Modify `src/app/{mod,command,input,comments,external}.rs`: use sources and sinks.

---

### Task 1: Baseline

- [ ] **Step 1:** `git switch rel.0.8 && git status --short` (clean).
- [ ] **Step 2:** Build an R2 reference binary outside the repo:

```bash
mkdir -p /tmp/lgtm-r2 && git archive HEAD | tar -x -C /tmp/lgtm-r2 && (cd /tmp/lgtm-r2 && cargo build --release)
```
- [ ] **Step 3:** `cargo test 2>&1 | grep "test result"` -> `79 passed`.

### Task 2: `Guard` trait and registry

**Files:** `src/guard/mod.rs` and the eight guard files.

**Interfaces:**
- Produces: `pub trait Guard { fn name(&self) -> &'static str; fn scan_file(&self, file: &FileDiff) -> Vec<GuardAlert> { Vec::new() } fn scan_snapshot(&self, files: &[FileDiff]) -> Vec<GuardAlert> { Vec::new() } }` and `pub fn registry() -> &'static [&'static dyn Guard]` in this order: `DependencyGuard`, `DuplicateHelperGuard`, `AdvancedSecurityGuard`, `ArchitectureGuard`, `ComplexityGuard`, `SymbolGuard`, `ApiDriftGuard`, `DeadCodeGuard`.

- [ ] **Step 1: Failing test** appended to the `tests` module of `src/guard/mod.rs` (create the module with `use super::*;` if absent):

```rust
#[test]
fn registry_order_is_stable() {
    let names: Vec<&str> = registry().iter().map(|g| g.name()).collect();
    assert_eq!(
        names,
        ["deps", "duplicate", "security", "arch", "complexity", "symbols", "api_drift", "dead_code"]
    );
}
```
- [ ] **Step 2:** `cargo test registry_order` -> FAIL (`registry` not found).
- [ ] **Step 3: Add trait and registry** to `src/guard/mod.rs` (after `GuardAlert`):

```rust
pub trait Guard {
    fn name(&self) -> &'static str;
    fn scan_file(&self, _file: &FileDiff) -> Vec<GuardAlert> {
        Vec::new()
    }
    fn scan_snapshot(&self, _files: &[FileDiff]) -> Vec<GuardAlert> {
        Vec::new()
    }
}

pub fn registry() -> &'static [&'static dyn Guard] {
    &[
        &deps::DependencyGuard,
        &duplicate::DuplicateHelperGuard,
        &security::AdvancedSecurityGuard,
        &arch::ArchitectureGuard,
        &complexity::ComplexityGuard,
        &symbols::SymbolGuard,
        &api_drift::ApiDriftGuard,
        &dead_code::DeadCodeGuard,
    ]
}
```
- [ ] **Step 4: Implement per guard**, appended to each guard file (outside its `tests` module). The inherent associated functions stay; `Self::scan_file(file)` resolves to them. Files with both (`arch`) forward both; files with one forward that one:

```rust
// deps.rs
impl super::Guard for DependencyGuard {
    fn name(&self) -> &'static str { "deps" }
    fn scan_file(&self, file: &FileDiff) -> Vec<GuardAlert> { Self::scan_file(file) }
}
```
Same shape, with these `name()` values and forwarded methods:

| File | Struct | name | Forwards |
|---|---|---|---|
| `duplicate.rs` | `DuplicateHelperGuard` | `duplicate` | `scan_file` |
| `security.rs` | `AdvancedSecurityGuard` | `security` | `scan_file` |
| `arch.rs` | `ArchitectureGuard` | `arch` | `scan_file` and `scan_snapshot` |
| `complexity.rs` | `ComplexityGuard` | `complexity` | `scan_file` |
| `symbols.rs` | `SymbolGuard` | `symbols` | `scan_file` |
| `api_drift.rs` | `ApiDriftGuard` | `api_drift` | `scan_file` |
| `dead_code.rs` | `DeadCodeGuard` | `dead_code` | `scan_snapshot` |

- [ ] **Step 5: Drive the pipeline from the registry.** In `SecurityAndTestGuard::scan_file` replace the seven `alerts.extend(...::scan_file(file));` lines (and the `// Subsystem guards` comment) with:

```rust
for g in registry() {
    alerts.extend(g.scan_file(file));
}
```
In `scan_snapshot` replace the two final blocks (`// Architectural Blast Radius Gate` and `// Dead Code Gate`) with:

```rust
// Snapshot-level guards (blast radius, dead code)
for g in registry() {
    all_alerts.extend(g.scan_snapshot(files));
}
```
Keep the assertion-density gate before this loop (it must stay between per-file alerts and snapshot guards to preserve order).
- [ ] **Step 6:** `cargo fmt --all && cargo test` -> `80 passed`; clippy clean.
- [ ] **Step 7: Golden parity**

```bash
cargo build --release
for c in "check" "check --format sarif" "check --format json" "scan"; do
  diff <(/tmp/lgtm-r2/target/release/lgtm $c 2>&1; echo $?) <(./target/release/lgtm $c 2>&1; echo $?) >/dev/null && echo "$c same" || echo "$c DIFF"
done
```
Expected: all `same`. If `scan` takes a path argument in `--help`, pass `.`.
- [ ] **Step 8: Commit** `refactor(guard): add Guard trait and registry` (with trailer).

### Task 3: `DiffSource`

**Files:** Create `src/diff_source.rs`; modify `src/main.rs` (`mod diff_source;`), `src/app/mod.rs`, `src/app/command.rs`, `src/app/input.rs`.

**Interfaces:**
- Produces: `pub trait DiffSource { fn snapshot(&self) -> anyhow::Result<DiffSnapshot>; fn describe(&self) -> String; }`; `WorkingTreeSource<'a>(pub &'a GitEngine)` (describe `"Diff"`), `RevisionSource<'a> { pub engine: &'a GitEngine, pub rev: &'a str }` (describe `"Revision '<rev>'"`), `PullRequestSource(pub u64)` (describe `"PR #<n> diff"`); `App::diff_source(&self) -> Box<dyn DiffSource + '_>` choosing PR if `pr.current_number` is set, else revision if `active_revision` is set, else working tree.

- [ ] **Step 1: Failing test** in `src/diff_source.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describe_matches_status_wording() {
        let g = GitEngine::new(".");
        assert_eq!(WorkingTreeSource(&g).describe(), "Diff");
        assert_eq!(RevisionSource { engine: &g, rev: "HEAD~1" }.describe(), "Revision 'HEAD~1'");
        assert_eq!(PullRequestSource(7).describe(), "PR #7 diff");
    }
}
```
- [ ] **Step 2:** add `mod diff_source;` to `src/main.rs`; `cargo test describe_matches` -> FAIL (types missing).
- [ ] **Step 3: Implement** `src/diff_source.rs`:

```rust
use crate::git::GitEngine;
use crate::model::DiffSnapshot;
use crate::pr::PrEngine;
use anyhow::Result;

pub trait DiffSource {
    fn snapshot(&self) -> Result<DiffSnapshot>;
    fn describe(&self) -> String;
}

pub struct WorkingTreeSource<'a>(pub &'a GitEngine);

impl DiffSource for WorkingTreeSource<'_> {
    fn snapshot(&self) -> Result<DiffSnapshot> {
        self.0.compute_diff_snapshot()
    }
    fn describe(&self) -> String {
        "Diff".to_string()
    }
}

pub struct RevisionSource<'a> {
    pub engine: &'a GitEngine,
    pub rev: &'a str,
}

impl DiffSource for RevisionSource<'_> {
    fn snapshot(&self) -> Result<DiffSnapshot> {
        self.engine.compute_revision_diff(self.rev)
    }
    fn describe(&self) -> String {
        format!("Revision '{}'", self.rev)
    }
}

pub struct PullRequestSource(pub u64);

impl DiffSource for PullRequestSource {
    fn snapshot(&self) -> Result<DiffSnapshot> {
        PrEngine::fetch_pr_diff(self.0)
    }
    fn describe(&self) -> String {
        format!("PR #{} diff", self.0)
    }
}
```
- [ ] **Step 4: Collapse `App::reload_diff`** in `src/app/mod.rs` (three near-identical branches) into:

```rust
pub fn diff_source(&self) -> Box<dyn crate::diff_source::DiffSource + '_> {
    use crate::diff_source::*;
    if let Some(n) = self.pr.current_number {
        Box::new(PullRequestSource(n))
    } else if let Some(rev) = self.active_revision.as_deref() {
        Box::new(RevisionSource { engine: &self.git_engine, rev })
    } else {
        Box::new(WorkingTreeSource(&self.git_engine))
    }
}

pub fn reload_diff(&mut self) {
    let source = self.diff_source();
    let label = source.describe();
    let Ok(new_snapshot) = source.snapshot() else {
        return;
    };
    drop(source);
    self.alerts = SecurityAndTestGuard::scan_snapshot(&new_snapshot.files);
    self.snapshot = new_snapshot;
    self.comments = self.comment_store.load_comments();
    if self.selected_file_idx >= self.snapshot.files.len() && !self.snapshot.files.is_empty() {
        self.selected_file_idx = self.snapshot.files.len() - 1;
    }
    self.update_file_picker_filter();
    self.status_message = Some(format!("{} updated ({} alerts)", label, self.alerts.len()));
}
```
Status texts are identical to the old three branches (`PR #n diff updated`, `Revision 'r' updated`, `Diff updated`).
- [ ] **Step 5: Route remaining PR fetches through the trait.** Replace every `crate::pr::PrEngine::fetch_pr_diff(<n>)` in `src/app/{mod,command,input}.rs` (`grep -rn fetch_pr_diff src/app`) with `crate::diff_source::DiffSource::snapshot(&crate::diff_source::PullRequestSource(<n>))`. Leave `post_review` and `list_pull_requests` alone. Leave the `reload` ex-command's working-tree branch untouched (it ignores `active_revision` today; preserving that).
- [ ] **Step 6:** `cargo fmt --all && cargo test` -> `81 passed`; clippy clean; golden parity loop from Task 2 Step 7 -> all `same`.
- [ ] **Step 7: Commit** `refactor: add DiffSource trait and unify diff reload` (with trailer).

### Task 4: `AgentSink`

**Files:** Create `src/ipc/sink.rs`; modify `src/ipc/mod.rs`, `src/app/mod.rs`, `src/app/{command,input,comments,external}.rs`.

**Interfaces:**
- Produces: `pub trait AgentSink { fn send(&self, text: &str) -> anyhow::Result<()>; fn label(&self) -> String; }`; `TmuxSink<'a>(pub &'a str)` (label `"pane <id>"`), `ClipboardSink` (label `"clipboard (OSC 52)"`, `send` never fails); `App::agent_sink(&self) -> Box<dyn AgentSink + '_>` = `TmuxSink` when `agent_pane` is set, else `ClipboardSink`.

- [ ] **Step 1: Failing test** in `src/ipc/sink.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_name_the_target() {
        assert_eq!(TmuxSink("%3").label(), "pane %3");
        assert_eq!(ClipboardSink.label(), "clipboard (OSC 52)");
    }
}
```
- [ ] **Step 2:** add `pub mod sink;` to `src/ipc/mod.rs`; `cargo test labels_name` -> FAIL.
- [ ] **Step 3: Implement** `src/ipc/sink.rs`:

```rust
use super::IpcBridge;
use anyhow::Result;

pub trait AgentSink {
    fn send(&self, text: &str) -> Result<()>;
    fn label(&self) -> String;
}

pub struct TmuxSink<'a>(pub &'a str);

impl AgentSink for TmuxSink<'_> {
    fn send(&self, text: &str) -> Result<()> {
        IpcBridge::send_to_tmux(self.0, text)
    }
    fn label(&self) -> String {
        format!("pane {}", self.0)
    }
}

pub struct ClipboardSink;

impl AgentSink for ClipboardSink {
    fn send(&self, text: &str) -> Result<()> {
        IpcBridge::send_osc52_clipboard(text);
        Ok(())
    }
    fn label(&self) -> String {
        "clipboard (OSC 52)".to_string()
    }
}
```
- [ ] **Step 4: Add `App::agent_sink`** to `src/app/external.rs`:

```rust
pub fn agent_sink(&self) -> Box<dyn crate::ipc::sink::AgentSink + '_> {
    use crate::ipc::sink::*;
    match self.agent_pane.as_deref() {
        Some(pane) => Box::new(TmuxSink(pane)),
        None => Box::new(ClipboardSink),
    }
}
```
- [ ] **Step 5: Migrate call sites.** Inventory: `grep -rn "send_to_tmux\|send_osc52_clipboard" src/app` (about 12 sites in `command.rs`, `input.rs`, `comments.rs`, `external.rs`; `src/app/command.rs` also lists panes via `list_tmux_panes`, which stays). Two shapes occur:

Shape A (ignore the tmux error):

```rust
// before
if let Some(ref pane) = self.agent_pane {
    let _ = IpcBridge::send_to_tmux(pane, &payload);
    self.status_message = Some(format!("Injected {} issues to pane {}", n, pane));
} else {
    IpcBridge::send_osc52_clipboard(&payload);
    self.status_message = Some(format!("Copied {} issues to clipboard (OSC 52)", n));
}
// after
let sink = self.agent_sink();
let _ = sink.send(&payload);
self.status_message = Some(format!("Sent {} issues to {}", n, sink.label()));
```
Shape B (tmux error falls back to clipboard), only in `comments.rs` and one in `external.rs`:

```rust
// after
let sink = self.agent_sink();
match sink.send(&payload) {
    Ok(()) => self.status_message = Some(format!("Sent to {}: {}", sink.label(), what)),
    Err(e) => {
        ClipboardSink.send(&payload).ok();
        self.status_message = Some(format!("Send failed: {}", e));
    }
}
```
Rules: keep each site's existing payload and verb (`Injected`, `Copied`, `Exported ... and notified`) where the wording still reads correctly with `sink.label()`; otherwise use `Sent ... to <label>`. Borrow note: `self.agent_sink()` borrows `self` immutably; compute the message string before assigning `self.status_message`, or `drop(sink)` first. The "Exported file and notified pane" sites write the directive file first and send `Please review directives in <path>` instead of the full prompt; on the clipboard path they send the full `prompt`. Preserve that by choosing the payload from `self.agent_pane.is_some()` and sending it through the sink.
- [ ] **Step 6:** `grep -rn "send_to_tmux\|send_osc52_clipboard" src/app` -> no matches. `cargo fmt --all && cargo test` -> `82 passed`; clippy clean (unused imports of `IpcBridge` in migrated files must be removed).
- [ ] **Step 7:** Golden parity loop (Task 2 Step 7) still `same`.
- [ ] **Step 8: Commit** `refactor(ipc): add AgentSink trait and route agent delivery through it` (with trailer).

### Task 5: Gate, docs, cleanup

**Files:** `docs/WIKI.md` (module map: add `src/diff_source.rs`; mention `Guard` registry in the `src/guard/` row and sinks in `src/ipc/`), `docs/superpowers/roadmap.md` (R3 -> ✅).

- [ ] **Step 1:** `cargo fmt --all --check && cargo clippy --all-targets --all-features -- -D warnings && scripts/coverage.sh && scripts/mutate.sh` -> all pass.
- [ ] **Step 2:** Final golden parity loop (Task 2 Step 7).
- [ ] **Step 3:** Update docs; commit `docs: record R3 completion` (with trailer). `rm -rf /tmp/lgtm-r2`.
