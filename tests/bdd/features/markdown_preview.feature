Feature: Markdown Preview with Diff Highlighting
  As a code and RFC reviewer using lgtm
  I want an in-terminal, rendered markdown preview mode with inline git diff annotations
  So that I can review formatted documentation changes without distraction from raw markdown tokens

  # REQ-MDP-01: Toggle Markdown Preview on Markdown Files
  Scenario: Toggle markdown preview on a markdown file
    Given a git diff containing modified file "RFC.md"
    And the reviewer is viewing the diff for "RFC.md" in "Diff" view
    When the reviewer presses the "m" key
    Then the active view changes to "MarkdownPreview"
    And the status bar displays the "[MARKDOWN PREVIEW]" mode badge
    When the reviewer presses the "m" key again
    Then the active view returns to "Diff"

  # REQ-MDP-01: Toggle Command Bar Shortcuts
  Scenario: Toggle markdown preview via colon commands
    Given the reviewer is viewing the diff for "README.md"
    When the reviewer enters command ":md"
    Then the active view changes to "MarkdownPreview"
    When the reviewer enters command ":preview"
    Then the active view returns to "Diff"

  # REQ-MDP-01: Guard Non-Markdown Files
  Scenario: Attempting to toggle preview on a non-markdown file
    Given a git diff containing modified file "src/lib.rs"
    And the reviewer is viewing the diff for "src/lib.rs"
    When the reviewer presses the "m" key
    Then the active view remains "Diff"
    And the status bar displays warning "Markdown preview only available for .md files"

  # REQ-MDP-02 & REQ-MDP-03: Render Markdown Structure with Diff Highlights
  Scenario: Render formatted markdown with additions, deletions, and context
    Given a modified file "spec.md" with git diff:
      """
      @@ -1,5 +1,5 @@
      -# Old Architecture Title
      +# New Architecture Title

       A refactored paragraph describing the pipeline.
      +* Added bullet point item 1
      +* Added bullet point item 2
      -```rust
      -fn old_fn() {}
      -```
      +```rust
      +fn new_fn() {}
      +```
      """
    When markdown preview is rendered
    Then the deleted heading "-# Old Architecture Title" is rendered in red with "-" gutter indicator
    And the added heading "+# New Architecture Title" is rendered as Level 1 Heading (Bold Cyan) in green with "+" gutter indicator
    And the added bullet items are rendered with bullet glyphs in green with "+" gutter indicator
    And the deleted code fence lines are rendered in red with "-" gutter indicator
    And the added code fence is rendered with contrast background and language tag "[rust]" in green with "+" gutter indicator
    And the context paragraph is reflowed to terminal width with neutral styling and " " gutter indicator

  # REQ-MDP-04: Viewport Scrolling and Bounds Clamping
  Scenario: Scrolling through rendered markdown document
    Given a rendered markdown document with 150 rows in a 40-row viewport
    When the reviewer presses "j" 10 times
    Then the preview scroll offset is 10
    When the reviewer presses "Ctrl-d"
    Then the preview scroll offset advances by half the viewport height to 30
    When the reviewer presses "G"
    Then the preview scroll offset is clamped to the maximum scroll position 110
    When the reviewer presses "j"
    Then the preview scroll offset remains 110
    When the reviewer presses "g"
    Then the preview scroll offset returns to 0

  # REQ-MDP-04: State Synchronization between Diff and Preview
  Scenario: Returning to diff view maintains position
    Given the reviewer is at line 45 in "README.md" preview
    When the reviewer presses "Esc"
    Then the active view returns to "Diff"
    And the cursor is positioned at the corresponding source line in the diff view

  # REQ-MDP-05: Status Bar and Help Shortcuts
  Scenario: Status bar and key hints in preview mode
    Given the reviewer is in "MarkdownPreview" mode
    Then the header shows the filename with diff stats
    And the status bar displays "[MARKDOWN PREVIEW]"
    And the shortcut bar displays "m: Raw Diff | j/k: Scroll | g/G: Top/End | q: Quit"

  # EDG-MDP-01: Empty Markdown File
  Scenario: Previewing an empty markdown document
    Given a modified file "empty.md" with 0 bytes
    When the reviewer switches to markdown preview
    Then the viewport renders a placeholder "(Empty markdown document)"
    And no panic occurs

  # EDG-MDP-02: Pure Deletion File
  Scenario: Previewing a deleted markdown document
    Given a deleted file "deprecated.md" with 20 deleted lines
    When markdown preview is rendered
    Then all 20 lines are rendered with red text tint and "-" gutter indicators

  # EDG-MDP-04 & EDG-MDP-05: Long Unbroken Words and Wide Unicode Characters
  Scenario: Long strings and CJK wide characters
    Given a markdown paragraph with an unbroken 120-character URL and CJK characters "日本語"
    When rendered in an 80-column viewport
    Then the unbroken URL wraps safely at column 80 without panic
    And visual display widths for CJK characters are calculated correctly without misaligning the gutter

  # NFR-MDP-01 & NFR-MDP-03: Performance Invariants
  Scenario: Viewport frame rendering and zero-allocation scrolling
    Given a 5,000-line rendered markdown document
    When scrolling down with "j"
    Then the frame render executes in less than 16 milliseconds
    And zero full-document re-parses occur

  # REQ-MDPERF-1; Rust test: navigation_heights_match_reference_and_reuse_buffers
  Scenario: Navigate populated uniform row heights
    Given 150 cached markdown rows with a prepared height of 1 for every row
    And a viewport height of 40
    When the reviewer scrolls down and up and moves the cursor to both document ends
    Then cursor and scroll bounds match the reference row-height calculation
    And viewport bounds use constant-time arithmetic
    And cached rows, source line numbers, and span buffers remain unchanged

  # REQ-MDPERF-1; Rust test: navigation_heights_match_reference_and_reuse_buffers
  Scenario: Navigate directive and reply expansions
    Given cached markdown rows with heights 1, 3, 1, 4, 1, and 2
    And the extra height represents directives and their replies
    When the reviewer scrolls and moves the cursor through each row in a 5-line viewport
    Then cursor visibility and maximum scroll match the reference full-row fit calculation
    And navigation does not repeatedly scan overlapping row ranges
    And cached rows and span buffers are reused without reparsing

  # REQ-MDPERF-1; Rust test: navigation_heights_match_reference_and_reuse_buffers
  Scenario: Navigate a row taller than the viewport
    Given cached markdown rows with heights 1, 8, and 1
    And a viewport height of 3
    When the reviewer navigates to and away from the expanded row
    Then the first row at the viewport offset remains selectable even when it cannot fit
    And subsequent bounds preserve the reference full-row fit behavior
    And cursor and offset stay within the document

  # REQ-MDPERF-2; Rust tests: navigation_heights_match_reference_and_reuse_buffers, directives_render_under_their_line
  Scenario: Directive edits refresh cached navigation bounds
    Given cached markdown rows with prepared uniform heights
    When a directive and a reply expand a row
    And the renderer replaces the prepared heights
    Then subsequent navigation uses the expanded height
    When the directive and reply are removed and heights are replaced again
    Then navigation returns to the uniform bounds without stale prefix data

  # REQ-MDPERF-2; Rust test: directives_render_under_their_line
  Scenario: Width changes and document rebuilds replace navigation metadata
    Given a preview with expanded row metadata and a cursor near the document end
    When the preview is rebuilt for a narrower width or a different document
    Then navigation metadata describes the current rendered rows
    And cursor and offset are valid for the rebuilt document
    And navigation never uses heights from the previous document

  # REQ-MDPERF-2; Rust test: navigation_empty_and_zero_viewport_are_bounded
  Scenario Outline: Navigate empty rows and zero-height viewports safely
    Given a cached preview with <rows> rows
    And a viewport height of <height>
    When the reviewer scrolls in both directions and moves to both document ends
    Then no panic occurs
    And cursor and offset are zero for an empty document
    And cursor and offset never exceed the final row for a nonempty document

    Examples:
      | rows | height |
      | 0    | 0      |
      | 0    | 40     |
      | 5    | 0      |

  # REQ-MDPERF-1/2; Rust test: navigation_heights_match_reference_and_reuse_buffers
  Scenario: Missing row heights retain the one-line fallback
    Given cached markdown rows whose height metadata is empty or shorter than the document
    When navigation metadata is prepared and the reviewer scrolls
    Then missing row heights count as one screen line
    And cursor and scroll bounds match the reference calculation

  # REQ-MDCOV-1, NFR-MDCOV-1; Rust tests: inline_*, wrap_text_exact_boundaries, heading_styles_are_distinct_per_level
  Scenario: Parser edge cases render predictably
    Given markdown with unterminated inline markers, an unclosed fence, an empty list item and an overlong word
    When the preview is built at a narrow width
    Then unterminated markers stay literal and no row panics
    And wrapped rows keep the source line number on the first row only
