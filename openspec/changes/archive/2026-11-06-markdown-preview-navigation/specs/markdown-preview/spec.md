# Spec Delta: markdown-preview

## MODIFIED Requirements

### Requirement: Preview Navigation and Scroll Parity
In preview mode a cursor row SHALL be shown and highlighted. `j`/`Down` and `k`/`Up` SHALL move it by one row, `Ctrl-d`/`PageDown` and `Ctrl-u`/`PageUp` by one page, and `g`/`Home` and `G`/`End` to the first and last row. Pressing `e` or `o` SHALL synchronize the diff cursor to the preview cursor row, set active view to `Diff`, and launch the external editor. Pressing `[` or `]` SHALL synchronize the diff cursor to the preview cursor row, set active view to `Diff`, and delegate to standard diff bracket navigation. Pressing `f` or `Ctrl-p` SHALL open the file picker. Pressing `v` SHALL toggle review on the current file. Pressing `r` SHALL reply to an active comment on the cursor row or open comment input. Pressing `?` SHALL open the help modal. The viewport MUST follow so the cursor stays visible, and the scroll offset MUST clamp to `0..max(0, total_lines - viewport_height)`. Toggling between preview and diff MUST map the cursor to the corresponding source line.

#### Scenario: Editor launch from preview
- **WHEN** the reviewer views a markdown file in preview mode and presses `e`
- **THEN** the diff cursor is synchronized to the source line of the preview cursor row
- **AND** the active view switches to `Diff`
- **AND** the external editor launches for that file and line

#### Scenario: Bracket navigation delegation from preview
- **WHEN** the reviewer presses `]` in preview mode
- **THEN** the diff cursor is synchronized to the source line of the preview cursor row
- **AND** the active view switches to `Diff`
- **AND** leader `]` is activated with navigation status hints

#### Scenario: Interactive shortcuts in preview
- **WHEN** the reviewer presses `f` in preview mode
- **THEN** the file picker modal opens
- **WHEN** the reviewer presses `v` in preview mode
- **THEN** the current file's reviewed status toggles

#### Scenario: Cursor movement scrolls the viewport
- **WHEN** the document has 100 rendered rows in a 30-row viewport and `j` is pressed 35 times
- **THEN** the cursor is on row 35 and remains visible
- **AND** the scroll offset is at most 35 and no less than 6

#### Scenario: Scrolling and clamping
- **WHEN** `G` is pressed
- **THEN** the cursor is on the last row and the offset is 70
- **WHEN** `j` is pressed
- **THEN** the cursor and offset are unchanged
- **WHEN** `g` is pressed
- **THEN** the cursor and offset are 0

### Requirement: Preview Status and Help Indicators
In preview mode the status bar SHALL show a `[MARKDOWN PREVIEW]` badge and the help bar SHALL show shortcuts including `[m/Esc] Raw Diff`, `[j/k] Move`, `[e] Edit`, `[[/]] Nav`, `[f] Files`, `[v] Review`, `[c] Comment`, and `[q] Quit`.

#### Scenario: Badge shown
- **WHEN** preview mode is active
- **THEN** the status bar shows `[MARKDOWN PREVIEW]` and the help bar lists `[m/Esc]`, `[j/k]`, `[e]`, `[[/]]`, `[f]`, `[v]`, `[c]`, `[q]`
