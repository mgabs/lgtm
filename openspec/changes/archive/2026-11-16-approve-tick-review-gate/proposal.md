# Proposal

## Intent
Allow reviewers in `lgtm` (Diff and MarkdownPreview views) to tick an unticked `## Review Gate` checkbox (`- [ ]` → `- [x]`) directly on the cursor line when pressing `:approve`, `:lgtm`, or `A`, requiring a second press to approve `.lgtm/gate.json`.

## Problem Statement
OpenSpec `spec-rails` workflows gate every phase behind a `## Review Gate` `- [ ]` checkbox in markdown files. Currently `:approve`, `:lgtm`, and `A` only approve `.lgtm/gate.json`, forcing reviewers to switch to an external editor to tick the Review Gate box before or after reviewing in `lgtm`.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `clean-commands-shortcuts`: Extend `:approve`, `:lgtm`, and `A` with two-step Review Gate checkbox ticking when the cursor is on an unticked `- [ ]` line inside a `## Review Gate` section of a working-tree file.

## Scope

### In-Scope
- Affected crates: `lgtm-core` (pure `## Review Gate` line detection and `[ ]` → `[x]` replacement) and root `lgtm` (`src/app/` two-step dispatch, working-tree file write, and `reload_diff()`).
- Support both `ActiveView::Diff` (`current_line_number()`) and `ActiveView::MarkdownPreview` (`markdown_preview_cursor_source_line()`).
- Preserve line endings (LF and CRLF) across Ubuntu, macOS, and Windows CI.
- Two-press behavior: first press ticks `- [ ]` → `- [x]` and reloads without touching `.lgtm/gate.json`; second press (when line is `[x]` or cursor is elsewhere) runs `approve_gate()` on `.lgtm/gate.json`.

### Out-of-Scope (YAGNI)
- Ticking arbitrary non-`Review Gate` markdown checkboxes (e.g., `tasks.md` implementation items).
- Unticking `[x]` back to `[ ]` via `:approve` or `:reject`.
- Auto-staging or auto-committing the ticked markdown file.
- Editing files when viewing a historical commit/revision (`active_revision.is_some()`), a PR diff (`pr.current_number.is_some()`), or a deleted diff line.

## Assumptions
- **ASM-GEN-1**: A Review Gate section starts at a `## Review Gate` markdown heading (case-sensitive or trimmed trailing whitespace) and ends at the next markdown heading (`#`–`######`) or EOF.
- **ASM-GEN-2**: The checkbox on the cursor line uses standard markdown task-list syntax (`- [ ] ...`, optionally indented).

## Success Metrics
- Pressing `:approve`, `:lgtm`, or `A` on an unticked `## Review Gate` line in Diff or MarkdownPreview updates `[ ]` → `[x]` on disk, refreshes the view, and leaves `.lgtm/gate.json` untouched.
- A second press approves `.lgtm/gate.json` as before.
- All unit and BDD tests pass on Ubuntu, macOS, and Windows CI with line coverage ≥ 95% and 100% mutation kill rate on `lgtm_core`.

## Review Gate
- [x] **Gate 1 Approval**: Proposal reviewed and approved by human before drafting specs.
