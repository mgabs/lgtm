## Context

`build_markdown_preview` in `src/app/markdown.rs` tracks `in_code_fence`; the fence start row prints `[<lang>]` and body lines render verbatim with a code background. Mermaid is currently just another language tag. No renderer for it exists in the repo or on PATH assumptions.

## Goals / Non-Goals

**Goals:** make flowchart diffs readable in the terminal with zero dependencies; never hide content.
**Non-goals:** graphical or box-and-arrow layout, external tools, non-flowchart parsing, subgraphs.

## Decisions

- Track the fence language (new `fence_lang` next to `in_code_fence`). When it is `mermaid`, and the first non-empty line starts with `graph` or `flowchart`, switch to flowchart mode; otherwise code mode.
- Parse edges with a small hand-written function `parse_edge(line) -> Option<(from, to, label)>`: split on the first of `-->`, `---`, `-.->`, `==>`; strip node shape syntax (`[x]`, `(x)`, `{x}`, `((x))`) to the label or id. Returns `None` for anything else, which falls back to verbatim. Rejected: a Mermaid parser crate (new dependency, large) and shelling out to `mmdc`/`mermaid-ascii` (not installed, breaks the offline, cross-platform guarantee).
- One source line yields one rendered row, preserving `line_number` and diff styling, so gutter attribution and comments keep working.
- Chained edges (`A --> B --> C`) split into consecutive rows on the same source line number.

## Risks / Trade-offs

- Mermaid syntax is large; the parser covers common arrows only. Unknown syntax is shown verbatim, so the worst case equals today's behavior.
- Chained edges produce several rows for one source line; the existing cursor/line mapping uses the first, acceptable.
- Hot-path budget: parsing runs once per rebuild, linear in block size.
