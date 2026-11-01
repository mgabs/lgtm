Feature: CI Quality & Performance Reporting
  As an engineering team maintaining lgtm
  I want deterministic behavioral test suites and separated release-mode performance checks
  So that CI failures pinpoint genuine regressions without false alarms from timing jitter

  # REQ-CIQR-1; Scenario: Instrumented behavioral run
  Scenario: Instrumented behavioral run
    Given behavioral tests running under test coverage or debug mode
    When the test suite executes
    Then all deterministic assertions are exercised
    And wall-clock timing assertions are not enforced

  # REQ-CIQR-1; Scenario: Optimized performance gate
  Scenario: Optimized performance gate
    Given a performance measurement run
    When the suite is executed in release mode with the perf-budgets feature
    Then strict latency budgets are enforced
    And a selector matching zero benchmark tests causes the gate to fail

  # REQ-CIQR-2; Scenario: Populated renderer state
  Scenario: Populated renderer state
    Given a mixed 5,000-line markdown document with uniform and expanded rows
    When scrolling operations are executed across seven warmed-up samples
    Then the median per-keystroke scrolling latency is less than 1 microsecond
    And resulting cursor and scroll offset states are consumed

  # REQ-CIQR-2; Scenario: Reproducible failure evidence
  Scenario: Reproducible failure evidence
    Given a performance benchmark execution
    When a measured workload exceeds its allowed latency threshold
    Then the output diagnostic reports workload name, sample policy, measured latency, and threshold
