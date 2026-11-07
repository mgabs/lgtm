# Proposal

## Intent
Consolidate the `lgtm` CLI entry point by removing obsolete and deprecated subcommands (`serve`, `prs`, `status`, `ci`), eliminating redundant top-level flags (`--init`, `--pr`), and establishing a cohesive 10-command CLI surface with unambiguous semantics and unified help text.

## Problem Statement
The `lgtm --help` command lists 17 subcommands with significant functional overlap, multiple deprecated aliases, and redundant top-level flags. For example:
- `serve` (`--mcp`) lingers as a deprecated alias for `mcp` while still referenced across documentation.
- `status` and `diff` duplicate bare `lgtm [REVISION]` for launching the diff review TUI.
- `ci` duplicates `check --no-user-config`.
- `prs` duplicates `pr list`.
- `risk` duplicates diff checking already provided by `check`.
- Top-level `--init` and `--pr` duplicate the `init` and `pr` subcommands.

This bloats the CLI help output, increases cognitive load for users and AI agents, and creates redundant code paths across CLI parsing, tests, and documentation.

## Affected Crates & CI Impact
- **Affected Crates**: `lgtm` (CLI definition in `src/cli.rs`, dispatch in `src/main.rs`, and tests).
- **CI Impact**: Zero impact on OS matrix (Ubuntu, macOS, Windows). All changes modify CLI argument parsing and dispatch. `Makefile` targets (`ci`, `risk`) will be updated to use canonical commands (`check`).

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `clean-commands-shortcuts`: Remove deprecated aliases (`serve`, `prs`, `status`, `ci`), remove duplicate `--init` flag, and update canonical CLI command definitions and scenarios.

## Scope

### In-Scope
- Remove `Commands::Serve` and `--mcp` flag from `src/cli.rs` and `src/main.rs`.
- Remove `Commands::Status`, `Commands::Prs`, and `Commands::Ci` from `src/cli.rs` and `src/main.rs`.
- Remove redundant top-level flags `--init` and `--pr` from `Cli` struct in `src/cli.rs`.
- Fold `Commands::Risk` into `Commands::Check` (or maintain `lgtm check` as the unified audit command).
- Ensure `lgtm review` requires an explicit action (`export` or `import`), removing ambiguous fallback to TUI launch.
- Update `Makefile` targets (`ci:`, `risk:`) to call canonical `cargo run -- check`.
- Update CLI unit tests in `src/cli.rs`, integration tests, and BDD scenario [`bdd_cli_010`](file:///Users/mmetawea/Workbench/lgtm/tests/bdd.rs#L122) to use `lgtm init`.
- Update documentation (`README.md`, `docs/WIKI.md`, `docs/bdd-scenarios.md`, `CHANGELOG.md`).
- Update `openspec/specs/clean-commands-shortcuts/spec.md`.

### Out-of-Scope (YAGNI)
- No changes to TUI keybindings, modals, or Ex-command syntax (`:q`, `:wq`, `:filter`, etc.).
- No changes to `lgtm-core` audit rules, AST parsers, or MCP protocol handler logic.
- No new third-party CLI crates or dependencies (keep existing `clap` configuration).
- No network changes or forge authentication alterations.

## Assumptions
- **ASM-GEN-1**: AI coding assistants and MCP clients will invoke `lgtm mcp` rather than the legacy `lgtm serve --mcp`.
- **ASM-GEN-2**: Existing scripts running `lgtm ci` can migrate to `lgtm check --no-user-config` with identical exit code behavior and report generation.
- **ASM-GEN-3**: The `init` subcommand fully covers the initialization workflow without requiring a top-level `--init` flag.

## Success Metrics
- `lgtm --help` outputs a concise, non-redundant list of canonical subcommands.
- All deprecated commands (`serve`, `prs`, `status`, `ci`) and top-level duplicate flags (`--init`, `--pr`) are eradicated.
- `make check` and the 90% workspace coverage gate pass cleanly.
- `clean-commands-shortcuts` specification matches current CLI behavior.

## Review Gate
- [x] **Gate 1 Approval**: Proposal reviewed and approved by human before drafting specs.
