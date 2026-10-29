# Feature Proposal: Online Comments (Forge as Source of Truth)

## Intent
Strip the local-first commenting engine (`.lgtm/comments.jsonl`, plan/post pipeline) and rebuild it as an online, robust flow: comment threads live on the forge (GitHub / Azure DevOps), are fetched live, and users can open a thread, reply, and resolve/unresolve directly from the TUI and CLI. Supersedes `comments-workflow`.

## Scope

### In-Scope
- Remove local comment store, association/binding, plan/preview/post pipeline, and `:c` vs `:threads` split.
- Live fetch of PR review threads (with refresh) as the single comments view.
- Open (create) a thread on a diff line/side, reply to a thread, resolve / unresolve a thread — each executed immediately against the forge.
- Robust errors: per-operation actionable failure (auth, 422 line mismatch, rate limit, offline), retry-safe (no duplicate posts on ambiguous timeout).
- CLI parity: `lgtm pr comment | reply | resolve`.
- Update MCP surface (`lgtm-mcp`) and watcher to read live threads instead of the jsonl.

### Out-of-Scope
- Offline drafting/queueing of comments.
- Markdown editor, websocket streaming, cross-fork/multi-repo.
- Autonomous bot posting.

## Success Metrics
- 0 reads or writes of `comments.jsonl`; drafts only in `.lgtm/drafts/`.
- Draft → submit batch (open, reply, resolve) verified against both forges (mocked + live provider test).
- 0 duplicate comments on retried/timeout requests.
- Every failure reports file, line, reason, remediation.

## Assumptions
- ASM-GEN-1: Drafting is local (`.lgtm/drafts/`); listing and submitting require a forge. Posting is batched, never one-by-one.
- ASM-GEN-2: Existing forge adapters (`forge/github.rs`, `forge/azure.rs`) are extended, not replaced.
- ASM-GEN-3: `#ref file:line` directives for coding agents (`.lgtm/reviews/`, MCP) are a separate channel and are kept
- ASM-GEN-4: Existing `comments-workflow` artifacts move to `specs/archive/` on approval.
