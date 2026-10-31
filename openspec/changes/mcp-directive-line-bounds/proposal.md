# Proposal

## Intent

Reject out-of-range MCP directive line numbers instead of silently wrapping them to a different source line.

## Problem Statement

The MCP directive tool accepts a `u64` line and casts it unchecked to a stored `u32`, so an oversized input can persist at an unrelated line.

## Capabilities

### New Capabilities
- `mcp-input-validation`: validation of MCP directive arguments before persistence.

### Modified Capabilities
- None.

## Scope

### In-Scope
- Reject line values above `u32::MAX` with a clear argument error before persisting a directive.
- Preserve the exact line value for representable inputs.
- Add regression tests for the maximum representable value and overflow.

### Out-of-Scope (YAGNI)
- Defining additional validation rules for representable values, including line zero.
- Broad validation changes to unrelated MCP tools.

## Assumptions
- **ASM-GEN-1**: The stored directive line remains a `u32`.
- **ASM-GEN-2**: Invalid inputs must fail explicitly and must not create or alter directive storage.

## Success Metrics
- Values above `u32::MAX` return an error and do not persist a directive.
- `u32::MAX` is converted without loss.

## Review Gate
- [ ] **Gate 1 Approval**: Proposal reviewed and approved by human before drafting specs.
