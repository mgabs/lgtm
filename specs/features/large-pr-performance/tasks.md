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

- [x] **Step 1: Implement zero-alloc `parse_hunk_header_fast`**
  In `crates/lgtm-core/src/pr/mod.rs`:
  - Replace `header.split("@@").collect::<Vec<&str>>()` and whitespace splitting with direct byte slice search for `"@@"`.
  - Parse `-old_start,old_lines` and `+new_start,new_lines` directly using `split_once(',')` and numeric `.parse()`.
  - Add unit tests verifying edge cases: missing comma (e.g. `@@ -1 +1 @@`), trailing text, and malformed headers.
- [x] **Step 2: Capacity pre-sizing in `parse_unified_diff`**
  - Pre-allocate `DiffHunk::lines` with `Vec::with_capacity(new_lines.max(old_lines) as usize)`.
  - Pre-allocate `FileDiff::hunks` with initial capacity 8.
  - Pre-allocate `files` vector in `parse_unified_diff` with capacity 32.
- [x] **Step 3: Benchmark unified diff parser**
  - Verify throughput on massive diff fixtures (e.g. 10,000 lines).

---

### Task 2: Fast Hunk Hashing & Caching (`lgtm-core::review` & `lgtm-core::model`)

- [x] **Step 1: Add native `u64` hunk hashing**
  In `crates/lgtm-core/src/review/mod.rs`:
  - Implement `pub fn hunk_hash_u64(h: &DiffHunk) -> u64`.
  - Implement `pub fn format_hunk_hash(hash: u64) -> String`.
  - Keep `pub fn hunk_hash(h: &DiffHunk) -> String` as an inline wrapper for backward compatibility.
  - Verify that `format_hunk_hash(hunk_hash_u64(h)) == hunk_hash(h)` on all existing test hunks.
- [x] **Step 2: Add cached hash to `DiffHunk`**
  In `crates/lgtm-core/src/model.rs`:
  - Add helper `impl DiffHunk { pub fn hash_u64(&self) -> u64 }`.
- [x] **Step 3: Eliminate redundant hashing in `completion.rs`**
  In `src/app/pr/completion.rs`:
  - Avoid redundant fold hashing when `state.folds.is_empty()`.

---

### Task 3: Indexed Comment Re-anchoring (`lgtm-core::review`)

- [x] **Step 1: Implement `HunkIndex`**
  In `crates/lgtm-core/src/review/mod.rs`:
  - Create `struct HunkIndex<'a>` mapping content hash -> list of candidate hunks.
  - Build `HunkIndex` once per re-anchor pass.
- [x] **Step 2: Refactor `reanchor_exact` to use `HunkIndex`**
  - Replace full diff scan with $O(1)$ candidate list lookup from `HunkIndex`.
  - Reduce `reanchor_anchors` execution time from $O(C \times F \times H)$ to $O(C + H)$.
- [x] **Step 3: Vectorized mark matching in `reviewed_flags`**
  - Filter marks by file before checking hunk content hashes.

---

### Task 4: Line Map Lookup & Streamed Guard Findings (`lgtm-core::guard` & `findings`)

- [x] **Step 1: Implement `DiffLineMap`**
  In `crates/lgtm-core/src/guard/mod.rs`:
  - Implement `DiffLineMap` indexing `(&str, u32) -> &str`.
  - Provide fast lookup for line content by file path and new line number.
- [x] **Step 2: Stream FNV-1a in `assign_fingerprints`**
  In `crates/lgtm-core/src/guard/mod.rs`:
  - Use `DiffLineMap` for $O(1)$ line lookup instead of linear hunk scans.
- [x] **Step 3: Fast `inline_ignored` lookup**
  In `crates/lgtm-core/src/findings/mod.rs`:
  - Use `DiffLineMap` for line retrieval in `apply` instead of calling linear `line_text`.

---

### Task 5: Parallel Guard Execution with Scoped Threads (`lgtm-core`)

- [x] **Step 1: Zero-dependency parallelization with `std::thread::scope`**
  - Use native scoped threads across available parallelism chunks.
- [x] **Step 2: Parallelize `scan_snapshot`**
  In `crates/lgtm-core/src/guard/mod.rs`:
  - Partition files across worker threads and aggregate alerts and metrics in parallel.

---

### Task 6: Zero-Alloc TUI Rendering Optimization (`src/ui`)

- [x] **Step 1: Scoped alert hashing in `annotation_signature`**
  In `src/ui/diff/cache.rs`:
  - Only hash alerts and threads belonging to the currently selected file instead of the entire multi-file PR.
- [x] **Step 2: Lightweight signature for file drawer**
  In `src/ui/diff/cache.rs`:
  - Retain minimal mark representation in `prepare_reviewed`.

---

### Task 7: Verification & Benchmarks

- [x] **Step 1: Run workspace test suite**
  - Execute `rtk cargo test --workspace` to ensure 100% test compatibility.
- [x] **Step 2: Run PR completion workload benchmark**
  - Execute `rtk cargo test --bin lgtm pr_completion_workload_benchmark -- --nocapture --ignored`.
- [x] **Step 3: Clippy & formatting verification**
  - Check zero warnings with `rtk cargo clippy --workspace --all-targets --all-features -- -D warnings`.
