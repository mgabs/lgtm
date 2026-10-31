## Why

A read-only scan found little dead code but some drift: three `#[allow(dead_code)]` suppressions in `src/app/command.rs` hide whether items are used, and `cargo update --dry-run` shows 35 semver-compatible lockfile updates. Clearing both keeps the build honest and picks up fixes.

## What Changes

- Remove `#[allow(dead_code)]`; delete the never-read `category`, `args_summary`, `description` fields and `CommandCategory`; gate test-only `find_command` with `#[cfg(test)]`.
- Run `cargo update` (lockfile only, compatible versions).
- Confirm no other dead code: clippy with `-W dead_code -W unused` reported no warnings; `#![allow(unused_imports)]` in test modules is left as is.
- No behavior, CLI, or manifest-version changes.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
None. Maintenance only.

## Impact

- `src/app/command.rs`, `Cargo.lock`.
- CI (3-OS matrix) must stay green after the lockfile update.
