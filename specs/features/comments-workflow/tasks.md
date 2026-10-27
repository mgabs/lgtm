# Consolidated Comments Workflow Implementation Plan

> **Goal:** Build an end-to-end, dependable comments workflow for planning, associating, line-accurate posting, replying, and resolving review comments across TUI and headless CLI.

**Architecture:**
- Core data models and `CommentSide` in `crates/lgtm-core/src/comment/mod.rs`.
- Forge discussion traits and side parameter in `crates/lgtm-core/src/forge/`.
- Associator, boundary validator, and planning in `crates/lgtm-core/src/github/post/mod.rs`.
- CLI commands and headless runners in `src/cli.rs`, `src/headless.rs`, `src/main.rs`.
- Consolidated TUI commands and modal views in `src/app/` and `src/ui/`.

**Spec:** [`specs/features/comments-workflow/design.md`](design.md)

---

### Task 1: Core Data Model & Side Tracking (`lgtm-core::comment`)

- [x] **Step 1: Define `CommentSide` enum**
  In `crates/lgtm-core/src/comment/mod.rs`:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
  #[serde(rename_all = "UPPERCASE")]
  pub enum CommentSide {
      #[default]
      Right,
      Left,
  }
  ```
- [x] **Step 2: Add `side` field to `CommentAnchor`**
  Add `#[serde(default)] pub side: CommentSide` to `CommentAnchor`.
- [x] **Step 3: Update `CommentStore` and unit tests**
  Verify serialization and deserialization backward compatibility with existing legacy comments lacking the `side` field.

---

### Task 2: Diff Line Accuracy & Forge Trait Updates (`lgtm-core::forge`)

- [x] **Step 1: Update `PrDiscussion` trait definition**
  In `crates/lgtm-core/src/forge/mod.rs`, update `post_inline` to take `side: CommentSide`:
  ```rust
  fn post_inline(&self, pr: u64, sha: &str, path: &str, line: u32, side: CommentSide, body: &str) -> Result<u64>;
  ```
- [x] **Step 2: Update `GitHubForge` and `GhClient`**
  In `crates/lgtm-core/src/forge/github.rs` and `crates/lgtm-core/src/github/mod.rs`:
  - Pass `"side": side.as_github_str()` (`"LEFT"` or `"RIGHT"`).
  - Enrich HTTP 422 error classification with diff boundary diagnostics.
- [x] **Step 3: Update `AzureForge`**
  In `crates/lgtm-core/src/forge/azure.rs`:
  - Use `leftFileStart` / `leftFileEnd` when `side == CommentSide::Left`.
  - Use `rightFileStart` / `rightFileEnd` when `side == CommentSide::Right`.
- [x] **Step 4: Update test mocks and forge unit tests**
  Update `Fake` runners and in-memory test forge clients to verify side parameter passing.

---

### Task 3: Automatic PR Association & Planning (`lgtm-core::github::post`)

- [x] **Step 1: Implement PR Directive Associator**
  In `crates/lgtm-core/src/github/post/mod.rs`:
  - Create `associate_directives(comments: &mut [CommentAnchor], files: &[FileDiff], pr: u64) -> usize`.
  - Bind unattached comments (`pr_number: None`) whose file matches PR files to `Some(pr)`.
- [x] **Step 2: Fix `in_diff` calculation for deletions**
  Update `in_diff(files: &[FileDiff], c: &CommentAnchor) -> bool` to check:
  - For `CommentSide::Left`: `l.line_type == LineType::Deletion && l.old_line_num == Some(c.target_line_no)`.
  - For `CommentSide::Right`: `l.line_type != LineType::Deletion && l.new_line_num == Some(c.target_line_no)`.
- [x] **Step 3: Structure `Plan` and `Report`**
  Implement `Plan` with `inline`, `general`, and `skipped` (with explicit reasons).
- [x] **Step 4: Unit tests for planning & association**
  Test association of `pr_number: None` comments, deletion line placement on LEFT side, and off-diff general comments.

---

### Task 4: CLI Headless Execution & Subcommand Support

- [x] **Step 1: Refactor `Commands::Pr` in `src/cli.rs`**
  Support both `lgtm pr <NUMBER>` and `lgtm pr post [NUMBER] [--yes]`. Add `reply` and `resolve` subcommands.
- [x] **Step 2: Implement headless PR runner in `src/headless.rs`**
  - Implement `run_pr_post(repo_path: &Path, pr: Option<u64>, yes: bool) -> Result<i32>`.
  - Support automatic PR discovery from branch or comments if PR number is omitted.
  - Implement dry-run preview formatting when `yes == false`.
  - Implement confirmed batch posting when `yes == true`.
- [x] **Step 3: Implement headless reply and resolve**
  Implement `run_pr_reply` and `run_pr_resolve` in `src/headless.rs`.
- [x] **Step 4: Wire CLI commands in `src/main.rs`**
  Route `Commands::Pr` subcommands to headless runners or launch TUI.

---

### Task 5: TUI Consolidated Commands & Modal View

- [x] **Step 1: Detect line side when adding comment**
  In `src/app/comments.rs`:
  - In `save_current_line_comment`, inspect line type. Set `CommentSide::Left` and `old_line_num` for deletions, `CommentSide::Right` and `new_line_num` otherwise.
- [x] **Step 2: Auto-associate directives on PR load**
  When `:pr <n>` loads, invoke `post::associate_directives` and save if count > 0.
- [x] **Step 3: Unified `:resolve` and `:reply` commands**
  In `src/app/command.rs`:
  - Route `:resolve` to resolve cursor thread/comment.
  - Route `:reply` to reply to cursor thread/comment.
- [x] **Step 4: Update TUI comments list modal**
  In `src/ui/modals/`:
  - Display unified list of directives and remote threads with clear badges (`[Local]`, `[Pending]`, `[Remote]`).

---

### Task 6: Actionable Diagnostics, Tests & Verification

- [x] **Step 1: Run unit tests**
  `rtk cargo test --package lgtm-core --lib`
- [x] **Step 2: Run CLI and TUI integration tests**
  Verify CLI post dry-run and confirmed execution.
- [x] **Step 3: Clippy & formatting audit**
  `rtk cargo clippy --all-targets --all-features`
