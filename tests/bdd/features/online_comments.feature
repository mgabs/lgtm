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
