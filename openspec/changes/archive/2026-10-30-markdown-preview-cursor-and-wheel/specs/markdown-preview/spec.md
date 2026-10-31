## MODIFIED Requirements

### Requirement: Preview Navigation and Scroll Parity
In preview mode a cursor row SHALL be shown and highlighted. `j`/`Down` and `k`/`Up` SHALL move it by one row, `Ctrl-d`/`PageDown` and `Ctrl-u`/`PageUp` by one page, and `g`/`Home` and `G`/`End` to the first and last row. The viewport MUST follow so the cursor stays visible, and the scroll offset MUST clamp to `0..max(0, total_lines - viewport_height)`. Toggling between preview and diff MUST map the cursor to the corresponding source line.

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
In preview mode the status bar SHALL show a `[MARKDOWN PREVIEW]` badge and the help bar SHALL show `m: Raw Diff | j/k: Move | c: Comment | g/G: Top/End | q: Quit`.

#### Scenario: Badge shown
- **WHEN** preview mode is active
- **THEN** the status bar shows `[MARKDOWN PREVIEW]` and the help bar lists the preview shortcuts

## ADDED Requirements

### Requirement: Preview Directive Target
In preview mode, `c`, `l`, `R` and `x` SHALL act on the source line of the cursor row. When the cursor row has no source line, the command MUST do nothing and MUST NOT open a modal.

#### Scenario: Comment on the cursor line
- **WHEN** the cursor is on the row for line 42 of `README.md`, scrolled mid-viewport, and `c` is pressed
- **THEN** the comment input opens targeting line 42

#### Scenario: Row without a source line
- **WHEN** the cursor is on a row with no source line and `c` is pressed
- **THEN** no modal opens

### Requirement: Preview Mouse Wheel
While the preview is active, the mouse wheel SHALL scroll the viewport by 3 rows per notch, clamped to the valid range, and the cursor MUST stay within the visible rows. Mouse capture MUST be enabled on entering the preview and disabled on leaving it or quitting.

#### Scenario: Wheel scrolling
- **WHEN** the preview is active at offset 0 with 100 rows in a 30-row viewport and the wheel scrolls down one notch
- **THEN** the offset is 3 and the cursor is on a visible row
- **WHEN** the wheel scrolls up past the top
- **THEN** the offset is 0

#### Scenario: Capture released
- **WHEN** the reviewer presses `m` to return to the diff view
- **THEN** mouse capture is disabled
