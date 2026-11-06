# Proposal

## Intent
Enable external editor launch (`e`/`o`), bracket navigation (`[`/`]`), file picker modal (`f`/`Ctrl-p`), reviewed toggle (`v`), comment reply (`r`), and help modal (`?`) from Markdown Preview by synchronizing the diff cursor and delegating to standard diff navigation handlers.

## Problem Statement
When reviewing documentation changes in `MarkdownPreview` mode:
1. Reviewers cannot jump to the external editor (`e`) to make edits directly on the line being read.
2. Reviewers cannot navigate files, hunks, or comments using standard bracket leader chords (`[` / `]`).
3. Opening the file picker (`f`), replying to comments (`r`), toggling reviewed state (`v`), or viewing help (`?`) are ignored.
4. If an external change occurs while in Markdown Preview, background updates are blocked by the view gate, leaving the screen frozen.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `markdown-preview`: Updates preview navigation requirements to include `e`/`o` editor dispatch, `[`/`]` chord delegation, `f`/`Ctrl-p` file picker, `v` review toggle, `r` comment reply, and `?` help modal, alongside status line shortcut indicators.

## Scope

### In-Scope
- `src/app/input.rs:handle_markdown_preview_key`:
  - `e` / `o`: Calls `sync_cursor_to_markdown_row()`, sets `active_view = ActiveView::Diff`, calls `open_in_editor()`.
  - `[` / `]`: Calls `sync_cursor_to_markdown_row()`, sets `active_view = ActiveView::Diff`, calls `handle_diff_key(key)`.
  - `f` / `Ctrl-p`: Opens `ActiveView::FilePicker`.
  - `v`: Toggles review state on current file.
  - `r`: Opens comment reply modal if comment exists on cursor row; opens comment input otherwise.
  - `?`: Opens `ActiveView::Help`.
- `src/ui/status.rs`:
  - Displays `[e] Edit | [[/]] Nav | [f] Files | [v] Review` in the Markdown Preview status line.
- Tests in `src/app/input.rs` and `src/app/markdown.rs` verifying keystroke handling and state transitions.

### Out-of-Scope (YAGNI)
- Modifying `apply_ready_local` or `apply_ready_pr` background refresh gates in `src/app/refresh.rs` or `src/app/pr.rs`.
- Maintaining an independent chord state machine inside `MarkdownPreviewState`.
- Retaining `MarkdownPreview` when navigating to non-markdown files.
- Modifying crates other than `lgtm` (crates `lgtm-core`, `lgtm-ast`, `lgtm-mcp` unaffected).

## Assumptions
- **ASM-GEN-1**: Toggling from `MarkdownPreview` to `Diff` on navigation chords (`[` / `]`) and editor (`e`) provides immediate access to full diff capabilities without code bloat.
- **ASM-GEN-2**: Existing `sync_cursor_to_markdown_row()` accurately targets the diff line corresponding to the preview cursor row when available.

## Success Metrics
- Keystroke latency for `e`, `[`, `]`, `f`, `v`, `r`, `?` remains under 1 microsecond.
- All unit tests pass with >= 95% line and branch coverage on modified code.
- Cross-platform CI matrix (ubuntu, macos, windows) remains green with zero regressions.

## Review Gate
- [x] **Gate 1 Approval**: Proposal reviewed and approved by human.
