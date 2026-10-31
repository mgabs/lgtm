# MCP Directive Line Overflow

## Problem

`lgtm_add_directive` converted an unsigned 64-bit line argument to the stored 32-bit line with an unchecked cast. Oversized values could wrap and persist a directive against the wrong line.

## Resolution

Reject values above `u32::MAX` before writing directive storage and preserve the maximum representable value exactly.

## Regression Coverage

The MCP test verifies that overflow returns an error without creating directive storage and that `u32::MAX` is persisted unchanged.
