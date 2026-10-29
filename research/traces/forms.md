# Buwiz Forms draft contracts: investigation report

**Legend.** CONFIRMED = seen in code or data. PARTIAL = seen in part. MISSING = searched and not found (method stated). UNKNOWN = not established. [L] = local checkout, HEAD 301f9a0f. [R] = remote main 63267ca, read read-only via `gh api` into scratch copies. Repo roots: F = `/Volumes/goldcoders/OSS/buwiz-forms`, O = `/Volumes/goldcoders/buwiz-oss`, S = `/private/tmp/claude-501/-Volumes-goldcoders-OSS-buwiz-books/d723bb20-1674-49c1-8fb6-568f056d2c7e/scratchpad/forms/`. No local checkout was edited, fetched, or built. No tests were run.

## 0. Repos and HEADs read

| Repo | HEAD read | State | Notes |
|---|---|---|---|
| F (OSS/buwiz-forms) | 301f9a0f (main, clean) | **STALE** | `origin/main` = 63267ca (ls-remote; compare API: ahead 30, behind 0, 300 files). 63267ca is not in the local object store. |
| O (buwiz-oss) | 0f34106 (main, clean; commits 7358ab7, 12978fd, 0f34106) | current | `origin/main` = 0f34106 (ls-remote). |
| /Volumes/goldcoders/buwiz-server | 417ec3ae (clean) | grep only | No hits for `bir_ticket` or `taxprofile`. |
| /Volumes/goldcoders/Projects/rox | not recorded | grep only | Other track. |

Remote crates at 63267ca: bir, bir-core, bir-desktop, bir-mcp, bir-print, bir-rules, bir-rules-codegen, bir-rules-platform, bir-shell. CONFIRMED (git/trees API).

## 1. Canonical copy per crate

**Verdict.** buwiz-forms@63267ca is canonical for the app, MCP, desktop, and transport code. buwiz-oss@0f34106 is canonical for the offline-filer crates it contains, and its schema data matches 63267ca byte for byte. Do not use the local F checkout as reference. Neither repo is a superset of the other.

| Crate | Forms (remote 63267ca) | O (0f34106) | Canonical now |
|---|---|---|---|
| bir-core | Full: email, Google Calendar, inbox OAuth, ipc, transport, sftp_loopback, dev_mode, integration/providers, bin/ | Cut: no network, no providers, no IPC or email. Adds calendar_events, deadline_overrides, post_commit, forms/agent/draft_locks, forms/taxform, forms/kept_text, forms/payment_cells, forms/event_trigger | Both. Forms-only: 97 files. O-only: 18 files. Agent-form JSON, `tax_profile/templates.rs`, and `forms/agent/probe.rs` are identical between R and O. |
| bir-rules | present, plus bir-rules-codegen and bir-rules-platform | present (runtime) | O for the runtime. Only Cargo.toml differs from F-local. |
| bir-print | present (frozen_html.rs; bin/generate_form_capabilities.rs) | present (print_sheet, static_html, pdf_sink, packaged, assets/, fixtures/) | Divergent. Not established which is newer. |
| bir-app-core | absent | present ("the one door the app calls", typed `BirApp` facade) | O (only copy). |
| bir-submit | absent | present (queue client) | O (only copy). |
| bir-mcp | present (17 tools) | absent | Forms (only copy). |
| bir-desktop | present (agent host) | absent | Forms (only copy). |
| bir-shell | present in R only (gpui-shell host, QuickJS, JS app UI) | absent | Forms-R. |

Evidence:
- O commit 12978fd: "Copied from the source repo at oss-cut-base (29ca915b) and cut to the offline filer … Removed: accounts, Google Calendar, data providers, notifications, IPC, shell-command jobs, email reading … PIN/TOTP helpers … no network transport ships here." CONFIRMED.
- 29ca915b is not in the local F object store (`cat-file`: not a valid object). Its relation to 63267ca is UNKNOWN.
- O `AGENTS.md`: "`bir-core` opens no network connection"; no SFTP code in the repo.
- Inference: O is a cut of a source at or near 63267ca, with local edits. Its `tax_profile/mod.rs` matches neither R nor F-local (it adds `tax_years`, `check_tax_year`, a reserved admin TIN root, and `TaxpayerCredentials.pin_hash`).

## 2. Tax profile model (`crates/bir-core/src/tax_profile/`, `profile.rs`)

Line numbers are [L]. R differs: `tax_profile/mod.rs` +100/−1, `profile.rs` +175/−12. `templates.rs` in R equals O and adds `TemplateSource::Return{draft_id}`.

- **Taxpayer = 9-digit TIN root.** `tax_profile/mod.rs:6-7`; `parse_tin_root` 65-74. Table `taxpayers` (`db/migrations.rs:561-568`, `tin_root` PK with a 9-digit CHECK). `Taxpayer` struct 135-143.
- **Branch = profile row keyed by full TIN** (root plus 5-digit branch code, 14 digits). `mod.rs:10-13`; `normalize_branch_code` 78-85 (3–5 digits, zero-padded). Table `profiles` (`migrations.rs:92-96`: tin UNIQUE, `data_json`).
- **Head office = branch 00000.** `is_head_office_code` 87-89; `HEAD_OFFICE_NAME` 36.
- **Inheritance.** `BranchField` {RdoCode, RegisteredAddress, ZipCode, Phone, Email} 173-179; `resolve_facts` 295-316.
- **Entity type.** `profile.rs:9-17` `TaxpayerType` {Individual, Corporation, Partnership, Cooperative, Estate, Trust}. Individual classification `TaxClassification` {PurelyCompensation, SelfEmployed, MixedIncome}, `profile.rs:25-33` (doc).
- **COR.** `CorRegistrationFacts` `profile.rs:233-250`: tin, registration_date, registered_name, trade_name, registered_address, rdo_code, line_of_business_code and description. Uploaded COR evidence `CorDocumentRef` 253-272 (stored_path, OCR text, bboxes).
- **Versions and effective dates.** `TaxProfileVersion` `profile.rs:307-350` (status, source, effective_from/until, `needs_effective_date_review`, cor, registered_tax_types, evidence, overrides). Resolution: `ResolvedTaxProfileForYear` 394-399, `ResolvedTaxProfileForPeriod` 412-416, issues 373-388. Validation `integration/validation.rs:281-297` and overlap 355-362.
- **Registered tax types.** `RegisteredTaxType` `profile.rs:170-182` {IncomeTax, ValueAddedTax, PercentageTax, RegistrationFee, WithholdingExpanded, WithholdingCompensation, WithholdingFinal, WithholdingVatAndPercentage, ExciseTax}. Mapping in `integration/validation.rs` `registered_tax_types_allow_form` 1227-1290:
  - VAT: 2550DS, 2550M, 2550Q (1314-1316).
  - Percentage: 2551M, 2551Q (1318-1320).
  - Expanded withholding: 0619E, 1601E, 1601EQ, 1604E, 1606, 1621 (1322-1327).
  - Compensation: 0620, 1601C, 1604C, 2316 (1329-1331).
  - 0605 maps to RegistrationFee.
  - Individuals and estates: 1702* never; PurelyCompensation only 1700.
- **Elections.** `IncomeTaxElection` `profile.rs:94-103` {GraduatedUnspecified, GraduatedOsd, GraduatedItemized, EightPercent}. `TaxElectionHistory` 137-141 (taxable_year, election, elected_at, source_form). `TaxpayerProfile.tax_elections` (~476). `has_8_percent_election` 650-655; `income_tax_election_for_year` 657; `eligible_for_income_tax_election_in_year` 674.
- **Annual ITR inference** (`integration/validation.rs` ~1393-1412): PurelyCompensation → 1700; MixedIncome → 1701; SelfEmployed with EightPercent or GraduatedOsd → 1701A; SelfEmployed micro or small with year ≥ 2024 and any other election (including none) → 1701MS. CONFIRMED for those branches. The fallback (itemized or none) is an INFERENCE from branch order. Conflict group at 1361 and 1531.
- **Per-year forms.** `TaxpayerProfile.per_year_forms: BTreeMap<u16, PerYearFormsSet>` (`profile.rs:570`). Table `per_year_forms` (`migrations.rs:238-250`, UNIQUE(tin, taxable_year, form_code)).
- **Form templates.** `TemplateSource` `tax_profile/templates.rs:25-29` {Filed{draft_id}, Blank, Copy{from_tin}} (R/O add Return). `FormTemplate` 63-75; `validate_template_fields` 107; `GuidedStop` 159; `PrefillPlan` 168. Table `form_templates` (`migrations.rs:571-579`, UNIQUE(tin, form_code)). `field_kinds.rs`: `FieldKind` 63, `FieldSpec` 69, `field_kinds_for` 132.
- **Starter packs** `tax_profile/mod.rs:330-370` (doc says "Contents for the maintainer to review"). Employer = 1601C, 0619E, 1601EQ, 1604C, 1604E. NonVat and Professional for individuals = 2551Q, 1701Q, plus 1701 or 1701A.
- **Drafts.** Table `form_drafts` (`migrations.rs:112-124`, UNIQUE(tin, form_code, taxable_year, quarter)). Real period identity is `period_key` (`db/drafts.rs:343-400`, writes 473, 521-528).
- **Ownership.** CONFIRMED: all profile, COR, election, per-year form, template, and draft values live in the local SQLCipher database (`F/README.md:53`: `~/Library/Group Containers/group.dev.goldcoders.bir/bir_data.db`, key in Keychain). The cloud supplies only the MCP license check and the tax-profile list (section 5). Rox owns none of these values.
- **Identity leftovers.** `profile.rs:484-488` keeps `profile_pin_hash` and `totp_secret`, and O `tax_profile/mod.rs:80-84` keeps `TaxpayerCredentials.pin_hash`. The O commit message says PIN/TOTP helpers were removed.

## 3. Pinned agent-form schemas

Files in `crates/bir-core/data/agent-forms/` (identical names in F and O): `2551q-v2018.json` (form 2551Q), `1601c-v2018.json` (1601C), `1701ms-v2024.json` (1701MS).

| File | sha256 [L] | sha256 [R] = O | Lines (R) |
|---|---|---|---|
| 2551q-v2018.json | 67188ca6966901b9 | a206416cabb09c5e | 200 |
| 1601c-v2018.json | 07ca2f04da430efa | 07ca2f04da430efa (all three identical) | 225 |
| 1701ms-v2024.json | 85a4fd3d9f2d7f17 | 6cf45329d89a09d6 | 418 |

Structure: `bindings` (official key → {path, type, lock, options[{value,set}]}); `boxes` (printed boxes: key, item, label, kind, required, computed, group, members, options); `rows` (repeated-row blank template); `unmapped` (reason per unbound box). Rust counterpart: `AgentFormDef` `forms/agent/mod.rs:546-558`; `Binding` 503-527 (Text, Amount, Whole, Choice); `Lock` 467-488 (identity, profile, election); `BoxDef` 405-440.

Lock assignment (CONFIRMED [L]; `probe.rs` in R equals O and was not re-read): `forms/agent/probe.rs:1326-1343`. Identity if the path is an identity path. Election if any path contains "election" (case-insensitive). Profile if a string value comes from the profile. So `/item_13_election` locks, while `/taxpayer/tax_option` and `/taxpayer/deduction` do not.

Item-number caveat (CONFIRMED): printed item numbers repeat across parts of 1701-MS. For example, item "5" is both `txtSalesRevenueNo5a` and `txtCreditableForm2316No5a`, and item "4" is both a gross-compensation row and `txtCreditableFourthQuarterNo4a`. Use the key suffix, not the item, as the identifier.

Pin guard: `crates/bir-core/tests/sample_payloads/agent_forms.rs` reads the environment variable `n` (`std::env::var_os("n")`). `forms/agent/mod.rs:10-11` documents `UPDATE_AGENT_FORMS=1`. These disagree.

### 3a. Form 2551Q (R = O version; [L] differences in brackets)

Schedule 1 rows: `rows["/schedule_1"]` = {atc, atc_description, tax_due, tax_rate, taxable_amount}. Row family is N = 1..6.

| Printed item (JSON label) | Official key | Typed key | JSON path (type, lock) | Notes |
|---|---|---|---|---|
| Sch 1 row N taxable amount | `txtATCAmtN` | `schedule_1.(N-1).taxable_amount` | `/schedule_1/(N-1)/taxable_amount` (amount) | Typed field `Schedule1Row.taxable_amount` (`form_2551q.rs:148`). |
| Sch 1 row N ATC | `drpATCN` (refused) | `schedule_1.(N-1).atc` (validated against ATC table) | not bound: "derived by the app" | `apply_2551q_row` `assist.rs:547-560`. |
| Sch 1 row N rate / tax due | `txtATCRateN`, `txtATCDueN` | none | computed | `tax_rate` 150, `tax_due` 152. |
| Item 7 total | `txtTotalSched1` | none | computed | |
| Item 13 income-tax rate | `taxRate`, `taxRate1`, `taxRate2` (R adds `taxRateNA`) | none | `/item_13_election` (choice); lock **election** | Refused by fill. R: Q2–Q4 locked to the filed Q1 election (`draft_locks.rs:1518-1690`, test assertions). |
| Item 14 total tax due | `txt14` | none | computed | `total_tax_due` 239. |
| Item 15 creditable % tax withheld | `txt15` | `creditable_tax_withheld` | `/creditable_tax_withheld` (amount) | 241. |
| Item 16 tax paid previously filed | `txt16` | `tax_paid_previous` | `/tax_paid_previous` (amount) | 243. Mapper applies it only when amended. |
| Item 17 other credit | `txt17` / `txt17Specify` | `other_tax_credit` / `_description` | `/other_tax_credit` / `_description` | 246, 249. |
| Item 18 total credits | `txt18` | none | computed | 252. |
| Item 19 tax still payable | `txt19` | none | computed | `tax_payable` calc 802. |
| Item 20 surcharge | `txt20` [L: unbound] | none | R: `/penalty_overrides/surcharge` | [L] root `surcharge` f64 266. |
| Item 21 interest | `txt21` [L: unbound] | none | R: `/penalty_overrides/interest` | [L] root `interest` 268. |
| Item 22 compromise | `txt22` | `compromise` (`assist.rs:190-191`) | R: `/penalty_overrides/compromise`; [L]: `/compromise` | `assist.rs` comment says "Item 21C". JSON label says "22 Compromise". |
| Item 23 / 24 totals | `txt23`, `txt24` | none | computed | 272, 274. |
| Item 2 year-end month / year | `rtnMonth` / `txtYear` | none | `/year_end_month` (whole) / `/taxable_year` (whole, **identity**) | |
| Item 1 period basis | `forThe`, `forThe_1`, `forThe_2` | none | `/tax_period_basis` (choice) | |
| Item 5 sheets | `txtSheets` | `number_of_attached_sheets` | `/number_of_attached_sheets` (whole) | |
| Items 6–10 TIN, RDO, name, address, ZIP, phone | `txtTIN1-3`, `txtBranchCode`, `txtRDOCode`, `registeredName`, `registeredAddress`, `zipCode`, `telNo` | none | TIN unbound (derived); RDO/name/address/ZIP/phone → profile lock | |
| Item 11 email | `txtEmail#occurrence-N` | none | R: `/email` (profile); [L] unbound | |
| Item 12 tax relief | `taxTreaty*`, `txtTaxReliefSpecify` | `tax_relief`, `tax_relief_specification` | `/tax_relief`, `/tax_relief_specification` | |

### 3b. Form 1601-C (identical in F and O)

| Printed item | Label (JSON) | Official key | Typed key | JSON path | Type; lock/computed |
|---|---|---|---|---|---|
| II.14 | Total amount of compensation | `frm1601c:txtTax14` | `tax_14_total_compensation` | `/tax_14_total_compensation` | amount; conditional |
| 15 | Statutory minimum wage | `txtTax15` | `tax_15_statutory_minimum_wage` | same pattern | amount |
| 16 | MWE holiday, OT, night, hazard pay | `txtTax16` | `tax_16_holiday_pay` | `/tax_16_holiday_pay` | amount |
| 17 | 13th month and other benefits | `txtTax17` | `tax_17_13th_month_pay` | `/tax_17_13th_month_pay` | amount |
| 18 | De minimis | `txtTax18` | `tax_18_de_minimis` | `/tax_18_de_minimis` | amount |
| 19 | "Mandatory contributions and union dues" (path says `sss_gsis`) | `txtTax19` | `tax_19_sss_gsis` | `/tax_19_sss_gsis` | amount |
| 20 | Other non-taxable comp | `txtTax20`, `txt20Other` | `tax_20_other_amount`, `tax_20_other_name` | `/tax_20_other_amount`, `/tax_20_other_name` | amount, text |
| 21, 22, 24, 26, 27, 30, 31, 35, 36 | totals | none | none | none | computed |
| 23 | Taxable comp not subject to withholding | `txtTax23` | `tax_23_not_subject` | `/tax_23_not_subject` | amount |
| 25 | Total taxes withheld | `txtTax25` | `tax_25_total_taxes_withheld` | `/tax_25_total_taxes_withheld` | amount; conditional |
| 28 | Tax remitted previously | `txtTax28` | `tax_28_tax_remitted_previously` | `/tax_28_tax_remitted_previously` | amount |
| 29 | Other remittances | `txtTax29`, `txt29Other` | `tax_29_other_remittances_amount`, `_name` | same pattern | amount, text |
| 32 / 33 / 34 | Surcharge / interest / compromise | `txtTax32/33/34` | `tax_32_surcharge` / `tax_33_interest` / `tax_34_compromise` (write target not traced) | `/penalty_overrides/{surcharge,interest,compromise}` | amount; unlocked |
| 3 | Taxes withheld (yes/no) | `TaxWithheld*` | `any_taxes_withheld` | `/any_taxes_withheld` | choice |
| 4 | Sheets | `txtSheets` | `number_of_sheets` | `/number_of_sheets` | whole |
| 5 | ATC | `txtATC` | `atc` | `/atc` | text |
| 1 | Month, year | `txtMonth`, `txtYear` | none | `/month`, `/taxable_year` | **identity** |
| 7–10, 12 | RDO, name, address, ZIP, phone, email | `txtRDOCode`, `txtTaxpayerName`, `txtAddress`, `txtZipCode`, `txtTelNum`, `txtEmail` | `registered_address_2` (address line 2) | profile lock | |
| 11 | Category | `CatAgent*` | `category_of_agent` | `/category_of_agent` | choice |
| 13 | Special rate / treaty | `SpecialTax*`, `selTreaty` | `tax_relief`, `tax_relief_specification` | `/tax_relief`, `/tax_relief_specification` | |
| Sch I rows N = 0..2 only | Date paid, bank, number, should-be, paid | `frm1601c:sched1:txtDatePaidN`, `txtBankCodeN`, `txtNumberN`, `txtShouldTaxDueN`, `txtTaxPaidN` | `date_paid`, `drawee_bank_code_or_agency`, `payment_number`, `should_be_tax_due`, `tax_paid` | `/schedule_1/N/…` | text/amount |
| Sch I month | `txtMonthYearN` (refused: "derived by the app") | `previous_month` (typed, accepted) | `/schedule_1/N/previous_month` | |
| Sch I adjustment, total | `txtAdjustmentsN`, `txtTotal1` | `adjustment` (typed) | computed | |
| Items 37–40 payment lines | `txtAmount37–40` etc. | none | unbound ("written by the app without a draft field") | not fillable |

Schedule I rows: the official keys stop at N = 2. The `{N>=0}` entries are pattern notes, not boxes.

### 3c. Form 1701-MS (R = O version, which adds spouse bindings; [L] differences noted)

| Key suffix (JSON item) | Label | Official key | Taxpayer path | Spouse path (R) | Type; lock |
|---|---|---|---|---|---|
| No1a (1) | Prior-year excess | `txtPriorYearsNo1a` | `/taxpayer/prior_year_excess` | `/spouse/prior_year_excess` (1b) | amount |
| No2a (2) | Quarterly tax payments | `txtTaxPaymentsNo2a` | `/taxpayer/quarterly_payments` | `/spouse/quarterly_payments` | amount |
| No3a (3) | Creditable tax Q1–Q3 | `txtCreditableTaxNo3a` | `/taxpayer/cwt_q1_q3` | 3b | amount |
| No4a (4) | Creditable tax Q4 | `txtCreditableFourthQuarterNo4a` | `/taxpayer/cwt_q4` | 4b | amount |
| No5a (5, Part V) | Creditable Form 2316 | `txtCreditableForm2316No5a` | `/taxpayer/cwt_2316` | 5b | amount |
| No5a (5, Part IV) | Sales / revenue | `txtSalesRevenueNo5a` | `/taxpayer/sales` | `/spouse/sales` (5b) | amount |
| No6a (6) | Sales returns | `txtSalesReturnsNo6a` | `/taxpayer/sales_returns` | 6b | amount |
| No6a (6) | Tax paid, previously filed | `txtTaxPaidPreviouslyFiledNo6a` | `/taxpayer/previously_filed` | 6b | amount |
| No7a (7) | Foreign tax credits | `txtForeignTaxCreditsNo7a` | `/taxpayer/foreign_tax_credits` | 7b | amount |
| No8a (8) | Cost of sales | `txtCostofSalesNo8a` | `/taxpayer/cost_of_sales` | R: `txtCostofSalesNo8b` → `/spouse/cost_of_sales` | amount |
| No8a (8) | Special tax credits | `txtSpecialTaxCreditsNo8a` | `/taxpayer/special_tax_credits` | 8b | amount |
| No9a (9) | Other credits and payments | `txtOtherCreditsPaymentsNo9a` | `/taxpayer/other_credits` | 9b | amount |
| No10a (10) | Itemized deductions | `txtAllowableItemizedDeductionsNo10a` | `/taxpayer/itemized_deductions` | R: `…10aSpouse` → `/spouse/itemized_deductions` | amount |
| No10a (10) | Special allowable deductions | `txtSpecialAllowableNo10a` | `/taxpayer/special_allowable_deductions` | R: 10b | amount |
| No10a (10) | NOLCO | `txtNetOperatingLossNo10a` | `/taxpayer/nolco` | R: 10b | amount |
| No13a (13) | Non-operating income | `txtNonOperatingIncomeNo13a` | `/taxpayer/non_operating_income` | 13b | amount |
| No19a (19) | Sales or fees under the 8% schedule | `txtSalesRevenuesFeesANo19a` | `/taxpayer/eight_sales` | R: 19b → `/spouse/eight_sales` | amount |
| No20a (20) | Other non-operating income, 8% | `txtOtherNonOperatingIncomeNo20a` | `/taxpayer/eight_other_income` | R: 20b | amount |
| No20a (20) | Share of other agencies | `txtShareOfOtherNo20a` | `/taxpayer/share_of_other_agencies` | R: 20b | amount |
| No26a (26) | Tax due allowed, second installment | `txtTaxDueAllowedNo26a` | `/taxpayer/second_installment` | R: 26b | amount |
| No28a (28) | Surcharge / interest / compromise | `txtSurchangeNo28a`, `txtInterestNo28a`, `txtCompromise28a` | `/taxpayer/penalty_overrides/{surcharge,interest,compromise}` | R: 28b | amount; unlocked |
| (IV-A) | Gross compensation | `txtGrossa` | `/taxpayer/gross_compensation` | `txtGrossb` | amount |
| Item 12 | Tax option; 8% = value `c` (spouse `c1`) | `txtTaxpayerNo12` | `/taxpayer/tax_option` = eight_percent | `/spouse/tax_option` | choice; **no lock** |
| Item 17 | Deduction: `a` itemized, `b` OSD | `txtTaxpayerNo17` | `/taxpayer/deduction` | `txtSpouseNo17` | choice; no lock |
| Part I | Tax code: `II015/16/17` set `tax_option` = eight_percent | `taxCodeDropdown` | `/taxpayer/tax_option`, `/taxpayer/source` | none | select; **no lock** |
| Status, filing | Civil status; joint or separate | `civilStatus`, `txtIfMarried*` | `/civil_status`, `/filing` | | choice |
| Identity | Year; month | `txtYearNo1`; `txtMonthNo1` | `/taxable_year` (**identity**); `/month` (unlocked) | | |
| Profile | Name; RDO; phone; email | `txtTaxpayerNo8a`, `txtRDOCodea`, `txtTelNum10a`, `txtEmaila` (R) | `/taxpayer_name`, `/rdo_code`, `/contact_number`, `/email` | | profile lock |
| Computed | Items 19, 21–25, 27–29 totals | | | | computed |
| Payment lines (Item 33) | Cash, check, debit memo | | unbound ("not written by this app's field map") | | not fillable |

Schedules 4.10A, 4.13A/B, 4.20A/B, and 5 No. 9 are keyed as `…Amount{N>=0}` row families. The pinned `rows` object is empty, so the agent cannot add rows. The typed `Form1701MsColumn.itemized_rows` (`form_1701ms.rs:213`) is a GUI-only path (INFERENCE from the absent row template).

Typed model: `Form1701MsDraft` `form_1701ms.rs:1029`; `Form1701MsColumn` 182+ with `eight_sales` 232, `eight_other_income` 234, `quarterly_payments` 243, `cwt_q1_q3` 244, `cwt_q4` 245, `cwt_2316` 246, `surcharge` 264, `interest` 265, `compromise` 266, and `#[serde(flatten)] penalty: PenaltyState` at 269–270. `PenaltyState.penalty_overrides` at `penalty_items.rs:58-77` (field 76), typed as `PenaltyOverrides` `penalties/assessment.rs:82-92` (`Option<f64>` fields plus a note). A scripted check resolved all 246 binding paths for 1701-MS, with no missing segments. The 1601-C penalty paths also resolve (in `assessment.rs`).

## 4. Typed drafts, registry, validation, calculation, mapper

**Money.** CONFIRMED: typed drafts are f64 throughout (`Schedule1Row` 148–152; `Form1601CDraft` 153–208; `Form1701MsColumn` 182–270; `Form2551QDraft` 239–274). The only integer-cent conversion is `cents(v) = round(v*100) as i64` (`form_2551q.rs:1259-1261`), used in `FilingCalculationSnapshot` (1262–1290). Payable rounding is `((total_tax_due − total_tax_credits)*100).round()/100` (`form_2551q.rs:802`). Agent `parse_amount` returns f64 (`agent/mod.rs:767`). `integration/models.rs` is f64 throughout (`gross_amount` 38, `tax_rate_override` 49, `creditable_withholdings` 105, `previous_tax_paid` 109, `total_gross_income` 137–139). R equals L for `models.rs` and `mapper.rs`.

**Mapper** (`integration/mapper.rs`; identical in R):
- `FormDraftOutput` 31–38: only `Form2551Q`. Form1701Q, Form1701, and Form2550M are commented out (35–37).
- `resolve_mappers` 308–341: explicit target "2551Q" → `Mapper2551Q`. "1701Q" is a comment. Auto-detect only when a BusinessNonVat category or an ATC override is present.
- No 1601C or 1701MS mapper (MISSING: the mapper module contains none).
- `resolve_atc_code` 98–111: ATC must come from `atc_code_override`; categories never map to ATCs (fail closed).
- `calendar_quarter_period` 74–96: exact calendar quarter only.
- Amounts must be finite, non-negative, and at most two decimals (~129–140, ~233–244). An override rate must match the registry rate within 1e-12 (~176–190).
- 8% gate (~265–273): a positive PT010 row in a year where `profile.has_8_percent_election(year)` is true is rejected. NIL PT010 is allowed.

**Registry** (`forms/support_level.rs`; identical in L, R, and O). Tally run with `S/tally_registry.py`:
- 43 records (`FORM_CAPABILITY_REGISTRY` 132–1047). Gate `FormCapabilities::can_queue` (32–38): typed_model, xml_round_trip, formula_evidence, persistence, and queue_submission must all be true.
- **can_queue = YES (31):** 2551Q, 1601C, 1606, 1600VT, 1600PT, 1600WP, 1601EQ, 1601FQ, 1603Q, 1602Q, 1604C, 1604E, 1604F, 1700, 1701A, 1701MS, 1702EX, 1702Q, 1706, 1707, 1707A, 1800, 1801, 2000, 2000OT, 2200C, 2200S, 2200T, 2550M, 2552, 2553.
- **can_queue = NO (12):** 0605, 0619E, 0619F, 1701, 1701Q, 1702MX, 1702RT, 2550Q (typed_model false); 2200A, 2200AN, 2200M, 2200P (queue_submission false).
- **release_ready:** only the `TEST` fixture is true (`support_level.rs:1149-1153`). No real form is release-ready, so `FormSupportLevel::ImplementedInApp` is unreachable for all 43. CONFIRMED.
- **Discrepancy:** the 1702RT record comment (389–391) says queueing goes through the generic `QueueableForm` path. The gate says NO, and `form_1702rt_view.rs:2122-2125` refuses with "1702-RT is not enabled for in-app submission in this build."
- GUI views calling the gate: `form_1601fq_view.rs:1606,1761`; `form_1604c_view.rs:1044`; `form_1701a_view.rs:2213`; `host.rs:4743` (generic).

**Which forms are implemented end to end** (INFERENCE from the code paths above):
- **Tier A (full agent path, fill to queue):** 2551Q (typed, agent, mapper, queue) and 1601C (typed, agent, queue, no mapper).
- **Tier B (queue gate open, agent cannot queue):** the other 29 forms in the YES list, including 1701MS, 1701A, 1702Q, 1601EQ, and 1604C/E/F. The agent host refuses them (section 6).
- **Tier C (no typed model, queue blocked):** the 12 NO forms. The agent can still fill and validate them from the pinned schema (`AgentFormKind::all()` = 43, `agent/mod.rs:44-49`; test `every_form_has_pinned_data_for_its_package` at 1075).

## 5. bir-mcp and the host contract

Server: `crates/bir-mcp` in F. Stdio, or Streamable HTTP on loopback with a bearer token (`http.rs`). Discovery reads `<runtime>/hosts/*.json` records with `app: "bir-desktop"` (`apps.rs:8-63`). Tokens come from the `BIR_MCP_TOKENS` file, which must not be group- or world-readable (`tokens.rs`), with `GPUI_AGENT_TOKEN` as fallback.

**Tool table** (R has 17; L has 16). Schemas are from R `tools.rs` unless marked L.

| MCP tool | Host op | Inputs | Notes |
|---|---|---|---|
| bir_list_apps, bir_connect, bir_status | app.info, hello | connect: `id` (required), `confirm_real` (bool) | REAL database refused without `confirm_real` |
| bir_profiles_list | profile.list | none | TIN (14-digit full), branch, name, RDO |
| bir_dues_list | dues.list | filter ∈ {upcoming, overdue, all}; scope ∈ {profile, global} | |
| bir_form_open | form.open | `code` (normalized to 4 digits), `year` 2000–2100, `period` 0–999 (default 0), `tin` (required) | `tools.rs` L 259-287, 403-429 |
| bir_form_fields | form.fields | none | Returns official boxes for generic forms; for 2551Q and 1601C also `fields` and `boxes` (L `host_generic.rs:283-299`, `host.rs:2448-2497`) |
| bir_form_context | form.context | none | Profile COR facts, `elections` (from `profile.tax_elections`, `host.rs:2703-2711`), past returns, documents |
| bir_form_fill | form.fill | `fields` (object; string, number, or bool; min 1); `source` ∈ {profile, past_return, document, ai (default), user (only with from_card)}; `from_card` | `tools.rs` L 289-314, 431-467 |
| bir_form_needs_you, _validate, _preview, _card | form.needs_you, validate, preview, card | none | |
| bir_form_review | form.review_in_app | none | Saves; requires Draft status and clean validation; never queues (`host_generic.rs:1010` test) |
| bir_form_save_draft, bir_form_dismiss | form.save_draft, form.dismiss | none | Dismiss refuses on unsaved edits |
| **bir_form_send (R only)** | form.send | none | See section 6 |

`ALLOWED_OPS` (L `tools.rs:469-484`, 15 items; R 16 with form.send). `banned()` (L 539-552) rejects names containing submit, queue, pay, paid, or filing, and words file, files, send, sftp, or efps. R adds an exception for `bir_form_send` and `form.send`.

**Sources.** CONFIRMED: `FILL_SOURCES` = [profile, past_return, document, ai] (`tools.rs:89`). `user` only with `from_card: true` and only for keys on the card (`fill_args` 431-467; `host_generic.rs:525-548`). The host `BoxSource` enum is Profile, PastReturn, Document, User, Ai (`assist.rs:30-35`). `parse_fill` rejects "user" outside a card. **No bookkeeping or ledger source exists** (MISSING: five variants, four allowed).

**Draft identity.** MCP key: (code, year, period, tin). Host: `DraftIdentity {tin, taxable_year, period, status}` (`agent/mod.rs:173-178`). Storage: `form_drafts`, with `period_key` the real identity. Period mapping (`host_generic.rs:88-100`): monthly 1–12, quarterly 1–4, annual 0, open-ended max(1, p). One form open at a time: `gate_form_lock` (`host.rs:776-800`) returns "form already open".

**Locked, computed, and user boxes.** `refusal()` (`agent/mod.rs:589-615` in L; R 818-840 has the same logic) refuses:
- keys not on the form;
- computed boxes;
- hidden boxes;
- any binding with a lock (identity, profile, or election), with reason text;
- unmapped boxes, with the pinned reason.

Gaps (CONFIRMED in the pinned data): 1701-MS Item 12 (8%, `c`/`c1`), Item 17 (OSD), and `taxCodeDropdown` `II015/16/17` (sets `tax_option` = eight_percent) have no lock. No layer in R `draft_locks.rs` references `tax_option` (grep: no hits). So the 8% election can be filled without `bir_form_context` showing it. The only guard is tool text and skill rule 2 (INFERENCE that nothing else enforces it). L Item 17 also set `/taxpayer/source` and `/taxpayer/tax_option`; R sets only `deduction`.

User-typed boxes: `form.assist.is_user_box(key)` skips them and returns them in `kept_user_boxes`. `fill_result` (`host.rs:4428-4440`) returns form, source, applied, filled, and kept_user_boxes; `not_shown` is appended. Typed forms clone the draft, apply all keys, and write back only on success (`host.rs` form_fill, L 2500–2600). Any change clears `validated`.

**What a filled field records.**
- Value: written into the draft and saved by save_draft (`host.rs:2042-2098`) into `form_drafts.data_json`.
- Provenance: `note_agent_fill(key, source)` (`assist.rs:104-106`) into `AssistState.sources` (`assist.rs:70-75`). **In memory only.** The `form_drafts` DDL has no field for it, `drafts.rs` never writes it, and the draft structs have no such field (grep: zero hits; the 2551Q hit is a test name). So provenance and user-box protection are lost on restart. CONFIRMED for L. R host changes (+28/−8 in `host.rs`, +770/−24 in `host_generic.rs`) were not checked for this.

**MCP license gate** (R only; `crates/bir-mcp/src/license.rs`, `server.rs:170-172`). Every tool calls `License::allow()` first.
- `Open`: when `BUWIZ_MCP_URL` is unset (`license.rs:46`).
- `Required{base, token}`: POST `{base}/mcp`, `tools/call` `list_tax_profiles` (`license.rs:184-205`). The doc says `/mcp` is a buwiz-server endpoint (`license.rs:220`).
- `Desktop{base, access, refresh, persist}`: GET `{base}/api/tax-profiles` (`license.rs:272-284`).
- Denial messages: `UNAUTHENTICATED` (30) and `UNLICENSED` (36). Test: `an_unlicensed_token_refuses_every_tool_before_the_host_is_touched`.
- `oauth.rs` and `signin.rs` exist in R and were not read.

## 6. Filing paths and gating

**Queue path** (L; the same `unknown invoke` fallthrough sits at R `host.rs:3472`, but the queue lines were not re-read in R):
1. `form.queue`, `filing.queue`, `form.submit` → `queue_open_return` (`host.rs:3444`, 2221). Gates in order:
   - `gate_locked`;
   - `require_queue_confirm`: `args.confirm` must be JSON `true` (`host.rs:4163-4173`). The error text says the host "cannot verify that assertion";
   - `prepare_queue_target`;
   - the active view must be 1601C or 2551Q ("supports 1601C and 2551Q only", 2221–2232).
2. `queue_open_2551q` (`host.rs:2354-2384`): draft must be editable, recompute, validate with no errors, `transition_to_queued`, save the queued row, revert on failure.
3. Authorization record: `QueueAuthorization {form_code, tin, period, completion = Confirmed, source, authorized_at}` (`filing_queue.rs` ~63–94). Written with `QueueAuthSource::Gui` at `form_2551q.rs:1040-1045` and `form_1601c.rs:497-502`. The agent `confirm=true` path uses the same source; the `AgentConfirm` variant is reserved.
4. Worker: `scoped_authorization_error` (`filing_queue.rs:99-123`) allows PUT only when form, TIN, and period match and completion is Confirmed. Otherwise it fails closed to Draft.
5. `filing.submit` → `request_submit_confirm` (`host.rs:2184-2219`): validates, sets a confirmation flag, returns `{confirmation: true, queued: false, filed: false, needs_confirm: true}`. No queue.
6. `form.file`, `form.submit_external`, `filing.file` → refused "not allow-listed" (`host.rs:3445-3447`).

**R design doc** `F-R:crates/bir-desktop/docs/QUEUE_AUTH_SCOPE.md` (79 lines, R only, saved at `S/remote/QUEUE_AUTH_SCOPE.md`): three ways to queue (`filing.queue confirm=true`, GUI Submit, cron for already-queued rows). States that "`confirm=true` is a caller assertion … operators must gate the agent token." Claim lease is 120 s. Live PUT requires `BIR_SFTP_LIVE=1`. Secrets are env-only.

**R new `bir_form_send`** (`tools.rs` ~173–177): "Queue the open form for upload from this computer, after the same validation as bir_form_validate. Requires a standing OK for this taxpayer and form, granted in the app. Without that OK the result is approval_required and pending, and nothing is queued."
- Host implementation at R: **NOT FOUND.** No `form.send` arm in R `host.rs`, `host_generic.rs`, `drain.rs`, `headless.rs`, or `bridge.rs`. `host.rs:3472` falls through to `unknown invoke`. The R file-name scan for approve, standing, grant, consent, allow, and scope turned up only the docs, `license`, `oauth`, `signin`, and `cloud_account`. The GitHub code-search API returned nothing for known symbols, so absence is not exhaustive.
- Result: at 63267ca the MCP advertises a queueing tool that the checked host files do not handle. This contradicts the "draft-only" wording in `tools.rs` and in the OpenGrok README. UNKNOWN whether it is implemented elsewhere.

## 7. Installed Rox mini app and tickets

**Installed state** (names and types only; no values printed). `~/Library/Application Support/Rox/mini-v1.json`:
- `attachments[0]`: plugin `dev.goldcoders.bir`, scope `"taxprofile"`, major 2, needs_update false; token present, not printed.
- `pins[4]`: apps tax-profile and forms, placement full, titles "BIR Tax Profile" and "BIR Forms"; ticket present, not printed.
- `opened`: five entries under `dev.goldcoders.bir/…`; `sent` empty.

`plugins/dev.goldcoders.bir/cache/`: 12 bundles (card-calc, card-dates, card-forms, card-help, card-money, card-people, card-quick, card-return, deadlines, forms, register, tax-profile). No manifest or source exists in that folder (UNKNOWN where the plugin source lives). Each bundle's header reads "The v1 bridge's JS half (spec 7, design §2), prepended by Rox to an app's compiled module."

**Names the bundles call** via `rox.call("…")` (CONFIRMED by scan):
- forms: bir_ticket_issue, bir_ticket_subject, bir_app_upload, bir_app_print_preview, bir_record_payment, bir_mark_paid, bir_payments_list, bir_payment_receipt, bir_card_remove_payment.
- tax-profile: bir_ticket_issue, bir_ticket_subject.
- card-quick: bir_deadline, bir_deadline_override_set, bir_deadline_override_remove, bir_calendar_set, bir_card_allow_send, bir_status, and more.
- deadlines: bir_calendar_get.
- Zero bundles reference any Forms agent op (`form.*`, `profile.*`, `dues.*`, `filing.*`).
- The forms bundle calls `rox.call("bir_ticket_subject", {ticket})`, then `bir_ticket_issue` (js lines ~5350–5353). Ticket values are not reproduced here.

**Search results** for those names:
- F, full tree excluding target and node_modules: none.
- R `bir-shell` Rust sources (25 files fetched): none.
- O: doc comments only. `deadline_overrides.rs:17`, `dashboard_module.rs:82-86`, `screen_form_module.rs:2919` and `6144` ("Rox's print preview (`bir_app_print_preview`)"), and `screen_form_module.rs:2934` ("the old mini app's" payments). These mark the mini-app host set as ported from the old shell.
- buwiz-server: none.
- Rox checkout: `bir_ticket_subject` not found in 5,175 files. Bridge locators only: `app/src/mini.rs` and `app/src/mini_v1/` (other track).

**Conclusions.**
- The installed mini app calls a `bir_*` namespace that is not the bir-mcp or agent namespace. The BIR-side handler for `bir_ticket_*` and the other names is UNKNOWN.
- Forms has no ticket or scope concept (MISSING: grep for `ticket`, `taxprofile`, `tkt_` across crates and integrations). Forms identifies a taxpayer only by the 14-digit full TIN from `profile.list` (`tools.rs` tin schema; `host.rs` filing.start: "use args.tin as 14 digits"). Any ticket-to-profile mapping must happen before a Forms call, in the unknown handler. UNKNOWN.
- The "taxprofile" scope has no Forms counterpart. The only "scope" in Forms is the dues scope (profile or global).

## 8. OpenGrok integration (`F:integrations/opengrok/bir/`, read in L only)

- `plugin.json`: name bir, v0.1.0, "Draft-only … Never submits, queues, files or pays."
- `mcp.json`: streamable-http `http://127.0.0.1:8765/mcp`, Authorization placeholder (not a secret).
- `README.md`: says `tools/list` must show "only the thirteen bir_* tools". Stale: 16 in L, 17 in R.
- `skills/file-tax/SKILL.md` (138 lines): never guess; never infer an election; state the source; never overwrite user boxes; one form at a time; never force dismiss; never print tokens; the user picks the app; `confirm_real` only on explicit request. Flow: list, ask, connect, profile, period, open, learn, fill by source, needs-you or card, validate, preview, save or review.

## 9. Discrepancies and risks (observed)

1. Local F is 30 commits behind `origin/main`; the 2551Q and 1701-MS JSON differ.
2. 2551Q penalties: L binds `/compromise` and leaves txt20 and txt21 unbound. R and O bind `/penalty_overrides/*`.
3. `assist.rs:190` says compromise is "Item 21C"; the JSON label says "22 Compromise".
4. 1701-MS item numbers repeat across parts (section 3c).
5. 1701-MS 8% fields are unlocked, and `II015/16/17` set eight_percent without a lock (L and R).
6. L Item 17 OSD side effect (source and tax_option); R has removed it.
7. 1702RT comment says generic queue; the gate says NO; the GUI refuses.
8. No real form is release_ready, while the README claims an in-app queue for 43 forms.
9. Regeneration: `UPDATE_AGENT_FORMS=1` (docs) versus env `n` (test).
10. MCP README "thirteen tools" against 16 (L) and 17 (R).
11. `bir_form_send` advertised at R but not wired in the checked files.
12. 1601-C Schedule I: official keys only for rows 0–2. Typed `schedule_1.N.*` keys for N ≥ 3 were not traced.
13. Provenance and user-box protection are in memory only.
14. Mapper: 2551Q only. The 8% gate blocks only positive PT010.
15. PIN and TOTP fields persist in both profile models despite the O commit message.
16. Annual ITR inference: 1701-MS is inferred for micro or small individuals with a graduated-itemized, unspecified, or no election, from 2024 (INFERENCE from branch order).

## 10. API key vs label

What `bir_form_fill` accepts as keys. For 1701-MS, `form.fields` returns official keys only, so there are no typed aliases.

| Form | Workbook or brief label | API key (accepted) | Printed label (JSON) | Draft path | Kind | Lock or computed | Status |
|---|---|---|---|---|---|---|---|
| 2551Q | "2551Q S1.1.base" | `txtATCAmt1` (official); `schedule_1.0.taxable_amount` (typed); `form-2551q-taxable-0` (widget alias) | Sch 1 row 1 Taxable Amount | `/schedule_1/0/taxable_amount` | amount | none | CONFIRMED |
| 2551Q | Schedule ATC | `drpATC1` refused; `schedule_1.0.atc` accepted | ATC 1 | `/schedule_1/0/atc` | select | unbound | CONFIRMED |
| 2551Q | Item 14 | `txt14` | Total Tax Due | none | | computed | CONFIRMED |
| 2551Q | Item 15 | `txt15` / `creditable_tax_withheld` | Creditable % tax withheld | `/creditable_tax_withheld` | amount | none | CONFIRMED |
| 2551Q | Item 16 | `txt16` / `tax_paid_previous` | Tax paid previously filed | `/tax_paid_previous` | amount | none | CONFIRMED |
| 2551Q | Item 17 | `txt17` / `other_tax_credit` | Other tax credit | `/other_tax_credit` | amount | none | CONFIRMED |
| 2551Q | Item 22 | `txt22` / `compromise` | Compromise | R: `/penalty_overrides/compromise`; L: `/compromise` | amount | none | CONFIRMED (both versions) |
| 2551Q | Item 13 | `taxRate` | Item 13 | `/item_13_election` | choice | **election lock** | CONFIRMED |
| 1601C | "1601-C II.14" | `frm1601c:txtTax14` / `tax_14_total_compensation` / `form-1601c-tax-14` | Total amount of compensation | `/tax_14_total_compensation` | amount | conditional | CONFIRMED |
| 1601C | "1601-C II.19" | `frm1601c:txtTax19` / `tax_19_sss_gsis` | Mandatory contributions and union dues | `/tax_19_sss_gsis` | amount | none | CONFIRMED (path narrower than label) |
| 1601C | Items 15–18, 20, 23, 25, 28, 29 | `txtTaxNN` / `tax_NN_…` | per label | per the table in section 3b | amount | none | CONFIRMED |
| 1601C | Items 32–34 | `txtTax32/33/34` / `tax_32_surcharge` etc. | Surcharge, interest, compromise | `/penalty_overrides/*` | amount | unlocked | CONFIRMED (typed write target not traced) |
| 1601C | Sch I month | `frm1601c:sched1:txtMonthYear0` refused; `schedule_1.0.previous_month` accepted | | `/schedule_1/0/previous_month` | text | | CONFIRMED |
| 1701MS | "1701-MS IV.5" | `frm1701MS:txtSalesRevenueNo5a` | Item 5 | `/taxpayer/sales` | amount | none | CONFIRMED (R adds `…5b` → `/spouse/sales`) |
| 1701MS | "1701-MS IV.19" | `frm1701MS:txtSalesRevenuesFeesANo19a` | Item 19 | `/taxpayer/eight_sales` | amount | none | CONFIRMED |
| 1701MS | Item 8 | `txtCostofSalesNo8a` | Item 8 | `/taxpayer/cost_of_sales` | amount | none | CONFIRMED |
| 1701MS | Item 10 | `txtAllowableItemizedDeductionsNo10a` | Item 10 | `/taxpayer/itemized_deductions` | amount | none | CONFIRMED |
| 1701MS | Item 12, 8% | `txtTaxpayerNo12` = `c` | Item 12 | `/taxpayer/tax_option` = eight_percent | choice | **no lock** | CONFIRMED |
| 1701MS | Item 17, OSD | `txtTaxpayerNo17` = `b` | Item 17 | `/taxpayer/deduction` = osd | choice | **no lock** | CONFIRMED |
| 1701MS | Part I tax code | `taxCodeDropdown` = `II015` | code | `/taxpayer/tax_option` = eight_percent | select | **no lock** | CONFIRMED |
| 1701MS | Item 20, 8% other | `txtOtherNonOperatingIncomeNo20a` | Item 20 | `/taxpayer/eight_other_income` | amount | none | CONFIRMED |
| 1701MS | Items 1–9 credits | `txtPriorYearsNo1a`, `txtTaxPaymentsNo2a`, `txtCreditableTaxNo3a`, `txtCreditableFourthQuarterNo4a`, `txtCreditableForm2316No5a`, `txtTaxPaidPreviouslyFiledNo6a`, `txtForeignTaxCreditsNo7a`, `txtSpecialTaxCreditsNo8a`, `txtOtherCreditsPaymentsNo9a` | per label | `prior_year_excess`, `quarterly_payments`, `cwt_q1_q3`, `cwt_q4`, `cwt_2316`, `previously_filed`, `foreign_tax_credits`, `special_tax_credits`, `other_credits` | amount | none | CONFIRMED |
| 1701MS | Item 28 penalties | `txtSurchangeNo28a`, `txtInterestNo28a`, `txtCompromise28a` | Item 28 | `/taxpayer/penalty_overrides/{surcharge,interest,compromise}` | amount | none | CONFIRMED |

## 11. Not verified, UNKNOWN, and limits

- No tests or builds were run. Remote reads were read-only `gh api` into scratch. Local scripts were `jq`, `rg`, and `python3 -I` over read-only files.
- Not re-read in R (local citations are labelled): the `host.rs` queue section, `form_2551q.rs` (+546), `drafts.rs` (+293/−43), `queueable.rs` (+83/−7), `penalty_items.rs` (+398/−18), `profile.rs` (+175/−12), 1701-MS typed (+86/−5), `form_1601c.rs` (+26), and most of R `draft_locks.rs`.
- `form.send` handler and the standing-OK store: MISSING by dispatch grep and file-name scan; UNKNOWN.
- BIR-side handlers for `bir_ticket_*`, `bir_app_*`, and `bir_record_payment`: UNKNOWN (absent from every repo read).
- Rox plugin source and manifest: UNKNOWN.
- Whether `official_boxes` filters locked boxes: not traced.
- Whether the 2551Q typed `compromise` writes the root field or the penalty override in R: not traced.
- The workbook (`Buwiz_Tax_Profiles_Categories_Form_Fields.xlsx`) was not provided to me. The crosswalk uses only the pinned JSON.
- Remote-only changes to note: R commit `ef5981d7` "2551Q Schedule 1 rows take an ATC choice (#172)" is a GUI change; `assist.rs` is identical in L and R, so the agent ATC path is unchanged. R `c14eacd0` and `32aee396` move draft-dependent locks into bir-core ("P4-19").

## 12. Scratch artifacts

- `S/tally_registry.py`: registry parser.
- `S/check_paths.py`: binding path check.
- `S/shape.jq`: Rox state shape, masking TIN- and ticket-like keys.
- `S/remote/`: R blobs fetched read-only (agent-form JSON, `tools.rs`, `server.rs`, `agent/mod.rs`, `draft_locks.rs`, `host.rs`, `license.rs`, `cloud_account.rs`, `QUEUE_AUTH_SCOPE.md`, and others).
- `S/bir-shell-remote/`: R `bir-shell` top-level Rust sources.