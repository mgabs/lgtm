# Proposal

## Intent

Make terminal cleanup reliable on UI errors and ensure failed agent-message delivery is never reported as successful.

## Problem Statement

The UI loop can return before restoring terminal modes, leaving the user's terminal unusable. The batch agent-delivery path ignores sink errors, reports success, and advances the turn, hiding a failed delivery.

## Capabilities

### New Capabilities
- `runtime-error-handling`: terminal lifecycle cleanup and truthful agent message delivery results.

### Modified Capabilities
- None.

## Scope

### In-Scope
- Restore terminal state on every exit after terminal setup, without masking the original operation error.
- Surface agent sink failures and do not advance the turn as if delivery succeeded.
- Add focused regression tests for both failure paths.

### Out-of-Scope (YAGNI)
- Retrying or replaying agent messages after a failed or ambiguous delivery.
- Changing normal terminal interaction or successful delivery behavior.

## Assumptions
- **ASM-GEN-1**: Existing terminal setup and cleanup functions define the required restoration state.
- **ASM-GEN-2**: A failed sink send means delivery was not accepted and should remain actionable to the user.

## Success Metrics
- UI operation errors still return an error after terminal modes are restored.
- Failed sink delivery is surfaced and does not advance the turn or claim success.
- Focused regression tests cover both failures.

## Review Gate
- [ ] **Gate 1 Approval**: Proposal reviewed and approved by human before drafting specs.
