# Spec Delta: comments

## MODIFIED Requirements

### Requirement: Live Thread Listing
Opening the comments view or running `:refresh` SHALL fetch threads live from the forge off the main loop. Each thread MUST show path, line, side, resolved badge, outdated badge, and all comments (author, body) in order. Diff gutter markers MUST derive from fetched threads, displaying `◆` for open threads, `◇` for resolved threads, and `~` for outdated threads whose target lines changed in the PR diff.

#### Scenario: Open comments view
- **WHEN** a PR is loaded and the comments view opens
- **THEN** live threads are listed with path, line, side, resolved badge, outdated badge, and comments

#### Scenario: Outdated comment marker in diff gutter
- **WHEN** a remote thread targets a line modified or deleted by a newer commit in the PR
- **THEN** the diff gutter renders an outdated marker `~` rather than an active open marker

#### Scenario: No PR target
- **WHEN** no PR or forge target exists
- **THEN** the view shows "Comments require a PR" and offers no create, reply, or resolve actions

### Requirement: Provider Parity
Live listing, outdated thread detection, reply, and resolve behavior SHALL behave identically across GitHub and Azure DevOps. GitHub threads SHALL source outdated status from `isOutdated`; Azure DevOps threads SHALL reconcile positions against the current diff snapshot. Unsupported capabilities MUST return an explicit error.

#### Scenario: GitHub outdated thread parsing
- **WHEN** a GitHub review thread GraphQL response contains `isOutdated: true`
- **THEN** the thread model sets `outdated = true`

#### Scenario: Azure DevOps outdated thread reconciliation
- **WHEN** an Azure DevOps thread targets a line not present in the current diff snapshot hunks
- **THEN** the thread model sets `outdated = true`

#### Scenario: Azure side mapping
- **WHEN** a `LEFT` thread is submitted to Azure DevOps
- **THEN** it is sent using the left-file position fields

### Requirement: Thread Command and Key Surface
PR discussion commands SHALL be namespaced under `:thread` (`comment`, `reply`, `resolve`, `unresolve`, `retry`, `refresh`, `outdated`) and `:threads [open|outdated|all]`. Bare `:reply`, `:resolve`, and `:unresolve` SHALL operate only on local directives; remote thread actions MUST use `:thread`. Obsolete draft queue commands (`drafts`, `submit`, `discard`, `edit`) SHALL be rejected as unknown. In the `:threads` modal, `Tab` and `o` MUST cycle status filters (`All`, `Open`, `Outdated`), and the dead `'s'` submit key SHALL be removed.

#### Scenario: Command domains stay separate
- **WHEN** a local directive and remote thread share the cursor line and the user runs bare `:reply`, `:resolve`, or `:unresolve`
- **THEN** only the local directive is changed; remote thread actions require the explicit `:thread` namespace

#### Scenario: Draft via command
- **WHEN** a PR is loaded and the user runs `:thread comment looks wrong`
- **THEN** the comment is posted directly to the remote forge without creating a local disk draft

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

## ADDED Requirements

### Requirement: REQ-COM-6 Outdated Thread Indication and Filtering
The system MUST provide dedicated visual indicators for outdated PR threads across diff gutters and the thread modal, and SHALL allow reviewers to jump directly between outdated threads in the diff view using `]T` and `[T`.

#### Scenario: Jump to next outdated thread
- **WHEN** the user presses `]T` in the diff view
- **THEN** the cursor jumps to the next line containing an unresolved outdated thread marker in the current file

#### Scenario: Jump to previous outdated thread
- **WHEN** the user presses `[T` in the diff view
- **THEN** the cursor jumps to the previous line containing an unresolved outdated thread marker in the current file

#### Scenario: Outdated thread modal badge
- **WHEN** an outdated thread is selected in the thread modal
- **THEN** its status displays `[⚠ Outdated]`

### Requirement: NFR-COM-1 Thread Filter and Gutter Budget
Reconciling thread outdated state against the diff snapshot and rendering gutter markers MUST execute in under 1 millisecond on diff reload to preserve the 16ms frame rendering budget.

#### Scenario: Outdated check maintains frame budget
- **WHEN** a PR diff with 100 discussion threads and 1,000 diff lines is measured in a release build
- **THEN** in-memory reconciliation and gutter marker mapping complete within 1ms
- **AND** the median of seven samples after warm-up is measured by `outdated_thread_performance_budgets`, with setup and network excluded as recorded in `docs/bdd-scenarios.md`
