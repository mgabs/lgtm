## Context

Current config has `context` plus `rules.tasks`. OpenSpec injects `rules.<artifact>` into the matching artifact's instructions as constraints (never copied into output).

## Goals / Non-Goals

**Goals:** consistent, project-aware proposals, designs, specs and tasks.
**Non-Goals:** custom schema, stores, regenerating `.github/` prompts or skills, changing the default `spec-driven` schema.

## Decisions

- Use per-artifact `rules` rather than a custom schema: no template maintenance, reversible by deleting lines.
- Keep rules short and checkable; the long-form policy stays in `context`.
- Strict validation limits (scenario `####`, ≤ 500-char requirement text) are encoded in the specs rule because they have already caused warnings here.

## Risks / Trade-offs

- Over-prescriptive rules add noise to small changes → phrase conditionally ("when relevant").
- Rules are only guidance for the agent; `openspec validate --strict` remains the enforcement.
