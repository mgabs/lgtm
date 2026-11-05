# Proposal

## Intent
Decouple local AI agent steering directives from online forge review discussions. Disambiguate local directives naming across `lgtm-core` and the TUI, and refactor forge comments into an immediate, non-blocking online tool without local disk draft queues or batch-submit workflows.

## Problem Statement
The codebase conflates local steering directives (`.lgtm/directives.jsonl`) with forge PR discussions (`.lgtm/drafts/pr-<N>.json`). Local directives are misleadingly named `CommentStore` and `CommentAnchor`. Meanwhile, forge PR discussions suffer from a heavy, multi-step draft queue requiring disk persistence, staging locks, and manual `:thread submit` confirmation, making commenting slow and cumbersome.

## Capabilities

### New Capabilities
- `directives`: Local agent-steering directives stored in `.lgtm/directives.jsonl`, managed by `DirectiveStore` and `Directive`, completely decoupled from network and forge PRs. Includes automatic conversion of static scanner findings (`GuardAlert`) into actionable directives for AI agents.

### Modified Capabilities
- `comments`: Refactored to pure online PR discussions against GitHub and Azure DevOps. Drops `.lgtm/drafts/` disk queue, draft states, and batch submit; posts inline comments, replies, and resolutions directly to the forge asynchronously.

## Scope

### In-Scope
- Rename `CommentStore` → `DirectiveStore`, `CommentAnchor` → `Directive`, and `self.comments` → `self.directives` in `crates/lgtm-core` and `src/app/`.
- Rename TUI views `CommentInput` → `DirectiveInput` and `CommentList` → `DirectiveList`.
- Store static guard scanner findings (`GuardAlert`) as directives in `.lgtm/directives.jsonl` for autonomous AI agent remediation via MCP.
- Remove `.lgtm/drafts/` directory management and `crates/lgtm-core/src/discussion/store.rs`.
- Replace `src/app/drafts.rs` draft queue with direct async forge dispatching (`post_inline`, `reply`, `set_resolved`).
- Implement an in-memory retry buffer so failed network requests do not discard typed input.
- Remove obsolete commands: `:thread drafts`, `:thread submit`, `:thread discard`, `:thread edit`.
- Keep review directives (`c`, `:c`, `:directives`, `#ref`) fully functional offline.
- Affected crates: `crates/lgtm-core`, `src/` (`lgtm` TUI). `crates/lgtm-ast` and `crates/lgtm-mcp` are unaffected.
- CI impact: Zero impact on the Ubuntu, macOS, and Windows matrix; retains existing test coverage and strict mutation kill gates.

### Out-of-Scope (YAGNI)
- Offline queuing of PR comments or background offline sync daemons.
- Rich Markdown preview within the comment input modal.
- Changing `lgtm-mcp` protocol or AST parsing logic.
- Adding third-party git forge providers beyond GitHub and Azure DevOps.

## Assumptions
- **ASM-GEN-1**: PR commenting requires an active PR context (`:pr <number>`) and online connectivity.
- **ASM-GEN-2**: Directives remain purely local and are consumed by local LLM agents and the MCP server without network I/O.
- **ASM-GEN-3**: The existing `PrDiscussion` trait methods in `lgtm-core` (`post_inline`, `reply`, `set_resolved`) handle all remote calls directly.

## Success Metrics
- 0 lines of disk serialization or file creation under `.lgtm/drafts/`.
- Deletion of ~700+ lines of draft state management and batch submission code.
- Typing a comment/reply or toggling resolution dispatches immediately to the forge and updates the gutter/threads view upon success.
- Directives and PR comments have zero shared data structures or naming collisions.

## Review Gate
- [ ] **Gate 1 Approval**: Proposal reviewed and approved by human before drafting specs.
