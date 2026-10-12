# lgtm Roadmap

Date: 2026-10-12 · Status: draft, prioritization in progress · Baseline: `main` @ a90d5b0 (v0.7.0)

Specs: [refactor](specs/2026-10-12-modular-breakdown-design.md) · [B guards](specs/2026-10-12-guard-config-baselines-design.md) · [C review state + GitHub](specs/2026-10-12-review-state-github-design.md) · [A agent loop + MCP](specs/2026-10-12-agent-loop-mcp-design.md)

Rules for every item: one PR, cut from trunk, merged before the next; `make check` green (fmt, clippy `-D warnings`, tests, coverage >= 90%, mutation >= 90%); docs updated in the same PR. Effort sizes are rough estimates (XS/S/M/L), not commitments.

## Work items

### Refactor (R) — structure
| ID | Item | Effort | Depends on |
|---|---|---|---|
| R1 ✅ | Mechanical split of `app`, `ui`, `main` (+ fix hard-coded `mutate` labels) — done on `refactor/r1-mechanical-split` | M | — |
| R2 ✅ | State extraction: `CommandLine`, `FilePicker`/`PanePicker`, `AuditState`, `PrState` (no `CommitForge`: one field) | M | R1 |
| R3 ✅ | Internal traits: `Guard` + registry, `DiffSource`, `AgentSink` (golden-output parity) | M | R2 |
| R4 ✅ | Cargo workspace: `lgtm-core`, `lgtm-mcp`, `lgtm` (scripts, CI, release done; tap and `cargo install --git` flows are a manual check) | L | R3 |
| R5 ✅ | Boundary enforcement CI check + docs | XS | R4 |
| R6 ❌ | Async spike for `mcp`/PR (go/no-go; dropped if no-go)  (no-go: stdio loop is sequential, PR fetch is a `gh` subprocess; tokio would add size and churn for no gain. Dropped; A4 stays threaded) | S | R4 |

### B — Guard config, baselines, suppressions
| ID | Item | Effort | Depends on |
|---|---|---|---|
| B1 ✅ | Rule IDs + fingerprints in model and reports | S | R3 |
| B2 ✅ | `.lgtm.toml` loader (repo + user), `config --show` | M | B1 |
| B3 ✅ | Findings pipeline: severity overrides, path ignores, scoped suppressions | M | B2 |
| B4 ✅ | Baseline commands and new/existing/stale partition | M | B3 |
| B5 ✅ | TUI display of suppressed/baselined + docs | S | B4 |

### C — Durable review state and GitHub
| ID | Item | Effort | Depends on |
|---|---|---|---|
| C1 ✅ | State model, migration, content-hash re-anchoring (core only, not yet wired to the UI) | M | R2 |
| C2 ✅ | Hunk marks UI, export/import (orphan handling waits for comments to move to the new store) | M | C1 |
| C3 ✅ | `GithubClient` trait + fake; read-only CI status and remote threads (no inline thread markers yet) | M | C1 (soft: R4) |
| C4 ✅ | Inline comment posting (resumable, preview before send) | M | C3 |
| C5 ✅ | Thread reply/resolve + docs | S | C4 |

### A — Agent loop, MCP, sinks
| ID | Item | Effort | Depends on |
|---|---|---|---|
| A1 ✅ | Directive outcome tracking + per-turn summary | M | C1 |
| A2 | TUI outcomes, filters, `:directives recheck`, confirm-resolve | S | A1 |
| A3 | New MCP tools incl. claims | M | A1, R4 |
| A4 | MCP resources, subscriptions, notifications | L | A3 |
| A5 | Agent sinks: Zellij, WezTerm, kitty | M | R3 |
| A6 | Docs incl. MCP client setup | S | A4, A5 |

Soft dependency: A3's `lgtm_scan_changes` prefers B1 fingerprints but has a fallback.

## Dependency spine
`R1 -> R2 -> R3 -> R4`. After R2: C1 can start. After R3: B1 and A5 can start. After R4: A3, R6.

## Prioritization (in progress)

Decided:
1. **Refactor fully first (R1–R5), then features** in the order B > C > A.
2. **Sequential refactor, then parallel feature tracks.** R1→R5 run strictly one PR at a time. After R5, tracks B, C and A may run side by side; within a track, PRs are sequential, each cut from trunk and merged before the next in that track. Priority still decides who gets attention first (B > C > A). A's tasks wait for C1 (and A3 for R4, already done by then). Shared hot spots to coordinate: the `Guard` trait, C1's state model, `app/input.rs`, and `docs/WIKI.md`.
3. **R6 (async spike) runs right after R4**, time-boxed, alongside the starting feature tracks. No-go drops R6 (not defers it) and A4 stays on the threaded design.

| Wave | Items | Why |
|---|---|---|
| 1 | R1, R2, R3, R4, R5 | Structure complete before any feature work |
| 2 | B1–B5 | Top feature priority |
| 3 | C1–C5 | Review state, then GitHub |
| 4 | A1–A6 | Agent loop last |
| — | R6 (async spike) | Right after R4, time-boxed; go/no-go |

Open: none. Prioritization complete; next step is implementation plans (writing-plans), starting with the refactor.

4. **Each wave is gated by outcome criteria and ends with a tagged release.**

| Wave | Release | Exit criteria (all must hold) |
|---|---|---|
| 1 Refactor | v0.8.0 | `make check` green on trunk; `app.rs`, `ui/mod.rs`, `main.rs` split into submodules (R1 splitter output accepted as-is: `app/input.rs` 1288, `app/motion.rs` 968, `ui/diff.rs` 911 lines; further splits only where R2 state extraction naturally shrinks them); workspace builds; `cargo install --git` verified; Homebrew tap flow verified; golden outputs of `check`/`scan`/`ci` identical to v0.7.0; boundary CI check in place |
| 2 B | v0.9.0 | No-config output byte-identical to v0.8.0 (except added `rule_id`); `.lgtm.toml`, scoped suppressions and baselines work end to end on a fixture repo; Critical never baselined by default |
| 3 C | v0.10.0 | Re-anchoring property tests pass (insert above, move, rename); migration from old files tested; posting resumable against the fake client; manual smoke test against a real PR |
| 4 A | v0.11.0 | Claim-vs-diff matrix tested; one real MCP client smoke-tested; sinks smoke-checklist completed; existing MCP tools backward compatible |

Branch protection / required status checks on trunk is a manual human step (not verifiable from the checkout); confirm before wave 1 starts.
