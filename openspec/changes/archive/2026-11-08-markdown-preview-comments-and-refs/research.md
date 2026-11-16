# Research & Technical Tradeoffs

## 1. Intent & Ground Truth

- **Problem**: Reviewers in Markdown preview mode (`ActiveView::MarkdownPreview`) cannot blast line references to agent sinks (`Enter` does nothing), cannot reliably reply to comments/threads (`r` evaluates line indexes on the stale diff cursor and returns to Diff view on submit/cancel), cannot use `c` leader templates (`cp`, `cw`, `cc`), and cannot see PR discussion threads (`app.threads`) rendered inline or open the threads modal (`t`).
- **Verified Constraints**:
  - `Enter` in Diff view invokes `App::inject_reference_to_agent()` (`src/app/external.rs:152`).
  - Wrapped continuation rows in `build_markdown_preview()` have `line_number: None` (`src/app/markdown.rs:376, 409, 432`). Finding the source line requires scanning backward to the parent line of the paragraph/block.
  - `handle_markdown_preview_key()` calls `thread_at_cursor()` and `comment_idx_at_cursor()` before `sync_cursor_to_markdown_row()` (`src/app/input.rs:1785-1796`).
  - `handle_comment_reply_key()` hardcodes `self.active_view = ActiveView::Diff;` on Enter/Esc instead of restoring `self.comment_source_view`.
  - Preview renderer in `src/ui/markdown.rs` queries `app.comments` (local directives) only, ignoring `app.threads`.
  - Hot path performance: scrolling must remain < 1 µs, frame rendering < 16 ms.

## 2. Options Explored

- **Option A (Shared Action Parity & Unified Inline Decorators)**:
  - Add `Enter` to `handle_markdown_preview_key` mapping the cursor row (including wrapped continuation lines) to the parent source line and triggering `inject_reference_to_agent()`.
  - Wire `pending_leader = Some('c')` in preview for `cp`, `cw`, `cs`, `cr`, `ca`, `cc` templates.
  - Call `sync_cursor_to_markdown_row()` before evaluating `thread_at_cursor()` / `comment_idx_at_cursor()` on `r`.
  - Preserve `comment_source_view` in `handle_comment_reply_key` to stay in MarkdownPreview upon reply completion or cancellation.
  - Extend `src/ui/markdown.rs` to render PR threads inline beneath their anchor line alongside local directives, and bind `t` to open `open_threads_modal()`.
  - *Pros*: Full behavioral and ergonomic symmetry between Diff and MarkdownPreview views; zero view disorientation.
  - *Cons*: Slightly increases line height computations in preview scroll math (already supported by variable row height cache).

- **Option B (Minimal Enter & Reply Fix Only)**:
  - Only bind `Enter` to inject references and fix `r` cursor sync. Leave `c` as instant plain input and keep PR threads modal-only (`t`).
  - *Pros*: Slightly fewer LOC touched.
  - *Cons*: Leaves preview experience broken and inconsistent with diff view; PR reviewers cannot see discussions contextually in prose preview.

## 3. Tradeoff Matrix

| Criteria | Option A (Unified Parity) | Option B (Minimal Fix) |
| :--- | :--- | :--- |
| **Complexity** | Low (uses existing state hooks & row expanders) | Very Low |
| **New Dependencies** | None | None |
| **Performance & Latency** | Retains < 16ms render & < 1µs scroll budgets | Retains budgets |
| **Maintenance Burden** | Low (reuses `comment_source_view` & existing thread structures) | Moderate (divergent view logic) |
| **Blast Radius** | Low (`src/app/input.rs`, `src/ui/markdown.rs`, `src/app/mod.rs`) | Low |

## 4. Recommendation

Adopt Option A. Unifies review interaction ergonomics across views without new dependencies or frame budget regressions, adhering strictly to YAGNI.

## 5. Breaking Changes & Deprecations

None. Extends `markdown-preview` and `comments` specs with preview interaction parity without changing existing diff view behaviors or data contracts.

## Review Gate
- [x] **Gate 0 Decision**: Go (Option A: Shared Action Parity & Unified Inline Decorators)
