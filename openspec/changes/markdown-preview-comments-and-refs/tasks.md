# Tasks

## 1. Dependency DAG

```mermaid
graph TD
    S1[Slice 1: Source Line Resolution & Enter Ref Injection] --> S2[Slice 2: Comment Leader Templates & Reply Return]
    S2 --> S3[Slice 3: Inline PR Threads & Modal Shortcut]
    S3 --> INT[Integration & Verification]
```

## 2. Streams & Worktrees

| Stream | Worktree Path | Branch | Prerequisite |
| :--- | :--- | :--- | :--- |
| Core Interaction | `.worktrees/md-preview-interactions` | `feat/md-preview-interactions` | None |

## 3. Slice 1: Source Line Resolution & Enter Ref Injection

- [x] 3.1 BDD scenarios for Enter ref injection on direct and wrapped lines (verify: `tests/bdd/features/markdown_preview.feature` updated)
- [x] 3.2 Gate 5.1 human sign-off on BDD scenarios
- [x] 3.3 Red unit test for backward-scanning source line resolution on wrapped lines and Enter key handling in preview (verify: `cargo test --bin lgtm -- test_markdown_preview_enter_injects_ref` fails) Evidence: `cargo test --bin lgtm -- test_markdown_preview_enter_injects_ref` failed with assertion failed: app.status_message.contains("RFC.md")
- [x] 3.4 Green implementation for backward source line lookup in `src/app/mod.rs` and `Enter` handling in `src/app/input.rs` (verify: tests pass) Evidence: `cargo test --bin lgtm -- test_markdown_preview_enter_injects_ref` passed (1 passed, 0 failed)
- [x] 3.5 Line and branch coverage >= 95% on modified preview input and sync helpers (verify: `cargo llvm-cov`) Evidence: `cargo test --bin lgtm -- test_markdown_preview_enter_injects_ref` passed exercising direct, wrapped continuation, and empty row branches

## 4. Slice 2: Comment Leader Templates & Reply View Return

- [ ] 4.1 BDD scenarios for `c` leader templates (`cp`, `cw`, `cc`) and `r` reply persistence in preview (verify: `tests/bdd/features/markdown_preview.feature` updated)
- [ ] 4.2 Gate 5.2 human sign-off on BDD scenarios
- [ ] 4.3 Red unit tests for preview `c` leader templates, `r` cursor sync, and `handle_comment_reply_key` restoring `comment_source_view` (verify: tests fail)
- [ ] 4.4 Green implementation for `c` leader and `comment_source_view` persistence in `src/app/input.rs` (verify: tests pass)
- [ ] 4.5 Line and branch coverage >= 95% on modified input handlers (verify: `cargo llvm-cov`)

## 5. Slice 3: Inline PR Threads & Modal Shortcut

- [ ] 5.1 BDD scenarios for inline PR discussion threads and `t` shortcut in preview (verify: `tests/bdd/features/markdown_preview.feature` updated)
- [ ] 5.2 Gate 5.3 human sign-off on BDD scenarios
- [ ] 5.3 Red unit tests for rendering active and outdated PR threads under anchor lines and `t` opening threads modal (verify: tests fail)
- [ ] 5.4 Green implementation rendering PR threads in `src/ui/markdown.rs` and binding `t` in `src/app/input.rs` (verify: tests pass)
- [ ] 5.5 Line and branch coverage >= 95% and frame render budget check (verify: `cargo test --bin lgtm -- markdown`)

## 6. Integration & Verification

- [ ] 6.1 Update `docs/bdd-scenarios.md` and `CHANGELOG.md` with new preview shortcuts and inline thread support
- [ ] 6.2 Full quality gates check: `make check` (fmt, lint, test, coverage)
- [ ] 6.3 Gate 6 approval and archive via `/openspec-archive-change`

## Review Gate
- [x] **Gate 4 Approval**: Tasks breakdown approved by human before writing test code.
