@command_line @line_editing
Feature: Command Line Vim/Readline Editing Shortcuts
  As an LGTM terminal reviewer
  I want standard Vim and Readline editing shortcuts in the command bar
  So that I can quickly fix typos, navigate cursor positions, and delete words or lines without repetitive backspacing

  Background:
    Given the application is running
    And the command line prompt is active

  @REQ-1 @movement
  Scenario: Cursor movement within input text
    Given the command input is "pr 12" with cursor at index 5
    When the user presses "Left" 2 times
    Then the cursor position is 3
    When the user presses "Ctrl-a"
    Then the cursor position is 0
    When the user presses "Right" 2 times
    Then the cursor position is 2
    When the user presses "Ctrl-e"
    Then the cursor position is 5

  @REQ-1 @clamping
  Scenario: Cursor clamping at boundaries
    Given the command input is "diff" with cursor at index 0
    When the user presses "Left"
    Then the cursor position remains 0
    When the user presses "End"
    Then the cursor position is 4
    When the user presses "Right"
    Then the cursor position remains 4

  @REQ-2 @insertion
  Scenario: Character insertion at cursor position
    Given the command input is "audi" with cursor at index 4
    When the user types "t"
    Then the command input is "audit"
    And the cursor position is 5
    When the user presses "Left" 3 times
    And the user types "x"
    Then the command input is "auxdit"
    And the cursor position is 3

  @REQ-3 @backspace
  Scenario: Backward character deletion
    Given the command input is "help" with cursor at index 2
    When the user presses "Backspace"
    Then the command input is "hlp"
    And the cursor position is 1

  @REQ-3 @backspace_exit
  Scenario: Backspace on empty input exits command mode
    Given the command input is "" with cursor at index 0
    When the user presses "Backspace"
    Then the active view switches back to "Diff"

  @REQ-3 @delete_forward
  Scenario: Forward character deletion under cursor
    Given the command input is "help" with cursor at index 1
    When the user presses "Delete"
    Then the command input is "hlp"
    And the cursor position remains 1
    When the user presses "Ctrl-d"
    Then the command input is "hp"
    And the cursor position remains 1

  @REQ-3 @delete_forward_boundary
  Scenario: Forward character deletion at end of input
    Given the command input is "exit" with cursor at index 4
    When the user presses "Delete"
    Then the command input remains "exit"
    And the cursor position remains 4

  @REQ-4 @word_deletion
  Scenario: Backward word deletion with Ctrl-w and Alt-Backspace
    Given the command input is "pr 12" with cursor at index 5
    When the user presses "Ctrl-w"
    Then the command input is "pr "
    And the cursor position is 3
    When the user presses "Alt-Backspace"
    Then the command input is ""
    And the cursor position is 0

  @REQ-4 @word_deletion_whitespace
  Scenario: Backward word deletion stripping trailing whitespace
    Given the command input is "audit   " with cursor at index 8
    When the user presses "Ctrl-w"
    Then the command input is ""
    And the cursor position is 0

  @REQ-4 @word_deletion_punctuation
  Scenario: Backward word deletion respecting punctuation token boundaries
    Given the command input is "foo-bar" with cursor at index 7
    When the user presses "Ctrl-w"
    Then the command input is "foo-"
    And the cursor position is 4
    When the user presses "Ctrl-w"
    Then the command input is "foo"
    And the cursor position is 3

  @REQ-5 @line_deletion_start
  Scenario: Deleting from cursor to start of line with Ctrl-u
    Given the command input is "diff main" with cursor at index 4
    When the user presses "Ctrl-u"
    Then the command input is " main"
    And the cursor position is 0

  @REQ-5 @line_deletion_clear
  Scenario: Clearing entire input with Ctrl-u from the end
    Given the command input is "scan repo" with cursor at index 9
    When the user presses "Ctrl-u"
    Then the command input is ""
    And the cursor position is 0

  @REQ-5 @line_deletion_end
  Scenario: Deleting from cursor to end of line with Ctrl-k
    Given the command input is "diff main" with cursor at index 4
    When the user presses "Ctrl-k"
    Then the command input is "diff"
    And the cursor position is 4

  @REQ-6 @autocomplete_sync
  Scenario: Cursor alignment on autocomplete cycling
    Given the command input is "co" with cursor at index 2
    When the user presses "Tab"
    Then the command input is autocompleted to "comments"
    And the cursor position is placed at the end of the text

  @REQ-6 @history_sync
  Scenario: Cursor alignment on history traversal
    Given the command history contains ["audit", "export patch"]
    And the command input is "" with cursor at index 0
    When the user presses "Up"
    Then the command input is "export patch"
    And the cursor position is 12
    When the user presses "Ctrl-p"
    Then the command input is "audit"
    And the cursor position is 5

  @REQ-7 @visual_rendering
  Scenario: Visual cursor rendering in status bar
    Given the command input is "audit" with cursor at index 2
    When the status bar is rendered
    Then character "d" is rendered with inverted block cursor styling
    Given the command input is "audit" with cursor at index 5
    When the status bar is rendered
    Then a cursor block symbol is appended at the end of the input

  @REQ-4 @word_deletion_forward
  Scenario: Forward word deletion with Alt-d from start of line
    Given the command input is "audit issues" with cursor at index 0
    When the user presses "Alt-d"
    Then the command input is " issues"
    And the cursor position is 0

  @REQ-4 @word_deletion_forward_midword
  Scenario: Forward word deletion with Alt-d from within word
    Given the command input is "audit issues" with cursor at index 2
    When the user presses "Alt-d"
    Then the command input is "au issues"
    And the cursor position is 2

  @REQ-4 @word_deletion_forward_punctuation
  Scenario: Forward word deletion with Alt-d on punctuation tokens
    Given the command input is "foo-bar" with cursor at index 0
    When the user presses "Alt-d"
    Then the command input is "-bar"
    And the cursor position is 0
    When the user presses "Alt-d"
    Then the command input is "bar"
    And the cursor position is 0

  @REQ-1 @word_motion_backward
  Scenario: Backward word motion with Alt-b and Ctrl-Left
    Given the command input is "audit issues" with cursor at index 12
    When the user presses "Alt-b"
    Then the cursor position is 6
    When the user presses "Ctrl-Left"
    Then the cursor position is 0
    When the user presses "Alt-b"
    Then the cursor position remains 0

  @REQ-1 @word_motion_forward
  Scenario: Forward word motion with Alt-f and Ctrl-Right
    Given the command input is "audit issues" with cursor at index 0
    When the user presses "Alt-f"
    Then the cursor position is 5
    When the user presses "Ctrl-Right"
    Then the cursor position is 12
    When the user presses "Alt-f"
    Then the cursor position remains 12

  @REQ-4 @utf8_word_editing
  Scenario: Backward and forward word deletion across multi-byte UTF-8
    Given the command input is "hello 🦀 rocket" with cursor at index 15
    When the user presses "Ctrl-w"
    Then the command input is "hello 🦀 "
    And the cursor position is 8
    When the user presses "Ctrl-w"
    Then the command input is "hello "
    And the cursor position is 6

  @NFR-INPUT-1 @latency
  Scenario: Keystroke latency budget compliance for word navigation
    Given a command line buffer with 100 characters
    When word motion operations are benchmarked over 10,000 iterations
    Then the average latency per operation is under 1 microsecond
