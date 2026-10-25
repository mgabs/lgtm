# Design Specification: Deterministic AST-Driven Static Analysis & Architectural Guard Engine

**Date:** 2026-10-17  
**Status:** Proposed  
**Author:** LGTM Core Team  
**Scope:** `crates/lgtm-ast`, `crates/lgtm-core`

---

## 1. Executive Summary & Intent

LGTM provides real-time diff monitoring and steering gates for human reviewers and coding agents. Currently, several checks in `crates/lgtm-core` rely on crude regex word tokenization or substring matching. This causes high-friction false positives on documentation, UI string literals, and language convention conversions (e.g. `CommentInput` vs `comment_input`, `directives` in UI docs vs `directive`).

This design replaces heuristic word splitting with a **deterministic, non-LLM, AST-driven static analysis engine** powered by Tree-sitter across **Rust, Python, TypeScript/TSX, and Swift**. It structurally eliminates false positives while expanding static architectural and anti-pattern enforcement.

---

## 2. Goals, Non-Goals & Anti-Goals

### Goals
- **100% AST-Filtered Symbol Scoping:** Extract only true code identifiers (variables, functions, types, fields, imports) across Rust, Python, TypeScript/TSX, and Swift.
- **Strict Literal & Comment Isolation:** Ensure string literals, format strings, docstrings (`"""`), comments, and UI spans (`Span::raw("...")`) are structurally ignored by typo and symbol mutation guards.
- **Convention & Inflection Invariants:** Recognize valid naming convention transformations (snake_case $\leftrightarrow$ PascalCase/camelCase) and grammatical inflections across distinct AST constructs without false typo alerts.
- **Deterministic Anti-Pattern Detection:** Flag silent error swallowing (`.ok()`, `let _ = ...`, bare `except: pass`), synchronous I/O in UI render loops, and AST-resolved import layer violations.
- **Zero LLM Dependency:** Retain millisecond-latency, offline execution with zero non-deterministic LLM variance.

### Non-Goals
- Full compiler type-checking or multi-crate linker analysis (rustc/mypy/tsc territory).
- Whole-program semantic reachability analysis across uncommitted repository history.

### Anti-Goals
- Adding heavy external C/C++ build chains beyond official tree-sitter language crates.
- Degrading check performance below sub-second turnaround on git diffs.

---

## 3. Architecture & Module Design

```
                     ┌──────────────────────────────────────────────┐
                     │              Git Diff Hunk                   │
                     └──────────────────────┬───────────────────────┘
                                            │
                       File Extension / Lang Dispatcher
                                            │
        ┌───────────────────┬───────────────┴───────────────┬───────────────────┐
        ▼                   ▼                               ▼                   ▼
┌───────────────┐   ┌───────────────┐               ┌───────────────┐   ┌───────────────┐
│  tree-sitter- │   │  tree-sitter- │               │  tree-sitter- │   │  tree-sitter- │
│     rust      │   │    python     │               │  typescript   │   │     swift     │
└───────┬───────┘   └───────┬───────┘               └───────┬───────┘   └───────┬───────┘
        │                   │                               │                   │
        └───────────────────┴───────────────┬───────────────┴───────────────────┘
                                            ▼
                           ┌─────────────────────────────────┐
                           │    lgtm-ast: Normalized AST     │
                           │  - AST Identifiers only         │
                           │  - Imports / Module Paths       │
                           │  - Error Handling Constructs    │
                           │  - Function Scope Context       │
                           └────────────────┬────────────────┘
                                            │
        ┌───────────────────────────────────┼───────────────────────────────────┐
        ▼                                   ▼                                   ▼
┌──────────────────┐               ┌──────────────────┐               ┌──────────────────┐
│   SymbolGuard    │               │ArchitectureGuard │               │ FragilityGuard   │
│ - Typo check     │               │ - Layer bounds   │               │ - Silent errors  │
│ - Convention norm│               │ - Blast radius   │               │ - Render I/O     │
└──────────────────┘               └──────────────────┘               └──────────────────┘
```

---

## 4. Detailed Component Specifications

### 4.1. Language Grammar Extensions in `crates/lgtm-ast`

`lgtm-ast` Cargo dependencies are updated:
```toml
[dependencies]
tree-sitter = "0.25"
tree-sitter-rust = "0.23"
tree-sitter-python = "0.23"
tree-sitter-typescript = "0.23"
tree-sitter-swift = "0.7"
```

The language enum and query mapping:
- `Lang::Rust`: maps `.rs` files. Queries node kinds `identifier`, `type_identifier`, `field_identifier`, `scoped_identifier`.
- `Lang::Python`: maps `.py` files. Queries node kinds `identifier`, `dotted_name`, `function_definition`, `class_definition`.
- `Lang::Tsx` / `Lang::Ts`: maps `.ts`, `.tsx`, `.js`, `.jsx`.
- `Lang::Swift`: maps `.swift`.

### 4.2. File-Scope vs. Hunk AST Parsing Strategy

To guarantee syntactic integrity across languages with indentation (Python) or block closures (Rust):
1. **Working-Tree (Disk Available):** Parse the complete file via Tree-sitter. Compute line-level byte spans and map findings exclusively to added/modified lines in the diff. This guarantees enclosing function scope (e.g. `fn render_*`), multiline import blocks (`use db::{...}`), and zero `ERROR` recovery degradation.
2. **Remote PR / In-Memory Diff (Disk Unavailable):** For pure patch diffs, fall back to hunk parsing with leading-whitespace normalization (auto-dedent for Python) and synthetic block wrappers for Rust.

### 4.3. Symbol Extraction & AST Normalization Rules

In `crates/lgtm-core/src/guard/symbols.rs`:
1. **Node Filtering:** Tokens are strictly acquired through Tree-sitter query captures matching definition or reference identifiers. String nodes (`string_literal`, `string_content`, `interpreted_string_literal`) and comment nodes (`line_comment`, `block_comment`) are excluded at the parse-tree level.
2. **Stem-Safe Convention & Pluralization Normalization:**
   ```rust
   pub fn is_naming_convention_variant(added: &str, existing: &str) -> bool {
       let norm_added = added.replace('_', "").to_lowercase();
       let norm_existing = existing.replace('_', "").to_lowercase();
       if norm_added == norm_existing {
           return true;
       }
       // Prevent false stemming on words ending in 'ss' (e.g. process, class, address)
       let is_valid_stem = |w: &str| w.len() >= 5 && !w.ends_with("ss");
       if is_valid_stem(&norm_added) && norm_added.strip_suffix('s') == Some(&norm_existing) {
           return true;
       }
       if is_valid_stem(&norm_existing) && norm_existing.strip_suffix('s') == Some(&norm_added) {
           return true;
       }
       false
   }
   ```

### 4.4. Dedicated `FragilityGuard` Architecture (`crates/lgtm-core/src/guard/fragility.rs`)

A dedicated guard module implementing `crate::guard::Guard`:
1. **Silent Error Swallowing:**
   - **Rust:** Flag additions containing `.ok();`, `let _ = `, or empty `Err(_)` match arms.
     - *Allowlist:* Permit standard benign drops (`writeln!`, `write!`, `tx.send`, `mpsc`, `Sender::send`).
     - *Severity:* `🟡 WARN`.
   - **Python:** Flag additions of `except:` or `except Exception: pass`.
   - **TypeScript:** Flag empty catch blocks without logging, rethrow, or return.
2. **Hot-Loop / UI Render I/O Hazards:**
   - Detect synchronous disk I/O (`fs::read`, `std::fs`, `open()`, `readFileSync`) or blocking process invocations within functions matching `render_*`, `draw_*`, or high-frequency tick loops.
   - *Severity:* `🟠 HIGH`.
3. **AST Layer Invariants (`crates/lgtm-core/src/guard/arch.rs`):**
   - Resolve multiline and single-line import AST nodes (`use_declaration`, `import_statement`) rather than substring matching on raw lines.
   - Enforce unidirectional layering: presentation (`ui`, `components`) cannot import persistence/DB modules (`sqlx`, `prisma`, `db::`, `repository::`).

---

## 5. Verification & Testing Strategy

1. **Unit Tests in `lgtm-ast`**:
   - Parse sample Rust snippets containing string literals with similar words (`Span::raw("directive")` vs `directives`) and verify strings are never returned as identifiers.
   - Parse Python snippets with docstrings, comments, and variables to ensure zero leakage.
2. **Unit Tests in `lgtm-core`**:
   - `test_convention_variants`: verify `CommentInput` vs `comment_input` and `directive` vs `directives` produce zero typo alerts.
   - `test_stemming_guards`: verify `process` vs `proces` is correctly flagged as a typo.
   - `test_silent_error_detection`: verify `.ok()` and `except: pass` are correctly flagged, while `let _ = tx.send(...)` is allowed.
   - `test_render_io_detection`: verify synchronous read in `render_view` triggers a hazard alert.
3. **End-to-End Self-Scan**:
   - Run `lgtm check` against the repository diff. Verify 0 false-positive warnings on `help.rs` and `input.rs`.

---

## 6. Implementation Deliverables
1. `crates/lgtm-ast/Cargo.toml` & `crates/lgtm-ast/src/lib.rs`: Tree-sitter Rust & Python parsers with full-file query support.
2. `crates/lgtm-core/src/guard/symbols.rs`: AST symbol integration and stem-safe convention normalization.
3. `crates/lgtm-core/src/guard/fragility.rs`: Dedicated fragility and silent-error guard.
4. `crates/lgtm-core/src/guard/arch.rs`: AST-resolved import layer invariants.
5. Comprehensive test suites validating false-positive elimination.

