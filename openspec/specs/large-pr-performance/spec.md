# large-pr-performance Specification

## Purpose

Keep review of large PRs (tens of files, thousands of lines) responsive by bounding allocations and algorithmic cost in diff parsing, hashing, anchoring, guard analysis and TUI rendering, without changing observable results.

## Requirements

### Requirement: Allocation-Lean Unified Diff Parsing
Unified diff parsing SHALL avoid temporary heap allocations in hunk header and line handling: hunk headers MUST be parsed without intermediate slice vectors, and hunk line and file collections MUST be pre-sized from known counts. Output snapshots MUST be identical to those produced before the optimization.

#### Scenario: Parsing is behavior-preserving
- **WHEN** the existing diff fixtures are parsed
- **THEN** the resulting snapshots equal the previous outputs

#### Scenario: Parsing budget
- **WHEN** a 66-file, 9,056-line diff is parsed
- **THEN** parsing completes in under 20 ms

### Requirement: Stable Cached Hunk Hashing
Hunk hashing SHALL be available as a native 64-bit value and a formatted string, each hunk MUST compute its content hash at most once, and the algorithm (FNV-1a 64-bit over whitespace-normalized changed lines) MUST remain byte-for-byte identical so existing `.lgtm/state.json` hunk marks stay valid.

#### Scenario: Hash compatibility
- **WHEN** the same hunk is hashed via the integer and string forms
- **THEN** the formatted integer equals the string form
- **AND** both equal the hash produced by the previous implementation

### Requirement: Indexed Directive Re-anchoring
Re-anchoring directives against a diff SHALL use a hash-to-hunk and file-path index so cost is O(comments + hunks) rather than nested scans. A comment on a file absent from the diff MUST skip scanning immediately, and reviewed-mark lookups MUST NOT be quadratic. Anchoring results MUST equal the unindexed behavior.

#### Scenario: Anchoring fidelity
- **WHEN** directives are re-anchored against a changed diff
- **THEN** each anchors to exactly the line the previous engine chose

#### Scenario: Anchoring budget
- **WHEN** 1,000 directives are re-anchored against the 66-file diff
- **THEN** re-anchoring completes in under 60 ms

### Requirement: Single-Pass Line Lookup and Streamed Fingerprints
Guard finding processing SHALL build one line lookup per diff snapshot and answer line-text queries in O(1) instead of scanning files, hunks and lines. Finding fingerprints MUST be computed by streaming tokens into the hasher without intermediate collected strings, and fingerprint values MUST be unchanged.

#### Scenario: Fingerprint stability
- **WHEN** findings from a fixed diff are fingerprinted
- **THEN** the fingerprints equal those from the previous implementation

### Requirement: Parallel Guard Analysis
Per-file guard scanning SHALL run concurrently across independent files, merging per-file metrics and alerts deterministically. Guards that need a global snapshot view MUST run after the per-file analysis completes.

#### Scenario: Deterministic results
- **WHEN** the same snapshot is scanned repeatedly
- **THEN** the merged alerts and metrics are identical on every run

#### Scenario: Analysis budget
- **WHEN** the 66-file diff is scanned
- **THEN** guard analysis completes in under 150 ms

### Requirement: Revision-Counter Render Invalidation
The TUI SHALL maintain monotonic revision counters for alerts, comments, threads and review marks, incremented only when the underlying state changes, and MUST derive its diff annotation cache signature from them in O(1) without traversing collections. Steady-state frames MUST NOT allocate.

#### Scenario: Unchanged state skips rebuild
- **WHEN** a key press changes only the cursor
- **THEN** annotation and viewport caches are reused without rebuilding

#### Scenario: Change invalidates
- **WHEN** a comment is added
- **THEN** the comments revision increments and the annotation cache is rebuilt on the next frame

#### Scenario: Frame budget
- **WHEN** a key press is rendered on the 66-file diff
- **THEN** the P95 render time is under 2 ms

### Requirement: Memoized Drawer Reviewed Counts
The file drawer SHALL read per-file reviewed counts from a cache invalidated only when review marks or the active diff document change, answering each visible file in O(1) per frame.

#### Scenario: Cache reuse
- **WHEN** the drawer renders repeatedly with no mark changes
- **THEN** reviewed counts are not recomputed

#### Scenario: Cache invalidation
- **WHEN** a hunk mark is toggled
- **THEN** the drawer's reviewed counts are recomputed on the next render

### Requirement: Large PR Preparation Budget
Total PR preparation on a worker thread SHALL complete in under 250 ms for the 66-file, 9,056-line reference diff, and the offline benchmark with 1,000 directives and 2,000 lines MUST complete in under 300 ms.

#### Scenario: Reference benchmark
- **WHEN** the PR completion workload benchmark runs with 1,000 directives and 2,000 lines
- **THEN** it completes in under 300 ms
