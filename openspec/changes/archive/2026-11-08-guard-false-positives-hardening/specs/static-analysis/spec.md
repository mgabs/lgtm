# Spec Delta

## MODIFIED Requirements

### Requirement: REQ-STATIC-1 Rust SQL Injection Detection
The static analysis engine MUST flag added Rust code that constructs SQL queries by dynamically interpolating expressions into query strings containing structured SQL clauses (such as `SELECT...FROM`, `INSERT INTO`, `UPDATE...SET`, `DELETE FROM`) with rule ID `sec/sql-injection` and Critical severity, without flagging plain UI or logging format strings.

#### Scenario: Dynamic SQL interpolation via format macro
- **WHEN** a diff hunk in a Rust file adds `format!("SELECT * FROM users WHERE id = '{}'", user)`
- **THEN** a security finding with rule ID `sec/sql-injection`, line number, and Critical severity is emitted.

#### Scenario: Parameterized query is not flagged
- **WHEN** a diff hunk in a Rust file adds a static parameterized query such as `sqlx::query("SELECT * FROM users WHERE id = $1").bind(user)`
- **THEN** no `sec/sql-injection` finding is emitted.

#### Scenario: English verbs in logging and UI format strings are not flagged
- **WHEN** a diff hunk adds `format!("Deleted comment on {}:{}", path, line)` or `format!("Selected PR #{}...", id)`
- **THEN** no `sec/sql-injection` finding is emitted.

## ADDED Requirements

### Requirement: REQ-STATIC-4 Process Exit and Non-Test Skip Discrimination
The guard analysis engine MUST discriminate JavaScript/Jest skip tokens (`xit(`) with word boundary or leading delimiter checks so that invocations of process termination functions such as `std::process::exit(...)` or `sys.exit(...)` are not flagged as skipped tests.

#### Scenario: Process exit call in test file is not flagged
- **WHEN** a test harness calls `std::process::exit(code)`
- **THEN** no `test/skipped` finding is emitted.

#### Scenario: Genuine test skip is flagged
- **WHEN** a test file adds `xit("skips test", () => {})` or `#[ignore]`
- **THEN** a `test/skipped` finding is emitted.

## Review Gate
- [x] **Gate 2 Approval**: Specifications reviewed and approved by human before technical design.
