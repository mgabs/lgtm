## 1. Dead code

- [x] 1.1 Remove `#[allow(dead_code)]` from `CommandCategory` and `CommandDef` in `src/app/command.rs`; confirm no warnings
- [x] 1.2 Gate `find_command` with `#[cfg(test)]` and remove its `#[allow(dead_code)]`
- [x] 1.3 Run `cargo clippy --workspace --all-targets -- -D warnings`

## 2. Dependency updates

- [x] 2.1 Run `cargo update` and review the `Cargo.lock` diff
- [x] 2.2 Run `cargo test --workspace` and `cargo clippy --workspace --all-targets`
- [x] 2.3 Run `cargo build --release` to confirm LTO build still works

## 3. Wrap-up

- [x] 3.1 Run `openspec validate --all --strict`
