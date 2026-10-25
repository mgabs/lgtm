# Architecture & Technical Design: Command Line Line Editing

## 1. Architectural Overview & Invariants

This design introduces character-level line editing and Vim/Readline shortcut support to `CommandLine` in `lgtm`.
It operates entirely within the existing application event loop and state management architecture, requiring **zero new external dependencies**.

### Key Invariants
1. **UTF-8 Safety**: Cursor indexing is character-based (`chars().count()`), not raw byte offsets, ensuring no slicing across multi-byte character boundaries.
2. **Bounds Clamping**: Cursor position `cursor` is strictly clamped to `[0, input.chars().count()]`.
3. **Purity of State**: All editing transformations (insert, delete char, delete word, clear to start/end) are pure methods on `CommandLine`, fully unit-testable without mocking TUI or crossterm events.
4. **Zero Regressions**: Autocompletion (`Tab`, `Shift-Tab`), history navigation (`Up`, `Down`, `Ctrl-p`, `Ctrl-n`), command execution (`Enter`), and prompt dismissal (`Esc`) remain fully operational with updated cursor placement.

---

## 2. State & Interface Changes

### 2.1 State Modifications (`src/app/state.rs`)
Add `pub cursor: usize` to `CommandLine`:

```rust
#[derive(Debug, Default, Clone)]
pub struct CommandLine {
    pub input: String,
    pub cursor: usize,
    pub history: Vec<String>,
    pub history_idx: Option<usize>,
    pub completions: Vec<String>,
    pub completion_idx: Option<usize>,
    pub prefix: String,
}
```

### 2.2 Core Manipulation Methods (`CommandLine`)
```rust
impl CommandLine {
    /// Resets input, completions, and cursor.
    pub fn reset(&mut self);

    /// Sets input text and repositions cursor to the end.
    pub fn set_text(&mut self, text: String);

    /// Moves cursor left by one char (clamped at 0).
    pub fn move_left(&mut self);

    /// Moves cursor right by one char (clamped at len).
    pub fn move_right(&mut self);

    /// Jumps cursor to index 0.
    pub fn move_home(&mut self);

    /// Jumps cursor to end of string.
    pub fn move_end(&mut self);

    /// Inserts char at cursor position and increments cursor.
    pub fn insert_char(&mut self, c: char);

    /// Deletes char before cursor (Backspace behavior).
    /// Returns false if cursor was at 0 and input was empty.
    pub fn delete_backward(&mut self) -> bool;

    /// Deletes char at current cursor (Delete / Ctrl-d behavior).
    pub fn delete_forward(&mut self);

    /// Deletes word backward from cursor (Ctrl-w / Alt-Backspace behavior).
    pub fn delete_word_backward(&mut self);

    /// Deletes all characters from start of input to cursor (Ctrl-u behavior).
    pub fn delete_to_start(&mut self);

    /// Deletes all characters from cursor to end of input (Ctrl-k behavior).
    pub fn delete_to_end(&mut self);
}
```

---

## 3. Algorithm: Backward Word Deletion (`delete_word_backward`)

To match Unix/Readline `unix-word-rubout` and Vim `Ctrl-w`:
1. Convert `input` into a vector of characters: `chars: Vec<char>`.
2. Inspect slice `chars[..cursor]`:
   - If empty, return.
3. Phase 1: Skip trailing whitespace characters backward from `cursor`.
4. Phase 2: If the first non-whitespace character is alphanumeric or underscore (`c.is_alphanumeric() || c == '_'`), delete backward while characters are alphanumeric or underscore.
   Otherwise (it is a punctuation symbol), delete backward while characters are punctuation symbols.
5. Reconstruct `input` and update `cursor` to the start index of the deleted range.

---

## 4. Event Dispatch Mapping (`src/app/input.rs`)

In `handle_command_line_key(key: KeyEvent)`:

| Key Chord | Action |
|---|---|
| `Left` | `self.command.move_left()` |
| `Right` | `self.command.move_right()` |
| `Home` / `Ctrl-a` | `self.command.move_home()` |
| `End` / `Ctrl-e` | `self.command.move_end()` |
| `Backspace` | If `!self.command.delete_backward()` -> switch to `ActiveView::Diff` |
| `Delete` / `Ctrl-d` | `self.command.delete_forward()` |
| `Ctrl-w` / `Alt-Backspace` | `self.command.delete_word_backward()` |
| `Ctrl-u` | `self.command.delete_to_start()` |
| `Ctrl-k` | `self.command.delete_to_end()` |
| `Char(c)` | `self.command.insert_char(c)` |
| `Tab` / `BackTab` | Autocomplete cycles and resets cursor to end |
| `Up` / `Down` / `Ctrl-p` / `Ctrl-n` | History traversal updates text and resets cursor to end |
| `Enter` | Executes command and resets prompt state |
| `Esc` / `Ctrl-c` | Clears prompt and returns to `ActiveView::Diff` |

---

## 5. UI Rendering (`src/ui/status.rs`)

When `app.active_view == ActiveView::CommandLine`:
- Split `app.command.input` into `before_cursor`, `char_at_cursor`, and `after_cursor`.
- If `char_at_cursor` is `Some(c)`, render `c` with inverted colors (`bg: Yellow, fg: Black, BOLD`) representing a block cursor over the character.
- If `char_at_cursor` is `None` (cursor at end of string), append `"█ "` with `fg: Yellow`.
- Render completion candidates and metadata following the cursor.

---

## 6. Performance, Concurrency & Main-Loop Invariants

### 6.1 Sub-Microsecond Keystroke Handling (Zero Thread-Hop Overhead)
- **Main Loop Synchronicity**: Keystroke handling for line editing (cursor movement, character insertions, word/line deletions) is strictly in-memory and operates in **< 1 microsecond** per event.
- **Why Keystroke Processing Stays on the Main Loop**: Moving individual keystroke string operations to an async runtime or worker thread would introduce channel synchronization latency (1–5 µs), mutex contention, and risk keystroke interleaving/dropped frames. Keystroke processing MUST remain synchronous on the main thread to ensure immediate terminal response (< 16ms frame deadline / 60+ FPS).
- **Allocation Optimization**: Avoid full string reallocations on each keypress. `delete_word_backward` and `delete_to_start` perform in-place byte manipulation (`drain` / `truncate` / `insert`) on pre-allocated `String` buffers rather than intermediate `Vec<char>` heaps where possible.

### 6.2 Offloading Heavy Work Off the Main Loop
- **Non-Blocking Command Dispatch**: The command bar only collects input. When a command is submitted via `Enter` (e.g., `:audit`, `:scan repo`, `:diff <branch>`, `:pr <num>`):
  - Heavy I/O (Git tree walking, AST parsing, network calls to GitHub/Forge) is dispatched to background worker threads via `crossbeam-channel` or `audit_jobs`.
  - The main event loop receives progress updates and results via non-blocking channel polling (`try_recv`), keeping UI rendering completely smooth.
- **Autocomplete Isolation**: Command autocompletion operates on an in-memory static table (~60 items) taking < 300 nanoseconds. No blocking filesystem or Git I/O is performed synchronously during `Tab` autocompletion.

---

## 7. Testing Strategy

1. **Unit Tests (`src/app/state.rs`)**:
   - Comprehensive tests for each `CommandLine` method with ASCII and multi-byte UTF-8 inputs.
   - Word boundary tests with spaces, mixed alphanumeric/punctuation tokens (`pr 12`, `:foo-bar`, `audit   `).
2. **Integration / Event Loop Tests (`src/app/input.rs`)**:
   - Test `handle_command_line_key` with key event streams verifying cursor moves, insertions, and deletions.
3. **Rendering Regression Tests (`src/ui/status.rs`)**:
   - Test backend render verification that cursor span is produced at the expected column index.
