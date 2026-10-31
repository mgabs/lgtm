# markdown-preview Specification

## Purpose

Rendered Markdown preview of changed `.md` files inside the diff view, with git additions and deletions overlaid, so reviewers can read documentation changes as prose.

## Requirements

### Requirement: Toggle Markdown Preview
For files with a Markdown extension (`.md`, `.markdown`, `.mdown`, `.mkd`), `m`, `:md` and `:preview` SHALL toggle between the diff view and the preview. `Esc` or `m` in preview MUST return to the diff view. For other files, `m` MUST NOT activate preview and MUST show a transient status warning.

#### Scenario: Toggle on a Markdown file
- **WHEN** the reviewer views `README.md` and presses `m`
- **THEN** the view mode is `MarkdownPreview` and the status bar shows `[MARKDOWN PREVIEW]`
- **WHEN** `m` is pressed again
- **THEN** the view returns to the standard Diff view

#### Scenario: Toggle on a non-Markdown file
- **WHEN** the reviewer views `src/main.rs` and presses `m`
- **THEN** the view mode remains Diff
- **AND** the status bar shows `Markdown preview only available for .md files`

### Requirement: Markdown Reflow and Styling
The preview SHALL parse CommonMark/GFM and render it reflowed to the viewport width with word-boundary wrapping. Headings, lists (bulleted and numbered, 2-space nesting), blockquotes (dimmed `│` gutter), fenced code blocks (contrasting background, optional language tag, verbatim whitespace, no wrapping), bold, italic, inline code and links MUST each have distinct styling.

#### Scenario: Styled elements
- **WHEN** a document contains headings, a nested list, a blockquote and a fenced `rust` block
- **THEN** headings are bold and colored by level, list items are indented per depth, the quote has a `│` gutter, and the code block shows `[rust]` without wrapping

#### Scenario: Viewport reflow
- **WHEN** the viewport width expands from 80 to 120 columns
- **THEN** wrapped paragraphs reflow into fewer lines
- **AND** the scroll offset is clamped to the new valid range without panic

### Requirement: Diff Overlay and Gutter Attribution
The preview SHALL overlay the file's git diff: added lines green with a `+` gutter marker, deleted lines red with a `-` marker, unchanged lines neutral with a blank marker. The gutter MUST show old and/or new line numbers aligned with the markers.

#### Scenario: Additions and deletions
- **WHEN** `docs/plan.md` has 3 added bullet items and 1 deleted heading
- **THEN** the heading is red with a `-` marker, the 3 bullets are green with `+` markers, and unchanged text is neutral with blank markers

#### Scenario: Deleted file
- **WHEN** the whole file was deleted
- **THEN** every line renders red with a `-` marker

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

### Requirement: Preview Edge Cases
The preview SHALL NOT panic or hang on edge input. An empty file MUST render `(Empty markdown document)`. Unclosed code fences or emphasis markers at EOF MUST terminate gracefully. Words longer than the viewport MUST wrap at the terminal boundary. Viewports narrower than 20 columns MUST show a graceful fallback notice.

#### Scenario: Empty document
- **WHEN** an empty `.md` file is previewed
- **THEN** `(Empty markdown document)` is shown

#### Scenario: Unclosed fence
- **WHEN** a document ends inside an unclosed code fence
- **THEN** rendering completes without error or infinite loop

### Requirement: Preview Performance Budgets
Rendering cached preview lines to the buffer MUST complete in under 16 ms. Parsing and wrapping up to 5,000 lines MUST complete in under 50 ms. Scrolling MUST NOT re-parse the document or allocate new line buffers (viewport slicing is O(1)). Rendered lines MUST NOT exceed 10x the raw text size in memory.

#### Scenario: Scrolling reuses rendered lines
- **WHEN** the user scrolls with `j`/`k`
- **THEN** the markdown AST is not re-parsed and no new line buffers are allocated

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
