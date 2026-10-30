## Context

Scan results: no clippy dead-code/unused warnings; `cargo machete`/`udeps` are not installed, but each direct dependency is imported in `src/`. Lockfile has 35 compatible updates (e.g. serde_json, uuid, icu_*, pest, wasm-bindgen).

## Goals / Non-Goals

**Goals:** drop the dead_code suppressions; refresh the lockfile.
**Non-Goals:** major-version bumps (e.g. crossterm 0.28 → newer), feature changes, removing test-module `unused_imports` allows.

## Decisions

- `find_command` is test-only: gate with `#[cfg(test)]` (keeps existing tests, no production dead code). Alternative, deleting it, loses test coverage of alias lookup.
- Lockfile-only update via `cargo update`, not manifest edits: lowest risk, reviewable diff.

## Risks / Trade-offs

- A transitive update could break Windows/macOS CI → run `cargo test --workspace` and `cargo clippy` locally; rely on the CI matrix for Windows.
- `pest_stack_guard` and `utf_types` are newly added transitive crates; acceptable, check `cargo tree` for licenses if policy requires.
