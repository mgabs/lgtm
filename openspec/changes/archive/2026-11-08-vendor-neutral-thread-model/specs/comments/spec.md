# Spec Delta

## ADDED Requirements

### Requirement: REQ-COMMENTS-1 Vendor-Neutral Thread Model
Every provider (GitHub, Azure DevOps and GitLab) SHALL return threads in one neutral model: id, path, optional line, `state` (`Open` or `Resolved(Done | WontFix | Other(text))`), `outdated`, `can_resolve` and `can_unresolve` (each true, false or unknown) and ordered comments. Core and UI logic MUST NOT branch on provider. A capability a provider cannot report MUST be unknown or false, never an error.

#### Scenario: Azure status mapping
- **WHEN** Azure returns thread statuses `fixed`, `wontFix`, `closed`, `byDesign` and `active`
- **THEN** they map to `Done`, `WontFix`, `Other("closed")`, `Other("byDesign")` and `Open`

#### Scenario: GitHub flags mapping
- **WHEN** GitHub returns `isResolved`, `isOutdated`, `viewerCanResolve` and `viewerCanUnresolve`
- **THEN** the thread has state `Done` or `Open`, the same `outdated` value and the same permissions

#### Scenario: GitLab discussion mapping
- **WHEN** GitLab returns a discussion with `resolved` true and notes with `old_line` or `new_line`
- **THEN** the thread has state `Done`, the notes in order, and side `LEFT` for `old_line` or `RIGHT` for `new_line`

#### Scenario: Unreported capability
- **WHEN** a provider reports no permission data
- **THEN** `can_resolve` and `can_unresolve` are unknown and no error is raised

### Requirement: REQ-COMMENTS-2 Resolution Fallback Reporting
When a provider cannot express a requested resolution, it SHALL apply the nearest supported one and the submit summary MUST name the requested and applied resolution. A fallback MUST NOT be reported as a plain success. `Other` resolutions MUST NOT be settable.

#### Scenario: WontFix on GitHub
- **WHEN** a `WontFix` resolve draft is submitted to GitHub
- **THEN** the thread is resolved and the summary states "requested WontFix, applied Done"

#### Scenario: WontFix on GitLab
- **WHEN** a `WontFix` resolve draft is submitted to GitLab
- **THEN** the discussion is resolved and the summary states "requested WontFix, applied Done"

#### Scenario: Other is read-only
- **WHEN** a user attempts to set resolution `closed`
- **THEN** the request is rejected as unsupported

### Requirement: REQ-COMMENTS-3 GitLab Delegated Authentication
The GitLab provider SHALL authenticate with the token stored by `glab auth login` for the MR's host, or `GITLAB_TOKEN` when set, and MUST NOT implement its own OAuth flow or credential store. Tokens MUST NOT appear in process arguments or logs. Missing or rejected credentials MUST map to an authentication failure naming `glab auth login`.

#### Scenario: Environment override
- **WHEN** `GITLAB_TOKEN` is set and non-blank
- **THEN** it is used for requests instead of the `glab` stored token

#### Scenario: Not authenticated
- **WHEN** no token exists or the API returns 401
- **THEN** the failure is reported as authentication with the remediation `glab auth login`

### Requirement: NFR-COMMENTS-1 Core Purity and Compatibility
The neutral model and state mapping SHALL live in `lgtm_core` without I/O. `lgtm_core::github::Thread` and `ThreadComment` MUST remain importable. Rendering the thread list MUST stay within the 16 ms frame budget.

#### Scenario: Legacy import path
- **WHEN** existing code imports `lgtm_core::github::Thread`
- **THEN** it compiles against the neutral model unchanged

## MODIFIED Requirements

### Requirement: Live Thread Listing
Opening the comments view or running `:refresh` SHALL fetch threads live from the forge off the main loop. Each thread MUST show path, line, side, resolution status (Open, Done, WontFix or the provider text), an `OUTDATED` badge when outdated, and all comments (author, body) in order. `OUTDATED` MUST be distinct from the draft `STALE` badge. Diff gutter markers MUST derive from fetched threads only.

#### Scenario: Open comments view
- **WHEN** a PR is loaded and the comments view opens
- **THEN** live threads are listed with path, line, side, resolution status and comments

#### Scenario: Outdated comment marker in diff gutter
- **WHEN** a remote thread targets a line modified or deleted by a newer commit in the PR
- **THEN** the diff gutter renders an outdated marker `~` rather than an active open marker

#### Scenario: Outdated thread
- **WHEN** a fetched thread is outdated
- **THEN** it shows an `OUTDATED` badge and no draft is marked `STALE` because of it

#### Scenario: No PR target
- **WHEN** no PR or forge target exists
- **THEN** the view shows "Comments require a PR" and offers no create, reply or resolve actions

### Requirement: REQ-COM-3 Direct Online Thread Resolution
Users SHALL be able to toggle thread resolution (resolve with `Done` or `WontFix`, or unresolve) directly against the forge API. The resolved state MUST update immediately in the TUI upon forge confirmation, rejecting actions when the thread's `can_resolve` or `can_unresolve` is false.

#### Scenario: Resolve thread online
- **WHEN** the user resolves a focused thread
- **THEN** the resolution status is updated directly on the forge and reflected in the TUI

#### Scenario: Resolve thread with WontFix
- **WHEN** the user resolves a focused thread with `wontfix`
- **THEN** the resolution status is set to `WontFix` or falls back to `Done` on providers without native support

#### Scenario: Known missing permission
- **WHEN** the user attempts to resolve on a thread with `can_resolve` false
- **THEN** the action is rejected with the reason and not sent to the forge

### Requirement: Provider Parity
Live listing and the draft thread, reply and resolve behavior SHALL behave identically on GitHub (`side`) and Azure DevOps (`leftFileStart` versus `rightFileStart`) through the neutral model, and on GitLab (`old_line` versus `new_line`). Provider statuses MUST map per REQ-COMMENTS-1. Azure outdated MUST be false unless derivable reliably. Unsupported capabilities MUST return an explicit error.

#### Scenario: GitHub outdated thread parsing
- **WHEN** a GitHub review thread GraphQL response contains `isOutdated: true`
- **THEN** the thread model sets `outdated = true`

#### Scenario: Azure DevOps outdated thread reconciliation
- **WHEN** an Azure DevOps thread targets a line not present in the current diff snapshot hunks
- **THEN** the thread model sets `outdated = true`

#### Scenario: Azure side mapping
- **WHEN** a `LEFT` thread is submitted to Azure DevOps
- **THEN** it is sent using the left-file position fields

#### Scenario: GitLab host detection
- **WHEN** the origin URL host is `gitlab.com` or a `gitlab.*` host
- **THEN** the GitLab provider is selected and never the GitHub fallback

#### Scenario: GitLab unsupported capability
- **WHEN** CI checks are requested on a GitLab MR
- **THEN** an explicit unsupported error is returned

#### Scenario: GitLab per-item submit
- **WHEN** a batch of drafts is submitted to GitLab
- **THEN** each item is sent individually and succeeds or fails independently

#### Scenario: Azure WontFix write
- **WHEN** a `WontFix` resolve draft is submitted to Azure DevOps
- **THEN** the thread status is set to `wontFix`

### Requirement: Thread Command and Key Surface
PR comment work SHALL be namespaced under `:thread` (`comment`, `reply`, `resolve [wontfix]`, `unresolve`, `submit`, `discard [id|all]`, `refresh`), and `:threads` MUST open the single PR comments view. Local directive commands MUST be unchanged. In the view, `j/k`, `Enter`, `c`, `r`, `x`, `e`, `d`, `s` and `Esc` MUST be scoped to the view with no global key reassigned. Removed commands MUST NOT be aliased, and without a loaded PR `:thread *` MUST show "Comments require a PR".

#### Scenario: Command domains stay separate
- **WHEN** a local directive and remote thread share the cursor line and the user runs bare `:reply`, `:resolve`, or `:unresolve`
- **THEN** only the local directive is changed; remote thread actions require the explicit `:thread` namespace

#### Scenario: Draft via command
- **WHEN** a PR is loaded and the user runs `:thread comment looks wrong`
- **THEN** a draft thread with that body is created at the cursor line

#### Scenario: Resolve as WontFix
- **WHEN** the user runs `:thread resolve wontfix` on a focused thread
- **THEN** a resolve draft with resolution `WontFix` is stored

#### Scenario: Removed command
- **WHEN** the user runs `:post`
- **THEN** it is reported as an unknown command

#### Scenario: Filter threads by outdated status
- **WHEN** the user runs `:threads outdated`
- **THEN** only threads with `outdated == true` are displayed in the list, including resolved outdated threads

#### Scenario: Cycle thread status filters
- **WHEN** the user presses `Tab` or `o` in the thread modal
- **THEN** the filter advances from `All` to `Open` to `Outdated` and back to `All`
- **AND** `Open` includes only unresolved, non-outdated threads, while `Outdated` includes all outdated threads

#### Scenario: Unknown removed command
- **WHEN** the user runs `:thread submit` or `:thread drafts`
- **THEN** the command is rejected as unknown with usage instructions

#### Scenario: Navigation to outdated thread line
- **WHEN** the user presses `Enter` on an outdated thread whose line is absent from the diff
- **THEN** cursor navigates to the file header and status bar displays "Line gone from current diff (outdated)"

### Requirement: MCP and Watcher Source Live Threads
The `lgtm-mcp` comment tools and the watcher SHALL source PR comments from live forge threads, not from local jsonl files. MCP thread output MUST include resolution status and `outdated`, and MUST keep the existing `resolved` boolean.

#### Scenario: MCP lists live threads
- **WHEN** an MCP client requests PR comments
- **THEN** the response reflects threads fetched from the forge with status, `outdated` and `resolved`
