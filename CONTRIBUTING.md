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

# 3. Run all unit & integration tests (39 tests)
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
├── git/mod.rs      # Streaming git diff engine + non-git directory fallback
├── pr/mod.rs       # GitHub PR engine via `gh` CLI & unified diff parser
├── guard/mod.rs    # AST & regex security scanner & test fraud detector
├── comment/mod.rs  # Directive storage, PR metadata & batch review compiler
├── turn/mod.rs     # Turn manager & reviewed file tracker (.lgtm/state.json)
├── ipc/mod.rs      # Tmux pane injector & OSC 52 clipboard fallback
└── mutate/mod.rs   # Mutation testing engine (12 AST/binary mutant operators)
```

---

## 5. Adding New Security Guard Rules

To add a new security detector or test fraud pattern:
1. Open [`src/guard/mod.rs`](file:///Users/mmetawea/Workbench/lgtm/src/guard/mod.rs).
2. Add regex pattern to `SecurityGuard::new()`.
3. Add a corresponding test case in `tests` checking that additions trigger alerts and deletions do not.
4. Run `make test && make mutate` to verify coverage.

---

## 6. Pull Request Guidelines

1. **Focused PRs**: Keep PRs small and focused on a single feature or bugfix.
2. **Test Coverage**: Accompany every code change with unit or integration tests.
3. **Commit Messages**: Use conventional commits (`feat: ...`, `fix: ...`, `docs: ...`, `refactor: ...`, `test: ...`).
4. **Clean Tree**: Verify `make check` passes cleanly before pushing.
