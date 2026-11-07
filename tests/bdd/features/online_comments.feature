Feature: Online Forge Comments
  As a code reviewer
  I want to create, reply to, and resolve PR discussion threads directly against the remote forge
  So that comments post immediately without local draft staging queues

  # REQ-COM-1; Scenario: Post inline thread online
  Scenario: Post inline thread online
    Given an open pull request on a supported git forge
    When the user creates an inline comment on a diff line
    Then the comment is dispatched directly to the forge API asynchronously
    And the new thread is reflected in the TUI upon successful response
    And zero draft files are written to disk

  # REQ-COM-2; Scenario: Reply to active thread
  Scenario: Reply to active thread
    Given an active PR thread displayed in the thread view
    When the user submits a reply
    Then the reply is posted directly to the forge without entering a draft queue
    And the updated thread conversation appears in the view

  # REQ-COM-3; Scenario: Resolve thread online
  Scenario: Resolve thread online
    Given an unresolved thread displayed in the thread view
    When the user toggles the thread to resolved
    Then the resolution is sent directly to the forge API
    And the thread is marked resolved in the TUI

  # REQ-COM-4; Scenario: UI responsiveness during comment post
  Scenario: UI responsiveness during comment post
    Given an in-flight comment post to a remote forge
    When the user navigates the diff or scrolls the viewport
    Then the TUI responds within the 16ms frame budget without blocking

  # REQ-COM-5; Scenario: Failed post retains input buffer
  Scenario: Failed post retains input buffer
    Given a network disconnection or forge API error
    When a comment submission fails
    Then an error message is displayed in the status bar
    And the typed comment body is retained in memory for retry
    And no draft file is created on disk

  # REQ-COM-6; executable mappings: docs/bdd-scenarios.md, REQ-COM-6.
  # Rust: thread_view_filter_cycles_all_open_outdated; test_full_ui_rendering_on_test_backend.
  Scenario: Inspect and filter outdated threads
    Given current, resolved, and outdated PR threads
    When the user cycles the thread modal filter with Tab or o
    Then the filter cycles All, Open, Outdated, and All
    And outdated threads show the outdated badge
    And the Open filter excludes resolved and outdated threads

  # REQ-COM-6; Rust: bracket_capital_t_jumps_between_outdated_threads;
  # outdated_thread_jump_skips_current_and_resolved_threads;
  # outdated_thread_enter_jumps_to_file_header_when_line_is_gone.
  Scenario: Navigate outdated thread locations safely
    Given outdated threads in the current file
    When the user presses ]T or [T
    Then the cursor jumps to the next or previous unresolved outdated line
    And Enter on a thread whose line is absent jumps to the file header

  # NFR-COM-1; Rust: outdated_thread_performance_budgets, src/ui/diff/regression.rs.
  # Release-only measurement; docs/bdd-scenarios.md records fixture and scope.
  Scenario: Outdated thread reconciliation preserves the frame budget
    Given 100 PR threads and 1000 diff lines at a 120 by 40 terminal size
    When the release acceptance test measures seven samples after warm-up
    Then median reconciliation and cold gutter mapping take less than 1 millisecond
    And median cold frame rendering takes less than 16 milliseconds
    And setup and network are excluded from the measurement
