Feature: Git Worktree Switcher
  As a reviewer conducting multi-agent code reviews
  I want to inspect and switch between multiple Git worktrees directly within lgtm
  So that I can review different agents' workspaces without restarting the application or losing directives

  # REQ-WT-01: Worktree Discovery
  Scenario: Discovering active git worktrees
    Given a git repository with multiple worktrees:
      | path                     | branch       | head     | is_current |
      | /repo                    | main         | a1b2c3d  | true       |
      | /repo/.worktrees/feat-ui | feat-ui      | e5f6a7b  | false      |
      | /repo/.worktrees/feat-db | feat-db      | 9c8b7a6  | false      |
    When the git engine discovers worktrees
    Then 3 worktrees are detected
    And the current worktree is "/repo" on branch "main"

  # REQ-WT-02: Interactive Picker Modal
  Scenario: Opening the worktree picker modal
    Given the reviewer is in the "Diff" view
    When the reviewer enters command ":wt"
    Then the active view changes to "WorktreePicker"
    And the picker lists all discovered worktrees
    And the current worktree is marked with an active badge

  Scenario: Navigating and closing the worktree picker modal
    Given the reviewer is in the "WorktreePicker" view with 3 worktrees
    When the reviewer presses "j"
    Then the selection moves to the second worktree
    When the reviewer presses "Esc"
    Then the active view returns to "Diff"
    And the active worktree remains unchanged

  # REQ-WT-03: Direct Command Jump
  Scenario: Jumping directly to a worktree by branch name
    Given active worktree "/repo" on branch "main"
    And a worktree at "/repo/.worktrees/feat-ui" on branch "feat-ui"
    When the reviewer enters command ":wt feat-ui"
    Then the active worktree switches to "/repo/.worktrees/feat-ui"
    And the diff is reloaded for branch "feat-ui"

  Scenario: Jumping directly to a worktree by directory basename
    Given active worktree "/repo" on branch "main"
    And a worktree at "/repo/.worktrees/feat-ui" on branch "feat-ui"
    When the reviewer enters command ":wt feat-ui"
    Then the active worktree switches to "/repo/.worktrees/feat-ui"

  # REQ-WT-04: Single-Diff Isolation
  Scenario: Single-diff isolation on worktree switch
    Given active worktree "/repo" displaying modified file "src/main.rs"
    And target worktree "/repo/.worktrees/feat-db" displaying modified file "crates/db/src/lib.rs"
    When the reviewer switches to worktree "feat-db"
    Then only changes from "/repo/.worktrees/feat-db" are visible in the diff
    And no diff hunks from "/repo" remain in the snapshot

  # REQ-WT-05: Dirty Directives Guard
  Scenario: Guarding unsaved review directives when switching worktrees
    Given active worktree "/repo" with 2 unexported review directives
    When the reviewer enters command ":wt feat-ui"
    Then a confirmation prompt is displayed: "Pending directives exist: [S]ave & Switch, [D]iscard & Switch, [C]ancel"
    When the reviewer inputs "c"
    Then the switch is aborted and the active worktree remains "/repo"
    And all 2 review directives are retained in memory

  Scenario: Saving review directives when switching worktrees
    Given active worktree "/repo" with 2 unexported review directives
    When the reviewer enters command ":wt feat-ui"
    And the reviewer inputs "s"
    Then the 2 directives are saved to ".lgtm/directives.jsonl" in "/repo"
    And the active worktree switches to "/repo/.worktrees/feat-ui"

  Scenario: Discarding review directives when switching worktrees
    Given active worktree "/repo" with 2 unexported review directives
    When the reviewer enters command ":wt feat-ui"
    And the reviewer inputs "d"
    Then the active worktree switches to "/repo/.worktrees/feat-ui"
    And in-memory directives for "/repo" are cleared

  # REQ-WT-06: Visual Header Badge
  Scenario: Displaying worktree badge in header and status bar
    Given the active worktree is "/repo/.worktrees/feat-ui" on branch "feat-ui"
    When the diff view is rendered
    Then the status bar contains "[wt: feat-ui]"
