## 1. Parsing (test-first)

- [x] 1.1 Add failing tests for `parse_edge`: plain, labeled, shaped nodes, chained, unparseable
- [x] 1.2 Implement `parse_edge` and node-label stripping in `src/app/markdown.rs`

## 2. Rendering (test-first)

- [x] 2.1 Add failing tests: header `[mermaid: <type>]`, flowchart edge rows, sequenceDiagram verbatim fallback, unparseable line verbatim, diff styling and line numbers kept
- [x] 2.2 Track fence language and render mermaid blocks in `build_markdown_preview`

## 3. Docs and verification

- [x] 3.1 Mention Mermaid in the `m` row of `docs/WIKI.md`; add a CHANGELOG entry
- [x] 3.2 Run `cargo fmt`, `cargo clippy --workspace --all-targets`, `cargo test --workspace`
- [x] 3.3 Run `openspec validate --all --strict`
