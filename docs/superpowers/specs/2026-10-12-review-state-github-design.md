# Durable Review State and GitHub Integration — Design (Project C)

Date: 2026-10-12 · Status: draft for review · Baseline: `main` @ a90d5b0 (v0.7.0)
Depends on: modular-breakdown refactor stage 2 (`PrState`, comment logic out of `App`); piece 3 uses the PR module, which moves to the core crate in stage 4. See `2026-10-12-modular-breakdown-design.md`.

## Goal

Make review work durable and connected: reviewed marks and comments survive edits and rebases, can be exported and shared, and the PR workflow gains inline comment posting, CI status and two-way remote thread handling.

## Decisions (from brainstorming)

1. Scope: all three pieces (durable local state, inline GitHub comments, PR context), phased.
2. Identity: content hash of the normalized changed lines of a hunk; the line number is a hint, not identity.
3. Storage: local `.lgtm/` (gitignored), scoped per review target, with opt-in `review export` / `import`.
4. Posting: individual inline comments (not a batched review), made safe by resumability and dry-run.
5. PR context: read-only CI status and remote threads, plus two-way reply and resolve.

## Current state (verified)

- `CommentAnchor` (`src/comment/mod.rs`) has file, line, `target_code`, `context_before/after`, text, replies, `created_at_turn`, `is_resolved`, optional `pr_number`. Stored in `.lgtm/comments.jsonl`.
- `.lgtm/state.json` holds the turn counter (`src/turn/mod.rs`). Reviewed-file marks are documented as persisted there, but only `turn` was found writing it; behavior to be confirmed during implementation.
- `.lgtm/` is gitignored. No hunk-level review state. No re-anchoring.
- `PrEngine` (`src/pr/mod.rs`) offers list, fetch diff, and `gh pr review` (single body). No inline comments, CI status or remote threads.

## Design

### 1. Review state model
- Layout: `.lgtm/reviews/<target>.json`. Target = branch or `base..head` for local reviews, `pr-<n>` for PRs. One-time automatic migration of `comments.jsonl` and `state.json`, leaving `*.bak`.
- `HunkMark { file, content_hash, last_line, reviewed_at }`. Matching: exact hash, then nearest `last_line` among equal hashes. No match => not reviewed.
- `Comment` keeps existing anchor fields and adds `content_hash`, `last_line`, `status` (`active|resolved|orphaned`), `remote_id`, `remote_thread`.
- File-level reviewed = all hunks reviewed.
- Re-anchoring on every diff reload: exact hash, nearest line, fuzzy context. No match => `orphaned`: kept, visible, flagged, never auto-posted, never deleted.
- `review export [target]` / `review import <file>`: merge by hunk hash and comment identity; conflicts reported, not silently overwritten.
- Schema `version` field; older versions migrate; newer unknown versions are refused with a clear message.

### 2. GitHub integration
- All calls via `gh api` behind a `GithubClient` trait (list threads, list checks, post comment, reply, resolve). Real impl shells out to `gh`; tests use a fake. lgtm never handles tokens. Missing or unauthenticated `gh` => clear message; local features unaffected.
- Posting: `POST /pulls/{n}/comments` with `path`, `line`, `side`, `commit_id` = PR head SHA. Out-of-diff comments are combined into one general PR comment. `remote_id` is saved after each success so re-runs post only unposted comments. Orphaned comments skipped and listed. `:pr post --dry-run` previews; real posting requires confirmation. Delay between calls for rate limits; on failure, stop, save progress, print "N of M posted". If the PR head SHA changed, warn and re-anchor first.
- CI status (read-only): `gh pr checks` or check-runs API; shown in the PR view header; refreshed on demand (`r`), not polled.
- Remote threads: GraphQL `reviewThreads`; shown inline as read-only markers (resolved dimmed) and in a thread list. Two-way: reply via REST (`in_reply_to`), resolve/unresolve via GraphQL `resolveReviewThread`. A 403 shows a clear message; no silent retries.
- Nothing is posted, replied to or resolved without an explicit confirmed action. Headless commands never post unless given `--post`.

### 3. UI
- `r` toggles current hunk reviewed; file drawer shows `✓ n/m`; existing file toggle marks/unmarks all hunks.
- Orphaned comments show ⚠ with their own filter; posted ones show `↑ #id`.
- PR view header: CI summary; thread list; `:pr post`, `:thread reply`, `:thread resolve` in wildmenu and help modal.

## Testing
- Unit: hash normalization, matching order, orphan handling, migration, export/import conflicts.
- Property-style: re-anchoring after inserting lines above, moving a hunk, renaming a file.
- Fake `GithubClient`: resumable posting (fail at comment k then re-run), out-of-diff fallback, 403, head-SHA change.
- Golden tests for exported JSON. Coverage >= 90%, mutation >= 90%.

## Delivery (one PR each, from trunk, merged before the next)
1. State model, migration, re-anchoring (no UI change; no `gh` needed).
2. Hunk marks UI, orphan handling, export/import (no `gh` needed).
3. `GithubClient` trait + fake; real read-only calls (CI status, threads).
4. Inline comment posting (resumable) and dry-run.
5. Thread reply/resolve; docs (`docs/WIKI.md`, `CONTRIBUTING.md`).

## Out of scope
Committing review state, GitHub Enterprise behavior beyond what `gh` supports, agent-loop features (Project A).

## Risks
| Risk | Mitigation |
|---|---|
| Wrong re-anchoring silently moves a comment | Exact-first matching, fuzzy only for comments, orphan instead of guess, property tests |
| Duplicate or partial posting | Save `remote_id` per success, resumable runs, dry-run and confirmation |
| Secondary rate limits from individual posts | Delay between calls, stop on failure, progress saved |
| Migration data loss | Backup `*.bak`, versioned schema, migration tests |
| `gh` unavailable or lacks permission | Local features independent; clear 403/auth messages |
| Mutation gate path breakage when files move | Follow the refactor spec's path-update rule |
