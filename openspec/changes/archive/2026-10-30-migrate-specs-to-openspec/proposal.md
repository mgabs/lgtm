# Proposal

## Why

Specifications live in two places: a hand-rolled `specs/` tree governed by `specs/WORKFLOW.md`, and an empty `openspec/` root that the project's `/opsx-*` prompts and skills already target. Two systems means drift, and the OpenSpec CLI (`list`, `validate`, `archive`) cannot see any existing spec.

## What Changes

- Convert each legacy capability spec (`specs/**/spec.md`) into an OpenSpec main spec at `openspec/specs/<capability>/spec.md` (`## Purpose`, `### Requirement:` with SHALL/MUST, `#### Scenario:` WHEN/THEN).
- Move legacy per-feature artifacts (proposal, design, tasks) into `openspec/changes/archive/<date>-<name>/`. All feature task lists are fully checked except `online-comments`, which has no checkboxes and is archived as historical.
- Move `specs/bug-reports/` content into `docs/bug-reports/` (not spec material; OpenSpec has no bug-report concept).
- Replace `specs/WORKFLOW.md` with OpenSpec-based guidance: keep the branching/worktree, test-first and coverage/mutation gates as `context`/`rules` in `openspec/config.yaml`; drop the manual gate/layout text.
- Delete `specs/` entirely.
- **BREAKING** for contributors: `specs/` paths and `WORKFLOW.md` disappear; new work goes through `/opsx-propose`.

## Capabilities

### New Capabilities
<!-- None as spec deltas: this change is a tooling/docs migration. The migrated capabilities are written straight into openspec/specs/ because they describe behavior that already ships; the change sets skip_specs: true. -->

### Modified Capabilities

## Impact

- Removed: `specs/` (34 files). Added: `openspec/specs/*`, `openspec/changes/archive/*`, `docs/bug-reports/`.
- Edited: `openspec/config.yaml`, and any docs/prompts referencing `specs/` or `WORKFLOW.md` (none found in README, CONTRIBUTING, Makefile, scripts; recheck `docs/` and `.github/` during apply).
- No source code or runtime behavior changes.
