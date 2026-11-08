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

### Requirement: REQ-MDPERF-1 Efficient Cached Navigation
Uniform-height preview scrolling MUST use constant-time viewport bounds after layout metadata is prepared, including populated uniform heights. Directive-expanded navigation MUST avoid repeated overlapping range scans. Both paths MUST reuse rendered line buffers and preserve cursor visibility, scroll clamping, source attribution, and directive/reply row accounting.

#### Scenario: Uniform rendered rows
- **WHEN** a preview with populated one-line row heights scrolls
- **THEN** viewport bounds are calculated without scanning viewport rows
- **AND** cursor and offset match the existing navigation semantics

#### Scenario: Directive-expanded rows
- **WHEN** a preview containing directive and reply rows moves its cursor or scrolls
- **THEN** expanded heights determine visibility and clamping without reparsing or allocating new rendered line buffers

### Requirement: REQ-MDPERF-2 Coherent Navigation Metadata
Cached navigation metadata MUST reflect current rendered rows and directive heights after document rebuilds, width changes, and directive/reply edits. Empty documents and zero-height viewports MUST NOT panic or permit invalid cursor or offset state.

#### Scenario: Directive height changes
- **WHEN** a directive or reply changes the height of a rendered row
- **THEN** subsequent navigation uses the updated height rather than stale cached bounds

#### Scenario: Reflow and empty viewport
- **WHEN** preview rows are rebuilt at a new width or the viewport has zero height
- **THEN** cursor and offset remain valid for the current rows without panic

### Requirement: NFR-MDPERF-1 Optimized Preview Budgets
Optimized, non-instrumented measurements MUST enforce scrolling below 1 microsecond per operation, parsing/wrapping 5,000 lines below 50 ms, and cached rendering below 16 ms. Scrolling MUST NOT reparse or allocate new line buffers. Existing rendered-memory limits MUST remain unchanged; representative uniform and directive-expanded workloads MUST retain these guarantees.

#### Scenario: Representative scrolling budget
- **WHEN** repeated warmed-up optimized samples measure actual scrolling on uniform and directive-expanded previews
- **THEN** measured per-operation latency satisfies the 1-microsecond budget under the reported sample policy
- **AND** navigation reuses parsed rows and line buffers

#### Scenario: Parsing and rendering budgets
- **WHEN** optimized checks measure a 5,000-line preview and its cached rendering
- **THEN** parsing/wrapping takes less than 50 ms and cached rendering takes less than 16 ms

### Requirement: Mermaid Blocks
Fenced `mermaid` blocks SHALL show a header `[mermaid: <type>]` taken from the first word of the first non-empty line. In `graph` and `flowchart` blocks each edge MUST render as `<from> ──▶ <to>` with its label and node labels. Anything unparseable, and other diagram types, MUST render verbatim. Rows MUST keep their source line number and diff overlay.

#### Scenario: Flowchart edges
- **WHEN** a block contains `flowchart TD`, `A[Start] --> B{Ok?}` and `B -->|yes| C[Done]`
- **THEN** the header reads `[mermaid: flowchart]` and rows read `Start ──▶ Ok?` and `B ──▶ Done (yes)`

#### Scenario: Unsupported diagram type
- **WHEN** a block starts with `sequenceDiagram`
- **THEN** the header reads `[mermaid: sequenceDiagram]` and the body renders verbatim as code

#### Scenario: Unparseable line
- **WHEN** a flowchart block contains `style A fill:#f9f`
- **THEN** that line renders verbatim and the other edges still render

#### Scenario: Changed edge
- **WHEN** a diagram edge line is added in the diff
- **THEN** its rendered row is green with a `+` marker and that line's number

### Requirement: REQ-MDCOV-1 Parser Regression Coverage
The Markdown parser tests MUST exercise headings, rules, block quotes, ordered and unordered lists, fenced code, Mermaid fences, inline emphasis and code, and width wrapping, asserting rendered text, styling and source-line attribution rather than only that rendering does not panic.

#### Scenario: Structure variants
- **WHEN** documents mixing each construct above are rendered at narrow and wide widths
- **THEN** each row's text, style and source line number match the expected values

#### Scenario: Malformed and edge input
- **WHEN** input has an unclosed fence, empty emphasis markers, an empty list item or a line wider than the viewport
- **THEN** the parser renders it without panic and with the documented verbatim or wrapped output

### Requirement: NFR-MDCOV-1 Parser Coverage Bar
`src/app/markdown.rs` MUST measure at least 95% line coverage and at least 95% branch coverage using `cargo +nightly llvm-cov -p lgtm --bin lgtm --branch`, counting each source branch location once and never excluding real outcomes. Scoped parser mutation testing MUST catch every non-equivalent mutant.

#### Scenario: Coverage measured
- **WHEN** the documented command runs on the final tree
- **THEN** the report shows at least 95% line and at least 95% branch coverage for `src/app/markdown.rs`

#### Scenario: Mutation survivors
- **WHEN** scoped mutation testing runs on the parser
- **THEN** no non-equivalent mutant survives, and each equivalent mutant is listed with its reason

### Requirement: Preview Live Refresh and Cache Invalidation
When preview mode is active, incoming background diff updates SHALL apply immediately to the diff snapshot and file drawer. The rendered markdown preview MUST invalidate its cached lines and re-render against the updated file content, clamping the cursor row and scroll offset to the new valid row range.

#### Scenario: Background file modification during preview
- **WHEN** the reviewer views `README.md` in markdown preview
- **AND** external edits to `README.md` or other workspace files arrive via the file watcher
- **THEN** the local diff snapshot applies without exiting markdown preview
- **AND** the file drawer reflects updated file change statuses
- **AND** the preview re-renders updated markdown lines while keeping the cursor in view

#### Scenario: Background file deletion during preview
- **WHEN** the active markdown file is deleted on disk while preview is open
- **THEN** the preview updates to render deletions with red `-` gutter markers
- **AND** cursor position clamps to the new document boundary without panic
