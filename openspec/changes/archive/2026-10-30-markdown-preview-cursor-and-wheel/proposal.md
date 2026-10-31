## Why

In the Markdown preview, `c`/`l`/`R`/`x` act on the first visible line, so a reviewer cannot pick the line they are reading. The app never enables mouse capture (observed: no mouse handling in `src/`), so the wheel does nothing in preview.

## What Changes

- Add a cursor row to the preview: `j`/`k` move it, the viewport follows, and it is highlighted. `c`, `l`, `R` and `x` target the cursor row's source line instead of the top visible line.
- Page, top and bottom keys move the cursor as well as the viewport.
- Mouse wheel scrolls the preview viewport. Mouse capture is enabled only while the preview is active and released on leaving it, so terminal text selection works elsewhere.
- Update help/status text and docs/WIKI.md.

Non-goals: mouse clicks or selection, wheel support in other views, changes to diff rendering. Affected crate: `lgtm` only.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `markdown-preview`: navigation, directive target and help text requirements change.

## Impact

- `src/app/markdown.rs` (cursor state), `src/app/input.rs` (keys, mouse), `src/event_loop.rs` and `src/main.rs` (mouse events/capture), `src/ui/markdown.rs` (highlight), `docs/WIKI.md`, `CHANGELOG.md`.
- Must work on ubuntu, macos and windows crossterm backends.
