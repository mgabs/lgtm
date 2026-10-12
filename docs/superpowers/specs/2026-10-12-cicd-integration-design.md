# CI/CD integration: design (draft)

Status: draft for review. No code written.

## Starting point (already shipped)
- `lgtm check` (gate, exit code), `lgtm ci` (audit, `markdown|json|sarif`), `lgtm scan`.
- Baseline (`.lgtm-baseline.json`) with stable fingerprints; `.lgtm.toml` config.
- `gh`-based PR posting (C4) with resumable `:pr post`.

So CI needs packaging and a reporting contract, not new analysis.

## Goal
On every PR: fail on **new** findings only, show them where reviewers look, and never require a TTY.

## Approaches
1. **Composite GitHub Action (recommended).** `action.yml` installs a release binary (checksum-verified), runs `lgtm ci --baseline --format sarif`, uploads SARIF (code scanning annotations), writes a job summary, sets the exit code. Smallest surface; reuses everything.
2. **Docs-only recipes.** Workflow snippets for GitHub, GitLab, Azure DevOps. Zero maintenance, but every user rewires install and upload.
3. **Headless PR bot.** `lgtm ci --post` posts inline comments via C4. Duplicates code scanning and needs a token with write scope; noisy.

Recommendation: 1, plus docs for other CI systems (2). Defer 3.

## Design
- **Install:** download the tagged release asset and verify SHA-256 (same checksums the Homebrew job uses). Fallback `cargo install --git --tag`.
- **Modes:** `gate` (default; exit non-zero on new Critical/High vs baseline), `report` (never fails).
- **Outputs:** SARIF file (with `lgtmFingerprint/v1`), markdown job summary, JSON artifact. Action outputs: `new-findings`, `exit-code`.
- **Baseline:** committed file; `lgtm baseline update` is a manual, reviewed change. CI never rewrites it.
- **Diff range:** default `origin/<base>...HEAD`; needs `fetch-depth: 0`, so the Action documents it and fails with a clear message otherwise.
- **Permissions:** `contents: read`, `security-events: write` for SARIF upload only. No PR write scope by default.
- **Agent loop in CI:** out of scope; MCP stays local.

## Error handling
- Missing base ref, shallow clone, unsupported platform, checksum mismatch: fail fast with one-line cause.
- SARIF upload failure does not mask the gate result.

## Testing
- Unit: exit-code mapping, summary rendering (new vs baselined counts).
- Integration: a workflow in this repo runs the Action against a fixture PR branch with a known finding (expects fail) and a baselined one (expects pass).

## Open questions
- Which CI systems beyond GitHub matter first?
- Is a code-scanning (SARIF) dependency acceptable, given it needs GitHub Advanced Security on private repos? Fallback: job summary plus `::error` annotations.
- Pre-commit hook for local use: wanted?
