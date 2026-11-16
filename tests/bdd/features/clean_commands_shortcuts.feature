Feature: Two-Step Review Gate Ticking and Approval
  As a reviewer inspecting OpenSpec documents in lgtm
  I want :approve, :lgtm, and A to tick an unticked Review Gate checkbox on first press
  So that I can approve both the document gate and the agent gate without leaving the TUI

  # REQ-GATE-1: Tick Unticked Review Gate Checkbox on Approve
  Scenario: First press on unticked Review Gate line in Diff view ticks the box
    Given a local working-tree diff containing "proposal.md" with an unticked "- [ ] **Gate 1 Approval**" line under "## Review Gate"
    And an active ".lgtm/gate.json" with status "pending"
    And the reviewer cursor is on that "- [ ]" line in "Diff" view
    When the reviewer invokes ":approve" or presses "A"
    Then "proposal.md" on disk is updated to "- [x] **Gate 1 Approval**"
    And the diff is reloaded
    And ".lgtm/gate.json" remains "pending"

  Scenario: First press on unticked Review Gate line in MarkdownPreview view ticks the box
    Given a local working-tree diff containing "design.md" in "MarkdownPreview" view
    And the preview cursor is on an unticked "- [ ]" line under "## Review Gate"
    When the reviewer invokes ":lgtm" or presses "A"
    Then "design.md" on disk is updated to "- [x]" on that line
    And the preview is reloaded
    And ".lgtm/gate.json" is not modified

  Scenario: CRLF line endings are preserved when ticking a Review Gate line
    Given a local working-tree markdown file using CRLF line endings
    When the reviewer ticks the Review Gate checkbox via ":approve"
    Then all CRLF line endings in the file are preserved

  # REQ-GATE-2: Two-Press Gate Approval
  Scenario: Second press after ticking approves .lgtm/gate.json
    Given "proposal.md" now has "- [x] **Gate 1 Approval**" under "## Review Gate"
    And ".lgtm/gate.json" has status "pending"
    When the reviewer invokes ":approve" or presses "A" a second time
    Then ".lgtm/gate.json" is updated to status "approved"
    And the status bar reports that the agent gate is approved

  Scenario: Non-gate checkbox falls through to approve_gate
    Given the reviewer cursor is on "- [ ] 1.1 Implement helper" under "## Tasks"
    And ".lgtm/gate.json" has status "pending"
    When the reviewer invokes ":approve"
    Then the markdown file on disk is not modified
    And ".lgtm/gate.json" is updated to status "approved"

  # REQ-GATE-3: Guard Against Historical or Deleted Targets
  Scenario: Historical revision, PR diff, or deleted line never edits disk
    Given the reviewer is viewing a PR diff, ":diff HEAD~1", or a deleted "- [ ]" line under "## Review Gate"
    When the reviewer invokes ":approve" or presses "A"
    Then no working-tree file is modified
    And the action falls through to "approve_gate()"

  # NFR-GATE-1: Keystroke Responsiveness
  Scenario: No Review Gate parsing or disk I/O occurs on per-frame render
    Given the reviewer is scrolling in "Diff" or "MarkdownPreview" view without invoking ":approve", ":lgtm", or "A"
    Then no Review Gate file reads or parsing execute during frame rendering
