# Windows Native Support Implementation Plan

> **Goal:** Deliver native Windows support for `lgtm`, providing cross-platform process execution (`az.cmd`, `code.cmd`), path prefix sanitization (`\\?\`), CRLF line ending invariance, clipboard fallbacks (`clip.exe`), and `windows-latest` CI/release automation.

**Architecture:** 
- Centralized OS abstraction in `crates/lgtm-core/src/sys/` (`process.rs`, `path.rs`, `clipboard.rs`).
- Transparent integration into `lgtm-core::forge`, `src/app/external.rs`, and `src/main.rs`.
- CI/CD quality gate and release packaging in `.github/workflows/ci.yml`.

**Spec:** [`specs/features/windows-support/spec.md`](spec.md) | **Design:** [`specs/features/windows-support/design.md`](design.md)

---

### Phase 1: Core OS Abstraction Layer (`crates/lgtm-core/src/sys/`)

- [x] **Step 1: Scaffold `sys` module structure**
  - Create `crates/lgtm-core/src/sys/mod.rs`, `process.rs`, `path.rs`, `clipboard.rs`.
  - Re-export `process::resolve_program`, `path::canonicalize_clean`, `path::normalize_git_path`, and `clipboard::copy_to_clipboard` in `sys/mod.rs`.
  - Expose `pub mod sys;` in `crates/lgtm-core/src/lib.rs`.

- [x] **Step 2: Implement `process.rs`**
  - Implement `resolve_program(program: &str) -> Cow<'_, str>`.
  - Implement Windows-specific fast-paths for `"az"` -> `"az.cmd"`, `"code"` -> `"code.cmd"`, `"cursor"` -> `"cursor.cmd"`.
  - Implement general `PATHEXT` probing fallback for unrecognized programs on Windows.
  - Implement `create_command(program: &str) -> Command` configuring `CREATE_NO_WINDOW` on Windows for utility subprocesses.

- [x] **Step 3: Implement `path.rs`**
  - Implement `strip_verbatim_prefix(path: PathBuf) -> PathBuf` handling `Prefix::VerbatimDisk` and `Prefix::VerbatimUNC`.
  - Implement `canonicalize_clean<P: AsRef<Path>>(path: P) -> io::Result<PathBuf>` calling `std::fs::canonicalize` and stripping verbatim prefixes.
  - Implement `normalize_git_path(path: &Path) -> String` replacing backslashes with forward slashes.

- [x] **Step 4: Implement `clipboard.rs`**
  - Implement `copy_to_clipboard(text: &str) -> io::Result<()>`.
  - Wire OSC 52 emission as primary tier.
  - Add Windows-specific `copy_via_clip_exe(text: &str)` piping to `%SystemRoot%\System32\clip.exe`.
  - Add secondary fallback `copy_via_powershell(text: &str)` using `Set-Clipboard`.

- [x] **Step 5: Unit tests for `sys` module**
  - Add unit tests in `crates/lgtm-core/src/sys/path.rs` verifying verbatim prefix removal on test strings.
  - Add unit tests in `crates/lgtm-core/src/sys/process.rs` validating fast-path mappings.
  - Add unit tests in `crates/lgtm-core/src/sys/clipboard.rs` validating non-panicking execution.

---

### Phase 2: Process Resolution & Forge Integration

- [x] **Step 1: Integrate `resolve_program` into Forge Runner**
  - Update `crates/lgtm-core/src/forge/failure.rs`:
    - Wrap `Command::new(program)` with `lgtm_core::sys::process::resolve_program(program)`.
  - Update `crates/lgtm-core/src/forge/azure.rs`:
    - Ensure `az` token acquisition and PR queries execute using resolved binary.

- [x] **Step 2: Integrate `resolve_program` into Editor Spawning**
  - Update `src/app/external.rs`:
    - In `launch_editor`, use resolved program names for `code`, `cursor`, and CLI editors.
    - In `spawn_cli_editor`, resolve binary before passing to `std::process::Command::new(bin)`.

- [x] **Step 3: Verification of process execution**
  - Test subprocess spawning with simulated `.cmd` and `.bat` files in test harness.

---

### Phase 3: Path Normalization Across Codebase

- [x] **Step 1: Replace bare `std::fs::canonicalize` with `canonicalize_clean`**
  - Update `src/main.rs`: replace all occurrences of `std::fs::canonicalize(&target)` with `lgtm_core::sys::path::canonicalize_clean(&target)`.
  - Update `crates/lgtm-core/src/git/mod.rs`: replace `canonicalize()` calls in worktree discovery and repo path initialization.

- [x] **Step 2: Normalize paths for Git index comparisons**
  - In `crates/lgtm-core/src/git/mod.rs`, ensure relative path stripping accounts for normalized paths.
  - Verify `#ref path:line` comment generation uses forward slashes.

- [x] **Step 3: Verification of path sanitization**
  - Verify on Windows (or simulated paths) that `\\?\C:\repo` strips cleanly and matches relative tree files.

---

### Phase 4: Line Ending Invariance (CRLF Normalization)

- [x] **Step 1: Audit and update diff parsers**
  - In `crates/lgtm-core/src/pr/mod.rs`: ensure `raw_line.trim_end_matches(['\r', '\n'])` is applied across hunk headers and line contents.
  - In `crates/lgtm-core/src/git/mod.rs`: verify `line_cb.content()` is stripped of `\r` and `\n`.

- [x] **Step 2: Verify hunk hashing invariance**
  - Ensure `hunk_hash_u64()` produces identical output for CRLF vs LF inputs.
  - Add regression test in `crates/lgtm-core/src/review/tests.rs` with CRLF diff inputs.

---

### Phase 5: Windows Clipboard Fallback

- [x] **Step 1: Update TUI clipboard actions**
  - In `crates/lgtm-core/src/ipc/mod.rs` (`send_osc52_clipboard`):
    - Delegate to `crate::sys::copy_to_clipboard` to enable transparent tiered delivery (`OSC 52` -> `clip.exe` -> PowerShell).
  - Automatically covers `src/app/external.rs`, `src/app/input.rs`, and `crates/lgtm-core/src/ipc/sink.rs`.

- [x] **Step 2: Verification of clipboard fallback**
  - Verify yanking works when OSC 52 is not processed by standard console.

---

### Phase 6: GitHub Actions CI Matrix & Packaging

- [x] **Step 1: Add `windows-latest` to `.github/workflows/ci.yml`**
  - Add `windows-latest` to `jobs.quality-gate.strategy.matrix.os`.
  - Set `shell: bash` on all workflow steps to ensure consistent shell execution.

- [x] **Step 2: Implement Windows release packaging step**
  - Add conditional step `Package Release Binary (Windows)` for `matrix.os == 'windows-latest'`.
  - Create `lgtm-v<VERSION>-windows-x86_64.zip` containing `target/release/lgtm.exe`.
  - Generate SHA-256 checksum file `lgtm-v<VERSION>-windows-x86_64.zip.sha256`.
  - Update `Upload Release Asset Artifact` to capture `.zip` files.

- [x] **Step 3: Update GitHub Release publication step**
  - Ensure `action-gh-release` includes `release-artifacts/*.zip` in the release asset list.

---

### Phase 7: Verification & BDD Tests

- [x] **Step 1: Run unit and clippy test suite**
  - Run `cargo fmt --all -- --check`
  - Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - Run `cargo test --workspace`

- [x] **Step 2: Add BDD feature test**
  - Add `bdd_win_001_windows_path_and_process_resolution` in `tests/bdd.rs`.
