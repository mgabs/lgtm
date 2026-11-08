Feature: Vendor-Neutral Thread Model and Resolution State
  As a developer reviewing code across GitHub, Azure DevOps, and GitLab
  I want PR threads to use a single provider-neutral data model with rich resolution states
  So that thread statuses, outdated markers, permissions, and fallbacks behave consistently everywhere

  # REQ-COMMENTS-1; Scenario: Azure status mapping
  Scenario: Azure status mapping
    Given an Azure DevOps thread with status "<azure_status>"
    When the thread is parsed into the neutral thread model
    Then the thread state is "<neutral_state>"

    Examples:
      | azure_status | neutral_state       |
      | fixed        | Resolved(Done)      |
      | wontFix      | Resolved(WontFix)   |
      | closed       | Resolved(Other)     |
      | byDesign     | Resolved(Other)     |
      | active       | Open                |

  # REQ-COMMENTS-1; Scenario: GitHub flags mapping
  Scenario: GitHub flags mapping
    Given a GitHub thread with isResolved "<resolved>", isOutdated "<outdated>", viewerCanResolve "<can_resolve>", and viewerCanUnresolve "<can_unresolve>"
    When the thread is parsed into the neutral thread model
    Then the thread state is "<state>"
    And the outdated flag is "<outdated>"
    And can_resolve permission is "<can_resolve>"
    And can_unresolve permission is "<can_unresolve>"

    Examples:
      | resolved | outdated | can_resolve | can_unresolve | state          |
      | true     | false    | true        | true          | Resolved(Done) |
      | false    | true     | true        | false         | Open           |
      | false    | false    | false       | false         | Open           |

  # REQ-COMMENTS-1; Scenario: GitLab discussion mapping
  Scenario: GitLab discussion mapping
    Given a GitLab discussion with resolved "true" and notes with position "<side_marker>"
    When the discussion is parsed into the neutral thread model
    Then the thread state is "Resolved(Done)"
    And the thread comment diff side is "<side>"

    Examples:
      | side_marker | side  |
      | old_line    | LEFT  |
      | new_line    | RIGHT |

  # REQ-COMMENTS-1; Scenario: Unreported capability
  Scenario: Unreported capability defaults safely without error
    Given a provider payload lacking permission metadata
    When the thread is parsed into the neutral thread model
    Then can_resolve is unknown
    And can_unresolve is unknown
    And no error is raised

  # REQ-COMMENTS-2; Scenario: WontFix on GitHub falls back to Done
  Scenario: WontFix on GitHub falls back to Done
    Given a WontFix resolution draft on a GitHub pull request thread
    When the resolution draft is submitted to the forge
    Then the thread is resolved with Done
    And the submit summary states "requested WontFix, applied Done"

  # REQ-COMMENTS-2; Scenario: WontFix on GitLab falls back to Done
  Scenario: WontFix on GitLab falls back to Done
    Given a WontFix resolution draft on a GitLab merge request thread
    When the resolution draft is submitted to the forge
    Then the discussion is resolved with Done
    And the submit summary states "requested WontFix, applied Done"

  # REQ-COMMENTS-3; Scenario: GitLab Delegated Authentication and host detection
  Scenario: GitLab host detection selects GitLab provider
    Given an origin remote URL with host "<host>"
    When the forge detects the provider
    Then the detected forge kind is "GitLab"

    Examples:
      | host                  |
      | gitlab.com            |
      | gitlab.internal.corp  |
      | my-gitlab.org         |

  Scenario: GitLab environment override token
    Given environment variable "GITLAB_TOKEN" is set to "gl-secret-token"
    When GitLab forge authenticates API requests
    Then the request uses bearer token "gl-secret-token"
    And the token is passed via private header file and not command arguments

  Scenario: GitLab authentication failure maps to glab auth login
    Given a GitLab API request returning HTTP 401
    When the error is classified
    Then the failure kind is "Authentication"
    And the error message suggests "glab auth login"

  Scenario: GitLab unsupported CI checks
    Given a GitLab merge request
    When CI checks are requested
    Then an explicit unsupported error is returned

  Scenario: GitLab per-item inline batch submit
    Given a batch of inline comments for a GitLab merge request
    When the batch is submitted
    Then each inline comment is posted individually

  # REQ-COMMENTS-2; Scenario: Other resolution is read-only
  Scenario: Other resolution is read-only
    Given an attempt to set resolution to an Other variant such as "closed"
    When the draft or state update is evaluated
    Then the request is rejected as unsupported

  # REQ-COMMENTS-1 / Persistent Draft Store; Scenario: Legacy draft loads with default Done resolution
  Scenario: Legacy draft loads with default Done resolution
    Given a persisted draft file containing legacy '{"resolved": true}' without a resolution field
    When the draft store loads the draft
    Then the draft is loaded as a resolve draft with resolution "Done"

  Scenario: Legacy unresolve draft loads as Open
    Given a persisted draft file containing legacy '{"resolved": false}' without a resolution field
    When the draft store loads the draft
    Then the draft is loaded as an unresolve draft
