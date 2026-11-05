# Research & Technical Tradeoffs

## 1. Intent & Ground Truth
- **Problem**: When `ci.yml` runs on push/merge to `main`, the `release` job extracts `VERSION` from `Cargo.toml`. If `v${VERSION}` was already published on GitHub with assets, GitHub's release immutability rules prevent asset replacement. `softprops/action-gh-release@v2` issues `DELETE /repos/{owner}/{repo}/releases/assets/{id}` before re-uploading, failing with `422 Unprocessable Entity`: `"Cannot delete asset from an immutable release"`.
- **Existing Triggers**: `.github/workflows/ci.yml` line 125 triggers `release` on `github.event_name == 'push' && (github.ref == 'refs/heads/main' || github.ref == 'refs/heads/master' || startsWith(github.ref, 'refs/tags/v'))`.
- **Verified Constraints**:
  - Pushes to `main` happen frequently (e.g. merge commits, docs, minor chores) without a version bump in `Cargo.toml`.
  - Releases and Homebrew tap formula updates should only happen once per semver tag or when a genuine new version is pushed.
  - Release assets must be immutable once published to prevent supply chain tampering.

## 2. Options Explored

### Option A: Idempotent Tag/Release Check in Workflow (Recommended)
- **Description**: Add a pre-check step in the `release` job using GitHub CLI (`gh release view "v${VERSION}"`) or `git tag -l "v${VERSION}"` to detect if the target release already exists. If it exists, output `skip_release=true` and cleanly exit or skip the release upload and Homebrew tap update steps with an explanatory notice.
- **Pros**:
  - Zero disruption to existing push-to-main workflow.
  - Handles automated merges cleanly without requiring manual version bumps on every commit to main.
  - Pure workflow YAML change; zero new runtime dependencies or binary changes.
- **Cons**: Requires step conditional checks in GitHub Actions (`if: steps.check_release.outputs.skip_release != 'true'`).

### Option B: Trigger Release Exclusively on Git Tags (`refs/tags/v*`)
- **Description**: Remove `main` and `master` branch pushes from the `release` job condition (`if: startsWith(github.ref, 'refs/tags/v')`). Releases only occur when a maintainer pushes an annotated/lightweight git tag matching `v*`.
- **Pros**:
  - Industry-standard release workflow; total separation between trunk commits and formal artifact publication.
  - Guarantees `main` commits never attempt release asset uploads.
- **Cons**:
  - Requires changing the release procedure: merging to `main` no longer auto-publishes; maintainers must remember to create and push a git tag for releases.

### Option C: Bump `Cargo.toml` on Every Main Push via CI Bot
- **Description**: Add an automated commit bot or semantic release step that increments patch version and tags automatically.
- **Pros**: Every push to `main` creates a unique version.
- **Cons**: High complexity, potential recursion in CI, unnecessary version bloat for chores, and fragile permissions.

## 3. Tradeoff Matrix

| Criteria | Option A (Idempotent Check) | Option B (Tag-Only Triggers) | Option C (Auto-Bump Bot) |
| :--- | :--- | :--- | :--- |
| **Complexity** | Low (single shell check step) | Low (one line `if` change) | High (git bot, PAT, push back) |
| **New Dependencies** | None (`gh` CLI is standard) | None | Bot credentials, release tool |
| **Performance & Latency** | Negligible (~1s API call) | Zero (job skipped on push) | Adds extra build/commit roundtrips |
| **Maintenance Burden** | Minimal | Low (requires tagging habit) | High (race conditions, merge loops) |
| **Blast Radius** | Scoped to `ci.yml` release job | Releases require manual tags | Repository git history & CI |

## 4. Recommendation
Option A is the most pragmatic, YAGNI-aligned solution: check if `v${VERSION}` already exists via `gh release view`, skip asset upload and Homebrew tap update if already published, and allow smooth operation without breaking changes to team workflow.

## 5. Breaking Changes & Deprecations
None. No existing `REQ-*` or `NFR-*` specifications in `openspec/specs/` are modified or deprecated.

## Review Gate
Option A
