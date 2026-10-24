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

All pull requests must pass the repository quality gates before merge:

```bash
# 1. Check code formatting
make fmt

# 2. Run Clippy linter with zero warnings allowed
make lint

# 3. Run all unit & integration tests
make test

# 4. Run automated BDD catalog coverage report
make bdd-report

# 5. Run full verification suite locally (fmt + lint + boundary check + test)
make check
```

See [Testing & Quality Gates](README.md#testing--quality-gates).

### Behavior scenarios (BDD)

Keep scenarios that exercise a crate's public API in its top-level `tests/`
directory. Application scenarios that launch `lgtm` belong in the root `tests/`
directory and use `CARGO_BIN_EXE_lgtm`. Internal tests that need private UI state
stay beside their modules under `#[cfg(test)]`.

Write scenarios as ordinary Rust tests with Given/When/Then comments and stable
scenario IDs. No Gherkin runner is required. Each test owns its temporary files
and repositories; offline scenarios must not depend on credentials or the network.

The [behavior scenario catalog](docs/bdd-scenarios.md) lists stable IDs, acceptance
criteria, integration scenarios, and related internal tests.

Run acceptance scenarios with `cargo test --workspace --test bdd`. They also run
as part of `cargo test --workspace`. Use `make bdd-report` to generate an automated
coverage report validating that 100% of catalog criteria are automated and passing.
Keep offline simulations, live-provider verification, and terminal journeys distinct
when reporting results.

---

## 4. Codebase Architecture

```
src/                    # `lgtm` binary: CLI + TUI (app/, ui/, event_loop.rs, diagnostics.rs, cli.rs, headless.rs)
crates/
├── lgtm-ast/src/       # Tree-sitter AST parsing, queries & block folding
├── lgtm-core/src/      # UI-free engine
│   ├── model.rs, debug.rs # Diff data models; structured JSONL timing recorder
│   ├── git/, pr/, forge/ # Git diffs; GitHub and Azure PR providers
│   ├── scan.rs         # Full repository audit scanner
│   ├── guard/          # Static analysis guards (security, complexity, symbols, api_drift, dead_code, arch, deps, duplicate)
│   ├── config/, findings/, baseline/ # Config loader, suppression pipeline & baseline partitioning
│   ├── comment/, turn/, review/, outcome/ # Directives, reviewed-file tracker, hunk marks & outcomes
│   ├── ipc/            # Multiplexer sinks, OSC 52 clipboard, AgentSink trait
│   ├── diff_source.rs  # DiffSource trait
│   └── watcher/        # Filesystem watcher & debouncer
└── lgtm-mcp/src/lib.rs # MCP JSON-RPC stdio server
```

`lgtm-core` and `lgtm-mcp` must not depend on `ratatui` or `crossterm`; `scripts/check-boundaries.sh` enforces this in CI and `make lint`.

---

## 5. Adding New Security Guard Rules

To add a new security detector or test fraud pattern:
1. Open [`crates/lgtm-core/src/guard/`](crates/lgtm-core/src/guard/) and select or create the appropriate submodule (`security.rs`, `arch.rs`, `complexity.rs`, etc.).
2. Add the detector logic to the subsystem scanner and hook it into `SecurityAndTestGuard::scan_file` or `scan_snapshot` in [`crates/lgtm-core/src/guard/mod.rs`](crates/lgtm-core/src/guard/mod.rs).
3. Add a corresponding test case checking that additions trigger alerts and deletions or valid code do not.
4. Run `make check`; add regression assertions for the new behavior.

---

## 6. Pull Request Guidelines

1. **Focused PRs**: Keep PRs small and focused on a single feature or bugfix.
2. **Test Coverage**: Accompany every code change with unit or integration tests.
3. **Commit Messages**: Use conventional commits (`feat: ...`, `fix: ...`, `docs: ...`, `refactor: ...`, `test: ...`).
4. **Clean Tree**: Verify `make check` passes cleanly before pushing.

### Verify claims against evidence

When changing tests, gates, diagnostics, or related documentation:

- Trace behavior claims to the implementation: what executes, what is measured,
  and what can make the command pass or fail. A command's name is not evidence.
- Check adjacent claims across README, wiki, contributor guide, CLI/TUI help,
  Make help, badges, scenario catalog, and roadmap. Update affected surfaces or
  record why they need no change.
- Distinguish measured execution coverage, fixed behavior checks, offline provider
  simulations, real-terminal tests, and live-provider verification.
- Give numerical claims a source, denominator or fixture, and scope. Date recorded
  benchmark observations; use generated evidence for badges that imply current
  measurements. Remove unsupported numbers.
- Report test execution and documentation review separately. Passing tests do not
  establish that documentation is accurate or acceptance coverage is complete.

Use the [PR template](.github/pull_request_template.md) to record the evidence and
remaining limits. Link and help-output checks supplement this semantic review;
they cannot establish that a documented claim matches the implementation.
