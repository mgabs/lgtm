# Agent Delivery Failure Reported as Success

## Problem

The batch review path ignored an agent sink error, displayed successful delivery, and advanced the review turn despite the message not being accepted.

## Resolution

Surface sink errors, retain the saved review for retry, and leave the turn unchanged when delivery fails. PR posting results remain independently reported.

## Regression Coverage

Tests reject the sink send and assert that the failure remains visible, the saved review remains available, and the turn is not advanced.
