# Research & Technical Tradeoffs

## 1. Intent & Ground Truth
- **Problem**: Currently, `:approve`, `:lgtm`, and `A` all invoke `App::approve_gate()` (`src/app/mod.rs`), which only writes `"status": "approved"` to `<repo>/.lgtm/gate.json`. Reviewers inspecting OpenSpec/spec-rails documents inside `lgtm` (in Diff or MarkdownPreview view) cannot tick an unticked `## Review Gate` checkbox (`- [ ] ...`) on the cursor line without leaving the TUI.
- **Desired Behavior**: When the reviewer triggers `:approve`, `:lgtm`, or `A` while their cursor is on an unticked `- [ ]` line inside a `## Review Gate` section of a local working-tree file, the first press ticks `[ ]` → `[x]` on disk and reloads the diff/preview without approving `.lgtm/gate.json`. A second press (now that the box is `[x]`) falls through to `approve_gate()` and approves `.lgtm/gate.json`.
- **Verified Facts in Codebase**:
  - `src/app/command.rs:270` (`"approve" | "lgtm"`) and `src/app/input.rs:313` (`KeyCode::Char('A')`) both call `self.approve_gate()`.
  - `src/app/comments.rs:45` (`current_line_number`) resolves the 1-based line number in Diff view; `src/app/mod.rs:578` (`markdown_preview_cursor_source_line`) resolves `(u32, MarkdownDiffKind)` in MarkdownPreview view.
  - `self.pr.current_number.is_none() && self.active_revision.is_none()` identifies a local `WorkingTree` diff (`src/app/refresh.rs:80-93`).
  - `self.reload_diff()` (`src/app/refresh.rs:80`) refreshes both the diff snapshot and `MarkdownPreview`.
  - `lgtm_core` must remain free of I/O (`openspec/config.yaml`).

## 2. Options Explored
- **Option A — Pure line-matching helper in `lgtm_core` + file edit & two-step dispatch in `src/app/`**:
  - Add a pure function in `lgtm_core` (e.g. `tick_review_gate_line(content: &str, line_1based: u32) -> Option<String>`) that checks whether `line_1based` is an unticked `- [ ]` item inside a `## Review Gate` section and returns the updated content with `[ ]` replaced by `[x]`, preserving line endings (LF/CRLF).
  - In `src/app/mod.rs`, add `approve_or_tick_gate(&mut self)` (called by `:approve`, `:lgtm`, and `A`): if viewing `WorkingTree` and the cursor resolves to a non-deleted line whose file on disk matches `tick_review_gate_line`, write the file, call `self.reload_diff()`, set a status message, and return without touching `.lgtm/gate.json`. Otherwise call `self.approve_gate()`.
  - *Pros*: Pure parsing/transformation in `lgtm_core` (easy 100% unit test + mutation coverage), zero new dependencies, clean separation of I/O in `src/app/`.
  - *Cons*: None significant.
- **Option B — Inline string manipulation directly in `src/app/mod.rs`**:
  - *Pros*: Slightly fewer files touched.
  - *Cons*: Mixes markdown section parsing with file I/O, harder to mutation-test with `cargo mutants` on `lgtm_core`.
- **Option C — Full markdown AST parser (`pulldown-cmark` / `tree-sitter-md`) for gate detection**:
  - *Pros*: AST-aware.
  - *Cons*: Over-engineered for `## Review Gate` + `- [ ]`; byte-offset editing would still be needed to preserve formatting and CRLF. Violates YAGNI.

## 3. Tradeoff Matrix

| Criteria | Option A (`lgtm_core` pure helper + `src/app/` I/O) | Option B (inline in `src/app/`) | Option C (AST parser) |
| :--- | :--- | :--- | :--- |
| **Complexity** | Low | Low | High |
| **New Dependencies** | None | None | Possibly `tree-sitter-md` |
| **Performance & Latency** | < 10 µs on single file (keystroke-triggered, not per-frame) | < 10 µs | Higher parse cost |
| **Maintenance Burden** | Low; pure function easily tested | Medium | High |
| **Blast Radius** | `lgtm_core` + `src/app/{mod,command,input}.rs` | `src/app/{mod,command,input}.rs` | Broad |

## 4. Recommendation
Adopt **Option A**: implement pure `## Review Gate` checkbox detection and replacement in `lgtm_core`, and wire the two-press `approve_or_tick_gate` dispatch in `src/app/`.

## 5. Breaking Changes & Deprecations
None. Existing `:approve`, `:lgtm`, and `A` behavior on non-gate lines or when `.lgtm/gate.json` is waiting remains unchanged on the second press (or immediately when not on an unticked Review Gate line).

## Review Gate
- [x] **Gate 0 Decision**: Human decides Go (Option A) or No-Go before the proposal is drafted.
