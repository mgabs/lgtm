## Purpose

Keep CI behavioral coverage, performance measurement, and scanner reporting independently meaningful so failures identify genuine regressions.

## ADDED Requirements

### Requirement: REQ-CIQR-1 Separate Performance Measurement
CI MUST execute deterministic behavioral tests on Ubuntu, macOS, and Windows and retain Ubuntu's 90% line-coverage threshold. Strict wall-clock budgets MUST be enforced in a separate optimized, non-instrumented check, not in debug or coverage runs. Separating timing MUST NOT remove behavioral assertions or silently omit the performance check.

#### Scenario: Instrumented behavioral run
- **WHEN** navigation tests execute under coverage instrumentation
- **THEN** correctness assertions are exercised without enforcing runtime wall-clock budgets
- **AND** a below-threshold coverage result still fails CI

#### Scenario: Optimized performance gate
- **WHEN** CI executes its performance check on each supported platform
- **THEN** it measures optimized, non-instrumented code and fails on a violated budget
- **AND** a selector matching no performance tests fails rather than succeeding silently

### Requirement: REQ-CIQR-2 Representative Benchmark Workloads
Performance checks MUST measure observable results, use warm-up and repeated samples, and report workload, build mode, sample policy, and measured latency. Navigation workloads MUST cover empty height metadata, populated uniform heights, and directive-expanded rows. Compiler elimination and repeated no-op boundary scrolling MUST NOT substitute for measured navigation.

#### Scenario: Populated renderer state
- **WHEN** scrolling latency is measured
- **THEN** populated uniform and variable-height workloads perform actual movement and their resulting cursor and scroll state are consumed

#### Scenario: Reproducible failure evidence
- **WHEN** a workload violates its latency budget
- **THEN** the report names the workload, measurement policy, latency, and threshold

### Requirement: REQ-CIQR-3 Publish Only Available SARIF
SARIF upload MUST require an existing generated report and the existing eligible event/platform conditions. A missing report after an upstream failure MUST skip upload without a secondary missing-path error. A valid report produced with blocking findings MUST remain eligible for upload; scanner failures MUST retain their failing status.

#### Scenario: Upstream failure prevents generation
- **WHEN** a failed test or coverage step prevents SARIF generation
- **THEN** upload is skipped and the original failure remains visible

#### Scenario: Scanner finds blocking issues
- **WHEN** the scanner writes valid SARIF and returns a blocking-findings exit code
- **THEN** the report remains eligible for upload and CI remains failed

### Requirement: REQ-CIQR-4 Isolate Synthetic Scanner Annotations
Tests scanning synthetic fixtures MUST capture or safely render workflow-command output so fixture findings cannot create GitHub annotations, including on test failure. Real CLI annotation behavior MUST remain unchanged. Test output isolation MUST preserve diagnostic evidence and nonzero failure status.

#### Scenario: Synthetic finding in a test
- **WHEN** a test scans a synthetic token fixture and exercises GitHub annotation formatting
- **THEN** the annotation is verified as captured data rather than emitted as an active workflow command

#### Scenario: Real CLI finding
- **WHEN** a user runs the real scanner in GitHub annotation mode against a blocking finding
- **THEN** the expected annotation is emitted and the command returns its blocking status

## Review Gate

- [ ] **Gate 2 Approval**: Human approves these specifications before technical design.
