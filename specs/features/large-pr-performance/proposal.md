# Feature Proposal: Large PR Performance Optimization

## Intent
Eliminate latency spikes, CPU saturation, and frame drops when inspecting and reviewing large pull requests (e.g., 66+ files, 9,000+ changed lines) by replacing quadratic algorithms, repeated allocations, and per-frame full-scan hashing with indexed lookups, zero-alloc primitives, parallel analysis, and dirty-state revisions.

## Scope

### In-Scope
- **Diff Parsing (`lgtm-core::pr`)**:
  - Pre-allocate line and hunk vectors using diff header size hints.
  - Zero-allocation hunk header parsing via direct integer extraction instead of intermediate vector allocations.
  - Efficient string slicing and amortized allocations for massive diff streams.
- **Hunk Hashing & Re-anchoring (`lgtm-core::review`)**:
  - Transition internal hunk hashing to native `u64` values and amortize hex formatting.
  - Hunk hash caching on `DiffHunk` or cached map to eliminate repeated byte hashing.
  - Multi-map hunk index for re-anchoring to reduce exact re-anchoring complexity from $O(C \times F \times H)$ to $O(C + H)$.
  - Vectorized or hash-indexed mark pairing in `reviewed_flags` to avoid nested comparisons.
- **Finding Pipeline & Line Indexing (`lgtm-core::guard` & `findings`)**:
  - Replace repeated $O(\text{Alerts} \times \text{Lines})$ linear searches in `line_text` with an indexed `LineMap` built once per diff reload.
  - Parallelize file-level guard scanning across CPU cores via `rayon`.
  - Stream byte chunks in `assign_fingerprints` to avoid intermediate whitespace joins and string allocations.
- **Background PR Completion Pipeline (`src/app/pr/completion.rs`)**:
  - Eliminate redundant hunk hashing passes between `file_reviewed` checks and fold loading.
  - Parallelize or batch directive re-anchoring across hunks.
- **TUI Frame Rate & Cache Revisions (`src/ui/diff/`)**:
  - Replace `annotation_signature(app)` (which traverses and hashes all alerts, comments, and threads twice per frame) with dirty revision counters (`alerts_revision`, `comments_revision`, `threads_revision`, `marks_revision`).
  - Cache drawer `reviewed_count` results against `marks_revision` to eliminate per-frame hunk evaluations across 66+ files.

### Out-of-Scope (YAGNI)
- Virtualized diff streaming over network / paged git fetching (local diff snapshot already fits in memory).
- GPU-accelerated syntax rendering (TUI terminal bandwidth is CPU bound; dirty render caching is sufficient).
- Custom memory allocators (e.g. jemalloc / mimalloc) unless standard heap profiling warrants it.
- Rewriting the diff algorithm itself (git and `similar` crate remain standard).

## Problem Statement
On large pull requests (e.g., 66 files, +3122 -5934 lines), `lgtm` exhibits extreme CPU churn and UI freezes:
1. **Diff Parsing Allocation Bloat**: `parse_unified_diff` allocates thousands of tiny heap strings and intermediate vectors for hunk headers and line contents without capacity hints, causing allocator thrashing during diff load.
2. **Quadratic Hunk Hashing & Re-anchoring**: `hunk_hash` formats a 16-hex heap `String` on every invocation. In `reanchor_anchors` and `reanchor_exact`, every comment traverses every hunk across every file, recalculating hunk hashes repeatedly ($O(C \times F \times H)$).
3. **Repeated Linear Scans in Guard Pipeline**: `assign_fingerprints` and `inline_ignored` call `line_text` for every alert, triggering nested loops over all files and hunk lines ($O(\text{Alerts} \times \text{Lines})$). For hundreds of findings across 9,000 lines, this consumes seconds of CPU time sequentially.
4. **Per-Frame Hash Traversal on Every Keypress**: `annotation_signature(app)` is executed twice per frame (viewport and drawer) on every keystroke, walking and hashing strings for every comment, alert, and thread.
5. **Drawer Review Calculation Churn**: Rendering the file drawer evaluates `reviewed_count` for all visible files, running full hunk mark comparisons on every tick.

## Goals, Non-Goals, and Anti-Goals

### Goals
- **Sub-Second PR Load**: Complete diff parsing, guard scanning, and comment re-anchoring for a 66-file / 9,000-line PR in < 500ms on modern multi-core hardware.
- **Steady 60 FPS Viewport Navigation**: Eliminate all per-frame full-collection hashing; reduce render preparation to $O(1)$ integer comparisons using dirty revision counters.
- **Deterministic Memory Footprint**: Eliminate temporary intermediate allocations in hot paths (hashing, re-anchoring, line lookups).
- **Zero Regression**: Preserve 100% correctness of hunk marks, comment re-anchoring, line ignores, and guard findings.

### Non-Goals (YAGNI)
- Implementing asynchronous chunked line rendering inside individual diff views.
- Changing the on-disk format of `.lgtm/state.json` or `.lgtm/comments.jsonl` (maintain full backward compatibility).
- Multi-threaded rendering in Ratatui (Ratatui rendering remains single-threaded; compute offloading belongs in worker jobs).

### Anti-Goals
- Never trade re-anchoring accuracy for speed: comments must still strictly snap to matching content hashes or fallback cleanly to fuzzy matching.
- Never drop findings or skip security guards on large diffs to save time.

## Success Metrics
- **PR Load & Prepare Latency**: Drops from > 3,500ms to < 400ms on a 66-file / 9,000-line changeset.
- **Keypress Input-to-Draw P95 Latency**: Stays strictly below 8ms (well within a 16ms 60 FPS frame window).
- **Hunk Hash Calculations**: Number of hunk hash computations reduced by > 90% (computed at most once per hunk).
- **Zero Allocations in Render Loop**: `annotation_signature` replaced by $O(1)$ dirty revision checks with 0 bytes allocated per frame.
