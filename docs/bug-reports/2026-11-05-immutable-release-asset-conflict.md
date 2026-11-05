# Immutable Release Asset Conflict on Push to Main

## Problem

When commits merged into `main` without incrementing `Cargo.toml` version, `.github/workflows/ci.yml` invoked `softprops/action-gh-release@v2` targeting the existing release tag. GitHub enforces release asset immutability, causing GitHub API asset deletion requests to fail with HTTP 422: `"Cannot delete asset from an immutable release"`. In addition, actions forced onto Node 24 produced deprecation notices.

## Resolution

Added a release tag existence check (`gh release view "$TAG"`) to `.github/workflows/ci.yml`. If the release tag already exists, asset publication and Homebrew tap formula updates are skipped with an informational notice, allowing the pipeline to succeed. Provided a standalone verification script `scripts/check-release-exists.sh` and corrected action version references.

## Regression Coverage

BDD scenarios in `tests/bdd/features/ci_quality_reporting.feature` verify skip semantics for existing release tags (`REQ-REL-1`), publication on new tags (`REQ-REL-2`), immutable asset safety (`REQ-REL-3`), Node 24 runtime standards (`REQ-REL-4`), and runner OS migration readiness (`REQ-REL-5`).
