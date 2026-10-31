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
In preview mode, `j`/`Down` and `k`/`Up` SHALL scroll by line, `Ctrl-d`/`PageDown` and `Ctrl-u`/`PageUp` by page, and `g`/`Home` and `G`/`End` jump to top and bottom. The scroll offset MUST clamp to `0..max(0, total_lines - viewport_height)`. Toggling between preview and diff MUST map the cursor to the corresponding source line.

#### Scenario: Scrolling and clamping
- **WHEN** the rendered document has 100 lines in a 30-line viewport and `j` is pressed 10 times
- **THEN** the offset is 10
- **WHEN** `G` is pressed
- **THEN** the offset is 70
- **WHEN** `j` is pressed
- **THEN** the offset remains 70
- **WHEN** `g` is pressed
- **THEN** the offset is 0

### Requirement: Preview Status and Help Indicators
In preview mode the status bar SHALL show a `[MARKDOWN PREVIEW]` badge and the help bar SHALL show `m: Raw Diff | j/k: Scroll | g/G: Top/End | q: Quit`.

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
