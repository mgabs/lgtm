# Proposal

## Intent
Make the GitHub Actions release publishing and Homebrew tap update pipeline idempotent on trunk pushes by verifying whether a tag and its assets are already released before attempting publication. Upgrade deprecated GitHub Actions targeting Node 20 to Node 24-compatible releases and audit workflow readiness for runner OS migration (`ubuntu-latest` migrating to Ubuntu 26).

## Problem Statement
When changes merge into `main` without a `Cargo.toml` version bump, `.github/workflows/ci.yml` invokes `softprops/action-gh-release@v2` targeting an existing release tag. Because GitHub prevents deleting assets from immutable releases, `softprops` fails with HTTP 422 (`Cannot delete asset from an immutable release`). Additionally, workflow runs emit runtime deprecation warnings for actions forced from Node 20 to Node 24 (`actions/checkout@v4`, `actions/upload-artifact@v4`, `actions/download-artifact@v4`, `softprops/action-gh-release@v2`) and migration notices for `ubuntu-latest` moving to Ubuntu 26.

## Capabilities

### New Capabilities
- `ci-release-automation`: Pre-release existence checks, skip semantics for immutable releases, conditional Homebrew tap updates, action runtime modernization (Node 24), and runner OS migration readiness.

### Modified Capabilities
None.

## Scope

### In-Scope
- Add an explicit existence check step in the `release` job of `.github/workflows/ci.yml` using `gh release view "v${VERSION}"` or git tag checks.
- Set an output flag (`skip_release=true` / `release_exists=true`) when the release tag is already published.
- Conditionally skip the GitHub release action and the Homebrew tap update step when `skip_release=true`, outputting a clean workflow notice instead of failing.
- Allow publication to proceed when a genuine new tag / version is detected.
- Audit and upgrade deprecated GitHub Action dependencies (`actions/checkout`, `actions/upload-artifact`, `actions/download-artifact`, `softprops/action-gh-release`) to versions with native Node 24 support.
- Audit and ensure workflow runner compatibility with Ubuntu 26 migration, updating tracking comments and runner definitions where appropriate.
- Affected implementation: Workflow files in `.github/workflows/`. No Rust application or library crates are modified.
- CI impact: Ubuntu release job exits cleanly on non-versioned main pushes; deprecation notices are cleared across the Ubuntu, macOS, and Windows matrix; runner migration is accounted for.

### Out-of-Scope (YAGNI)
- Modifying Rust source crates (`lgtm`, `lgtm-core`, `lgtm-ast`, `lgtm-mcp`).
- Changing semantic versioning in `Cargo.toml`.
- Introducing automated version-bumping bots or semantic-release dependencies.

## Assumptions
- **ASM-GEN-1**: `GITHUB_TOKEN` provided by GitHub Actions has read access to query existing releases via GitHub CLI (`gh`).
- **ASM-GEN-2**: Pushes to `main` without version increments are expected standard operations and should result in a successful, skipped release step.
- **ASM-GEN-3**: Upstream action dependencies provide stable tags supporting Node 24 runtime without breaking workflow inputs.

## Success Metrics
- Pushing or merging commits to `main` with an existing version in `Cargo.toml` completes the CI workflow with a green status.
- Release asset deletion HTTP 422 errors are completely eliminated.
- Node 20 runtime deprecation warnings are cleared for workflow steps.
- Runner matrix definitions and dependencies are documented and verified ready for Ubuntu 26 transition.

## Review Gate
- [ ] **Gate 1 Approval**: Proposal reviewed and approved by human before drafting specs.
