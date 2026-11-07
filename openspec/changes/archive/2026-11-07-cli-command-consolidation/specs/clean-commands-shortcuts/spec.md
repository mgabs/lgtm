# Spec Delta

## MODIFIED Requirements

### Requirement: Canonical CLI Commands
The CLI SHALL expose only these canonical commands: `lgtm [REVISION]` (interactive diff TUI, default entrypoint), `check` (headless diff audit), `audit` (interactive repository issues TUI), `scan` (headless full-repo security & architecture report), `pr` (`[NUMBER]` and `list [--state]`), `review` (`export`/`import`), `mcp` (stdio Model Context Protocol server), `gate` (agent approval barrier), `hook` (git hook manager), `config` (configuration inspector), `init` (configuration initializer), and `baseline` (findings baseline manager).

#### Scenario: Default entrypoint
- **WHEN** the user runs `lgtm` or `lgtm HEAD~1`
- **THEN** the interactive diff reviewer launches on the working tree or that revision

#### Scenario: Headless check
- **WHEN** the user runs `lgtm check --format sarif --output r.sarif`
- **THEN** a SARIF report is written and the exit code reflects the gate result

#### Scenario: Review command requires subcommand
- **WHEN** the user runs `lgtm review` without `export` or `import`
- **THEN** the CLI exits with an argument error and shows command usage

### Requirement: REQ-CLI-1 Initialization Repository Target
The `init` subcommand MUST initialize the repository selected by `--repo` when supplied, and MUST initialize the current working directory when `--repo` is omitted. Top-level `--init` flag SHALL NOT be accepted.

#### Scenario: Initialize selected repository
- **WHEN** the user runs `lgtm init --repo <path>`
- **THEN** initialization targets `<path>` and writes starter configuration there

#### Scenario: Initialize default repository
- **WHEN** the user runs `lgtm init` without `--repo`
- **THEN** initialization targets the current directory

#### Scenario: Top-level init flag rejected
- **WHEN** the user runs `lgtm --init`
- **THEN** argument parsing fails with an unrecognized option error

## REMOVED Requirements

### Requirement: Deprecated CLI Aliases
- **Reason**: Deprecated aliases (`serve`, `prs`, `status`, `ci`) have completed their deprecation window and are removed to streamline `--help`, remove dead code paths, and prevent confusion.
- **Migration**:
  - `lgtm serve --mcp` -> `lgtm mcp`
  - `lgtm prs` -> `lgtm pr list`
  - `lgtm status` -> `lgtm` or `lgtm [REVISION]`
  - `lgtm ci` -> `lgtm check --no-user-config`
  - `lgtm risk` -> `lgtm check`

## Review Gate
- [x] **Gate 2 Approval**: Specifications reviewed and approved by human before technical design.
