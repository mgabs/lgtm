Feature: Universal key navigation
  As a reviewer
  I want the same navigation keys in every list picker and document viewer
  So that I do not relearn bindings per view

  # Executable: src/app/input.rs universal_navigation_bindings_cover_all_list_pickers
  @REQ-NAV-1
  Scenario: Navigate WorktreePicker with arrows and vim keys
    Given the "WorktreePicker" view is open with 8 items
    When the reviewer presses "Ctrl-n" or "Ctrl-j"
    Then the selection index increments by 1

  # Executable: src/app/input.rs universal_navigation_bindings_cover_all_list_pickers
  @REQ-NAV-2
  Scenario: Page through WorktreePicker
    Given the "WorktreePicker" view is open with 8 items
    When the reviewer presses "Ctrl-d", "Ctrl-f" or "PageDown"
    Then the selection advances by 5 items clamped to the list boundary

  # Executable: src/app/input.rs universal_page_navigation_covers_document_views
  @REQ-NAV-3
  Scenario: Jump to bottom in Diff
    Given the "Diff" view is active
    When the reviewer presses "End"
    Then the cursor jumps to the bottom line of the diff

  # Executable: src/app/input.rs universal_navigation_bindings_cover_all_list_pickers
  @REQ-NAV-4
  Scenario: Dismiss PanePicker with q
    Given the "PanePicker" view is open
    When the reviewer presses "q"
    Then the active view returns to "Diff"
