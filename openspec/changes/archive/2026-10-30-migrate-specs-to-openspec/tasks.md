# Tasks

## 1. Convert capability specs

- [x] 1.1 Convert `line-editing` and `markdown-preview` (from `specs/archive/*/specs/*/spec.md`) to `openspec/specs/<cap>/spec.md`; verify with `openspec validate <cap> --type spec --strict`
- [x] 1.2 Convert `comments` from `specs/features/online-comments/specs/comments/spec.md`, folding in non-conflicting requirements of `specs/archive/comments-workflow/spec.md`; verify with `openspec validate comments --type spec --strict`
- [x] 1.3 Convert `clean-commands-shortcuts`, `large-pr-performance`, `windows-support`, `worktree-switcher` from `specs/features/*/spec.md`; verify each with `openspec validate <cap> --type spec --strict`
- [x] 1.4 Run `openspec list --specs` and confirm all 7 capabilities appear and `openspec validate --specs --strict` passes

## 2. Move history and non-spec docs

- [x] 2.1 `git mv` each legacy proposal/design/tasks set (features and archive) into `openspec/changes/archive/YYYY-MM-DD-<name>/`; verify no `proposal.md`, `design.md` or `tasks.md` remains under `specs/`
- [x] 2.2 `git mv specs/bug-reports` to `docs/bug-reports/`; verify the file exists there

## 3. Preserve workflow rules

- [x] 3.1 Port the branching/worktree, test-first (≥95% coverage, mutation) and `rtk` rules from `specs/WORKFLOW.md` into `openspec/config.yaml` `context` and `rules`; verify `openspec context --json` still parses and shows the context

## 4. Remove legacy tree

- [x] 4.1 Grep the repo for `specs/` and `WORKFLOW.md` references outside `openspec/` and update or remove them; verify the grep returns nothing stale
- [x] 4.2 `git rm -r specs/` (including `WORKFLOW.md`); verify `ls specs` fails and `openspec validate --all --strict` passes
- [x] 4.3 Run `cargo test --workspace` to confirm nothing referenced the removed files
