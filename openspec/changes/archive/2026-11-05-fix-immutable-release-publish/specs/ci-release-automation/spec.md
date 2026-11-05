# Spec Delta

## Purpose
Ensure CI/CD release publishing and Homebrew tap updates are idempotent, eliminate Node 20 runtime deprecation warnings across workflow actions, and ensure runner configuration compatibility with Ubuntu 26 migration.

## ADDED Requirements

### Requirement: REQ-REL-1 Idempotent Release Tag Check
The CI release workflow MUST inspect the repository to determine whether release tag `v${VERSION}` already exists prior to uploading artifacts. If the tag is already published, the pipeline MUST mark the release step as skipped and MUST NOT invoke release asset deletion or asset upload operations.

#### Scenario: Push to main with already-published version
- **WHEN** a push or merge to `main` occurs where `v${VERSION}` exists as a published release
- **THEN** the workflow detects the existing release
- **AND** skips the GitHub release upload and Homebrew tap update steps with an informational notice
- **AND** the workflow completes with a successful green status

### Requirement: REQ-REL-2 Unreleased Version Publication
When the release workflow determines that release tag `v${VERSION}` does not exist, it MUST proceed with creating the release, attaching all build artifacts, and updating the Homebrew tap formula.

#### Scenario: New version tagged or bumped on main
- **WHEN** a commit or tag is pushed with a new `v${VERSION}` that does not exist in releases
- **THEN** the workflow creates the GitHub release with attached tarball, zip, and sha256 assets
- **AND** the workflow updates the Homebrew tap formula with the new checksums

### Requirement: REQ-REL-3 Immutable Release Asset Protection
The release workflow MUST NOT execute API calls that attempt to delete existing assets from a release that is locked or marked immutable by GitHub.

#### Scenario: Duplicate release attempt avoids asset deletion
- **WHEN** the release job evaluates an existing release tag
- **THEN** no DELETE calls are issued against GitHub ReleaseAsset endpoints

### Requirement: REQ-REL-4 Node 24 Action Runtime Compatibility
All steps across `.github/workflows/` utilizing official or third-party GitHub Actions MUST reference versions targeting the Node 24 runtime where available, preventing runner-level forced migration warnings.

#### Scenario: Workflow step executes under Node 24 runner
- **WHEN** CI runs checkout, upload-artifact, download-artifact, or gh-release steps
- **THEN** the actions run natively without emitting Node 20 deprecation notices

### Requirement: REQ-REL-5 Runner OS Migration Compatibility
Workflows MUST maintain compatibility with the upcoming `ubuntu-latest` transition to Ubuntu 26, ensuring environment declarations, toolchain installations, and runner configurations do not rely on Ubuntu 24-deprecated packages or flags.

#### Scenario: Quality gate executes on updated runner images
- **WHEN** the CI pipeline runs on Ubuntu runners
- **THEN** toolchains and build steps succeed without runner image obsolescence errors

## Review Gate
- [ ] **Gate 2 Approval**: Specifications reviewed and approved by human before technical design.
