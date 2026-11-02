## ADDED Requirements

### Requirement: Mermaid Blocks
Fenced `mermaid` blocks SHALL show a header `[mermaid: <type>]` taken from the first word of the first non-empty line. In `graph` and `flowchart` blocks each edge MUST render as `<from> ──▶ <to>` with its label and node labels. Anything unparseable, and other diagram types, MUST render verbatim. Rows MUST keep their source line number and diff overlay.

#### Scenario: Flowchart edges
- **WHEN** a block contains `flowchart TD`, `A[Start] --> B{Ok?}` and `B -->|yes| C[Done]`
- **THEN** the header reads `[mermaid: flowchart]` and rows read `Start ──▶ Ok?` and `B ──▶ Done (yes)`

#### Scenario: Unsupported diagram type
- **WHEN** a block starts with `sequenceDiagram`
- **THEN** the header reads `[mermaid: sequenceDiagram]` and the body renders verbatim as code

#### Scenario: Unparseable line
- **WHEN** a flowchart block contains `style A fill:#f9f`
- **THEN** that line renders verbatim and the other edges still render

#### Scenario: Changed edge
- **WHEN** a diagram edge line is added in the diff
- **THEN** its rendered row is green with a `+` marker and that line's number
