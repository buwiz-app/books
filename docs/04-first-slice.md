# 04 First slice: Q3 2026 sales measures and the Form 2551Q inputs they feed, from synthetic books

Status: plan, dated 2026-10-29. Writer: W3.
Read first: [01 architecture](01-architecture.md). Tests: [06](06-testing-plan.md). Open decisions: [07](07-decisions-and-blockers.md).

Labels follow [01, How to read this file](01-architecture.md#how-to-read-this-file). Acceptance IDs (A, B) are testable statements. Test IDs for each one are in [06](06-testing-plan.md).

---

## 1. Goal

Prove the smallest end-to-end chain that is useful:

**synthetic sale → ledger line → accounting measure (with lineage) → bridge mapping → proposal (measures + fill args for input boxes) → BIR 2551Q input box → Forms computes the tax → reviewed draft**

Books supplies the measures and the fill arguments for 2551Q **input** boxes only. Forms selects the ATC, applies the rate, computes the tax due and owns the due date ([ADR 0007](adr/0007-books-supplies-measures-forms-owns-compliance.md)). The slice ends at a reviewable BIR draft. It never sends, submits, files, queues or pays (owner decision 5 `[decisions.md]`). It uses synthetic data only.

## 2. Why 2551Q first

| Reason | Evidence |
|---|---|
| The crosswalk is verified for every field the slice uses. | `[verify §4]` (2551Q rows for `txtATCAmt1`, `frm2551Qv2018:txt15`; `schedule_1.0.atc` and `frm2551Qv2018:taxRate` are Forms-owned and never sent) |
| Forms has a typed model, an agent path and a mapper for 2551Q. 1601-C has an agent path but no mapper. | `[forms §4 Tier A]` |
| It needs only sales measures, which are the simplest accounting totals. | `data/measures.v1.json` (gross_sales, sales_returns, net_sales) |
| It needs no payroll. 1601-C needs payroll and has a known Books defect. | `[books §1 item 1]` (1601-C drops employee contributions; probe taxable 40,000 against 38,000) |
| 1701-MS has unlocked election fields, and Books has no 1701 code. | `[verify §1 item 3]`; `[books §1 item 2]` |
| Q3 2026 is a real return with a real due date (2026-10-25). The slice uses synthetic numbers, so no real filing is touched. | `[bir §3.6 D-1]`; decisions owner 5 |

## 3. Scope

### 3.1 In

- **Slice A.** Books alone, in the real Rox host, with synthetic data. Covers ledger, periods, void, return-event acknowledgement, the Q3 sales measures, the 2551Q input-box proposal, lineage, staleness and retry.
- **Slice B.** A chat hand-off into a 2551Q draft on a scratch BIR profile. Covers the model-mediated steps in [01 section 9.2](01-architecture.md#92-mvp-mechanism-p3-model-mediated-proposal).

### 3.2 Out of scope (explicit)

| Out | Why |
|---|---|
| Payroll and 1601-C (P6) | Needs payroll. BK-TS defect not ported. |
| 1701-MS (P7) | Needs Forms locks first (B7). |
| Evidence and 2307 import (P4) | Line 15 in Slice A is zero. |
| Cloud sign-in and link (P5) | Blocked by B1. Slice A runs offline. |
| Sync (P9) and accountant access | Not designed. |
| Forms provenance, signed envelope (P8) | Slice B uses the `document` label. |
| Any send, submit, file, queue or pay | Owner decision 5. |
| Any election, applicability, rate, tax or due-date logic | Forms' (ADR 0007). Books output does not depend on the election; whether a 2551Q is needed is Forms' call. |
| VAT, 2550Q | Out of MVP. Returns and credit notes are measured and need acknowledgement. |
| Multi-currency | Not in MVP. |
| Any TIN in Books | Decision (ADR 0003). |
| The live BIR database | Owner decision 5; B0. |
| Tax due, rounding, due date, overdue status, penalties | Forms' (handed over with D7 and D14). |

## 4. Synthetic data set

All values are invented for this slice. Accounts use the `philippines-smb` preset numbers `[books §3a; BK-TS base.ts]`.

| # | Date (Asia/Manila) | Description (synthetic) | Debit | Credit | Amount | Line attributes (accounting facts) |
|---|---|---|---|---|---|---|
| E1 | 2026-07-01 | Consulting sale 1 | 11000 Bank accounts | 41000 Sales of services | 120,000.00 | Revenue line: `income_class=business_sales`, `measure_side=credit` |
| E2 | 2026-07-15 | Consulting sale 2 | 11000 | 41000 | 100,000.00 | Same |
| E3 | 2026-08-20 | Consulting sale 3 | 11000 | 41000 | 80,000.00 | Same |
| E4 | 2026-08-05 | Office rent (expense, Q3) | 66100 Rent | 11000 | 25,000.00 | `income_class=operating_expense`. Must not change the sales measures. |
| E5 | 2026-07-15 | Void of E2 | 41000 | 11000 | 100,000.00 | Reversal dated like E2 (see A4) |
| N1 | 2026-08-05 | Unbalanced negative test | 11000 100.00 | 41000 99.99 | refused | Expect `unbalanced` |

Expected Q3 2026 results:

| Quantity | Value | Who |
|---|---|---|
| Gross sales before void (E1 + E2 + E3) | 300,000.00 | Books measure |
| Net sales after void (E1 + E3), lineage E1, E3 | 200,000.00 | Books measure → `txtATCAmt1` |
| Item 15 (no 2307 in the slice) | 0.00 | Books measure → `frm2551Qv2018:txt15` |
| ATC, rate, tax due (6,000.00 at 3%), totals | — | Forms |
| Due date (2026-10-25), overdue status, penalties | — | Forms |

The Forms figures are recorded only as `expected_forms_computed` documentation in the PoC vectors (`fixtures/vectors/f-2551q-2026-Q3.json`).

## 5. Setup

| Item | Setting |
|---|---|
| Rox host | Local install. Stage A trust: per-run file via `ROX_TRUST_FILE` (Rox repo, `server/src/v1/trust.mjs:14` [V]). No change to the packaged `trust.json` yet (D21). |
| Clock | Injected. Fixed "as of" date 2026-10-29 for the overdue check. |
| Books data | A fresh temp folder per run. Key file generated for the run, mode 0600. No Keychain. |
| BIR (Slice B only) | A second broker on its own port and `--data` folder, with BIR installed and `BIR_DATABASE_PATH` set to a scratch file. Never the live Grok Shell broker. See B0 and RK-1 in [01 section 11](01-architecture.md#11-architecture-risks-summary). |
| Node | Used only for the Rox conformance runner and the signing tool (D17). Books itself runs on Rust. |
| Tokens | Redacted in every captured transcript. |

## 6. Demo script, Slice A

Each step lists what to do, what must be seen, and the criteria it proves.

| Step | Do | Expect | Proves |
|---|---|---|---|
| S1 | Connect apps, pick `books`, review permissions, install. | `books` listed with 6 apps and 1 command. Permissions match 01 section 4.2. | A1 |
| S2 | Home, "New book", label "Demo Book Q3", country PH. | Book created. Ticket `tkt_...` issued. Pin shows only the neutral label. | A2 |
| S3 | Period app or home: apply `philippines-smb`. Apply it a second time. | 21 PH accounts and the base accounts exist. The second apply changes nothing. | P1 idempotency (BK-TS `CLAUDE.md` describes its COA apply as idempotent and non-destructive, `[books §3b]` for the chart) |
| S4 | Post E1, E2, E3 in the ledger app. | Three entries posted in 2026-Q3. Revenue lines show `income_class=business_sales`. | A3 |
| S5 | Post N1. | Refused with `unbalanced`. Nothing stored. | A3 |
| S6 | Post E4. | Posted. Sales measures unchanged (gross still 300,000.00 before void). | Negative control |
| S7 | Void E2 in the ledger app. | Reversal E5 posted, dated 2026-07-15. Q3 net sales 200,000.00. | A4 |
| S8 | (No tax facts.) Elections are set by the user in the Forms tax profile, not in Books. | Books asks for nothing here. | ADR 0007 |
| S9 | Open card-proposal for 2026-Q3 (2551Q). | Status `ready`. Net sales measure 200,000.00 with lineage. `fill_args`: `txtATCAmt1` 200000.00, `frm2551Qv2018:txt15` 0.00. No tax, no due date, no ATC. | A5 |
| S10 | In card-proposal, open lineage (app-only). | Lists E1 and E3 with contributions summing to 200,000.00. E2 and E5 are excluded. | A5, A14 |
| S11 | Period app: close 2026-Q3. Try to edit E1's amount. | Close stores a snapshot payload and hash. Edit refused. | A7 |
| S12 | Period app: reopen 2026-Q3 with reason "synthetic correction". | Proposal marked `stale`. `period_event` written. Regenerate creates a new row. The old row is `superseded`. | A7, B4 |
| S13 | Call generate three times with one `client_request_id`. | One proposal row. Same result each time. | A9 |
| S14 | Run the Books privacy scan on the transcript, store and logs. | No TIN-shaped run. No `tkt_` in titles. No key material in env. | A11 |
| S15 | Stop the Books process, then the broker. Start both. | Data unchanged. Sessions reconnect. | A8 |
| S16 | Open the store with a wrong key. | Refuses. No partial state. `strings` on the file finds no memo marker. | A8 |

## 7. Demo script, Slice B (scratch BIR profile)

Precondition: B0 passes. The scratch path is asserted, and the live database checksum is recorded.

| Step | Do | Expect | Proves |
|---|---|---|---|
| B-S0 | Record the live BIR database checksum. Confirm `BIR_DATABASE_PATH` points at the scratch file. | Checksum recorded. Path confirmed. | B0 |
| B-S1 | In one chat, attach the Books `proposal` token and pick the BIR `taxprofile` token through BIR's own composer flow. | A consent dialog for the Books attach, accepted by the user. The BIR token comes from BIR's resolver (`[bir-shell §6]`). | B1 |
| B-S2 | Assistant calls `books_form_proposal_get`. | Status `ready`. `fill_args` only. No TIN. No Books IDs. | B1, B5 |
| B-S3 | Assistant calls `bir_form_open` with code 2551Q, year 2026, period 3, and the BIR ticket as `tin`. | A Draft opens. | B1 |
| B-S4 | Assistant calls `bir_form_fill` once with `fields = fill_args` and `source = "document"`. | Applied with no refused key. `kept_user_boxes` empty. | B1, B8 |
| B-S5 | Assistant calls `bir_form_fields`. Compare every value with `fill_args`, exact string. | Equal. | B7 |
| B-S6 | User opens the BIR draft. Check schedule 1 row 1. Forms has chosen the ATC and computed item 14. | Row 1 taxable amount 200000.00. Item 14 is Forms' output (6000.00 at 3% if the user's Forms setup applies PT010); Books does not compare it. | B2 |
| B-S7 | Assistant calls `bir_form_validate`, then `bir_form_save_draft`. | Saved as Draft. No send. | B3 |
| B-S8 | User clicks "BIR shows these values" in card-proposal. | `fill_args_sha256` and time recorded. App-only call. | B5, A15 |
| B-S9 | Check the audit log and the live checksum. | No `bir_form_send` call. Nothing queued. Item 13 untouched. Live checksum unchanged. | B0, B3, B6 |
| B-S10 | Change one unclosed ledger line in Books. Ask the assistant to fill again. | Proposal `stale`. Assistant refuses and says why. | B4 |

Stop the flow at any mismatch in B-S5 or B-S6. Do not "fix" a value in BIR by hand to make it match.

## 8. Acceptance criteria

Each criterion is a testable statement. A criterion passes only with the evidence named in 06.

### 8.1 Slice A

| ID | Statement | Evidence |
|---|---|---|
| A0 | BK-NEW has a Cargo workspace and its own `CLAUDE.md`. No crate depends on a BK-TS or BIR path. BK-TS `git status --short` output is the same before and after the run. | Static check (grep for path dependencies). Before and after `git status` text. |
| A1 | The signed manifest installs through Connect apps with `manifest_sha256` pinned. `books` appears. `rox-plugin-test` checks P3, P4 and P7 pass. The trust entry is present through the per-run trust file. | Conformance JSON output. Install record. |
| A2 | A synthetic book is created with a `tkt_` ticket. The pin shows only the neutral label. No TIN-shaped string appears in any output. | Output capture scan (06 section 8). |
| A3 | E1, E2 and E3 post in 2026-Q3, plus E4. N1 is refused with `unbalanced` and nothing is stored. | Ledger test. Store row count before and after N1. |
| A4 | Voiding E2 posts E5, dated 2026-07-15. Q3 gross becomes 200,000.00 exactly. | Ledger and tax-base test. Reading 1 or 2 per 01 section 7.6. |
| A5 | 2026-Q3 status is `ready`. Net sales measure 200,000.00. `fill_args` holds input keys only (`txtATCAmt1` 200000.00, `frm2551Qv2018:txt15` 0.00): no ATC, rate, tax, total, due date or penalty. Every `fill_args` key passes the contract test (pinned bindings, fill_class input). Lineage lists E1 and E3 and not E2 or E5. (This lineage shape assumes Reading 1 of the void rule. Under Reading 2 the lineage lists E2 and E5 netting to zero. See 01 section 7.6, Q-1.) | Golden test (06 section 3). Contract test (06 section 4). |
| A6 | A returns, allowances or discounts line in the quarter (or a credit note, or a later-period reversal) holds the proposal until acknowledged, and acknowledging regenerates it. Returns are a separate measure. The Books output does not change with the taxpayer's election (an 8% election, the VAT threshold and applicability are Forms'). | Blocking matrix tests (06 section 3). |
| A7 | A closed entry cannot be edited. A snapshot payload and hash are stored at close. Reopening with a reason marks the proposal stale and writes `period_event`. Regeneration creates a new row. | Period tests (06 section 3.4) and staleness tests (06 section 7). |
| A8 | Restarting the Books process and the broker leaves data unchanged. A wrong key file refuses to open with no partial state. `strings` on the database file finds no memo marker. | Restart and store tests (06 section 5). |
| A9 | A repeated write with the same `client_request_id` returns the same result. Three `books_proposal_generate` calls with one id create one row. | Idempotency tests (06 section 6). |
| A10 | Money is integer centavos and two-place decimal text end to end. No float touches a measure. Measures are exact sums and are never rounded (tax rounding is Forms', ADR 0007). | Money tests (06 section 3). |
| A11 | A scanner over tool results, app props, attachment titles, logs and the store file finds no TIN-shaped run (dashed or undashed, 9, 12 or 14 digits), no `tkt_` value in a title, and no key material. Env holds only paths. | Privacy scanner (06 section 8). |
| A12 | An app calling a tool outside its permissions gets `permission_denied` (Rox repo, `app/src/mini_v1/bridge.rs:457`). A plugin-side refusal arrives as `tool_error` (Rox repo, `server/src/v1/sessions.mjs:300`). Books treats both as refusal and never branches on the code. | Denial tests (06 section 10). |
| A13 | An entry list of 1,000 synthetic lines pages at 200 rows. No result exceeds 64 KiB. Every call completes inside the 30 s bridge timeout. | Limits test (06 section 9). |
| A14 | **Traceability.** For each `fill_args` key, exactly one mapping produces it, from named measures. Its lineage contributions sum to the value. Each contributing line is a posted line that is not voided. A transaction in the lineage can be followed to its measure, mapping and proposal row, and back. | Traceability test (06 section 4). |
| A15 | **User review.** No proposal reaches BIR without a user click in card-proposal. `books_app_handoff_confirm` is app-only. A model-origin call to it is refused. Inline cards show only counts and status. | Permission and user-review tests (06 section 10). |

### 8.2 Slice B

| ID | Statement | Evidence |
|---|---|---|
| B0 | The live BIR database checksum is identical before and after the run. The scratch path is asserted before any call. | Checksum before and after. Path assertion log. |
| B1 | In one chat the user attaches the proposal and the BIR `taxprofile` token. The assistant calls `books_form_proposal_get`, then `bir_form_open` (2551Q, 2026, period 3), then `bir_form_fill` once with `source: "document"`. No key is refused. | Transcript (redacted). |
| B2 | The BIR draft shows schedule 1 row 1 taxable amount 200000.00, equal to Books' fill argument. Computed boxes are Forms' output and are not compared with any Books value. A mismatch on an input stops the flow. | Screenshot or text read of the draft. Read-back output. |
| B3 | `bir_form_send` is never called. Nothing is queued. The draft stays Draft. | Audit log (06 section 12). |
| B4 | After a ledger change the proposal is `stale`. The assistant refuses to fill and says why. | Transcript. Proposal status. |
| B5 | Books never receives a TIN. `fill_args_sha256` is recorded only after the user's app click. No `user` source and no `from_card`. | Scanner. Handoff table row. Transcript. |
| B6 | Item 13 (`frm2551Qv2018:taxRate` (with its `_1`, `_2` and `NA` variants)) is untouched. | Read-back of item 13 before and after. |
| B7 | `bir_form_fields` returns the same strings as `fill_args`. | Compare output. |
| B8 | The source value is exactly the one agreed under D3. Default is `document`. | Transcript. Decision record. |

### 8.3 Traceability summary

The chain for the headline number, net sales 200,000.00:

```
E1 (120,000.00, 2026-07-01, business_sales)  ─┐
E3 ( 80,000.00, 2026-08-20, business_sales)  ─┴─> measure net_sales 2026-Q3 ─> mapping 2551Q.S1.1.base ─> proposal row (ready) ─> fill_args txtATCAmt1 = "200000.00" ─> Forms computes the tax
E2 (100,000.00) and reversal E5 (-100,000.00) ─> netted out of lineage (Reading 1)  [or listed, netting to zero (Reading 2)]
```

Expected: net sales are 200,000.00 under both readings, so A4 and A14 pass under either.

## 9. Stop conditions

Stop the slice and report if any of these happens:

- A TIN-shaped string appears in any output, log, title, prop, link or store file.
- `bir_form_send` is called, by anyone, for any reason.
- A Books value is labelled `user`, or `from_card` is sent.
- The live BIR database checksum changes.
- Any money value is computed with a float.
- A `bir_form_fields` read-back differs from `fill_args`.
- A refusal is handled by branching on its code.

## 10. Evidence to keep

| Evidence | Where | Redaction |
|---|---|---|
| Conformance JSON from `rox-plugin-test --json` | Run folder (not the repo) | None needed. No TINs. |
| Golden test output | `cargo test` logs | None needed |
| Privacy scan report | Run folder | Matches shown as hashes only |
| Transcript of Slice B | Run folder | `tkt_` values and tokens replaced with placeholders |
| Audit log extract | Run folder | Counts only |
| Store checks (`strings`, wrong-key result) | Run folder | None needed |

Nothing from a run goes into the repo, except the decision record and the updated status in [07](07-decisions-and-blockers.md).

## 11. Known limits of this slice

- Only one form's inputs (2551Q), one quarter (2026-Q3), one tax year. The election, applicability, ATC and tax are Forms'.
- Synthetic numbers only. The numbers prove the mechanics, not a filing.
- Which sales measure feeds row 1 (net or gross) is a mapping question for tax review (former D20). The slice uses net sales.
- Slice B cannot run on the live BIR install without touching real data. It needs the scratch setup in section 5, and that setup is itself UNKNOWN until tried (B2 in 07).
- The void reading (Q-1) must be chosen before P1 exit.
- Nothing here proves any compliance rule (the 8% case, the 1% extension, the rate, the deadline): those are Forms'. The 1601-C employee-share measure is proved in the PoC, not in this slice.
