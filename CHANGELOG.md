# Changelog

## 2.7.1

### Removed
- **BREAKING**: Removed deprecated subcommands `lgtm serve` (use `lgtm mcp`), `lgtm prs` (use `lgtm pr list`), `lgtm status` (use `lgtm` or `lgtm diff`), and `lgtm ci` (use `lgtm check --no-user-config`).
- **BREAKING**: Removed duplicate top-level flags `--init` (use `lgtm init [--repo <path>]`) and `--pr` (use `lgtm pr <number>`).
- **BREAKING**: Removed `lgtm risk` (folded into `lgtm check`).
- **BREAKING**: `lgtm review` now requires an explicit action (`export` or `import`) instead of ambiguously launching the TUI.

### Added
- Static analysis guard rules for Rust source code: AST detection for dynamic SQL injection via `format!` (`sec/sql-injection`), shell command injection via `Command::new("sh" | "bash" | ...).arg("-c")` (`sec/command-injection`), and un-gated tautological assertion detection (`test/tautological-assert`) across all files including inline tests in `src/*.rs`.
- Markdown preview navigation shortcuts: `e` / `o` to launch external editor directly at cursor line, `[` / `]` to trigger diff bracket navigation chords (`]f`, `]h`, `]c`, etc.), `f` / `Ctrl-p` to open file picker, `v` to toggle reviewed status, `r` to reply to comments, and `?` for help modal.
- Live diff snapshot application in Markdown Preview (`ActiveView::MarkdownPreview`) and automatic cache invalidation on `diff_revision` bump while preserving and clamping cursor position.
- Linked git worktree administrative path classification (`.git/worktrees/<name>/index`, `HEAD`, branch refs) in `lgtm_core::watcher::affects_diff`.
- Dynamic file watcher rebinding via `WatcherHandle` on `:wt` worktree switch.

### Fixed
- Static analysis guard false positives: tightened `sec/sql-injection` tree-sitter AST query to require structured SQL clauses (`SELECT...FROM`, `INSERT INTO`, `UPDATE...SET`, `DELETE FROM`), preventing false flags on UI/CLI logging strings like `format!("Deleted...")` or `format!("Selected...")`.
- Test skip discrimination: fixed `test/skipped` heuristic so `std::process::exit(...)` is not falsely flagged as a Jest `xit(` skip.
- Tautological assertion detection: sanitized internal guard description strings, test fixtures, and help text to prevent self-flagging.
- Markdown preview cache staleness: diff reload now rebuilds rendered markdown rows automatically via `cached_diff_revision`.

## 2.6.2
- Consistent `Ctrl-n` / `Ctrl-p` item navigation, page movement, and top/bottom jumps across pickers and document views; non-text pickers also close with `Ctrl-c` and `q`.
- Shared word deletion in text inputs (`Ctrl-w`, `Alt-Backspace`, `Ctrl-Backspace`), command-line word motion/deletion (`Alt-b` / `Alt-f`, `Ctrl-Left` / `Ctrl-Right`, `Alt-d`), and line deletion (`Ctrl-u` / `Ctrl-k`).
- Direct online PR commenting: `:thread comment`, `:thread reply`, and `:thread resolve` post asynchronously directly to the remote forge without draft staging.
- Local directive commands (`:directives`, `:reply`, `:resolve`, `:unresolve`) no longer route to remote threads based on cursor position; use `:thread` for PR discussion actions.
- In-memory `CommentRetryBuffer`: failed comment submissions retain typed text in memory with `:thread retry`, eliminating local draft disk files.
- Outdated PR threads are fetched from GitHub and detected against the active diff for Azure DevOps; the TUI marks, filters, and navigates them without storing PR data locally.
- Dedicated `DirectiveStore` and `Directive` subsystem in `crates/lgtm-core/src/directive/` storing local review directives exclusively in `.lgtm/directives.jsonl`.
- Conversion of static guard scanner alerts (`GuardAlert`) into review directives.

### Removed
- **BREAKING**: Removed `:thread submit`, `:thread drafts`, `:thread discard`, `:thread edit`, the `.lgtm/drafts/` disk queue, and its unused draft-queue code.

## 2.6.1

### Added
- Markdown preview: `mermaid` blocks get a `[mermaid: <type>]` header, and `graph`/`flowchart` edges render as `A ──▶ B` rows; anything else stays verbatim.
- Markdown preview: a highlighted cursor row (directive commands act on it) and mouse-wheel scrolling; mouse capture is enabled only while the preview is open.

### Tests
- Markdown parser: `src/app/markdown.rs` now has 100% line and branch coverage, and scoped mutation testing leaves only equivalent mutants.

### Fixed
- Windows CI: staging tests no longer depend on `core.autocrlf`; worktree jump-by-path matches paths without the `\\?\` prefix.
- Terminal state is restored after UI-loop errors, failed agent delivery is reported without advancing the review turn, `--init --repo` honors its target, and MCP directive line overflow is rejected before persistence.

### Changed
- Specs now live in `openspec/` (legacy `specs/` archived under `openspec/changes/archive/`).
- Removed unused command-registry metadata and `#[allow(dead_code)]` suppressions; refreshed `Cargo.lock`.

## 2.6.0

### Added
- Online PR comments: live threads, local drafts (`:thread comment|general|reply|resolve|unresolve|edit|discard|drafts`), and a confirmed batch `:thread submit` with per-item results, stale detection and duplicate-safe retries; MCP `lgtm_list_threads`; pending inline drafts show a `✎` gutter marker and an inline row (pending/stale/failed) in the diff view. On GitHub, new inline comments in a submit are posted as a single `COMMENT` review (one notification) with automatic per-comment fallback.

### Fixed
- A fresh draft whose text already appears in a thread (e.g. a second "Done") is now sent; only drafts that previously failed are checked against the forge before retrying.
- A failed draft-store save after a successful submit no longer leaves sent drafts pending (which could re-send them) or lets a pending quit proceed.

### Removed
- Local PR comment posting (`:pr post`, `lgtm pr post|reply|resolve`); agent directives now live in `.lgtm/directives.jsonl` (legacy `comments.jsonl` is migrated automatically).

## 1.1.0

### Added
- Markdown preview: `Tab` toggles the file drawer, directives render under their line, and `c`/`l`/`R`/`x` add, list, resolve and delete directives at the top visible line.
- Clean working tree shows a large LGTM logo, "looks good to me" and a one-line purpose.
- Diff and markdown views show a scrollbar; the markdown preview shows its position, and scrolling counts directive rows so the end is always reachable.
- A wrapped cursor row is kept fully visible in the diff view.
- `↑` / `↓` walk the `:` completion list while it is open.
- Anonymous HTTPS fallback for public GitHub.com PR listings and stored patches when `gh` is missing or needs login and no API token is set; BDD scenarios verify request budgets and failure boundaries. Private reads and writes still require authentication.
- Public API and application-launch BDD scenarios, with a catalog mapping internal UI regressions and remaining acceptance work.
- Opt-in `--debug` timing diagnostics with `--debug-log PATH`, plus offline responsiveness benchmarks.
- Open, closed, and all PR listing filters for GitHub and Azure DevOps.
- `lgtm check` / `lgtm ci` take `--rev <spec>` (for example `origin/main...HEAD`) to gate a committed range.
- Full Repo Audit: AST/regex scan over every tracked file with AST parsing (Rust, Python, Go, Java, Swift, TypeScript/TSX).
- Baseline: suppress existing findings via `.lgtm/baseline.json`.
- Forge discussions for GitHub and Azure DevOps.
- Native MCP agent review server (`lgtm agent --mcp`).
- Keystroke latency budget (< 1us) and 60fps frame rate (< 16ms).
- Zero-panic invariant with regression tests for every panic site.
