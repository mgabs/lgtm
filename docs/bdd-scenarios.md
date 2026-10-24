# Behavior scenario catalog

This catalog links expected behavior to executable tests. It is a starting set of
acceptance criteria, not an inventory of every LGTM feature. Stable IDs identify
scenarios as their test names or implementation change.

Public API scenarios live in `crates/lgtm-core/tests/bdd.rs`. Application scenarios
live in `tests/bdd.rs` and launch the compiled binary. Given/When/Then comments
explain the acceptance criteria; assertions check the observable outcome.

## Acceptance scenarios

All scenarios below are offline. The GitHub and Azure runners return fixture
responses and record requests; posting uses an in-memory client. Persistence
checks use real files in isolated temporary repositories. `bdd_cli_006` launches
the compiled binary inside a real Unix pseudo-terminal (PTY) against local fake
provider commands. None of these tests verifies a live provider or machine-speed
latency thresholds. Core scenarios that "reopen" state create a new store and
reload its real files in the same test process; they do not simulate an
application crash or operating-system restart.

| ID | Given / When | Expected outcome |
|---|---|---|
| `bdd_core_001` | A custom hook exists; install LGTM twice, then remove it | No duplicate gate; custom hook preserved; LGTM status absent |
| `bdd_core_002` | A saved PR review is reopened after code moves or changes | Moved content stays reviewed; changed content requires review; another PR inherits no marks |
| `bdd_core_003` | A historical GitHub PR's merge ref is missing; load its diff | Stored patch returned after one ref attempt; no guessed branch fetch |
| `bdd_core_004` | GitHub PR fetch fails authentication | Error returned after one request; no patch or transport fallback |
| `bdd_core_005` | First post succeeds, next fails; reopen saved directives and retry | First receipt survives reopening; only the remaining directive is posted |
| `bdd_core_006` | First remote post succeeds but its receipt cannot be saved | Save failure reported; second remote write never starts |
| `bdd_core_007` | Saved directives include posted, resolved, other-PR and local-only entries | Only the current PR's unresolved, unposted directive is sent; excluded entries remain unchanged |
| `bdd_core_008` | Equivalent GitHub and Azure thread payloads have open or completed statuses | Normalized discussion text, location and resolution markers agree |
| `bdd_core_009` | Saved review uses an unsupported future format version | Load reports an upgrade is needed; original data stays untouched |
| `bdd_core_010` | GitHub and Azure listing payloads include open, merged and closed reviews | Normalized open/closed/all filters select equivalent entries |
| `bdd_core_011` | No API token; `gh` needs login or is missing | One anonymous GET lists public open/closed/all PRs without credentials |
| `bdd_core_012` | Missing merge ref and unauthenticated `gh` | Historical public patch loads through anonymous HTTPS |
| `bdd_core_013` | Anonymous listing gets 404, 403, or 429 | Status is retained with authentication/rate-limit guidance; no retry |
| `bdd_core_014` | CLI fails for network, timeout, access denial, or another reason | Original error returned without anonymous fallback |
| `bdd_core_015` | Enterprise, SSH-alias, or lookalike-host origin; CLI needs login | No request is made to GitHub.com's public API |
| `bdd_core_016` | API token is rejected | Reads and writes fail without CLI retry or anonymous downgrade |
| `bdd_core_017` | CLI already returns the listing | CLI result is used without an extra anonymous request |
| `bdd_core_018` | Anonymous listing response is malformed or not an array | Error reported without retry or an empty successful listing |
| `bdd_core_019` | GitHub (API/CLI) and Azure (`az`) list open, closed, and all PRs | Request targets, state flags, and normalized summaries match provider/transport contracts |
| `bdd_core_020` | GitHub (API/CLI) and Azure (`az`) reply, resolve, and reopen threads | Request endpoints, payloads, numeric ID guards, and confirmation checks reject unconfirmed responses |
| `bdd_core_021` | Remote directive post times out after provider accepts the comment | Directive is persisted as uncertain, skipped on blind retry, and reconciled against remote threads without duplicate write |
| `bdd_core_022` | Local receipt save fails after remote post accepts comment, followed by process restart | Fresh process recovers remote id via thread reconciliation, avoids duplicate post, and completes remaining directives |
| `bdd_cli_001` | User installs, inspects, and removes the hook | Successful exit codes, matching status, actual hook created and removed |
| `bdd_cli_002` | User supplies `--debug-log` without `--debug` | Argument error and no log file |
| `bdd_cli_003` | `LGTM_DEBUG_LOG` is set; run normally, then with `--debug --debug-log PATH` | Environment alone creates no log; explicit flags create the requested file |
| `bdd_cli_004` | Install a hook outside a Git repository | Nonzero exit, clear error, no created files |
| `bdd_cli_005` | Request an unsupported PR listing state | Argument error shows valid choices before the picker is launched |
| `bdd_cli_006` | Real-terminal (PTY) session navigates while loading, switches PRs, retains visual selection and draft, and rejects stale PR load | Stale PR #10 is discarded, visual selection and comment draft defer PR #20 apply, directive persists on PR #20 without extra fetch |
| `bdd_cli_007` | Large review (1,000 changed lines, 50 directives) in real PTY under navigation load | Responsiveness percentiles (p50, p90, p95) are measured and reported without machine-speed assertions |

Run all of these with:

```sh
cargo test --workspace --test bdd
```

Filter by a stable ID to run one scenario, for example:

```sh
cargo test -p lgtm-core --test bdd bdd_core_005
```

## Related internal behavior tests

These remain module tests because they exercise private application state or
provider internals. They run under `cargo test --workspace`, but are not included
by `--test bdd`.

| Catalog ID | Acceptance criterion | Existing test and location |
|---|---|---|
| `bdd_ui_001` | Completed PR preparation waits while a draft is open | `preparation_runs_off_thread_and_does_not_replace_an_open_draft`, `src/app/pr/completion/tests.rs` |
| `bdd_ui_002` | New edits, replies, and receipts survive preparation without refetching | `edits_and_post_receipts_during_loading_survive_without_refetch`, `src/app/pr/completion/tests.rs` |
| `bdd_ui_003` | An older request cannot replace the latest selection; failure preserves the current review | `newest_load_wins_and_failure_preserves_current_review`, `src/app/pr.rs` |
| `bdd_ui_004` | Failed remote reply retains its draft | `failed_remote_reply_retains_its_draft`, `src/app/pr.rs` |
| `bdd_ui_005` | Normal quit waits for remote posting and receipt persistence | `normal_quit_waits_for_successful_post_receipt_to_be_saved`, `src/app/pr.rs` |
| `bdd_ui_006` | Successful reply survives a failed thread refresh without resubmission | `bdd_ui_006_successful_reply_survives_failed_refresh_without_resubmission`, `src/app/pr.rs` |
| `bdd_ui_007` | Acknowledging an older reply preserves a newer unsent draft | `bdd_ui_007_acknowledged_reply_preserves_a_newer_draft`, `src/app/pr.rs` |
| `bdd_ui_008` | Failed resolution can be retried, then reopened | `bdd_ui_008_failed_resolution_can_be_retried_and_reopened`, `src/app/pr.rs` |
| `bdd_ui_009` | Ambiguous remote thread reply timeout blocks blind resubmission until thread refresh reconciles delivery | `bdd_ui_009_ambiguous_reply_timeout_blocks_blind_resubmission_until_reconciled`, `src/app/pr.rs` |
| `bdd_ui_010` | Failed receipt save recovers after restart via remote thread reconciliation | `bdd_ui_010_restart_after_failed_receipt_save_reconciles_against_remote_threads_without_duplicate_post`, `src/app/pr.rs` |
| `bdd_provider_001` | Azure historical reloads use recorded commits without repeated ref probes | `historical_pr_cold_and_reload_request_budgets`, `crates/lgtm-core/src/forge/azure/performance_tests.rs` |
| `bdd_provider_002` | GitHub stored-patch retrieval keeps a bounded request count | `historical_api_patch_request_budget` and `historical_cli_patch_request_budget`, `crates/lgtm-core/src/forge/github/performance_tests.rs` |

Run a mapped internal test by its test name, for example:

```sh
cargo test --bin lgtm preparation_runs_off_thread_and_does_not_replace_an_open_draft
cargo test -p lgtm-core historical_pr_cold_and_reload_request_budgets
```

## Automated reporting and coverage metrics

Run the automated BDD catalog report:

```sh
make bdd-report
# or ./scripts/bdd-report.sh
```

The script maps all 41 agreed catalog criteria to executable test implementations,
evaluates test execution, and reports:
- **Automation coverage**: automated criteria divided by agreed catalog criteria (100.0%).
- **Verified coverage**: passing criteria in the current run divided by agreed criteria (100.0%).
- **Pass rate**: passing criteria divided by executed criteria (100.0%).

## Acceptance milestones (Q0–Q6)

The [roadmap's Q0–Q6 checklist](superpowers/roadmap.md#q--bdd-acceptance-gaps-added-2026-10-23)
is complete, establishing provider request contracts (Q0), ambiguous remote write
recovery (Q1), real-terminal journeys (Q2), receipt failure recovery across restarts (Q3),
automated BDD catalog reporting (Q4), opt-in live-provider verification (Q5), and real-terminal
load responsiveness percentiles (Q6).
