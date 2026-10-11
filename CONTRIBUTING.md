# Contributing to LGTM

Thank you for contributing to `lgtm`! We welcome bug reports, feature implementations, security guard detectors, and performance optimizations.

---

## 1. Development Prerequisites

- **Rust Toolchain**: Stable (1.75+)
- **Make**: Standard GNU `make`
- **Optional Tools**:
  - `cargo-llvm-cov` (for detailed HTML coverage reports)
  - `gh` CLI (for testing PR review features)
  - `tmux` (for testing `#ref` agent injection)

---

## 2. Quick Setup & Build

```bash
# Clone repository
git clone https://github.com/mgabs/lgtm.git
cd lgtm

# Build debug binary
make build

# Run interactive TUI against local git diff
./target/debug/lgtm
```

---

## 3. Quality Standards & Gates

All pull requests must pass our strict automated quality gates before merge:

```bash
# 1. Check code formatting
make fmt

# 2. Run Clippy linter with zero warnings allowed
make lint

# 3. Run all unit & integration tests (75 tests)
make test

# 4. Enforce test coverage (>= 90% required)
make coverage

# 5. Enforce mutation testing (>= 90% score required)
make mutate

# 6. Run full verification suite locally
make check
```

---

## 4. Codebase Architecture

```
src/
├── main.rs         # CLI dispatcher, headless CI runner & subcommands
├── app.rs          # State machine, key handlers & Vim motion engine
├── ui/mod.rs       # Ratatui rendering, drawer, inline alerts & modals
├── model.rs        # Diff data models (DiffSnapshot, FileDiff, DiffHunk, DiffLine)
├── scan.rs         # Full repository security & architectural audit scanner
├── git/mod.rs      # Streaming git diff engine + non-git directory fallback
├── pr/mod.rs       # GitHub PR engine via `gh` CLI & unified diff parser
├── guard/          # Deterministic static analysis & guardrail subsystem
│   ├── mod.rs        # SecurityAndTestGuard aggregator & severity definitions
│   ├── security.rs   # Entropy secrets, OWASP injection & auth bypass checks
│   ├── complexity.rs # Cognitive complexity spike analyzer
│   ├── symbols.rs    # Symbol typo & hallucination detector (Levenshtein)
│   ├── dead_code.rs  # YAGNI dead function & unreferenced code detector
│   ├── api_drift.rs  # Breaking public API signature drift detector
│   ├── arch.rs       # Layer boundary violations & stray file detector
│   ├── deps.rs       # Supply chain & unpinned/typosquatted dependency checks
│   └── duplicate.rs  # Duplicate helper function anti-pattern detector
├── comment/mod.rs  # Directive storage, PR metadata & batch review compiler
├── turn/mod.rs     # Turn manager & reviewed file tracker (.lgtm/state.json)
├── ipc/mod.rs      # Tmux pane injector & OSC 52 clipboard fallback
├── watcher/mod.rs  # Cross-platform filesystem watcher & debouncer
├── mcp/mod.rs      # Model Context Protocol (MCP) JSON-RPC stdio server
├── coverage/mod.rs # LCOV parsing & diff-aware delta coverage enforcer
├── verify/mod.rs   # Inverted test execution & fraud detector
└── mutate/mod.rs   # Mutation testing engine (12 AST/binary mutant operators)
```

---

## 5. Adding New Security Guard Rules

To add a new security detector or test fraud pattern:
1. Open [`src/guard/`](src/guard/) and select or create the appropriate submodule (`security.rs`, `arch.rs`, `complexity.rs`, etc.).
2. Add the detector logic to the subsystem scanner and hook it into `SecurityAndTestGuard::scan_file` or `scan_snapshot` in [`src/guard/mod.rs`](src/guard/mod.rs).
3. Add a corresponding test case checking that additions trigger alerts and deletions or valid code do not.
4. Run `make test && make mutate` to verify coverage.

---

## 6. Pull Request Guidelines

1. **Focused PRs**: Keep PRs small and focused on a single feature or bugfix.
2. **Test Coverage**: Accompany every code change with unit or integration tests.
3. **Commit Messages**: Use conventional commits (`feat: ...`, `fix: ...`, `docs: ...`, `refactor: ...`, `test: ...`).
4. **Clean Tree**: Verify `make check` passes cleanly before pushing.
