# lgtm Roadmap

Date: 2026-10-12 · Status: in implementation · Baseline: `main` @ a90d5b0 (v0.7.0)

**Progress (updated 2026-10-23):** R1–R5, B1–B5, C1–C5, A1–A6, E1–E4, E5′, E6–E8, F1–F3, G1–G5, D1–D4 done; R6 and E5 dropped. Original roadmap items complete. P0 diagnostics, P1 cached viewport rendering, P2 background local refreshes, P3 bounded event processing, P4 background audits, P5 prepared render data, and P6 shared navigation mappings are complete; P7 remains open.

| Branch | Contents | Release | State |
|---|---|---|---|
| `rel.0.8.2` | D1–D3, F1–F3, G1–G5 | v0.8.2 | `--rev`, Action, CI recipes, orphaned comments, Azure DevOps read path; cut from `main` after v0.8.0, plus the roadmap commit |
| `rel.0.8` | R1–R5, B1–B5, C1–C5, A1–A3, E1–E4 | v0.8.0 | merged to `main` via PR #10, released as v0.8.0 |

Historical release plan: `rel.0.10`, `rel.0.11` and `rel.0.12` were superseded by `rel.0.8`. A4–A6 and D1–D4 have since been completed; the progress summary above reflects the current backlog.

Specs: [refactor](specs/2026-10-12-modular-breakdown-design.md) · [B guards](specs/2026-10-12-guard-config-baselines-design.md) · [C review state + GitHub](specs/2026-10-12-review-state-github-design.md) · [A agent loop + MCP](specs/2026-10-12-agent-loop-mcp-design.md)

Rules for every item: one PR, cut from trunk, merged before the next; `make check` green (fmt, clippy `-D warnings`, tests, coverage >= 90%, mutation >= 90%); docs updated in the same PR. Effort sizes are rough estimates (XS/S/M/L), not commitments.

## Work items

### P — Main-thread performance TODOs (added 2026-10-23)

P0 added debug-only render/input/refresh/callback timings, idle redraw and input-batch
counts, and an opt-in offline workload benchmark. Initial unoptimized TestBackend samples
with logging disabled measured about 408 ms for a 2,000-file local refresh, 22 ms per
10,000-line draw, and 17 ms per draw with 1,000 comments (baseline draw: 2.6 ms).
These fixture measurements motivated P2 for working-tree stalls and P1/P6 for
large-review rendering/navigation; P3 addresses repeated idle work. A release-build pseudo-terminal
smoke test with debug enabled confirmed about 9 idle redraws/second across these scenarios
(roughly 30 idle frames in each three-second sample); selected-line/comment counts were
checked against the fixtures. Recheck with representative user workloads before treating
this as a general bottleneck ranking. See README's
debugging section for reproduction and metric definitions. P7 remains pending; completed
PR background jobs and provider optimizations are not counted as UI TODOs.

After P2, the same unoptimized 2,000-file fixture measured about 0.05 ms to request a
refresh and 398 ms to complete it: computation still takes time, but runs off the UI thread.
A release-build terminal check handled input during a 306 ms snapshot computation, with
2–3 ms input-to-draw samples and under 1 ms for the resulting UI apply. This is one local
fixture run, not a general latency guarantee. Revision loading, explicit reloads, startup,
and staging/watcher requests share a coordinator with one active and one latest queued
request. Nine deterministic tests cover coalescing, input responsiveness while blocked,
watcher/completion ordering, draft/directive preservation, selection handling, failures,
review-target cancellation, revision reloads, and initial diff-audit startup.

After P1/P6, an unoptimized 120x40 TestBackend run with diagnostics disabled measured
warm draw medians of 2.6 ms for 10,000 lines (previously 21.2 ms), and 3.5 ms for 1,000
comments (previously 16.2 ms). First draws took 158 ms and 51 ms respectively: layout
preparation is cached, but remains synchronous. The benchmark now reports that cold cost
separately. These are local fixture observations, not timing guarantees. Golden text/style
fixtures and deterministic tests cover Unicode wrapping, annotations, folding, split-pair
selection, resize, tiny viewports, scrolling beyond 65,535 rows, cache invalidation, and
bounded word preparation. Selection markers now use a fixed-width gutter to avoid reflow.
A release terminal check with 1,000 changed pairs completed 51 draws at a 2.5 ms median;
12 navigation steps caused no row-map or layout rebuilds. Folding, split view, and resize
also completed successfully. All 287 automated tests and the repository quality gates pass.
P5 subsequently removed filesystem queries from rendering.

After P3, a release terminal run recorded zero redraws or diagnostic records during three
idle seconds (previously about nine redraws/second). It processed all 1,600 held-key inputs
while a watcher refresh completed, including 38 frames during its background scan. Its
211 draws had a 2.4 ms median. The debug log lived inside `.lgtm/` to check that ignored
log notifications do not create a feedback loop. Background checks have a 16 ms polling
interval; input remains on the UI thread to preserve terminal-editor access. Separate
count/time budgets bound each batch, but cannot interrupt an individual handler. Tests
cover idle/startup, held keys, resize/focus, quit, draw failure, missing-watcher fallback,
watcher/callback fairness, slow callback yielding, internal log exclusion, and stale
completions and newer external directives behind watcher backlogs. All 299 automated
tests and the repository quality gates pass.

P4 moves repository audits, changed-file scans, risk checks, and export's fallback scan into
workers. One audit runs at a time with one latest queued request. File progress is throttled
to about twenty updates; repository scans stop between files when superseded. Old audit
results remain usable, and completions preserve newer filters/drafts without reopening a
closed view. Target, snapshot, and watcher invalidation prevent obsolete results from applying.
Export uses the displayed audit or scans when none exists; normal quit waits for its write.
Thirteen new tests cover coordination, cancellation, scanner parity, progress bounds, drafts,
failures, risk checks, and export/quit behavior. Diagnostic isolation tests cover the new spans.
All 312 automated tests and the quality gates pass. A release terminal fixture with 17 source
files measured 39.7 ms of background repository scanning, 0.05 ms to enqueue it, and under
0.01 ms to apply it. Three input batches were presented during scanning, with a maximum
1.6 ms input-to-draw sample. Full/diff/risk scans, progress counts, and report export completed.
These are local fixture observations. Diff-snapshot copying and completion UI work remain
synchronous and measured separately.

P5 removes filesystem/Git reads from production render functions. Repository availability,
turn/gate state, prompt context, staged counts, and source snippets are prepared outside
drawing. A silent worker coalesces current view demand and rejects stale results. A bounded
32-entry snippet cache keeps seven-line contexts, including missing-file results; selection
and invalidation keys prevent displaying another finding's source. Opening the commit modal
still validates the staged count once; typing uses cached data and index events refresh it.
Nine new tests cover render independence from changed/removed files, selection coalescing,
cache reuse, invalidation, watcher/gate refresh, malformed files, and modal drafts. Privacy
checks exercise cached prompt and source contents. All 321 automated tests and quality gates pass.
An unoptimized offline run measured warm draw medians of 1.4 ms for 10,000 lines (previously
2.6 ms) and 2.0 ms for 1,000 comments (previously 3.5 ms). Initial view preparation is now
reported separately (about 2.6 ms including the test callback wait); the 10,000-line cold
layout preparation remains about 157 ms. A release terminal fixture with 500 untracked files
recorded no additional view/Git-status reads during commit typing, prompt redraws, or inspector
scrolling (median draws about 1.0/1.0/0.8 ms). An external index change updated the staged
count from one to two, and two idle seconds produced no redraws. These are local fixture
observations, not latency guarantees. P7 is the remaining proposed item.

- [x] **P0 — Measure UI work behind `--debug`.** Render/draw, input-to-draw, idle frame/event counts, callback execution/queue waits, and refresh/completion phases are instrumented. Offline fixtures compare large diffs, many-comment reviews, and large working trees. Isolated-process tests verify disabled logging, paired records, activity classification, and content exclusion. Entry points: `src/main.rs`, `src/diagnostics.rs`, `src/ui/mod.rs`, `src/app/mod.rs`.
- [x] **P1 — Render only the viewport and cache diff preparation.** Avoid rebuilding off-screen lines and word differences on every redraw. Cache content/token differences and folded-row mappings; index comments, threads, and alerts by file and line. Keep cursor, selection, and search styling separate so navigation does not invalidate content caches. Entry points: `src/ui/diff.rs`, `src/ui/mod.rs`.
- [x] **P2 — Coordinate working-tree refreshes in the background.** Local startup, revision selection, and reloads compute snapshots/findings in a worker. Watcher, staging, and explicit requests coalesce; stale completions cannot replace a newer request or review target. The current snapshot, modal drafts, visual selections, newer directive edits, and posted-comment receipts are preserved until a safe update. Entry points: `src/app/refresh.rs`, `src/app/mod.rs`, `src/app/staging.rs`, `src/app/command.rs`. Directive reconciliation and mark loading remain UI work tracked by P7.
- [x] **P3 — Schedule redraws and bound event processing.** Draw after relevant activity; idle background checks wait up to 16 ms. Process at most 32 terminal events, 32 watcher messages, and eight callbacks per iteration, with separate 4 ms batch budgets. Preserve stale-result guards across watcher backlogs and ignore internal debug-log notifications. Missing-watcher status polling runs once a second. Entry points: `src/event_loop.rs`, `src/main.rs`, `src/app/mod.rs`. Individual slow completion handlers remain subject to P7 follow-up.
- [x] **P4 — Run audit coordination in a background job.** Repository/diff/risk scans run in a serialized worker coordinator with one latest queued request, numeric file progress, cancellation between repository files, and stale-target guards. Previous results and newer filters/drafts survive. Report generation and fallback scanning also run in a worker; normal quit waits for export. Entry points: `src/app/audit/jobs.rs`, `crates/lgtm-core/src/scan.rs`.
- [x] **P5 — Remove filesystem and Git status work from rendering.** Render prepared repository, turn/gate, staged-count, prompt, and snippet data. Silent worker reads coalesce to current demand; watcher/action/view changes invalidate caches and stale results are rejected. Keep a bounded snippet cache and preserve modal drafts. Opening the commit modal retains one staged-count validation. Entry points: `src/app/view_data.rs`, `src/event_loop.rs`, `src/ui/modals/commit.rs`, `src/ui/audit.rs`, `src/ui/header.rs`, `src/ui/mod.rs`.
- [x] **P6 — Reuse visible-row mappings for cursor movement.** Stop rebuilding the selected file's visible-line list for every movement. Share the cached mapping with the renderer and invalidate it on snapshot or fold changes. Verify folded navigation, split/unified views, and large-file scrolling retain their behavior. Entry points: `src/app/motion.rs`, `src/app/staging.rs`.
- [ ] **P7 — Reduce PR completion work on the UI thread.** Measure directive re-anchoring, outcome checks, and review-mark loading after a PR fetch. Prepare expensive results in the background and apply them only if their snapshot and directive versions still match. Preserve durable save acknowledgements before posting continues; never replace newer local edits with an older worker snapshot. Entry points: `src/app/pr.rs`, `src/app/comments.rs`.

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
| A4 ✅ | MCP resources, subscriptions, notifications | L | A3 |
| A5 ✅ | Agent sinks: Zellij, WezTerm, kitty | M | R3 |
| A6 ✅ | Docs incl. MCP client setup | S | A4, A5 |

### D — CI/CD step (spec: `specs/2026-10-12-cicd-integration-design.md`, draft)
| ID | Item | Effort | Depends on |
|---|---|---|---|
| D1 ✅ | `--rev` on `check`/`ci` (reuses `compute_revision_diff`); exit codes 0 pass / 1 blocking findings / 2 tool error | S | B |
| D2 ✅ | Composite GitHub Action: verified install, gate, SARIF upload, job summary | M | D1 |
| D3 ✅ | CI recipes (GitLab, Azure DevOps), pre-commit hook, WIKI section | S | D1 |
| D4 ✅ | Self-test workflow against fixture PR branches | S | D2 |

### E — AST layer, web (React/Next.js) and Swift packs (spec: `specs/2026-10-12-ast-web-swift-design.md`, draft)
| ID | Item | Effort | Depends on |
|---|---|---|---|
| E1 ✅ | `lgtm-ast` crate (tree-sitter), TS/TSX, diff mapping, one rule end to end | L | B |
| E2 ✅ | `web/` + `next/` guard pack (6 rules; `"use client"` leak and server-action validation deferred, need whole-file parse) | M | E1 |
| E3 ✅ | Swift grammar + `swift/` pack | M | E1 |
| E4 ✅ | Complexity and symbol guards on AST for TS/JS/Swift. Test-neutering stays line-based: its rules are one-line patterns, so the AST adds little | M | E1 |
| E5 ❌ | TUI folding on AST. Dropped: fold state is per hunk and cursor motion indexes hunk lines, so in-hunk blocks would touch every motion path for a feature unverifiable without interactive use. Revisit on demand | S | E1 |
| E6 ✅ | Whole-file parse mode in `lgtm-ast`; `"use client"` leak and server-action validation rules. Diff hunks only select which findings to keep | M | E2 |
| E7 ✅ | Python grammar: AST complexity and identifiers, a few rules (`eval`/`exec`, `subprocess` with `shell`, `pickle.loads`, bare `except`) | M | E1 |
| E8 ✅ | Go, then Java grammars: complexity and identifiers first, rules on demand | M | E7 |
| E5′ ✅ | Block-level folding (revisit E5): fold state keyed by AST block range inside a hunk; needs a visible-rows indirection for cursor motion. Do it after F1 so folds can persist in the review store | M | E1, F1 |

### G — Azure DevOps pull requests (spec: `specs/2026-10-13-azure-devops-design.md`, draft)
| ID | Item | Effort | Depends on |
|---|---|---|---|
| G1 ✅ | `forge` module: `Forge` trait, `origin` URL detection, GitHub moved onto it unchanged; Azure remotes get a clear "not supported yet" instead of a `gh` failure | M | C4 |
| G2 ✅ | Azure read path: `:prs`, `:pr <n>` (diff from git ref), policies as checks | M | G1 |
| G3 ✅ | Azure threads read (`az devops invoke`) + inline markers (shared with F3) | M | G1, F3 |
| G4 ✅ | Azure post, reply, resolve, vote | M | G2 |
| G5 ✅ | `##vso` annotations, WIKI, Azure Pipelines recipe | S | G2, D3 |

### F — Review UX debt (comments and threads)
| ID | Item | Effort | Depends on |
|---|---|---|---|
| F1 ✅ | Re-anchor `comments.jsonl` comments on every reload (fuzzy code+context match, nearest line) and flag orphans via `orphaned`; storage stays `comments.jsonl` so directive ids and MCP stay stable. Comments on files outside the diff or on deleted lines are left alone | M | C1 |
| F2 ✅ | Orphan UI: `⚠` marker and `:directives orphans`, jump to the nearest hunk, re-pin or delete. Directive recheck re-anchors instead of only re-hashing | S | F1 |
| F3 ✅ | Markers `◆`/`◇` done with G3, `]t`/`[t` jump done; inline reply/resolve directly from marker with `r`/`R` (`:thread` covers it) | M | C4 |

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
| 4 A | v0.8.0 / v0.8.2 | A1–A6 done: Claim-vs-diff matrix tested; MCP clients (Claude Code, Cursor, Windsurf, Aider) documented; sinks smoke-checklist completed; existing MCP tools backward compatible |
| 5 E | v0.8.0 / v0.8.2 | E1–E4, E5′, E6–E8 done: web, Swift, Python, Go rules have positive/negative tests; whole-file parse for `"use client"` and server-action rules; block-level AST folding with visible-rows indirection and review store persistence |
| 6 D | v0.8.2 | D1–D4 done: `--rev`, exit codes, `action.yml`, WIKI recipes, self-test workflow (`.github/workflows/self-test.yml`) |

Unverified in the field (manual checks before release): all `az` calls (list, show, policy, invoke, set-vote), the Azure Pipelines annotations, the composite Action, `gh` calls (`checks`, GraphQL threads, REST post/reply/resolve), TUI parts (`M`, `✓n/m`, CI header, verdict tags), Homebrew tap and `cargo install --git`.

Branch protection / required status checks on trunk is a manual human step (not verifiable from the checkout); confirm before wave 1 starts.
