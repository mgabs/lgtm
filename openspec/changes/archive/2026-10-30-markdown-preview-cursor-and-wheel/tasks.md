## 1. Cursor row (test-first)

- [x] 1.1 Add failing tests: cursor moves with j/k/page/g/G, viewport follows, clamping, `c` targets the cursor line, no-source-line row opens no modal
- [x] 1.2 Add `cursor_row` and `ensure_cursor_visible` to `MarkdownPreviewState` in `src/app/markdown.rs`
- [x] 1.3 Update `handle_markdown_preview_key` in `src/app/input.rs` to move the cursor; key `c`/`l`/`R`/`x` off `cursor_row`
- [x] 1.4 Highlight the cursor row in `src/ui/markdown.rs`
- [x] 1.5 Update existing scroll tests that assumed viewport-only scrolling

## 2. Mouse wheel (test-first)

- [x] 2.1 Add failing tests for `App::handle_mouse`: wheel scrolls 3 rows, clamps, ignored outside preview
- [x] 2.2 Handle mouse events in `src/event_loop.rs` and add `App::handle_mouse`
- [x] 2.3 Enable mouse capture on entering preview and disable on leaving and on quit (`src/main.rs` / toggle path), tolerating errors

## 3. Docs and verification

- [x] 3.1 Update the help bar text and the `m` row in `docs/WIKI.md`; add a CHANGELOG entry
- [x] 3.2 Run `cargo fmt`, `cargo clippy --workspace --all-targets`, `cargo test --workspace`
- [x] 3.3 Run `openspec validate --all --strict`
