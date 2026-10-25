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
