## Why

Documentation diffs often contain ```` ```mermaid ```` blocks. The preview shows them as an anonymous `[mermaid]` code block, so reviewers must mentally parse the syntax to see what a diagram change means.

## What Changes

- Recognize `mermaid` fences in the Markdown preview and label them with the diagram type (e.g. `[mermaid: flowchart]`, `[mermaid: sequenceDiagram]`), styled apart from ordinary code.
- For `graph`/`flowchart` diagrams, render each edge as a readable line (`A ──▶ B`, with `|label|` text kept), listed beneath the header; node labels (`A[Label]`) show as `Label`.
- Every other diagram type, and any line that cannot be parsed, falls back to verbatim source so nothing is hidden.
- Diff overlay and gutter attribution stay per source line.

Non-goals: pixel/graphical rendering, external renderers (`mmdc`, `mermaid-ascii`), layout of boxes and arrows, subgraph nesting, other diagram types beyond the labeled fallback. Affected crate: `lgtm` (`src/app/markdown.rs`) only.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `markdown-preview`: adds a Mermaid block requirement.

## Impact

- `src/app/markdown.rs`, `docs/WIKI.md`, `CHANGELOG.md`. No new dependencies; behavior is identical on ubuntu, macos and windows.
