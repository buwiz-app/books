# 02. Categories, line attributes and measures

Owner: W2 (tax mapping data). Status: research and design. Nothing here is code, and no BIR, Rox or cloud call was made.

Labels: CONFIRMED, PARTIAL, MISSING, UNKNOWN, UNCONFIRMED. INFERENCE and PROPOSAL are marked where used. Unmarked design statements are PROPOSAL. Each section says what was **Observed** (in a file, schema or code) and what is **Recommended**.

**Revised for [ADR 0007](adr/0007-books-supplies-measures-forms-owns-compliance.md) (2026-10-29).** Line attributes are accounting and evidence facts. Measures are the Books output. The bridge maps measures to Forms input boxes. Tax attributes, elections, rates and the compliance edge cases moved to Forms; the old attribute catalogue is archived at `research/handoff-to-forms/rules-r1/tax-attributes.r1.json`.

Repo HEADs used for citations: Books `OSS/buwiz-books` 1662ca3 (read-only reference); buwiz-forms pin 63267ca (local checkout 301f9a0f is stale); buwiz-oss 0f34106 (schema copies byte-identical to the pin). Workbook sources are in `research/inputs/`.

## 1. The idea in plain words

- An **account** says what a transaction is (sales, rent, bank). It does not say which form box it feeds.
- A **category** is a starting point the user picks. It suggests an account and default line attributes. Suggestions may use the linked tax profile's registered-forms list as a hint (for example, show payroll categories when 1601-C is registered). That list is never an election.
- A **line attribute** records an accounting or evidence fact on one posted line (for example: a sales return, an employee contribution, a received percentage-tax 2307). Only the line decides.
- A **measure** is an accounting total per business profile per period, with lineage (for example net sales, gross compensation). Catalogue: `data/measures.v1.json`.
- A **mapping rule** (the bridge) says which Forms input box a measure feeds. Rules are versioned. Where a box depends on a taxpayer choice, the rule reads that choice from Forms.

**Observed (CONFIRMED):** the workbook puts tax meaning on accounts and categories. Books has no tax attribute on `journal_lines`, `accounts`, bill lines or invoice lines (books.md s2, finding 3). Sheets.md s8 #1 calls this the main conceptual problem: `salaries` feeds 1601-C II.14 and 1701-MS IV.10A.

**Recommended:** adopt the five layers (account, category, line attributes, measures, mapping). Keep an account as bookkeeping only. Put every fact on the line. Version every mapping rule. Keep compliance (elections, rates, thresholds, tax) in Forms.

## 2. Category catalog

The catalog has 62 categories: 59 from the workbook (after the corrections) plus 3 PH control accounts that were missing (correction 21). 54 reuse an existing Books key and number. 2 are new children under an existing Books parent. 5 have no Books equivalent. 1 waits for an owner decision (`owner_drawings`). 13 are core (always shown). Source: `data/categories.v1.json`.

**Observed (CONFIRMED):** 51 of the workbook's 59 categories reuse a Books key and number that match the pinned preset (sheets.md s4; verify.md s2 #39). The other 8 are proposed and have no Books key. The 3 added PH rows match ph-chart.ts.

| Category | Name | Statement section | Books type (candidate) | Books key and number | Status | Activation | Core | Form links (rule IDs) |
|---|---|---|---|---|---|---|---|---|
| `bank_accounts` | Bank Accounts | asset | asset | bank_accounts / 11000 | REUSE | Always | yes | - |
| `accounts_receivable_gross` | Accounts Receivable - Gross | asset | asset | accounts_receivable_gross / 12010 | REUSE | Always | yes | - |
| `inventory` | Inventory | asset | asset | inventory / 13500 | REUSE | Always | yes | - |
| `prepaid_expenses` | Prepaid Expenses | asset | asset | prepaid_expenses / 13000 | REUSE | Always | yes | - |
| `computer_assets` | Computer Assets | asset | asset | computer_assets / 16100 | REUSE | Always | yes | - |
| `accounts_payable` | Accounts Payable | liability | liability | accounts_payable / 21000 | REUSE | Always | yes | - |
| `owners_equity` | Owners Equity | equity | equity | owners_equity / 32000 | REUSE | Always | yes | - |
| `owner_drawings` | Owner Drawings | equity | equity | (reuse owners_equity / 32000? pending) | PENDING_DECISION | INDIVIDUAL | no | - |
| `loan_payable` | Loans Payable | liability | liability | (no Books key) | NEW | Always | yes | - |
| `sales_revenue` | Sales Revenue | income | revenue | sales_revenue / 41000 | REUSE | Always | yes | 1701MS.IV.19, 1701MS.IV.5, 2551Q.S1.1.base |
| `subscription_revenue` | Subscription Revenue | income | revenue | subscription_revenue / 42000 | REUSE | Always | yes | 1701MS.IV.19, 1701MS.IV.5, 2551Q.S1.1.base |
| `transaction_revenue` | Transaction Revenue | income | revenue | transaction_revenue / 43000 | REUSE | Always | yes | 1701MS.IV.19, 1701MS.IV.5, 2551Q.S1.1.base |
| `refunds_and_discounts` | Refunds & Discounts | income (contra) | revenue (subtype refunds_discounts) | refunds_and_discounts / 49000 | REUSE | Always | yes | 1701MS.IV.19, 1701MS.IV.6, 2551Q.S1.1.base |
| `other_non_operating_income` | Other Non-Operating Income | income | other_income | (child of other_income / 80000) | NEW_UNDER_PARENT | IT | no | 1701MS.IV.13, 1701MS.IV.20 |
| `interest_income` | Interest Income | income | other_income | interest_income / 81000 | REUSE | IT | no | 1701MS.IV.13, 1701MS.IV.20 |
| `cost_of_goods_sold` | Cost of Goods Sold | expense | cost_of_revenue | cost_of_goods_sold / 51000 | REUSE | Always | yes | 1701MS.IV.8 |
| `direct_service_cost` | Direct Cost of Services | expense | cost_of_revenue | (child of cost_of_revenue / 50000) | NEW_UNDER_PARENT | IT | no | 1701MS.IV.8 |
| `salaries` | Salaries | expense | expense | salaries / 61100 | REUSE | IT_OR_PAYROLL | no | 1601C.II.14, 1701MS.IV.10A |
| `employee_benefits` | Employee Benefits | expense | expense | employee_benefits / 61400 | REUSE | IT_OR_PAYROLL | no | 1601C.II.14, 1701MS.IV.10A |
| `retirement_contributions` | Retirement Contributions | expense | expense | retirement_contributions / 61500 | REUSE | IT_OR_PAYROLL | no | 1701MS.IV.10A |
| `employer_payroll_taxes` | Employer Payroll Taxes | expense | expense | employer_payroll_taxes / 61600 | REUSE | IT_OR_PAYROLL | no | 1701MS.IV.10A |
| `payroll_processing_fees` | Payroll Processing Fees | expense | expense | payroll_processing_fees / 61700 | REUSE | IT_OR_PAYROLL | no | 1701MS.IV.10A |
| `payment_processing_fees` | Payment Processing Fees | expense | cost_of_revenue | payment_processing_fees / 53000 | REUSE | IT | no | 1701MS.IV.8 |
| `hosting_fees` | Hosting Fees | expense | cost_of_revenue | hosting_fees / 54000 | REUSE | IT | no | 1701MS.IV.8 |
| `consultants_and_contractors` | Consultants and Contractors | expense | expense | consultants_and_contractors / 63100 | REUSE | IT | no | 1701MS.IV.10A |
| `legal` | Legal | expense | expense | legal / 63200 | REUSE | IT | no | 1701MS.IV.10A |
| `tax_and_accounting` | Tax & Accounting | expense | expense | tax_and_accounting / 63300 | REUSE | IT | no | 1701MS.IV.10A |
| `advertising` | Advertising | expense | expense | advertising / 64100 | REUSE | IT | no | 1701MS.IV.10A |
| `business_applications_and_software` | Business Applications & Software | expense | expense | business_applications_and_software / 65000 | REUSE | IT | no | 1701MS.IV.10A |
| `rent` | Rent | expense | expense | rent / 66100 | REUSE | IT | no | 1701MS.IV.10A |
| `utilities` | Utilities | expense | expense | utilities / 66200 | REUSE | IT | no | 1701MS.IV.10A |
| `repairs_and_maintenance` | Repairs and Maintenance | expense | expense | repairs_and_maintenance / 66300 | REUSE | IT | no | 1701MS.IV.10A |
| `office_supplies` | Office Supplies | expense | expense | office_supplies / 67200 | REUSE | IT | no | 1701MS.IV.10A |
| `airfare` | Airfare | expense | expense | airfare / 62100 | REUSE | IT | no | 1701MS.IV.10A |
| `meals` | Meals | expense | expense | meals / 62400 | REUSE | IT | no | 1701MS.IV.10A |
| `bank_fees` | Bank Fees | expense | expense | bank_fees / 68300 | REUSE | IT | no | 1701MS.IV.10A |
| `interest_expense` | Interest Expense | expense | other_expense | interest_expense / 91000 | REUSE | IT | no | 1701MS.IV.10A |
| `depreciation_expense` | Depreciation Expense | expense | expense | (no Books key) | NEW | IT | no | 1701MS.IV.10A |
| `ph_cwt_receivable` | Creditable Withholding Tax Receivable | asset | asset | ph_cwt_receivable / 12600 | REUSE | IT | no | 1701MS.V.3, 1701MS.V.4 |
| `pct_tax_credit_receivable` | Percentage Tax Withheld Receivable | asset | asset | (no Books key) | NEW | PT | no | 2551Q.II.15 |
| `ph_percentage_tax_payable` | Percentage Tax Payable | liability | liability | ph_percentage_tax_payable / 21800 | REUSE | PT | no | 2551Q.II.14 |
| `ph_percentage_tax_expense` | Percentage Tax Expense | expense | other_expense | ph_percentage_tax_expense / 94100 | REUSE | PT | no | 1701MS.IV.10A |
| `income_tax_paid` | Income Tax Payments / Credits | asset | asset | (no Books key) | NEW | IT | no | 1701MS.V.2 |
| `income_tax_payable` | Income Tax Payable | liability | liability | (no Books key) | NEW | IT | no | - |
| `ph_wtc_payable` | Withholding Tax on Compensation Payable | liability | liability | ph_wtc_payable / 25110 | REUSE | PAYROLL | no | 1601C.II.25 |
| `ph_sss_payable` | SSS Contributions Payable | liability | liability | ph_sss_payable / 25120 | REUSE | PAYROLL | no | 1601C.II.19 |
| `ph_philhealth_payable` | PhilHealth Contributions Payable | liability | liability | ph_philhealth_payable / 25130 | REUSE | PAYROLL | no | 1601C.II.19 |
| `ph_pagibig_payable` | Pag-IBIG Contributions Payable | liability | liability | ph_pagibig_payable / 25140 | REUSE | PAYROLL | no | 1601C.II.19 |
| `ph_union_dues_payable` | Union Dues Payable | liability | liability | ph_union_dues_payable / 25150 | REUSE | PAYROLL | no | 1601C.II.19 |
| `ph_net_pay_payable` | Net Pay Payable | liability | liability | ph_net_pay_payable / 25170 | REUSE | PAYROLL | no | - |
| `ph_employee_tax_refund_payable` | Employee Tax Refund Payable | liability | liability | ph_employee_tax_refund_payable / 25175 | REUSE | PAYROLL | no | - |
| `ph_ewt_payable` | Expanded Withholding Tax Payable | liability | liability | ph_ewt_payable / 21600 | REUSE | EWT | no | - |
| `ph_input_vat` | Input VAT | asset | asset | ph_input_vat / 13800 | REUSE | VAT | no | - |
| `ph_input_vat_unpaid` | Input VAT - Unpaid Payables | asset | asset | ph_input_vat_unpaid / 13810 | REUSE | VAT | no | - |
| `ph_output_vat` | Output VAT | liability | liability | ph_output_vat / 21700 | REUSE | VAT | no | - |
| `ph_output_vat_uncollected` | Output VAT - Uncollected Receivables | liability | liability | ph_output_vat_uncollected / 21710 | REUSE | VAT | no | - |
| `ph_vat_payable_net` | VAT Payable - Net | liability | liability | ph_vat_payable_net / 21750 | REUSE | VAT | no | - |
| `ph_creditable_vat_withheld` | Creditable VAT Withheld | asset | asset | ph_creditable_vat_withheld / 12610 | REUSE | VAT | no | - |
| `ph_input_vat_exempt_expense` | Input VAT Attributable to Exempt Sales | expense | expense | ph_input_vat_exempt_expense / 68400 | REUSE | VAT | no | - |
| `ph_employee_advances` | Employee Advances & Receivables | asset | asset | ph_employee_advances / 12700 | REUSE | PAYROLL | no | - |
| `ph_employee_tax_advanced` | Employee Receivable - Tax Advanced | asset | asset | ph_employee_tax_advanced / 12800 | REUSE | PAYROLL | no | - |
| `ph_unrecovered_employee_tax` | Unrecovered Employee Tax | expense | expense | ph_unrecovered_employee_tax / 61950 | REUSE | PAYROLL | no | - |

Selection logic (observed, workbook formula, identical in all 59 rows):

| Activation rule | Shown when |
|---|---|
| Always | Always shown (CORE). |
| INDIVIDUAL | Shown when Profile C6 = Individual. |
| PT | Shown when Profile C7 = Yes (2551Q selected). |
| PAYROLL | Shown when Profile C8 = Yes (1601-C selected). |
| IT | Shown when an income-tax form is selected: Profile C9 (1701-MS), G6 (1701Q), G7 (1702 group) or G10 (1701/1701A). |
| IT_OR_PAYROLL | Shown when C8 = Yes (1601-C) or any income-tax form is selected (C9, G6, G7, G10). |
| VAT | Shown when Profile G8 = Yes (2550Q). |
| EWT | Shown when Profile G9 = Yes (0619-E or 1601-EQ). |

**Limitation (observed):** the profile cannot tell 8% from graduated income tax, and cells C11-C15 are not used by any formula (sheets.md s1.1, verify.md s2 #58).

**Recommended:** show categories by activation only as suggestions. The activation reads the registered-forms list as a hint, never an election: categories never depend on 8% versus graduated, OSD versus itemized, or VAT status. `data/categories.v1.json` now carries `default_line_attributes` (accounting facts) and the `measures` each category typically feeds.

## 3. Line attributes

Every posted line carries these attributes. Values are closed sets. Source: `data/tax-attributes.v1.json` (schema_version 2; the path keeps its v1 name).

| Attribute | Allowed values | Default | Status | Rule |
|---|---|---|---|---|
| `measure_side` | debit; credit | - | PROPOSAL | Which side is positive for the line's measure. A reversal mirrors it, so its amount is the exact negative. |
| `income_class` | business_sales; contra_sales; non_operating_income; cost_of_sales; operating_expense; none | none | PROPOSAL | Statement bucket. Output VAT, payables and receivables are none. |
| `contra_kind` | return; allowance; discount | - | PROPOSAL | Required on contra_sales. Three separate measures. |
| `person_scope` | self; spouse; employee; payor | self | PROPOSAL | Whose activity the line records. Spouse is measured but refused by the bridge in MVP. |
| `counterparty_kind` | customer; supplier; employee; owner; bank; government; payor; other | - | PROPOSAL | Party kind. The party is an opaque party_ref. No TIN on a line. |
| `party_ref` | opaque id (never a TIN, never a name) | - | PROPOSAL | Party record. Payroll 13th-month lines carry the employee's ref (per-employee cap, 1601-C item 17). |
| `comp_component` | basic_pay; statutory_minimum_wage; holiday_ot_nsd_hazard; thirteenth_month_and_benefits; de_minimis; contribution; tax_withheld | - | PROPOSAL | Payroll component as recorded by payroll. tax_withheld is a fact; Books does not compute withholding. |
| `contribution_side` | employee; employer; none | none | CONFIRMED | Employee and employer shares are separate measures (EXC.EMPLOYER_SHARE_NOT_19). |
| `contribution_scheme` | sss_gsis; philhealth; pagibig; union_dues | - | PROPOSAL | Per-scheme breakdown. |
| `credit_kind` | pt; it; none | none | CONFIRMED (series routing: INFERENCE) | Which tax a received certificate reports as withheld (WB = pt; WI, WC = it). |
| `certificate_atc` | WB080; WB082; WB084; WI010; WC010 | - | PROPOSAL | The ATC printed on a received certificate. Evidence only; Books never selects an ATC for a return. |
| `certificate_status` | received; pending; lost; disputed | pending | CONFIRMED | Only received counts in a certificate measure. |
| `document_kind` | sale_invoice; payment_receipt; expense_document; adjustment; credit_note; certificate_2307; certificate_2306; certificate_2316; payroll_register; other | other | PROPOSAL | A credit_note is a return event. A 2306 (final withholding) never counts in a 2307 measure. |
| `evidence_ref` | evidence_id; sha256; document_kind; redaction_state | - | PROPOSAL | The model sees the sha256 and kind only. |
| `final_tax_withheld` | yes; no; unknown | unknown | UNKNOWN | Evidence fact. Its effect on 1701-MS IV.13/IV.20 is an open mapping question (EXC.FINAL_TAX_INTEREST). |

**Book setting, not a line attribute:** `accounting_basis` on the business profile, `cash` (default) or `accrual` ([ADR 0008](adr/0008-cash-basis-by-default.md)). A cash-basis book posts income when the money is received and expenses when they are paid, and refuses trade receivable and payable accounts (`accrual_only` in the category catalogue). `period_override` is removed: the basis makes it unnecessary.

**Removed by ADR 0007 (Forms-supplied):** `tax_type`, `atc` (as a choice), `treatment`, `regime`, `vat_status`, `returns_state`, `election_event`, `deductibility`, `sign_and_rounding`. Where the bridge needs a taxpayer choice, it reads it from the Forms draft (`form_context`).

Money and dates (PROPOSAL, consistent with books.md s4i):

- **storage:** integer centavos (i64); intermediates i128
- **text:** exactly two decimal places in JSON, for example "200000.00"; more than two decimals is refused
- **floats:** not allowed in Books money paths
- **sign:** signed; no clamp to zero
- **entry_date:** DATE in Asia/Manila (calendar day); the recognition date set by the book's basis: the day money moved on a cash-basis book (default), the invoice or bill date on an accrual book (ADR 0008)
- **certificate_period:** period_start and period_end as DATE (BK-TS uses text: tax-certificates.ts:53-54)

Forbidden in Books (PROPOSAL):

- any TIN or branch code
- BIR tickets (tkt_...) or Rox tokens in records
- floats for money
- account-implied meaning
- compliance choices on a line or profile (regime, election, VAT status, ATC selection, deduction method)
- rates, thresholds, due dates, penalties, tax rounding

### 3.1 Measures

`data/measures.v1.json` lists each measure with its unit (centavos), sign convention, period basis, person scope, the categories and attributes it is derived from, and its lineage requirement. Measures in the catalogue: gross_sales; sales_returns; sales_allowances; sales_discounts; net_sales; cost_of_sales; operating_expenses_by_category; other_non_operating_income; gross_compensation; basic_pay; mwe_basic_pay; holiday_ot_nsd_hazard_pay; thirteenth_month_and_benefits_paid; de_minimis_paid; employee_mandatory_contributions; employer_mandatory_contributions; tax_withheld_from_employees; pt_withheld_2307_received; it_withheld_2307_received; certificates_2316_received (planned).

## 4. Mapping rules: category to form field

Books does not map categories to form fields. A category feeds one or more **measures** ([`data/measures.v1.json`](../data/measures.v1.json)), and a versioned mapping rule maps a measure to a Forms input box ([`data/rules/*.rules.json`](../data/rules/), r2; [03 crosswalk](03-crosswalk.md) shows who fills each box). The workbook's 52 category-to-line rows used a different model, and their text names some compliance choices (8% pathway, itemized method, eligibility). They are kept as research evidence in [`research/workbook-form-links.md`](../research/workbook-form-links.md), and as `workbook_form_links` in the category catalogue. Every rule ID they cite resolves to an r2 mapping or a `not_mapped` pointer.

## 5. Edge cases

| Case | Rule (PROPOSAL unless marked) | Status |
|---|---|---|
| Returns, allowances, discounts and credit notes | Separate measures (contra_kind). A return event holds the mappings that read sales measures until acknowledged (GATE.RETURNS_ACK). 1701-MS IV.6 takes the three reductions once (REV.COUNT_ONCE). Which measure feeds 2551Q row 1 is a mapping question (former D20). | Mapping open for tax review |
| Void in an open period | Post a reversal dated the original entry date. Period measures net to zero (REV.VOID_OPEN_PERIOD). | PROPOSAL (INFERENCE) |
| Reversal dated after the period closed | Return event; counted once, in its own period (REV.LATER_PERIOD_RETURN_EVENT). | PROPOSAL |
| Employee share vs employer share | Separate measures. Only the employee measure feeds 1601-C II.19 (EXC.EMPLOYER_SHARE_NOT_19). | CONFIRMED (RR 11-2018 text); statute UNVERIFIED |
| Income-tax vs percentage-tax certificates | Separate measures by credit_kind. PT feeds 2551Q II.15; IT is shown for 1701-MS V.3-V.4 as reference (EXC.IT_CWT_NOT_PT, EXC.PT_CWT_NOT_IT). | CONFIRMED rule; series routing INFERENCE (bir.md W-3) |
| 2306 final withholding (WB084) | Never in a 2307 measure (EXC.WB084_FINAL). | CONFIRMED (bir.md W-2, W-4) |
| Pending, lost or disputed certificate | Not in any certificate measure (EXC.UNRECEIVED_CERT). BK-TS posts it today (books.md s4g): defect. | CONFIRMED |
| Spouse | Measured; refused by the bridge in MVP (EXC.SPOUSE_MVP). | PROPOSAL |
| 13th month and other benefits | Measured gross. The bridge computes the non-taxable part for 1601-C item 17 (annual cap per employee): the one input-level tax rule in the bridge. | Open for the Forms owner (07 DD-29) |
| Negative amounts | Measures keep their sign. A negative sales measure is not proposed for a box. No clamp. | CONFIRMED (workbook note) |
| Recognition period | The entry date in Asia/Manila, set by the book's basis: cash (default) posts on the day money moves, accrual at invoice or bill. Certificates use their own period dates. | Decided (ADR 0008) |
| Interest subject to final tax | Evidence fact final_tax_withheld; mapping to IV.13/IV.20 fails closed until reviewed (EXC.FINAL_TAX_INTEREST). | UNKNOWN (sheets.md s10) |
| Depreciation | Book depreciation is an operating expense measure. A tax-depreciation schedule (correction 17) is Forms' concern. | Needs review |
| 8% election, VAT threshold, mixed income, rounding, penalties, overdue, 1701 vs 1701-MS routing | Not Books rules. | Handed to Forms (07 section 2.2; `research/handoff-to-forms/compliance-notes.md`) |

## 6. Worked examples (synthetic)

All amounts are synthetic. They come from the workbook's fictional worked example or from the Slice A plan. No real financial data is used.

**Sales measures (Slice A).** Posted sales: 120,000.00 (2026-07-01), 100,000.00 (2026-07-15), 80,000.00 (2026-08-20). The 100,000.00 sale is voided by a reversal dated 2026-07-15. Net sales Q3 = 200,000.00, with lineage to the two live sales. The bridge maps it to 2551Q `txtATCAmt1`. Forms computes the tax (6,000.00 at 3%) and owns the due date.

**Workbook quarter 3.** Gross sales 300,000.00; returns 10,000.00 (a separate measure); net sales 290,000.00 → `txtATCAmt1` (mapping uses net sales; open for tax review).

**1601-C employee share (BK-TS defect).** Basic 40,000.00 plus de minimis 2,000.00: gross compensation 42,000.00 → item 14. De minimis 2,000.00 → item 18. Employee SSS share 2,000.00 → item 19. The employer share 1,500.00 is a separate measure and never reaches item 19. Forms computes the non-taxable total and taxable compensation. BK-TS drops the employee share (verify.md s2 #28).

**1701-MS (workbook example).** Gross sales 1,200,000.00, returns 10,000.00, net sales 1,190,000.00. If the Forms draft says graduated: IV.5 = 1,200,000.00 and IV.6 = 10,000.00. If it says eight_percent: IV.19 = 1,190,000.00. Forms computes OSD, taxable income and the tax.

## 7. Observed discrepancies that affect categories and rules

| Item | Observed | Status | Owner |
|---|---|---|---|
| Account type labels | Workbook used 'contra revenue', 'other income', 'cost', 'other expense'. Books enum is revenue, other_income, cost_of_revenue, other_expense (account-constants.ts:9-18). | Corrected (correction 19) | Workbook |
| Three PH accounts missing | 12700, 12800 and 61950 were not in the catalog. | Added (correction 21; verify s2 #40) | Workbook |
| owner_drawings | Proposed key. Books has owners_equity 32000 for distributions to owners. | Pending owner decision (correction 20) | Owner |
| Activation cannot tell 8% from graduated | The IT flag covers both (sheets.md s8 #10). | Intended: categories never depend on the election (ADR 0007) | Workbook |
| Profile C11-C15 unused | No formula reads them (verify.md s2 #58). | CONFIRMED | Workbook |
| Interest final tax | Attribute not stored (sheets.md s10). | UNKNOWN | Books |
| OSD base wording | Part I item 17 says gross sales; Part IV item 11 says net sales (bir.md s2). | UNCONFIRMED (conflict). Handed to Forms. | Forms owner |
| Credit notes in 2551Q | No printed deduction found (bir.md PT-8). The workbook example deducts 10,000. | UNCONFIRMED. Mapping question (former D20). | Tax review |

## 8. Open owner decisions

| ID | Decision | Default in this design | Blocks |
|---|---|---|---|
| D20 | Which sales measure feeds 2551Q row 1 (now a mapping question) | net_sales, review open | P2 release (tax review) |
| DD-29 | 1601-C item 17 cap: Forms or the bridge | The bridge keeps it, labelled | P6 |
| owner_drawings | Reuse owners_equity 32000 or new key | Pending | Category catalog |

D5, D6, D7 and D14 are handed to Forms ([07 section 2.2](07-decisions-and-blockers.md#22-handed-to-forms-compliance-side)).

**Recommended:** settle the D20 mapping in tax review before P2 is released. Keep the crosswalk, the measures catalogue and the mapping rule sets as the single source for what Books proposes.
