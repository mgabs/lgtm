## Why

`openspec/config.yaml` only constrains `tasks`. Proposals, designs and specs are generated with no project-specific guidance, so they drift (missing non-goals, no cross-platform or test-impact notes, inconsistent requirement style).

## What Changes

- Add `rules.proposal`, `rules.design` and `rules.specs` to `openspec/config.yaml`:
  - proposal: state non-goals and affected crates; note CI/Windows impact when relevant.
  - design: record alternatives rejected, hot-path budget impact, and `lgtm_core` purity.
  - specs: SHALL/MUST requirements, each with at least one `####` scenario, descriptions ≤ 500 characters.
  - tasks (extend): final task updates CHANGELOG.md and docs when user-visible.
- No code, CLI or spec changes.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
None. Tooling configuration only.

## Impact

- `openspec/config.yaml` only. Affects how future changes are drafted.
