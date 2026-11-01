# Research & Technical Tradeoffs

## 1. Intent & Ground Truth

Make CI failures reflect real regressions rather than instrumentation overhead or synthetic scanner output. This research continues the existing change on branch `fix/ci-performance-budget`; no implementation is included.

Verified evidence:

- Run `41239075222` on commit `fa124c6` failed the Markdown scrolling timing assertion at `src/app/markdown.rs:955`: Ubuntu coverage measured 1,284 ns and Windows unit tests measured 1,288 ns against a 1,000 ns limit.
- `.github/workflows/ci.yml` runs ordinary debug tests, then Ubuntu coverage with `cargo llvm-cov --workspace --all-features --fail-under-lines 90 --summary-only`. Exit 101 came from a test failure; no below-threshold coverage result was established.
- The timing test builds 5,000 lines and averages 1,000 scroll operations. Its state has empty `row_heights`, whereas `src/ui/markdown.rs` populates those heights during rendering, including for documents without directives. Optimized measurements need observable results so the compiler cannot remove the measured work.
- `last_visible_row` scans viewport rows. A uniform-height fast path could reduce navigation cost, but the renderer currently supplies populated heights; treating only an empty vector as uniform would not cover normal rendering.
- SARIF generation is skipped after the failed coverage step, while upload uses `always()`. Upload then fails because `lgtm-results.sarif` does not exist.
- `src/headless.rs` tests deliberately scan `b.py` fixtures containing synthetic token strings. Explicit GitHub format and environment-based detection emit workflow annotations, exposed by `--nocapture`. These annotations do not establish a real credential leak.
- Node.js and Ubuntu migration notices are not the causes of the failed checks.

Unresolved acceptance details: repeated optimized measurements must establish runner variability; the existing 90% coverage threshold may expose a separate failure once the timing test is measured appropriately.

## 2. Options Explored

### Option A: Relax timing thresholds or suppress failing checks

Smallest edit, but conflates runtime performance with debug and instrumentation cost. Weakening assertions or disabling coverage would hide regressions and is rejected.

### Option B: Separate measurement from behavioral coverage

Retain behavioral tests under debug and coverage, and enforce performance budgets through an explicit optimized, non-instrumented CI check. Benchmark uniform and directive-expanded row heights with observable state, warm-up, and repeated measurements. Guard SARIF upload on report availability and avoid emitting synthetic annotations from successful tests.

This gives clearer failure attribution but requires ensuring the benchmark really runs in CI and is not optimized away. Removing `--nocapture` alone does not prevent annotations from failing tests or subprocesses; annotation-specific tests may need captured output boundaries.

### Option C: Optimize scrolling and leave the current timing gate unchanged

Investigate constant-time uniform-row navigation and bounded variable-height handling while preserving cursor and directive semantics. This addresses genuine costs, but cannot make a strict debug/coverage wall-clock assertion portable by itself.

## 3. Tradeoff Matrix

| Criteria | A: Relax checks | B: Separate measurement | C: Optimize only |
| :--- | :--- | :--- | :--- |
| Complexity | Low | Moderate | Moderate |
| New Dependencies | None | None required | None required |
| Performance & Latency | No runtime benefit | Meaningful optimized measurements | Potential navigation benefit |
| Maintenance Burden | Hidden regressions | Explicit benchmark and output contracts | Timing gate remains environment-sensitive |
| Blast Radius | Weakens CI guarantees | Localized tests and workflow | Localized runtime code, with behavior risk |

## 4. Recommendation

Selected approach: separate optimized, non-instrumented performance measurement from behavioral coverage and optimize scrolling. Cover populated uniform-height and directive-expanded rows while preserving cursor visibility, scroll clamping, existing latency budgets, and allocation guarantees. Preserve coverage enforcement, keep real scanner failures visible, and gate SARIF upload on an actually generated report.

The user selected this combined approach and approved recording it in this research artifact. This approval does not authorize implementation or approve later planning artifacts.

## 5. Breaking Changes & Deprecations

No command or data-format deprecations are proposed. The existing unkeyed `Preview Performance Budgets` requirement in `openspec/specs/markdown-preview/spec.md` may need a clarified measurement contract; its rendering, parsing, allocation, and memory guarantees must remain intact.

The `NFR-MDP-02` and `NFR-MDP-03` labels appear in test comments, not as stable requirement headers in that main spec. New CI measurement and reporting requirements should use distinct stable keys without silently renaming existing requirements.

## Review Gate

- [ ] **Gate 0 Decision**: Human decides Go (Option B, with measurement-led optimization) or No-Go before the proposal is drafted.
