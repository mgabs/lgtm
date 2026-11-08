# Tasks

## 1. Dependency DAG

```
+--------------------------------------------------------------+
| Slice 1: Rust SQL AST Regex Hardening (sec/sql-injection)     |
+--------------------------------------------------------------+
                               |
                               v
+--------------------------------------------------------------+
| Slice 2: Test Skip & Tautological Assert False Positives     |
+--------------------------------------------------------------+
                               |
                               v
+--------------------------------------------------------------+
| Slice 3: Verification & Quality Gates (Coverage & Scan)       |
+--------------------------------------------------------------+
```

## 2. Streams & Worktrees
| Stream | Worktree Path | Branch | Prerequisite |
| :--- | :--- | :--- | :--- |
| `guard-hardening` | `.worktrees/guard-false-positives` | `feat/guard-false-positives` | None |

## 3. Slice 1: Rust SQL AST Regex Hardening

- [x] 3.1 BDD scenarios in `docs/bdd-scenarios.md` mapping to REQ-STATIC-1 (verify: scenario committed)
- [x] 3.2 Red unit tests in `crates/lgtm-core/src/guard/rust.rs` adding negative test cases for `format!("Deleted comment on {}:{}", path, line)` and `format!("Selected PR #{}...", id)` (verify: fails on current master regex)
- [x] 3.3 Green implementation: update Tree-sitter query regex in `crates/lgtm-core/src/guard/rust.rs` to require structured SQL clauses (verify: `cargo test -p lgtm-core --lib guard::rust` passes)
- [x] 3.4 Verify true positives (`format!("SELECT * FROM users WHERE id = '{}'")`, `INSERT INTO`, `UPDATE...SET`, `DELETE FROM`) remain detected

## 4. Slice 2: Test Skip & Tautological Assertion Hardening

- [x] 4.1 Red unit tests for `is_test_skip` verifying `std::process::exit(1)` and `exit(0)` are not flagged as `test/skipped` (verify: fails on current code)
- [x] 4.2 Green implementation: update `is_test_skip` in `crates/lgtm-core/src/guard/mod.rs` to check token boundaries for `xit(` (verify: test passes)
- [x] 4.3 Annotate internal test fixtures and help strings with `// lgtm:ignore` or sanitize literals to eliminate self-flagging tautological assertions
- [x] 4.4 Verify genuine `xit("...", ...)` and `#[ignore]` continue to be flagged

## 5. Slice 3: Integration, Quality Gates & Full Audit Verification

- [x] 5.1 Run `cargo test` and `make check` across workspace crates (verify: 0 warnings, all tests pass)
- [x] 5.2 Verify line coverage >= 95% via `cargo llvm-cov` on touched files
- [x] 5.3 Run `lgtm scan -o /tmp/lgtm-report-remediated.md` and verify 0 Critical and 0 High alerts on repository
- [x] 5.4 Update `CHANGELOG.md` with guard false-positive fixes

## Review Gate
- [x] **Gate 4 Approval**: Tasks breakdown approved to proceed to implementation.
