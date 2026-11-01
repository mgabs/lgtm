# Research & Technical Tradeoffs

## 1. Intent & Ground Truth

Render Mermaid diagrams inside the Markdown preview (`m`) as text, so reviewers reading `design.md`/`README.md` changes see diagrams instead of raw ```` ```mermaid ```` source.

Verified:
- The preview parser is hand-rolled in `src/app/markdown.rs` (no `pulldown-cmark`/`comrak`). A ```` ```mermaid ```` fence currently renders as a verbatim code block with a `[mermaid]` header; nothing in the repo handles Mermaid.
- Preview budgets (spec `markdown-preview`): cached render < 16 ms; parse and wrap of 5,000 lines < 50 ms; scrolling must not re-parse or allocate (REQ-MDPERF-1). Rows carry source-line attribution for directives and the diff overlay.
- Spike on this repo's own diagrams (release build, scratch project, since removed): `mermaid-text` 0.57 produced about 200 columns for a 13-node flowchart despite a 100-column budget, with overlapping edges, and has no sequence support. `merman` 0.9.3 (`ascii` feature) rendered the same diagram correctly at 226x25, a 7-node DAG at 183x27 and a sequence diagram at 34x10, in 0.6 to 10 ms each. With `OverflowPolicy::Error` it refuses over-wide output with an explicit error; `Fallback` emits a 457-line structured dump that is not suitable for display.
- `merman` license MIT OR Apache-2.0, MSRV 1.95 (toolchain here is 1.98); larger dependency tree (`merman-core`, `serde`, `serde_json`, `unicode-*`, optional `dugong`). `mermaid-text`: MIT, small tree (`ascii-dag`, `chrono`, `unicode-width`). Both are under a year old.
- `lgtm_core` must stay free of external API and I/O dependencies; Mermaid rendering is presentation, so it belongs in the `lgtm` UI crate.
- Related open change `markdown-parser-coverage` only adds tests for the existing parser and does not overlap in behavior.

## 2. Options Explored

**A. `merman` (feature-gated) with horizontal pan and source fallback (recommended).** Render supported diagrams to cached text rows per width using `AsciiRequest` with `OverflowPolicy::Error`, pin the version, enable only `ascii` plus needed families (`diagram-flowchart`, `diagram-sequence`, optionally state). When the diagram is wider than the viewport, offer horizontal pan over the cached rows; on any error or unsupported family keep today's source block.
- Pros: correct layout, sequence/state support, deterministic text, millisecond cost.
- Cons: bigger dependency tree, MSRV 1.95, pan UX to design.

**B. `mermaid-text`.** Smaller tree, flowchart only.
- Pros: tiny, simple API.
- Cons: ignores width, overlapping edges on dense graphs, no sequence; fallback heuristics would hide most real diagrams.

**C. Keep source blocks (status quo) or browser/image export.**
- Pros: zero risk (status quo).
- Cons: no diagram value; images and a browser handoff do not work over tmux or ssh and add moving parts.

## 3. Tradeoff Matrix

| Criteria | A: merman | B: mermaid-text | C: status quo |
| :--- | :--- | :--- | :--- |
| **Complexity** | Medium (cache, pan, fallback) | Low-medium (needs width heuristic) | None |
| **New Dependencies** | `merman` tree, needs audit | 3 small crates | None |
| **Performance & Latency** | 0.6 to 10 ms per diagram, cached per width | Similar | None |
| **Maintenance Burden** | Medium (young crate, pin and review) | Medium (limited coverage) | None |
| **Blast Radius** | `src/app/markdown.rs`, `src/ui/markdown.rs`, `Cargo.toml` | Same | None |

## 4. Recommendation

Option A, behind a Cargo feature, with source-block fallback so the feature can never make the preview worse; require a dependency audit and a pinned version before merge.

## 5. Breaking Changes & Deprecations

- `markdown-preview` spec, "Markdown Reflow and Styling": MODIFIED (a `mermaid` fence renders as a diagram when supported and fits or pans; otherwise as a code block).
- `markdown-preview` spec, "Preview Navigation and Scroll Parity": MODIFIED (horizontal pan keys; cursor to source-line mapping preserved for diagram rows).
- `markdown-preview` spec, "Preview Performance Budgets" and `REQ-MDPERF-1`: unchanged in budget, extended to cover diagram rows (cached per width, no re-render on scroll).
- `markdown-preview` spec, "Preview Edge Cases": MODIFIED (malformed Mermaid, unsupported family and over-wide output must fall back without panic or hang).
- No requirement is removed; no change to `comments` or the thread model.

Open questions for Gate 0: pan keys (`h`/`l` conflict check against existing preview keys); whether the feature is on by default; which diagram families to enable first; dependency audit tool (`cargo deny` or `cargo audit` is not currently configured in CI).

## Review Gate
- [ ] **Gate 0 Decision**: Human decides Go (name the option) or No-Go before the proposal is drafted.
