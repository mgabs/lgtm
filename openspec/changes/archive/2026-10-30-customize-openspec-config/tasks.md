## 1. Config rules

- [x] 1.1 Add `rules.proposal` (non-goals, affected crates, CI/Windows impact) to `openspec/config.yaml`
- [x] 1.2 Add `rules.design` (rejected alternatives, hot-path budgets, `lgtm_core` purity)
- [x] 1.3 Add `rules.specs` (SHALL/MUST, `####` scenarios, ≤ 500-char requirements)
- [x] 1.4 Extend `rules.tasks` with a CHANGELOG/docs task for user-visible changes

## 2. Verify

- [x] 2.1 Run `openspec validate --all --strict`
- [x] 2.2 Run `openspec instructions proposal --change <any> --json` and confirm the new rules appear
