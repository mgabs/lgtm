# R2 State Extraction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Move related `App` fields (and their pure logic) into small sub-structs, with no behavior change.

**Architecture:** New `src/app/state.rs` holds `CommandLine`, `FilePicker`, `PanePicker`, `AuditState`, `PrState`. `App` owns one of each. Field accesses are renamed mechanically (`app.command_input` -> `app.command.input`); compile errors find the rest. One commit per struct.

**Tech Stack:** Rust 2021, perl (one-off renames), cargo fmt/clippy/test.

**Spec:** `docs/superpowers/specs/2026-10-12-modular-breakdown-design.md` stage 2; roadmap item R2.

## Global Constraints

- Starts from R1 merged on trunk (branch `rel.0.8` builds on it). One PR for R2, one commit per struct.
- Behavior-identical: all existing tests pass unchanged except for field-path renames; `lgtm check` and `lgtm scan` output matches a trunk build.
- `cargo fmt --all --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `scripts/coverage.sh` and `scripts/mutate.sh` (both >= 90%) stay green.
- `handle_*_key` stays in `app/input.rs`; `ui` reads sub-structs through `&App`.
- Diff, cursor, search, visual and fold state stay on `App`.
- Deviations from spec (YAGNI): no `CommitForge` (one field, `commit_message_input`, stays on `App`); two small pickers instead of one generic `Picker` (their shapes differ: file picker has query + filtered indices, pane picker is list + index).
- Commits carry `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>`.

## File Structure

- Create `src/app/state.rs`: the five structs, their constructors, pure logic and unit tests.
- Modify `src/app/mod.rs`: `mod state; pub use state::*;`, struct fields, `App::new` initializers.
- Modify (renames only): `src/app/{input,command,motion,comments,external,pr,audit}.rs`, `src/ui/{mod,command,status,header,audit}.rs`, `src/ui/modals/pickers.rs`.

## Rename helper (used by every task)

Run from the repo root. Each task passes its own pairs.

```bash
rn() { # usage: rn old new   (rewrites "<expr>.old" as "<expr>.new" in all sources)
  find src -name '*.rs' -print0 | xargs -0 perl -pi -e "s/\\.\\Q$1\\E\\b/.$2/g"
}
```

---

### Task 1: Baseline

- [ ] **Step 1:** `git switch rel.0.8 && git status --short` (only ` D prd` allowed).
- [ ] **Step 2:** Build trunk golden outputs as in R1 Task 1 into `/tmp/lgtm-baseline` (`--help`, `check`, `scan`, sorted passing test names). `cargo test 2>&1 | grep "test result"` must show `75 passed`.

### Task 2: `CommandLine`

**Files:** Create `src/app/state.rs`; modify `src/app/mod.rs`, `src/app/command.rs`, `src/app/input.rs`, `src/ui/{command,mod,status}.rs`.

**Interfaces:**
- Produces: `pub struct CommandLine { pub input: String, pub history: Vec<String>, pub history_idx: Option<usize>, pub completions: Vec<String>, pub completion_idx: Option<usize>, pub prefix: String }` with `Default`; `App.command: CommandLine`.

- [ ] **Step 1: Write the failing test** in `src/app/state.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_line_default_is_empty() {
        let c = CommandLine::default();
        assert!(c.input.is_empty() && c.history.is_empty() && c.completions.is_empty());
        assert_eq!((c.history_idx, c.completion_idx), (None, None));
    }
}
```

- [ ] **Step 2:** `cargo test command_line_default` -> FAIL (`CommandLine` not found).
- [ ] **Step 3: Add the struct** at the top of `src/app/state.rs`:

```rust
#[derive(Debug, Default, Clone)]
pub struct CommandLine {
    pub input: String,
    pub history: Vec<String>,
    pub history_idx: Option<usize>,
    pub completions: Vec<String>,
    pub completion_idx: Option<usize>,
    pub prefix: String,
}
```

In `src/app/mod.rs` add `mod state; pub use state::*;`.

- [ ] **Step 4: Swap the fields.** In `App` delete the six `command_*` fields and add `pub command: CommandLine,`; in `App::new` delete the six `command_*: ...` initializers and add `command: CommandLine::default(),`. Then:

```bash
rn command_input command.input
rn command_history_idx command.history_idx
rn command_history command.history
rn command_completion_idx command.completion_idx
rn command_completions command.completions
rn command_prefix command.prefix
cargo fmt --all && cargo build
```
Rename `command_history_idx` before `command_history` (the `\b` guard makes order safe, but keep it). Fix any remaining compile errors by hand (struct-literal sites, local variables).

- [ ] **Step 5: Move pure autocomplete logic.** If `autocomplete_next`/`autocomplete_prev` in `src/app/command.rs` touch only `self.command.*`, move both into `impl CommandLine` in `state.rs` (body unchanged except `self.command.` -> `self.`), and change callers from `app.autocomplete_next()` to `app.command.autocomplete_next()`. If a body reads other `App` fields, leave it on `App`.
- [ ] **Step 6:** `cargo test` -> `76 passed`; `cargo clippy --all-targets --all-features -- -D warnings` clean.
- [ ] **Step 7: Commit**

```bash
git add src && git commit -m "refactor(app): extract CommandLine state

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

### Task 3: `FilePicker` and `PanePicker`

**Files:** `src/app/state.rs`, `src/app/mod.rs`, `src/app/motion.rs` (`update_file_picker_filter`), `src/app/{input,command}.rs`, `src/ui/modals/pickers.rs`.

**Interfaces:**
- Produces: `FilePicker { pub query: String, pub selected_idx: usize, pub filtered_indices: Vec<usize> }` with `fn refilter(&mut self, names: &[String])`; `PanePicker { pub list: Vec<crate::ipc::TmuxPane>, pub selected_idx: usize }`; `App.file_picker`, `App.pane_picker`. `App::update_file_picker_filter` remains as a thin wrapper that builds `names` from `snapshot.files` and calls `self.file_picker.refilter(&names)`.

- [ ] **Step 1: Failing test** in `state.rs` tests:

```rust
#[test]
fn file_picker_refilter_fuzzy_and_resets_selection() {
    let names: Vec<String> = ["src/main.rs", "src/app/mod.rs", "README.md"]
        .iter().map(|s| s.to_string()).collect();
    let mut p = FilePicker::default();
    p.refilter(&names);
    assert_eq!(p.filtered_indices, vec![0, 1, 2]);
    p.query = "readme".into();
    p.selected_idx = 2;
    p.refilter(&names);
    assert_eq!(p.filtered_indices, vec![2]);
    assert_eq!(p.selected_idx, 0);
}
```

- [ ] **Step 2:** `cargo test file_picker_refilter` -> FAIL.
- [ ] **Step 3: Add structs** (move the matcher logic verbatim from `update_file_picker_filter`):

```rust
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

#[derive(Debug, Default, Clone)]
pub struct FilePicker {
    pub query: String,
    pub selected_idx: usize,
    pub filtered_indices: Vec<usize>,
}

impl FilePicker {
    pub fn refilter(&mut self, names: &[String]) {
        if self.query.is_empty() {
            self.filtered_indices = (0..names.len()).collect();
        } else {
            let matcher = SkimMatcherV2::default();
            let mut scored: Vec<(i64, usize)> = names
                .iter()
                .enumerate()
                .filter_map(|(i, n)| matcher.fuzzy_match(n, &self.query).map(|s| (s, i)))
                .collect();
            scored.sort_by_key(|a| std::cmp::Reverse(a.0));
            self.filtered_indices = scored.into_iter().map(|(_, i)| i).collect();
        }
        if self.selected_idx >= self.filtered_indices.len() {
            self.selected_idx = 0;
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct PanePicker {
    pub list: Vec<crate::ipc::TmuxPane>,
    pub selected_idx: usize,
}
```

(If `TmuxPane` lacks `Debug`/`Clone`, drop those derives from `PanePicker` rather than editing `ipc`.) Replace the body of `update_file_picker_filter` in `motion.rs`:

```rust
pub(crate) fn update_file_picker_filter(&mut self) {
    let names: Vec<String> = self.snapshot.files.iter().map(|f| f.display_name()).collect();
    self.file_picker.refilter(&names);
}
```
Remove now-unused `SkimMatcherV2` imports from `motion.rs`.

- [ ] **Step 4: Swap fields** (delete old fields/initializers, add `file_picker: FilePicker::default()`, `pane_picker: PanePicker::default()`), then:

```bash
rn picker_query file_picker.query
rn picker_selected_idx file_picker.selected_idx
rn picker_filtered_indices file_picker.filtered_indices
rn pane_list pane_picker.list
rn pane_selected_idx pane_picker.selected_idx
cargo fmt --all && cargo build
```
- [ ] **Step 5:** `cargo test` -> `77 passed`; clippy clean.
- [ ] **Step 6: Commit** `refactor(app): extract FilePicker and PanePicker state` (with trailer).

### Task 4: `AuditState`

**Files:** `src/app/state.rs`, `src/app/mod.rs`, `src/app/{audit,command,input}.rs`, `src/ui/{audit,header,status}.rs`.

**Interfaces:**
- Produces: `AuditState { pub alerts: Vec<GuardAlert>, pub files: Vec<FileDiff>, pub selected_idx: usize, pub filter_severity: Option<Severity>, pub root_dir: PathBuf, pub focus: AuditPaneFocus, pub file_filter: Option<String>, pub inspector_scroll: usize, pub maximized_inspector: bool }`, `AuditState::new(root_dir: PathBuf) -> Self`; `App.audit`.

- [ ] **Step 1: Failing test:**

```rust
#[test]
fn audit_state_new_starts_on_issues_pane() {
    let a = AuditState::new(std::path::PathBuf::from("/repo"));
    assert!(a.alerts.is_empty() && a.files.is_empty());
    assert_eq!(a.selected_idx, 0);
    assert_eq!(a.focus, AuditPaneFocus::Issues);
    assert!(a.filter_severity.is_none() && a.file_filter.is_none() && !a.maximized_inspector);
}
```
(Requires `AuditPaneFocus: PartialEq + Debug`; if missing, add the derives where it is defined.)

- [ ] **Step 2:** `cargo test audit_state_new` -> FAIL.
- [ ] **Step 3: Add struct** (imports: `crate::guard::{GuardAlert, Severity}`, `crate::model::FileDiff`, `std::path::PathBuf`, and `AuditPaneFocus` from its current module):

```rust
#[derive(Debug, Clone)]
pub struct AuditState {
    pub alerts: Vec<GuardAlert>,
    pub files: Vec<FileDiff>,
    pub selected_idx: usize,
    pub filter_severity: Option<Severity>,
    pub root_dir: PathBuf,
    pub focus: AuditPaneFocus,
    pub file_filter: Option<String>,
    pub inspector_scroll: usize,
    pub maximized_inspector: bool,
}

impl AuditState {
    pub fn new(root_dir: PathBuf) -> Self {
        Self {
            alerts: Vec::new(),
            files: Vec::new(),
            selected_idx: 0,
            filter_severity: None,
            root_dir,
            focus: AuditPaneFocus::Issues,
            file_filter: None,
            inspector_scroll: 0,
            maximized_inspector: false,
        }
    }
}
```

- [ ] **Step 4: Swap fields** (`audit: AuditState::new(repo_path.clone())` in `App::new`), then:

```bash
for f in alerts files selected_idx filter_severity root_dir focus file_filter inspector_scroll maximized_inspector; do rn audit_$f audit.$f; done
cargo fmt --all && cargo build
```
- [ ] **Step 5:** `cargo test` -> `78 passed`; clippy clean.
- [ ] **Step 6: Commit** `refactor(app): extract AuditState` (with trailer).

### Task 5: `PrState`

**Files:** `src/app/state.rs`, `src/app/mod.rs`, `src/app/{pr,comments,external,command,input}.rs`, `src/ui/{header,modals/pickers}.rs`.

**Interfaces:**
- Produces: `PrState { pub current_number: Option<u64>, pub list: Vec<crate::pr::PullRequestSummary>, pub selected_idx: usize }`, `PrState::new(current_number: Option<u64>)`; `App.pr`.

- [ ] **Step 1: Failing test:**

```rust
#[test]
fn pr_state_new_keeps_explicit_number() {
    let p = PrState::new(Some(42));
    assert_eq!(p.current_number, Some(42));
    assert!(p.list.is_empty());
    assert_eq!(p.selected_idx, 0);
}
```
- [ ] **Step 2:** `cargo test pr_state_new` -> FAIL.
- [ ] **Step 3: Add struct:**

```rust
#[derive(Debug, Clone)]
pub struct PrState {
    pub current_number: Option<u64>,
    pub list: Vec<crate::pr::PullRequestSummary>,
    pub selected_idx: usize,
}

impl PrState {
    pub fn new(current_number: Option<u64>) -> Self {
        Self { current_number, list: Vec::new(), selected_idx: 0 }
    }
}
```
(Drop `Debug`/`Clone` if `PullRequestSummary` lacks them.)

- [ ] **Step 4: Swap fields** (`pr: PrState::new(explicit_pr)`), then:

```bash
rn current_pr_number pr.current_number
rn pr_list pr.list
rn pr_selected_idx pr.selected_idx
cargo fmt --all && cargo build
```
`rn pr_list` is safe only on field accesses (`.pr_list`); free functions named `pr_list` in `src/pr/` are not dot-accessed. Check `git diff --stat` shows no edits under `src/pr/`.

- [ ] **Step 5:** `cargo test` -> `79 passed`; clippy clean.
- [ ] **Step 6: Commit** `refactor(app): extract PrState` (with trailer).

### Task 6: Gate, docs, cleanup

**Files:** `docs/WIKI.md` (module map row for `src/app/`), `docs/superpowers/roadmap.md` (mark R2 done; note no `CommitForge`).

- [ ] **Step 1:** Parity against a trunk build run in the same working tree:

```bash
for c in --help check scan; do diff <(/tmp/lgtm-trunk/target/release/lgtm $c 2>&1; echo $?) <(./target/release/lgtm $c 2>&1; echo $?) >/dev/null && echo "$c same" || echo "$c DIFF"; done
```
Build `/tmp/lgtm-trunk` first with `mkdir /tmp/lgtm-trunk && git archive <R1 commit> | tar -x -C /tmp/lgtm-trunk && (cd /tmp/lgtm-trunk && cargo build --release)`.
- [ ] **Step 2:** `cargo fmt --all --check && cargo clippy --all-targets --all-features -- -D warnings && scripts/coverage.sh && scripts/mutate.sh` -> all pass.
- [ ] **Step 3:** Add `state` to the `src/app/` row in `docs/WIKI.md`; mark R2 ✅ in the roadmap.
- [ ] **Step 4: Commit** `docs: record R2 completion` (with trailer). Remove `/tmp/lgtm-baseline` and `/tmp/lgtm-trunk`.
