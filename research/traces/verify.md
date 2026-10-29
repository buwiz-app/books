# Verifier report: Forms crosswalk, fill-tool keys, Books claims, workbook corrections, tax rules

**Method.** I re-read sources directly. Remote schemas came from `git show 63267ca:…` (object store, no fetch). Rust came from `git show` and `git grep` against the remote tree. Books and buwiz-oss were read in place. The workbooks were parsed with stdlib zip/XML, and every corrected figure was recomputed. BIR claims were checked against local text copies, and Annex C was read from a scanned PDF in the session's tool-results. The Books probe was re-run read-only, and the Books repo status was unchanged afterward. One early `git grep` tried a lazy fetch from the promisor remote and failed. Later reads used `GIT_NO_LAZY_FETCH=1`. Dumps are in `/private/tmp/claude-501/-Volumes-goldcoders-OSS-buwiz-books/d723bb20-1674-49c1-8fb6-568f056d2c7e/scratchpad/verify/`. No TINs or ticket values were printed.

| Repo | HEAD | State |
|---|---|---|
| OSS/buwiz-forms (L, worktree) | 301f9a0f | clean, stale |
| Forms remote pin (R) | 63267ca | present as a commit; read via object store |
| buwiz-oss (O) | 0f34106 | **40 modified/untracked**; agent-forms JSON clean; `form_2551q.rs` differs from HEAD by one deleted line |
| OSS/buwiz-books | 1662ca3 | dirty only outside `src` (`skills-lock.json`, untracked) |

## 1. Top corrections

1. The fill tool accepts only exact official box keys, including the `frm<Form>:` prefix. Bare `txt15` or `txt22` are rejected. For 2551Q and 1601-C, one bad key fails the whole call. For 1701-MS, a bad key is refused per key.
2. Typed 2551Q `compromise` writes the root field. Official `frm2551Qv2018:txt22` writes `/penalty_overrides/compromise`, which survives. The typed root value can be overwritten by the penalty recompute in auto mode.
3. 1701-MS Item 12 (8%), Item 17 (OSD), and `taxCodeDropdown` II015–17 are unlocked. Nothing compares them to the profile's recorded election.
4. Books confirmed: 1601-C drops employee contributions. The probe shows taxable 40,000 against 38,000 from `segregate()`.
5. Books has ten unposted PH accounts, not twelve.
6. Annex C is retrievable as a scan. A10 says a Micro/Small filer may file 1701-MS manually even if the COR omits it. The scope is TY2025.
7. RMC 34-2025 Annex A extends 1701-MS to fiduciaries. The workbook's "Individual/Fiduciary" label is supported, so the proposed change to "Individual" should not be made.
8. The reported 1% PT extension to 31 Dec 2025 has no primary support.

## 2. Claim verdicts

**Forms state and crosswalk**

| # | Claim (source) | Verdict | Evidence | Corrected statement |
|---|---|---|---|---|
| 1 | 63267ca "not in local object store" (Forms §0) | REFUTED | `git cat-file -t 63267ca…` → commit; `git show 63267ca:crates/bir-core/data/agent-forms/*.json` works | Pin is local; repo is a partial clone |
| 2 | buwiz-oss 0f34106 "clean" (Forms §0) | REFUTED | `git -C /Volumes/goldcoders/buwiz-oss status --short` → 40 entries incl. `integration/*`, `tax_profile/mod.rs`, `profile.rs` | Agent-forms JSON clean; O citations into integration/tax_profile are worktree text |
| 3 | Pinned schema hashes (Forms §3) | CONFIRMED | L 2551q `67188ca6966901b9`, 1701ms `85a4fd3d9f2d7f17`; R=O 2551q `a206416cabb09c5e`, 1701ms `6cf45329d89a09d6`; 1601c `07ca2f04da430efa` in all three | — |
| 4 | Bare official keys (`txt15`, `txt22`, `txtSalesRevenueNo5a`) (Forms §3, §10) | REFUTED as accepted notation | JSON keys are `frm2551Qv2018:txt15`, `frm1601c:txtTax14`, `frm1701MS:txtSalesRevenueNo5a`. Bare keys are only 2551Q `txtATCAmt1..6`, `txtEmail#occurrence-1`; 1601-C `txtEmail`; 1701-MS `taxCodeDropdown(Spouse)` | Use exact prefixed keys (§3) |
| 5 | 2551Q items 20/21 unbound; 22 → `/compromise` in local (Forms §3a, §9) | CONFIRMED for L only | Diff of binding dumps L vs R; R=O bind `txt20/21/22` → `/penalty_overrides/{surcharge,interest,compromise}` | L is stale |
| 6 | 2551Q email → `/email` profile lock (Forms §3a) | CONFIRMED (R=O); unbound in L | Binding dump | — |
| 7 | Item 13 lock = election; `taxRateNA` added in R (Forms §3a) | CONFIRMED | `frm2551Qv2018:taxRate*` lock=election; NA option only in R/O | — |
| 8 | Lock rule: identity, then election if a path contains "election", then profile (Forms §3, cites probe.rs 1326–1343) | CONFIRMED rule; line cites differ | R `forms/agent/probe.rs` `lock_for` 2466–2475; `is_election` 1752–1753 | Use R line numbers |
| 9 | Typed 2551Q `compromise` target "not traced" (Forms §11) | CONFIRMED (resolved) | R `bir-desktop/src/agent/assist.rs` 191 and `apply_2551q` 520+ → `set_top_level` (root). R `forms/form_2551q.rs` 961–991 recomputes root surcharge/interest/compromise from PenaltyEngine when `auto_compute_penalties` (default true, line 455) and a deadline exists, then applies overrides (Rule 8) | Typed root writes are overwritten in auto mode (PARTIAL: `refresh_auto_penalties` not traced). `txt22` is the durable input |
| 10 | 1701-MS Items 12, 17, and `taxCodeDropdown` II015–17 have no lock; no layer touches `tax_option` (Forms §5, §9) | CONFIRMED | Bindings have no lock field. `git grep tax_option` over R `forms/agent/`, `bir-desktop/src/agent/`, `data/agent-locks.json` hits only `probe.rs` 631–658 (CHOICE_OPTION_SETS) | Fillable without a profile check. INFERENCE: nothing else enforces it |
| 11 | L Item 17 also sets source/tax_option; R sets only deduction (Forms §9 #6) | CONFIRMED | L `txtTaxpayerNo17` sets `/taxpayer/deduction`, `/taxpayer/source`, `/taxpayer/tax_option`; R sets deduction only | — |
| 12 | 1601-C item 19 → `/tax_19_sss_gsis`, label "Mandatory contributions and union dues" (Forms §3b) | CONFIRMED | Box label and binding path, L=R=O | — |
| 13 | 1601-C items 37–40 unbound; item 39 has no drawee key (Spreadsheet) | CONFIRMED | Unmapped list; `txtAgency37/38/40` exist; no `txtAgency39` | — |
| 14 | 1701-MS II.34 representative TIN missing (Spreadsheet) | CONFIRMED | Only `authorizeRepresentative` → `/authorized_representative`; page TIN keys are taxpayer/spouse | — |
| 15 | 1701-MS IV.7 "bound" to `txtTaxableCompensationIncomeNo7a` (Spreadsheet) | REFUTED as "binding" | Box is computed with no binding. O HEAD `form_1701ms.rs` 559–569 lists the key with `self.net_sales` as a value map | It is an output map |
| 16 | Rename official keys such as `txtTaxableCompensationIncomeNo7a`, `txtSurchangeNo28a`, `txtTotalAllowableItemixedDeductionsNo10a` (Spreadsheet §6 #35) | UNCONFIRMED (risky) | O uses these keys as field-map names. INFERENCE: they mirror official PDF field names | Fix labels and paths; do not rename official keys without checking the PDF field list |
| 17 | 1601-C Schedule I rows 0–2 only; typed N≥3 "not traced" (Forms §11 #12) | UNCONFIRMED | Official rows 0–2. R `form_1601c.rs:35` `MAX_SCHEDULE_1_ROWS = 3`. `canonical_1601c_key` accepts any N syntactically; rejection path not traced | — |
| 18 | Typed `schedule_1.N.previous_month` accepted (Forms §3b) | CONFIRMED | R `assist.rs` `ROW_FIELDS_1601C` 228–233 | — |
| 19 | `bir_form_send` has no host handler (Forms §6) | CONFIRMED | Grep of R `bir-desktop/src` and `bir-mcp/src`: hits only in `bir-mcp` (`tools.rs` 173–174, 487, 550–564). `server.rs` 496–499 is inside the FakeHost test | — |
| 20 | 2551Q item 3 quarter radios and items 25–28 payment lines (Forms §3a) | CONFIRMED (omission) | R unmapped: `qtr_1` derived, `qtr_2..4` app-written; `txtAgency25…txtAmount28`, `txtParticular28` unmapped | Add rows |
| 21 | 1601-C email path (Forms §3b implies line-2 path) | CONFIRMED (omission) | Bare `txtEmail` → `/email_address`, lock profile | Use `/email_address` |
| 22 | 1701-MS "item 4 gross-compensation row" is a fillable key (Forms §3c) | REFUTED | `txtTaxpayerNo4a` is derived (unmapped). Gross compensation is `txtGrossa` → `/taxpayer/gross_compensation` | — |

**Leads (section B)**

| # | Lead | Verdict | Evidence | Corrected statement |
|---|---|---|---|---|
| 23 | `txtATCAmt1` → `/schedule_1/0/taxable_amount` | CONFIRMED | L, R, O identical; amount; no lock | Accepted exact; also `schedule_1.0.taxable_amount`, `taxable_amount`, widget `form-2551q-taxable-0` (`assist.rs` 252) |
| 24 | `frm1601c:txtTax14` → `/tax_14_total_compensation` | CONFIRMED | L=R=O; amount; "conditional" required (not computed) | Accepted exact; typed `tax_14_total_compensation`, `tax_14`, `form-1601c-tax-14` |
| 25 | `frm1601c:txtTax19` → `/tax_19_sss_gsis` | CONFIRMED | L=R=O | Accepted exact; typed `tax_19_sss_gsis` |
| 26 | `frm1701MS:txtSalesRevenueNo5a` → `/taxpayer/sales` | CONFIRMED | L=R=O | Accepted exact only; no 1701-MS typed aliases |
| 27 | `frm1701MS:txtSalesRevenuesFeesANo19a` → `/taxpayer/eight_sales` | CONFIRMED | L=R=O | Accepted exact only |

**Books (section C)**

| # | Claim | Verdict | Evidence | Corrected statement |
|---|---|---|---|---|
| 28 | `compensationFromPayrollLine` ignores employee contributions | CONFIRMED | `src/lib/tax/form-1601c.ts:52–109` has no SSS/PhilHealth/Pag-IBIG/union input. Caller `src/lib/tax/issue-1601c.ts:52`. Probe A/C: taxable 40,000 vs `segregate()` 38,000 | II.22 overstated by employee shares; II.19 absent; tax unchanged |
| 29 | Only unit test covers MWE/exempt | CONFIRMED | `tests/unit/lib/tax/form-1601c.test.ts:172–185` | — |
| 30 | Wiring test is string presence only | CONFIRMED | `tests/unit/tax-reference-wiring.test.ts:593` | — |
| 31 | 8% blocking string without suppressing figures | CONFIRMED | `src/lib/tax/percentage-tax.ts:355–360`; probe F (taxDue 9000.00 with block) | — |
| 32 | 2551Q route takes gross and 8% flag from request; election table unread | CONFIRMED | `src/routes/api/-tax-returns.ts:86–87` (`z.boolean().default(false)`); no `orgTaxYearElections` reference | — |
| 33 | UI defaults 8% to true; saves quarter 1 / year 2026 | CONFIRMED | `src/routes/tax.compute.tsx:37`, `:108` | — |
| 34 | `ComputedReturnForm` lacks 1701-MS | CONFIRMED | `src/db/schema/tax-stage-remainder.ts:21`; `src/lib/tax/filing-period.ts:30–37`; no `1701` match in src/server/tests/drizzle | — |
| 35 | 1% window hard-coded | CONFIRMED | `src/lib/tax/percentage-tax.ts:65–68`. Statute: RA 11534 §116 "until June 30, 2023" (`statutes/ra11534.clean.txt:141`) | — |
| 36 | Books 8% base omits other non-operating income | CONFIRMED | `computeEightPercent` 167–171 takes grossReceipts and `hasCompensationIncome` only. RMO 23-2018 base includes other non-operating income (`rmo23-2018/doc.txt:182–186`) | Add it; mixed earners get no ₱250k reduction (lines 194–196) |
| 37 | "Twelve of 21 PH accounts have no posting code" (Books §1 #11, §3b) | REFUTED | The report's own §3b MISSING list is ten: 12610, 12700, 13800, 13810, 21700, 21710, 21750, 21800, 68400, 94100. 12800, 25175, 61950 are posted in `src/lib/tax/annualization-posting.ts:133–145` | Ten |
| 38 | PH chart has 21 entries; `philippines-smb.ts:2` says 20 | CONFIRMED | 21 `accountNumber` fields at `ph-chart.ts` 71–255 (23 lines total with interface and helper) | Doc comment stale |
| 39 | 59 categories: 33 base, 18 PH, 8 new; 51 numbers match | CONFIRMED | Keyed comparison of catalog vs `base.ts` and `ph-chart.ts`: zero number mismatches | — |
| 40 | Three PH accounts missing from catalog (12700, 12800, 61950) | CONFIRMED | Catalog keys | — |
| 41 | `ph-account-resolver.ts:13` cites 21610; chart says 25110 | CONFIRMED | `ph-chart.ts:170` | — |
| 42 | Books probe A–I outputs | CONFIRMED | Re-run identical | — |
| 43 | Semi-monthly 1601-C collision | UNCONFIRMED | Probe D reproduced: same period, 500 vs 600. Upsert key (`issue-1601c.ts:67–73`) not re-read | — |

**Workbook corrections (section D; recomputed from formulas and cached values)**

| # | Claim | Verdict | Evidence | Corrected statement |
|---|---|---|---|---|
| 44 | C7 = 290,000; C9 = 8,700; C14 = 42,000; C18 = 38,000; C21 = 2,808.4 | CONFIRMED | Formulas evaluated from Example data | — |
| 45 | C11 `=C10` labelled II.18 | CONFIRMED | `2551Q fields` E25 "II.15 + II.16 + II.17" | Value 0 unchanged |
| 46 | C12 labelled II.24 is II.19 | CONFIRMED | `2551Q fields` B26/E26 "II.14 − II.18" | II.24 = II.19 + II.23 = 8,700 |
| 47 | C17 labelled II.21 must sum II.15–II.20 | CONFIRMED | `1601-C fields` E28 | Value 4,000 unchanged |
| 48 | C25 labelled II.36 is II.31 | CONFIRMED | `1601-C fields` E38 (II.27 − II.30), E43 (II.31 + II.35) | Value unchanged |
| 49 | Monthly 1601-C ≈ 2,808.33 | CONFIRMED arithmetic | 38,000×12 = 456,000; 22,500 + 0.2×56,000 = 33,700; ÷12 = 2,808.33 | Payroll method UNCONFIRMED |
| 50 | C33 = 85,300 applied to IV.14; field tab applies to IV.15 | CONFIRMED | `1701-MS fields` E78 (IV.3 + IV.14); IV.3 = 0 | Value unchanged |
| 51 | C29 = 1,190,000; C30 = 476,000; C32 = 714,000 | CONFIRMED | Arithmetic | — |
| 52 | H8 = 2,900 vs 3,000 peers; C35 = 8,900; C37 = 52,300 | CONFIRMED values | If H8 = 3,000: C35 9,000, C37 52,400, C39 32,900 | Whether 2,900 is an error UNCONFIRMED |
| 53 | C39 labelled II.29; chain II.23→II.25→II.27→II.29 | CONFIRMED | `1701-MS fields` E28, E30, E32, E37 | Numerically equal here |
| 54 | C37 labelled V.10 must sum V.1–V.9 | UNCONFIRMED | B99 = V.10 confirmed; E99 formula not printed | — |
| 55 | PAY-Q3 42,000 is one month | CONFIRMED | 40,000 + 2,000 = E-001 September | Relabel |
| 56 | 8% sheet: 24 cells; ties to 1701-MS (V.2 43,000; V.3 9,000; V.4 3,000; II.25 21,000) | CONFIRMED | Recomputed | — |
| 57 | 8% sheet `MAX(0,…)` clamps overpayment | CONFIRMED | `8 percent example` B14:E14; 2551Q fields F26 "Keep sign" | — |
| 58 | Profile C11–C15 referenced by no formula | CONFIRMED | No sheet-qualified or local references | — |
| 59 | "Form catalog!A5 is the 1700 row" | REFUTED | A5 is the header "Form" | — |
| 60 | 1701-MS scope "Individual/Fiduciary" → "Individual" (Annex C A7) | REFUTED as stated | RMC 34-2025 Annex A item 3 (`rmc34-2025/raw.txt` 355–370) includes trustees, guardians, and executors where the trust, minor, or estate is in trade or business | Keep "Individual/Fiduciary"; annotate |
| 61 | Category selectors share one template | CONFIRMED | 59 formulas, one template | — |

**Tax rules (section E)**

| # | Claim | Verdict | Evidence | Corrected statement |
|---|---|---|---|---|
| 62 | PT010 = 3% after 30 Jun 2023 | CONFIRMED | RMC 69-2023 lines 9–14; RA 11534 §116 proviso ends 30 Jun 2023; probe H: 300 bps for 2026-09-30 | — |
| 63 | 1% extended to 31 Dec 2025 by a later law | UNCONFIRMED | Only source is respicio.ph commentary (fetched). It names no law and carries "may involve AI assistance. Information may be inaccurate." Search did not confirm | If true, Books and buwiz-oss PT010 are wrong for 2023-07-01 to 2025-12-31; Q3 2026 unaffected |
| 64 | 2551Q due 25 days after quarter-end | CONFIRMED | RA 11976 §128(1) as amended by §29 (`ra11976.norm2.txt:211`); `guide2551q/doc.txt:53` | — |
| 65 | VAT threshold ₱3M CPI-indexed every 3 years | CONFIRMED | `ra11976.norm2.txt:131` | — |
| 66 | "109(BB) under TRAIN, 109(CC) under EOPT" | REFUTED as stated | 109(BB) appears 4× in RA 10963; 109(CC) appears in RA 11534 §116 (CREATE, line 141) and RA 11976 (5×) | CC predates EOPT |
| 67 | 1601-C item 17 cap ₱90,000 | CONFIRMED | `ra10963.clean.txt:103` | — |
| 68 | MWE holiday pay exempt | CONFIRMED | `ra10963.clean.txt:63` | OT/NSD/hazard basis UNCONFIRMED |
| 69 | 1601-C item 19 = employee share only | CONFIRMED (RR); statute UNCONFIRMED | `rr11-2018/doc.txt:1126` | — |
| 70 | RR 8-2024 tiers | CONFIRMED | `rr8-2024b/doc.txt:7–17` | — |
| 71 | Lawphil numerals off by 1,000 | CONFIRMED | `ra11976.norm2.txt:31`: words "Three million pesos" vs "P3,000.00" | — |
| 72 | 8% election in lieu of graduated and PT; ₱3M cap | CONFIRMED | `rmo23-2018/doc.txt:36–42` | — |
| 73 | 8% electors do not file 2551Q | CONFIRMED (RMO) | `rmo23-2018/doc.txt:160` | Conflicts with O HEAD `form_2551q.rs` ~2216–2226 (NIL PT010 each quarter). Books `deadlines.ts:122` agrees with RMO. RR 8-2018 full text not read (UNCONFIRMED) |
| 74 | Mid-year breach → graduated from start of year; PT until VAT | CONFIRMED | `rmo23-2018/doc.txt:164–170` | — |
| 75 | Mixed earners: no ₱250k; base includes other non-op income | CONFIRMED | `rmo23-2018/doc.txt:182–196` | — |
| 76 | 1701-MS manual filing allowed even if COR omits it | CONFIRMED (manual route, TY2025 FAQ) | Annex C A10, `tool-results/webfetch-1793246366651-jtcwoi.pdf` p.2 (stamp MAR 16 2026). Supersedes BIR track's "unverified" | Electronic-filing COR question not addressed |
| 77 | Annex C: single income-tax rate for business income; mixed earners included | CONFIRMED | Annex C A5 row 3, p.1 | — |
| 78 | Annex C: 1701-MS not yet on eFPS/eBIRForms | CONFIRMED (as of Mar 2026) | A6, p.2 | Superseded by `rmc36-2026/doc.txt:27` and `rmc37-2026/doc.txt:11` (electronic 1701-MS, eBIRForms 7.9.6) |
| 79 | Annex C thresholds, spouse, and special-rate limits | MISSING | Both pages read | — |
| 80 | Annex C as a TY2026 source (both reports: UNKNOWN) | REFUTED as a TY2026 source | Title "…ON FILING THE 2025 ANNUAL INCOME TAX RETURN" | TY2025 FAQ |
| 81 | 1701Q due May 15 / Aug 15 / Nov 15 | CONFIRMED | `guide1701q/doc.txt:40–42` | — |
| 82 | WB080/082 (2307); WB080/082/084 (1600-PT) | CONFIRMED | `form2307b/doc.txt:159, 162`; `form1600pt/doc.txt:142–144` | — |
| 83 | 2551M status; 1600-PT due date; 1701-MS rounding and Oct 15 installment; TIN 5-digit branch (RMC 36-2026 item 3); printed 1601-C due-date text | UNCONFIRMED | Image or OCR only, not re-read here | — |
| 84 | X1: 2551Q in an 8% year | UNCONFIRMED (conflict CONFIRMED) | Books blocks; O HEAD requires NIL PT010; RMO 160 says not required | Needs RR 8-2018 |

## 3. Lead B: which string the fill tool accepts (R, 63267ca)

- Path: `form.fill` → `host.rs` `form_fill` (2500). For 2551Q and 1601-C, `split_fill_keys` (host.rs 4479–4499) routes a key to the typed path if `canonical_*_key` accepts it: widget ids, legacy aliases, names in `FIELDS_*`, or `schedule_1.N.<field>`. Otherwise it goes to the official path only if `box_def(key)` matches exactly (`forms/agent/mod.rs` 790–792). Anything else fails the whole call with "unknown or read-only 2551Q field `…`".
- For 1701-MS (generic path, `host_generic.rs` 373 → 762), only exact official keys work. A non-box key is refused per key.
- So the report's bare `txt15` or `txt22` is rejected. `frm2551Qv2018:txt15` and `creditable_tax_withheld` are accepted.
- At the assistant layer, `fill` passes an empty allowed list, so election and identity locks are refused. A profile lock lifts only if the profile leaves that box blank (fill doc comment, `agent/mod.rs`).
- Typed-only input with no official box: 2551Q `original_return_filed_and_paid_on_time` (`assist.rs` 182). Its effect is not traced.

## 4. Verified crosswalk

Key = exact string the fill tool accepts on the official path. Lock = JSON flag. Scope: taxpayer (A) or spouse (B) for 1701-MS; schedule rows for 2551Q and 1601-C. Versions: R=O unless noted.

**2551Q** (L differs at items 20–22 and email; L lacks taxRateNA)

| Item | Key | Draft path | Type | Lock / computed | Typed alias | Scope / note |
|---|---|---|---|---|---|---|
| 1 | `frm2551Qv2018:forThe` (+`_1`, `_2`) | `/tax_period_basis` | choice | — | `tax_period_basis` | — |
| 2 | `frm2551Qv2018:rtnMonth` / `txtYear` | `/year_end_month` / `/taxable_year` | whole | — / identity | `year_end_month` | — |
| 3 | `frm2551Qv2018:qtr_1..4` | unmapped (derived) | radio | not fillable | — | omitted in report |
| 4 | `frm2551Qv2018:amendedRtn` (+`_1`, `_2`) | `/is_amended` | choice | — | `is_amended` | — |
| 5 | `frm2551Qv2018:txtSheets` | `/number_of_attached_sheets` | whole | — | same | — |
| 6 | `txtTIN1–3`, `txtBranchCode` (prefixed) | unmapped (derived from /tin) | text | not fillable | — | — |
| 7 | `frm2551Qv2018:txtRDOCode` | `/rdo_code` | select | profile | — | — |
| 8 | `frm2551Qv2018:registeredName` | `/taxpayer_name` | text | profile | — | — |
| 9 | `frm2551Qv2018:registeredAddress`; `zipCode` | `/registered_address`; `/zip_code` | text | profile | — | — |
| 10 | `frm2551Qv2018:telNo` | `/contact_number` | text | profile | — | — |
| 11 | `txtEmail#occurrence-1` (bare); `-2` | `/email`; unmapped | text | profile | — | L unbound |
| 12 | `frm2551Qv2018:taxTreaty` (+`_1`, `_2`); `txtTaxReliefSpecify` | `/tax_relief`; `/tax_relief_specification` | choice; text | — | `tax_relief`, `tax_relief_specification` | — |
| 13 | `frm2551Qv2018:taxRate` (1 graduated, 2 eight_percent, NA not_applicable; `_1`, `_2`, `NA`) | `/item_13_election` | choice | **election** (refused) | none | L lacks NA |
| 14, 18, 19, 23, 24 | `frm2551Qv2018:txt14`, `txt18`, `txt19`, `txt23`, `txt24` | — | amount | computed | — | — |
| 15 | `frm2551Qv2018:txt15` | `/creditable_tax_withheld` | amount | — | `creditable_tax_withheld`; widget `form-2551q-creditable` | — |
| 16 | `frm2551Qv2018:txt16` | `/tax_paid_previous` | amount | — | `tax_paid_previous` | Counts only if amended |
| 17 | `frm2551Qv2018:txt17`; `txt17Specify` | `/other_tax_credit`; `/other_tax_credit_description` | amount; text | — | both typed; widget `form-2551q-other-credit` | — |
| 20 | `frm2551Qv2018:txt20` | `/penalty_overrides/surcharge` | amount | — | none | L unbound |
| 21 | `frm2551Qv2018:txt21` | `/penalty_overrides/interest` | amount | — | none | L unbound |
| 22 | `frm2551Qv2018:txt22` | `/penalty_overrides/compromise` | amount | — | `compromise` (root; overwritten in auto mode) | L binds `/compromise` |
| — | `frm2551Qv2018:overPayment` (+`1`, `2`) | `/overpayment_disposition` | choice | — | `overpayment_disposition` | — |
| Sch 1 row N=1..6 amount | `txtATCAmtN` (bare) | `/schedule_1/(N−1)/taxable_amount` | amount | — | `schedule_1.(N−1).taxable_amount`; `taxable_amount` and `form-2551q-taxable-0` → row 1 | Handover lead |
| Sch 1 row N ATC | `drpATCN` | derived; refused | select | not fillable | `schedule_1.(N−1).atc` (ATC table) | — |
| Sch 1 rate, due, item 7 | `txtATCRateN`, `txtATCDueN`, `txtTotalSched1` | — | amount | computed | — | — |
| 25–28, Particular 28 | `txtAgency25…txtAmount28`, `txtParticular28` | unmapped | — | not fillable | — | omitted in report |
| Tax agent | `txtTaxAgentNo`, `txtDateIssue`, `txtDateExpiry` | unmapped | — | not fillable | — | — |

**1601-C** (L = R = O)

| Item | Key | Draft path | Type | Lock / computed | Typed alias | Scope / note |
|---|---|---|---|---|---|---|
| 1 | `frm1601c:txtMonth` / `txtYear` | `/month` / `/taxable_year` | whole | identity / identity | — | — |
| 2 | `frm1601c:AmendedRtn` (+`_1`, `_2`) | `/is_amended` | choice | — | `is_amended` | — |
| 3 | `frm1601c:TaxWithheld` (+`_1`, `_2`) | `/any_taxes_withheld` | choice | — | `any_taxes_withheld`; widget `form-1601c-withheld` | — |
| 4 | `frm1601c:txtSheets` | `/number_of_sheets` | whole | — | `number_of_sheets` | — |
| 5 | `frm1601c:txtATC` | `/atc` | text | — | `atc` | — |
| 6 | `txtTIN1–3`, `txtBranchCode` | unmapped (derived) | text | not fillable | — | — |
| 7 | `frm1601c:txtRDOCode` | `/rdo_code` | select | profile | — | — |
| 8 | `frm1601c:txtTaxpayerName`; `txtLineBus` | `/taxpayer_name`; `/line_of_business` | text | profile | — | — |
| 9 | `frm1601c:txtAddress`; `txtAddress2` | `/registered_address`; `/registered_address_2` | text | profile; — | `registered_address_2` | — |
| 9A | `frm1601c:txtZipCode` | `/zip_code` | text | profile | — | — |
| 10 | `frm1601c:txtTelNum` | `/contact_number` | text | profile | — | — |
| 11 | `frm1601c:CatAgent` (P/G; `_P`, `_G`) | `/category_of_agent` | choice | — | `category_of_agent`; widget `form-1601c-category` | — |
| 12 | `txtEmail` (bare) | `/email_address` | text | profile | — | Path not in report |
| 13 | `frm1601c:SpecialTax` (+`_1`, `_2`); `selTreaty` | `/tax_relief`; `/tax_relief_specification` | choice; select | — | both typed | — |
| 14 | `frm1601c:txtTax14` | `/tax_14_total_compensation` | amount | required conditional | `tax_14_total_compensation`, `tax_14`, `form-1601c-tax-14` | Handover lead |
| 15 | `frm1601c:txtTax15` | `/tax_15_statutory_minimum_wage` | amount | — | same name | — |
| 16 | `frm1601c:txtTax16` | `/tax_16_holiday_pay` | amount | — | same name | — |
| 17 | `frm1601c:txtTax17` | `/tax_17_13th_month_pay` | amount | — | same name | — |
| 18 | `frm1601c:txtTax18` | `/tax_18_de_minimis` | amount | — | same name | — |
| 19 | `frm1601c:txtTax19` | `/tax_19_sss_gsis` | amount | — | `tax_19_sss_gsis` | Handover lead |
| 20 | `frm1601c:txtTax20`; `txt20Other` | `/tax_20_other_amount`; `/tax_20_other_name` | amount; text | — | both typed | — |
| 21, 22, 24, 26, 27, 30, 31, 35, 36 | `frm1601c:txtTax21` etc. | — | amount | computed | — | — |
| 23 | `frm1601c:txtTax23` | `/tax_23_not_subject` | amount | — | same name | — |
| 25 | `frm1601c:txtTax25` | `/tax_25_total_taxes_withheld` | amount | conditional | same name | — |
| 28 | `frm1601c:txtTax28` | `/tax_28_tax_remitted_previously` | amount | — | same name | — |
| 29 | `frm1601c:txtTax29`; `txt29Other` | `/tax_29_other_remittances_amount`; `/…_name` | amount; text | — | both typed | — |
| 32 | `frm1601c:txtTax32` | `/penalty_overrides/surcharge` | amount | — | `tax_32_surcharge` (write target not traced) | — |
| 33 | `frm1601c:txtTax33` | `/penalty_overrides/interest` | amount | — | `tax_33_interest` | — |
| 34 | `frm1601c:txtTax34` | `/penalty_overrides/compromise` | amount | — | `tax_34_compromise` | — |
| Sch I rows 0–2 | `frm1601c:sched1:txtDatePaidN`, `txtBankCodeN`, `txtNumberN`, `txtShouldTaxDueN`, `txtTaxPaidN` | `/schedule_1/N/{date_paid, drawee_bank_code_or_agency, payment_number, should_be_tax_due, tax_paid}` | text; amount | — | `schedule_1.N.*` incl. `previous_month` | Official rows 0–2; typed cap 3 |
| Sch I month | `frm1601c:sched1:txtMonthYearN` | derived (shows 07/2031); refused | — | not fillable | `schedule_1.N.previous_month` | — |
| Adjustments, total | `frm1601c:sched1:txtAdjustmentsN`, `txtTotal1` | — | amount | computed | — | — |
| 37–40 | `txtAgency37/38/40`, `txtNumber37–40`, `txtDate37–40`, `txtAmount37–40`, `txtParticular40` | unmapped | — | not fillable | — | Item 39 has no drawee key |

**1701-MS** (R = O; L lacks most spouse, signature, and email bindings and couples tax_option at items 17 and 31)

| Item | Key | Draft path | Type | Lock / computed | Scope / note |
|---|---|---|---|---|---|
| 1 | `frm1701MS:txtMonthNo1`; `txtYearNo1` | `/month`; `/taxable_year` | whole | —; identity | A |
| 2 | `frm1701MS:txtAmended` (+`YesNo2`, `NoNo2`) | `/is_amended` | choice | — | A |
| 3 | `frm1701MS:txtReturnPeriod` (+ variants) | `/is_short_period` | choice | — | A |
| 5 | `frm1701MS:civilStatus`; `txtIfMarried` (+Jointly, Separately, NotApplicable) | `/civil_status`; `/filing` | choice | — | — |
| 8 | `frm1701MS:txtTaxpayerNo8a`; `txtTaxpayerNo8b` | `/taxpayer_name`; `/spouse_info/name` | text | profile; — | A; B |
| 10 | `frm1701MS:txtTelNum10a` | `/contact_number` | text | profile | A |
| RDO | `frm1701MS:txtRDOCodea` | `/rdo_code` | text | profile | A |
| Email | `frm1701MS:txtEmaila`; `txtEmailb` | `/email`; `/spouse_info/email` | text | profile; — | L lacks both |
| 11 | `frm1701MS:txtTaxpayerNo11group` (a business, b mixed); `txtTaxpayerNo11a/b/c` | `/taxpayer/source` | choice | — | A |
| 12 | `frm1701MS:txtTaxpayerNo12` (c = eight_percent); `txtTaxpayerNo12b/c/d/e` | `/taxpayer/tax_option` (graduated, eight_percent, exempt, special) | choice | **no lock** | A; spouse `…1` → `/spouse/tax_option`; L couples source |
| 17 | `frm1701MS:txtTaxpayerNo17` (a itemized, b osd); `txtTaxpayerNo17a/b` | `/taxpayer/deduction` | choice | **no lock** | A; L also sets source and tax_option |
| Part I code | `taxCodeDropdown` (bare): II012–14 graduated; II015/16/17 eight_percent | `/taxpayer/source` + `/taxpayer/tax_option` | select | **no lock** | A; probe overrides II012 → graduated |
| 19 | `txtIncomeTaxDueNo19a` | — | amount | computed | — |
| 20 | `frm1701MS:txtShareOfOtherNo20a`; `txtOtherNonOperatingIncomeNo20a` | `/taxpayer/share_of_other_agencies`; `/taxpayer/eight_other_income` | amount | — | A |
| 26 | `frm1701MS:txtTaxDueAllowedNo26a` | `/taxpayer/second_installment` | amount | — | A |
| 28 | `frm1701MS:txtSurchangeNo28a`; `txtInterestNo28a`; `txtCompromise28a` | `/taxpayer/penalty_overrides/{surcharge, interest, compromise}` | amount | — | A |
| 31 | `frm1701MS:txtToBeRefunded`, `txtToBeIssued`, `txtToBeCarried` (checkbox); `…1` amounts | `/to_be_refunded`, `/to_be_issued_tcc`, `/to_be_carried_over`; `/refund_amount`, `/tcc_amount`, `/carry_over_amount` | checkbox; amount | — | L couples tax_option |
| 32 | `frm1701MS:perjuryClause` | `/perjury_agreed` | checkbox | required | — |
| Signature | `frm1701MS:printedName`; `authorizeRepresentative` | `/signatory_name`; `/authorized_representative` | text | — | No representative TIN key |
| 33 | `frm1701MS:txtPg1I33NumberOfAttachments` | unmapped (constant) | — | not fillable | — |
| IV-A | `frm1701MS:txtGrossa`; `txtLessNonTaxablea` | `/taxpayer/gross_compensation`; `/taxpayer/non_taxable_compensation` | amount | — | A; `txtTaxpayerNo4a` derived |
| IV 5 | `frm1701MS:txtSalesRevenueNo5a` | `/taxpayer/sales` | amount | — | A; Part V 5 is `txtCreditableForm2316No5a` |
| IV 6, 8 | `frm1701MS:txtSalesReturnsNo6a`; `txtCostofSalesNo8a` | `/taxpayer/sales_returns`; `/taxpayer/cost_of_sales` | amount | — | A |
| IV 7 | `frm1701MS:txtTaxableCompensationIncomeNo7a` | — | — | computed | Display map only |
| IV 10 | `frm1701MS:txtAllowableItemizedDeductionsNo10a`; `txtSpecialAllowableNo10a`; `txtNetOperatingLossNo10a` | `/taxpayer/itemized_deductions`; `…/special_allowable_deductions`; `…/nolco` | amount | — | A; 10D computed |
| IV 11, 12, 14, 15, 25 | `txtOptionalDeductionNo11a`, `txtNetIncomeLossNo12a`, `txtTaxableIncomeBusinesNo14a`, `txtCompensationBusinessNo15a`, `txtCompensationBusinessNo25a` | — | amount | computed | A |
| IV 13 | `frm1701MS:txtNonOperatingIncomeNo13a` | `/taxpayer/non_operating_income` | amount | — | A |
| IV 19 (8%) | `frm1701MS:txtSalesRevenuesFeesANo19a` | `/taxpayer/eight_sales` | amount | — | A; handover lead |
| Part V 1–9 | `txtPriorYearsNo1a`, `txtTaxPaymentsNo2a`, `txtCreditableTaxNo3a`, `txtCreditableFourthQuarterNo4a`, `txtCreditableForm2316No5a`, `txtTaxPaidPreviouslyFiledNo6a`, `txtForeignTaxCreditsNo7a`, `txtSpecialTaxCreditsNo8a`, `txtOtherCreditsPaymentsNo9a` | `/taxpayer/{prior_year_excess, quarterly_payments, cwt_q1_q3, cwt_q4, cwt_2316, previously_filed, foreign_tax_credits, special_tax_credits, other_credits}` | amount | — | A; spouse `…b` → `/spouse/…` |
| Payment lines | `txtCashBank*`, `txtCheck*`, `txtTaxDebitMemo*`, `txtOther*` | unmapped | — | not fillable | No tax-debit-memo drawee key |

## 5. Not verified

- Forms registry tallies (43 records, 31 can_queue), the provenance-in-memory claim, and Rox and cloud state.
- Report line cites in L and R `draft_locks.rs` and `form_2551q.rs` (except as marked).
- Printed-form items read only from images (1601-C due text, 1701-MS rounding and installment, 2551Q item 13 instruction).
- RR 8-2018 full text, RMC 30-2026, 2551M status, 1600-PT due date, the `refresh_auto_penalties` semantics, and the spreadsheet's E99 formula.