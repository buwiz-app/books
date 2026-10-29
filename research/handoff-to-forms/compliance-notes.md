# Handed to Forms: compliance notes from the Books research

Date: 2026-10-29. Basis: owner correction (`../traces/decisions.md`, addendum 2026-10-29) and [ADR 0007](../../docs/adr/0007-books-supplies-measures-forms-owns-compliance.md).

Books no longer decides anything in this file. It is reference material for the Buwiz Forms / BIR mini app owner and the tax reviewer. Books code does not load anything in this folder.

## Files here

| File | What it is |
|---|---|
| [`rates-reference.v1.json`](rates-reference.v1.json) | The former `data/rules/rates.v1.json`, unchanged except a `handoff_note`: PT010 rate rows (1% 2020-07-01 to 2023-06-30; 3% from 2023-07-01; the disputed 1% extension to 2025-12-31, stored OFF), the VAT threshold, the 8% parameters, the 2023 graduated table, the OSD fraction, rounding notes and due-date rules, each with citations and status. |
| [`rules-r1/`](rules-r1/) | The r1 rule sets, attribute catalogue and coverage matrix as they stood before ADR 0007. The r1 per-form files hold the formulas Books used for computed boxes (tax due, totals, OSD, graduated tax, penalties left to the user). `coverage-matrix.r1.json` is the taxpayer-type × election × form applicability matrix. |

## 1. Decisions handed over (were Books decisions in `docs/07` §2)

| Former ID | Question | Evidence carried over | Status |
|---|---|---|---|
| D5 | 2551Q in an 8% year: block, NIL PT010 row each quarter, or no 2551Q | RMO 23-2018 pt 9(a)-(e) says 8% electors do not file 2551Q (`../traces/bir.md` 8-6; `../traces/verify.md` §2 #73). buwiz-oss `crates/bir-core/src/forms/form_2551q.rs:2202-2228` requires NIL PT010 rows (`verify.md` §2 #84). BK-TS blocks (`src/lib/tax/deadlines.ts:122`; `percentage-tax.ts:354-360`). RR 8-2018 full text not read (BIR CDN HTTP 403, `bir.md` §6). | CONFLICT CONFIRMED; resolution UNCONFIRMED. Forms decides. |
| D6 | Source for the claimed 1% PT extension to 2025-12-31 | Only commentary (respicio.ph); no enacting law found (`verify.md` §2 #63; `bir.md` PT-2, §4). CONFIRMED rows: RA 11534 s13 proviso (1%, 2020-07-01 to 2023-06-30); RMC 69-2023 (3% from 2023-07-01) (`verify.md` §2 #62). | Claim UNCONFIRMED. Stored OFF in `rates-reference.v1.json`. |
| D7 | Rounding of tax amounts (half away from zero, once, at output) | The live Forms cents helper is `cents(value: f64) -> i64 { (value * 100.0).round() as i64 }` (`form_2551q.rs:1467-1469`, live tree). Golden ties: 1.50 at 3% → 0.05; 1.49 → 0.04; −1.50 → −0.05. Python `round()` is banker's and would give 0.04. An f64 input can move a tie (R-11). | Forms owns it. Books measures are exact centavos and never rounded. |
| D14 | Overdue Q3 2026 2551Q (due 2026-10-25): show overdue, compute penalties | Workbook 2551Q II.23 "Do not invent penalties"; Forms computes penalties when a deadline exists (`verify.md` §2 item 9). | Forms owns due dates, overdue status and penalties. |
| D4 (tax part) | PT timing basis for 2551Q (entry, receipt or invoice date); treatment of returns and non-operating income | `bir.md` PT-3, PT-8. | The revenue-recognition date is an accounting policy and stays with Books (D4 in `docs/07`). Whether the tax uses another date is Forms'. |
| D20 (now a mapping question) | Which measure feeds 2551Q `txtATCAmt1`: net sales or gross sales | Returns, allowances and discounts are separate Books measures. No printed 2551Q deduction for returns was verified (`bir.md` PT-8, UNVERIFIED). | Mapping `2551Q.S1.1.base` uses `net_sales`, `review.status: open`, for tax review. |

## 2. Forms bugs and asks found during the Books research

| # | Finding | Evidence | Ask |
|---|---|---|---|
| F-1 | 1701-MS item 12 (`frm1701MS:txtTaxpayerNo12` group), item 17 (`frm1701MS:txtTaxpayerNo17`) and `taxCodeDropdown` II015-II017 are **not locked** against the profile election. Nothing compares them with the yearly election. | `../traces/verify.md` §1 item 3, §2 item 10; `data/crosswalk/1701ms-v2024.json` notes; former blocker B7 | Lock them in `draft_locks.rs` to the tax-profile election. The Books bridge reads `/taxpayer/tax_option` and `/taxpayer/deduction` from the draft, so an unlocked, wrong value would route Books' sales to the wrong box. |
| F-2 | The cents helper is f64 at the input (`form_2551q.rs:1467`). | `docs/01` §9.1 | State the rounding rule for the PT line and keep a golden tie test (1.50 at 3% → 0.05). Consider an integer path. |
| F-3 | 2551Q item 13 "Not applicable" (`taxRateNA`) exists in the pin; the printed form read lists two options. | `verify.md` §2 #7 | Confirm the printed option. |
| F-4 | Typed `compromise` writes the root field and is overwritten by the penalty recompute; official `txt22` is the durable input. | `verify.md` §2 #9 | Penalty inputs are user/Forms only. |
| F-5 | Whether `bir_form_fields` echoes computed item 14. | `docs/07` C15 | Needed only for Forms' own read-back. |

## 3. Rules removed from `data/rules/common.rules.json` (common-v1 → common-v2)

The r1 text is in [`rules-r1/common.rules.r1.json`](rules-r1/common.rules.r1.json).

| Rule | Formula (r1, abridged) | Status and citations |
|---|---|---|
| GATE.VAT_THRESHOLD | Block the 8% option and the PT return when posted sales YTD are strictly above 3,000,000.00; force VAT status. | CONFIRMED (RA 11976; `verify.md` §2 #65; `bir.md` PT-7). CPI-adjusted figure UNKNOWN. BK-TS `percentage-tax.ts:106` (strict >); defined twice (`percentage-tax.ts:50`, `vat.ts:70`). |
| GATE.8PCT_YEAR_2551Q | In an 8% year, produce no 2551Q values; no NIL row in MVP. | Conflict CONFIRMED, resolution UNCONFIRMED (see D5). |
| GATE.MIXED_INCOME_8PCT | Mixed earners get no 250,000 reduction on the 8% component. | CONFIRMED (RMO 23-2018 pts 3, 10-12; `verify.md` §2 #75; `bir.md` 8-8). |
| GATE.OVERDUE | Show overdue after the due date; compute no surcharge or interest. | Former owner default D14. |
| ELIG.PT_116 | 2551Q applies to Section 116 persons; other PT ATCs need their own row. | CONFIRMED (RA 11534 s13 / RA 11976 s24; `bir.md` PT-1). |
| ELIG.MICRO_SMALL_TIERS | 1701-MS tiers by gross sales (micro below 3M; small 3M to below 20M). | CONFIRMED (RR 8-2024 s2; `bir.md` M-2, M-3); lawphil numerals off by 1,000 (`verify.md` §2 #71). |
| ELIG.ESTATE_TRUST | Trust or estate in business is in 1701-MS scope in named cases. | CONFIRMED (RMC 34-2025 Annex A item 3; `verify.md` §2 #60). |
| ELIG.COR_NOT_METHOD | A COR or form list does not choose the method (8%, OSD, itemized). | CONFIRMED (workbook Build notes row 17). Books keeps the spirit: the registered-forms list is only a category-suggestion hint. |
| ELIG.ANNEX_C_TY2025 | RMC 20-2026 Annex C is TY2025; TY2026 applicability UNKNOWN. | UNCONFIRMED (`verify.md` §2 #76, #80). |

## 4. Other compliance items moved out of Books

| Item | Evidence | Where it was |
|---|---|---|
| PT010 rate rows, the 1% history and the lookup rule "windows apply to the filing period" | `rates-reference.v1.json` | `data/rules/rates.v1.json`; `docs/07` DD-9 |
| No PT rate row before 2020-07-01 (a 2019 quarter failed closed) | former Q-7 | `docs/07` §2.1 |
| 8% base includes other non-operating income; BK-TS omits it | `verify.md` §2 #36 | `rates.v1.json` eight_percent.books_gap |
| 8% elector who breaches 3,000,000.00 mid-year (graduated from the start of the year; PT until VAT liability) | `bir.md` 8-7, §5; mechanics UNKNOWN | coverage case C07 |
| OSD base: printed Part I item 17 says 40% of gross sales; Part IV item 11 and the code say 40% of net sales | `bir.md` §2; former correction C24 | `rates.v1.json` osd; PoC `DISC-OSD-BASE-1701MS` |
| 1701-MS whole-peso ("49 centavos") rule | UNCONFIRMED, image only (`verify.md` §2 #83) | former correction C20 |
| 1701-MS 15 October second installment | UNCONFIRMED (`verify.md` §2 #83) | former correction C21 |
| 1601-C printed due-date text (10th of the next month; 15 January for December; eFPS groups) | Code CONFIRMED (BK-TS `form-1601c.ts:178-199`); printed text UNCONFIRMED (`verify.md` §2 #83) | former correction C22 |
| 2551Q due date (25 days after quarter end) and 1701Q dates | CONFIRMED (RA 11976 s128(1); 1701Q guide Jan 2018) | `rates.v1.json` due_dates |
| 1601-C item 23 (taxable compensation not subject to withholding; a 250,000 test) | Legal basis UNVERIFIED (`bir.md` 3.4) | rule 1601C.II.23 |
| 1601-C item 20 (other non-taxable compensation) | A tax classification | rule 1601C.II.20 |
| ATC selection (2551Q `drpATC1-6` / `schedule_1.N.atc`; 1601-C `txtATC` WW010) | Crosswalk now `never_fill`, source `forms` | rules 2551Q.S1.N.ATC, 1601C.H.5 |
| 1701-MS IV.10A allowable itemized deductions | Deductibility needs a judgement; Books offers the measure `operating_expenses_by_category` | rule 1701MS.IV.10A |
| Applicability matrix (taxpayer type × registration or election × form) | `rules-r1/coverage-matrix.r1.json` | `data/coverage-matrix.v1.json` v1 content |

## 5. Open for the Forms owner

1. **1601-C item 17, the one input-level tax rule in the bridge.** Item 17 asks for the NON-TAXABLE part of 13th month pay and other benefits (annual cap ₱90,000.00 per employee; NIRC s32(B)(7)(e) as amended by TRAIN s9; RR 11-2018 s2.78.1(B)(11); `bir.md` 3.4). It needs year-to-date payroll per employee, which only Books has. Today the bridge applies the cap (`data/rules/1601c-v2018.rules.json` `1601C.II.17`, `input_level_tax_rule`). Options: (a) Forms applies the cap, and Books sends the gross paid amount and the year-to-date paid amount through new Forms inputs; (b) the bridge keeps it. The cap may be raised by the Finance Secretary, so under (b) the value would need a dated source owned by Forms.
2. Confirm the `document` fill-source label for Books values (D3).
3. Lock the 1701-MS election inputs (F-1) before any 1701-MS fill.

## 6. Mapping questions for tax review (Books side, listed for the reviewer)

| Mapping | Question |
|---|---|
| `2551Q.S1.1.base` | Net or gross sales for Schedule 1 row 1 (former D20). |
| `2551Q.II.15` | Does the percentage-tax 2307 credit go only to item 15? (`bir.md` W-3, INFERENCE) |
| `1701MS.IV.13`, `1701MS.IV.20` | Is income final-taxed at source excluded? (`EXC.FINAL_TAX_INTEREST`; final-tax fact UNKNOWN in BK-TS, `sheets.md` §10) |
| `1601C.II.16` | The measure exists for all employees; item 16 is MWE-only. The per-employee MWE split is planned (P6). |
| `1601C.II.19` | RR 11-2018 CONFIRMED; statute text UNVERIFIED (`verify.md` §2 #69). |
