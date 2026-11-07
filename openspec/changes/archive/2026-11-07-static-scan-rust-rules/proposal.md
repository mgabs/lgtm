# Proposal

## Intent
Add static security analysis and test fraud detection rules for Rust source code in `lgtm-core` via a dedicated AST-powered `RustGuard`, detecting SQL injection via dynamic string interpolation, command injection via shell invocations, and tautological assertions in test functions.

## Problem Statement
The `lgtm` security engine contains AST guard packs for Python, Go, Java, Swift, and TypeScript/JavaScript, but lacks a dedicated Rust guard pack. As reported in Issue #82, `lgtm_security_scan` (MCP) and headless checks miss critical vulnerabilities in Rust code (`src/*.rs`):
1. SQL statements interpolated via `format!` (`format!("SELECT * FROM t WHERE id = '{}'", user)`).
2. External shell commands executed with dynamic arguments (`Command::new("sh").arg("-c").arg(c)`).
3. No-op test assertions (`#[test] fn t() { assert!(true); }` or `assert_eq!(x, x)`) because tautological assertion detection was restricted to files containing `"test"` in their path.

## Affected Crates & CI Impact
- **Affected Crates**: `lgtm-core` (`crates/lgtm-core/src/guard/`).
- **CI Impact**: Zero impact on OS matrix (Ubuntu, macOS, Windows). `lgtm-core` remains purely in-memory with zero I/O or network dependencies.

## Capabilities

### New Capabilities
- `static-analysis`: Static AST and semantic guard rules detecting SQL injection, command injection, and test neutering in Rust and other supported languages.

### Modified Capabilities
None.

## Scope

### In-Scope
- Implement `RustGuard` in `crates/lgtm-core/src/guard/rust.rs` using `lgtm_ast::hits_within(Lang::Rust, ...)`.
- Add rule for SQL string interpolation (`sec/sql-injection`, Severity: Critical) matching `format!` or string concatenation containing SQL keywords (`SELECT`, `INSERT`, `UPDATE`, `DELETE`) with dynamic variable interpolation.
- Add rule for shell command injection (`sec/command-injection`, Severity: Critical) matching `Command::new("sh" | "bash" | "cmd" | "powershell" | "zsh")` with `-c` / `/C` / `-Command` and non-literal arguments.
- Un-gate tautological assertion detection (`test/tautological-assert`, Severity: High) so `assert!(true)`, `assert!(1 == 1)`, and `assert_eq!(x, x)` are detected inside Rust files and inline `#[test]` / `#[tokio::test]` functions.
- Ensure negative cases (parameterized queries, direct executable invocation without shell `-c`, and valid assertions) pass cleanly without false positives.
- Add unit tests in `crates/lgtm-core/src/guard/rust.rs` and BDD scenarios in `crates/lgtm-core/tests/bdd.rs` / `docs/bdd-scenarios.md`.

### Out-of-Scope (YAGNI)
- No whole-program dataflow or taint analysis across crate boundaries.
- No shell subprocess execution (no external linters like `cargo-audit` or `clippy` invoked at runtime).
- No modifications to MCP protocol schemas or CLI arguments.
- No autofix or AST refactoring rewrites.

## Assumptions
- **ASM-GEN-1**: The existing `lgtm_ast` parser with `Lang::Rust` provides sufficient AST fidelity to capture `macro_invocation` and `call_expression` nodes accurately.
- **ASM-GEN-2**: Parameterized queries using bind variables (e.g., `sqlx::query("SELECT ... $1").bind(...)`) do not contain `{}` interpolation and will not trigger `sec/sql-injection`.
- **ASM-GEN-3**: Tautological assertions (`assert!(true)`, `assert_eq!(a, a)`) are never legitimate in tests or production code and can be flagged safely.

## Success Metrics
- `lgtm_security_scan` and `lgtm check` flag the three planted cases from Issue #82 in Rust source files.
- Zero false positives on parameterized SQL, direct binary execution (`Command::new("git").arg("status")`), and legitimate test assertions.
- Workspace test suite passes with 100% mutation kill rate in `lgtm-core` and >= 90% line coverage.

## Review Gate
- [x] **Gate 1 Approval**: Proposal reviewed and approved by human.
