# Bug Report: TUI Fails to Render After Exiting External CLI Editor (Vim/Neovim)

## Status
- **Status**: Resolved
- **Severity**: High (Forces user to kill application)
- **Component**: External Editor Launcher (`src/app/external.rs`), Main TUI Render Loop (`src/main.rs`)
- **Tests Added**:
  - `src/app/external.rs::test_cli_editor_flags_terminal_clear`
  - `src/app/external.rs::test_gui_editor_does_not_flag_terminal_clear`

---

## Summary
When launching an external terminal editor (such as Vim or Neovim) from within `lgtm` and subsequently exiting the editor, the application returns to a completely blank or unrendered terminal screen. The application appears frozen because no UI components are painted, forcing the user to terminate the process (`Ctrl-C` or closing the terminal window).

---

## Steps to Reproduce
1. Start `lgtm` in any git repository with active diffs:
   ```bash
   cargo run
   ```
2. Set the editor to Vim (or use default `nvim`):
   ```
   :editor vim
   ```
3. Trigger editor launch on the current file/line (press `e` or issue `:e`).
4. Vim launches on the alternate screen.
5. In Vim, save or exit without changes:
   ```
   :q
   ```
6. Return to `lgtm`.

### Expected Result
`lgtm` immediately repaints the diff view, file tree, status bar, and header.

### Actual Result
The terminal display remains completely blank. Keystrokes do not produce visual updates because Ratatui diffs against a stale pre-vim buffer.

---

## Root Cause Analysis
1. In `src/app/external.rs:launch_editor`:
   ```rust
   if !is_gui {
       let _ = disable_raw_mode();
       let _ = crossterm::execute!(std::io::stdout(), LeaveAlternateScreen);
   }
   // CLI editor executes and blocks...
   if !is_gui {
       let _ = crossterm::execute!(std::io::stdout(), EnterAlternateScreen);
       let _ = enable_raw_mode();
   }
   ```
2. When the editor exits, `crossterm::execute!(std::io::stdout(), EnterAlternateScreen)` re-enters alternate screen, which clears the terminal screen at the terminal emulator level.
3. However, `Ratatui::Terminal` in `src/main.rs` retains its internal cached `previous_buffer`.
4. In `src/main.rs`:
   ```rust
   event_loop.step(&mut app, &mut input, &mut diagnostics, |app| {
       lgtm_core::debug::measure("ui.draw", || terminal.draw(|f| ui::render(f, app)))?;
       Ok(())
   })?;
   ```
5. `terminal.draw()` computes a diff between `current_buffer` and `previous_buffer`. Because `App` state is virtually unchanged, Ratatui computes zero cell updates and skips sending ANSI escape sequences to the terminal.
6. `terminal.clear()` was never called on the `Terminal` instance to invalidate Ratatui's internal buffers.

---

## Remediation Plan
1. **Flag State Invalidation**:
   - Add `pub needs_terminal_clear: bool` to `App`.
   - Set `self.needs_terminal_clear = true` whenever `launch_editor` returns from a CLI editor.
2. **Explicit Buffer Invalidation in Draw Loop**:
   - In `src/main.rs` (the render closure):
     ```rust
     if app.needs_terminal_clear {
         terminal.clear()?;
         app.needs_terminal_clear = false;
     }
     lgtm_core::debug::measure("ui.draw", || terminal.draw(|f| ui::render(f, app)))?;
     ```
3. **Cursor and Flush Synchronization**:
   - Ensure stdout is flushed and cursor visibility is refreshed after alternate screen re-entry.
