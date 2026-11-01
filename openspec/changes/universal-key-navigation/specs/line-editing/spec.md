# Spec Delta

## MODIFIED Requirements

### Requirement: Character Insertion
Typing a character SHALL insert it at the cursor and advance the cursor by 1. Key events with `CONTROL` or `ALT` modifiers MUST NOT insert characters into the input buffer.

#### Scenario: Mid-string insertion
- **WHEN** the input is `:audi` with the cursor at the end and `t` is pressed
- **THEN** the input is `:audit` with the cursor at index 5
- **WHEN** `Left` is pressed 3 times and `x` is pressed
- **THEN** the input is `:auxdit` with the cursor at index 3

#### Scenario: Normal character entry
- **WHEN** the input is `test` and character `s` is pressed with no modifiers
- **THEN** the input becomes `tests`

#### Scenario: Control chord rejected
- **WHEN** the user presses `Ctrl-w` or `Ctrl-u` in any input field
- **THEN** the character `w` or `u` is NOT appended to the text

### Requirement: Word Deletion
`Ctrl-w`, `Alt-Backspace`, and `Ctrl-Backspace` SHALL delete backward from the cursor (or end of text) to the start of the previous word across all text input fields (`CommandLine`, `Search`, `CommentInput`, `CommentReply`, `CommitModal`, `FilePicker`). Trailing whitespace before the cursor MUST be skipped, then a run of alphanumeric/underscore characters (or a run of punctuation) MUST be removed, and the cursor MUST move to the deletion point. `Alt-d` and `Alt-Delete` in `CommandLine` SHALL delete forward from the cursor to the end of the current or next word.

#### Scenario: Repeated word deletion
- **WHEN** the input is `pr 12` with the cursor at the end and `Ctrl-w` is pressed
- **THEN** the input is `pr ` with the cursor at index 3
- **WHEN** `Ctrl-w` is pressed again
- **THEN** the input is empty with the cursor at index 0

#### Scenario: Trailing spaces
- **WHEN** the input is `audit   ` with the cursor at the end and `Ctrl-w` is pressed
- **THEN** the trailing spaces and `audit` are removed and the input is empty

#### Scenario: Backward word deletion in Search
- **WHEN** the search query is `foo bar` and `Ctrl-w` is pressed
- **THEN** the search query becomes `foo `

#### Scenario: Forward word deletion in CommandLine
- **WHEN** the command line is `audit issues` with cursor at index 0 and `Alt-d` is pressed
- **THEN** the command line becomes ` issues` with cursor at index 0

### Requirement: Line Deletion
`Ctrl-u` SHALL delete from the start of the line to the cursor and set the cursor to 0 in `CommandLine`, and SHALL clear the entire text in `Search`, `CommentInput`, `CommentReply`, and `CommitModal`. `Ctrl-k` SHALL delete from the cursor to the end of the line in `CommandLine` and clear text when cursor is at the start or in single-line modal inputs.

#### Scenario: Clear to start
- **WHEN** the input is `scan repo` with the cursor at the end and `Ctrl-u` is pressed
- **THEN** the input is empty with the cursor at index 0

#### Scenario: Mid-string clears
- **WHEN** the input is `diff main` with the cursor at index 4 and `Ctrl-u` is pressed
- **THEN** the input is ` main` with the cursor at index 0
- **WHEN** the input is `diff main` with the cursor at index 4 and `Ctrl-k` is pressed
- **THEN** the input is `diff` with the cursor at index 4

#### Scenario: Clear search input
- **WHEN** the search query is `error filter` and `Ctrl-u` is pressed
- **THEN** the search query is cleared to empty

#### Scenario: Clear comment input
- **WHEN** the comment input is `needs refactor` and `Ctrl-k` is pressed
- **THEN** the comment input is cleared to empty

## ADDED Requirements

### Requirement: REQ-NAV-1 Universal Line and Item Navigation
All list pickers (`FilePicker`, `PanePicker`, `CommentList`, `ThreadView`, `PrPicker`, `WorktreePicker`) MUST support `Down`, `Ctrl-n`, and `Ctrl-j` to select the next item, and `Up`, `Ctrl-p`, and `Ctrl-k` to select the previous item. Where typing text is not active, `j` and `k` MUST also navigate down and up.

#### Scenario: Navigate WorktreePicker with arrows and vim keys
- **WHEN** `WorktreePicker` is open and `j` or `Down` is pressed
- **THEN** the selection index increments by 1

### Requirement: REQ-NAV-2 Universal Page Navigation
All list pickers and document viewers (`Diff`, `MarkdownPreview`, `AuditReport`, `FilePicker`, `PanePicker`, `CommentList`, `ThreadView`, `PrPicker`, `WorktreePicker`) MUST support `PageDown`, `Ctrl-d`, and `Ctrl-f` to page downward, and `PageUp`, `Ctrl-u`, and `Ctrl-b` to page upward.

#### Scenario: Page through WorktreePicker
- **WHEN** `WorktreePicker` contains 10 items and `Ctrl-d` or `PageDown` is pressed
- **THEN** selection advances by 5 items clamped to the list boundary

### Requirement: REQ-NAV-3 Universal Top and Bottom Jumps
All list pickers and document viewers MUST support `Home` to jump to the first item or line, and `End` to jump to the last item or line. Where unambiguous, `g` / `gg` MUST jump to the top, and `G` MUST jump to the bottom.

#### Scenario: Jump to bottom in Diff
- **WHEN** `Diff` view is active and `End` is pressed
- **THEN** cursor jumps to the bottom line of the diff

### Requirement: REQ-NAV-4 Universal Modal Dismissal
All modal views (`FilePicker`, `PanePicker`, `CommentList`, `ThreadView`, `PrPicker`, `WorktreePicker`, `Search`, `CommitModal`) MUST support `Esc` and `Ctrl-c` to dismiss and return to the underlying view. Pickers without text entry MUST also support `q` to dismiss.

#### Scenario: Dismiss PanePicker with q
- **WHEN** `PanePicker` is open and `q` is pressed
- **THEN** the modal closes and active view returns to `Diff`

### Requirement: NFR-INPUT-1 Non-Blocking Key Handling
Keypress handlers MUST perform in-memory input/state processing and MUST NOT perform blocking I/O. This change imposes no fixed per-keystroke latency threshold.

#### Scenario: Key handling avoids blocking I/O
- **WHEN** key handling executes
- **THEN** it completes without performing blocking I/O
