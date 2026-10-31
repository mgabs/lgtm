## Context

Preview state (`MarkdownPreviewState`) holds `scroll_offset`, `rendered_rows` and `row_heights`; directive commands call `sync_cursor_to_markdown_top`, which maps the first visible row's `line_number` to a diff cursor. No mouse events are handled anywhere (`event_loop.rs` matches only key/resize/focus events).

## Goals / Non-Goals

**Goals:** a visible cursor row driving comment commands; wheel scrolling in preview.
**Non-goals:** clicks, selection, wheel in other views, a global mouse mode.

## Decisions

- Add `cursor_row: usize` to `MarkdownPreviewState`. Key handlers move it and call a `ensure_cursor_visible` that adjusts `scroll_offset` using `row_heights`. Replace `sync_cursor_to_markdown_top` with a version keyed on `cursor_row` (same line-matching logic).
- Enable crossterm mouse capture only while `ActiveView::MarkdownPreview` is active (on toggle in, off on toggle out/quit) rather than globally. Rejected: global capture, which breaks native text selection in every view.
- Handle `MouseEventKind::ScrollUp/Down` in `event_loop.rs`, forwarding to an `App::handle_mouse` that ignores events outside preview. Wheel moves the viewport by 3 and clamps the cursor into view.
- Highlight the cursor row in `ui/markdown.rs` with a reversed/bg style; keep diff-kind colors.

## Risks / Trade-offs

- Mouse capture on Windows terminals or multiplexers (tmux, zellij) may behave differently → guard enable/disable errors without failing, and cover with a unit test of `handle_mouse` rather than a terminal test.
- `j`/`k` now move a cursor instead of scrolling; behavior at viewport edges stays equivalent, so existing scroll tests need updating.
- Frame budget (< 16ms) must hold with the extra highlight; cursor styling is O(visible rows).
- Assumption: wheel step of 3 rows.
