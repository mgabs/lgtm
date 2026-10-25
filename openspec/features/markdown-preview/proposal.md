# Change Proposal: Markdown Preview with Diff Highlighting

## Intent
Provide an in-terminal, rendered markdown preview mode for markdown documents (`.md`, `.markdown`) in `lgtm`, enabling reviewers to view formatted documents reflowed to screen width with inline diff highlights (+green additions, -red deletions).

## Scope

### In-Scope
- Hotkey toggle (e.g. `m`) between raw unified/split diff and rendered markdown preview when reviewing markdown files.
- Terminal-width text reflow for headings, paragraphs, lists, and code blocks.
- Markdown styling in Ratatui:
  - Headers rendered with bold, distinct levels, and color accents.
  - Bulleted and numbered lists cleanly indented.
  - Fenced code blocks styled with background contrast and language headers.
- Diff overlay:
  - Additions rendered inline with green text tint and `+` gutter marker.
  - Deletions rendered inline with red text tint and `-` gutter marker.
  - Unchanged context rendered with standard markdown styling.
- Navigation parity:
  - Scrolling (`j`/`k`, `PageUp`/`PageDown`, mouse wheel) through the rendered preview.
  - Returning to raw diff preserves cursor/scroll position where feasible.

### Out-of-Scope (YAGNI)
- External browser-based rendering or HTML exports.
- Complex diagram rendering (e.g. ASCII-art rendering of Mermaid graphs; mermaid blocks remain syntax-highlighted code fences).
- Inline comment creation directly inside rendered markdown preview in the initial version (leave comments in the diff view).

## Problem Statement
When reviewing changes to architectural designs, RFCs, and documentation in `lgtm`, reading raw markdown diffs forces reviewers to parse raw markdown syntax tokens (`#`, `**`, ````mermaid`, long unwrapped lines) alongside git diff markers. This makes it difficult to assess formatting, flow, and visual readability of documentation changes.

## Success Metrics
- **Instant Toggle**: Switching between raw diff and rendered markdown preview takes < 50ms for documents up to 5,000 lines.
- **Visual Clarity**: Headings, lists, code blocks, and emphasized text render formatted to the terminal width without line overflow or clipping.
- **Diff Accuracy**: All changed hunks in the markdown document clearly display their additions in green (`+`) and deletions in red (`-`), matching git diff content exactly.
