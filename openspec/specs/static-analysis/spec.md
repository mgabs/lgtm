# static-analysis Specification

## Purpose
Provide static AST and semantic guard rules in `lgtm-core` to detect security vulnerabilities (SQL injection, command injection) and test quality fraud (tautological assertions) across Rust source files and inline tests.

## Requirements

### Requirement: REQ-STATIC-1 Rust SQL Injection Detection
The static analysis engine MUST flag added Rust code that constructs SQL queries by dynamically interpolating expressions into query strings containing SQL statements (such as `SELECT`, `INSERT`, `UPDATE`, `DELETE`) with rule ID `sec/sql-injection` and Critical severity.

#### Scenario: Dynamic SQL interpolation via format macro
- **WHEN** a diff hunk in a Rust file adds `format!("SELECT * FROM users WHERE id = '{}'", user)`
- **THEN** a security finding with rule ID `sec/sql-injection`, line number, and Critical severity is emitted.

#### Scenario: Parameterized query is not flagged
- **WHEN** a diff hunk in a Rust file adds a static parameterized query such as `sqlx::query("SELECT * FROM users WHERE id = $1").bind(user)`
- **THEN** no `sec/sql-injection` finding is emitted.

### Requirement: REQ-STATIC-2 Rust Command Injection Detection
The static analysis engine MUST flag added Rust code that invokes shell interpreters (including `sh`, `bash`, `cmd`, `powershell`, and `zsh` via `std::process::Command`) with command-execution arguments (`-c`, `/C`, `-Command`) and non-literal expressions with rule ID `sec/command-injection` and Critical severity.

#### Scenario: Dynamic command via shell interpreter
- **WHEN** a diff hunk in a Rust file adds `Command::new("sh").arg("-c").arg(user_input)`
- **THEN** a security finding with rule ID `sec/command-injection`, line number, and Critical severity is emitted.

#### Scenario: Direct executable execution is not flagged
- **WHEN** a diff hunk in a Rust file adds `Command::new("git").arg("status")` or static string literals
- **THEN** no `sec/command-injection` finding is emitted.

### Requirement: REQ-STATIC-3 Inline Test Tautological Assertion Detection
The guard engine MUST evaluate tautological assertions (including `assert!(true)`, `assert!(1 == 1)`, `assert_eq!(x, x)`, and `assert_ne!(x, x)`) across all analyzed source files without restricting evaluation to file paths containing `test`, emitting findings with rule ID `test/tautological-assert` and High severity.

#### Scenario: Tautological assert in Rust source file
- **WHEN** a diff hunk in `src/foo.rs` adds `assert!(true)` or `assert_eq!(val, val)` inside a test or production function
- **THEN** a test quality finding with rule ID `test/tautological-assert`, line number, and High severity is emitted.

#### Scenario: Meaningful assertion is not flagged
- **WHEN** a diff hunk in `src/foo.rs` adds `assert!(is_valid)` or `assert_eq!(result, expected)`
- **THEN** no `test/tautological-assert` finding is emitted.

### Requirement: NFR-STATIC-1 Static Scan Performance Budget
The static AST guard analysis MUST complete within 5 milliseconds per diff hunk in memory without triggering external process executions or network/filesystem I/O.

#### Scenario: In-memory evaluation budget
- **WHEN** the static guard analyzes a diff hunk containing up to 200 lines of Rust code
- **THEN** analysis completes in under 5 milliseconds with zero I/O operations.
