# 02. Categories and tax attributes

Owner: W2 (tax mapping data). Status: research and design. Nothing here is code, and no BIR, Rox or cloud call was made.

Labels: CONFIRMED, PARTIAL, MISSING, UNKNOWN, UNCONFIRMED. INFERENCE and PROPOSAL are marked where used. Unmarked design statements are PROPOSAL. Each section says what was **Observed** (in a file, schema or code) and what is **Recommended**.

Repo HEADs used for citations: Books `OSS/buwiz-books` 1662ca3 (read-only reference); buwiz-forms pin 63267ca (local checkout 301f9a0f is stale); buwiz-oss 0f34106 (schema copies byte-identical to the pin). Workbook sources are in `research/inputs/`.

## 1. The idea in plain words

- An **account** says what a transaction is (sales, rent, bank). It does not say what tax applies.
- A **category** is a starting point the user picks. It suggests account and tax defaults.
- A **line attribute** says the tax meaning of one posted line (for example: percentage tax, creditable withholding, employee share). Only the line decides the tax meaning.
- A **mapping rule** says which form field a line feeds, with a sign, a period and a method. Rules are versioned and dated.

**Observed (CONFIRMED):** the workbook puts tax meaning on accounts and categories. Books has no tax attribute on `journal_lines`, `accounts`, bill lines or invoice lines (books.md s2, finding 3). Sheets.md s8 #1 calls this the main conceptual problem: `salaries` feeds 1601-C II.14 and 1701-MS IV.10A.

**Recommended:** adopt the four layers. Keep an account as bookkeeping only. Put every tax meaning on the line. Version every mapping rule.

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

**Recommended:** show categories by activation only as suggestions. A category never sets a tax treatment on its own.

## 3. Line attributes

Every posted line carries these attributes. Values are closed sets. MVP values are the ones Slice A and B need.

| Attribute | Allowed values | MVP values | Default | Status | Rule |
|---|---|---|---|---|---|
| `tax_type` | income_tax; percentage_tax; vat; compensation_withholding; expanded_withholding; final_withholding; none | percentage_tax; income_tax; none | none | PROPOSAL | Which tax the line belongs to. Drives the form family. VAT and expanded withholding are later phases (P10+). |
| `atc` | PT010; WB080; WB082; WB084; WW010; II011; II012; II013; II014; II015; II016; II017; (later) WI and WC series; null | PT010; WB080; WB082; WB084 (final; never credited); WW010 (1601-C, P6); II011-II017 (1701-MS I.18; not ATCs) | - | CONFIRMED | Alphanumeric tax code. Closed set in MVP. PT010 = persons exempt from VAT under Sec. 109(BB) (Sec. 116), 3% (printed Table 1). WB080/WB082 = creditable percentage-tax withholding (2307). WB084 = final, Section 116 applies (2306). WW010 = compensation withhold… |
| `treatment` | sale_pt; cwt_credit; expense; owner_money; none; (later P7) non_op_income; (later P6) comp_income; (later P6) contribution; (later P7) final_taxed_income; (reserved 2550Q) sale_vat; (reserved 2550Q) sale_zero_rated; (re… | sale_pt; cwt_credit; expense; owner_money; none | none | PROPOSAL | How the line enters the tax computation. An account never implies a treatment (sheets.md s8 #1). |
| `income_class` | business_sales; contra_sales; non_operating_income; final_taxed_income; compensation; cost; none | business_sales; contra_sales; none | none | PROPOSAL | Which income or cost bucket the line feeds: 1701-MS IV.5 (business sales), IV.6 (contra), IV.13 or IV.20 (non-operating), IV.8 (cost). |
| `regime` | graduated; eight_percent; special_exempt; unknown | graduated (user-asserted); eight_percent and unknown block | unknown | PROPOSAL | Per taxable year. Income-tax rate regime. 'unknown' blocks output. Mixed earners cannot use the 250,000 reduction. |
| `vat_status` | non_vat; vat_registered; unknown | non_vat (asserted); others block | unknown | PROPOSAL | Per taxable year. A VAT-registered person cannot use the 8% option. Posted sales above the threshold force VAT. |
| `returns_state` | none; present_unacknowledged; acknowledged | none; present_unacknowledged blocks until acknowledged | none | PROPOSAL | Per quarter. A return or credit note in the quarter blocks the 2551Q until a person acknowledges it. |
| `person_scope` | self; spouse; employee; payor | self (spouse refused) | self | PROPOSAL | Whose tax the line belongs to. Spouse is out of MVP; employee comes with P6 (1601-C); payor with P4 (certificates). |
| `counterparty_kind` | customer; supplier; employee; owner; bank; government; payor; other | customer; owner; bank; other | - | PROPOSAL | What kind of party is on the line. The party is referenced by an opaque party_ref. No TIN is stored on a line. |
| `party_ref` | opaque UUID (never a TIN, never a name) | opaque UUID | - | PROPOSAL | Reference to a party record kept by Books. No TIN, branch or legal name appears here. |
| `comp_component` | taxable_basic; statutory_minimum_wage; holiday_ot_nsd_hazard; thirteenth_month_and_benefits; de_minimis; mandatory_contributions_employee; union_dues; other_non_taxable | - | - | CONFIRMED | Payroll component of a compensation line (P6). Maps to 1601-C II.14-II.20. Mandatory contributions and union dues feed II.19 only at the employee share. |
| `contribution_side` | employee; employer; none | - | none | CONFIRMED | Whose share a statutory contribution is. Employee share goes to 1601-C II.19. Employer share goes to expense and to the payable account, never to II.19 (EXC.EMPLOYER_SHARE_NOT_19). |
| `credit_kind` | pt; it; none | pt; none | none | CONFIRMED (series routing: INFERENCE) | Which tax a withholding credit reduces. pt = percentage tax (2551Q II.15). it = income tax (1701-MS V.3-V.5, including the 2316 credit in V.5, and 1701Q). Routing by ATC series: WB = pt; WI and WC = it (INFERENCE from the form headings, bir.md W-3). WB084 is … |
| `certificate_status` | received; pending; lost; disputed | received only credits | pending | CONFIRMED | Status of a 2307 or 2306 certificate. Only 'received' credits. Pending, lost and disputed never post a receivable (books.md s4g: BK-TS does not check it). |
| `document_kind` | sale_invoice; payment_receipt; expense_document; adjustment; credit_note; certificate_2307; payroll_register; tax_form; other | sale_invoice; payment_receipt; expense_document; adjustment | other | PROPOSAL | Kind of evidence document. A credit_note blocks 2551Q until acknowledged (UNCONFIRMED for 2551Q, bir.md PT-8). certificate_2307 is the only certificate type that can credit. |
| `evidence_ref` | evidence_id (opaque); sha256 of the stored file; document_kind; redaction_state | - | - | PROPOSAL | Pointer to stored evidence. The model sees the sha256 and kind only; OCR text is not sent by default (design R-23). |
| `tax_period_key` | null (use the default); 2026-Q3 (quarter); 2026-09 (month); 2026 (year) | null; quarter override | - | PROPOSAL | Explicit override of the default period derived from the recognition date. The default is the calendar day in Asia/Manila. The PT timing basis (invoice, receipt or entry date) is UNKNOWN (decision D4). |
| `final_tax_flag` | none; final_tax; unknown_not_stored | none; unknown_not_stored (blocks IV.13/IV.20 for interest) | unknown_not_stored | UNKNOWN | Whether the income was final-taxed (excluded from graduated or 8% business lines). Books does not store this today (sheets.md s10: UNKNOWN). |
| `deductibility` | itemized_only; osd_excluded; not_applicable | itemized_only | not_applicable | CONFIRMED | Whether a cost line can reach 1701-MS IV.8 or IV.10A. Itemized only. OSD and 8% do not deduct it separately. |
| `election_event` | form (1901 / 1905 / 2551Q item 13 / 1701Q); taxable_year; quarter; recorded_at (DATE); source_form | - | - | CONFIRMED | Dated 8% or graduated election record per taxable year. The 8% election is irrevocable within the year. Stored as an event, not overwritten (BK-TS upserts it; books.md s2). |
| `sign_and_rounding` | signed amounts; rounding at output only | - | - | PROPOSAL | Totals keep their sign. Half-away rounding is applied once, at the output line (decision D7, PROPOSAL; Forms rounding statement pending). |

Money and dates (PROPOSAL, from the design; consistent with books.md s4i):

- **storage:** integer centavos (i64); intermediates i128
- **text:** exactly two decimal places in JSON, for example "200000.00"; more than two decimals is refused
- **rates:** integer basis points (300 = 3.00%)
- **floats:** not allowed in Books money paths
- **sign:** signed; no clamp to zero; overpayment is a separate signed line
- **entry_date:** DATE in Asia/Manila (calendar day)
- **recognition_date:** DATE; may differ from entry date when the recognition rule says so (decision D4 open)
- **certificate_period:** period_start and period_end as DATE (BK-TS uses text: tax-certificates.ts:53-54)

Forbidden in Books (PROPOSAL, from the design):

- any TIN or branch code
- BIR tickets (tkt_...) or Rox tokens in records
- floats for money or rates
- account-implied tax meaning
- credit from pending, lost or disputed certificates
- WB084 as a credit

## 4. Mapping rules: category to form field

The workbook's category field rules (52 rows) map a category to a form line. Each row keeps the workbook's method, period, filter and exclusion text. The rule ID links to the rules file. The workbook text is not verified rule by rule. Rule status is in `data/rules/`.

| Category | Form | Target | Method (workbook) | Period | Filter (workbook) | Rule ID |
|---|---|---|---|---|---|---|
| `sales_revenue` | 2551Q | S1.1.base | Aggregate tax-tagged revenue | Quarter | PT010 eligible sales, applicable recognition date | 2551Q.S1.1.base |
| `sales_revenue` | 1701-MS | IV.5 | Aggregate income-eligible gross sales | Year | Filer, non-final income, exclude VAT | 1701MS.IV.5 |
| `sales_revenue` | 1701-MS | IV.19 | Aggregate eligible net sales | Year | 8% pathway only; subtract eligible returns | 1701MS.IV.19 |
| `subscription_revenue` | 2551Q | S1.1.base | Aggregate tax-tagged revenue | Quarter | PT010 eligible sales, applicable recognition date | 2551Q.S1.1.base |
| `subscription_revenue` | 1701-MS | IV.5 | Aggregate income-eligible gross sales | Year | Filer, non-final income, exclude VAT | 1701MS.IV.5 |
| `subscription_revenue` | 1701-MS | IV.19 | Aggregate eligible net sales | Year | 8% pathway only; subtract eligible returns | 1701MS.IV.19 |
| `transaction_revenue` | 2551Q | S1.1.base | Aggregate tax-tagged revenue | Quarter | PT010 eligible sales, applicable recognition date | 2551Q.S1.1.base |
| `transaction_revenue` | 1701-MS | IV.5 | Aggregate income-eligible gross sales | Year | Filer, non-final income, exclude VAT | 1701MS.IV.5 |
| `transaction_revenue` | 1701-MS | IV.19 | Aggregate eligible net sales | Year | 8% pathway only; subtract eligible returns | 1701MS.IV.19 |
| `refunds_and_discounts` | 2551Q | S1.1.base | Subtract eligible sales adjustments | Quarter | PT010 adjustment to covered sale | 2551Q.S1.1.base |
| `refunds_and_discounts` | 1701-MS | IV.6 | Sum eligible contra sales | Year | Graduated/special pathway | 1701MS.IV.6 |
| `refunds_and_discounts` | 1701-MS | IV.19 | Subtract eligible contra sales | Year | 8% pathway | 1701MS.IV.19 |
| `other_non_operating_income` | 1701-MS | IV.13 | Aggregate eligible other income | Year | Graduated / special pathway | 1701MS.IV.13 |
| `other_non_operating_income` | 1701-MS | IV.20 | Aggregate eligible other income | Year | 8% pathway and eligibility test | 1701MS.IV.20 |
| `interest_income` | 1701-MS | IV.13 | Aggregate eligible other income | Year | Graduated / special pathway | 1701MS.IV.13 |
| `interest_income` | 1701-MS | IV.20 | Aggregate eligible other income | Year | 8% pathway and eligibility test | 1701MS.IV.20 |
| `cost_of_goods_sold` | 1701-MS | IV.8 | Sum allowable cost | Year | Itemized deduction method | 1701MS.IV.8 |
| `direct_service_cost` | 1701-MS | IV.8 | Sum allowable cost | Year | Itemized deduction method | 1701MS.IV.8 |
| `salaries` | 1701-MS | IV.10A | Sum tax-eligible expenses | Year | Itemized method + deductible amount + substantiation | 1701MS.IV.10A |
| `salaries` | 1601-C | II.14 | Reconcile to payroll detail | Month | Employee gross compensation | 1601C.II.14 |
| `employee_benefits` | 1701-MS | IV.10A | Sum tax-eligible expenses | Year | Itemized method + deductible amount + substantiation | 1701MS.IV.10A |
| `employee_benefits` | 1601-C | II.14 | Reconcile to payroll detail | Month | Employee gross compensation | 1601C.II.14 |
| `retirement_contributions` | 1701-MS | IV.10A | Sum tax-eligible expenses | Year | Itemized method + deductible amount + substantiation | 1701MS.IV.10A |
| `employer_payroll_taxes` | 1701-MS | IV.10A | Sum tax-eligible expenses | Year | Itemized method + deductible amount + substantiation | 1701MS.IV.10A |
| `payroll_processing_fees` | 1701-MS | IV.10A | Sum tax-eligible expenses | Year | Itemized method + deductible amount + substantiation | 1701MS.IV.10A |
| `payment_processing_fees` | 1701-MS | IV.8 | Review classification | Year | Itemized and treated as cost of services | 1701MS.IV.8 |
| `hosting_fees` | 1701-MS | IV.8 | Review classification | Year | Itemized and treated as cost of services | 1701MS.IV.8 |
| `consultants_and_contractors` | 1701-MS | IV.10A | Sum tax-eligible expenses | Year | Itemized method + deductible amount + substantiation | 1701MS.IV.10A |
| `legal` | 1701-MS | IV.10A | Sum tax-eligible expenses | Year | Itemized method + deductible amount + substantiation | 1701MS.IV.10A |
| `tax_and_accounting` | 1701-MS | IV.10A | Sum tax-eligible expenses | Year | Itemized method + deductible amount + substantiation | 1701MS.IV.10A |
| `advertising` | 1701-MS | IV.10A | Sum tax-eligible expenses | Year | Itemized method + deductible amount + substantiation | 1701MS.IV.10A |
| `business_applications_and_software` | 1701-MS | IV.10A | Sum tax-eligible expenses | Year | Itemized method + deductible amount + substantiation | 1701MS.IV.10A |
| `rent` | 1701-MS | IV.10A | Sum tax-eligible expenses | Year | Itemized method + deductible amount + substantiation | 1701MS.IV.10A |
| `utilities` | 1701-MS | IV.10A | Sum tax-eligible expenses | Year | Itemized method + deductible amount + substantiation | 1701MS.IV.10A |
| `repairs_and_maintenance` | 1701-MS | IV.10A | Sum tax-eligible expenses | Year | Itemized method + deductible amount + substantiation | 1701MS.IV.10A |
| `office_supplies` | 1701-MS | IV.10A | Sum tax-eligible expenses | Year | Itemized method + deductible amount + substantiation | 1701MS.IV.10A |
| `airfare` | 1701-MS | IV.10A | Sum tax-eligible expenses | Year | Itemized method + deductible amount + substantiation | 1701MS.IV.10A |
| `meals` | 1701-MS | IV.10A | Sum tax-eligible expenses | Year | Itemized method + deductible amount + substantiation | 1701MS.IV.10A |
| `bank_fees` | 1701-MS | IV.10A | Sum tax-eligible expenses | Year | Itemized method + deductible amount + substantiation | 1701MS.IV.10A |
| `interest_expense` | 1701-MS | IV.10A | Sum tax-eligible expenses | Year | Itemized method + deductible amount + substantiation | 1701MS.IV.10A |
| `depreciation_expense` | 1701-MS | IV.10A | Sum tax-eligible expenses | Year | Itemized method + deductible amount + substantiation | 1701MS.IV.10A |
| `ph_cwt_receivable` | 1701-MS | V.3 | Aggregate eligible income-tax credits | Q1–Q3 | 2307 / 1606 income tax | 1701MS.V.3 |
| `ph_cwt_receivable` | 1701-MS | V.4 | Aggregate eligible income-tax credits | Q4 | 2307 / 1606 income tax | 1701MS.V.4 |
| `pct_tax_credit_receivable` | 2551Q | II.15 | Aggregate eligible percentage-tax credits | Quarter | 2307 percentage-tax withholding | 2551Q.II.15 |
| `ph_percentage_tax_payable` | 2551Q | II.14 | Reconcile tax liability | Quarter | Posted percentage-tax assessment | 2551Q.II.14 |
| `ph_percentage_tax_expense` | 1701-MS | IV.10A | Sum tax-eligible expenses | Year | Itemized method + deductible amount + substantiation | 1701MS.IV.10A |
| `income_tax_paid` | 1701-MS | V.2 | Aggregate previous quarterly income tax paid | Q1–Q3 | 1701Q payment proof | 1701MS.V.2 |
| `ph_wtc_payable` | 1601-C | II.25 | Reconcile withholding postings | Month | Posted withholding by payroll event | 1601C.II.25 |
| `ph_sss_payable` | 1601-C | II.19 | Reconcile to employee-share detail | Month | Payroll employee share only | 1601C.II.19 |
| `ph_philhealth_payable` | 1601-C | II.19 | Reconcile to employee-share detail | Month | Payroll employee share only | 1601C.II.19 |
| `ph_pagibig_payable` | 1601-C | II.19 | Reconcile to employee-share detail | Month | Payroll employee share only | 1601C.II.19 |
| `ph_union_dues_payable` | 1601-C | II.19 | Reconcile to employee-share detail | Month | Payroll employee share only | 1601C.II.19 |

## 5. Edge cases

| Case | Rule (PROPOSAL unless marked) | Status |
|---|---|---|
| Returns and credit notes | Credit note is a document kind. It links to the original sale. It blocks the 2551Q until acknowledged (GATE.RETURNS_ACK). 1701-MS IV.6 takes contra sales once (REV.COUNT_ONCE). | UNCONFIRMED for 2551Q (bir.md PT-8) |
| Void in an open period | Post a reversal dated the original entry date. Period totals net to zero (REV.VOID_OPEN_PERIOD). Otherwise Q3 gross would stay at 300,000.00 (design A4). | PROPOSAL (INFERENCE) |
| Reversal dated after the period closed | Return event. Blocks the affected proposal until acknowledged (REV.LATER_PERIOD_RETURN_EVENT). | PROPOSAL; tax treatment UNCONFIRMED |
| Employee share vs employer share | Employee share goes to 1601-C II.19. Employer share goes to expense and to the payable account. Never II.19 (EXC.EMPLOYER_SHARE_NOT_19). | CONFIRMED (RR 11-2018 text); statute UNVERIFIED |
| Income-tax CWT vs percentage-tax CWT | WB series (PT) feeds 2551Q II.15. WI and WC series (IT) feed 1701-MS V.3-V.5. Route by ATC series (EXC.IT_CWT_NOT_PT, EXC.PT_CWT_NOT_IT). | CONFIRMED rule; series routing INFERENCE (bir.md W-3) |
| WB084 final withholding | Never credited. It belongs to 2306 (EXC.WB084_FINAL). | CONFIRMED (bir.md W-2, W-4) |
| Pending, lost or disputed certificate | Creates no credit (EXC.UNRECEIVED_CERT). BK-TS posts it today (books.md s4g): defect. | CONFIRMED |
| Spouse | Refused in MVP (EXC.SPOUSE_MVP). | PROPOSAL |
| Mixed earners (compensation and business) | No 250,000 reduction on the 8% component (GATE.MIXED_INCOME_8PCT). Compensation is split from receipts. | CONFIRMED (RMO 23-2018 pts 10-12) |
| Rounding | Centavos inside. Half away from zero once, at output (decision D7). The 1701-MS whole-peso rule is UNCONFIRMED. | PROPOSAL; 1701-MS rule UNCONFIRMED |
| Negative amounts | Totals keep their sign. Overpayment is a separate signed line. No clamp to zero (2551Q II.19, II.24; 8% sheet fix, correction 15). | CONFIRMED (workbook note) |
| Tax period | Default: entry date in Asia/Manila. tax_period_key overrides. Certificates use their own period dates. PT timing basis (invoice, receipt or entry) is open. | PT timing UNKNOWN (decision D4) |
| 8% election and 2551Q | No 2551Q values in an 8% year (GATE.8PCT_YEAR_2551Q). No NIL row in MVP. | CONFLICT CONFIRMED; resolution UNCONFIRMED (decision D5) |
| VAT threshold | Posted sales year-to-date strictly above 3,000,000.00 blocks the 8% option and forces VAT (GATE.VAT_THRESHOLD). | CONFIRMED text; CPI-adjusted figure UNKNOWN |
| Penalties and overdue | Show overdue from the due date. Compute no surcharge or interest (GATE.OVERDUE; decision D14). | CONFIRMED (workbook note); owner decision D14 |
| Interest subject to final tax | Excluded from the graduated and 8% other-income lines when final tax applies. Books does not store the attribute yet. | UNKNOWN (sheets.md s10) |
| Depreciation | Use the tax-depreciation schedule, not the GL account (correction 17). | Needs review |
| Mixed income: 1701 or 1701-MS | Two activities or two regimes route to 1701 (printed note, bir.md M-6). Books has no 1701 path. | CONFIRMED routing; Books UNSUPPORTED |

## 6. Worked examples (synthetic)

All amounts are synthetic. They come from the workbook's fictional worked example or from the Slice A plan. No real financial data is used.

**Percentage tax (2551Q, Slice A).** Posted sales: 120,000.00 (2026-07-01), 100,000.00 (2026-07-15), 80,000.00 (2026-08-20). The 100,000.00 sale is voided by a reversal dated 2026-07-15. Base = 200,000.00. Tax = 200,000.00 x 3% = 6,000.00 (PT010, rate row 300 bps). Due date: 2026-10-25 (quarter end + 25 days). As of 2026-10-29 the return is overdue by 4 days. Books computes no surcharge.

**Rounding tie (golden test).** 1.50 at 3% gives 0.05. 1.49 gives 0.04. -1.50 gives -0.05. Python's built-in round() would give 0.04 for 1.50 (banker's rounding), so the test must assert half-away explicitly.

**1601-C employee share (BK-TS defect).** Basic 40,000.00 plus de minimis 2,000.00 (total 42,000.00). Employee SSS share 2,000.00. Non-taxable total 4,000.00. Taxable compensation 38,000.00. BK-TS gives 40,000.00 because it drops the employee share (verify.md s2 #28).

**1701-MS graduated (workbook example).** Sales 1,200,000.00 less returns 10,000.00 = net 1,190,000.00. OSD 40% = 476,000.00. Taxable business income 714,000.00. Graduated tax (2023+ table) = 22,500.00 + 20% x (714,000.00 - 400,000.00) = 85,300.00 (rates-v1.json test vector).

## 7. Observed discrepancies that affect categories and rules

| Item | Observed | Status | Owner |
|---|---|---|---|
| Account type labels | Workbook used 'contra revenue', 'other income', 'cost', 'other expense'. Books enum is revenue, other_income, cost_of_revenue, other_expense (account-constants.ts:9-18). | Corrected (correction 19) | Workbook |
| Three PH accounts missing | 12700, 12800 and 61950 were not in the catalog. | Added (correction 21; verify s2 #40) | Workbook |
| owner_drawings | Proposed key. Books has owners_equity 32000 for distributions to owners. | Pending owner decision (correction 20) | Owner |
| Activation cannot tell 8% from graduated | The IT flag covers both (sheets.md s8 #10). | Limitation | Workbook |
| Profile C11-C15 unused | No formula reads them (verify.md s2 #58). | CONFIRMED | Workbook |
| Interest final tax | Attribute not stored (sheets.md s10). | UNKNOWN | Books |
| OSD base wording | Part I item 17 says gross sales; Part IV item 11 says net sales (bir.md s2). | UNCONFIRMED (conflict) | Tax adviser |
| Credit notes in 2551Q | No printed deduction found (bir.md PT-8). The workbook example deducts 10,000. | UNCONFIRMED | Owner (D20) |

## 8. Open owner decisions

| ID | Decision | Default in this design | Blocks |
|---|---|---|---|
| D4 | PT timing basis and other-income treatment | Entry date, with a flag; tax adviser decides | P2 non-default cases |
| D5 | 2551Q in an 8% year | Block; read RR 8-2018 full text | Any 8%-year output |
| D6 | 1% PT extension to 2025-12-31 | Off; warn; obtain the primary text | 2023-25 proposals as ready |
| D7 | Rounding (half away, once) | Yes | P3 golden tie |
| D14 | Overdue Q3 2026 return | Show overdue; no penalties | Slice A status text |
| D20 | Returns and credit notes | Block until acknowledged | P2 |
| owner_drawings | Reuse owners_equity 32000 or new key | Pending | Category catalog |

**Recommended:** resolve D5 and D20 before P2 is built. Keep the crosswalk and rules files as the single source for form mappings.
