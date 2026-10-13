# Azure DevOps pull requests (design, draft)

Status: draft, brainstormed 2026-10-13. Track G in the roadmap.

## Problem

`:pr <n>`, `:prs`, checks, threads and posting all shell out to `gh`, so they fail on Azure DevOps repos. The GitHub assumption is spread over `pr/mod.rs` (`PrEngine`) and `github/` (`GithubClient`, `GhClient`).

## Decisions

1. **Provider seam, not a fork.** One `Forge` trait (list, diff, head sha, checks, threads, post inline/general, reply, set resolved, vote). `GitHub` (`gh`) and `AzureDevOps` (`az`) implement it. `GithubClient` and `PrEngine` fold into it; GitHub behavior stays identical.
2. **Detect from `origin`.** Recognized: `https://dev.azure.com/{org}/{project}/_git/{repo}`, `git@ssh.dev.azure.com:v3/{org}/{project}/{repo}`, `https://{org}.visualstudio.com/{project}/_git/{repo}`, and `github.com`. Override with `[forge] kind = "azure" | "github"` in `.lgtm.toml`. Unknown host: clear message, local features unaffected.
3. **Azure transport is the `az` CLI** (`azure-devops` extension), mirroring `gh`. No PAT handling in lgtm. REST via `curl` is a later option only if `az` proves too thin.
4. **Diff from git, not from the forge.** Azure has no `az repos pr diff`. Fetch `refs/pull/{id}/merge` (or source and target branches), then reuse `compute_revision_diff("target...source")`. This is faster and skips unified-diff parsing; GitHub can adopt it later.
5. **Thread model.** Azure threads have an int id and a status (`active`, `fixed`, `closed`, `wontFix`, `byDesign`, `pending`). Map active/pending to open and the rest to resolved. Replies address (thread id, parent comment id), so `reply` and `set_resolved` take an opaque `ThreadRef` instead of GitHub's numeric comment id.
6. **Calls** (all `az`):
   - list: `az repos pr list --status active --output json`
   - show: `az repos pr show --id N`
   - checks: `az repos pr policy list --id N` mapped to pass/fail/pending/skipped
   - threads, post, reply, resolve: `az devops invoke --area git --resource pullRequestThreads|pullRequestThreadComments ...`; inline threads carry `threadContext.filePath` and `rightFileStart.line`
   - vote: `az repos pr set-vote --id N --vote approve|approve-with-suggestions|wait-for-author|reject`
7. **Auth and setup errors**, like `explain()` for gh: missing `az`, missing `azure-devops` extension (print the `az extension add --name azure-devops` hint, never auto-install), not logged in (`az login` or `az devops login`), no org/project (use `--detect true` or the parsed remote).
8. **CI output.** `lgtm ci` on Azure Pipelines emits `##vso[task.logissue type=error;sourcepath=..;linenumber=..]` lines next to the existing GitHub annotations. Pairs with D3 recipes.

## Stages

| ID | Item | Effort |
|---|---|---|
| G1 | `Forge` trait and detection; GitHub moved onto it with no behavior change | M |
| G2 | Azure read path: `:prs`, `:pr <n>` via git ref fetch, policies as checks | M |
| G3 | Azure threads read plus inline thread markers (shares F3 UI) | M |
| G4 | Azure post, reply, resolve, vote; resumable posting through the existing remote-id scheme | M |
| G5 | `##vso` annotations, WIKI section, Azure Pipelines recipe | S |

## Testing

Fake runner per provider (same pattern as the existing fake GitHub client) with recorded `az` JSON fixtures for each call; remote-URL parse table tests; the git-ref diff path tested on a temp repo. Real-service behavior is unverified until run against an Azure DevOps project (needs `az login`); G2 and G4 include a manual smoke checklist.

## Open

- Whether `git fetch origin refs/pull/N/merge` is permitted on the user's org (branch policies can hide it); fallback is fetching source and target branches by name from `az repos pr show`.
- Azure DevOps Server (on-prem) URL shapes; start with cloud.
