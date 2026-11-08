# Proposal

## Intent
Make the PR thread model vendor-neutral and extend it with outdated state, resolve permissions and a settable resolution status (Done, WontFix), so GitHub, Azure DevOps and GitLab share one model, a GitLab provider is added with the same thread parity, and the user can resolve a thread as "won't fix" end to end where the provider supports it.

## Problem Statement
`Thread` lives in the `github` module although Azure and the discussion store depend on it, and it only carries `resolved: bool`. Azure's `fixed | closed | wontFix | byDesign` are collapsed to a bool and `set_resolved(true)` always writes `fixed`, so the original status is lost and WontFix cannot be set. Outdated threads are not flagged, and resolve permissions are unknown until submit fails. Resolve drafts store only a bool intent. GitLab merge requests have no provider at all, and unknown hosts silently fall back to GitHub behavior.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `comments`: Live Thread Listing (outdated badge, resolution status), Draft Resolve and Unresolve (target resolution, permission rejection), Persistent Draft Store (resolution in schema, legacy bool drafts still load), Provider Parity (GitLab joins GitHub and Azure; neutral status and outdated mapping, unknown capability is `None`), Thread Command and Key Surface (`:thread resolve [wontfix]`), MCP and Watcher Source Live Threads (new thread fields exposed).

## Scope

### In-Scope
- Move `Thread` and `ThreadComment` to a provider-neutral `forge` model; keep `lgtm_core::github::Thread` as a re-export.
- `ThreadState { Open, Resolved(Resolution) }` with `Resolution { Done, WontFix, Other(String) }`.
- `outdated: bool`; `can_resolve` and `can_unresolve` as `Option<bool>` (`None` = unknown).
- GitHub: fetch `isOutdated`, `viewerCanResolve`, `viewerCanUnresolve`; map resolved to `Done`.
- Azure: map `fixed` to `Done`, `wontFix` to `WontFix`, `closed | byDesign` to `Other`; outdated stays `false` unless reliably derivable.
- Replace `PrDiscussion::set_resolved(bool)` with `set_state(ThreadState)`; providers fall back to the nearest supported resolution and report it.
- Drafts carry a target resolution; legacy `SetResolved { resolved: bool }` drafts load unchanged.
- `:thread resolve [wontfix]`, `OUTDATED` badge (distinct from draft `STALE`), resolution shown in the thread view.
- GitLab provider (`forge::gitlab`, via `glab`/REST like the other CLI-backed providers): host detection (`gitlab.com` and self-hosted `gitlab.*`), MR listing and diff, head SHA, live discussions, inline and general notes, reply, resolve/unresolve, per-item draft submit.
- GitLab mapping: discussion id as thread id; `resolved` maps to `Done`; `WontFix` falls back to `Done` with reporting; `outdated` false and permissions unknown unless the API reports them; positions map `LEFT`/`RIGHT` to `old_line`/`new_line`.
- Affected crates: `lgtm-core` (model, forge, discussion), `lgtm` (UI, commands), `lgtm-mcp` (thread output).

### Out-of-Scope (YAGNI)
- GitLab CI pipelines as checks (`checks` returns the explicit unsupported error), GitLab draft notes batching and approvals.
- Built-in OAuth (redirect or device flow), token storage or refresh; auth stays delegated to `gh`, `az` and `glab` plus environment-variable overrides.
- Any change to `reply` comment-id typing; GitLab note ids are numeric and fit it.
- Setting `Other(..)` resolutions (`closed`, `byDesign`); they are read-only.
- Collapse-on-resolve or other UI behavior from VS Code.
- Resolve-with-comment shortcut; reply plus resolve drafts already cover it.
- Any change to local directives or `.lgtm/directives.jsonl`.

## Assumptions
- **ASM-GEN-1**: GitHub exposes only resolved/unresolved, so `WontFix` maps to resolve with a reported fallback to `Done`.
- **ASM-GEN-2**: Azure iteration data is not reliable enough to derive outdated; it stays `false` and the spec says so.
- **ASM-GEN-3**: Legacy drafts `{resolved: true|false}` map to `Done` and `Open` on load; no destructive migration.
- **ASM-GEN-6**: GitLab is reachable through `glab` or REST using the token from `glab auth login`, with `GITLAB_TOKEN` as override; no new credential store. Self-hosted detection relies on a `gitlab` host-name segment or explicit config.
- **ASM-GEN-4**: `lgtm_core` stays free of I/O; only provider adapters in `forge` touch the network.
- **ASM-GEN-5**: No CI matrix impact beyond existing ubuntu, macos and windows runs; no path or line-ending logic changes.

## Success Metrics
- Azure `wontFix` thread displays as WontFix and can be set via `:thread resolve wontfix`.
- GitHub outdated threads show `OUTDATED`; a resolve on a thread with `can_resolve == Some(false)` is rejected before submit.
- GitLab MRs list, show threads, accept reply and resolve drafts and submit them with per-item outcomes identical to Azure.
- Existing draft files load and submit unchanged.
- Line and branch coverage >= 95% and 100% mutation kill rate on the new `lgtm_core` logic; no regression in frame and keystroke budgets.

## Review Gate
- [x] **Gate 1 Approval**: Proposal reviewed and approved by human before drafting specs.
