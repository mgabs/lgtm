# Research & Technical Tradeoffs

## 1. Intent & Ground Truth

Create a separate test-coverage feature for the existing Markdown parser rather than expanding the navigation optimization's scope. The user requested committing the navigation work and creating this coverage feature; parser implementation is not authorized by this planning artifact.

- Focused Markdown tests passed 29 cases after navigation optimization.
- `src/app/markdown.rs` measured 97.43% line coverage and 82% branch coverage (123/150) in focused nightly measurements.
- Changed navigation source branch locations were fully exercised; raw coverage exports also contain unexecuted duplicate compiled instances. Deduplicate only with documented source-location evidence, never by hiding uncovered outcomes.
- The module also implements code fences, rules, headings, quotes, lists, inline formatting, and wrapping. Existing parser branches are outside the prior navigation slice.
- Stable Rust cannot enable the required unstable branch instrumentation; a preinstalled nightly was used. Reproducible branch evidence must specify the toolchain, target, command, and aggregation policy.
- Navigation mutations were 24/24 caught. Parser mutation resistance remains unmeasured.
- Existing CI enforces 90% workspace line coverage. This feature targets >=95% line and branch coverage for the Markdown module without lowering that gate.

## 2. Options Explored

### Option A: Focused parser regression tests

Inspect uncovered source branch outcomes, then add small representative Markdown inputs with assertions on rendered content, styling, wrapping, and source attribution. Keep production behavior unchanged unless a test demonstrates a genuine bug requiring separately reviewed scope.

### Option B: Extract the parser into another module

Separates coverage accounting but changes structure without improving tests. More churn and risk than necessary; not recommended solely to meet a metric.

### Option C: Reduce the coverage target or exclude branches

Easy but masks gaps. Reject lowering thresholds or excluding real source outcomes; duplicate compiled instances need transparent accounting rather than blanket exclusions.

## 3. Tradeoff Matrix

| Criteria | A: Focused tests | B: Extract parser | C: Reduce/exclude |
| :--- | :--- | :--- | :--- |
| Complexity | Low/moderate | Moderate | Low |
| New Dependencies | None intended | None | None |
| Performance & Latency | No runtime change | Refactor risk | No benefit |
| Maintenance Burden | Behavioral regression protection | Structural churn | Hidden gaps |
| Blast Radius | Existing Markdown tests | Parser and callers | Weakened validation |

## 4. Recommendation

Choose Option A: add behavior-focused tests for verified uncovered parser branches and measure the complete Markdown module at >=95% line and branch coverage. Run scoped parser mutations and strengthen meaningful assertions against survivors without weakening existing budgets or assertions.

## 5. Breaking Changes & Deprecations

None. Existing Markdown preview behavior and performance requirements remain unchanged; future delta requirements should describe coverage evidence, not redefine parsing semantics. CI performance measurement, SARIF, and annotation isolation remain in `ci-performance-and-reporting`.

## Review Gate

- [x] **Gate 0 Decision**: Go with Option A, requested by the user on 2026-11-02.
