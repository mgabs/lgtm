# Spec Delta: markdown-preview

## MODIFIED Requirements

### Requirement: Preview Navigation and Scroll Parity
In preview mode a cursor row SHALL be shown and highlighted. `j`/`Down` and `k`/`Up` SHALL move it by one row, `Ctrl-d`/`PageDown` and `Ctrl-u`/`PageUp` by one page, and `g`/`Home` and `G`/`End` to the first and last row. Pressing `e` or `o` SHALL synchronize the diff cursor to the preview cursor row, set active view to `Diff`, and launch the external editor. Pressing `[` or `]` SHALL synchronize the diff cursor to the preview cursor row, set active view to `Diff`, and delegate to standard diff bracket navigation. Pressing `f` or `Ctrl-p` SHALL open the file picker. Pressing `v` SHALL toggle review on the current file. Pressing `r` SHALL synchronize the diff cursor to the preview row's source line, then reply to an active PR thread or directive on that line, and returning from reply input MUST restore `MarkdownPreview` view. Pressing `Enter` SHALL resolve the source line (scanning backward to the block start if on a wrapped continuation line) and deliver `#ref <file>:<line>` to the agent sink. Pressing `t` SHALL open the PR threads modal. Pressing `?` SHALL open the help modal. The viewport MUST follow so the cursor stays visible, and the scroll offset MUST clamp to `0..max(0, total_lines - viewport_height)`. Toggling between preview and diff MUST map the cursor to the corresponding source line.

#### Scenario: Agent reference injection on enter
- **WHEN** the reviewer is on line 12 of `RFC.md` in preview mode and presses `Enter`
- **THEN** a reference `#ref RFC.md:12` is sent to the active agent sink
- **AND** the status bar indicates delivery to the sink

#### Scenario: Agent reference injection on wrapped continuation line
- **WHEN** the reviewer is on a wrapped continuation line of paragraph starting at line 12 with no direct line number
- **THEN** pressing `Enter` resolves to line 12 and sends `#ref RFC.md:12` to the agent sink

#### Scenario: Open PR threads modal from preview
- **WHEN** the reviewer presses `t` in preview mode
- **THEN** the active view switches to `ThreadView` modal

#### Scenario: Reply to thread from preview
- **WHEN** a PR thread exists at the cursor row's source line and `r` is pressed
- **THEN** the diff cursor synchronizes to that line
- **AND** the command bar pre-fills `:thread reply <id> `
- **AND** canceling or completing the command returns to `MarkdownPreview`

#### Scenario: Reply to local directive from preview
- **WHEN** a local directive exists at the cursor row's source line and `r` is pressed
- **THEN** `ActiveView::CommentReply` opens targeting that directive
- **AND** pressing `Esc` or `Enter` returns the view to `MarkdownPreview`

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

### Requirement: Preview Directive Target
In preview mode, `c` SHALL activate leader template matching (`cp`, `cw`, `cs`, `cr`, `ca`, `cc`) targeting the source line of the cursor row (including wrapped continuation lines resolving to the start line). Saving or canceling comment input MUST return the view to `MarkdownPreview`. `l`, `R` and `x` SHALL act on the source line of the cursor row. When the cursor row has no source line, the command MUST do nothing and MUST NOT open a modal.

#### Scenario: Comment on the cursor line
- **WHEN** the cursor is on the row for line 42 of `README.md`, scrolled mid-viewport, and `c` is pressed
- **THEN** the comment input opens targeting line 42

#### Scenario: Comment leader template on preview row
- **WHEN** the reviewer presses `c` followed by `p` on line 42 in preview mode
- **THEN** comment input opens targeting line 42 with template `🚀 [PERF] Optimize hot-path allocations and database query batching.`
- **AND** canceling with `Esc` or saving with `Enter` returns to `MarkdownPreview`

#### Scenario: Comment on wrapped continuation line
- **WHEN** the cursor is on a continuation row of line 42 and `c` `c` is pressed
- **THEN** comment input opens targeting line 42 with empty text

#### Scenario: Row without a source line
- **WHEN** the cursor is on a row with no source line and `c` is pressed
- **THEN** no modal opens

## ADDED Requirements

### Requirement: REQ-MDP-THREADS Inline PR Thread Rendering
The rendered markdown preview SHALL query active and outdated PR discussion threads matching the file and line, and render them inline beneath the source row. Each thread MUST display a thread status icon, author, thread text, and subsequent replies, styled distinctly from local directives.

#### Scenario: Inline PR discussion thread
- **WHEN** a PR thread by `@reviewer` with one reply is anchored to line 20
- **THEN** the preview renders the thread under line 20 with `🧵 [@reviewer]` and its reply indented with `↳`

#### Scenario: Outdated PR thread indication
- **WHEN** an outdated PR thread is anchored to line 20
- **THEN** the thread header displays an outdated badge `[Outdated]`
