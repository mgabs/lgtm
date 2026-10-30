# Changelog

## Unreleased

### Added
<<<<<<< HEAD
- Online PR comments: live threads, local drafts (`:thread comment|general|reply|resolve|unresolve|edit|discard|drafts`), and a confirmed batch `:thread submit` with per-item results, stale detection and duplicate-safe retries; MCP `lgtm_list_threads`; pending inline drafts show a `✎` gutter marker and an inline row (pending/stale/failed) in the diff view.
=======
- Online PR comments: live threads, local drafts (`:thread comment|general|reply|resolve|unresolve|edit|discard|drafts`), and a confirmed batch `:thread submit` with per-item results, stale detection and duplicate-safe retries; MCP `lgtm_list_threads`. On GitHub, new inline comments in a submit are posted as a single `COMMENT` review (one notification) with automatic per-comment fallback.
>>>>>>> feat/oc-batching

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
- Composite GitHub Action (`action.yml`): checksum-verified install, `--rev` gate, SARIF upload, job summary, `exit-code` and `new-findings` outputs.
- CI recipes for GitLab CI, Azure Pipelines and a pre-commit hook (WIKI, section 7).
- Azure DevOps pull requests via the `az` CLI: `:prs`, `:pr <n>`, and `:checks` (branch policies) and `:threads`. Provider chosen from the `origin` URL. Posting inline and general comments, replies, resolve/unresolve and votes (approve, wait-for-author) work too.
- `lgtm ci` emits Azure Pipelines `##vso[task.logissue]` annotations (auto on `TF_BUILD`, or `-f azure`); WIKI has an Azure Pipelines recipe.
- `:diff <branch>` compares the current branch with `<branch>` (merge-base, like a PR) and audits the result.
- `]t` / `[t` jump between thread lines in the file.
- `:directives orphans` lists orphaned comments; `repin <n>` moves one to the cursor line, `drop --yes` deletes them.
- Inline thread markers: after `:threads`, lines with remote discussion show `◆` (open) or `◇` (resolved) in the diff gutter, on GitHub and Azure DevOps.
- Orphaned comments: comments are re-pinned after every diff reload and flagged `⚠ Orphaned` when their code is gone (never deleted).

- Provider request contracts (`bdd_core_019`, `bdd_core_020`), ambiguous remote write recovery (`bdd_core_021`, `bdd_ui_009`), real-terminal acceptance journeys (`bdd_cli_006`), failed receipt save recovery across restart (`bdd_core_022`, `bdd_ui_010`), automated BDD catalog reporting (`scripts/bdd-report.sh`, `make bdd-report`), opt-in live provider checks (`live_provider.rs`), and terminal responsiveness measurements (`bdd_cli_007`).
- Inline and modal `[↑]` posted badge for directives with remote receipts (`remote_id.is_some()`).

### Fixed
- `lgtm-mcp` (`lgtm_security_scan`, `lgtm_scan_changes`, `lgtm://findings`) now routes through `lgtm_core::findings::run_in`, respecting `.lgtm.toml` and `.lgtm-baseline.json`.
- `lgtm init` generates `.lgtm.toml` matching the `LgtmConfig` schema.
- `lgtm review export` and `lgtm review import` synchronize target-scoped directives with `.lgtm/comments.jsonl` without renaming `.lgtm/comments.jsonl` to `.bak`.
- `GhClient` passes JSON payloads via `--input -` on stdin instead of `ARGV`; `PrivateTempFile` enforces `0o600` permissions on existing temp files.
- PR loading prepares directive anchors, outcomes, review marks, and folds in the background. Changed inputs reprepare without refetching; drafts, selections, newer edits, and posting receipts are preserved.
- Background refreshes, audits, and PR writes keep the current review usable; render caches and bounded event processing reduce navigation work.
- Navigation with folded AST blocks is much faster: fold lookups are computed in one pass per hunk instead of per line, and queued key presses are applied before a redraw so held `j`/`k` doesn't lag.
- `:<num>` jumps to the nearest diff line when the line isn't in the diff, and says so.
- Azure DevOps `:prs`, `:checks` and `:threads` are fast: REST calls via `curl` with a per-session `az` token (falls back to `az` commands), results cached for 5 minutes (cleared by writes), and all three run in the background so the UI never freezes.
- Azure DevOps PR loads read metadata first and prefer recorded commits, avoiding repeated missing merge refs for historical PRs. GitHub also caches missing merge refs for 5 minutes and falls back to its stored PR patch. `az` runs with telemetry off.
- File sidebar follows the active file when navigating with `]]` / `[[` instead of scrolling out of view (diff and audit views).
- On-prem Azure DevOps Server remotes (`.../_git/...`) are detected as Azure.

### Changed
- Exit codes for `check` / `ci`: 0 pass, 1 blocking findings, 2 tool error (was 1 for errors).
- `:audit` is described and documented as a whole-codebase audit; `:scan` without `repo` and `:risk` cover the diff only.
- PR features go through a `Forge` trait; GitHub behavior is unchanged.
- Removed simulated `coverage`, `mutate`, and `verify-tests` subcommands to keep the CLI focused on review, static guardrails, and MCP steering.

## 0.8.0

Module split and Cargo workspace, `.lgtm.toml` config, findings pipeline with baselines and suppressions, review state and hunk marks, GitHub checks/threads/inline posting, directive outcome tracking, new MCP tools, tree-sitter AST rules for TS/JS/React/Next.js and Swift.
