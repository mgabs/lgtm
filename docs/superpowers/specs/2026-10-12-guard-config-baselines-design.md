# Guard Configuration, Baselines and Suppressions — Design (Project B)

Date: 2026-10-12 · Status: draft for review · Baseline: `main` @ a90d5b0 (v0.7.0)
Depends on: modular-breakdown refactor stage 3 (`Guard` trait + registry). See `2026-10-12-modular-breakdown-design.md`.

## Goal

Make guard findings addressable, tunable and adoptable on existing codebases: stable rule IDs, a `.lgtm.toml` config, scoped suppressions, and baselines so `check`/`ci` fail only on new findings.

## Decisions (from brainstorming)

1. Scope: config + baselines/suppressions only. Custom regex rules and tree-sitter/more languages are separate later specs.
2. Baseline identity: fingerprint of rule ID + file + whitespace-normalized flagged line text (no line number), with an occurrence index for duplicate lines.
3. Config locations: repo-root `.lgtm.toml` and user-level `~/.config/lgtm/config.toml`. Precedence CLI > repo > user > defaults. Tables merge key-by-key; lists replace. `ci` ignores the user file by default; `--no-user-config` / `LGTM_NO_USER_CONFIG=1` ignores it everywhere.
4. Critical findings are never baselined unless `[baseline] allow_critical = true`.

## Current state (verified)

- `GuardAlert { severity, title, description, line_no, file_path }` in `src/guard/mod.rs` has no rule ID.
- Line-level suppression exists: `guard::is_line_ignored` accepts `lgtm:ignore`, `lgtm-ignore`, `nolint:lgtm`, `lgtm[ignore]`, `lgtm_ignore`.
- Thresholds are hard-coded (e.g. complexity increase >= 5). No config file exists.
- `check`/`ci` fail on Critical/High; `--strict` adds Warning (`src/main.rs`).

## Design

### 1. Rule IDs and finding model
- Add `rule_id` and `fingerprint` to `GuardAlert`. IDs are kebab-case with category prefix: `sec/sql-injection`, `sec/command-injection`, `sec/hardcoded-secret`, `sec/tls-bypass`, `test/neutered-assert`, `cx/complexity-spike`, `api/breaking-drift`, `dead/unused-function`, `dup/duplicate-block`, `typo/identifier`, `arch/layering`, `deps/*`. Each guard owns its IDs.
- `title`/`description` unchanged. JSON and SARIF gain `rule_id` (SARIF `ruleId`) and `partialFingerprints`.
- Severity overrides are applied after a guard emits an alert (config layer rewrites or drops with `off`). Thresholds are passed to guards via a typed `GuardSettings` with current values as defaults.

### 2. Config schema (v1)
Unknown keys warn, not error.

```toml
[thresholds]
complexity_increase = 5

[severity]            # rule ID or category glob -> critical|high|warning|info|off
"sec/tls-bypass" = "high"
"dead/*" = "off"

[ignore]
paths = ["vendor/**", "**/generated/**"]
rules = { "dup/duplicate-block" = ["tests/**"] }

[baseline]
file = ".lgtm-baseline.json"
allow_critical = false
```

`lgtm config --show` prints effective values with their sources.

### 3. Suppressions
- Existing `lgtm:ignore` variants keep suppressing all rules on the line.
- New: `lgtm:ignore(sec/tls-bypass, dead/*)` suppresses only listed rules.
- Suppressed findings are counted (`N suppressed`) in reports; `--show-suppressed` lists them.

### 4. Baseline
- `lgtm baseline update` writes `.lgtm-baseline.json`: sorted `{rule_id, file, fingerprint}` entries (deterministic, diff-friendly).
- `check`, `ci`, `scan` partition findings into new / existing / stale. Only new findings affect the exit code.
- Stale entries are reported; `lgtm baseline update --prune` removes them.

### 5. Integration
- New `config` module: `LgtmConfig::load(repo, flags)` merges, validates, records sources.
- New pure `findings` pipeline after guards: suppress (inline + path ignores) -> severity overrides -> baseline partition. No I/O.
- `run_ci_audit` and the full-repo scan call the pipeline. TUI shows suppressed/baselined findings dimmed and filterable; they never block.
- New dependency: `toml`.

### 6. Compatibility
With no config and no baseline, output and exit codes are byte-identical to today except for the added `rule_id` fields in JSON/SARIF. Enforced by golden tests.

## Testing
- Unit: glob matching, merge precedence, fingerprint stability (moves stable; edits change; duplicates distinct), Critical-not-baselined default.
- Golden: `check`, `scan`, `ci` output with and without config.
- Must hold coverage >= 90% and mutation >= 90%.

## Delivery (one PR each, from trunk, merged before the next)
1. Rule IDs in model and reports.
2. Config loader and `config --show`.
3. Pipeline: severity overrides, path ignores, scoped suppressions.
4. Baseline commands and partitioning.
5. TUI display and docs (`docs/WIKI.md`, `CONTRIBUTING.md`).

## Out of scope
Custom rules, tree-sitter/more languages, per-directory config overrides.

## Risks
| Risk | Mitigation |
|---|---|
| Fingerprint collisions or churn | Normalized text + occurrence index; stability tests |
| Local vs CI divergence from user config | `ci` ignores user file; `config --show`; `--no-user-config` |
| Baseline hides real issues | Critical never baselined by default; stale reporting; counts always shown |
| Output regressions | Golden tests before changes |
| Mutation gate path breakage when files move | Follow the refactor spec's path-update rule |
