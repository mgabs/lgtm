# Proposal

## Intent
Harden static analysis heuristics in `lgtm-core` to eliminate 15 false-positive Critical and High security findings caused by naive regex and substring matching, while maintaining 100% detection rate for real SQL injections, test skips, and tautological assertions.

## Problem Statement
Full codebase audits (`lgtm scan`) currently report 11 Critical SQL injection vulnerabilities and 4 High severity test fraud alerts on clean code. This occurs because:
1. Tree-sitter query in `crates/lgtm-core/src/guard/rust.rs` matches common English verbs like "Deleted" and "Selected" in UI log formatting strings.
2. The JS test skip matcher in `crates/lgtm-core/src/guard/mod.rs` matches `std::process::exit(...)` because `exit(` contains `xit(`.
3. Tautological assertion string matching flags its own rule description, help text, and test fixtures.

## Capabilities

### Modified Capabilities
- `static-analysis`: Tighten requirement `REQ-STATIC-1` to mandate SQL clause grammar patterns; refine test skip and tautological assertion matching.

## Scope

### In-Scope
- Refine tree-sitter AST query in `crates/lgtm-core/src/guard/rust.rs` to require SQL clause syntax (`SELECT...FROM`, `INSERT INTO`, `UPDATE...SET`, `DELETE FROM`).
- Refine JS test skip matcher in `crates/lgtm-core/src/guard/mod.rs` to prevent matching `exit(`.
- Restrict `is_tautological_assertion` matching to avoid triggering on user-facing help text and rule descriptions; annotate test fixtures with `// lgtm:ignore`.
- Add regression tests in `crates/lgtm-core` verifying zero false positives on UI formatting, process exit, and help text.
- Affected crates: `lgtm-core`, `lgtm`.

### Out-of-Scope (YAGNI)
- Introducing heavy external SQL AST parsing libraries (`sqlparser-rs`).
- Rewriting tree-sitter grammars or altering the AST engine architecture.
- Modifying other languages (`python.rs`, `go.rs`, `java.rs`) unless regression is demonstrated.

## Assumptions
- **ASM-GEN-1**: Genuine SQL queries constructed via `format!` will always contain standard SQL clause verbs and syntax keywords.
- **ASM-GEN-2**: In-memory static analysis execution time will remain below the 5ms budget per hunk.
- **ASM-GEN-3**: Cross-platform CI matrix (Ubuntu, macOS, Windows) behavior is unaffected since string matching and regex logic is OS-agnostic.

## Success Metrics
- `lgtm scan` on the repository reports 0 Critical vulnerabilities and 0 High severity alerts.
- Existing tests for `sec/sql-injection`, `test/skipped`, and `test/tautological-assert` continue to pass 100%.
- New unit tests confirm that UI formatting strings (`format!("Deleted comment...")`, `format!("Selected PR #{}...")`) and `std::process::exit(...)` are not flagged.

## Review Gate
- [x] **Gate 1 Approval**: Proposal approved to draft specs.
