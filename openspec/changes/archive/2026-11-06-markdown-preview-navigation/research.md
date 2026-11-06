# Research & Technical Tradeoffs

## 1. Intent & Ground Truth
- **Intent**: Enable navigation (`[` / `]`), external editor launch (`e` / `o`), file picking (`f` / `Ctrl-p`), reviewed toggle (`v`), comment reply (`r`), and help (`?`) directly from Markdown Preview, while keeping the TUI responsive and avoiding stale diff/live-update locks.
- **Ground Truth**:
  - `src/app/input.rs:handle_markdown_preview_key` currently ignores `e`, `o`, `[`, `]`, `f`, `v`, `r`, `?`.
  - In `src/app/refresh.rs` (`local_ready_to_apply`, `apply_ready_local`) and `src/app/pr.rs` (`apply_ready_pr`), background diff updates are blocked with `"Diff update ready; close this view or selection to apply"` unless `active_view == ActiveView::Diff`.
  - `src/app/rows.rs:invalidate_diff_content` does not clear `markdown_preview.cached_file_idx`, causing Markdown Preview to render stale cached lines even when underlying file content changes.
  - Diff view already implements complete bracket chord navigation (`]f`, `]h`, `]c`, `]u`, `]w`, `]t`) and editor launching (`open_in_editor`).

## 2. Options Explored

### Option A: Exit to Diff on Action (YAGNI / Minimal Surface)
- Pressing `e`, `o`, `[`, `]` syncs the preview cursor to the corresponding source line (`sync_cursor_to_markdown_row`), sets `active_view = ActiveView::Diff`, and delegates immediately to Diff view handlers (`open_in_editor` or `handle_diff_key`).
- `f` opens `ActiveView::FilePicker`; picking a file returns to `Diff`.
- `v` toggles review on current file directly.
- `r` triggers reply/comment input.
- `?` opens `ActiveView::Help`.
- **Pros**:
  - Reuses 100% of battle-tested Diff view navigation and editor handling.
  - Zero state duplication or chord synchronization.
  - Automatically resolves the frozen live-update issue because user returns to Diff view where live-reload is active.
- **Cons**:
  - Consecutive `.md` file reading requires pressing `m` again on the next markdown file.

### Option B: Sticky In-Preview Navigation & Live-Reload Unification
- Implement native chord navigation and file navigation inside `handle_markdown_preview_key`.
- Relax `apply_ready_local` and `apply_ready_pr` to allow `ActiveView::MarkdownPreview`.
- Invalidate `markdown_preview.cached_file_idx` on diff reload.
- **Pros**:
  - Keeps user inside Markdown Preview across multiple markdown files.
- **Cons**:
  - Significantly larger blast radius across refresh loops, PR loading, and watcher concurrency.
  - More complex state management when stepping between markdown and non-markdown files.

## 3. Tradeoff Matrix

| Criteria | Option A (Exit to Diff) | Option B (Sticky Preview) |
| :--- | :--- | :--- |
| **Complexity** | Low (~25 lines in `input.rs` and `status.rs`) | Medium-High (touches `refresh.rs`, `pr.rs`, `rows.rs`, `input.rs`, `markdown.rs`) |
| **New Dependencies** | None | None |
| **Performance & Latency** | Instant (< 1 µs keystroke dispatch) | Keystroke instant, but adds watcher/re-render overhead |
| **Maintenance Burden** | Minimal; single source of truth for chords | Moderate; dual-view maintenance |
| **Blast Radius** | Isolated to `handle_markdown_preview_key` | Touches core view-apply gates and watcher handling |

## 4. Recommendation
Option A is recommended. It strictly follows YAGNI, keeps the change localized, guarantees immediate access to all existing diff navigation chords and editor workflows, and avoids modifying core background refresh gates.

## 5. Breaking Changes & Deprecations
- Modifies `openspec/specs/markdown-preview/spec.md`:
  - Updates `Requirement: Preview Navigation and Scroll Parity` to include `e`/`o` editor dispatch, `[`/`]` bracket navigation delegation, `f`/`Ctrl-p` file picker, `v` review toggle, `r` comment reply, and `?` help modal.
  - Updates `Requirement: Preview Status and Help Indicators` to document `[e] Edit | [[/]] Nav` in the status bar.

## Review Gate
- [x] **Gate 0 Decision**: Option A approved by human.
