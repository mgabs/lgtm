## ADDED Requirements

### Requirement: REQ-MDPERF-1 Efficient Cached Navigation
Uniform-height preview scrolling MUST use constant-time viewport bounds after layout metadata is prepared, including populated uniform heights. Directive-expanded navigation MUST avoid repeated overlapping range scans. Both paths MUST reuse rendered line buffers and preserve cursor visibility, scroll clamping, source attribution, and directive/reply row accounting.

#### Scenario: Uniform rendered rows
- **WHEN** a preview with populated one-line row heights scrolls
- **THEN** viewport bounds are calculated without scanning viewport rows
- **AND** cursor and offset match the existing navigation semantics

#### Scenario: Directive-expanded rows
- **WHEN** a preview containing directive and reply rows moves its cursor or scrolls
- **THEN** expanded heights determine visibility and clamping without reparsing or allocating new rendered line buffers

### Requirement: REQ-MDPERF-2 Coherent Navigation Metadata
Cached navigation metadata MUST reflect current rendered rows and directive heights after document rebuilds, width changes, and directive/reply edits. Empty documents and zero-height viewports MUST NOT panic or permit invalid cursor or offset state.

#### Scenario: Directive height changes
- **WHEN** a directive or reply changes the height of a rendered row
- **THEN** subsequent navigation uses the updated height rather than stale cached bounds

#### Scenario: Reflow and empty viewport
- **WHEN** preview rows are rebuilt at a new width or the viewport has zero height
- **THEN** cursor and offset remain valid for the current rows without panic

### Requirement: NFR-MDPERF-1 Optimized Preview Budgets
Optimized, non-instrumented measurements MUST enforce scrolling below 1 microsecond per operation, parsing/wrapping 5,000 lines below 50 ms, and cached rendering below 16 ms. Scrolling MUST NOT reparse or allocate new line buffers. Existing rendered-memory limits MUST remain unchanged; representative uniform and directive-expanded workloads MUST retain these guarantees.

#### Scenario: Representative scrolling budget
- **WHEN** repeated warmed-up optimized samples measure actual scrolling on uniform and directive-expanded previews
- **THEN** measured per-operation latency satisfies the 1-microsecond budget under the reported sample policy
- **AND** navigation reuses parsed rows and line buffers

#### Scenario: Parsing and rendering budgets
- **WHEN** optimized checks measure a 5,000-line preview and its cached rendering
- **THEN** parsing/wrapping takes less than 50 ms and cached rendering takes less than 16 ms

## Review Gate

- [x] **Gate 2 Approval**: Human approves these specifications before technical design.
