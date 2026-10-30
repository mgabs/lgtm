# Feature Proposal: Clean Commands and Shortcuts

## Intent
Systematically clean up, consolidate, and align LGTM's command and shortcut surfaces across the CLI, TUI Ex commands (`:`), keyboard shortcut handlers, and embedded documentation modals into a unified, predictable, and zero-conflict interface.

## Scope

### In-Scope
- **CLI Subcommand Consolidation**:
  - Eliminate duplicate CLI subcommands (`lgtm check` vs `lgtm ci`, `lgtm scan` vs `lgtm audit`, `lgtm prs` vs `lgtm pr list`, `lgtm diff` vs `lgtm status` vs `lgtm review`, `lgtm mcp` vs `lgtm serve --mcp`).
  - Formalize canonical commands, aliases, and deprecation warnings with forward-compatible migration paths.
- **TUI Ex Command (`:`) Consolidation**:
  - Establish a single source of truth for command metadata, aliases, arguments, and execution.
  - Group commands into 5 clear categories: Navigation & Layout, Directives & Threads, Agent Steering, PR & Remote Lifecycle, and Audit & Git Lifecycle.
  - Eliminate confusing collisions (specifically `:r` batch review dispatch vs `r` normal mode reply).
  - Unify Wildmenu completion (`src/app/state.rs`) with the command dispatcher (`src/app/command.rs`), replacing detached hardcoded string tables.
- **TUI Keyboard Shortcut Reconciliation**:
  - Fix the shadowed/dead shortcut bug in `src/app/input.rs` where `KeyCode::Char('X')` at line 264 blocks line 405 (`discard_current_file_changes`).
  - Restore `X` to discard current file changes (symmetric with `S` stage file and `U` unstage file).
  - Rebind agent hunk rejection (`reject_hunk_and_reprompt`) to an unambiguous chord (`<Space>x` and `:reject`).
  - Disambiguate `r` (reply), `R` (resolve), and `<Space>s` (send review to agent).
  - Reconcile `m` (markdown preview / audit maximize) and `M` (review hunk mark).
- **Help Modal (`src/ui/help.rs`) Synchronization**:
  - Update all 7 documentation tabs to reflect 100% canonical commands and keybindings.
  - Add test harness to verify help modal documentation does not drift from command handlers.

### Out-of-Scope (YAGNI)
- Dynamic runtime key remapping engine (e.g. Vim `.vimrc` / `:map` or customizable TOML keymaps). Built-in canonical shortcuts are sufficient.
- Emacs keybinding mode or alternate modal keymaps.
- Multi-column wildmenu popover widget with interactive cursor selection (horizontal completion cycling is sufficient).
- Breaking existing git passthrough functionality.

## Problem Statement
Over successive releases, LGTM accumulated redundant subcommands, uncoordinated Ex commands, and conflicting keyboard shortcuts:
1. **CLI Redundancies**: `lgtm check` and `lgtm ci` execute identical headless CI routines with differing flags. `lgtm scan --tui` and `lgtm audit` launch the same audit TUI. `lgtm prs` exists as a separate top-level command alongside `lgtm pr`. `lgtm diff`, `lgtm status`, and parameterless `lgtm review` duplicate the default TUI launcher. `lgtm mcp` and `lgtm serve --mcp` do the same job.
2. **Shadowed & Dead Keyboard Shortcuts**: In `src/app/input.rs`, line 264 unconditionally intercepts `KeyCode::Char('X')` for `reject_hunk_and_reprompt()`. As a result, line 405 (`discard_current_file_changes`) is completely unreachable. Reviewers cannot discard whole-file modifications via keyboard, breaking symmetry with `S` (stage file) and `U` (unstage file).
3. **Mismatched Normal Keys vs Ex Commands**: Normal key `r` replies to a comment/thread, but `:r` / `:review` sends batch reviews to the multiplexer agent pane. Users attempting to reload the diff or reply via Ex mode trigger an unintended agent batch send.
4. **Detached Wildmenu Autocompletion**: `src/app/state.rs` maintains a hardcoded array of 58 string literals for Tab completion. It is disconnected from `src/app/command.rs`, causing newly introduced commands to be missing from autocomplete and deprecated commands to persist.
5. **Help Modal Inconsistencies**: `src/ui/help.rs` documents 7 tabs with contradictory bindings (e.g. Tab 3 lists `X` as `:reject`, while Tab 4 lists `X` as `git restore`).

## Goals, Non-Goals, and Anti-Goals

### Goals
- **Single Source of Truth**: One command registry drives CLI aliases, TUI Ex execution, Wildmenu completion, and Help Modal references.
- **Zero Shadowed Keys**: Every keyboard binding in `src/app/input.rs` is reachable and performs its documented action.
- **Symmetric Git Staging**: Single keys operate on hunks (`s`, `u`, `x`); uppercase keys operate on whole files (`S`, `U`, `X`).
- **Consistent Mental Model**: Navigation, directives, agent steering, and remote PR actions follow uniform conventions across normal keys, leader chords, and Ex commands.
- **100% Documentation Accuracy**: All 7 tabs in `src/ui/help.rs` match actual runtime behavior.

### Non-Goals
- Adding customizable user keybinding configs in `.lgtm.toml`.
- Changing Vim core motions (`j`, `k`, `h`, `l`, `gg`, `G`, `Ctrl-d`, `Ctrl-u`).
- Altering the wire format of `.lgtm/comments.jsonl` or `.lgtm/gate.json`.

### Anti-Goals
- Never silently break existing user scripts: deprecated CLI commands must continue to function with a warning notice for at least one release cycle.
- Never introduce ambiguous multi-character key timeouts for single-key actions.

## Success Metrics
- **0 Unreachable Key Handlers**: Compiler and static analysis confirm no dead branches in `handle_diff_key`.
- **0 Discrepancies in Autocomplete**: 100% of callable TUI Ex commands appear in Wildmenu completion.
- **100% Sync with Help Docs**: Every documented shortcut in `src/ui/help.rs` matches an active handler in `input.rs` or `command.rs`.
- **Reduced CLI Surface**: Top-level CLI command surface reduced from 15 commands to 9 canonical commands without loss of functionality.
