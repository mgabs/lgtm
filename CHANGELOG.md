# Changelog

## 0.8.2 (unreleased)

### Added
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

### Fixed
- Azure DevOps `:pr <n>` is faster: it fetches `refs/pull/<n>/merge` with one `git fetch` and skips the slow `az` call; it falls back to the branch fetch when that ref is missing. `az` runs with telemetry off.
- File sidebar follows the active file when navigating with `]]` / `[[` instead of scrolling out of view (diff and audit views).
- On-prem Azure DevOps Server remotes (`.../_git/...`) are detected as Azure.

### Changed
- Exit codes for `check` / `ci`: 0 pass, 1 blocking findings, 2 tool error (was 1 for errors).
- `:audit` is described and documented as a whole-codebase audit; `:scan` without `repo` and `:risk` cover the diff only.
- PR features go through a `Forge` trait; GitHub behavior is unchanged.

## 0.8.0

Module split and Cargo workspace, `.lgtm.toml` config, findings pipeline with baselines and suppressions, review state and hunk marks, GitHub checks/threads/inline posting, directive outcome tracking, new MCP tools, tree-sitter AST rules for TS/JS/React/Next.js and Swift.
