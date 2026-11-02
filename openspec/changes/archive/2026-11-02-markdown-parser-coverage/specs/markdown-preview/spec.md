# Spec Delta

## ADDED Requirements

### Requirement: REQ-MDCOV-1 Parser Regression Coverage
The Markdown parser tests MUST exercise headings, rules, block quotes, ordered and unordered lists, fenced code, Mermaid fences, inline emphasis and code, and width wrapping, asserting rendered text, styling and source-line attribution rather than only that rendering does not panic.

#### Scenario: Structure variants
- **WHEN** documents mixing each construct above are rendered at narrow and wide widths
- **THEN** each row's text, style and source line number match the expected values

#### Scenario: Malformed and edge input
- **WHEN** input has an unclosed fence, empty emphasis markers, an empty list item or a line wider than the viewport
- **THEN** the parser renders it without panic and with the documented verbatim or wrapped output

### Requirement: NFR-MDCOV-1 Parser Coverage Bar
`src/app/markdown.rs` MUST measure at least 95% line coverage and at least 95% branch coverage using `cargo +nightly llvm-cov -p lgtm --bin lgtm --branch`, counting each source branch location once and never excluding real outcomes. Scoped parser mutation testing MUST catch every non-equivalent mutant.

#### Scenario: Coverage measured
- **WHEN** the documented command runs on the final tree
- **THEN** the report shows at least 95% line and at least 95% branch coverage for `src/app/markdown.rs`

#### Scenario: Mutation survivors
- **WHEN** scoped mutation testing runs on the parser
- **THEN** no non-equivalent mutant survives, and each equivalent mutant is listed with its reason

## Review Gate
- [x] **Gate 2 Approval**: Specifications reviewed and approved by human before technical design, approved by the user on 2026-11-02.
