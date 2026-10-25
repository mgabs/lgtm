# Specification: Command Line Vim/Readline Line Editing

## Capability: `line-editing`

### 1. Requirements

- **REQ-1 (Cursor Movement)**: The command bar MUST track a 0-indexed cursor position within the command string, clamped between `0` and `input.chars().count()`.
  - `Left` arrow MUST decrement cursor position by 1 (clamped at 0).
  - `Right` arrow MUST increment cursor position by 1 (clamped at input length).
  - `Ctrl-a` / `Home` MUST set cursor position to 0.
  - `Ctrl-e` / `End` MUST set cursor position to input length.
- **REQ-2 (Character Insertion)**: Typing a character MUST insert it at the current cursor position and advance the cursor position by 1.
- **REQ-3 (Character Deletion)**:
  - `Backspace` MUST delete the character immediately preceding the cursor and decrement the cursor position by 1. If cursor is at 0 and input is empty, exit command mode to Diff view. If cursor is at 0 and input is non-empty, do nothing.
  - `Delete` / `Ctrl-d` MUST delete the character under the cursor without changing cursor position (unless cursor was at the end, where it does nothing).
- **REQ-4 (Word Deletion)**:
  - `Ctrl-w` / `Alt-Backspace` MUST delete backward from the cursor to the start of the previous word.
  - Word boundary rule: Skip trailing whitespace before cursor, then delete continuous alphanumeric/underscore characters (or continuous punctuation symbols).
  - Cursor position MUST be updated to the deletion point.
- **REQ-5 (Line Deletion)**:
  - `Ctrl-u` MUST delete all characters from the beginning of the line up to the cursor position, setting cursor position to 0. If cursor is already at end, it clears the entire input.
  - `Ctrl-k` MUST delete all characters from the current cursor position to the end of the line.
- **REQ-6 (Autocomplete & History Sync)**:
  - Cycling autocompletions (Tab / Shift-Tab) MUST update the input text and place the cursor at the end of the input string.
  - History traversal (Up / Down / `Ctrl-p` / `Ctrl-n`) MUST place the cursor at the end of the loaded history string.
- **REQ-7 (Visual Feedback)**:
  - The rendered command bar in `render_status_bar` MUST visually indicate the cursor position within the input string.

---

### 2. Behavioral Scenarios (Given / When / Then)

#### Scenario 2.1: Cursor Movement
- **Given** command input is `pr 12` with cursor at index 5 (end)
- **When** key `Left` is pressed twice
- **Then** cursor position is index 3 (pointing to space before `12`)
- **When** key `Ctrl-a` is pressed
- **Then** cursor position is index 0
- **When** key `Ctrl-e` is pressed
- **Then** cursor position is index 5

#### Scenario 2.2: Mid-String Insertion
- **Given** command input is `:audi` with cursor at index 4 (end)
- **When** key `t` is pressed
- **Then** command input is `:audit` with cursor at index 5
- **When** key `Left` is pressed 3 times (cursor at index 2, pointing to `d`)
- **When** key `x` is pressed
- **Then** command input is `:auxdit` with cursor at index 3

#### Scenario 2.3: Backward Word Deletion (`Ctrl-w`)
- **Given** command input is `pr 12` with cursor at index 5 (end)
- **When** key `Ctrl-w` is pressed
- **Then** command input is `pr ` with cursor at index 3
- **When** key `Ctrl-w` is pressed again
- **Then** command input is empty `""` with cursor at index 0

#### Scenario 2.4: Word Deletion with Trailing Spaces
- **Given** command input is `audit   ` with cursor at index 8 (end)
- **When** key `Ctrl-w` is pressed
- **Then** trailing spaces and preceding word `audit` are removed, resulting in `""` with cursor at index 0

#### Scenario 2.5: Line Clear to Start (`Ctrl-u`)
- **Given** command input is `scan repo` with cursor at index 9 (end)
- **When** key `Ctrl-u` is pressed
- **Then** command input is empty `""` with cursor at index 0

#### Scenario 2.6: Mid-String Line Clear (`Ctrl-u` and `Ctrl-k`)
- **Given** command input is `diff main` with cursor at index 4 (between `diff` and ` main`)
- **When** key `Ctrl-u` is pressed
- **Then** command input is ` main` with cursor at index 0
- **Given** command input is `diff main` with cursor at index 4
- **When** key `Ctrl-k` is pressed
- **Then** command input is `diff` with cursor at index 4

#### Scenario 2.7: Character Deletion (`Backspace` vs `Delete`/`Ctrl-d`)
- **Given** command input is `help` with cursor at index 2 (between `e` and `l`)
- **When** key `Backspace` is pressed
- **Then** command input is `hlp` with cursor at index 1
- **When** key `Delete` or `Ctrl-d` is pressed
- **Then** command input is `hp` with cursor at index 1

#### Scenario 2.8: Boundaries and Edge Cases
- **Given** command input is empty `""` with cursor at index 0
- **When** key `Backspace` is pressed
- **Then** active view switches back to `ActiveView::Diff`
- **Given** command input is `foo` with cursor at index 0
- **When** key `Backspace` or `Left` or `Ctrl-u` is pressed
- **Then** input remains `foo` and cursor remains at index 0
- **When** key `Delete` is pressed with cursor at end of input
- **Then** input remains unchanged
