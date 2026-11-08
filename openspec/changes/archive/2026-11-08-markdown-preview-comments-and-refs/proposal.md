# Proposal

## Intent

Equip the rendered Markdown preview view (`ActiveView::MarkdownPreview`) with interactive parity to the diff view by enabling agent `#ref` injection on `Enter`, multi-key leader comment templates on `c`, reliable cursor-synced replies on `r` with persistent preview view return, threads modal access via `t`, and inline rendering of PR discussion threads.

## Problem Statement

When reviewers switch to Markdown preview mode to read rendered documentation diffs as formatted prose, key review interactions break:
1. Pressing `Enter` is ignored, preventing reviewers from sending line references (`#ref file:line`) to the agent sink.
2. Pressing `r` evaluates line indexes before syncing the preview cursor, targeting whichever line the diff view last pointed to, and submitting or canceling a reply hardcodes a return to `ActiveView::Diff`.
3. Pressing `c` directly opens an unformatted directive input, missing the `cp`, `cw`, `cs`, `cr`, `ca`, and `cc` quick-template leader prefixes available in diff view.
4. Active PR discussion threads (`app.threads`) are invisible in the preview layout, and the `t` shortcut to inspect threads is unbound.

## Capabilities

### Modified Capabilities
- `markdown-preview`: Add `Enter` agent `#ref` injection with wrapped continuation line resolution, `c` multi-key leader template support, cursor-synced `r` replying with preview view return, `t` thread modal delegation, and inline rendering of PR discussion threads alongside local directives.

## Scope

### In-Scope
- **Agent Reference Injection (`Enter`)**: Resolve the source line number (scanning backward to the paragraph/block start if on a wrapped row) and call `inject_reference_to_agent()`.
- **Comment Leader Sequence (`c`)**: Implement `pending_leader = Some('c')` in `handle_markdown_preview_key` with prefixes for `cp`, `cw`, `cs`, `cr`, `ca`, and `cc`.
- **Thread/Directive Replies (`r`)**: Synchronize preview cursor to diff line before checking `thread_at_cursor()` and `comment_idx_at_cursor()`.
- **View Return Continuity**: Set `comment_source_view = ActiveView::MarkdownPreview` during comment and reply initialization, and restore `self.comment_source_view` in `handle_comment_reply_key`.
- **Inline PR Threads Rendering**: In `src/ui/markdown.rs`, query and render PR threads and their replies under the matching anchor lines, styled distinctly from local directives.
- **Thread Modal Access (`t`)**: Delegate to `open_threads_modal()` from preview mode.

### Out-of-Scope (YAGNI)
- Interactive mouse clicking on inline comments/threads.
- Editing existing thread replies inside preview mode.
- Changing `lgtm-core` neutral thread structures or forge network logic.
- Adding third-party markdown extensions or changing CommonMark rendering rules.

## Assumptions
- **ASM-GEN-1**: Rendered markdown rows for wrapped lines (`line_number: None`) belong to the nearest preceding row with `line_number: Some(n)` within the same block.
- **ASM-GEN-2**: Existing thread and directive line numbers match 1-based source lines in `file.hunks`.
- **ASM-GEN-3**: Rendering PR threads inline reuses the existing variable row height caching system in `MarkdownPreviewState` without violating the 16ms frame render budget.

## Success Metrics
- Keystroke latency in markdown preview remains < 1 µs.
- `Enter` on any rendered markdown row (including wrapped rows) successfully dispatches `#ref <path>:<line>` to the configured agent sink.
- Submitting or cancelling a reply initiated from markdown preview returns to markdown preview mode.
- PR threads and replies render inline under corresponding source lines in preview mode.
- All unit, integration, and BDD tests pass with >= 95% line and branch coverage on modified modules.

## Review Gate
- [x] **Gate 1 Approval**: Proposal reviewed and approved by human before drafting specs.
