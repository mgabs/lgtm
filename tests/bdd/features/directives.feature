Feature: Local Review Directives
  As a reviewer steering AI agents
  I want to create, resolve, and list directives stored locally in directives.jsonl
  So that agents receive targeted instructions without remote forge or PR dependencies

  # REQ-DIR-1; Scenario: Directives loaded offline
  Scenario: Directives loaded offline
    Given no active network connection and no open PR
    When the user creates or lists review directives
    Then directives are read and written to ".lgtm/directives.jsonl"
    And zero network or forge API calls are made

  # REQ-DIR-2; Scenario: Add directive on cursor line
  Scenario: Add directive on cursor line
    Given the cursor is positioned on a diff line
    When the user invokes the directive input modal with key "c"
    Then the entered text is persisted to ".lgtm/directives.jsonl" with file and line anchors
    And a directive marker is visible on that diff line

  # REQ-DIR-3; Scenario: MCP client queries directives
  Scenario: MCP client queries directives
    Given stored directives in ".lgtm/directives.jsonl"
    When an AI agent calls the "lgtm_list_directives" tool via MCP
    Then all non-resolved directives are returned with "#ref file:line" references

  # REQ-DIR-4; Scenario: Guard scan populates directives
  Scenario: Guard scan populates directives
    Given static guard scans detecting alerts on modified diff files
    When alerts are converted to review directives
    Then directives are stored in ".lgtm/directives.jsonl" with rule ID, severity, and remediation text
    And MCP agents receive them as actionable tasks

