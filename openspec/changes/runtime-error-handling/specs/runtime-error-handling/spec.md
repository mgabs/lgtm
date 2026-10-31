# Spec Delta

## Purpose

Runtime failures must leave the terminal usable and must not be presented as successful agent-message delivery.

## ADDED Requirements

### Requirement: REQ-RUNTIME-1 Restore Terminal State on Errors
The application MUST restore terminal modes enabled during startup before returning from a UI-loop error, while preserving the original operation error.

#### Scenario: Event loop fails after terminal setup
- **WHEN** a UI event-loop step returns an error after terminal setup
- **THEN** raw mode, the alternate screen, cursor visibility, and any enabled mouse capture are restored before the error is returned

#### Scenario: Terminal restoration also fails
- **WHEN** a UI-loop operation and terminal restoration both fail
- **THEN** the operation failure remains observable and the restoration failure is not silently discarded

### Requirement: REQ-RUNTIME-2 Report Agent Delivery Failures
The application MUST report agent-message delivery as successful only after the agent sink accepts the message, and MUST NOT advance the review turn as if a failed delivery succeeded.

#### Scenario: Batch delivery is rejected
- **WHEN** the agent sink rejects a batch message
- **THEN** the user is shown the delivery failure and the review turn is not advanced

#### Scenario: Batch delivery is accepted
- **WHEN** the agent sink accepts a batch message
- **THEN** the user is shown successful delivery and the review turn advances

## Review Gate
- [ ] **Gate 2 Approval**: Specifications reviewed and approved by human before technical design.
