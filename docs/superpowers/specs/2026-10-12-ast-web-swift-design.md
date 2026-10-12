# AST layer and React / Next.js / Swift guard packs: design (draft)

Status: draft for review. No code written.

## Problem
Guards are regex and brace-counting over diff text. That is wrong for JSX nesting, Python indentation, multi-line constructs, and cannot express "a hook called conditionally" or "server action without validation". React, Next.js and Swift have no rules at all.

## Approaches
1. **tree-sitter in a new `lgtm-ast` crate (recommended).** One parser API, grammars for TS/TSX/JS, Swift, Rust, Python. Guards query syntax nodes instead of text. Incremental, fast, tolerant of broken code (agents write half-finished code).
2. **Per-language LSP/compilers (swiftc, tsc).** Accurate types, but heavy, slow, needs toolchains in CI. Reject for core; maybe optional later.
3. **Keep regex, add packs.** Cheapest, but the React/Swift rules that matter are structural, and false positives will erode trust.

## Design
- **Crate:** `lgtm-ast` (no ratatui/crossterm; add to `check-boundaries.sh`). API: `parse(lang, src) -> Tree`, `lang_of(path)`, `query(tree, q) -> Vec<Match{range, captures}>`.
- **Diff mapping:** parse the new-side file content (not just hunks), run queries, keep matches whose range intersects added lines. Reports only findings the change introduced; fingerprints stay line-independent (rule + normalized snippet) so baselines survive.
- **Fallback:** unknown language or parse failure keeps today's regex guards. A guard pack never reduces existing coverage.
- **Packs** are ordinary `Guard` impls registered by language, so config (`.lgtm.toml` rules, suppressions, baseline) applies unchanged.
  - **web/** (TS/JS/TSX): `web/dangerous-html` (`dangerouslySetInnerHTML`, `innerHTML`, `eval`), `web/hook-conditional` and `web/hook-deps` (hook in condition/loop; effect with missing deps array), `next/public-secret` (`NEXT_PUBLIC_` holding key/token/secret), `next/client-server-leak` (`"use client"` file importing server-only or reading non-public env), `next/action-unvalidated` (`"use server"` function or route handler using input without a schema parse or auth check), `web/ts-escape` (`any`, `@ts-ignore`, `as unknown as` added in the diff).
  - **swift/**: `swift/force-unwrap` (`!`, `try!`, `as!` added), `swift/ats-disabled` (`NSAllowsArbitraryLoads` in plist), `swift/secret-in-userdefaults`, `swift/weak-crypto` (`CC_MD5`, `Insecure.MD5/SHA1`), `swift/webview-js` (permissive `WKWebView` config), `test/xctskip` (neutered XCTest: `XCTSkip`, empty assertions), `swift/empty-catch`.
- **Existing guards upgraded on AST:** complexity (real branch nodes, nesting), test neutering (real test functions), symbols/typos (real identifiers, not keyword lists). Done last and gated by equal-or-better fixtures.
- **Folding/TUI:** TUI fold uses the same parse to find function/block ranges (separate, later).
- **Config:** `[languages] enable = [...]`, per-rule severity override via the existing rule table.

## Risks
- Build time and binary size: grammars are C; measure, and gate behind cargo features (`lang-web`, `lang-swift`) defaulting on in release.
- Swift grammar is the least mature; budget fixture time. Cross-compilation of C grammars in the release matrix must be verified.
- Next.js conventions change (App Router vs Pages); rules key on explicit directives and file paths (`app/**/route.ts`), not version sniffing.

## Testing
- Per rule: positive and negative fixture files under `tests/fixtures/<lang>/`; assert rule ids and that fingerprints are stable across a line shift.
- Parse-failure and mixed-language diffs: guard pack silent, regex guards still fire.
- Mutation and coverage gates unchanged.

## Phasing
E1 `lgtm-ast` + TS/TSX + diff mapping + one rule end to end. E2 web/Next pack. E3 Swift grammar + pack. E4 upgrade complexity/test-neutering/symbols to AST. E5 TUI folding on AST.

## Open questions
- ~~Which matters first?~~ Decided: web first (E1, E2), Swift after (E3).
- Is a binary size increase acceptable (est. low single-digit MB per grammar)?
- Add Go/Java next, or stop at these three?
