# Copilot instructions

- Answer in under 100 words. No recaps, no filler, no restating the question.
- Terse by default: one line when it fits, no explanation unless asked, never more than five bullets of one sentence each.
- Ask one question at a time.
- Don't commit unless asked. Include the `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>` trailer when you do.
- Put long output (test runs, builds, large searches) in sub-agents; return only the result.
- Skip verbose skill ceremony (brainstorming, writing-plans) unless asked.

## OpenSpec and BDD

- When reviewing or changing an OpenSpec change, check proposal, design, delta specs, tasks, BDD scenarios, and the scenario catalog for consistent scope, criteria, and references; resolve mismatches or report them.
- BDD scenarios must map to executable Rust tests and entries in `docs/bdd-scenarios.md`. Treat feature files as documentation, and tie numeric acceptance claims to an explicit measurement and scope.
- When continuing gated OpenSpec work, preserve the active feature worktree and branch. State the completed slice and next gate, and do not advance past a gate without its required approval.
