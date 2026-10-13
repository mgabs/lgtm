# lgtm Roadmap

Date: 2026-10-12 · Status: in implementation · Baseline: `main` @ a90d5b0 (v0.7.0)

**Progress:** R1–R5, B1–B5, C1–C5, A1–A3, E1–E4 done; R6 and E5 dropped. Pending: A4–A6, D1–D4.

| Branch | Contents | Release | State |
|---|---|---|---|
| `rel.0.8` | R1–R5, B1–B5, C1–C5, A1–A3, E1–E4 | v0.8.0 | all work merged here; version 0.8.0; pushed; PR to `main` not opened yet (`gh` unauthenticated) |

All waves land in v0.8.0 (decision: one release). `rel.0.10`, `rel.0.11` and `rel.0.12` are superseded by `rel.0.8`. A4–A6 and D1–D4 are open and will follow in later releases.

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
| A2 ✅ | TUI outcomes, filters, `:directives recheck`, confirm-resolve | S | A1 |
| A3 ✅ | New MCP tools incl. claims | M | A1, R4 |
| A4 | MCP resources, subscriptions, notifications | L | A3 |
| A5 | Agent sinks: Zellij, WezTerm, kitty | M | R3 |
| A6 | Docs incl. MCP client setup | S | A4, A5 |

### D — CI/CD step (spec: `specs/2026-10-12-cicd-integration-design.md`, draft)
| ID | Item | Effort | Depends on |
|---|---|---|---|
| D1 | Shared `DiffMode` + `--rev` on `check`/`ci`; exit codes 0/1/2 | S | B |
| D2 | Composite GitHub Action: verified install, gate, SARIF upload, job summary | M | D1 |
| D3 | CI recipes (GitLab, Azure DevOps), pre-commit hook, WIKI section | S | D1 |
| D4 | Self-test workflow against fixture PR branches | S | D2 |

### E — AST layer, web (React/Next.js) and Swift packs (spec: `specs/2026-10-12-ast-web-swift-design.md`, draft)
| ID | Item | Effort | Depends on |
|---|---|---|---|
| E1 ✅ | `lgtm-ast` crate (tree-sitter), TS/TSX, diff mapping, one rule end to end | L | B |
| E2 ✅ | `web/` + `next/` guard pack (6 rules; `"use client"` leak and server-action validation deferred, need whole-file parse) | M | E1 |
| E3 ✅ | Swift grammar + `swift/` pack | M | E1 |
| E4 ✅ | Complexity and symbol guards on AST for TS/JS/Swift. Test-neutering stays line-based: its rules are one-line patterns, so the AST adds little | M | E1 |
| E5 ❌ | TUI folding on AST. Dropped: fold state is per hunk and cursor motion indexes hunk lines, so in-hunk blocks would touch every motion path for a feature unverifiable without interactive use. Revisit on demand | S | E1 |
| E6 | Whole-file parse mode in `lgtm-ast`; `"use client"` leak and server-action validation rules. Diff hunks only select which findings to keep | M | E2 |
| E7 | Python grammar: AST complexity and identifiers, a few rules (`eval`/`exec`, `subprocess shell=True`, `pickle.loads`, bare `except`) | M | E1 |
| E8 | Go, then Java grammars: complexity and identifiers first, rules on demand | M | E7 |
| E5′ | Block-level folding (revisit E5): fold state keyed by AST block range inside a hunk; needs a visible-rows indirection for cursor motion. Do it after F1 so folds can persist in the review store | M | E1, F1 |

### F — Review UX debt (comments and threads)
| ID | Item | Effort | Depends on |
|---|---|---|---|
| F1 | Comments into the review store: anchor by hunk hash, re-anchor on reload, mark orphans. One-time import of `comments.jsonl`; directive ids stay stable | M | C1 |
| F2 | Orphan UI: `⚠` marker and `:directives orphans`, jump to the nearest hunk, re-pin or delete. Directive recheck re-anchors instead of only re-hashing | S | F1 |
| F3 | Inline thread markers: map remote thread path/line to diff rows (reuse the `post` mapper), margin glyph (`◆` open, `✓` resolved), `]t`/`[t` to jump, reply and resolve from the marker | M | C4 |

Order: F1 → F2 → F3, then E6 → E7 → E8, then E5′. D1–D3 (CI step with `--rev` and the Action) stay first, since they unlock adoption.

Order: web first (E1 → E2), then Swift (E3), then E4/E5.

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
| 2 B | v0.8.0 | No-config output byte-identical to v0.8.0 (except added `rule_id`); `.lgtm.toml`, scoped suppressions and baselines work end to end on a fixture repo; Critical never baselined by default |
| 3 C | v0.8.0 | Re-anchoring property tests pass (insert above, move, rename); migration from old files tested; posting resumable against the fake client; manual smoke test against a real PR |
| 4 A | v0.8.0 for A1–A3; A4–A6 later | Claim-vs-diff matrix tested; one real MCP client smoke-tested; sinks smoke-checklist completed; existing MCP tools backward compatible |
| 5 E | v0.8.0 | E1–E4 done: web and Swift rules have positive/negative fixtures; complexity/typo guards on AST; non-AST languages unchanged. Open: whole-file parse for `"use client"` and server-action rules |
| 6 D | later | Pending: `--rev` on `check`/`ci`, exit codes 0/1/2, composite Action |

Unverified in the field (manual checks before release): `gh` calls (`checks`, GraphQL threads, REST post/reply/resolve), TUI parts (`M`, `✓n/m`, CI header, verdict tags), Homebrew tap and `cargo install --git`.

Branch protection / required status checks on trunk is a manual human step (not verifiable from the checkout); confirm before wave 1 starts.
