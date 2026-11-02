# Proposal

## Intent
Raise the Markdown preview parser (`src/app/markdown.rs`) to at least 95% line and branch coverage with behavior-focused regression tests, and make the measurement reproducible.

## Problem Statement
Nightly measurement of the current tree gives `src/app/markdown.rs` 97.38% line coverage (967/993) but only 82.53% branch coverage (137/166). The 29 uncovered branch outcomes sit in parser paths (fences, lists, quotes, inline formatting, wrapping, Mermaid edge parsing) that no test exercises, so regressions there go unnoticed. Parser mutation resistance has never been measured.

## Capabilities

### Modified Capabilities
- `markdown-preview`: adds a requirement that parser behavior is protected by tests meeting the coverage bar, with the measurement command documented.

## Scope

### In-Scope
- Add small, representative Markdown inputs to the existing unit tests in `src/app/markdown.rs`, asserting rendered text, styling, wrapping and source-line attribution.
- Document the reproducible measurement (toolchain, command, aggregation policy) in the change design and in `docs/WIKI.md`.
- Run scoped mutation testing on the parser and strengthen assertions against survivors.
- Add one BDD scenario tagged with the new requirement keys.

### Out-of-Scope (YAGNI)
- No parser behavior change; a genuine bug found by a test gets its own change.
- No extraction of the parser into another module.
- No lowering of the CI 90% workspace gate, and no coverage-exclusion attributes.
- No new CI job or dependency; branch coverage needs nightly Rust, so it is measured locally, not gated in CI.

## Assumptions
- **ASM-GEN-1**: `cargo +nightly llvm-cov --branch` is available locally (nightly toolchain and `cargo-llvm-cov` 0.9.1 are installed).
- **ASM-GEN-2**: Duplicate compiled instances of one source branch are counted once per source location, never by hiding an uncovered outcome.

## Success Metrics
- `src/app/markdown.rs` at least 95% line and 95% branch coverage (baseline: 97.38% / 82.53%, i.e. 158 of 166 branches needed).
- Scoped parser mutations: 100% caught, or each survivor documented as equivalent.
- Full suite passes; existing preview budgets and assertions are unchanged.

## Review Gate
- [x] **Gate 1 Approval**: Proposal reviewed and approved by human before drafting specs, approved by the user on 2026-11-02.
