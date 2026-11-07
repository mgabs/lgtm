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

  # REQ-DIR-4; Scenario: Guard scan populates directives via MCP
  Scenario: Guard scan populates directives via MCP
    Given static guard scans detecting alerts on modified diff files
    When an AI agent invokes "lgtm_security_scan" with persist set to true
    Then active guard alerts are persisted to ".lgtm/directives.jsonl"
    And the directives appear in "lgtm_list_directives" and "lgtm_directive_status"

  # REQ-DIR-4; Scenario: Baseline-suppressed findings do not generate directives
  Scenario: Baseline-suppressed findings do not generate directives
    Given static scans detecting alerts that match repository baseline suppressions
    When an AI agent invokes "lgtm_security_scan" with persist set to true
    Then no directives are created for those suppressed alerts

  # REQ-DIR-4; Scenario: Idempotent scan persistence
  Scenario: Idempotent scan persistence
    Given "lgtm_security_scan" is invoked multiple times with persist set to true without code changes
    Then open directives for existing alerts are not duplicated in ".lgtm/directives.jsonl"

