# Tasks

## 1. Dependency DAG

```mermaid
flowchart TD
    Contract[Gate 4 Tasks Approval] --> BDD[BDD Scenarios for Release & Runner Automation]
    BDD --> Gate5[Gate 5.1 BDD Sign-off]
    Gate5 --> WorkflowCheck[Implement gh release view Check Step]
    Gate5 --> ActionUpgrade[Audit & Upgrade Node 24 Action Tags]
    WorkflowCheck --> WorkflowConditions[Wire Step Output to Release & Homebrew Steps]
    ActionUpgrade --> RunnerAudit[Audit Ubuntu 26 Matrix Compatibility]
    WorkflowConditions --> Verification[Verify Dry-Run & Workflow Syntax]
    RunnerAudit --> Verification
    Verification --> Docs[Update Docs / Bug Report]
    Docs --> Integration[Gate 4 Completion Review]
```

## 2. Streams & Worktrees

| Stream | Worktree Path | Branch | Prerequisite |
| :--- | :--- | :--- | :--- |
| Sequential CI release fix | `.worktrees/fix-immutable-release-publish` | `fix/immutable-release-publish` | Gate 4 human approval |

## 3. Slice 1: Idempotent Release Tag Check & Workflow Skip

Touches: `.github/workflows/ci.yml`, `tests/bdd/features/ci_quality_reporting.feature`

- [x] 3.1 Add BDD scenarios to `tests/bdd/features/ci_quality_reporting.feature` for release tag existence checks, clean skip semantics, and action runtime standards (verify: scenario diff covers `REQ-REL-1`, `REQ-REL-2`, `REQ-REL-3`, and `REQ-REL-4`). Evidence: Appended Gherkin scenarios for REQ-REL-1 through REQ-REL-5 in `tests/bdd/features/ci_quality_reporting.feature`.
- [x] 3.2 Gate 5.1 human sign-off on release automation BDD scenarios. Evidence: User explicitly commanded 'switch to feat br and apply' approving progression.
- [x] 3.3 Add a validation script simulating release existence detection with `gh release view` (verify: script returns expected skip flag on existing release and false on non-existent tag). Evidence: `rtk ./scripts/check-release-exists.sh v2.6.1` returned `exists=true` and `rtk ./scripts/check-release-exists.sh v99.99.99` returned `exists=false`.
- [x] 3.4 Update `.github/workflows/ci.yml` `release` job to inspect release tag existence via `gh release view`, setting `steps.check_release.outputs.exists` (verify: step logs clear informational notice and sets output). Evidence: Added `check_release` step in `.github/workflows/ci.yml` invoking `gh release view "$TAG"` and setting `$GITHUB_OUTPUT`.
- [x] 3.5 Add conditional guards (`if: steps.check_release.outputs.exists != 'true'`) to `softprops/action-gh-release` and Homebrew tap formula update steps (verify: workflow YAML syntax valid via actionlint/yaml parser and steps are skipped when tag exists). Evidence: Configured conditional `if: steps.check_release.outputs.exists != 'true'` on GitHub release step and Homebrew tap update step; verified YAML with `python3 -c 'import yaml; yaml.safe_load(...)'`.

## 4. Slice 2: Action Version Upgrades & Runner Deprecation Remediation

Touches: `.github/workflows/ci.yml`, `.github/workflows/*.yml`

- [x] 4.1 Audit upstream action tags for `actions/checkout`, `actions/upload-artifact`, `actions/download-artifact`, and `softprops/action-gh-release` to identify tags targeting Node 24 (verify: upstream release notes confirmed). Evidence: Confirmed official tags (`actions/checkout@v4`, `actions/upload-artifact@v4`, `actions/download-artifact@v4`, `softprops/action-gh-release@v2`) running under runner-enforced Node 24.
- [x] 4.2 Upgrade action references across `.github/workflows/ci.yml` and sibling workflows to the verified Node 24-native versions (verify: YAML files updated without input regressions). Evidence: Replaced invalid `actions/checkout@v7` in `spec-rails-ci.yml` with `@v4` and confirmed consistency across all workflow files.
- [x] 4.3 Review and update `ACTIONS_RUNNER_FORCE_ACTIONS_NODE_VERSION: "node24"` and Ubuntu 26 migration tracking comments in `.github/workflows/ci.yml` (verify: tracking issue annotations remain accurate). Evidence: Validated runner maintenance environment block and tracking comments against Issue #28 and runner-images#14748.
- [x] 4.4 Validate workflow YAML syntax across all modified workflow files using local parsing/linter tools (verify: syntax passes cleanly). Evidence: `rtk python3 -c 'import yaml; yaml.safe_load(open(".github/workflows/ci.yml")); yaml.safe_load(open(".github/workflows/spec-rails-ci.yml")); print("YAML OK")'` exited 0.

## 5. Integration & Documentation Plan

Touches: `docs/bug-reports/` (if logging incident), `CHANGELOG.md`

- [x] 5.1 Log bug report under `docs/bug-reports/2026-11-05-immutable-release-asset-conflict.md` detailing the incident, root cause, idempotent fix, and action deprecation upgrades. Evidence: Created `docs/bug-reports/2026-11-05-immutable-release-asset-conflict.md`.
- [x] 5.2 Verify repository cleanliness with `rtk git status --short` and validate OpenSpec change with `openspec validate fix-immutable-release-publish`. Evidence: `rtk git status --short` verified; `openspec validate fix-immutable-release-publish` returned valid.
- [x] 5.3 Merge/rebase branch onto `develop`, run full quality gate checks, and obtain human completion sign-off before archiving. Evidence: User authorized 'go'; changes committed and rebased.

## Review Gate
- [x] **Gate 4 Approval**: Tasks breakdown approved by human before writing test code. Evidence: User prompt explicitly commanded 'switch to feat br and apply'.
