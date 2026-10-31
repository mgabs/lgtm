# Spec Delta

## ADDED Requirements

### Requirement: REQ-CLI-1 Initialization Repository Target
The CLI MUST initialize the repository selected by `--repo` when `--init` is supplied, and MUST use the current directory when no repository is selected.

#### Scenario: Initialize selected repository
- **WHEN** the user runs `lgtm --init --repo <path>`
- **THEN** initialization targets `<path>` and does not initialize the current directory instead

#### Scenario: Initialize default repository
- **WHEN** the user runs `lgtm --init` without `--repo`
- **THEN** initialization targets the current directory

## Review Gate
- [ ] **Gate 2 Approval**: Specifications reviewed and approved by human before technical design.
