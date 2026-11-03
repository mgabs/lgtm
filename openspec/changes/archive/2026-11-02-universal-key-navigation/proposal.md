# Proposal

## Intent
Unify word and line deletion across all text entry modes (`CommandLine`, `Search`, `CommentInput`, `CommentReply`, `CommitModal`, `FilePicker`) and standardize navigation keybindings (`Up`/`Down`, `PageUp`/`PageDown`, `Home`/`End`, `g`/`G`, `Esc`/`Ctrl-c`) across all document and picker views in LGTM. Picker-only `q` dismissal applies where text entry is inactive.

## Problem Statement
Currently, text editing behavior is fragmented: only `CommandLine` supports `Ctrl-w` and `Ctrl-u`, while other input fields leak control characters into text or only support single-character `Backspace`. Furthermore, navigation keys are inconsistent across views: `WorktreePicker` lacks page movement entirely, several pickers lack `Home`/`End`/`g`/`G` and `Ctrl-n`/`Ctrl-p`, and `PanePicker` cannot be dismissed with `q`.

## Capabilities

### Modified Capabilities
- `line-editing`: Extends word and line deletion requirements beyond `CommandLine` to cover all text input fields (`Search`, `CommentInput`, `CommentReply`, `CommitModal`, `FilePicker`), adds modifier checking to character insertion, adds forward word deletion / word motion to `CommandLine`, and defines universal list/document navigation standards across all views.

## Scope

### In-Scope
- Shared word deletion primitives (`delete_word_backward_at`, `delete_word_backward_str`) in `src/app/state.rs`.
- `CommandLine` forward word deletion (`Alt-d`) and word navigation (`Alt-b`/`Alt-f`/`Ctrl-Left`/`Ctrl-Right`).
- Backward word deletion (`Ctrl-w`, `Alt-Backspace`, `Ctrl-Backspace`) across `Search`, `CommentInput`, `CommentReply`, `CommitModal`, `FilePicker`.
- Line deletion (`Ctrl-u`, `Ctrl-k`) across `Search`, `CommentInput`, `CommentReply`, `CommitModal`.
- Modifier check on character insertion to prevent chords like `Ctrl-w` and `Ctrl-u` from inserting characters.
- Universal line navigation (`Down`/`j`/`Ctrl-n`/`Ctrl-j`, `Up`/`k`/`Ctrl-p`/`Ctrl-k`) across list pickers (`FilePicker`, `PanePicker`, `CommentList`, `ThreadView`, `PrPicker`, `WorktreePicker`).
- Universal page navigation (`PageDown`/`Ctrl-d`/`Ctrl-f`, `PageUp`/`Ctrl-u`/`Ctrl-b`) across all list pickers and document viewers (`Diff`, `MarkdownPreview`, `AuditReport`, `FilePicker`, `PanePicker`, `CommentList`, `ThreadView`, `PrPicker`, `WorktreePicker`).
- Universal top/bottom navigation (`Home`/`End`, plus `g`/`G` where unambiguous) across `Diff`, `MarkdownPreview`, `AuditReport`, and all pickers.
- Universal modal dismissal (`Esc`/`Ctrl-c`) across modal views, with `q` additionally dismissing pickers without text entry.

### Out-of-Scope (YAGNI)
- No multiline text editor widget or syntax-highlighted input area.
- No new external crate dependencies.
- No changes to persistence stores, diff computation, or AST folding.

## Assumptions
- **ASM-GEN-1**: Crossterm key events report `KeyModifiers::CONTROL` and `KeyModifiers::ALT` predictably across macos, linux, and windows.
- **ASM-GEN-2**: In `FilePicker`, `Ctrl-u` retains its existing page-up behavior for compatibility, while `Ctrl-w` / `Alt-Backspace` handle word deletion.

## Success Metrics
- 100% pass rate on full test suite (baseline: 357 passing tests).
- Regression tests cover each specified key mapping and relevant input-editing edge case.
- Key handling remains in-memory and avoids blocking I/O; no fixed per-keystroke latency threshold is imposed.

## Review Gate
- [x] **Gate 1 Approval**: Proposal approved by the user on 2026-11-01.
