# Spec Delta

## ADDED Requirements

### Requirement: REQ-GATE-1 Tick Unticked Review Gate Checkbox on Approve
When the reviewer invokes `:approve`, `:lgtm`, or `A` in `Diff` or `MarkdownPreview` view on a local working-tree file whose cursor line is an unticked `- [ ]` item inside a `## Review Gate` section, the system MUST replace `[ ]` with `[x]` on disk, reload the diff, and NOT modify `.lgtm/gate.json`.

#### Scenario: Tick Review Gate checkbox in Diff view
- **WHEN** the reviewer views a local working-tree diff of a markdown file with cursor on `- [ ] **Gate 1 Approval**: ...` under `## Review Gate` and presses `A` or runs `:approve`
- **THEN** `[ ]` on that line is replaced with `[x]` on disk, the diff reloads, and `.lgtm/gate.json` remains unchanged

#### Scenario: Tick Review Gate checkbox in MarkdownPreview view
- **WHEN** the reviewer views a local working-tree markdown file in `MarkdownPreview` with cursor on an unticked `- [ ]` line under `## Review Gate` and presses `A` or runs `:lgtm`
- **THEN** `[ ]` on that line is replaced with `[x]` on disk, the preview reloads, and `.lgtm/gate.json` remains unchanged

#### Scenario: Preserve CRLF line endings when ticking
- **WHEN** the target markdown file uses CRLF (`\r\n`) line endings
- **THEN** ticking `[ ]` to `[x]` preserves all CRLF line endings in the file

### Requirement: REQ-GATE-2 Two-Press Gate Approval
If the cursor is not on an unticked `- [ ]` line inside a `## Review Gate` section of a local working-tree file, invoking `:approve`, `:lgtm`, or `A` MUST fall through to `approve_gate()` and approve `.lgtm/gate.json` if present.

#### Scenario: Second press after ticking approves .lgtm/gate.json
- **WHEN** the reviewer has ticked the Review Gate checkbox on the first press (line is now `- [x]`) and presses `A` or runs `:approve` a second time while `.lgtm/gate.json` is waiting
- **THEN** `.lgtm/gate.json` is updated with `"status": "approved"` and the status bar reports gate approval

#### Scenario: Non-gate checkbox is not ticked
- **WHEN** the cursor is on `- [ ]` outside a `## Review Gate` section (e.g., under `## Tasks`)
- **THEN** the file on disk is not modified and the command falls through to `approve_gate()`

### Requirement: REQ-GATE-3 Guard Against Historical or Deleted Targets
The system MUST NOT edit files on disk when viewing a PR diff, a historical revision diff, or a deleted diff line.

#### Scenario: Historical revision or PR diff falls through
- **WHEN** the reviewer views a PR diff or `:diff <rev>` and invokes `:approve` on a `## Review Gate` `- [ ]` line
- **THEN** no working-tree file is modified and the command falls through to `approve_gate()`

#### Scenario: Deleted line falls through
- **WHEN** the cursor is on a deleted `- [ ]` line in Diff or MarkdownPreview view
- **THEN** no working-tree file is modified and the command falls through to `approve_gate()`

### Requirement: NFR-GATE-1 Keystroke Responsiveness
Checking whether the cursor line is an unticked Review Gate checkbox MUST NOT run on per-frame render paths and MUST execute only on explicit `:approve`, `:lgtm`, or `A` invocation.

#### Scenario: Zero per-frame overhead
- **WHEN** rendering frames in Diff or MarkdownPreview view without pressing `A` or running `:approve`
- **THEN** no disk reads or Review Gate parsing occur during rendering

## Review Gate
- [x] **Gate 2 Approval**: Specifications reviewed and approved by human before technical design.
