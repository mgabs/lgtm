# windows-support Specification

## Purpose

Make lgtm run natively on Windows with the same review behavior as Unix: correct process resolution, path handling, line endings, clipboard delivery, CI and release packaging.

## Requirements

### Requirement: Platform-Aware Command Resolution
Subprocess execution SHALL resolve programs per platform. On Windows, a program without an extension MUST be resolved by probing `PATHEXT` (default `.COM;.EXE;.BAT;.CMD`) across `PATH`, with fast-path mappings `az`→`az.cmd`, `code`→`code.cmd` and `cursor`→`cursor.cmd`. On Unix and macOS the program MUST pass through unchanged. Resolution MUST take under 1 ms when cached and under 10 ms on a cold scan.

#### Scenario: Azure CLI on Windows
- **WHEN** Azure DevOps PR inspection runs on Windows with Azure CLI installed
- **THEN** `az` resolves to `az.cmd` and token and PR fetches succeed without a NotFound error

#### Scenario: Unix passthrough
- **WHEN** resolving `az` on Linux or macOS
- **THEN** the program name is unchanged

### Requirement: Resilient External Invocation
External editors (including GUI editors such as `code` and `cursor`) and forge operations that call `az` SHALL launch through the resolved executable so they work from Windows Terminal and Command Prompt.

#### Scenario: GUI editor launch
- **WHEN** the user opens a file in `code` from the TUI on Windows
- **THEN** the editor launches without a command-not-found error

### Requirement: Verbatim Path Prefix Sanitization
Filesystem canonicalization SHALL strip Windows verbatim prefixes: `\\?\C:\path` MUST become `C:\path` and `\\?\UNC\server\share` MUST become `\\server\share`. The result MUST work with Git CLI, libgit2 and prefix stripping. On Unix, canonicalization MUST delegate to the standard library.

#### Scenario: Worktree path
- **WHEN** the worktree path `C:\Projects\lgtm\.worktrees\feat-win` is canonicalized
- **THEN** the result has no `\\?\` prefix and stripping the repo root yields relative file paths

#### Scenario: UNC share
- **WHEN** a repository on `\\server\share\repo` is canonicalized
- **THEN** the result is `\\server\share\repo`, not a blindly truncated string

### Requirement: Slash-Normalized Git Paths
Path comparisons between Git (forward slashes) and the Windows filesystem SHALL use forward-slash normalized paths, including file drawer indexing, `#ref path:line` anchors and diff hunk associations. Dirty-file matching MUST tolerate case-insensitive filesystems by normalizing comparisons or preserving Git's case.

#### Scenario: Backslash input
- **WHEN** a comment anchor is looked up with `src\main.rs`
- **THEN** it matches the diff file `src/main.rs`

### Requirement: CRLF-Invariant Diffs
Diff parsing, from both unified-diff text and libgit2 line callbacks, SHALL trim trailing `\r` and `\n` from line contents, and hunk headers, line counts and visual line staging MUST NOT retain `\r`. Hunk hashes MUST be identical for LF and CRLF forms of equivalent content.

#### Scenario: Mixed line endings
- **WHEN** a PR diff with `\r\n` line endings is parsed
- **THEN** no `DiffLine` content ends with `\r`
- **AND** the hunk hash equals the one generated on Linux or macOS

### Requirement: Tiered Clipboard Delivery
Yank operations SHALL try, in order: an OSC 52 sequence on stdout; on Windows, `clip.exe` via standard input when OSC 52 is unacknowledged or the native sink is selected; then PowerShell `Set-Clipboard` invoked with `-NoProfile -Command` (never a `.ps1` file). Subprocesses MUST set `CREATE_NO_WINDOW` so no console window flashes.

#### Scenario: Classic conhost
- **WHEN** the user presses `yr` on line 42 of `src/main.rs` in `cmd.exe` without OSC 52 support
- **THEN** `#ref src/main.rs:42` is piped to `clip.exe`, the clipboard contains it and `Copied to clipboard` is shown

#### Scenario: clip.exe unavailable
- **WHEN** `clip.exe` is missing or fails
- **THEN** PowerShell `Set-Clipboard` is used

### Requirement: Windows CI Matrix
The CI quality gate SHALL run on `ubuntu-latest`, `macos-latest` and `windows-latest`, with portable step shells, and the crate boundary check MUST run via `bash` on all platforms.

#### Scenario: Windows job present
- **WHEN** CI runs on a pull request
- **THEN** a `windows-latest` quality-gate job runs formatting, lints and tests

### Requirement: Windows Release Packaging
On Windows, release packaging SHALL zip `target/release/lgtm.exe` as `lgtm-v<VERSION>-windows-x86_64.zip`, generate a `.sha256` checksum file, and attach both to the GitHub release alongside the Linux and macOS archives. `lgtm.exe` MUST NOT exceed 15 MB.

#### Scenario: Release artifacts
- **WHEN** a release is published
- **THEN** the zip and its `.sha256` are attached next to the Linux and macOS tarballs

### Requirement: Windows Runtime Constraints
The Windows build SHALL NOT add runtime dependencies on external C DLLs or COM libraries beyond standard Win32 system libraries, MUST support long paths over 260 characters without breaking Git CLI interactions, and TUI rendering via Crossterm on Windows Terminal and conhost MUST hold a stable 60 FPS (under 16 ms per draw) without flicker.

#### Scenario: Deep repository
- **WHEN** a repository contains paths longer than 260 characters
- **THEN** file operations succeed and Git CLI interactions are unaffected
