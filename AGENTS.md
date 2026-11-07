# Agent guide

- Workspace crates: `lgtm-core`, `lgtm-mcp`, `lgtm-ast` (+ root `src/`, `tests/`).
- Quality gates: `make fmt`, `make lint`, `make test`, `make bdd-report`, `make check`; `make coverage` enforces 90% line coverage.
- BDD scenarios map to Rust tests and `docs/bdd-scenarios.md`; behavior specs live in `openspec/`.
- Serena: prefer its symbol tools for Rust navigation; each git worktree is its own Serena project, so run `serena project index` in new worktrees.
- Optional ast-grep MCP configuration in `.mcp.json` requires `uvx` and fetches the server from its upstream Git repository; it is not required for builds or tests.
- Model routing: utility/scans (`research`, `code-janitor`) must use `Model: 'flash_lite'` or `'flash'`; reserve `pro` for `system-architect` or `deep-thinker`.
- Token hygiene: trigger `/compact` when active tokens cross 100k; delegate heavy diffs and scans to worktrees (`Workspace: 'share'` / `'branch'`).
