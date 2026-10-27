Feature: PR Comments, Reply, and Post Workflow
  As a code reviewer steering autonomous AI agents and peer engineers
  I want to anchor review directives, post them accurately to GitHub and Azure DevOps PRs, and manage threaded replies
  So that comments appear on the correct diff sides, network failures never duplicate comments, and review threads can be resolved seamlessly

  # REQ-COM-01: Line Positioning and Side Accuracy
  Scenario: Posting inline comments to modified and deleted diff lines
    Given an open pull request 42 with file "src/main.rs"
    And a diff hunk containing deleted line 10 and added line 12
    And a local review directive anchored to deleted line 10
    And a local review directive anchored to added line 12
    When the reviewer posts comments to the PR
    Then the deleted line comment is sent with side "LEFT" at line 10
    And the added line comment is sent with side "RIGHT" at line 12
    And both directives receive remote IDs and are saved to disk

  Scenario: Routing non-diff or out-of-hunk directives to general review comments
    Given an open pull request 42 with file "README.md"
    And a local review directive anchored to file "README.md" outside any modified hunk
    When the reviewer posts comments to the PR
    Then the directive is classified as general commentary
    And it is posted as a top-level review comment without line coordinates
    And the local directive receipt records the top-level remote ID

  # REQ-COM-02: Headless CLI Preview and Confirmed Posting
  Scenario: Previewing unposted comments without confirmation (Dry Run)
    Given an active review on pull request 42 with 3 unposted directives
    When the reviewer executes "lgtm pr post 42"
    Then the command exits with code 0
    And the output displays a summary preview of 3 pending comments
    And no comments are sent to the remote provider
    And all 3 directives remain unposted in ".lgtm/comments.jsonl"

  Scenario: Confirmed batch comment posting via CLI
    Given an active review on pull request 42 with 2 unposted directives
    When the reviewer executes "lgtm pr post 42 --yes"
    Then the command exits with code 0
    And 2 comments are published to the remote pull request
    And ".lgtm/comments.jsonl" is updated with remote IDs for both comments

  # REQ-COM-03: Thread Replying via CLI and TUI
  Scenario: Replying to an existing review thread via CLI
    Given an existing remote review thread "thread_123" on pull request 42
    When the reviewer executes "lgtm pr reply 42 thread_123 'Fix applied in commit abc'"
    Then the reply message is appended to thread "thread_123"
    And the command exits with code 0

  Scenario: Replying to a review thread in the interactive TUI
    Given the reviewer is in the "Diff" view with cursor on line with review thread "thread_123"
    When the reviewer presses "r"
    Then the active view changes to "CommentReply" with target thread "thread_123"
    When the reviewer types "Will address this shortly" and presses "Enter"
    Then the reply is published to the remote thread
    And the active view returns to "Diff"

  # REQ-COM-04: Thread Resolution and Reopening
  Scenario: Resolving and unresolving review threads via CLI
    Given an active review thread "thread_123" on pull request 42
    When the reviewer executes "lgtm pr resolve 42 thread_123"
    Then thread "thread_123" is marked as resolved on the remote provider
    When the reviewer executes "lgtm pr resolve 42 thread_123 --unresolve"
    Then thread "thread_123" is marked as unresolved on the remote provider

  Scenario: Toggling thread resolution in the interactive TUI
    Given the reviewer is in the "Diff" view with cursor on thread "thread_123"
    When the reviewer presses "R"
    Then the resolution status of thread "thread_123" is toggled
    And the thread status badge in the gutter updates immediately

  # REQ-COM-05: Multi-line and Markdown Content Delivery
  Scenario: Posting comments containing multi-line Markdown and code blocks
    Given a local review directive on pull request 42 containing:
      """
      ```rust
      fn test_optimization() {
          assert!(elapsed < 50);
      }
      ```
      """
    When the reviewer posts the comment
    Then the payload preserves all backticks, indentation, and newlines
    And the comment renders properly without JSON escaping corruption

  # REQ-COM-06: Partial Batch Delivery and Crash Recovery
  Scenario: Recovering from remote error mid-batch without duplicate delivery
    Given 3 pending directives on pull request 42
    And the remote provider accepts the first comment but fails on the second
    When the reviewer executes "lgtm pr post 42 --yes"
    Then the command reports a partial failure
    And the first directive is durably saved with its remote receipt
    When the reviewer retries "lgtm pr post 42 --yes" after network recovery
    Then only the remaining 2 directives are dispatched
    And the first comment is never duplicated
