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

  # REQ-CIQR-3; Scenario: Upstream failure prevents generation
  Scenario: Upstream failure prevents generation
    Given a test suite failure upstream of SARIF generation
    When the CI workflow evaluates the SARIF upload step
    Then upload is skipped without producing a missing-file error

  # REQ-CIQR-3; Scenario: Scanner finds blocking issues
  Scenario: Scanner finds blocking issues
    Given a code review scan finding blocking security warnings
    When the scanner writes a SARIF report and exits with status 1
    Then the SARIF report remains eligible for upload
    And the CI run reflects the failing exit status

  # REQ-CIQR-4; Scenario: Synthetic finding in a test
  Scenario: Synthetic finding in a test
    Given a test scanning synthetic fixtures with token warnings
    When the test runs under GitHub Actions environment detection
    Then annotations are captured in an in-memory buffer rather than emitted as workflow commands

  # REQ-CIQR-4; Scenario: Real CLI finding
  Scenario: Real CLI finding
    Given a real CLI invocation of the scanner with blocking findings
    When annotations format is requested
    Then error annotations are emitted to stdout and exit status is 1

  # REQ-REL-1; Scenario: Push to main with already-published version
  Scenario: Push to main with already-published version
    Given a push or merge to main for a version that already has a published GitHub release
    When the release job inspects release tag existence
    Then the release and Homebrew tap update steps are skipped
    And the job logs an informational notice and completes with exit status 0

  # REQ-REL-2; Scenario: New version tagged or bumped on main
  Scenario: New version tagged or bumped on main
    Given a commit or tag with a version that does not exist in published releases
    When the release job executes
    Then the GitHub release is created with build bundle assets
    And the Homebrew tap formula is updated with computed checksums

  # REQ-REL-3; Scenario: Duplicate release attempt avoids asset deletion
  Scenario: Duplicate release attempt avoids asset deletion
    Given an existing release with immutable assets
    When the release job evaluates the target release
    Then no HTTP DELETE calls are dispatched against ReleaseAsset endpoints

  # REQ-REL-4; Scenario: Workflow step executes under Node 24 runner
  Scenario: Workflow step executes under Node 24 runner
    Given workflow actions executed in GitHub Actions runners
    When checkout, artifact upload/download, or release steps run
    Then actions run natively under Node 24 without runtime deprecation notices

  # REQ-REL-5; Scenario: Quality gate executes on updated runner images
  Scenario: Quality gate executes on updated runner images
    Given CI runner environments migrating to updated Ubuntu releases
    When the quality gate job executes on Ubuntu runners
    Then toolchains and build steps succeed without runner image obsolescence errors

