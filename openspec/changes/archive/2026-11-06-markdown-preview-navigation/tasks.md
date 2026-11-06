# Tasks

## 1. Dependency DAG

```
+-----------------------------------------------------------+
| Slice 1: Editor & Bracket Navigation Keystrokes           |
| (sync cursor, switch active view, launch editor / diff)   |
+-----------------------------------------------------------+
                             |
                             v
+-----------------------------------------------------------+
| Slice 2: Auxiliary Shortcuts & Status Indicators          |
| (f/Ctrl-p file picker, v review, r reply, ? help, status) |
+-----------------------------------------------------------+
                             |
                             v
+-----------------------------------------------------------+
| Slice 3: Documentation & Changelog                        |
| (CHANGELOG.md, docs, sync specs)                          |
+-----------------------------------------------------------+
```

## 2. Streams & Worktrees

| Stream | Worktree Path | Branch | Prerequisite |
| :--- | :--- | :--- | :--- |
| Core | `.worktrees/md-nav-core` | `feat/markdown-preview-navigation` | None |

## 3. Slice 1: Editor & Bracket Navigation Keystrokes

- [x] 3.1 BDD scenarios for editor launch (`e`/`o`) and bracket navigation (`[`/`]`) from `MarkdownPreview` (verify: tests declared in `src/app/input.rs`)
- [x] 3.2 Red unit tests verifying `e`/`o` syncs cursor, sets `active_view = Diff`, and calls `open_in_editor` fail for the expected reason (verify: `rtk cargo test test_markdown_preview_editor_launch`)
- [x] 3.3 Red unit tests verifying `[` and `]` sync cursor, set `active_view = Diff`, and trigger diff leader chords fail for the expected reason (verify: `rtk cargo test test_markdown_preview_bracket_navigation`)
- [x] 3.4 Green implementation in `src/app/input.rs:handle_markdown_preview_key` for `e`, `o`, `[`, `]` (verify: tests pass)
- [x] 3.5 Line and branch coverage >= 95% on modified lines in `src/app/input.rs` (verify: `rtk cargo test -p lgtm --bin lgtm`)

## 4. Slice 2: Auxiliary Shortcuts & Status Indicators

- [x] 4.1 Red unit tests for `f`/`Ctrl-p` (file picker), `v` (toggle review), `r` (reply/comment), `?` (help modal) from `MarkdownPreview` (verify: test failure output)
- [x] 4.2 Green implementation in `src/app/input.rs:handle_markdown_preview_key` for `f`, `Ctrl-p`, `v`, `r`, `?` (verify: tests pass)
- [x] 4.3 Update status bar in `src/ui/status.rs` to render `[e] Edit | [[/]] Nav | [f] Files | [v] Review` in preview mode (verify: `rtk cargo test status`)
- [x] 4.4 Mutation testing verification ensuring all new branches are covered (verify: mutation suite report)

## 5. Slice 3: Documentation & Changelog

- [x] 5.1 Update `CHANGELOG.md` with new `MarkdownPreview` shortcuts (`e`, `[`/`]`, `f`, `v`, `r`, `?`)
- [x] 5.2 Sync specs with `openspec/specs/markdown-preview/spec.md`

## 6. Integration & Rebase Plan

- [x] 6.1 Run full workspace test suite and linter checks (`rtk cargo test --workspace`, `rtk cargo clippy --all-targets`)
- [x] 6.2 Merge stream branch to working tree and verify clean git status

## Review Gate
- [x] **Gate 4 Approval**: Tasks breakdown approved by human.
