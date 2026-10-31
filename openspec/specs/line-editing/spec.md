# line-editing Specification

## Purpose

Vim/Readline-style cursor-aware editing of the TUI command bar, so users can fix, extend and recall commands without retyping them.

## Requirements

### Requirement: Cursor Movement
The command bar SHALL track a 0-indexed cursor clamped between `0` and the input's character count. `Left`/`Right` MUST move it by one; `Ctrl-a`/`Home` MUST jump to 0; `Ctrl-e`/`End` MUST jump to the input length.

#### Scenario: Move within the input
- **WHEN** the input is `pr 12` with the cursor at index 5 and `Left` is pressed twice
- **THEN** the cursor is at index 3
- **WHEN** `Ctrl-a` is pressed
- **THEN** the cursor is at index 0
- **WHEN** `Ctrl-e` is pressed
- **THEN** the cursor is at index 5

#### Scenario: Movement is clamped
- **WHEN** the input is `foo` with the cursor at index 0 and `Left` is pressed
- **THEN** the cursor remains at index 0

### Requirement: Character Insertion
Typing a character SHALL insert it at the cursor and advance the cursor by 1.

#### Scenario: Mid-string insertion
- **WHEN** the input is `:audi` with the cursor at the end and `t` is pressed
- **THEN** the input is `:audit` with the cursor at index 5
- **WHEN** `Left` is pressed 3 times and `x` is pressed
- **THEN** the input is `:auxdit` with the cursor at index 3

### Requirement: Character Deletion
`Backspace` SHALL delete the character before the cursor and move the cursor back by 1. `Delete`/`Ctrl-d` SHALL delete the character under the cursor without moving it, and do nothing at the end of the input.

#### Scenario: Backspace then Delete
- **WHEN** the input is `help` with the cursor at index 2 and `Backspace` is pressed
- **THEN** the input is `hlp` with the cursor at index 1
- **WHEN** `Delete` or `Ctrl-d` is pressed
- **THEN** the input is `hp` with the cursor at index 1

#### Scenario: Backspace at the start
- **WHEN** the input is empty and `Backspace` is pressed
- **THEN** the active view switches back to the Diff view
- **WHEN** the input is `foo` with the cursor at index 0 and `Backspace` is pressed
- **THEN** the input and cursor are unchanged

#### Scenario: Delete at the end
- **WHEN** `Delete` is pressed with the cursor at the end of the input
- **THEN** the input is unchanged

### Requirement: Word Deletion
`Ctrl-w` and `Alt-Backspace` SHALL delete backward from the cursor to the start of the previous word. Trailing whitespace before the cursor MUST be skipped, then a run of alphanumeric/underscore characters (or a run of punctuation) MUST be removed, and the cursor MUST move to the deletion point.

#### Scenario: Repeated word deletion
- **WHEN** the input is `pr 12` with the cursor at the end and `Ctrl-w` is pressed
- **THEN** the input is `pr ` with the cursor at index 3
- **WHEN** `Ctrl-w` is pressed again
- **THEN** the input is empty with the cursor at index 0

#### Scenario: Trailing spaces
- **WHEN** the input is `audit   ` with the cursor at the end and `Ctrl-w` is pressed
- **THEN** the trailing spaces and `audit` are removed and the input is empty

### Requirement: Line Deletion
`Ctrl-u` SHALL delete from the start of the line to the cursor and set the cursor to 0. `Ctrl-k` SHALL delete from the cursor to the end of the line.

#### Scenario: Clear to start
- **WHEN** the input is `scan repo` with the cursor at the end and `Ctrl-u` is pressed
- **THEN** the input is empty with the cursor at index 0

#### Scenario: Mid-string clears
- **WHEN** the input is `diff main` with the cursor at index 4 and `Ctrl-u` is pressed
- **THEN** the input is ` main` with the cursor at index 0
- **WHEN** the input is `diff main` with the cursor at index 4 and `Ctrl-k` is pressed
- **THEN** the input is `diff` with the cursor at index 4

### Requirement: Autocomplete and History Cursor Sync
Cycling autocompletions (`Tab`/`Shift-Tab`) and traversing history (`Up`/`Down`/`Ctrl-p`/`Ctrl-n`) SHALL place the cursor at the end of the resulting input.

#### Scenario: Autocomplete moves cursor to end
- **WHEN** `Tab` selects a completion
- **THEN** the input shows the completion and the cursor is at its end

#### Scenario: History recall moves cursor to end
- **WHEN** `Up` loads a history entry
- **THEN** the cursor is at the end of that entry

### Requirement: Cursor Visual Feedback
The rendered command bar SHALL visually indicate the cursor position within the input.

#### Scenario: Cursor rendered mid-string
- **WHEN** the cursor is in the middle of the input
- **THEN** the status bar renders the cursor at that position, not at the end
