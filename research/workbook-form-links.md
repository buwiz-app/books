# Workbook form links (research trail)

Moved here from `docs/02-categories-and-attributes.md` section 4 on 2026-10-29. This is the workbook's own text, kept as evidence: `Buwiz_Tax_Profiles_Categories_Form_Fields.xlsx`, sheet "Category field rules", 52 rows. Corrections are in [`spreadsheet-corrections.md`](spreadsheet-corrections.md).

**Not a Books rule.** The Method, Period and Filter columns are copied as the workbook wrote them. Some cells state compliance choices: "PT010 eligible sales", "8% pathway", "itemized method", "eligibility test". Those belong to Buwiz Forms ([ADR 0007](../docs/adr/0007-books-supplies-measures-forms-owns-compliance.md)). In Books a category feeds a **measure** ([`data/measures.v1.json`](../data/measures.v1.json)), and a versioned mapping rule maps the measure to a Forms input box ([`data/rules/`](../data/rules/), r2). Each Rule ID below now resolves to an r2 mapping or to a `not_mapped` entry, whose owner is `forms`, `user`, `profile` or `prior_filing`. The same text is kept in [`data/categories.v1.json`](../data/categories.v1.json) as `workbook_form_links`.

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
