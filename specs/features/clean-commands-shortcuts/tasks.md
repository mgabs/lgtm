# Implementation Plan: Clean Commands and Shortcuts

> **Goal:** Eliminate overlapping CLI subcommands, resolve shadowed/dead keyboard shortcuts, establish a unified Command Registry for Wildmenu completion and execution, and synchronize the 7-tab Help Modal.

**Architecture:** [`design.md`](design.md)  
**Specification:** [`spec.md`](spec.md)

---

### Phase 1: CLI Command Consolidation & Deprecation Handling

- [ ] **Task 1.1: Add `lgtm pr list` Subcommand**
  - **File:** `src/cli.rs`, `src/main.rs`
  - Add `List { state: String }` variant to `PrSubcommand`.
  - In `src/main.rs`, route `PrSubcommand::List` to `lgtm_core::forge::list_prs_state`.
  - Mark top-level `Prs` command with deprecation notice redirecting to `pr list`.
  - *Acceptance Criteria:* `lgtm pr list --state open` lists pull requests; `lgtm prs` outputs deprecation notice on stderr and lists pull requests.

- [ ] **Task 1.2: Consolidate `lgtm check` and `lgtm ci`**
  - **File:** `src/cli.rs`, `src/main.rs`
  - In `src/main.rs`, add deprecation warning to `Commands::Ci`: `⚠️  Notice: 'lgtm ci' is deprecated. Use 'lgtm check --no-user-config'.`
  - Ensure `Commands::Ci` delegates directly to `run_check` with `no_user_config: true`.
  - *Acceptance Criteria:* `lgtm ci` prints notice to stderr and executes headless check with same exit codes as `check`.

- [ ] **Task 1.3: Align `lgtm scan` and `lgtm audit`**
  - **File:** `src/cli.rs`, `src/main.rs`
  - In `src/main.rs`, when `Commands::Scan` is passed `--tui` or target `"repo"`, print deprecation notice recommending `lgtm audit`.
  - Ensure `lgtm audit` remains the canonical command for launching interactive TUI audit mode.
  - *Acceptance Criteria:* `lgtm audit` launches full-repo audit TUI without warnings; `lgtm scan --tui` warns and launches TUI.

- [ ] **Task 1.4: Deprecate `lgtm status` and `lgtm serve --mcp`**
  - **File:** `src/cli.rs`, `src/main.rs`
  - Add deprecation warning to `Commands::Status`: `⚠️  Notice: 'lgtm status' is deprecated. Use 'lgtm' or 'lgtm diff'.`
  - Add deprecation warning to `Commands::Serve`: `⚠️  Notice: 'lgtm serve --mcp' is deprecated. Use 'lgtm mcp'.`
  - *Acceptance Criteria:* Existing invocations continue to work without breaking scripts while warning on stderr.

---

### Phase 2: Unified Command Registry & Ex Command Dispatcher

- [ ] **Task 2.1: Implement `CommandDef` and `COMMAND_REGISTRY`**
  - **File:** `src/app/command/mod.rs` (or `src/app/command/registry.rs`)
  - Define `CommandCategory` and `CommandDef` structs.
  - Define static `COMMAND_REGISTRY: &[CommandDef]` containing all 36 canonical Ex commands and their aliases.
  - Implement helper functions:
    - `find_command(name: &str) -> Option<&'static CommandDef>`
    - `autocomplete_candidates(prefix: &str) -> Vec<String>`
  - *Acceptance Criteria:* Helper functions return sorted, deduplicated canonical and alias matches.

- [ ] **Task 2.2: Refactor Wildmenu Autocompletion in `CommandLine`**
  - **File:** `src/app/state.rs`
  - Replace the hardcoded `commands` string array in `CommandLine::autocomplete_next` with `autocomplete_candidates`.
  - Verify Tab and Shift-Tab completion cycles through registry entries correctly.
  - *Acceptance Criteria:* Autocomplete includes all registry commands; no hardcoded command lists remain in `state.rs`.

- [ ] **Task 2.3: Reconcile Collision of `:r` in `execute_command`**
  - **File:** `src/app/command.rs`
  - Remove alias `"r"` from `export_and_send_batch_review`.
  - Establish `:send` as the canonical command (aliases: `:review`, `:push`).
  - Ensure `:reply` / `:rep` handles comment/thread replies.
  - Ensure `:reload` / `:edit` / `:e` reloads the diff from disk.
  - Update unknown command error feedback to reference `:help`.
  - *Acceptance Criteria:* `:send` dispatches review to agent; `:r` is no longer a silent batch send; `:reply` replies.

---

### Phase 3: Keyboard Shortcut Reconciliation & Bug Fixes

- [ ] **Task 3.1: Fix Shadowed `KeyCode::Char('X')` Bug**
  - **File:** `src/app/input.rs`
  - Remove lines 264–267 in `handle_diff_key`:
    ```rust
    (KeyModifiers::SHIFT, KeyCode::Char('X'))
    | (KeyModifiers::NONE, KeyCode::Char('X')) => {
        self.reject_hunk_and_reprompt();
    }
    ```
  - Verify line 405 (`(_, KeyCode::Char('X')) => self.discard_current_file_changes()`) is fully reachable.
  - In leader chord handler (`match (leader, key.code)`), bind `(' ', KeyCode::Char('x')) => self.reject_hunk_and_reprompt()`.
  - *Acceptance Criteria:* Pressing `X` in normal mode discards current file changes via `git restore`; pressing `<Space>x` prompts to reject hunk and re-prompt agent.

- [ ] **Task 3.2: Verify Symmetrical Staging Keymap**
  - **File:** `src/app/input.rs`
  - Verify normal mode mappings:
    - `s` -> `stage_current_hunk`
    - `S` -> `toggle_stage_current_file`
    - `u` -> `unstage_current_hunk`
    - `U` -> `unstage_current_file`
    - `x` -> `discard_current_hunk`
    - `X` -> `discard_current_file_changes`
  - *Acceptance Criteria:* Staging operations follow consistent single-key (hunk) vs shift-key (whole file) behavior.

- [ ] **Task 3.3: Disambiguate `m` and `M`**
  - **File:** `src/app/input.rs`
  - Verify `m` toggles markdown preview in diff view (`toggle_markdown_preview`) and maximizes inspector in audit view (`maximized_inspector`).
  - Verify `M` toggles review state on current hunk (`toggle_review_current_hunk`).
  - *Acceptance Criteria:* No modal ambiguity between markdown preview and hunk review marks.

---

### Phase 4: Help Modal & Quick Reference 7-Tab Synchronization

- [ ] **Task 4.1: Synchronize Tabs 1 to 4 in `src/ui/help.rs`**
  - **File:** `src/ui/help.rs`
  - Tab 1: Clean up mode descriptions and remove deprecated aliases.
  - Tab 2: Ensure all motions and bracket jumps (`]f`, `]h`, `]c`, `]t`, `]w`, `]u`) are accurately documented.
  - Tab 3: Update agent steering: document `<Space>s` (`:send`), `<Space>x` (`:reject`), `A` (`:approve`), `P` (`:prompt`), `Enter`.
  - Tab 4: Explicitly document the symmetric staging matrix (`s`/`S`, `u`/`U`, `x`/`X`, `C`/`:commit`).
  - *Acceptance Criteria:* Documentation accurately matches updated `input.rs` bindings.

- [ ] **Task 4.2: Synchronize Tabs 5 to 7 in `src/ui/help.rs`**
  - **File:** `src/ui/help.rs`
  - Tab 5: Verify fold commands (`zc`, `zo`, `za`, `zM`, `zR`).
  - Tab 6: Retain security guardrail alert descriptions and audit navigation.
  - Tab 7: Align all Ex commands with the canonical `COMMAND_REGISTRY`; update headless CI commands (`lgtm check`, `lgtm audit`, `lgtm scan`).
  - *Acceptance Criteria:* Tab 7 lists all canonical commands; no phantom commands exist.

---

### Phase 5: Verification, Automated Testing, and Regression Protection

- [ ] **Task 5.1: Unit Tests for Command Registry and Wildmenu Autocomplete**
  - **File:** `src/app/command/tests.rs` (or `src/app/command.rs`)
  - Test registry lookup for canonical names and aliases.
  - Test prefix filtering and deduplication in autocomplete.
  - *Acceptance Criteria:* 100% test pass rate for autocomplete candidate resolution.

- [ ] **Task 5.2: Unit Tests for Key Handlers (`X`, `<Space>x`, `r`, `R`)**
  - **File:** `src/app/input.rs`
  - Add test verifying `KeyCode::Char('X')` invokes `discard_current_file_changes`.
  - Add test verifying `<Space>x` invokes `reject_hunk_and_reprompt`.
  - *Acceptance Criteria:* Automated tests fail if `X` is shadowed or `<Space>x` is unbound.

- [ ] **Task 5.3: Automated Help Modal Sync Test**
  - **File:** `src/app/command.rs`
  - Add test asserting that every canonical command in `COMMAND_REGISTRY` is documented in `src/ui/help.rs`.
  - *Acceptance Criteria:* Test fails if a new command is added to the registry without updating the help modal.

- [ ] **Task 5.4: CLI Deprecation Tests**
  - **File:** `src/cli.rs`
  - Test `lgtm ci`, `lgtm scan --tui`, `lgtm prs`, `lgtm status`, `lgtm serve --mcp` trigger proper deprecation warnings.
  - *Acceptance Criteria:* CLI tests confirm backward compatibility and stderr warning emission.
