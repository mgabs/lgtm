# Spec Delta

## Purpose

MCP tools validate directive arguments before storing them so invalid numeric inputs cannot silently point to a different source line.

## ADDED Requirements

### Requirement: REQ-MCP-1 Validate Directive Line Range
The `lgtm_add_directive` tool MUST reject line values greater than the maximum representable stored line before persisting a directive, and MUST preserve every accepted line value exactly.

#### Scenario: Maximum representable line
- **WHEN** `lgtm_add_directive` receives line `4294967295`
- **THEN** it stores the directive at that exact line

#### Scenario: Line exceeds storage range
- **WHEN** `lgtm_add_directive` receives a line greater than `4294967295`
- **THEN** it returns an argument error and does not persist a directive

## Review Gate
- [ ] **Gate 2 Approval**: Specifications reviewed and approved by human before technical design.
