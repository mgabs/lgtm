# Change Proposal: Command Line Vim/Readline Editing Shortcuts

## Intent
Provide standard Vim/Readline command-line editing shortcuts in `lgtm`'s `:` command bar, allowing reviewers to delete words (`Ctrl-w`), clear lines (`Ctrl-u`), navigate cursor positions, and fix command typos without having to repeatedly hammer backspace.

## Scope

### In-Scope
- **Cursor Position Tracking**: Track cursor index within `app.command.input` instead of treating the prompt as append-only.
- **Delete Word (`Ctrl-w`, `Alt-Backspace`)**: Delete the word immediately preceding the cursor.
- **Delete Line (`Ctrl-u`)**: Delete from the cursor position to the start of the line (or clear entire input if at the end).
- **Delete Forward (`Delete`, `Ctrl-d`)**: Delete the character under the cursor.
- **Delete to End (`Ctrl-k`)**: Delete from the cursor position to the end of the line.
- **Cursor Movement**:
  - `Left` / `Right`: Move cursor by one character.
  - `Home` / `Ctrl-a`: Jump cursor to the start of the line.
  - `End` / `Ctrl-e`: Jump cursor to the end of the line.
- **Visual Cursor Rendering**: Render the cursor at the active cursor index in `render_status_bar`.

### Out-of-Scope (YAGNI)
- Full modal normal mode inside the command prompt (pressing `Esc` will continue to abort and return to the diff view).
- Multi-line command editing.
- Ex-mode range addressing (e.g. `:'<,'>s/...`).

## Problem Statement
When typing commands such as `:audit` or `:pr 12`, users who make a typo (e.g., `:audi`) or want to change arguments currently have to press Backspace repeatedly. There is no way to delete a word, clear the line, or move the cursor to edit a character in the middle of the input string. This violates muscle memory for users accustomed to Vim and Readline environments.

## Success Metrics
- **Muscle Memory Parity**: Hitting `Ctrl-w` deletes the preceding word cleanly in one keystroke.
- **Instant Line Reset**: Hitting `Ctrl-u` clears back to the `:` prompt in one keystroke.
- **Visual Accuracy**: Cursor positioning reflects exact insertion/deletion points in the rendered command bar.
- **Zero Regressions**: Autocompletion (Tab/Shift-Tab), history recall (Up/Down/Ctrl-p/Ctrl-n), command execution (Enter), and cancellation (Esc) continue working as expected.
