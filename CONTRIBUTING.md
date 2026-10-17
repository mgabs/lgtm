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

# Install pre-commit hooks (checks fmt, clippy, and tests)
make install-hooks
# or with python pre-commit:
# pre-commit install

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
src/                    # `lgtm` binary: CLI + TUI (app/, ui/, cli.rs, headless.rs)
crates/
├── lgtm-core/src/      # UI-free engine
│   ├── model.rs        # Diff data models
│   ├── git/, pr/       # Git diff engine; GitHub PR via `gh`
│   ├── scan.rs         # Full repository audit scanner
│   ├── guard/          # Static analysis guards (security, complexity, symbols, api_drift, dead_code, arch, deps, duplicate)
│   ├── comment/, turn/ # Directives; reviewed-file tracker
│   ├── ipc/            # Tmux injector, OSC 52 clipboard, AgentSink trait
│   ├── diff_source.rs  # DiffSource trait
│   ├── watcher/        # Filesystem watcher & debouncer
│   └── coverage/, verify/, mutate/  # Quality gates
└── lgtm-mcp/src/lib.rs # MCP JSON-RPC stdio server
```

`lgtm-core` and `lgtm-mcp` must not depend on `ratatui` or `crossterm`; `scripts/check-boundaries.sh` enforces this in CI and `make lint`.

---

## 5. Adding New Security Guard Rules

To add a new security detector or test fraud pattern:
1. Open [`crates/lgtm-core/src/guard/`](crates/lgtm-core/src/guard/) and select or create the appropriate submodule (`security.rs`, `arch.rs`, `complexity.rs`, etc.).
2. Add the detector logic to the subsystem scanner and hook it into `SecurityAndTestGuard::scan_file` or `scan_snapshot` in [`crates/lgtm-core/src/guard/mod.rs`](crates/lgtm-core/src/guard/mod.rs).
3. Add a corresponding test case checking that additions trigger alerts and deletions or valid code do not.
4. Run `make test && make mutate` to verify coverage.

---

## 6. Pull Request Guidelines

1. **Focused PRs**: Keep PRs small and focused on a single feature or bugfix.
2. **Test Coverage**: Accompany every code change with unit or integration tests.
3. **Commit Messages**: Use conventional commits (`feat: ...`, `fix: ...`, `docs: ...`, `refactor: ...`, `test: ...`).
4. **Clean Tree**: Verify `make check` passes cleanly before pushing.
