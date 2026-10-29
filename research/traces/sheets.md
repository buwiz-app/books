# Track report: Buwiz research spreadsheets (audit)

Labels: **CONFIRMED** = seen in a file, cell or code line. **PARTIAL** = partly seen, or depends on an unverified source. **MISSING** = searched, not found (method given). **UNKNOWN** = not verifiable in this track (method given). INFERENCE marks reasoning beyond observed text.

## 0. Scope, method, repo HEADs

- Inputs (xlsx read directly with a stdlib parser, so every cell reference below is a sheet coordinate). The CSV dumps under `work/csv` drop blank rows, so CSV line numbers are not sheet rows.
  - `/private/tmp/claude-501/-Volumes-goldcoders-OSS-buwiz-books/d723bb20-1674-49c1-8fb6-568f056d2c7e/scratchpad/inputs/Buwiz_Tax_Profiles_Categories_Form_Fields.xlsx`: 10 sheets, all read.
  - `.../scratchpad/inputs/Philippine_Bookkeeping_and_BIR_Form_Map.xlsx`: 5 sheets, all read.
- Helper scripts, all in `.../scratchpad/audit/`: `xlsx_cells.py`, `recompute.py`, `eight_check.py`, `rowfind.py`, `bindings_check.py`.
- Repo HEADs (read-only):
  - **buwiz-books** `/Volumes/goldcoders/OSS/buwiz-books`: `1662ca37404ccc6c270ad48888eda3505ec45165`. Equals the workbook's snapshot pin (CONFIRMED). Only `skills-lock.json` is modified; the cited src paths are clean.
  - **buwiz-forms** `/Volumes/goldcoders/OSS/buwiz-forms`: `301f9a0f22f3e9635f2aecb07107b43fe65140c9` (2026-10-15). This is not the handover pin `63267ca3d4d526a8ec490394470f026de82ac385` (2026-10-24), which exists locally but is not checked out. Pin-to-HEAD diff: `2551q-v2018.json` 16 lines, `1701ms-v2024.json` 102 lines, `1601c-v2018.json` identical (CONFIRMED). I did not reconcile the pin's contents, so schema findings below are for HEAD unless marked. Agent-forms working tree is clean.
  - **buwiz-oss** `/Volumes/goldcoders/buwiz-oss`: `0f3410693b078e2ce90c61719d5b167d06ce7242` (2026-10-29). Agent-forms JSON clean. Uncommitted edits exist in `crates/bir-core/src/forms/form_2551q.rs`, `form_2551q_xml.rs` and `form_1601c_xml.rs`; citations into `form_2551q.rs` reflect the working tree.
  - Not read in this track: buwiz-server, rox.

## 1. Structure

### 1.1 Buwiz_Tax_Profiles_Categories_Form_Fields.xlsx

- **Profile** (A1:G29). Inputs: C6 filer type (DV: Individual / Corporation / Partnership / Estate/Trust) = Individual; C7 2551Q = Yes; C8 1601-C = Yes; C9 1701-MS = Yes; C10 regime (DV: Graduated / 8% / Special/Exempt / Unknown) = Graduated; C11 deduction (DV: OSD / Itemized / Not applicable / Unknown) = OSD; C12 "Filer earns compensation" = No; C13–C15 reference only. G6:G10 switches: 1701Q Yes, 1702 group No, 2550Q No, 0619-E/1601-EQ No, 1701/1701A No. Review formulas in A18:A21.
  - CONFIRMED by reading every formula: C11, C12 and C13–C15 are referenced by no formula. The Category and Form catalogs reference only C6–C10 and G6–G10.
- **Category catalog** (rows 6–64, 59 categories). Key in A, account number in D, origin in E, activation in F, selection formula in G. All 59 selection formulas share one template (CONFIRMED). Origins: 33 Existing base, 18 Existing PH control, 8 Proposed. IDs are listed in §4.
- **Category field rules** (rows 6–57): key → form → target field, with method, period, filter and exclusion.
- **2551Q fields** (rows 6–76, 71 rows), **1601-C fields** (rows 6–83, 78 rows), **1701-MS fields** (rows 6–100, 95 rows). Sections: H (header), I (identity), II (computation), III (payment rows), DECL, RECEIPT. Schedule 1 adjustment rows for 1601-C; ATC rows S1.1–S1.6 plus S1.7 for 2551Q; Part IV (IV.1–IV.25 including IV.10A–D and IV.18A/B), Part V (V.1–V.10) and VI.1 for 1701-MS.
- **Worked example** (C6:C39): 2551Q in C6–C12; 1601-C in C14–C25; 1701-MS in C27–C39.
- **Example data**: transactions in rows 6–12 (INV-Q1 to INV-Q4 are Sale; CN-Q3 is Return; EXP-Q3; PAY-Q3). Employee E-001 in row 17 (D17 40,000; E17 2,000; F17 2,000; G17 0; H17 2,808.4). Inputs B20 40,400; B21 0; B22 0.03; B23 0.4.
- **Form catalog** (rows 6–31, 26 forms). Col G is the entity-screen formula and col H the selection formula. Both reference Profile C6–C10 and G6–G10. Forms hard-coded "Not selected here" include 1700, which ignores Profile!C12.
- **Build notes and sources** (rows 2–42): design rules (5–12), coverage (13–16), review findings (17–22), rounding (23–24), current rule (25), reconciliation (26), sources S1–S7 (27–33), repo links R1–R9 (34–42). The snapshot pin matches buwiz-books HEAD (CONFIRMED).

### 1.2 Philippine_Bookkeeping_and_BIR_Form_Map.xlsx

- **8 percent example** (B7:E14 formulas). Sales YTD (row 7); other income YTD (row 8); reduction (row 9 = $B$17); 8% base (row 10); tax YTD (row 11 = base × $B$18); CWT YTD (row 12); previous payments (row 13); amount still payable (row 14). Inputs: B17 = 250,000, B18 = 0.08. Transaction rows 21–25; col H "Net cash" feeds no tax line.
- **Account map** (header row 4, rows 5–42), **Filer combinations** (header row 4, rows 5–25), **Forms and timing** (header row 4, rows 5–30; timing text in col D), **Sources** (header row 4, rows 5–21; S1–S17).

## 2. Recomputation

Scripts: `recompute.py` and `eight_check.py`.

- **Worked example**: 22 cached cells recomputed from the Example data inputs. All match, so the cache is not stale. Values: C7 290,000; C9 8,700; C12 8,700; C14 42,000; C17 4,000; C18 38,000; C20 38,000; C25 2,808.4; C27 1,200,000; C28 10,000; C29 1,190,000; C30 476,000; C32 714,000; C33 85,300; C34 40,400; C35 8,900; C36 3,000; C37 52,300; C39 33,000. Linked-only cells (C10, C11, C15, C16, C19, C21, C23, C31, C38) were verified by reading.
- **Arithmetic errors: none.** Errors are in labels, template formulas and one input (§6).
- **Graduated bracket** (C33): matches the 2023+ table (CONFIRMED). Boundary values are 0 at 250,000; 22,500 at 400,000; 102,500 at 800,000; 402,500 at 2,000,000; 2,202,500 at 8,000,000. Same table as `form_1701ms.rs:293–305`.
- **1601-C monthly withholding** (INFERENCE on method): 38,000 × 12 = 456,000; tax = 22,500 + 20% × 56,000 = 33,700; ÷ 12 = 2,808.33. The workbook's 2,808.4 is 0.067 high. Payroll method not verifiable (PARTIAL).
- **8% example**: 24 cells recomputed, all match. Sales YTD 300k / 600k / 900k / 1.2M; base 50k / 350k / 650k / 950k; tax 4k / 28k / 52k / 76k; CWT YTD 3k / 6k / 9k / 12k; previous payments 0 / 1k / 22k / 43k; payable 1k / 21k / 21k / 21k.
- **Cross-reconciliation** (CONFIRMED): 8% annual column ties to 1701-MS: V.2 = 43,000, V.3 = 9,000, V.4 = 3,000, II.25 = 76,000 − 55,000 = 21,000.

## 3. Field tabs vs Forms schemas

Schemas: `bir-core/data/agent-forms/{2551q-v2018, 1601c-v2018, 1701ms-v2024}.json` in buwiz-forms (HEAD) and buwiz-oss. The 1601c files are identical in both repos. The 2551q and 1701ms files differ: buwiz-oss adds bindings and fixes defects noted below. Engine key map: `buwiz-oss/crates/bir-core/src/forms/form_1701ms.rs:543–641`.

Status categories: MATCH; MATCH-unbound (key exists, no draft binding); MISSING (no schema key); LABEL-ONLY (no item number, or label differs); WRONG-ID; KEY-NAME defect (ID right, JSON key wrong).

| Form | Rows | MATCH | MATCH-unbound | MISSING | LABEL-ONLY | Defects |
|---|---|---|---|---|---|---|
| 2551Q | 71 | 51 | 17 | 2 | 1 | 2 forms-side bindings (below) |
| 1601-C | 78 | 56 | 16 | 3 | 2 | 1 path-name defect (II.19) |
| 1701-MS | 95 | 67 | 15 | 3 | 4 | 1 WRONG-ID (II.35), 4 KEY-NAME, 1 forms-side binding (I.9) |

**2551Q**
- MATCH on items 1–24, the Schedule 1 rows and the overpayment radio. Examples: II.15 → `/creditable_tax_withheld`; I.13 → `/item_13_election`; S1.1.base → `/schedule_1/0/taxable_amount`. Sheet rows: I.13 row 20, II.14 row 21, II.18 row 25, II.19 row 26, II.24 row 31, III.28.bank row 45, DECL.signature row 49, S1.1.rate row 54.
- Forms-side defect, buwiz-forms only: items 20 and 21 (surcharge, interest) are unbound. The unmapped notes say they show "/surcharge combined with /creditable_tax_withheld" and "/interest combined with /creditable_tax_withheld". buwiz-oss binds them to `/penalty_overrides/surcharge` and `/interest`.
- I.13: buwiz-oss adds option "Not applicable" (`taxRateNA` → `not_applicable`). The sheet lacks it. `form_2551q.rs:1867` and `:1875` restrict it to a PT010 initial-quarter return.
- III.28 "Particular 28" (`txtParticular28`) exists in the schema and has no sheet row.
- MISSING: DECL.signature (no signer box; agent fields `txtTaxAgentNo`, `txtDateIssue`, `txtDateExpiry` exist but are unbound) and RECEIPT (evidence, no box).
- LABEL-ONLY: I.9A `zipCode` has no item number.

**1601-C**
- MATCH on items 1–36. H.5 ATC `/atc` holds WW010, which is the preprinted ATC (`buwiz-oss/crates/bir-print/assets/html-frozen/1601c-2018/writer-cells.json:620`) and the official default (`buwiz-oss/crates/bir-core/data/official-xml/1601c-v2018.json:80`).
- III.39.bank "Tax debit memo: Drawee bank / agency" (row 52): MISSING. Item 39 has only `txtNumber39`, `txtDate39`, `txtAmount39`.
- III.40 "Particular 40" (`txtParticular40`) has no sheet row (III.40.bank is row 56).
- II.19 (row 26): ID correct. Schema path `/tax_19_sss_gsis` is narrower than the printed label "Mandatory contributions and union dues". The sheet content (employee shares) is right.
- I.12 email has no item number (LABEL-ONLY). I.13 label "Payees with relief…" differs from schema label "Special tax rate", though binding `/tax_relief` fits the sheet's meaning (LABEL-ONLY).
- S1.1.1 "Previous month" (`txtMonthYear0`) is unbound, and the JSON note says the app re-sorts or recomputes it.

**1701-MS**
- MATCH on Part IV and Part V IDs. The engine maps each sheet ID to the same printed-item key, e.g. IV.5 `txtSalesRevenueNo5a`, IV.6 `txtSalesReturnsNo6a`, IV.11 `txtOptionalDeductionNo11a`, IV.13 `txtNonOperatingIncomeNo13a`, IV.15 `txtCompensationBusinessNo15a` (→ `taxable_income`), IV.18B `TotalTaxDueCompensationBTP`, IV.20 `txtOtherNonOperatingIncomeNo20a`, V.1–V.9 per item (CONFIRMED, `form_1701ms.rs:543–641`).
- KEY-NAME defect: IV.7 (row 67, net sales) is bound in the engine (`form_1701ms.rs:566–569`) to the key `txtTaxableCompensationIncomeNo7a`, whose name says taxable compensation. Engine and workbook both treat line 7 as net sales. The sheet is right; the JSON key name is wrong.
- Other key-name defects: II.28A `txtSurchangeNo28a`; IV.10D `txtTotalAllowableItemixedDeductionsNo10a`; IV.14 `txtTaxableIncomeBusinesNo14a`; spouse IV.10A key `txtAllowableItemizedDeductionsNo10aSpouse` breaks the A/B pattern.
- WRONG-ID: II.35 "Number of attachments" (row 43). The schema places attachments at Item 33 (`txtPg1I33NumberOfAttachments`, constant-written). Sheet II.33 (row 41, name and signature) maps to `printedName` / `authorizeRepresentative` with no item number (buwiz-oss → `/signatory_name`, `/authorized_representative`).
- MISSING: II.34 TIN of authorized representative (row 42). Searched all keys for "TIN" and "Representative" in both repos; none found.
- MISSING: III.38.bank tax-debit-memo drawee (row 52). Keys are `txtTaxDebitMemoNumber/Date/Amount` only.
- III.39 "Other particulars" (`txtOtherParticulars`) has no sheet row.
- Forms-side binding defect (buwiz-forms only, I.9 row 14): unmapped note says the email box shows "/email combined with /foreign_tax_credits_description". Fixed in buwiz-oss.
- Coupled bindings (buwiz-forms only, output of `bindings_check.py`, CONFIRMED): `txtToBeRefunded`, `txtToBeIssued` and `txtToBeCarried` (item 31) each also set `/taxpayer/tax_option`. `txtTaxpayerNo17`, `17a` and `17b` (item 17) also set `/taxpayer/source` and `/taxpayer/tax_option`. A single click can change the regime. buwiz-oss binds each to one path. `taxCodeDropdown` (I.18) sets source and tax_option in both repos, which looks intended.
- I.18 engine ATC values `II011`–`II017` (`form_1701ms.rs:414–422`) are not listed in the workbook.
- H.4 (civil status, row 8) and H.1 label "For the year (MM/YYYY)" are LABEL-ONLY.

PDF status: the printed Aug 2024 item numbers are inferred from schema and engine. The S3 PDF was not fetched (UNKNOWN).

## 4. Category catalog IDs and Books COA cross-check

**All 59 IDs.**
- Existing base (33): bank_accounts, accounts_receivable_gross, inventory, prepaid_expenses, computer_assets, accounts_payable, owners_equity, sales_revenue, subscription_revenue, transaction_revenue, refunds_and_discounts, interest_income, cost_of_goods_sold, salaries, employee_benefits, retirement_contributions, employer_payroll_taxes, payroll_processing_fees, payment_processing_fees, hosting_fees, consultants_and_contractors, legal, tax_and_accounting, advertising, business_applications_and_software, rent, utilities, repairs_and_maintenance, office_supplies, airfare, meals, bank_fees, interest_expense.
- Proposed (8): owner_drawings, loan_payable, other_non_operating_income, direct_service_cost, depreciation_expense, pct_tax_credit_receivable, income_tax_paid, income_tax_payable.
- Existing PH control (18): ph_cwt_receivable, ph_percentage_tax_payable, ph_percentage_tax_expense, ph_wtc_payable, ph_sss_payable, ph_philhealth_payable, ph_pagibig_payable, ph_union_dues_payable, ph_net_pay_payable, ph_employee_tax_refund_payable, ph_ewt_payable, ph_input_vat, ph_input_vat_unpaid, ph_output_vat, ph_output_vat_uncollected, ph_vat_payable_net, ph_creditable_vat_withheld, ph_input_vat_exempt_expense.

**Results**
- **REUSE (51 of 59)**: all 33 base keys and numbers match `buwiz-books/src/lib/coa/presets/base.ts` (`BASE_ACCOUNTS`). All 18 PH keys and numbers match `src/lib/tax/ph-chart.ts` `PH_CHART` (lines 68–262), and all numbers sit in `PH_RESERVED_BANDS` (lines 51–60). No duplicate numbers (CONFIRMED).
- **NEW (8)**: no key or number exists in base.ts or ph-chart.ts (MISSING, searched base.ts keys and subtypes). Reuse candidates to decide: `owner_drawings` → `owners_equity` 32000 (base.ts description: "distributions to owners"); `other_non_operating_income` and `direct_service_cost` → parents `other_income` 80000 and `cost_of_revenue` 50000, still needing child accounts. `depreciation_expense`, `income_tax_paid`, `income_tax_payable`, `loan_payable` and `pct_tax_credit_receivable` have no Books equivalent. Keep `pct_tax_credit_receivable` separate from income-tax `ph_cwt_receivable`.
- **RENAME (labels, not keys)**: Category catalog column C uses display labels. The Books enum is in `src/db/schema/account-constants.ts:9–18`: asset, liability, equity, revenue, expense, cost_of_revenue, other_income, other_expense. "contra revenue" has no enum (Books: revenue + subtype `refunds_discounts`). "cost" → cost_of_revenue; "other income" → other_income; "other expense" → other_expense.
- **MISSING from catalog** but present in ph-chart.ts: `ph_employee_advances` 12700 (lines 87–95), `ph_employee_tax_advanced` 12800 (96–104), `ph_unrecovered_employee_tax` 61950 (235–243). The last one matters for 1601-C reconciliation.
- **Doc drift**: `philippines-smb.ts:2` and `:19` say "20 frozen" accounts; PH_CHART has 21 entries (CONFIRMED by count).
- **Conceptual**: base.ts `taxes` (94000, subtype taxes) is described as "Income taxes, franchise taxes, and other governmental tax obligations". Income tax and deductible business tax share one subtype (CONFIRMED text).

## 5. Rates, law and source status

| Claim | Status | Evidence |
|---|---|---|
| PT010 is 3% for Q3 2026 | CONFIRMED in code; PARTIAL externally | Books `percentage-tax.ts:65–68` (`percentageTaxRateBps`): 1% only for 2020-07-01 to 2023-06-30, else 3%. buwiz-oss `forms/atc.rs:140–165` (`resolve_2551q_atc_rate`) same window. `official-xml/2551q-v2018-atc-options.json` lists PT010 at 3.0 (value 1) and 1.0 (value 23). Secondary web source (respicio.ph, fetched) says 3% from 2026-01-01 absent new law. |
| The 1% window ended 30 Jun 2023 | CONFIRMED in code | Books and buwiz-oss both hard-code this end date. |
| The 1% window was extended to 31 Dec 2025 by an unnamed later law | UNVERIFIED | Only the respicio.ph commentary says this; it names no law and carries its own "may be inaccurate" disclaimer. Enacted text not found. If true, PT010 periods 2023-07-01 to 2025-12-31 are mis-rated in both code bases. Q3 2026 is unaffected. |
| Graduated 2023+ brackets | CONFIRMED in code | `form_1701ms.rs:293–305`, `form_1701q.rs:919–936`, `form_1701q_official.rs:75–103`. The 2018–2022 table (20/25/30/32/35) is year-gated in `form_1701a.rs:364–372` and not used here. |
| OSD = 40% of net sales (sales − returns) | CONFIRMED in code | `form_1701a.rs:470–471`. Printed 1701-MS line 11 wording not checked (UNKNOWN). |
| 8% = 8% × (gross + other non-operating income − 250,000); 250,000 only for purely self-employed | CONFIRMED in code | `percentage-tax.ts:53, :55, :167–190` (`computeEightPercent`); `form_1701ms.rs:44–45`. |
| 8% base includes other non-operating income | CONFIRMED (workbook and buwiz-oss); Books omits it | Workbook: Account map!D9 and the 8% example's row 8. buwiz-oss: `form_1701ms.rs:49` (3M test) and IV.20 mapping. Books: `RegimeInput` (`percentage-tax.ts:80–91`) and `computeEightPercent` take `grossReceipts` only. Statute text not fetched, so legal basis is PARTIAL. |
| VAT threshold 3,000,000 | CONFIRMED | `percentage-tax.ts:50`; `form_1701ms.rs:46–47`. |
| 1701-MS: single business income-tax rate (graduated, 8%, special or exempt); includes mixed-income earners; individual business taxpayers | CONFIRMED against RMC 20-2026 Annex C, pages 1–2, read in full | Page 1 A5 table row 3; page 2 A7 ("Individual business taxpayers classified as Micro and Small"). Forms and timing!C10 wording is consistent. |
| Form catalog!C10 scope "Individual/Fiduciary" | CONFIRMED mismatch | Annex C A7 says individual business taxpayers only. |
| 1701-MS micro/small thresholds, spouse limits, special-rate limits | MISSING | Searched Annex C pages 1–2 (the whole document; 2 pages). WebSearch returned no Annex text. |
| Annex C applies to TY2026 | UNKNOWN; apparently not | Annex C title: "…ON FILING THE 2025 ANNUAL INCOME TAX RETURN". The 1701-MS Aug 2024 edition for TY2026 is UNKNOWN. |
| 1701Q due May 15 / Aug 15 / Nov 15 | CONFIRMED in code; not fetched | `buwiz-oss/crates/bir-core/src/calendar_rules.rs:1004–1007`, sourced in the code comment to the BIR 1701Q Jan 2018 guide. |
| 2550Q / 2551Q due quarter-end + 25 days | CONFIRMED in code | Books `deadlines.ts:68`, `:112–122`; `percentage-tax.ts:327`. |
| 1702Q due +60 days | CONFIRMED in code | `calendar_rules.rs:1041`. |
| 1604-C and 2316 due 31 Jan | CONFIRMED in code | Books `deadlines.ts` 1604C and 2316 entries (about lines 128–142). |
| 1601-C: December → 15 Jan regardless of channel; eFPS group days A15/B14/C13/D12/E11 | CONFIRMED in code | `deadlines.ts:86`; `form-1601c.ts:41`. |
| 2307 within 20 days after quarter; 1604-E 1 Mar; 1601-EQ last day; SAWT; 1706/1707/1800/1801; 2000 | UNKNOWN | Not checked against code or web. |
| 1701-MS whole-peso rounding ("49 centavos down") | UNKNOWN | Build notes row 23 cites S3 PDF; not in Annex C. |
| RR 6-2026 / RMC 112-2026 invoice CWT rule | UNKNOWN | Not fetched. |
| Other Sources S1–S17 (except S3) | UNKNOWN | Not fetched. |
| "1606" is a CWT certificate (Category field rules rows 52–53) | UNKNOWN | `1606-v2018.json` exists (form_code 1606) but its role is not established from the schema. Treat as a review item. |
| 2551Q in an 8% year: NIL or not filed | Conflict, unresolved | See §7. Needs RR 8-2018 text, not fetched. |

## 6. Corrections

Format: sheet!cell → current → corrected → evidence. The workbook path is in §0.

**Worked example and inputs**
1. Worked example!A12:B12 (C12 = C9−C11 = 8,700), labelled "2551Q II.24 Amount payable" → relabel as II.19 "Tax still payable" (II.14 − II.18). Add II.14 (= S1.7) and II.24 (= II.19 + II.23 = 8,700). Evidence: 2551Q fields!B26 (II.19), B31 (II.24); `2551q-v2018.json` items 19 and 24.
2. Worked example!C11 (= C10), labelled II.18 → must be II.15 + II.16 + II.17 (fields!B25). Value unchanged (0).
3. Worked example!A25:B25 (C25 = C23−C24 = 2,808.4), labelled "1601-C II.36" → II.31 "Tax still due / over-remittance" = II.27 − II.30 (1601-C fields!B38). Add II.36 = II.31 + II.35 (fields!B43). Evidence: `1601c-v2018.json` items 31 and 36.
4. Worked example!C17 (= C15+C16), labelled II.21 → must sum II.15–II.20 (1601-C fields!B28). Template defect; value unchanged.
5. Worked example!C33 (graduated tax applied to C32 = IV.14) → dictionary applies it to IV.15 = IV.3 + IV.14 (1701-MS fields!B78, B82). Add a row IV.3 (= 0). Value unchanged (85,300).
6. Worked example!C37 (= SUM(C34:C36)), labelled V.10 → must sum V.1–V.9 (1701-MS fields!B99). Value unchanged (52,300).
7. Worked example!A39:B39 (C39 = C33 − C38 = 33,000), labelled "1701-MS II.29" → show the chain II.23 (= II.21 + II.22) → II.25 (= II.23 − II.24; fields!B30) → II.27 (= II.25 − II.26; B32) → II.29 (= II.27 + II.28D; B37). Value unchanged.
8. Example data!H17 (2808.4) → 2,808.33 under the 2023+ annualized method, or document the payroll source. Severity: low.
9. Example data!H8 (INV-Q3 CWT 2,900) = 1% of 290,000 net of CN-Q3. The other three sales show 3,000 (1% of 300,000). Confirm the 2307 amount. If 3,000: Worked example!C35 → 9,000; C37 → 52,400; C39 → 32,900.
10. Example data!A12:F12 (PAY-Q3 "salaries" 42,000) is labelled Q3 but holds one month of payroll (E-001, month 9, row 17). It also includes the 2,000 de minimis (E17), which the Category catalog would place in employee_benefits (row 23). Relabel as September, or add July and August payroll. Period-scope error risk.
11. Worked example!C8 (= Example data!B22) and C30 (= C29 × B23): no change. Both are consistent with code (see §8).

**Profile, Form catalog and Category rules**
12. Form catalog!C10 (1701-MS scope "Individual/Fiduciary") → "Individual" (Annex C A7). Profile!A18 true-branch text "1701-MS is an individual/fiduciary form" → same fix.
13. Profile!A18:A21 (review formulas): add checks for (a) C9 = Yes with C6 = Estate/Trust (out of scope); (b) C9 = Yes and G10 = Yes, since 1701-MS and 1701/1701A are alternative annual returns (Annex C page 1 A4 "any of the following"); (c) C10 = 8% and C12 = Yes (no 250,000 deduction for mixed income); (d) C10 = 8% and C11 ∈ {OSD, Itemized} (deduction method not applicable); (e) C10 = 8% and C7 = Yes (the 2551Q question in §7).
14. Profile!C12 (unused) → wire into Form catalog!A5 (1700 row, hard-coded "Not selected here") and into check (c).
15. '8 percent example'!B14:E14 (= MAX(0,B11−B12−B13)) → clamps an overpayment to 0. The workbook's own rule "Keep sign; no silent clamp" (2551Q fields!F26) requires a signed formula with a separate overpayment line. Value unchanged here.
16. Category field rules!A7:B8 (sales_revenue → 1701-MS IV.5 and IV.19) and A16:A17 (refunds → IV.6 and IV.19): add a regime filter from 1701-MS I.12 (graduated path IV.5–IV.18B vs 8% path IV.19–IV.25). IV.5 currently says only "Filer, non-final income, exclude VAT", so an 8% electee could populate both. PARTIAL (engine behaviour under 8% not checked).
17. Category field rules!A42 (depreciation_expense → IV.10A "Sum tax-eligible expenses"): the source should be the tax-depreciation schedule, not the GL account. Category catalog row 43 (col I) already says "Use fixed asset schedule and tax adjustment".
18. Category field rules!A18:A21 (other_non_operating_income and interest_income → IV.13 / IV.20): the final-tax exclusion needs a tax attribute. UNKNOWN whether Books stores one (not searched).
19. Category catalog!C18 ("contra revenue"), C20 ("other income"), and the "cost" and "other expense" labels → Books enum values (§4).
20. Category catalog!A13 (owner_drawings, Proposed): decide reuse of owners_equity 32000 or keep new.
21. Category catalog: add rows for ph_employee_advances (12700), ph_employee_tax_advanced (12800) and ph_unrecovered_employee_tax (61950), from ph-chart.ts.
22. Forms and timing!D18 (1601-C timing): add "December → 15 January regardless of channel" (deadlines.ts:86) and eFPS group day A15/B14/C13/D12/E11 (form-1601c.ts:41). "Dec Jan 15 for manual" is ambiguous.
23. Filer combinations!F9 ("OSD is generally 40% of individual gross sales") → "40% of net sales (gross less returns, allowances and discounts)" (`form_1701a.rs:470–471`; 1701-MS fields!E74 "40% of IV.7").
24. Build notes and sources!C21 (8% + 2551Q): state that Books blocks 2551Q (`percentage-tax.ts:354–358`) and buwiz-oss requires NIL PT010 rows (`form_2551q.rs:2217–2226`). See §7.
25. Build notes and sources!C16 (1701-MS scope, row 16) and Sources!C8 (S3): add the caveat that Annex C covers the 2025 annual return filed in 2026.
26. Build notes and sources!C20 (8% threshold inputs): add that the Forms engine includes other non-operating income in the 3M test (`form_1701ms.rs:49`) and Books omits it.
27. Build notes and sources!C22 (1701-MS implementation): "ComputedReturnForm lacks 1701-MS" is Books-only. buwiz-oss has `form_1701ms.rs`.

**Field tabs**
28. 2551Q fields!C20 (I.13): add the "Not applicable" option (buwiz-oss `taxRateNA`).
29. 2551Q fields around row 45 (III.28): add a "Particular 28" row (`txtParticular28`).
30. 1601-C fields!B52 (III.39.bank): no drawee key in the schema. Drop the field, or add the key in the Forms repo.
31. 1601-C fields around row 56 (III.40): add "Particular 40" (`txtParticular40`).
32. 1701-MS fields!B43 (II.35): ID should be II.33 per schema Item 33. Renumber II.33/II.34 against the printed form. II.34 has no schema key.
33. 1701-MS fields!B52 (III.38.bank): no tax-debit-memo drawee in the schema. Add a row for III.39 "Other particulars" (`txtOtherParticulars`).
34. 1701-MS fields!B22 (I.18): list the official values II011–II017 (`form_1701ms.rs:414–422`).
35. 1601-C fields!B26 (II.19) and 1701-MS fields!B67 (IV.7): keep the IDs. In the Forms repo, rename `/tax_19_sss_gsis` and `txtTaxableCompensationIncomeNo7a`.

## 7. Cross-repo findings surfaced by this track

- **X1, 2551Q in an 8% year (CONFLICT, UNRESOLVED)**: Books `deadlines.ts:122` says "Do not file if the 8% option was elected", and `percentage-tax.ts:354–358` blocks 2551Q. buwiz-oss `form_2551q.rs:2217–2226` requires PT010 to be a NIL row in every quarter of an 8% year. `form_2551q.rs:3344` reads "A NIL PT010 return is still a Sec. 116 activity return". Three sources disagree. Needs RR 8-2018 text.
- **X2, 1% window end date**: see §5 (UNVERIFIED extension claim).
- **X3, 1601-C item 19 omitted in Books (CONFIRMED)**: `buwiz-books/src/lib/tax/form-1601c.ts:52–109` (`compensationFromPayrollLine`) has no employee-share input. Its only caller is `src/lib/tax/issue-1601c.ts:52`. Payroll stores the shares separately (`src/db/schema/payroll.ts:367–370`). Result: 1601-C II.19 is missing and II.22 is overstated by the employee shares. Matches the Build notes row 18 finding.
- **X4, 8% base omits other income in Books (CONFIRMED)**: see §5.
- **X5, stale comment (CONFIRMED)**: `buwiz-books/src/lib/tax/ph-account-resolver.ts:13` cites 21610 for `ph_wtc_payable`. PH_CHART says 25110 (`ph-chart.ts:170`).
- **X6, 1701-MS absent from Books (MISSING)**: no "1701-MS", "1701MS" or "1701ms" string in buwiz-books `src/**/*.ts`. `src/db/schema/tax-stage-remainder.ts:21` (`ComputedReturnForm`) excludes it. buwiz-oss has `form_1701ms.rs`.
- **X7, forms-side defects at buwiz-forms HEAD (CONFIRMED)**: 2551Q items 20 and 21 unbound; 1701-MS coupled bindings; 1701-MS I.9 email display. Fixed in buwiz-oss JSON.
- **X8, buwiz-oss working tree (CONFIRMED)**: `form_2551q.rs`, `form_2551q_xml.rs` and `form_1601c_xml.rs` are uncommitted edits. Findings citing them reflect the working tree.
- **X9, WW010 catalog gap (CONFIRMED)**: `buwiz-oss/crates/bir-core/data/atcCodes.json:2408` lists WW010 with blank description and rate, although the form uses it as the default.
- **X10, PT010 catalog (CONFIRMED)**: `atcCodes.json` lists PT010 three times (lines 2128 at 3.0, 4893 at 1.0, 5238 blank). The official 2551Q option list (`2551q-v2018-atc-options.json`) carries the two rated entries.

## 8. Conceptual problems

1. **Account is not tax meaning.** `salaries` (61100) feeds 1601-C II.14 (employer withholding return) and 1701-MS IV.10A (owner's itemized deduction), with different legal treatment. `refunds_and_discounts` (49000) reduces the PT base (2551Q S1.1.base), graduated sales (IV.6) and 8% net sales (IV.19). The mapping tab holds these as free-text filters, not machine-readable dimensions. Build notes row 6 says ATC, party, period and regime must be separate attributes. The Books schema was not checked for these attributes (UNKNOWN).
2. **Percentage-tax credits vs income-tax credits.** The same 2307 form can carry either credit. The workbook separates them by tax type (2551Q II.15 vs 1701-MS V.3/V.4), but Books stores all CWT in one control account (`ph_cwt_receivable`, 12600). `pct_tax_credit_receivable` is only proposed. Handover line 133 warns against routing one credit to both forms.
3. **Employer vs taxpayer.** 1601-C is the employer's withholding return. The worked example's employee is not the owner, which is correct. The owner's own compensation (1701-MS IV.1–IV.4, from the owner's 2316) has no Profile or Category path (MISSING). Profile C12 is unwired.
4. **Mixed income and 8%.** Profile is single-valued per filer. The 250,000 rule for mixed income depends on C12, which is unwired. 1701-MS I.12 and IV.22 are per person (A/B), and the Profile cannot represent spouse-level regimes.
5. **Alternatives not enforced.** Profile allows C9 (1701-MS) and G10 (1701/1701A) both Yes. Annex C describes one annual return per filer (CONFIRMED, page 1 A4).
6. **Period scope.** 1601-C is monthly, 2551Q quarterly, 1701Q cumulative and 1701-MS annual. The worked example's Q3 payroll row holds one month (C-10). The worked example lacks due dates. INFERENCE from deadlines.ts logic: Q3 2551Q was due 25 Oct 2026 and the September 1601-C was due around 10–15 Oct 2026. Both had passed as of 2026-10-29.
7. **Missing drawee fields.** 1601-C item 39 and 1701-MS item 38 have no tax-debit-memo drawee key, but the sheets assume one for every payment method (CONFIRMED, §3).
8. **Hard-coded zero.** 1601-C II.23 "Assumed zero" is correct here (annualized taxable 456,000 is above the 250,000 zero bracket; INFERENCE on threshold), but it should be derived.
9. **Template formulas.** Worked-example formulas hard-code partial sums (C11, C17, C37). A copy of the sheet would produce wrong totals once more rows are populated (template defects, corrections 2, 4, 6).
10. **Category activation.** The IT flag cannot distinguish 8% from graduated. `other_non_operating_income` is SUGGESTED under both.

## 9. Fine items

- All 22 + 24 cached worked-example and 8% cells match formulas (§2).
- Graduated 2023+ bracket formula (C33) and the 40% OSD on net sales (C29–C30) match the Forms engine.
- PT010 base 290,000 and 3% → 8,700 are consistent with code for 2026.
- Income-tax CWT is kept out of 2551Q credits (C10 = 0; Example data B21 note).
- 1601-C II.22 = 38,000, II.23 = 0 (INFERENCE), and II.27/II.31 keep sign.
- 1701-MS V.2 (cash payments) and V.3/V.4 (2307 credits) do not double-count. The 8% reconciliation ties to 21,000.
- Sheet numbering of Part IV and V IDs equals the engine's printed-item keys (`form_1701ms.rs:543–641`).
- 2551Q items 1–24 and 1601-C items 14–36 match schema item numbers.
- All Category selection formulas share one template. The 51 reused account numbers match Books exactly and sit in reserved PH bands.
- Profile cell references are correct: C6 filer, C7 2551Q, C8 1601-C, C9 1701-MS, C10 regime, G6 1701Q, G7 1702 group, G8 2550Q, G9 0619-E/1601-EQ, G10 1701/1701A.
- Forms and timing: 1701Q dates, 1702Q (+60), 2550Q/2551Q (+25), 1604-C (31 Jan) and 1701-MS single-rate/mixed-income wording all check out against code or Annex C.
- Build notes row 17 (1601-C item 19 gap), row 19 (8% input exposes grossReceipts only) and row 21 (2551Q helper blocks 8%, Books only) are accurate.
- R1–R9 repo links point to files that exist at buwiz-books HEAD.
- WW010 as the 1601-C ATC is correct (preprinted and official default).

## 10. MISSING and UNKNOWN, with method

- **MISSING**: 1701-MS micro/small thresholds, spouse limits and special-rate limits. Read Annex C fully (2 pages). No other source found in WebSearch.
- **MISSING**: Books 1701-MS implementation (grep of `src/**/*.ts`). Books base.ts has no depreciation, income-tax-payable or loan-payable accounts (key search).
- **MISSING**: Books employee-share feed into 1601-C (grep for `compensationFromPayrollLine` and `buildForm1601C` callers).
- **MISSING**: 1701-MS II.34 representative TIN key (key search, both repos).
- **MISSING**: signer and signature boxes for DECL.signature (all three forms).
- **UNVERIFIED**: 1% PT extension to 31 Dec 2025 (secondary commentary only; enacting law not found).
- **UNKNOWN**: TY2026 applicability of the Aug 2024 1701-MS edition and of Annex C (scoped to TY2025).
- **UNKNOWN**: RR 8-2018 text (8% base content; 2551Q in an 8% year). Not fetched.
- **UNKNOWN**: due dates not in code (2307, 1604-E, 1601-EQ, SAWT, 1706/1707/1800/1801, 2000).
- **UNKNOWN**: whether Books stores a final-tax attribute on interest income (not searched).
- **UNKNOWN**: legal role of Form 1606 (schema form_code only).
- **UNKNOWN**: payroll's monthly withholding method (only the table was recomputed).
- **UNKNOWN**: OSD base wording on printed 1701-MS line 11 (S3 PDF not fetched).
- **UNKNOWN**: printed Aug 2024 item numbers vs schema item numbers (schema and engine agree with each other; PDF not fetched).
- **UNKNOWN**: sources S1–S17 other than S3, and the Build notes S6/S7 links (not fetched).