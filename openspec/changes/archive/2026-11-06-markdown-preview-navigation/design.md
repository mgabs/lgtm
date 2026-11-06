# Architecture & Technical Design

## 1. Overview & Invariants
This design implements keystroke delegation and cursor synchronization from `MarkdownPreview` mode into standard `Diff` view operations:
- **Cursor Synchronization Invariant**: Before switching `active_view` from `MarkdownPreview` to `Diff`, `sync_cursor_to_markdown_row()` maps the preview row index to the corresponding line in `self.snapshot.files[self.selected_file_idx]`. If no direct line mapping exists (e.g. unnumbered fence padding), the current diff cursor remains intact.
- **View Transition Invariant**: Pressing `e`, `o`, `[`, or `]` immediately sets `self.active_view = ActiveView::Diff` and sets `self.comment_source_view = ActiveView::Diff`, ensuring any follow-up events or editor closures operate within the standard Diff lifecycle.
- **Rejected Alternatives**:
  - *Internal Chord Engine in Markdown Preview*: Rejected due to state synchronization overhead and YAGNI. Duplicating chord buffers, unreviewed file jumps, and alert traversals would duplicate ~150 lines of logic.
  - *Lifting the `apply_ready_local` active view restriction*: Rejected for this change scope to minimize blast radius; exiting to Diff view guarantees safe, battle-tested diff reload.

## 2. Domain Models, State & Interface Contracts
No new structs or enums are introduced. Existing types are leveraged:
- `ActiveView::MarkdownPreview` and `ActiveView::Diff` in `src/app/mod.rs`.
- `App::sync_cursor_to_markdown_row(&mut self) -> bool` in `src/app/mod.rs`.
- `App::open_in_editor(&mut self)` in `src/app/external.rs`.
- `App::handle_diff_key(&mut self, key: KeyEvent)` in `src/app/input.rs`.
- `App::toggle_review_current_file(&mut self)` in `src/app/motion.rs`.

Key transitions in `src/app/input.rs:handle_markdown_preview_key`:
```rust
(KeyModifiers::NONE, KeyCode::Char('e')) | (KeyModifiers::NONE, KeyCode::Char('o')) => {
    self.sync_cursor_to_markdown_row();
    self.comment_source_view = ActiveView::Diff;
    self.active_view = ActiveView::Diff;
    self.open_in_editor();
}
(KeyModifiers::NONE, KeyCode::Char(']')) | (KeyModifiers::NONE, KeyCode::Char('[')) => {
    self.sync_cursor_to_markdown_row();
    self.comment_source_view = ActiveView::Diff;
    self.active_view = ActiveView::Diff;
    self.handle_diff_key(key);
}
(KeyModifiers::NONE, KeyCode::Char('f')) | (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
    self.active_view = ActiveView::FilePicker;
    self.file_picker.query.clear();
    self.file_picker.selected_idx = 0;
    self.update_file_picker_filter();
}
(KeyModifiers::NONE, KeyCode::Char('v')) => {
    self.toggle_review_current_file();
}
(KeyModifiers::NONE, KeyCode::Char('r')) => {
    if let Some(t_idx) = self.thread_at_cursor() {
        self.start_thread_reply(t_idx);
    } else if let Some(idx) = self.comment_idx_at_cursor() {
        self.start_reply_to_comment(idx);
    } else {
        self.sync_cursor_to_markdown_row();
        self.comment_input.clear();
        self.comment_source_view = ActiveView::MarkdownPreview;
        self.active_view = ActiveView::CommentInput;
    }
}
(KeyModifiers::NONE, KeyCode::Char('?')) => {
    self.active_view = ActiveView::Help;
}
```

## 3. Architecture Diagram & Data Flow

```
+--------------------------------------------------------------------+
|               handle_markdown_preview_key(key)                     |
+--------------------------------------------------------------------+
       |                     |                  |             |
   'e' / 'o'             '[' / ']'             'f'           'v'
       |                     |                  |             |
       v                     v                  v             v
sync_cursor()         sync_cursor()       FilePicker    toggle_review()
active_view=Diff      active_view=Diff    active_view   (in place)
open_in_editor()      handle_diff_key()   =FilePicker
(launches editor)     (initiates chord)
```

## 4. Boundary Rules & Budgets
- **Pure Core vs. I/O Shell**:
  - All changes reside in the UI shell (`src/app/input.rs`, `src/ui/status.rs`).
  - `lgtm-core` remains untouched, strictly free of I/O, terminal dependencies, and external framework coupling.
- **Concurrency, Latency & Resource Budgets**:
  - Hot path: Keystroke handling involves only pointer/state updates and branch matching (< 1 microsecond execution budget).
  - Memory: Zero new allocations during navigation or view transitions.

## 5. Testing Strategy
1. **BDD Scenarios**:
   - `test_markdown_preview_editor_launch`: Verify `e`/`o` calls `open_in_editor` and transitions to `Diff`.
   - `test_markdown_preview_bracket_navigation`: Verify `[` and `]` set `pending_leader` and switch to `Diff`.
   - `test_markdown_preview_auxiliary_shortcuts`: Verify `f` opens file picker, `v` toggles review, `?` opens help modal.
2. **Unit Tests (>= 95% coverage)**:
   - Target `src/app/input.rs` and `src/ui/status.rs`.
3. **Mutation Testing (100% kill)**:
   - Verify all mutants in modified handlers are killed by tests.

## Review Gate
- [x] **Gate 3 Approval**: Architecture & design reviewed and approved by human.
