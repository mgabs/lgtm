# Architecture & Technical Design

## 1. Overview & Invariants
This change splits Review Gate checkbox ticking into two layers:
1. **Pure Core (`lgtm-core`)**: `tick_review_gate_line(content: &str, line_1based: u32) -> Option<String>` determines whether a 1-based line lies inside a `## Review Gate` section and contains an unticked markdown task item (`- [ ] ...`), returning the updated file content with `[ ]` replaced by `[x]`.
2. **I/O Shell (`src/app/`)**: `App::approve_or_tick_gate(&mut self)` resolves the working-tree file and 1-based non-deleted cursor line from either `ActiveView::Diff` or `ActiveView::MarkdownPreview`, calls `tick_review_gate_line`, writes the updated file to disk, and triggers `reload_diff()`. If any precondition fails (non-working-tree, deleted line, non-gate line, already `[x]`), it falls through to `App::approve_gate()`.

**Invariants**:
- `lgtm_core` performs zero disk or network I/O.
- Line endings (LF `\n` and CRLF `\r\n`) and surrounding lines are preserved byte-for-byte.
- First press on an unticked Review Gate line never modifies `.lgtm/gate.json`; second press (now `[x]`) falls through to `approve_gate()`.

## 2. Domain Models, State & Interface Contracts

### `lgtm-core` (`crates/lgtm-core/src/review_gate.rs` or `lib.rs` module)
```rust
/// If `line_1based` in `content` is an unticked `- [ ]` item inside a `## Review Gate`
/// section, returns `Some(updated_content)` with the first `[ ]` on that line replaced by `[x]`.
/// Preserves LF/CRLF line endings. Returns `None` otherwise.
pub fn tick_review_gate_line(content: &str, line_1based: u32) -> Option<String>
```
**Section & Line Rules**:
- Iterate lines using `split_inclusive('\n')` so original `\n` / `\r\n` endings are preserved without re-joining transformations.
- Track `in_review_gate: bool`:
  - `true` when `line.trim_end_matches(&['\r', '\n'][..]).trim() == "## Review Gate"`
  - `false` when any other markdown heading (`trimmed_start.starts_with('#')` followed by space or `#`) is encountered (excluding code fences if needed, or simple heading check).
- At `idx + 1 == line_1based` (for `line_1based >= 1`):
  - Only match if `in_review_gate` is `true`.
  - Check `trimmed_start` starts with `"- [ ] "` (or `"- [ ]"` at end of line).
  - Replace the first `"[ ]"` in that line slice with `"[x]"` and reconstruct the string.

### `src/app/` (`src/app/mod.rs`, `src/app/command.rs`, `src/app/input.rs`)
- `App::current_working_tree_cursor(&self) -> Option<(PathBuf, u32)>`:
  - Returns `None` if `self.pr.current_number.is_some()` or `self.active_revision.is_some()`.
  - Gets `file = self.snapshot.files.get(self.selected_file_idx)?`.
  - Resolves 1-based line depending on `self.active_view`:
    - `ActiveView::MarkdownPreview`: `let (line, kind) = self.markdown_preview_cursor_source_line()?;` — returns `None` if `kind == MarkdownDiffKind::Deletion`, else `Some(line)`.
    - `ActiveView::Diff`: looks up `self.cursor_line_idx` in `file.hunks` (like `current_line_number()`), returning `None` if `LineType::HunkHeader` or `LineType::Deletion`, else `line.new_line_num`.
    - Other views: `None`.
  - Returns `Some((self.git_engine.repo_path.join(file.display_name()), line))`.
- `App::approve_or_tick_gate(&mut self)`:
  - Calls `try_tick_review_gate_at_cursor(&mut self) -> bool`.
  - If `true`: sets status `"✅ Ticked Review Gate checkbox."`, calls `self.reload_diff()`, and returns without calling `approve_gate()`.
  - If `false`: calls `self.approve_gate()`.
- Wire `A` (`src/app/input.rs:314`) and `"approve" | "lgtm"` (`src/app/command.rs:271`) to `self.approve_or_tick_gate()`.

### Rejected Alternatives
- **Inline string editing in `src/app/mod.rs`**: Rejected because it couples markdown section parsing with file I/O and bypasses `lgtm_core` mutation testing.
- **Full markdown AST parser**: Rejected as unnecessary complexity (YAGNI) and requires byte-offset reconstruction to preserve CRLF.

## 3. Architecture Diagram & Data Flow

```
  [A / :approve / :lgtm]
           |
           v
  App::approve_or_tick_gate()
           |
           +---> current_working_tree_cursor()
           |        |-- None (PR/rev/deleted/other view) --------+
           |        |-- Some((path, line_1based))                |
           |                 |                                   |
           |                 v                                   |
           |     fs::read_to_string(&path)                       |
           |                 |                                   |
           |                 v                                   |
           |     lgtm_core::tick_review_gate_line(&text, line)   |
           |        |-- None (not in ## Review Gate / not [ ]) --+
           |        |-- Some(updated)                            |
           |                 |                                   |
           |                 v                                   |
           |     fs::write(&path, updated)                       |
           |     self.reload_diff()                              |
           |     (done; gate.json untouched)                     |
           |                                                     |
           +<----------------------------------------------------+
           |
           v
  App::approve_gate()  --> updates .lgtm/gate.json
```

## 4. Boundary Rules & Budgets
- **Pure Core vs. I/O Shell**: `tick_review_gate_line` lives in `lgtm_core` with zero I/O, network, or framework dependencies. All `std::fs` reads/writes and `reload_diff()` stay in `src/app/`.
- **Concurrency, Latency & Resource Budgets**: Zero work on the `< 16 ms` render frame path (`NFR-GATE-1`). Execution only occurs on explicit `:approve`, `:lgtm`, or `A` keystrokes and reads/writes a single markdown file (< 1 ms typical).

## 5. Testing Strategy
1. **BDD Scenarios**: Add scenario in `tests/bdd/features/clean_commands_shortcuts.feature` (or `worktree_switcher.feature` / `markdown_preview.feature`) and Rust BDD test covering 1st-press tick + 2nd-press `gate.json` approval in Diff/MarkdownPreview.
2. **Unit Tests (>= 95% coverage)**:
   - `lgtm-core`: unticked gate line, already-ticked `[x]` line, non-gate `- [ ]` line, heading reset after `## Review Gate`, CRLF preservation, out-of-bounds/zero line, indented `- [ ]`.
   - `src/app/`: `approve_or_tick_gate` in `Diff` and `MarkdownPreview` (1st press ticks file and leaves `gate.json` pending; 2nd press approves `gate.json`), guard on PR/revision/deleted line.
3. **Mutation Testing (100% kill)**: Run `cargo mutants` on `lgtm_core::review_gate` and verify all mutants are killed.

## Review Gate
- [x] **Gate 3 Approval**: Architecture & design reviewed and approved by human before task breakdown.
