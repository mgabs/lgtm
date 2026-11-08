# Architecture & Technical Design

## 1. Overview & Invariants
This design refines the static AST and line-level heuristic guards in `lgtm-core` to eliminate false positives without altering the public API or degrading performance:
- **Invariant 1**: Static guard analysis remains 100% pure, in-memory, and free of filesystem or network I/O.
- **Invariant 2**: Static guard analysis execution time remains under 5ms per 200-line hunk (NFR-STATIC-1).
- **Invariant 3**: True positive detection rate for SQL injection and test skips remains 100%.

## 2. Domain Models, State & Interface Contracts

### 2.1 Rust Tree-sitter SQL Query Rule
File: `crates/lgtm-core/src/guard/rust.rs`

Change regex in the Tree-sitter query rule `sec/sql-injection`:
```scheme
((macro_invocation macro: (identifier) @m (#eq? @m "format") 
  (token_tree (string_literal (string_content) @sql 
    (#match? @sql "(?i)\\b(SELECT\\s+[\\s\\S]+\\s+FROM|INSERT\\s+INTO\\s+|UPDATE\\s+[\\s\\S]+\\s+SET|DELETE\\s+FROM\\s+)[\\s\\S]*\\{")))) @hit)
```

### 2.2 Test Skip Discriminator
File: `crates/lgtm-core/src/guard/mod.rs`

Replace naive substring check:
```rust
// Before:
t.contains("xit(")

// After:
t.split(|c: char| !c.is_alphanumeric() && c != '_').any(|part| part == "xit")
    || (t.contains("xit(") && !t.contains("exit("))
```

### 2.3 Tautological Assertion Sanitization
File: `crates/lgtm-core/src/guard/mod.rs` & `src/ui/help.rs`
- In `crates/lgtm-core/src/guard/mod.rs:190`, append `// lgtm:ignore` to the description string initialization.
- In `src/ui/help.rs:479`, rephrase help text slightly or annotate line to avoid literal `assert True`.
- In `crates/lgtm-core/tests/bdd.rs:884`, ensure test fixture string uses `// lgtm:ignore` if necessary.

## 3. Architecture Diagram & Data Flow

```
+---------------------------------------------------------------------------------+
|                                 DIFF / FILE HUNK                                |
+---------------------------------------------------------------------------------+
                                         |
                                         v
                      +--------------------------------------+
                      |      SecurityAndTestGuard Scanner    |
                      +--------------------------------------+
                                  |              |
                   [Tree-sitter AST]            [Line-level Heuristics]
                                  |              |
                                  v              v
               +--------------------+          +--------------------+
               | RustGuard (AST)    |          | Guard Helper Rules |
               | - Structured SQL   |          | - Non-exit "xit("  |
               |   Clause Regex     |          | - Code-only assert |
               +--------------------+          +--------------------+
                                  \              /
                                   v            v
                      +--------------------------------------+
                      |           GuardAlert Vector          |
                      | (0 false positives on clean code)    |
                      +--------------------------------------+
```

## 4. Boundary Rules & Budgets
- **Pure Core vs. I/O Shell**: All logic remains strictly within pure functions in `crates/lgtm-core`. Zero external crates or I/O introduced.
- **Latency & Resource Budgets**: The regex modification replaces a greedy wildcard with bounded clause tokens, reducing regex backtracking and executing in < 1ms per hunk.
- **Rejected Alternatives**:
  - `sqlparser-rs`: Rejected because it adds heavy dependency compilation overhead and violates the < 5ms per hunk budget on malformed query fragments.
  - Ad-hoc ignore list: Rejected because it is brittle and requires endless maintenance.

## 5. Testing Strategy
1. **BDD Scenarios**: Run `cargo test -p lgtm-core --test bdd` to verify existing behavior specs.
2. **Unit Tests (>= 95% coverage)**:
   - Add unit tests in `crates/lgtm-core/src/guard/rust.rs` verifying that `format!("Deleted comment on {}:{}", path, line)` and `format!("Selected PR #{}...", id)` produce zero alerts.
   - Add unit test verifying that `std::process::exit(1)` does not trigger `test/skipped`.
3. **Mutation Testing (100% kill)**: Run `cargo mutants` on `guard/rust.rs` to verify new regex mutations are killed.

## Review Gate
- [x] **Gate 3 Approval**: Architecture & design approved to proceed to task breakdown.
