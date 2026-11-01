# Proposal

## Intent

Separate optimized performance measurement from behavioral coverage and optimize Markdown scrolling without weakening its latency or correctness guarantees. Make CI reporting distinguish genuine scanner findings from synthetic test output and missing downstream artifacts.

## Problem Statement

CI run `41239075222` failed a sub-microsecond scrolling assertion in Windows debug tests and Ubuntu coverage instrumentation. The benchmark does not represent the populated row-height state supplied by the renderer. Coverage failure then skipped SARIF generation, but unconditional upload failed on the missing file. Scanner unit tests emitted GitHub annotations for deliberately synthetic token fixtures.

## Capabilities

### New Capabilities

- `ci-quality-reporting`: separate behavioral coverage and optimized performance gates, conditional SARIF publication, and isolation of synthetic scanner annotations.

### Modified Capabilities

- `markdown-preview`: clarify the performance measurement contract and preserve cached scrolling behavior while reducing navigation work for uniform and directive-expanded rows.

## Scope

### In-Scope

- Keep deterministic navigation and layout tests in ordinary debug and coverage suites; run strict timing checks in a dedicated optimized, non-instrumented CI step.
- Preserve parsing, rendering, scrolling, allocation, and memory budgets rather than relaxing thresholds.
- Benchmark observable navigation state with warm-up and repeated samples, covering empty metadata, populated uniform heights, and directive-expanded heights.
- Optimize scrolling with a constant-time uniform-row path and bounded variable-height navigation, preserving cursor visibility, clamping, source-line mapping, and directive semantics.
- Upload SARIF only when a report exists, including a generated report from a scanner command that returns blocking findings.
- Capture synthetic scanner annotations inside tests without changing real CLI annotation behavior or suppressing genuine test failures.
- Update targeted regression tests, benchmark instructions, changelog, and bug reports.
- Affected implementation: the `lgtm` application crate (`src/app/markdown.rs`, `src/ui/markdown.rs`, `src/headless.rs` and nearby tests), CI workflow, and documentation.
- CI impact: retain behavioral checks on Ubuntu, macOS, and Windows; enforce optimized performance separately on the supported matrix; retain Ubuntu coverage enforcement.

### Out-of-Scope (YAGNI)

- Raising latency limits, lowering the existing 90% CI coverage threshold, disabling coverage, deleting assertions, or excluding behavioral tests to make CI green.
- Broad renderer, parser, agent-delivery, or core architecture refactoring.
- New benchmark dependencies unless existing tools cannot provide reliable measurement.
- Changing release version, publishing a release, or upgrading runners/actions merely to silence migration notices.
- Hiding real scanner findings or treating every exit 101 as a coverage-threshold failure.
- Altering `lgtm-core`, `lgtm-ast`, or `lgtm-mcp` behavior.

## Assumptions

- **ASM-GEN-1**: The runtime budget is evaluated on optimized, non-instrumented code; debug and coverage runs verify behavior rather than machine-specific latency.
- **ASM-GEN-2**: Row-height data must stay coherent after resizing, preview rebuilding, and directive/reply changes; any optimization must respect those invalidation points.
- **ASM-GEN-3**: The coverage threshold is preserved at its current CI value. Whether it passes after the instrumentation-sensitive test is separated remains unverified.
- **ASM-GEN-4**: Synthetic annotation tests need an output boundary; removing `--nocapture` alone is insufficient protection for failing tests or subprocess output.

## Success Metrics

- Debug and coverage suites exercise navigation correctness without failing because instrumentation changes wall-clock cost.
- A dedicated optimized benchmark actually runs in CI, prevents compiler elimination of measured work, and enforces the existing scrolling latency budget using representative row heights.
- Uniform-row scrolling avoids scanning viewport rows; directive-expanded navigation remains correct and does not reparse or allocate new line buffers per scroll.
- Missing SARIF after an upstream failure causes a skipped upload, not a secondary missing-path failure; available SARIF remains publishable when findings block the gate.
- Scanner fixture tests cannot create misleading `b.py` annotations; real CLI finding annotations remain unchanged.
- Relevant unit, integration, formatting, lint, coverage, and optimized performance checks have recorded results before completion; CI failures are not assumed resolved solely from local macOS tests.

## Review Gate

- [ ] **Gate 1 Approval**: Proposal reviewed and approved by human before drafting specs.
