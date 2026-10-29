# Specification: Windows Native Support

## Capability: `windows-support`

### 1. Requirements

#### Cross-Platform Process Execution
- **REQ-WIN-01 (Command Resolution & PATHEXT Probing)**:
  - Subprocess execution via `lgtm_core::sys::process::resolve_program()` MUST resolve commands according to platform conventions.
  - On Windows:
    - If `program` does not include an extension, probe `PATHEXT` (defaulting to `.COM;.EXE;.BAT;.CMD`) across all directories in `PATH`.
    - Provide explicit zero-overhead fast-path mappings for known developer tools:
      - `"az"` -> `"az.cmd"` (Azure CLI)
      - `"code"` -> `"code.cmd"` (VS Code)
      - `"cursor"` -> `"cursor.cmd"` (Cursor IDE)
    - If found with an extension, invoke the resolved binary directly.
  - On Unix / macOS:
    - Pass through `program` unchanged.
- **REQ-WIN-02 (Subprocess Invocation Resilience)**:
  - Invoking external editors in `src/app/external.rs` (`launch_editor` / `spawn_cli_editor`) MUST use `resolve_program()` to ensure GUI editors like `code` and `cursor` launch cleanly from Windows Terminal or Command Prompt.
  - Forge operations in `crates/lgtm-core/src/forge/failure.rs` and `crates/lgtm-core/src/forge/azure.rs` MUST execute `az` using the resolved executable name (`az.cmd` on Windows), ensuring token acquisition and PR fetching succeed.

#### Cross-Platform Path Handling
- **REQ-WIN-03 (Extended-Length Path Prefix Sanitization)**:
  - All calls to filesystem canonicalization MUST use `lgtm_core::sys::path::canonicalize_clean(path)`.
  - On Windows, if canonicalization returns a verbatim prefix:
    - `\\?\C:\path\...` MUST be stripped to `C:\path\...`.
    - `\\?\UNC\server\share\...` MUST be converted to `\\server\share\...`.
  - The sanitized path MUST be returned as a standard `PathBuf` compatible with Git CLI commands, libgit2, and `path.strip_prefix()`.
  - On Unix, `canonicalize_clean` MUST delegate directly to `std::fs::canonicalize`.
- **REQ-WIN-04 (Path Separator Normalization for Git Keys)**:
  - When comparing paths between Git (which strictly uses forward slashes `/`) and the Windows filesystem (which uses backslashes `\`), `lgtm_core::sys::path::normalize_git_path(path)` MUST convert all backslashes to forward slashes.
  - File drawer indexing, comment anchor lookups (`#ref path:line`), and diff hunk associations MUST operate on normalized slash paths to guarantee cross-platform identity.

#### Line Ending & Diff Invariance (CRLF vs LF)
- **REQ-WIN-05 (CRLF Trimming in Diff Parsers)**:
  - Both unified diff parsing (`lgtm_core::pr::parse_unified_diff`) and git2 diff line callbacks (`lgtm_core::git::GitEngine::compute_diff_snapshot`) MUST trim trailing `\r` and `\n` characters from line contents (`line.trim_end_matches(['\r', '\n'])`).
  - Line counts, hunk header parsing (`@@ -o,l +n,l @@`), and visual line staging MUST NOT retain trailing `\r` bytes in memory.
- **REQ-WIN-06 (Deterministic Hunk Hashing)**:
  - `hunk_hash_u64()` and `hunk_hash()` MUST produce bit-identical hashes for equivalent hunk content regardless of whether the source files or diff streams use LF (`\n`) or CRLF (`\r\n`) line terminators.

#### Windows Clipboard Fallback
- **REQ-WIN-07 (Tiered Clipboard Delivery)**:
  - Yanking operations (`yy`, `yr`, `:sink clipboard`, violation remediation yank) MUST execute through `lgtm_core::sys::clipboard::copy_to_clipboard(text)`.
  - Delivery sequence:
    1. Send ANSI OSC 52 sequence to stdout (preferred for modern terminals including Windows Terminal, Alacritty, WezTerm).
    2. On Windows, if OSC 52 is unacknowledged or when native clipboard sink is selected, pipe text via standard input to `clip.exe` (`%SystemRoot%\System32\clip.exe`).
    3. If `clip.exe` is unavailable or fails, fallback to executing PowerShell `Set-Clipboard -Value ...`.
  - Ensure zero console window flash during subprocess invocation by setting `CREATE_NO_WINDOW` (`0x08000000`) flag in Windows process creation attributes.

#### CI Matrix & Release Packaging
- **REQ-WIN-08 (Windows CI Matrix)**:
  - `.github/workflows/ci.yml` `quality-gate` job MUST include `windows-latest` in its matrix strategy (`os: [ubuntu-latest, macos-latest, windows-latest]`).
  - Shell execution in workflow steps MUST specify `shell: bash` or cross-platform command syntax to ensure portable execution on Windows runners.
  - Crate boundary checking (`./scripts/check-boundaries.sh`) MUST execute via `bash ./scripts/check-boundaries.sh` on all platforms.
- **REQ-WIN-09 (Windows Release Artifact Packaging)**:
  - On `windows-latest`, the release packaging step MUST:
    - Locate `target/release/lgtm.exe`.
    - Package the binary into a zip archive: `lgtm-v<VERSION>-windows-x86_64.zip`.
    - Generate a SHA-256 checksum file: `lgtm-v<VERSION>-windows-x86_64.zip.sha256`.
  - Attach the `.zip` and `.sha256` files to GitHub releases alongside Linux and macOS tarballs.

---

### 2. Non-Functional Requirements & Performance Budgets

- **NFR-WIN-01 (Process Resolution Latency)**:
  - `resolve_program()` caching and path checks MUST complete in $< 1\text{ ms}$ for cached commands and $< 10\text{ ms}$ for cold PATH scans.
- **NFR-WIN-02 (Zero Native C Dependencies)**:
  - Implementation MUST NOT add runtime dependencies on external C DLLs or COM runtime libraries beyond standard Win32 system libraries (`kernel32`, `user32`).
- **NFR-WIN-03 (Binary Footprint Parity)**:
  - The compiled release binary `lgtm.exe` MUST NOT exceed 15 MB in size.
- **NFR-WIN-04 (Terminal Frame Timing)**:
  - TUI rendering on Windows Terminal and conhost via Crossterm MUST maintain stable 60 FPS ($< 16\text{ ms}$ draw pass) without flicker.

---

### 3. Assumptions

- **ASM-WIN-01**: Target Windows operating system is Windows 10 Version 1809 (Build 17763) or higher, or Windows 11, with Virtual Terminal Processing enabled by Crossterm.
- **ASM-WIN-02**: Git for Windows ($\ge 2.25$) is installed and accessible on `PATH`.
- **ASM-WIN-03**: Built-in Windows utility `clip.exe` is present in `%SystemRoot%\System32\` on all standard Windows desktop and server installations.

---

### 4. Edge Cases & Errors

- **EDG-WIN-01 (Verbatim UNC Network Paths)**:
  - When a repository is hosted on a network share (e.g. `\\server\share\repo`), `canonicalize()` yields `\\?\UNC\server\share\repo`. `canonicalize_clean()` MUST convert this specifically to `\\server\share\repo` rather than stripping the first 4 characters blindly.
- **EDG-WIN-02 (Case-Insensitive File Systems)**:
  - Windows file systems (NTFS, ReFS) are case-preserving but case-insensitive. Internal path lookups for dirty file matching MUST normalize path comparisons or preserve Git's exact case from tree entries.
- **EDG-WIN-03 (Extended-Length File Paths > 260 Characters)**:
  - For deep repositories exceeding `MAX_PATH` (260 characters), Windows long path support MUST be enabled via application manifest or fallback handling in file operations without breaking Git CLI interactions.
- **EDG-WIN-04 (PowerShell Execution Policy Restrictions)**:
  - Fallback clipboard operations MUST NOT invoke `.ps1` script files directly to avoid failures under `Restricted` PowerShell Execution Policies. `clip.exe` or `powershell.exe -NoProfile -Command` MUST be used.

---

### 5. Behavioral Scenarios (Given / When / Then)

#### Scenario 5.1: Execute Azure CLI on Windows
- **Given** user is running `lgtm` on Windows 11 with Azure CLI installed at `C:\Program Files\Microsoft SDKs\Azure\CLI2\wbin\az.cmd`
- **When** user triggers Azure DevOps PR inspection (`lgtm pr 1234` or `:pr`)
- **Then** `resolve_program("az")` resolves to `"az.cmd"`
- **And** the subprocess executes successfully without `NotFound` errors, returning the access token and PR details.

#### Scenario 5.2: Path Canonicalization in Git Worktree
- **Given** active worktree is located at `C:\Projects\lgtm\.worktrees\feat-win`
- **When** `canonicalize_clean()` is invoked on the worktree path
- **Then** the returned `PathBuf` is `C:\Projects\lgtm\.worktrees\feat-win` without `\\?\` prefix
- **And** `path.strip_prefix(repo_root)` resolves relative file paths without error.

#### Scenario 5.3: Clipboard Yanking in Classic Conhost
- **Given** user runs `lgtm.exe` inside a classic `cmd.exe` conhost terminal lacking OSC 52 support
- **When** user presses `yr` on line 42 of `src/main.rs`
- **Then** `lgtm` sends OSC 52 and pipes `#ref src/main.rs:42` to `clip.exe`
- **And** Windows clipboard contains `#ref src/main.rs:42` with status notification `Copied to clipboard`.

#### Scenario 5.4: Reviewing a PR with Mixed CRLF Line Endings
- **Given** a PR diff where files have `\r\n` Windows line endings
- **When** `parse_unified_diff` builds the `DiffSnapshot`
- **Then** all `DiffLine.content` strings contain clean text without trailing `\r`
- **And** hunk hash matches the expected hash generated on Linux/macOS.
