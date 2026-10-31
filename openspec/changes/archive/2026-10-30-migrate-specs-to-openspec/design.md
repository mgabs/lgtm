# Design

## Context

Legacy layout: `specs/features/<name>/{proposal,design,tasks,spec}.md` (some with nested `specs/<cap>/spec.md`), `specs/archive/<name>/...`, one bug report, and `WORKFLOW.md` (gates, worktree rules, TDD/coverage rules). Legacy specs use `REQ-XX-nn` bullets, not OpenSpec requirement/scenario format. `openspec/specs/` and `openspec/changes/archive/` are empty.

## Goals / Non-Goals

**Goals:** one spec system; every shipped behavior spec reachable via `openspec list --specs`; all migrated specs pass `openspec validate --specs --strict`; no history lost (git history plus archive).

**Non-Goals:** re-verifying spec accuracy against code; changing behavior; rewriting design/task prose.

## Decisions

- **Specs go directly to `openspec/specs/`, change uses `skip_specs: true`.** The behavior already ships, so delta ADDED requirements would archive into the same files. Alternative (one delta per capability, then archive) adds ceremony and ~1000 lines of duplicate review for no gain.
- **Capability names** follow existing legacy names: `line-editing`, `markdown-preview`, `comments` (merging `comments-workflow` and `online-comments`; `online-comments` supersedes the older one per its T0 "Archive old spec" task), `clean-commands-shortcuts`, `large-pr-performance`, `windows-support`, `worktree-switcher`. Flat layout, matching the legacy layout.
- **REQ conversion:** each `REQ-*` becomes one `### Requirement:` (keep REQ id in the name for traceability), description ≤500 chars, the MUST sub-bullets become `#### Scenario:` WHEN/THEN blocks. Requirements without testable detail get a minimal scenario rather than being dropped.
- **Non-spec artifacts are archived verbatim** (moved with `git mv`) under `openspec/changes/archive/YYYY-MM-DD-<name>/`, using the date of the last commit touching each directory.
- **Bug report to `docs/bug-reports/`**, not deleted, since it records a regression and its tests.
- **WORKFLOW.md rules are preserved as OpenSpec config**: branching/worktree rule, test-first with ≥95% coverage and mutation gates, and the `rtk` command prefix go into `config.yaml` `context`/`rules`. Manual Gate 1–6 text is dropped since OpenSpec's artifact flow replaces it.

## Risks / Trade-offs

- [Lossy conversion of REQ text] → convert mechanically, keep REQ ids, review diff per capability; legacy files stay in git history.
- [Merge of two comments specs creates contradictions] → use `online-comments` as the base; carry over only non-conflicting requirements from `comments-workflow`, listing any dropped ones in the commit message.
- [Dangling references to `specs/`] → grep the repo (`docs/`, `.github/`, `CHANGELOG.md`, source comments) before deletion.
