# Research & Technical Tradeoffs

## 1. Intent & Ground Truth

### Problem
1. **Word & Line Deletion Inconsistency**:
   - `CommandLine` has cursor-aware word deletion (`Ctrl-w`, `Alt-Backspace`) and line deletion (`Ctrl-u`, `Ctrl-k`).
   - `Search`, `CommentInput`, `CommentReply`, and `CommitModal` lack word deletion (`Ctrl-w`, `Alt-Backspace`, `Ctrl-Backspace`) and universal line deletion (`Ctrl-u`, `Ctrl-k`).
   - In `Search`, `CommentInput`, `CommentReply`, and `FilePicker`, modifier checking is missing: typing `Ctrl-w` or `Ctrl-u` matches `KeyCode::Char(c)` and pushes `'w'` or `'u'` into the input buffer.
   - `CommandLine` lacks forward word deletion (`Alt-d`) and word navigation (`Alt-b`/`Alt-f`/`Ctrl-Left`/`Ctrl-Right`).

2. **Non-Universal Navigation**:
   - List & Document views have divergent key bindings for identical semantic actions:
     - **Line/Item Navigation**: `FilePicker` supports `Ctrl-j`/`Ctrl-k` but not `Ctrl-n`/`Ctrl-p`. Other pickers (`PanePicker`, `CommentList`, `ThreadView`, `PrPicker`, `WorktreePicker`) lack `Ctrl-n`/`Ctrl-p` and `Ctrl-j`/`Ctrl-k`.
     - **Page Navigation**: `WorktreePicker` has zero page navigation (`PageDown`, `PageUp`, `Ctrl-d`, `Ctrl-u`, `Ctrl-f`, `Ctrl-b` are all unhandled). `ThreadView`, `MarkdownPreview`, and `AuditReport` lack `Ctrl-f`/`Ctrl-b`.
     - **Top / Bottom Jumps**: `Diff` lacks `Home`/`End`. `MarkdownPreview` has `Home`/`End`. All modal pickers (`FilePicker`, `PanePicker`, `CommentList`, `ThreadView`, `PrPicker`, `WorktreePicker`) and `AuditReport` lack `Home`/`End`, and pickers lack `g`/`G` where unambiguous.
     - **Modal Dismissal**: `PanePicker` handles `Esc` but not `q` or `Ctrl-c`. Other pickers handle `Esc` and `q`.

### Ground Truth in Codebase
- Verified in `src/app/state.rs`: `CommandLine` implements `delete_word_backward` and `delete_to_start`.
- Verified in `src/app/input.rs`:
  - `handle_search_key` matches raw `KeyCode::Char(c)` without checking modifiers.
  - `handle_comment_input_key` matches raw `KeyCode::Char(c)` without checking modifiers.
  - `handle_comment_reply_key` matches raw `KeyCode::Char(c)` without checking modifiers.
  - `handle_file_picker_key` matches raw `KeyCode::Char(c)` without checking modifiers.
  - `handle_worktree_picker_key` only handles `Up`, `Down`, `k`, `j`, `Enter`, `Esc`, `q`.

## 2. Options Explored

### Option A: Shared String Primitives & Universal Navigation Normalization (Recommended)
- Extract `delete_word_backward_at` and `delete_word_backward_str` into `src/app/state.rs`.
- Wire `Ctrl-w`, `Alt-Backspace`, `Ctrl-Backspace` to word deletion across all text inputs (`Search`, `CommentInput`, `CommentReply`, `CommitModal`, `FilePicker`).
- Wire `Ctrl-u` and `Ctrl-k` to clear / delete line across single-line text inputs.
- Guard character insertion to `KeyModifiers::NONE | KeyModifiers::SHIFT` to prevent modifier leakage.
- Standardize list motions:
  - Line: `Down`/`j`/`Ctrl-j`/`Ctrl-n` and `Up`/`k`/`Ctrl-k`/`Ctrl-p` across all list pickers.
  - Page: `PageDown`/`Ctrl-d`/`Ctrl-f` and `PageUp`/`Ctrl-u`/`Ctrl-b` across all list pickers and document views.
  - Top/Bottom: `Home`/`End` across all views, and `g`/`G` across list pickers without text entry.
  - Exit: `Esc`/`q`/`Ctrl-c` across all modal pickers.

### Option B: Monolithic Text-Buffer Struct Replacement
- Replace all `String` input fields (`search_query`, `comment_input`, etc.) with full `CommandLine` instances.
- *Pros*: Uniform struct across all inputs.
- *Cons*: High blast radius, complicates ratatui rendering and tests that inspect `app.search_query: String` or `app.comment_input: String`. Heavy violation of YAGNI.

### Option C: Isolated Ad-Hoc Key Handlers per View
- Copy-paste string manipulation logic into each `handle_*_key` function independently.
- *Pros*: No shared state helper.
- *Cons*: High duplication, inconsistent word-boundary rules, maintenance debt.

## 3. Tradeoff Matrix

| Criteria | Option A (Shared Primitives & Universal Nav) | Option B (Monolithic Structs) | Option C (Ad-Hoc Duplication) |
| :--- | :--- | :--- | :--- |
| **Complexity** | Low | High | Medium |
| **New Dependencies** | 0 | 0 | 0 |
| **Performance & Latency** | Sub-microsecond | Sub-microsecond | Sub-microsecond |
| **Maintenance Burden** | Low (single word boundary logic) | Medium (state migrations) | High (drifting implementations) |
| **Blast Radius** | Minimal (input & nav handlers only) | High (all UI & test fields) | Low |

## 4. Recommendation
Adopt Option A. Cleanly extracts word deletion to a shared helper in `state.rs`, guards character entry against modifier leakage, and harmonizes navigation shortcuts across all views without disrupting existing public struct fields.

## 5. Breaking Changes & Deprecations
- Modifies and extends `openspec/specs/line-editing/spec.md` to cover all input fields and view navigation.
- No breaking API changes; strictly additive key mappings and bug fixes for modifier leakage.

## Review Gate
- [x] **Gate 0 Decision**: Go with Option A, approved by the user on 2026-11-01.
