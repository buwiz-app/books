# Decisions and peer contracts (authoritative for phase 2)

Date: 2026-10-29. Sources: the owner (via AskUserQuestion in the buwiz-books session), the `rox` session, the `buwiz-cloud` session.

## Owner decisions (binding)

1. **Books is a separate service**, in the new repo `github.com/buwiz-corp/buwiz-books` (private, empty; local clone `/Volumes/goldcoders/buwiz-books`). It owns business profiles, the ledger, categories, and the Books→Forms bridge. buwiz-cloud is used only for sign-in and to confirm which approved tax profiles the user owns.
2. **Ledger is local-first**: encrypted local storage inside the Books mini app's own MCP server process (same pattern as BIR's SQLCipher DB). Cloud sync is a later, separate step.
3. Bookkeeping is universal (business profile); PH tax compliance (tax profile / TIN / COR / BIR forms) is a separate, optional, country-scoped layer that a business profile *links to*.
4. Goal of the new repo: hold all the research and details needed to build the mini app, so intelligence can be added without hard-coupling to the existing (hard to maintain) TypeScript Books repo at /Volumes/goldcoders/OSS/buwiz-books. That repo is a **reference** for domain logic, not the runtime backend.
5. Do not deploy, alter real financial data, or file returns. The first slice stops at a reviewable draft.

## From the `rox` session (owner of Rox)

- Live BIR plugin source: `/Volumes/goldcoders/reverse-engineer-ebir-forms/bir` (hexuria/buwiz-forms, branch `rox-mini-app-round2`). Apps `crates/bir-shell/mini-apps/*.tsx` + `apps.json`; manifest generated in `crates/bir-shell/src/mini_app/v1/manifest.rs`, signed with publisher key gc-2026-01 via the broker's `src/v1/sign-manifest.mjs`; backend `bir-mini-app mcp` stdio MCP server; broker spawns it (`rox/server/src/v1/plugins.mjs`, 127.0.0.1:8787, now fronted by a Rust `rox-broker`).
- Protocol v1 only (v0 retired for BIR). Spec: `drafts/rox-mini-app-protocol-v1.md` (private rox-stow). SDK/template: `rox/server/sdk/mini-app`; smallest example: `rox/server/src/v1/fixture/`.
- New plugin steps: MCP server exposing tools + `rox://manifest` + app sources → manifest (id, short, version, apps[{id, entry rox-app://…, permissions.tools, props, placement}], scopes/commands/routines) → sign (`node rox/server/src/v1/sign-manifest.mjs <manifest> --keychain gc-2026-01` or `--key-file`) → `npm run plugin-test` → install via Connect apps or `/v1/host/plugins/inspect` + `/install`. A new plugin must also be added to the broker's approved_plugins trust list (Rox-side change).
- Constraints: tickets never raw IDs; attachments `@short:scope:value` with declared scopes + resolve tool; money as text, signed app calls carry safe integers only, JSON args as `args_json`; content-sized cards (rox.fit); every tool an app calls is in its manifest permissions; same-plugin app opens don't prompt, outside links do; rox.storage per app 1 MB; routines declare reads/writes/params; avoid Keychain in rebuilt binaries (env key file + *_NO_KEYCHAIN=1); agent submits only with a send grant; TINs resolve locally.

## From the `buwiz-cloud` session (status: design only, nothing built)

- Repo `github.com/buwiz-corp/buwiz-server` (private, empty); local `/Volumes/goldcoders/buwiz-cloud` (not yet created). axum + Postgres.
- v1: email one-time-code sign-in; one account; **no organizations, no workspaces** (owner's direction "make it simple"). Do not contradict.
- Tax profile = TIN (9-digit root + 5-digit branch). One account owns 1+ TINs; one TIN has one owner. Active only after claim: Form 2303 + ID + docs, manual staff review (awaiting_file → pending → approved/rejected/withdrawn).
- `GET /api/tax-profiles` → `{"profiles":[{ id (stable opaque UUID, never reused), display (masked e.g. ***-***-054-00000), permissions[] }]}`; approved = non-empty permissions. Any token for the user sees only their own approved TINs; no extra scope in v1.
- Auth: device-code (`POST /auth/device/start`, `/auth/device/poll`), `POST /oauth/token` refresh w/ rotation, `GET /me` → {user_id, email}. buwiz-cloud will register a sibling public client **`buwiz-books`** (device-code + refresh, same /me).
- **Books links a business profile to a tax profile by the opaque tax-profile `id` UUID, never by TIN** (the registration key *is* the raw TIN).
- Accountant-for-client delegation: NOT in v1 (buwiz-oss internal/PLAN.md §5.4 sketches owner-signed revocable delegation later). Submission tickets per buwiz-oss internal/PLAN.md §5 not built.
- Server decisions: BIR secrets only on server; mini app submits to BIR directly over SFTP today; buwiz-oss internal/DECISIONS.md D2/D3 (server = TIN ownership + submission relay).
- Rox `tkt_…` tickets are Rox/plugin-local capabilities; nothing on the server maps them.

## Update 2026-10-29 (later): buwiz-cloud contract revised

Source: the `buwiz-cloud` session, after this repo reported F-50. Fix in `github.com/buwiz-corp/buwiz-server/pull/4` (head `42190a5`; open with CI running when recorded).

- Device-flow clients get their own scope: `buwiz-desktop` → `app`, `buwiz-books` → `books`. (Before the fix, `buwiz-books` had `app` and could create submissions.)
- `GET /api/tax-profiles` for a `books` or `mcp` token returns `{id, tin_masked, registered_name, activated_at, permissions:["profile.view"]}` with no `tin` field. The field is `tin_masked`, not `display` as worded above.
- `books` and `mcp` tokens get 403 `insufficient_scope` on `POST /api/submissions` and the relay.
- MCP `list_tin_claims` returns `tin_masked` only.
- Link business profiles by `id`. Close F-50 and B1 once PR #4 is merged.

## Addendum 2026-10-29: owner correction of the Books scope (binding)

Source: the owner, relayed to the buwiz-books session on 2026-10-29. Recorded in `docs/adr/0007-books-supplies-measures-forms-owns-compliance.md`. It corrects the scope that the phase-2 design had drifted into (Books computing tax).

- **Books** is bookkeeping and accounting. It records transactions, payroll facts and evidence, and produces **accounting measures** per business profile per period, each with lineage (source line ids). Examples: gross sales; sales returns, allowances and discounts (separate measures); cost of sales; other non-operating income; expenses by category; payroll gross compensation and its components (basic, holiday/OT, 13th month and other benefits paid, de minimis paid); EMPLOYEE mandatory contributions withheld (SSS/GSIS, PhilHealth, Pag-IBIG, union dues), kept separate from EMPLOYER shares; tax withheld from employees as recorded by payroll (a fact; Books does not compute withholding); certificates received (2307/2316) as evidence with status.
- **The bridge** maps measures onto Forms **input** boxes only (`fill_class: input`), with lineage. Where a form has alternative boxes that depend on a taxpayer choice (for example 1701-MS taxpayer sales: `frm1701MS:txtSalesRevenueNo5a` → `/taxpayer/sales` for graduated versus `frm1701MS:txtSalesRevenuesFeesANo19a` → `/taxpayer/eight_sales` for 8%), the bridge reads the choice as an INPUT from the Forms draft or profile context. It never decides it, and never targets computed, locked or not_fillable boxes.
- **Buwiz Forms / the BIR mini app** (the compliance side) owns everything else: elections (8% versus graduated, chosen by the user at the start of each year in the tax profile), whether a form is required or applicable for a period (including 2551Q in an 8% year and NIL returns), ATC selection, rates (including the 1%/3% percentage-tax history), tax due, thresholds (VAT ₱3M, 8% eligibility, mixed income), OSD versus itemized and the OSD computation, graduated tax, rounding of tax amounts, penalties, due dates and overdue status.
- Consequences for open decisions: D5, D6, D7 and D14 are no longer Books decisions. D4 splits: the revenue-recognition date is an accounting policy and stays with Books. D20 becomes a mapping question for tax review (returns are a separate measure; which measure feeds 2551Q `txtATCAmt1` is a mapping rule).
- One gray area is kept and labelled: 1601-C item 17 asks for the NON-TAXABLE part of 13th month pay and other benefits (annual ₱90,000 cap), which needs year-to-date payroll that only Books has. It stays in the mapping layer as "the one input-level tax rule in the bridge", open for the Forms owner (Forms applies the cap, or the bridge keeps it).

## Addendum 2026-10-29: owner decisions D4 and D9 (binding)

Source: the owner, in the buwiz-books session on 2026-10-29, answering the question about D4 and D9.

- **D4, accounting basis.** Owner: "default should always cash as that is the simplest". Every business profile has an accounting basis, cash or accrual; the default is cash. A cash-basis book posts income when money is received and expenses when they are paid. This replaces the earlier "entry date with a `period_override`" recommendation. Recorded in `docs/adr/0008-cash-basis-by-default.md` and `docs/07` as O7. Which basis a tax return needs stays with Forms (ADR 0007).
- **D9, publisher key.** Owner: "then we can just use the same key, im the only one publishing on goldcoders". Books is signed at release with `gc-2026-01`. Recorded in `docs/07` as O8. Tests and stage A sign with a throwaway key under a test key id (`docs/06` section 13).
- Observed while answering: the TypeScript Books repo has an `accrual`/`cash` report parameter (`src/db/validation/reports.ts:4`) that its report handlers parse and drop (`src/routes/api/-reports.ts:59, 68`). Recorded in `docs/00` section 4.

## Update 2026-10-29: buwiz-server PR #4 merged; Books cloud contract confirmed

Sources: `gh pr view 4 -R buwiz-corp/buwiz-server` (state MERGED, 2026-10-29 07:01:52 UTC, head `42190a5`), and the `buwiz-cloud` session's reply to this session.

- B1, B11 and F-50 are closed.
- The v1 contract for Books is sign-in plus `GET /api/tax-profiles`, nothing more. Nothing else for Books is on the cloud's v1 plan. Sync comes to the cloud session first.
- Sign-in: client `buwiz-books`, scope `books`; `POST /oauth/device/code`, then poll `POST /oauth/token`. Refresh rotates, and reusing an old refresh token revokes the whole grant.
- `books` tokens get 403 `insufficient_scope` on `/api/submissions` and the relay.
- Ignore unknown profile fields; the cloud may add non-sensitive ones, such as `activated_at` (unix seconds). Stable: `id`, the `tin_masked` format, no `tin` for `books`, `permissions:["profile.view"]`. A withdrawn claim drops its profile from the list.
- D18 (`tin_root` at rest) is still open and needs the owner's sign-off. The cloud recommends an HMAC of root and branch under a server key for the uniqueness indexes, plus AES-GCM encryption of the TIN. It does not affect Books.

