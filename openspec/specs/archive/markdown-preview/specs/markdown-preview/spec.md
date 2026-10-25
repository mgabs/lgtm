# Specification: Markdown Preview with Diff Highlighting

## Capability: `markdown-preview`

### 1. Requirements

- **REQ-MDP-01 (Toggle Markdown Preview)**:
  - When the active file in the diff view has a Markdown extension (`.md`, `.markdown`, `.mdown`, `.mkd`), pressing `m` MUST toggle between the standard diff view and rendered markdown preview mode.
  - The preview mode MUST also be toggleable via `:` command bar commands `:md` and `:preview`.
  - When viewing a non-markdown file, pressing `m` MUST NOT activate preview mode and MUST display a transient warning in the status bar (e.g. `Markdown preview only available for .md files`).
  - Pressing `Esc` or `m` while in Markdown Preview mode MUST return to the standard diff view.

- **REQ-MDP-02 (Markdown Terminal Reflow & Styling)**:
  - The preview MUST parse CommonMark/GFM syntax and render formatted Ratatui `Line` and `Span` primitives reflowed to the available viewport width.
  - **Headings**:
    - Level 1 (`#`): Bold Cyan, underlined or accented.
    - Level 2 (`##`): Bold Cyan.
    - Level 3 (`###`): Bold Blue.
    - Level 4-6 (`####`..`######`): Bold Light Gray / Dim Cyan.
  - **Paragraphs**: Text reflowed cleanly to viewport width with word-boundary wrapping.
  - **Lists**:
    - Unordered lists: Formatted with bullet glyphs (`•`, `-`) and indented by list nesting level (2 spaces per depth).
    - Ordered lists: Formatted with numeric prefixes (`1.`, `2.`) and aligned indentation.
  - **Blockquotes**: Left-gutter vertical rule (`│`) in dimmed color, with italicized or dimmed blockquote text.
  - **Fenced Code Blocks**:
    - Distinct visual contrast background (e.g. Dark Gray).
    - Optional header tag displaying code language identifier (e.g. `[rust]`).
    - Verbatim indentation and whitespace preservation without word-wrapping.
  - **Inline Spans**:
    - Bold (`**text**`): Bold text attribute.
    - Italic (`*text*`): Italic text attribute.
    - Inline code (`` `code` ``): Contrasting background and monospace text.
    - Links (`[text](url)`): Underlined text with subtle hint styling.

- **REQ-MDP-03 (Diff Overlay & Gutter Attribution)**:
  - Markdown preview MUST overlay git diff changes from `lgtm-core::diff::FileDiff`:
    - Added lines/blocks: Green foreground tint, subtle green background tint, and `+` gutter indicator.
    - Deleted lines/blocks: Red foreground tint, strikethrough or subtle red background tint, and `-` gutter indicator.
    - Unmodified context: Standard text styling and blank `' '` gutter indicator.
  - Gutter MUST display line numbers (old and/or new line numbers) aligned with diff markers.

- **REQ-MDP-04 (Navigation & Scroll Parity)**:
  - Navigation controls in preview mode MUST support:
    - Down line: `j`, `Down`
    - Up line: `k`, `Up`
    - Page Down: `Ctrl-d`, `PageDown`
    - Page Up: `Ctrl-u`, `PageUp`
    - Jump to Top: `g`, `Home`
    - Jump to Bottom: `G`, `End`
  - Scroll offset MUST clamp between `0` and `max(0, total_rendered_lines - viewport_height)`.
  - When toggling back from Preview to Diff view (or vice versa), the active cursor/scroll line MUST map to the corresponding source line to preserve reading context.

- **REQ-MDP-05 (Status Bar & Mode Indicators)**:
  - While in Markdown Preview mode, the status bar MUST display an active badge: `[MARKDOWN PREVIEW]`.
  - Help bar at the bottom MUST display preview shortcuts: `m: Raw Diff | j/k: Scroll | g/G: Top/End | q: Quit`.

---

### 2. Non-Functional Requirements & Performance Budgets

- **NFR-MDP-01 (Frame Render Budget)**:
  - Viewport rendering of cached preview lines into Ratatui buffer MUST complete in $< 16\text{ ms}$ (60 FPS target).
- **NFR-MDP-02 (Parsing & Reflow Budget)**:
  - Document parsing and line-wrapping for up to 5,000 lines of markdown MUST complete in $< 50\text{ ms}$.
- **NFR-MDP-03 (Zero-Allocation Navigation)**:
  - Scrolling through pre-rendered lines (`j`/`k`) MUST NOT re-parse markdown AST or allocate new line buffers; slicing viewport rows MUST be $O(1)$ relative to total lines.
- **NFR-MDP-04 (Memory Footprint)**:
  - Rendered line structures MUST NOT exceed $10\times$ the raw text size of the markdown file.

---

### 3. Assumptions

- **ASM-MDP-01**: Document parsing uses `pulldown-cmark` crate for standards-compliant CommonMark parsing.
- **ASM-MDP-02**: Diff information is extracted from `lgtm-core::diff::FileDiff`.
- **ASM-MDP-03**: Unicode characters and emoji visual widths are calculated accurately via `unicode-width`.
- **ASM-MDP-04**: Minimum supported terminal viewport width for preview is 20 columns. Narrower viewports display a graceful fallback notice.

---

### 4. Edge Cases & Errors

- **EDG-MDP-01 (Empty Markdown File)**:
  - An empty `.md` file renders a placeholder message: `(Empty markdown document)` without panicking.
- **EDG-MDP-02 (Pure Deletion / Deleted File)**:
  - If a file is completely deleted, the preview renders all deleted lines with red `-` gutter indicators.
- **EDG-MDP-03 (Unclosed Code Fences & Tokens)**:
  - Unclosed code fences or bold/italic markers at EOF MUST gracefully terminate without parser error or infinite loop.
- **EDG-MDP-04 (Long Unbroken Words)**:
  - Words or string literals exceeding viewport width wrap at terminal boundary to prevent line clipping or crash.
- **EDG-MDP-05 (Terminal Resize)**:
  - Resizing terminal recalculates line wrapping for the new width while preserving the relative scroll position.

---

### 5. Behavioral Scenarios (Given / When / Then)

#### Scenario 5.1: Toggle Preview on Markdown File
- **Given** reviewer is viewing diff for `README.md`
- **When** key `m` is pressed
- **Then** view mode changes to `MarkdownPreview`
- **And** the status bar displays `[MARKDOWN PREVIEW]`
- **When** key `m` is pressed again
- **Then** view mode returns to standard `Diff` view

#### Scenario 5.2: Toggle Attempt on Non-Markdown File
- **Given** reviewer is viewing diff for `src/main.rs`
- **When** key `m` is pressed
- **Then** view mode remains `Diff`
- **And** a warning `Markdown preview only available for .md files` is displayed in status bar

#### Scenario 5.3: Render Additions and Deletions
- **Given** `docs/plan.md` contains 3 added lines in a bulleted list and 1 deleted heading
- **When** markdown preview is rendered
- **Then** the deleted heading has red text tint and `-` gutter marker
- **And** the 3 added bullet items have green text tint and `+` gutter markers
- **And** surrounding unchanged text has neutral styling and `' '` gutter markers

#### Scenario 5.4: Scrolling Navigation & Clamping
- **Given** rendered markdown document has 100 lines and viewport height is 30
- **When** key `j` is pressed 10 times
- **Then** preview scroll offset is 10
- **When** key `G` is pressed
- **Then** preview scroll offset is 70 (`100 - 30`)
- **When** key `j` is pressed
- **Then** preview scroll offset remains 70 (clamped at bottom)
- **When** key `g` is pressed
- **Then** preview scroll offset is 0 (top)

#### Scenario 5.5: Viewport Resize Reflow
- **Given** preview viewport width is 80 columns with wrapped paragraph occupying 4 lines
- **When** terminal width expands to 120 columns
- **Then** paragraph is reflowed into fewer lines
- **And** scroll offset is clamped to new valid boundary without panic
