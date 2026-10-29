# Feature Proposal: Windows Native Support

## Intent

Deliver native, zero-friction Windows support for `lgtm`, enabling developers and AI agents running on Windows 10/11 (PowerShell, Windows Terminal, and Command Prompt) to inspect diffs, execute code steering directives, and interact with GitHub and Azure DevOps pull requests with identical performance, rendering fidelity, and ergonomics as Linux and macOS.

## Scope

### In-Scope
- **Cross-Platform Process Execution (`lgtm-core::sys::process`)**:
  - Transparently resolve executable batch and command wrappers on Windows (`az.cmd`, `code.cmd`, `cursor.cmd`) without hardcoding paths or requiring users to run in Git Bash.
  - Consistent subprocess spawning for Git, Azure CLI (`az`), GitHub CLI (`gh`), and external editors (`code`, `cursor`, `subl`, `hx`, `notepad`).
- **Path Sanitization & Normalization (`lgtm-core::sys::path`)**:
  - Automatically strip Windows extended-length UNC path prefixes (`\\?\` and `\\?\UNC\`) produced by `std::fs::canonicalize`, preventing Git CLI and `strip_prefix` path calculation errors.
  - Bidirectional conversion between Windows filesystem backslashes (`\`) and Git index forward slashes (`/`) for deterministic file tree indexing, comment anchors, and diff hunks.
- **Line Ending Invariance (CRLF vs LF)**:
  - Normalize line endings across unified diff parsing, git2 callbacks, and hunk hash generation so that repositories with `core.autocrlf = true` do not produce spurious hunk mismatches, duplicate lines, or broken directive anchors.
- **Native Clipboard Integration (`lgtm-core::sys::clipboard`)**:
  - Support clipboard operations (`yy`, `yr`, `:sink clipboard`) on Windows via standard OSC 52 escape sequences with fallback to native `clip.exe` (and PowerShell `Set-Clipboard` secondary fallback) when running inside legacy Windows console hosts (`conhost.exe`).
- **CI Matrix & Quality Gate (`.github/workflows/ci.yml`)**:
  - Add `windows-latest` to the GitHub Actions CI matrix for unit tests, clippy linter, and integration tests.
  - Cross-platform test suite maintenance ensuring 100% test pass rate on Windows.
- **Release Packaging**:
  - Automate building and packaging `lgtm.exe` release binaries into `.zip` archives (`lgtm-v<VERSION>-windows-x86_64.zip`) with SHA-256 checksums published alongside Linux and macOS tarballs.

### Out-of-Scope (YAGNI)
- **Windows MSI / InnoSetup Installer**: Distributing portable `.zip` archives and supporting `cargo install` is sufficient for developer tooling; enterprise MSI installers are deferred until requested.
- **Custom Win32 GDI / DirectX Renderer**: `crossterm` and `ratatui` already target the Windows Virtual Terminal (VT) engine; raw Win32 console buffer rewriting is unnecessary.
- **Winget / Chocolatey Registry Automation**: Package manager recipes can be community-maintained post-release once the official `.zip` asset distribution is stable.
- **WSL Interop Shim**: Windows Subsystem for Linux already runs the native Linux ELF binary; native Windows support targets the host OS without requiring WSL.

## Problem Statement

Currently, `lgtm` is built, tested, and distributed solely for Linux and macOS. When developers attempt to run or compile `lgtm` on Windows, they encounter several architectural and environmental roadblocks:
1. **Subprocess Failure on Windows Batch Wrappers**: External CLI tools like `az` (Azure CLI) and `code` (VS Code) are installed as `.cmd` batch files on Windows (e.g. `az.cmd`, `code.cmd`). Rust's `std::process::Command::new("az")` delegates directly to the Win32 `CreateProcessW` API, which fails with `ERROR_FILE_NOT_FOUND` (exit error) because it does not probe the `PATHEXT` environment variable (`.COM;.EXE;.BAT;.CMD`).
2. **Path Prefix Pollution (`\\?\`)**: Invoking `std::fs::canonicalize()` on Windows prepends the verbatim path prefix `\\?\` (e.g., `\\?\C:\Users\repo`). Passing these paths to the Git CLI, comparing them against Git's slash-separated paths, or calculating relative paths via `path.strip_prefix(repo_root)` fails consistently, breaking worktree switches, comment persistence, and file tree navigation.
3. **CRLF Line Ending Mismatches**: Windows checkouts often convert line endings to CRLF (`\r\n`). Incomplete stripping of carriage returns (`\r`) in diff parsers causes `hunk_hash` collisions, visual gutter corruption, and directive anchoring failures when diffing across branches or forges.
4. **Clipboard Sinks Broken on Classic Conhost**: `lgtm` currently relies entirely on ANSI OSC 52 escape sequences for clipboard copying (`yy`, `yr`). Classic Command Prompt and older Windows PowerShell consoles ignore OSC 52, leaving users unable to copy code references or agent directives without external multiplexers.
5. **No Automated Windows CI Validation**: With CI running only on `ubuntu-latest` and `macos-latest`, Windows-breaking regressions enter the main branch undetected, and Windows users cannot download pre-compiled release binaries.

## Goals, Non-Goals, and Anti-Goals

### Goals
- **First-Class Windows Compatibility**: Every core capability—diff viewing, git staging, comments/directives, AST folding, forge synchronization (GitHub & Azure DevOps), and MCP server—must function identically on Windows as on macOS/Linux.
- **Zero Heavy Dependencies**: Avoid heavyweight Windows-specific C-runtime dependencies, COM wrappers, or external GUI crates. Keep binary footprint small (< 15MB) using native Win32/System32 utilities (`clip.exe`, `cmd.exe`).
- **Transparent Command Execution**: Automatically resolve `.cmd` and `.bat` executables for forge and editor workflows without user configuration.
- **Continuous Verification**: Enforce quality gates on `windows-latest` for every pull request and release build.

### Non-Goals (YAGNI)
- Re-architecting terminal input handling away from `crossterm`.
- Supporting obsolete Windows versions prior to Windows 10 Version 1809 (Build 17763) which lacks native ConPTY and ANSI VT100 support.
- Windows-specific configuration file formats (continue using standard `.lgtm.toml` in repository root or user config directory).

### Anti-Goals
- Never sprinkle unstructured `#[cfg(windows)]` directives across UI views or business logic. All platform distinctions must be encapsulated cleanly within `lgtm-core::sys`.
- Never mutate or re-encode repository file contents on disk when handling CRLF line endings.

## Success Metrics

- **CI Matrix Coverage**: 100% test passing rate on `windows-latest` across all crates (`lgtm`, `lgtm-core`, `lgtm-ast`, `lgtm-mcp`).
- **Forge Compatibility**: 100% functional parity for Azure DevOps and GitHub pull request fetching, thread replies, and diff inspection on Windows.
- **Subprocess Parity**: 0% failure rate when invoking `az.cmd`, `code.cmd`, `cursor.cmd`, or `git.exe`.
- **Release Parity**: Automated `.zip` bundle produced on every tag release containing `lgtm.exe` and valid SHA-256 checksum.
