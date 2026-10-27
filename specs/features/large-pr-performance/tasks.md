# Large PR Performance Implementation Plan

> **Goal:** Optimize `lgtm` to handle large PRs (66+ files, 9,000+ lines) with sub-second background load and silky-smooth 60 FPS viewport navigation.

**Architecture:**
- Diff parser capacity hints and zero-alloc header slicing in `crates/lgtm-core/src/pr/mod.rs`.
- Integer hunk hashing, hunk hash caching, and indexed re-anchoring in `crates/lgtm-core/src/review/mod.rs`.
- Line map indexing and streaming fingerprints in `crates/lgtm-core/src/guard/mod.rs` & `crates/lgtm-core/src/findings/mod.rs`.
- Parallel file scanning with `rayon` in `crates/lgtm-core/src/guard/mod.rs`.
- Elimination of redundant hunk hashing in `src/app/pr/completion.rs`.
- Dirty revision counters and memoized drawer counts in `src/app/`, `src/ui/diff/cache.rs`, `src/ui/diff.rs`, `src/ui/diff/viewport.rs`.

**Spec:** [`specs/features/large-pr-performance/design.md`](design.md)

---

### Task 1: Zero-Allocation Hunk Header & Diff Parsing (`lgtm-core::pr`)

- [ ] **Step 1: Implement zero-alloc `parse_hunk_header_fast`**
  In `crates/lgtm-core/src/pr/mod.rs`:
  - Replace `header.split("@@").collect::<Vec<&str>>()` and whitespace splitting with direct byte slice search for `"@@"`.
  - Parse `-old_start,old_lines` and `+new_start,new_lines` directly using `split_once(',')` and numeric `.parse()`.
  - Add unit tests verifying edge cases: missing comma (e.g. `@@ -1 +1 @@`), trailing text, and malformed headers.
- [ ] **Step 2: Capacity pre-sizing in `parse_unified_diff`**
  - Pre-allocate `DiffHunk::lines` with `Vec::with_capacity(new_lines.max(old_lines) as usize)`.
  - Pre-allocate `FileDiff::hunks` with initial capacity 8.
  - Pre-allocate `files` vector in `parse_unified_diff` with capacity 32.
- [ ] **Step 3: Benchmark unified diff parser**
  - Verify throughput on massive diff fixtures (e.g. 10,000 lines).

---

### Task 2: Fast Hunk Hashing & Caching (`lgtm-core::review` & `lgtm-core::model`)

- [ ] **Step 1: Add native `u64` hunk hashing**
  In `crates/lgtm-core/src/review/mod.rs`:
  - Implement `pub fn hunk_hash_u64(h: &DiffHunk) -> u64`.
  - Implement `pub fn format_hunk_hash(hash: u64) -> String`.
  - Keep `pub fn hunk_hash(h: &DiffHunk) -> String` as an inline wrapper for backward compatibility.
  - Verify that `format_hunk_hash(hunk_hash_u64(h)) == hunk_hash(h)` on all existing test hunks.
- [ ] **Step 2: Add cached hash to `DiffHunk`**
  In `crates/lgtm-core/src/model.rs`:
  - Add `#[serde(skip)] pub cached_hash: Option<u64>` to `DiffHunk`.
  - Add helper `impl DiffHunk { pub fn hash_u64(&self) -> u64 }`.
- [ ] **Step 3: Eliminate redundant hashing in `completion.rs`**
  In `src/app/pr/completion.rs`:
  - Use precomputed/cached hunk hashes in the hunk fold loading loop.
  - Avoid recalculating hunk hashes between `file_reviewed` and fold retrieval.

---

### Task 3: Indexed Comment Re-anchoring (`lgtm-core::review`)

- [ ] **Step 1: Implement `HunkIndex`**
  In `crates/lgtm-core/src/review/mod.rs`:
  - Create `struct HunkIndex<'a>` mapping `u64` content hash -> list of candidate hunks.
  - Build `HunkIndex` once per re-anchor pass.
- [ ] **Step 2: Refactor `reanchor_exact` to use `HunkIndex`**
  - Replace full diff scan with $O(1)$ candidate list lookup from `HunkIndex`.
  - Reduce `reanchor_anchors` execution time from $O(C \times F \times H)$ to $O(C + H)$.
- [ ] **Step 3: Vectorized mark matching in `reviewed_flags`**
  - Avoid quadratic iteration over all marks for each hunk; sort or index marks by file and hash.

---

### Task 4: Line Map Lookup & Streamed Guard Findings (`lgtm-core::guard` & `findings`)

- [ ] **Step 1: Implement `DiffLineMap`**
  In `crates/lgtm-core/src/guard/mod.rs`:
  - Implement `DiffLineMap` indexing `(&str, u32) -> &str`.
  - Provide fast lookup for line content by file path and new line number.
- [ ] **Step 2: Stream FNV-1a in `assign_fingerprints`**
  In `crates/lgtm-core/src/guard/mod.rs`:
  - Eliminate `split_whitespace().collect::<Vec<_>>().join(" ")`.
  - Stream tokens directly into an FNV-1a accumulator to assign stable 16-hex fingerprints.
- [ ] **Step 3: Fast `inline_ignored` lookup**
  In `crates/lgtm-core/src/findings/mod.rs`:
  - Use `DiffLineMap` for line retrieval instead of calling linear `line_text`.

---

### Task 5: Parallel Guard Execution with Rayon (`lgtm-core`)

- [ ] **Step 1: Add `rayon` dependency**
  In `crates/lgtm-core/Cargo.toml`:
  - Add `rayon = "1.10"` to workspace dependencies if not present.
- [ ] **Step 2: Parallelize `scan_snapshot`**
  In `crates/lgtm-core/src/guard/mod.rs`:
  - Convert `for file in files` in `SecurityAndTestGuard::scan_snapshot` to `files.par_iter().map(...).reduce(...)`.
  - Merge file alerts and metric counters in parallel.

---

### Task 6: Dirty Revisions & Zero-Alloc TUI Rendering (`src/app` & `src/ui`)

- [ ] **Step 1: Add revision counters to `App`**
  In `src/app/mod.rs`:
  - Add `alerts_revision: u64`, `comments_revision: u64`, `threads_revision: u64`, `marks_revision: u64`.
  - Increment counters on respective state mutations (alerts update, comment CRUD, thread sync, mark toggle).
- [ ] **Step 2: Replace full collection hashing in `annotation_signature`**
  In `src/ui/diff/cache.rs`:
  - Replace string-walking `annotation_signature` with an $O(1)$ compound hash of `(selected_file_idx, alerts_revision, comments_revision, threads_revision)`.
- [ ] **Step 3: Memoize drawer review counts**
  In `src/ui/diff/cache.rs` and `src/ui/diff.rs`:
  - Key `RenderCache::reviewed_source` by `(doc_key, marks_revision)`.
  - Cache per-file reviewed counts so file drawer rendering executes in $O(1)$ per visible item.

---

### Task 7: Verification & Benchmarks

- [ ] **Step 1: Run workspace test suite**
  - Execute `rtk cargo test --workspace` to ensure 100% test compatibility.
- [ ] **Step 2: Run PR completion workload benchmark**
  - Execute `rtk cargo test --bin lgtm pr_completion_workload_benchmark -- --nocapture --ignored`.
  - Verify that `worker_prepare_ms` and `input_draw_p95_ms` meet performance criteria (< 60ms and < 2ms).
- [ ] **Step 3: Manual sanity check with large diff**
  - Verify TUI scrolling and keypress responsiveness on a multi-file diff.
