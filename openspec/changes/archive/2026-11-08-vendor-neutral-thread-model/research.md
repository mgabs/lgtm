# Research & Technical Tradeoffs

## 1. Intent & Ground Truth

Make the PR thread model vendor-neutral and add outdated, permission and resolution-status data, informed by how VS Code (`CommentThread.state`, `applicability`, `contextValue`; extensions map provider data into it).

Verified in the codebase:
- `Thread { id, path, line, resolved: bool, comments }` lives in `lgtm-core/src/github/mod.rs`, yet `forge/azure.rs`, `forge/mod.rs` and `discussion/mod.rs` import it from there. Many call sites (mostly tests) use `lgtm_core::github::Thread`.
- GitHub queries (`THREAD_QUERY`, `THREADS_QUERY`) fetch `isResolved` only; no `isOutdated`, `viewerCanResolve`, `viewerCanUnresolve`.
- Azure `thread_closed()` collapses `fixed | closed | wontFix | byDesign` to `bool`; `set_resolved(true)` always writes `"fixed"`.
- `PrDiscussion::set_resolved(pr, thread_id, bool)` is the single write path; unsupported capabilities already `bail!`.
- Drafts (`.lgtm/drafts/pr-<N>.json`) store resolve/unresolve as a bool intent.
- `lgtm_core` must stay free of I/O; mutation kill rate 100% on core logic.

Decision (supersedes the earlier display-only lean): the developer chose Option B, so resolution status is settable end to end (new write path, draft schema change, `:thread resolve [wontfix]`).

## 2. Options Explored

**A. Neutral model in `forge`, additive fields.** Move `Thread`/`ThreadComment` to `forge::model`; add `state: ThreadState`, `outdated: bool`, `can_resolve`/`can_unresolve: Option<bool>`. Keep a `github` re-export shim. `set_resolved(bool)` stays.
- Pros: small, drafts untouched, per-provider mapping isolated.
- Cons: bool write path cannot express WontFix (accepted, display-only).

**B. Neutral model plus `set_state(ThreadState)` write path (developer's choice).** As A, but replace `set_resolved` and extend drafts/commands.
- Pros: full round-trip of status.
- Cons: draft schema migration, new commands, larger blast radius; no current user demand.

**C. Keep `Thread` as is; add a provider-specific side struct.**
- Pros: zero churn.
- Cons: UI must branch per provider, defeats parity requirement.

## 3. Tradeoff Matrix

| Criteria | Option A | Option B | Option C |
| :--- | :--- | :--- | :--- |
| **Complexity** | Low-medium | High | Low, but leaks into UI |
| **New Dependencies** | None | None | None |
| **Performance & Latency** | Negligible (extra GraphQL fields) | Same | Same |
| **Maintenance Burden** | Low | Medium (draft migration) | High (per-provider branches) |
| **Blast Radius** | Thread call sites, 2 queries, Azure parser, thread view | A plus drafts, commands, MCP | Thread view only |

## 4. Recommendation

Developer chose Option B. Keep the YAGNI risk in check by shipping `Done` and `WontFix` only as settable resolutions; `Other(String)` stays read-only. A remained the leaner alternative.

## 5. Breaking Changes & Deprecations

- `comments` spec, "Live Thread Listing": MODIFIED (adds outdated badge and resolution status display).
- `comments` spec, "Draft Resolve and Unresolve": MODIFIED (drafts carry a target resolution; rejected when the thread is known not resolvable; last-draft-wins and idempotency unchanged).
- `comments` spec, "Persistent Draft Store": MODIFIED (draft schema gains a resolution; legacy bool drafts must still load).
- `comments` spec, "Thread Command and Key Surface": MODIFIED (`:thread resolve [wontfix]`).
- `comments` spec, "MCP and Watcher Source Live Threads": MODIFIED (new thread fields exposed).
- `PrDiscussion::set_resolved(bool)` superseded by `set_state(ThreadState)`.
- `comments` spec, "Provider Parity": MODIFIED (neutral mapping of provider status and outdated; unknown capabilities are `None`, not errors).
- Public path `lgtm_core::github::Thread` kept via re-export; no removal.
- No `REQ-*`/`NFR-*` identifiers exist in this spec; requirements are referenced by name.

## Review Gate
- [x] **Gate 0 Decision**: Human decides Go (name the option) or No-Go before the proposal is drafted.
