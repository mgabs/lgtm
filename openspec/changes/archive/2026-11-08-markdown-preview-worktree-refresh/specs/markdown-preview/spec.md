# Spec Delta

## ADDED Requirements

### Requirement: Preview Live Refresh and Cache Invalidation
When preview mode is active, incoming background diff updates SHALL apply immediately to the diff snapshot and file drawer. The rendered markdown preview MUST invalidate its cached lines and re-render against the updated file content, clamping the cursor row and scroll offset to the new valid row range.

#### Scenario: Background file modification during preview
- **WHEN** the reviewer views `README.md` in markdown preview
- **AND** external edits to `README.md` or other workspace files arrive via the file watcher
- **THEN** the local diff snapshot applies without exiting markdown preview
- **AND** the file drawer reflects updated file change statuses
- **AND** the preview re-renders updated markdown lines while keeping the cursor in view

#### Scenario: Background file deletion during preview
- **WHEN** the active markdown file is deleted on disk while preview is open
- **THEN** the preview updates to render deletions with red `-` gutter markers
- **AND** cursor position clamps to the new document boundary without panic

## Review Gate
- [ ] **Gate 2 Approval**: Specifications reviewed and approved by human before technical design.
