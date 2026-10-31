# Proposal

## Intent

Make the `--init` shortcut initialize the repository selected by `--repo`, matching normal parsed initialization.

## Problem Statement

The early `--init` path bypasses CLI parsing and hardcodes the current directory, silently ignoring an explicit repository target.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `openspec/specs/clean-commands-shortcuts/spec.md`: define repository targeting for `--init`.

## Scope

### In-Scope
- Honor `--repo <path>` when `--init` is supplied.
- Retain current-directory behavior when no repository is selected.
- Add regression coverage for the selected target and default.

### Out-of-Scope (YAGNI)
- Redesigning global CLI argument parsing or changing unrelated command options.
- Changing the semantics of `init` beyond repository selection.

## Assumptions
- **ASM-GEN-1**: The repository path parsed by the CLI is the authoritative target for initialization.
- **ASM-GEN-2**: Omitting `--repo` continues to mean the current directory.

## Success Metrics
- `--init --repo <path>` initializes only the selected path.
- `--init` without `--repo` retains existing default behavior.

## Review Gate
- [ ] **Gate 1 Approval**: Proposal reviewed and approved by human before drafting specs.
