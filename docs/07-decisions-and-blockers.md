# 07 Decisions, open decisions, blockers and design corrections

Status: dated 2026-10-29. Writer: W3.
Read with: [01 architecture](01-architecture.md), [04 first slice](04-first-slice.md), [05 plan](05-implementation-plan.md), [06 tests](06-testing-plan.md), [ADRs](adr/).

Labels follow [01, How to read this file](01-architecture.md#how-to-read-this-file).

---

## 1. Decisions made

### 1.1 Owner decisions (binding)

Source: [decisions.md](../research/traces/decisions.md), "Owner decisions (binding)", dated 2026-10-29.

| ID | Decision | Effect in these docs |
|---|---|---|
| O1 | Books is a separate service in its own repo (`github.com/buwiz-corp/buwiz-books`, local clone `/Volumes/goldcoders/buwiz-books`). It owns business profiles, the ledger, categories, and the Books-to-Forms bridge. buwiz-cloud is used only for sign-in and to confirm which approved tax profiles the user owns. | ADR 0001. [01 section 4.5](01-architecture.md#45-cross-plugin-reach). |
| O2 | The ledger is local-first: encrypted local storage inside the Books mini app's own MCP process, the same pattern as BIR's SQLCipher database. Cloud sync is a later, separate step. | ADR 0002. [01 section 6](01-architecture.md#6-data-ownership-and-local-first-store). |
| O3 | Bookkeeping is universal (business profile). PH tax compliance (tax profile, TIN, COR, BIR forms) is a separate, optional, country-scoped layer that a business profile links to. | [01 section 0](01-architecture.md#0-summary), section 10. |
| O4 | The new repo holds the research and details needed to build the mini app. The TypeScript Books repo at `/Volumes/goldcoders/OSS/buwiz-books` is a reference for domain logic, not the runtime backend. | ADR 0005. [05 section 1](05-implementation-plan.md#1-principles). |
| O5 | Do not deploy, alter real financial data, or file returns. The first slice stops at a reviewable draft. | [04 section 1](04-first-slice.md#1-goal). Slice B stops at a draft. |

### 1.2 Peer contracts (binding)

Sources: [decisions.md](../research/traces/decisions.md), "From the rox session" and "From the buwiz-cloud session", both dated 2026-10-29.

| ID | Peer | Contract | Effect |
|---|---|---|---|
| P-R1 | Rox | Protocol v1 only. Tickets never raw IDs. Attachments `@short:scope:value`. Money as text. Signed calls carry safe integers only. `args_json` for JSON args. `rox.fit` for cards. Same-plugin opens do not prompt. `rox.storage` 1 MB per app. Routines declare reads and writes. No Keychain in rebuilt binaries (env key file and `*_NO_KEYCHAIN=1`). The agent submits only with a send grant. TINs resolve locally. | [01 section 4](01-architecture.md#4-rox-plugin-manifest-sketch), section 6.2. |
| P-R2 | Rox | A new plugin must be added to the broker's `approved_plugins` trust list. That is a Rox-side change. | `server/src/v1/trust.json:11-23` [V]. [05 P0](05-implementation-plan.md#p0-plumbing-s). |
| P-C1 | buwiz-cloud | v1 has one account and no organizations or workspaces. Do not contradict. | [01 section 10.3](01-architecture.md#103-consistency-with-buwiz-cloud-v1-no-organizations). |
| P-C2 | buwiz-cloud | A tax profile is a TIN: 9-digit root and 5-digit branch. Active only after claim, Form 2303, ID and manual review. | [01 section 5.1](01-architecture.md#51-observed-identity-facts). |
| P-C3 | buwiz-cloud | `GET /api/tax-profiles` returns `{"profiles":[{id, display, permissions[]}]}`. `id` is a stable opaque UUID, never reused. Approved means non-empty `permissions`. | Contract. Live drift is B1. |
| P-C4 | buwiz-cloud | Device-code sign-in, rotating refresh, `/me`. A sibling public client `buwiz-books`. | `src/oauth.rs:21` [V] registers it. |
| P-C5 | buwiz-cloud | Books links a business to a tax profile by the opaque `id`, never by TIN. | [ADR 0003](adr/0003-tax-profile-link-by-opaque-id.md). |
| P-C6 | buwiz-cloud | Accountant-for-client delegation is not in v1. | [01 section 10.2](01-architecture.md#102-accountants-for-clients-d15). |
| P-C7 | buwiz-cloud | Rox `tkt_` tickets are plugin-local capabilities. Nothing on the server maps them. | [01 section 5.4](01-architecture.md#54-ticket-rules-proposal). |

### 1.3 Design decisions (W3 write-up, 2026-10-29)

These follow the canonical design's final decisions, numbered as in that design. Each is a PROPOSAL until the owner accepts it. Where the owner has already decided, the row says so.

| DD | Decision | Where |
|---|---|---|
| DD-1 | Books is a separate Rox plugin `dev.goldcoders.books` (owner O1 covers the service; the plugin split is the design's choice) | [ADR 0001](adr/0001-separate-books-plugin.md) |
| DD-2 | Ledger is local-first in the Books MCP process, encrypted, with a key file | [ADR 0002](adr/0002-local-first-ledger.md) |
| DD-3 | Books stores no TIN in any phase. Cloud link by opaque `id`. | [ADR 0003](adr/0003-tax-profile-link-by-opaque-id.md) |
| DD-4 | TIN, profile, draft and validation state stay in BIR. Books reaches them only through the model with BIR tickets. | [01 section 4.5](01-architecture.md#45-cross-plugin-reach) |
| DD-5 | Money is integer centavos. Rates are basis points. JSON carries two-decimal text. No floats in Books. | [01 section 6.4](01-architecture.md#64-money-and-time) |
| DD-6 | PT = round half away from zero of (base centavos × rate bps ÷ 10000), once, at output | [01 section 7.7](01-architecture.md#77-worked-example-synthetic-golden-data); A10 |
| DD-7 | Tax meaning is an explicit line attribute. An account never implies a form field. | [ADR 0006](adr/0006-tax-meaning-on-lines-not-accounts.md) |
| DD-8 | Rules are versioned, effective-dated, bound to a pinned Forms schema SHA-256. Proposals store rule set id, schema hash, rate row ids and inputs hash. | [01 section 7.3-7.4](01-architecture.md#73-rule-schema-illustrative-proposal) |
| DD-9 | PT rate rows: 1% for 2020-07-01 to 2023-06-30; 3% otherwise. The reported 1% extension to 2025-12-31 is stored disputed and off, and Books warns. | [01 section 7.3](01-architecture.md#73-rule-schema-illustrative-proposal); D6 |
| DD-10 | VAT gate: posted sales year-to-date strictly above ₱3,000,000.00 (300000000 centavos). The threshold is a dated fact, not a constant. | T-BLK-01, T-BLK-02 |
| DD-11 | Hand-off v1 is model-mediated: `bir_form_fill` with `source: "document"` (label pending D3). Never `user`, never `from_card`. | [01 section 9.2](01-architecture.md#92-mvp-mechanism-p3-model-mediated-proposal) |
| DD-12 | Hand-off v2 (P8): a Books-signed envelope with its own `books` source, and per-box provenance persisted in BIR | [01 section 9.3](01-architecture.md#93-later-mechanism-p8-proposal); [ADR 0004](adr/0004-forms-bridge.md) |
| DD-13 | Books never sends, submits, files, queues or pays, and never calls `bir_form_send` | [01 section 9.6](01-architecture.md#96-hazards); T-UR-04 |
| DD-14 | In an 8%-election year, Books produces no 2551Q values until D5 is decided, and emits no NIL PT010 row in MVP | [01 section 7.5](01-architecture.md#75-edge-cases); D5 |
| DD-15 | A void in an open period posts a reversal dated like the original entry. A reversal in a later period is a return event that blocks until acknowledged. | [01 section 7.6](01-architecture.md#76-void-rule-proposal-open-question-q-1); Q-1 |
| DD-16 | Locks are per tax period, not the org-wide `closedThrough`. Closed entries are immutable. Reopen needs a reason, writes `period_event`, and marks proposals stale. | T-PER-01 to T-PER-08 |
| DD-17 | Every write takes `client_request_id`. A repeat returns the stored result. | [06 section 6](06-testing-plan.md#6-idempotency-and-retry-tests) |
| DD-18 | Certificates credit only status `received`. Period fields are DATE. One certificate feeds one form. WB084 is final and never credited. | T-P4-01 to T-P4-04 |
| DD-19 | Cloud contract: Books reads `id`, `display`, `permissions`. Non-empty `permissions` means approved. Books refuses TIN-shaped fields and never relies on `filing.submit`. | [01 section 5.7](01-architecture.md#57-cloud-link-p5-proposal); B1 |
| DD-20 | The assistant may create drafts. Posting, voiding, closing, reopening, fact acknowledgement and hand-off confirmation are app-only, behind a user click. | [01 section 5.5](01-architecture.md#55-who-may-do-what) |
| DD-21 | Titles and cards carry no amounts, TINs or legal names by default | [01 section 5.3](01-architecture.md#53-identity-and-secret-table-proposal-unless-marked); T-PRIV-02 |
| DD-22 | Inline cards show counts and status only, until the owner accepts more | [01 section 5.6](01-architecture.md#56-consent-points) |
| DD-23 | Trust path: stage A runs under a per-run trust file. The packaged entry ships only after A0 to A15 and B0 to B8 pass. | [05 section 2](05-implementation-plan.md#2-phase-summary) release gate; D21 |
| DD-24 | Rox signing and conformance tools are Node. The Books repo is Rust. The Bun rule belongs to BK-TS. | [05 P0](05-implementation-plan.md#p0-plumbing-s); D17 |
| DD-25 | Slice B runs only against a scratch BIR profile. No test opens the live BIR database. | [04 section 5](04-first-slice.md#5-setup); B0 |
| DD-26 | BK-TS is a reference only. No import, no change, and its defects are not ported. | [ADR 0005](adr/0005-stack.md); [05 section 1](05-implementation-plan.md#1-principles) |

---

## 2. Open owner decisions

Each entry gives the options, the recommendation, and what it blocks. The default is the recommendation.

| ID | Decision | Options | Recommendation | Blocks |
|---|---|---|---|---|
| D1 | Titles: neutral label or business name | (a) neutral label; (b) business name | (a) neutral label | A2; P1 release |
| D2 | Key custody | (a) key file now; (b) passphrase-wrapped key (Argon2id) with a backup prompt; (c) Keychain | (a) for MVP; (b) before release. Avoid (c), per the Rox constraint. | Release. Also P8 envelope key (see C5). |
| D3 | Hand-off source label | `document`, `ai`, or a new `books` value in P8 | `document`, confirmed by the Forms owner. `books` in P8. | B8 (label). Slice B. |
| D4 | PT timing basis: entry, receipt or invoice date. Returns and non-operating income treatment. | (a) entry date with a flag; (b) receipt date; (c) invoice date | (a), with a flag. Tax adviser decides the rest. | P2 non-default cases |
| D5 | 2551Q in an 8% year | (a) block, no values, no NIL row; (b) NIL PT010 row each quarter, as buwiz-oss requires; (c) no 2551Q, as RMO 23-2018 and Books' deadline note say | (a) block in MVP. Fetch RR 8-2018 full text. | Any 8%-year output. Conflict is CONFIRMED; the resolution is UNCONFIRMED. [verify §2 items 73, 84]; [bir §3.2 8-6]; buwiz-oss `form_2551q.rs:2202-2228` [V]; BK-TS `deadlines.ts:122` |
| D6 | Authoritative source for the 1% extension to 2025-12-31 | (a) off and warn, get primary text; (b) on | (a) | 2023-25 proposals as `ready`. Claim UNCONFIRMED [verify §2 item 63]. |
| D7 | Rounding: half away from zero in Books and Forms; integer centavos on the PT line | Yes / no | Yes | P3 golden tie (R-11) |
| D8 | Routine writes for Books | None / some | None | P9 |
| D9 | Publisher key | (a) reuse `gc-2026-01` (first-party inline behaviour); (b) separate key (cards need a click) | (a), with counts-only cards (D22) | P0 signing |
| D10 | Uninstall: keep or delete data | Keep / delete | Keep | None |
| D11 | Model visibility of lineage and entry references | (a) totals and tickets only; (b) entry references too | (a). Lineage app-only. | P1 tool schemas |
| D12 | Slice B isolation | (a) scratch BIR through a second broker; (b) BIR's own harness | (b) if (a) cannot be shown to be safe (UNKNOWN). See B2. | Slice B on the Rox host |
| D13 | Repo naming: buwiz-cloud local, buwiz-server remote | Keep / rename | Keep. Settle before first release. | Release |
| D14 | Overdue Q3 2026 2551Q (due 2026-10-25) | Show overdue, compute nothing / compute penalties | Show overdue with no penalties. The owner decides the real return outside Books. | Not Slice A |
| D15 | Accountant access | (a) pass-based; (b) owner-signed delegation | (a). Timeline P9. | P9 |
| D16 | Assistant may create drafts (`books_entry_draft`) | Yes / no | Yes. Posting is app-only. | P1 |
| D17 | Node tooling in a Rust repo | Exempt Rox tools / no | Exempt Rox signing and conformance tools | P0 |
| D18 | Cloud contract drift (B1) | Per-field decisions | Yes to all: drop `tin`, `tin_masked` (provide `display`), `registered_name`, `activated_at`; drop `filing.submit` for `buwiz-books`; hash or encrypt stored TINs | P5 |
| D19 | Sign-in at first run | Required / optional | Optional. Books works offline. | P5 |
| D20 | Sales returns and credit notes in the 2551Q base | (a) block until acknowledged; (b) gross with a provisional flag | (a). Use (b) once the rule is verified. | P2 |
| D21 | Trust release timing | Before or after stage A | After A0 to A15 and B0 to B8 pass in stage A | Installed users |
| D22 | Canonical tree for any BIR or Forms change | Live `reverse-engineer-ebir-forms/bir`; buwiz-forms pin; buwiz-oss | Live tree for code. Buwiz-forms pin for schema data only. | P3, P8 |
| D23 | Payor TIN for 2307 matching | Plaintext / HMAC plus masked display | HMAC for matching, masked display only | P4 |

### 2.1 New open questions (raised while writing)

These are not in the canonical design. Each needs an owner's call.

| ID | Question | Options | Recommendation | Blocks |
|---|---|---|---|---|
| Q-1 | How does a void affect the tax base? | (1) original becomes `voided`; base skips voided entries and reversal entries; (2) original stays `posted`; reversal is `posted`; base sums both to zero | (1) | P1 exit. A4 passes under either. [01 section 7.6] |
| Q-2 | Does Books post the PT accrual to the ledger (Dr 94100, Cr 21800)? | (a) no accrual in MVP; proposal only; (b) post at period close | (a). Decide at P2 exit. BK-TS never posts these accounts `[books §3b]`. | P2 exit. Not Slice A. |
| Q-3 | Where does the refresh token live in the Books process? | (a) 0600 file under the data folder; (b) Keychain | (a). Avoid Keychain per the Rox constraint. | P5 |
| Q-4 | What happens when a user types a TIN-shaped number into a label or memo? | (a) refuse with an explanation; (b) warn and store; (c) store and never output | (a) | T-PRIV-06 |
| Q-5 | P6 "per-run keys": what do they protect? | (a) export encryption keys per run; (b) Rox signing keys per run; (c) other | Confirm the meaning. The canonical text does not say. | P6 scope |
| Q-6 | Is `tax_type` one value per line, or a set? A sales line can be a percentage-tax sale (2551Q) and an income-tax sale (1701-MS IV.5, IV.6, IV.13) at once. | (a) make `tax_type` a set; (b) keep one value and add a second attribute for the income-tax family | (a), which the PoC already models. Owner or W2 decision. | P2 and P7 attribute schema |
| Q-7 | What covers PT rate rows before 2020-07-01? The rate table has none, so a 2019 quarter fails closed (`pt_rate_row_missing`). | (a) leave it failing closed until a cited source row is read; (b) add a row now from a cited source | (a). Not needed for Q3 2026 or the MVP. | P2 coverage |
| Q-8 | How is a credit note or sales return classified on its line? The `refunds_and_discounts` default has treatment `none` and no ATC, so it cannot classify a return. | (a) explicit line classification before posting (`sale_pt`, PT010, `contra_sales`); (b) classification from the category default | (a). An unclassified sales-class line blocks (`unclassified_sales_line`). | P2 exit; D20 |
| Q-9 | Should cloud agent (`mcp`) tokens be able to read `tin`? Today `GET /api/tax-profiles` and MCP `list_tax_profiles` return it to any token (F-50). | (a) remove `tin` from every response and keep only the masked field (contract D18); (b) keep `tin` for agents that the person approved | (a). Books itself never uses `mcp` tokens. | P5; cloud contract (D18) |

---

## 3. Access blockers

These blocker IDs (B1 to B11) are separate from the Slice B acceptance criteria B0 to B8 in `docs/04` section 8.2. Where a Slice B row says "B3", it means that acceptance criterion.

| ID | Blocker | Phase | Owner | What unblocks it | Status |
|---|---|---|---|---|---|
| B1 | Cloud contract drift. Live `GET /api/tax-profiles` returns `tin`, `tin_masked`, `registered_name`, `activated_at`. `PROFILE_PERMISSIONS` includes `filing.submit` for every client. | P5 | Cloud session | D18 applied. Response carries `id`, `display`, `permissions` only. `filing.submit` not given to `buwiz-books`. `README.md:74` updated. | OPEN. Confirmed on disk [V] at 9f5ee16: `src/claims.rs:27, 410-433`; `README.md:74`. Agent (`mcp`) tokens reach the same response (F-50). |
| B2 | Scratch BIR isolation for Slice B. The installed BIR plugin's env does not set `BIR_DATABASE_PATH`, so the default path is the live database `[rox §6]`. | P3 | BIR owner, Rox owner | A documented scratch install (second broker, own data folder, own trust file, or BIR's harness) that cannot reach the live database. Account and licence gates passing for a scratch profile. | UNKNOWN. Not tried. [06 section 14] |
| B3 | D3 (hand-off label) confirmed with the Forms owner | P3 | Forms owner | Written confirmation of the label | OPEN |
| B4 | Schema digests for 63267ca | P3 | Forms owner | Hash computed here (see 01 section 9.1) and recorded in `data/crosswalk/*.json` (`schema_sha256`). Owner confirms it. | PARTLY CLOSED. Hash computed and recorded. Owner sign-off OPEN. |
| B5 | 8%-year rules | Any 8% output | Owner, tax adviser | D5. RR 8-2018 full text. BIR CDN returned HTTP 403 for it `[bir §6]`. | OPEN |
| B6 | 1% extension primary text | 2023-25 proposals as ready | Owner, tax adviser | D6. A primary source. | OPEN. Claim UNCONFIRMED. |
| B7 | 1701-MS locks in Forms: items 12, 17 and II015 to II017 | P7 fill | Forms owner | Locks in `draft_locks.rs` `[verify §1 item 3, §2 item 10]` | OPEN |
| B8 | Trust release | Installed users | Rox owner | D21 gate passed | OPEN |
| B9 | Record correction: `decisions.md` says the local buwiz-cloud clone is "not yet created". It exists at `/Volumes/goldcoders/buwiz-cloud` [V]. | None (record only) | Decisions owner | Update the record | OPEN (record) |
| B10 | Tax review of rule sets before P2 and P6 release | P2, P6 release | Tax adviser, via owner | Written review of the PT rule set, the 1601-C item 19 statute text, the PT date basis (D4), the VAT threshold CPI status | OPEN |
| B11 | Cloud agent (`mcp`) tokens can read full TINs (F-50) | Before any cloud change that relies on the contract, and before any agent use | Cloud session; owner | Decide Q-9. Drop `tin` from every response, or limit it to `app` tokens. | OPEN |

---

## 4. Cross-team asks

### 4.1 Rox owner

1. Trust entry `dev.goldcoders.books` in `server/src/v1/trust.json` under `approved_plugins` (lines 11-23). Release only, after B8.
2. Confirm that a second `rox-broker` (its own `--port` and `--data`) can run with a per-run `ROX_TRUST_FILE`. Confirm whether the Rust launcher forwards that variable to the Node side. [06 section 13]. Addresses RK-2 and B2.
3. Confirm the plugin-side refusal mapping. The broker turns any `isError` into `tool_error` (`server/src/v1/sessions.mjs:300`). The spec may intend `permission_denied` for authorization refusals. [rox §5 stage 7, INFERENCE]
4. Confirm that the short name `books` and the command `books` are free among enabled plugins (RK-24).
5. Confirm that the running broker build matches the perf-parity source. [rox §10, UNKNOWN] (RK-20).

### 4.2 buwiz-cloud session

1. Align `GET /api/tax-profiles` with the decisions contract (B1, D18):
   - drop `tin` (`src/claims.rs:429`);
   - rename `tin_masked` to `display` (`src/claims.rs:430`);
   - drop `registered_name` (`src/claims.rs:431`) and `activated_at` (`src/claims.rs:432`);
   - give `buwiz-books` no `filing.submit` (`src/claims.rs:27`; the list is shared by all clients today).
2. Update the contract row at `README.md:74`.
3. Confirm the `buwiz-books` client at `src/oauth.rs:21` is the one Books uses, and confirm its device-flow endpoints.
4. Decide hashing or encryption for `tin_claims.tin_root` (`migrations/0001_init.sql:79`) (D18).
5. Note: `/api/tax-profiles` filters on `status = 'approved'` (`src/claims.rs:418`). Books relies on that.
6. Decide Q-9 with the owner: stop returning `tin` to `mcp`-scope tokens (F-50), or confirm that the current behaviour is intended.

### 4.3 Forms owner

1. Confirm the `document` label semantics for a Books hand-off (D3; R-4, R-19).
2. Add the 1701-MS locks (items 12, 17, II015 to II017) before any 1701-MS fill (B7; `[verify §1 item 3]`).
3. State the half-away rounding rule for the PT line in the Forms cents helper, and keep a tie test (R-11). The live helper is `crates/bir-core/src/forms/form_2551q.rs:1467` [V].
4. Confirm the three full digests in [01 section 9.1](01-architecture.md#91-observed-constraints).
5. Say whether an f64 input is acceptable for the PT line, or propose an integer path (R-11).

### 4.4 BIR owner

1. Confirm the scratch install recipe for Slice B (D12; B2).
2. Gate or remove `bir_form_send` before any live use. Make "allow once" session-scoped. Give standing grants an expiry (RK-13; `[bir-shell §8 item 1]`).
3. P8: add the `books` source and persisted per-box provenance (changes 5, 6, 7 in [01 section 9.4](01-architecture.md#94-changes-needed)).
4. Say whether BIR writes in the background while idle (RK-19).
5. Say whether `bir_form_fields` echoes computed item 14 (B2).
6. Remove the claim path for `source: user` with `from_card` (RK-14; `[bir-shell §8 item 2]`), in a later change.

### 4.5 Tax professional review (through the owner)

1. RR 8-2018 full text: the 8% base and 2551Q in an 8% year (D5).
2. The 1% extension to 2025-12-31: a primary source, or confirmation that none exists (D6).
3. 1601-C item 19 statute text (RR 11-2018 is CONFIRMED); the OT, NSD and hazard basis; the item 23 basis (P6).
4. Annex C scope for TY2026. Annex C is TY2025 `[verify §2 item 80]`.
5. The PT date basis for 2551Q (D4).
6. Whether the ₱3,000,000 VAT threshold has a published CPI-adjusted figure (`[bir PT-7]`, UNKNOWN).
7. Whether the withheld PT credit goes only to 2551Q item 15 (`[bir W-3]`, INFERENCE).

---

## 5. Design corrections

The canonical design is the input to this write-up. This section records every place where the evidence checked on 2026-10-29 disagrees with it, and what these files do about it. The owner and the peers are not asked to re-decide anything here unless a row says so.

| # | Canonical design says | Evidence checked 2026-10-29 | What these files do |
|---|---|---|---|
| C1 | buwiz-cloud at `2887e50`, main, 7 modified files, untracked code | HEAD `9f5ee16` on main, clean, after `085c36d` (MCP and agent OAuth). `2887e50`, `6523fcf` and `12d63e8` are ancestors. `feat/submission-relay` is no longer listed (merged). Code is present (`src/oauth.rs:21`, `src/claims.rs`). | Cite `9f5ee16` with its time. Record the drift here. |
| C2 | Contract row at `README.md:59` | Row is at `README.md:74` at 9f5ee16 (`:67` at 12d63e8) [V] | Cite line 74 |
| C3 | BIR `banned()` at `tools.rs:539-552` guards the send path | In the live tree `banned()` is at `crates/bir-mcp/src/tools.rs:629`, inside `#[cfg(test)]` (module at line 551). The send tool is declared at `tools.rs:174-175`. The production gate is `crates/bir-shell/src/agent/session.rs:1388-1400` (send grant required). | Cite the live gate. Books does not rely on `banned()`. |
| C4 | `bir_form_fill` source enum at `tools.rs:310` | Enum at `tools.rs:309`. Schema default at 310. Code default at 547. | Cite 309-311 and 547 |
| C5 | P8 envelope key decision is "owner (key, D6)" | D6 is the 1% extension. The key-custody decision is D2. | Use D2 |
| C6 | D3 blocks "Slice B (B3)" | B3 in the Slice B list is "no send". The label criterion is B8. | D3 blocks B8 |
| C7 | P2 deliverable `docs/CROSSWALK-2551Q.md` | The repo layout gives the crosswalk to `docs/03-crosswalk.md` (W2) | Folded into `03` |
| C8 | Schema digests "16-hex prefix; full digest recorded from the pinned file, B4" | Full digests computed from `git show 63267ca:...` with `GIT_NO_LAZY_FETCH=1`. No fetch. | Listed in [01 section 9.1](01-architecture.md#91-observed-constraints). B4 hashing closed. |
| C9 | Restart backoff "1 to 32 s" | `server/src/v1/plugins.mjs:34` is `[1, 2, 4, 8, 16, 32, 60]` [V] | Use the full array |
| C10 | Plugin option letters in rox §8 and bir-shell §6 | Letters are inverted between the two traces (canonical correction 6) | Use names only |
| C11 | Journey "D8", delivery "D5", correctness "D-2" are one decision | Canonical correction 8 | D5 everywhere |
| C12 | identity §1 item 1: "buwiz-cloud does not exist" | Stale. The clone exists. | Use the current state (C1) |
| C13 | "R-1 to R-24 register grafted from correctness" | The full register text was not supplied to this writer | [05 section 5](05-implementation-plan.md#5-risk-register) uses the R-IDs the design cites, and RK-n for new risks. A reviewer should merge the full register. |
| C14 | Cents helper at `form_2551q.rs:1259` (buwiz-forms checkout) | Live tree at `form_2551q.rs:1467` [V] | Cite 1467 |
| C15 | Slice B expects computed item 14 in `bir_form_fields` (implied) | UNKNOWN (canonical's own open discrepancy) | Fall back to validate or preview ([01 section 9.2](01-architecture.md#92-mvp-mechanism-p3-model-mediated-proposal) step 6) |
| C16 | Cloud profile field `display` | Live field is `tin_masked` [V] | Noted in [01 section 5.1](01-architecture.md#51-observed-identity-facts); B1 |
| C17 | A4 "voiding posts a reversal" does not say what the base does with the original | Gap, not a contradiction | Q-1 added. Both readings tested. |
| C18 | Canonical treats Books' gate as `bun check` (BK-TS `CLAUDE.md`) | Books is Rust. BK-TS gates do not apply here. | [05 section 7](05-implementation-plan.md#7-definition-of-done-per-phase) and [06 section 11](06-testing-plan.md#11-test-tiers) use Rust gates |
| C19 | Decisions: "no extra scope in v1"; the contract returns a masked `display` only | 9f5ee16 (`085c36d`): grant scopes `app` and `mcp`. `mcp` tokens reach `GET /api/tax-profiles` and MCP `list_tax_profiles`, and both return `tin` (`src/claims.rs:429`; `src/mcp.rs:156-158`) | Recorded as a deviation (F-50). Q-9 asks the owner. Books never uses `mcp` tokens (AGENTS.md "Do not"). |
| C20 | 1701-MS printed rounding ("49 centavos") is CONFIRMED | [verify] #83: UNCONFIRMED (image only) | Set to UNCONFIRMED (docs/03 D-14). Not in code until the printed rule is read. |
| C21 | 1701-MS 15 October installment is CONFIRMED ([bir] D-4) | [verify] #83: UNCONFIRMED | Set to UNCONFIRMED (docs/03 D-15; `data/rules/rates.v1.json` status_detail). |
| C22 | 1601-C printed due-date text is CONFIRMED ([bir] D-3) | [verify] #83: UNCONFIRMED. The code rule (P6) is CONFIRMED in code only | Printed text UNCONFIRMED (docs/03 D-16). The code rule stays. |
| C23 | 2551Q item 13 "Not applicable" is a printed option | [bir] §2 lists two printed options. The pin has `taxRateNA` (CONFIRMED) | Pin key CONFIRMED; printed option UNCONFIRMED (docs/03 D-11). Fill refuses item 13 under the election lock either way. |
| C24 | Nothing recorded on the OSD base | Printed Part I item 17 says 40% of gross sales. Part IV item 11 and the code say 40% of net sales | UNCONFIRMED conflict (docs/03 D-17). The PoC uses net sales and raises `DISC-OSD-BASE-1701MS`. |
| C25 | Nothing recorded on `tax_type` | PoC: a sales line needs `percentage_tax` (2551Q) and `income_tax` (1701-MS IV.5, IV.6, IV.13) at once, and `tax_type` is single-valued in `data/tax-attributes.v1.json` | Open question Q-6. |
| C26 | Nothing recorded on rate coverage before 2020-07-01 | PoC: no PT rate row before 2020-07-01; a 2019 quarter fails closed (`pt_rate_row_missing`) | Gap. Open question Q-7. |
| C27 | Nothing recorded on refund and credit-note classification | PoC: `refunds_and_discounts` defaults to treatment `none` and no ATC; an unclassified sales-class line blocks (`unclassified_sales_line`) | Explicit line classification. Open question Q-8. |

### 5.1 Verified, no change needed

- Money rules, the 2551Q crosswalk keys (`[verify §4]`), and the typed alias `schedule_1.0.atc` (`[verify §2 item 23]`).
- `source` enum and the `user` gate: `tools.rs:309`, `:547`; `session.rs:1214-1249` [V on 1214].
- `is_app_only` at `crates/bir-shell/src/mini_app/server.rs:1191` [V].
- The send grant gate at `session.rs:1388-1400` [V].
- `approved_plugins` at `server/src/v1/trust.json:11-23`, with BIR at line 13 [V].
- `FIRST_PARTY` at `server/src/v1/plugins.mjs:32` [V]. Spawn env persisted at `plugins.mjs:463` [V].
- `ROX_TRUST_FILE` at `server/src/v1/trust.mjs:14` [V].
- `oauth.rs:21` and `claims.rs:27, 429, 433` on buwiz-cloud [V] (9f5ee16).
- BK-TS citations: `percentage-tax.ts:50, 65-68, 106, 354-360`; `vat.ts:70`; `form-1601c.ts:52`; `src/routes/api/-tax-returns.ts:87`; `tax.compute.tsx:37, 108`; `auth.ts:92`; `tax-certificates.ts:47, 53-54`; `certificate-2307.ts:293`; `deadlines.ts:122`; `src/routes/api/-tax-settings.ts:17` [V].

---

## 6. Known gaps in this write-up

- No test was run. No Books code exists yet.
- No live call was made to Rox, BIR or buwiz-cloud.
- The canonical R-1 to R-24 register text is not reproduced (C13).
- Q-1 to Q-9 are open.
- UNKNOWN items: the second broker (B2), `bir_form_fields` echo (C15), BIR background writes (RK-19), BIR audit trail (T-REP-05), whether the Rust launcher forwards `ROX_TRUST_FILE` (06 section 13).
- Resolved (2026-10-29): `fixtures/vectors/money-and-rules.json` now stores `*_centavos` values as integer strings, not JSON numbers, so no consumer parses money into a float. The PoC privacy scanner exempts integer strings under `*_centavos` keys from the TIN check by field context (AGENTS invariant 14), and `poc/test/privacy.test.ts` proves the exemption is narrow. Integer-centavo strings in test vectors are a deliberate exception to the two-decimal display text of invariant 1.
- Records to update: `decisions.md` (B9), and the canonical design's corrections in section 5.
- The PoC does not implement the 1701-MS 8% pathway (`poc/README.md`, suggestion (c)).

## 7. Records to update outside this repo

| Record | Owner | Change |
|---|---|---|
| `research/traces/decisions.md` | Decisions owner | Note that buwiz-cloud exists and has moved since 2887e50 (B9) |
| buwiz-cloud `README.md:74` | Cloud session | Contract row fixed (B1) |
| Live BIR `crates/bir-mcp/src/tools.rs` comment for `document` | BIR owner | Clarify the label (R-4, D3) |
