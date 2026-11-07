# Research & Technical Tradeoffs

## 1. Intent & Ground Truth
- **Problem**: `lgtm_security_scan` (MCP) and headless audits miss critical vulnerability patterns and test fraud in Rust source files (`src/*.rs`):
  1. Dynamic SQL constructed via `format!`: `format!("SELECT * FROM t WHERE id = '{}'", user)`.
  2. Shell command injection via `sh -c` / `bash -c` / `cmd /c` with variable arguments: `Command::new("sh").arg("-c").arg(c)`.
  3. Tautological assertions in inline tests: `#[test] fn t() { assert!(true); }` or `assert_eq!(x, x)`.
- **Verified Facts in Codebase**:
  - `registry()` in `crates/lgtm-core/src/guard/mod.rs` registers language packs for Python, Go, Java, Swift, and Web, but has no `RustGuard`.
  - `tree-sitter-rust` is already bundled in the workspace and exposed via `lgtm_ast::Lang::Rust`.
  - `is_sql_injection` in `guard/mod.rs` checks only Python/JS string formats and requires `execute(` / `query(` / `raw(` on the exact same line.
  - `is_command_injection` in `guard/mod.rs` only checks `shell=True`, `os.system`, `child_process.exec`, and `exec.Command("sh"`.
  - `is_tautological_assertion` in `guard/mod.rs:181` is strictly gated on `if is_test_file`, which checks path strings for `"test"`, silently skipping inline tests inside `src/*.rs`.

## 2. Options Explored

### Option A: Dedicated `RustGuard` AST Pack + Generic Rule IDs (Recommended)
- Implement `RustGuard` in `crates/lgtm-core/src/guard/rust.rs` using `lgtm_ast::hits_within(Lang::Rust, ...)`.
- Emit stable generic rule IDs: `sec/sql-injection`, `sec/command-injection`, and `test/tautological-assert`.
- Evaluate tautological assertions in Rust files regardless of whether the file path contains `"test"`.
- **Pros**: High precision, minimal false positives, leverages existing Tree-sitter grammar, consistent with Go and Web guards.
- **Cons**: Requires crafting Tree-sitter queries for Rust macro invocations and method chaining.

### Option B: Line-level Regex Rules in `guard/mod.rs`
- Expand `is_sql_injection`, `is_command_injection`, and remove the `is_test_file` condition on `is_tautological_assertion`.
- **Pros**: Very fast, zero AST overhead.
- **Cons**: High false-positive rate (e.g. static shell commands `Command::new("sh").arg("-c").arg("echo 1")` or comments/strings).

### Option C: External Tool Subprocess Invocation (cargo-audit / clippy)
- Shell out to external analysis tools during `lgtm_security_scan`.
- **Pros**: Out-of-the-box linting rules.
- **Cons**: Violates pure core boundary rules (`lgtm_core` must remain zero I/O and sub-second fast), slow, external tool dependencies.

## 3. Tradeoff Matrix

| Criteria | Option A (AST Pack + Generic IDs) | Option B (Regex Only) | Option C (External CLI) |
| :--- | :--- | :--- | :--- |
| **Precision (Low FP)** | High | Moderate / Low | High |
| **New Dependencies** | None (already in `lgtm-ast`) | None | High (`cargo-audit`, etc.) |
| **Performance & Latency** | High (<5ms per file) | Highest (<1ms) | Very Low (>2-5s) |
| **Core Purity (Zero I/O)** | 100% pure in-memory | 100% pure in-memory | Fails (requires subprocesses) |
| **Blast Radius** | Low (additive guard module) | Low | High |

## 4. Recommendation
Adopt Option A. Implement a dedicated `RustGuard` in `crates/lgtm-core/src/guard/rust.rs` utilizing the existing `tree-sitter-rust` grammar in `lgtm_ast` with generic rule IDs (`sec/sql-injection`, `sec/command-injection`, `test/tautological-assert`), and ensure tautological assertions in Rust inline test functions (`#[test]`) are detected regardless of file path.

## 5. Breaking Changes & Deprecations
None. New static rules are purely additive findings.

## Review Gate
- [x] **Gate 0 Decision**: Option A (AST Pack + Generic IDs) approved by user.
