# Specification: Large PR Performance Optimization

## Capability: `large-pr-performance`

### 1. Requirements

#### REQ-LPR-01 (Amortized Allocations & Zero-Alloc Unified Diff Parsing)
- `PrEngine::parse_unified_diff` MUST eliminate temporary heap allocations during hunk and header parsing:
  1. Hunk header extraction (`parse_hunk_header`) MUST NOT allocate intermediate `Vec<&str>` slices. It MUST parse range integers (`-old_start,old_lines +new_start,new_lines`) directly from byte/char slices using zero-alloc iterators.
  2. Diff hunk line vector (`DiffHunk::lines`) MUST pre-allocate capacity based on `new_lines.max(old_lines) as usize` to eliminate dynamic vector resizing during line parsing.
  3. Diff file collection (`DiffSnapshot::files`) MUST pre-allocate based on an estimated file count or dynamic growth hints.
  4. Header string storage MUST slice directly without superfluous allocations.

#### REQ-LPR-02 (Fast Hunk Hashing & Native u64 Representations)
- Core hunk hashing (`lgtm_core::review::hunk_hash`) MUST support native integer hashing without requiring heap `String` allocation:
  ```rust
  pub fn hunk_hash_u64(h: &DiffHunk) -> u64;
  pub fn format_hunk_hash(hash: u64) -> String;
  pub fn hunk_hash(h: &DiffHunk) -> String; // backward-compatible wrapper
  ```
- Every `DiffHunk` MUST cache its computed content hash once upon construction or upon first access, preventing repeated byte-level hashing of the same lines:
  ```rust
  pub struct DiffHunk {
      pub header: String,
      pub old_start: u32,
      pub old_lines: u32,
      pub new_start: u32,
      pub new_lines: u32,
      pub lines: Vec<DiffLine>,
      pub is_folded: bool,
      pub folded_blocks: Vec<BlockRange>,
      #[serde(skip)]
      pub content_hash: Option<u64>,
  }
  ```
- The hash calculation algorithm (FNV-1a 64-bit on whitespace-normalized changed lines) MUST remain 100% byte-for-byte identical with previous implementations to preserve compatibility with existing `.lgtm/state.json` hunk marks.

#### REQ-LPR-03 (Indexed Comment Re-anchoring)
- Re-anchoring comments against diff files (`reanchor_anchors` and `reanchor_exact`) MUST NOT perform $O(C \times F \times H)$ nested loops:
  1. `reanchor` MUST construct a single hash-to-hunk multi-map index before scanning comments:
     ```rust
     pub struct HunkIndex<'a> {
         // Maps u64 content hash -> list of (file_index, hunk_index)
         pub by_hash: std::collections::HashMap<u64, SmallVec<[HunkRef<'a>; 2]>>,
         // Maps file_path -> file_index for O(1) file lookups
         pub by_file: std::collections::HashMap<&'a str, usize>,
     }
     ```
  2. Exact re-anchoring (`reanchor_exact`) MUST lookup candidate hunks via `HunkIndex::by_hash` in $O(1)$ time, reducing complexity from $O(C \times F \times H)$ to $O(C + H)$.
  3. Comments referencing files not present in `HunkIndex::by_file` MUST skip re-anchoring immediately without scanning any hunks.
  4. `reviewed_flags` MUST use hash-based lookup or sorted vector scanning, eliminating quadratic comparisons between file hunks and review marks.

#### REQ-LPR-04 (Line Map Lookup & Streamed Finding Fingerprints)
- The guard finding pipeline MUST eliminate repeated full diff traversals for line content lookups:
  1. `DiffLineMap` MUST be constructed once for a diff snapshot:
     ```rust
     pub struct DiffLineMap<'a> {
         // (file_path, new_line_num) -> line content &str
         map: std::collections::HashMap<(&'a str, u32), &'a str>,
     }
     ```
  2. `line_text` queries in `assign_fingerprints` and `inline_ignored` MUST query `DiffLineMap` in $O(1)$ time instead of scanning every file, hunk, and line.
  3. `assign_fingerprints` MUST stream tokens into the FNV-1a hasher directly without intermediate `split_whitespace().collect::<Vec<_>>().join(" ")` vector allocations and without temporary intermediate strings.

#### REQ-LPR-05 (Parallel Guard Analysis via Rayon)
- File-level guard scanning in `SecurityAndTestGuard::scan_snapshot` MUST execute concurrently using `rayon`:
  1. Independent files in `files: &[FileDiff]` MUST be partitioned and scanned in parallel using `par_iter()`.
  2. Aggregated file metrics (`total_logic_lines`, `total_assert_additions`) and alert lists MUST be merged via standard parallel fold/reduce.
  3. Snapshot-level guards that require global view (`registry().scan_snapshot`) MUST execute after per-file parallel analysis completes.

#### REQ-LPR-06 (Dirty Revision Counters for TUI Rendering)
- The TUI diff view MUST eliminate per-frame collection traversals and hashing:
  1. `App` MUST maintain monotonic 64-bit revision counters:
     ```rust
     pub struct App {
         // ...
         pub alerts_revision: u64,
         pub comments_revision: u64,
         pub threads_revision: u64,
         pub marks_revision: u64,
     }
     ```
  2. The revision counters MUST be incremented only when underlying state changes:
     - `alerts_revision`: upon guard scanning completion or finding status changes.
     - `comments_revision`: upon adding, editing, deleting, replying, or resolving comments.
     - `threads_revision`: upon fetching, replying, or resolving forge threads.
     - `marks_revision`: upon toggling hunk marks or file reviewed marks.
  3. `annotation_signature(app)` MUST be replaced with an $O(1)$ composite signature:
     ```rust
     pub fn annotation_signature(app: &App) -> u64 {
         let mut hasher = std::collections::hash_map::DefaultHasher::new();
         app.selected_file_idx.hash(&mut hasher);
         app.alerts_revision.hash(&mut hasher);
         app.comments_revision.hash(&mut hasher);
         app.threads_revision.hash(&mut hasher);
         hasher.finish()
     }
     ```
  4. Viewport and render caches MUST check the compound revision in $O(1)$ without allocating or traversing any elements.

#### REQ-LPR-07 (Memoized Drawer Reviewed Counts)
- File drawer rendering MUST NOT recalculate `reviewed_flags` on every frame:
  1. `RenderCache` MUST cache per-file reviewed counts keyed by `marks_revision`:
     ```rust
     pub struct RenderCache {
         // ...
         reviewed_source: Option<(Key, u64)>, // (doc_key, marks_revision)
         reviewed_counts: Vec<Option<usize>>, // indexed by file_idx
     }
     ```
  2. Invalidation of drawer counts occurs ONLY when `marks_revision` changes or the active diff document changes.
  3. File drawer line generation queries the cached array in $O(1)$ time per visible file.

---

### 2. Performance Benchmarks & Targets

| Operation | Baseline (66 files, 9056 lines) | Target (Optimized) | Constraint |
|:---|:---|:---|:---|
| **Diff Parsing** (`parse_unified_diff`) | ~85ms | **< 20ms** | 0 heap vector reallocations |
| **Guard Analysis** (`scan_snapshot` + `findings`) | ~1,850ms | **< 150ms** | Full parallel utilization via Rayon |
| **Directive Re-anchoring** (1000 comments) | ~1,400ms | **< 60ms** | Indexed $O(C + H)$ lookup |
| **Total PR Preparation** (`completion::prepare`) | ~3,200ms | **< 250ms** | Worker thread execution |
| **Keypress Render P95** (`ui::render`) | ~18ms (frame drops) | **< 2ms** | Steady 60 FPS ($O(1)$ dirty revisions) |
| **Per-Frame Allocations** | ~45 KB / frame | **0 B / frame** | Zero heap allocs in steady state |

---

### 3. Acceptance Criteria

- [ ] `PrEngine::parse_unified_diff` passes all existing test suites without changing DiffSnapshot outputs.
- [ ] `hunk_hash_u64` generates identical hex strings as `hunk_hash` for any given diff hunk.
- [ ] Comment re-anchoring preserves 100% fidelity: any comment anchored to a hunk in the legacy engine anchors to the exact same line in the optimized engine.
- [ ] Offline benchmark (`pr_completion_workload_benchmark`) with 1000 directives and 2000 lines executes in < 300ms total.
- [ ] TUI keypress handling and diff navigation exhibits zero observable stutter on a 66-file diff.
- [ ] `cargo check --workspace` and `cargo test --workspace` pass with zero compiler warnings.
