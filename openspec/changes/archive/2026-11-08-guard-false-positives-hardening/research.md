# Research & Technical Tradeoffs

## 1. Intent & Ground Truth
Full codebase audit (`lgtm scan`) flags 11 Critical vulnerabilities and 4 High risks that are all false positives:
- **Critical SQL Injection**: In `crates/lgtm-core/src/guard/rust.rs:12`, tree-sitter AST regex `(?i)(SELECT|INSERT|UPDATE|DELETE).*\\{` matches standard UI/CLI log formatting verbs (e.g. `format!("Deleted comment on {}:{}", path, line)`, `format!("Selected PR #{}...", id)`).
- **Test Skipped**: In `crates/lgtm-core/src/guard/mod.rs:566`, `t.contains("xit(")` intended for JS test runner skips matches `std::process::exit(...)` in `crates/lgtm-core/src/github/tests.rs:186`.
- **Tautological Assertions**: In `crates/lgtm-core/src/guard/mod.rs:574`, raw substring match `lower.contains("assert true")` matches the internal description string in `guard/mod.rs:190` (`"assert True / x == x"`), user help text in `src/ui/help.rs:479`, and test fixtures in `crates/lgtm-core/tests/bdd.rs:884`.
- **Performance Budget**: Static scans must remain in-memory and under 5ms per diff hunk (NFR-STATIC-1).

## 2. Options Explored

### Option A: Refined SQL AST Clause Regex & Word Boundary Assertions (Minimal & YAGNI)
- **SQL Injection**: Tighten tree-sitter query regex to match SQL statement grammar:
  `(?i)\\b(SELECT\\s+[\\s\\S]+\\s+FROM|INSERT\\s+INTO\\s+|UPDATE\\s+[\\s\\S]+\\s+SET|DELETE\\s+FROM\\s+)[\\s\\S]*\\{`
- **Test Skip**: Enforce word boundary / identifier check: `(^|[^a-zA-Z0-9_])xit\\(`.
- **Tautological Assert**: Disallow matching inside string literals, or require code-level boundary delimiters `\\bassert\\s+True\\b` / `assert!\\(true\\)`. Add `// lgtm:ignore` to internal fixture and doc lines.

### Option B: Full SQL AST Parser Integration (sqlparser-rs)
- Pass format string literals to `sqlparser::parser::Parser::parse_sql`.
- **Pros**: Perfectly accurate SQL detection; eliminates all false positives.
- **Cons**: Adds a heavy dependency (`sqlparser`), slows down the <5ms scan budget, fails on incomplete query fragments.

### Option C: Whitelist Heuristic Dictionary
- Maintain a list of ignored substrings (e.g. `"Deleted comment"`, `"Selected PR"`, `"process::exit"`).
- **Pros**: Fast to implement.
- **Cons**: High maintenance burden, fragile, easily bypassed, brittle across new features.

## 3. Tradeoff Matrix

| Criteria | Option A (Refined Regex & Boundaries) | Option B (sqlparser-rs crate) | Option C (Whitelist Dictionary) |
| :--- | :--- | :--- | :--- |
| **Complexity** | Low (regex refinement) | High (crate integration & dialect parsing) | Low-Medium (ad-hoc list) |
| **New Dependencies** | None (uses existing tree-sitter regex) | `sqlparser` crate | None |
| **Performance & Latency** | Zero overhead (< 1ms) | Adds 2-10ms per hunk | Zero overhead |
| **Maintenance Burden** | Minimal | High (SQL dialect drifts) | High (continuous whack-a-mole) |
| **Blast Radius** | Localized to `rust.rs` & `guard/mod.rs` | Broad dependency tree | Fragmented exception rules |

## 4. Recommendation
Adopt Option A. It eliminates all 15 false positives with zero external dependencies, maintains the strict <5ms static scan performance budget, and keeps rule logic localized.

## 5. Breaking Changes & Deprecations
- **REQ-STATIC-1**: Clarified to require SQL clause structure (`SELECT...FROM`, `INSERT INTO`, `UPDATE...SET`, `DELETE FROM`) so English sentences in UI strings are excluded.
- **REQ-STATIC-3**: Maintained. Tautological assertions in test fixtures and help strings annotated with `// lgtm:ignore`.

## Review Gate
- [x] **Gate 0 Decision**: Go (Option A: Refined SQL AST Clause Regex & Word Boundary Assertions).
