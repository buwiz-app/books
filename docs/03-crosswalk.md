# 03. Crosswalk: Buwiz Forms keys, typed aliases and Books sources

Owner: W2 (tax mapping data). Status: research and design. Nothing here is code, and no BIR, Rox or cloud call was made.

Repo HEADs: buwiz-forms pin 63267ca (local 301f9a0f is stale); buwiz-oss 0f34106 (byte-identical schema copies); Books `OSS/buwiz-books` 1662ca3 (reference only).

Labels: CONFIRMED, PARTIAL, MISSING, UNKNOWN, UNCONFIRMED. INFERENCE and PROPOSAL are marked where used. Unmarked design statements are PROPOSAL. Each statement is either **Observed** (read in a file, schema, workbook or code) or **Recommended** (a design proposal).

## 1. Observed: the pinned schemas

| Form | Edition | Pinned file (buwiz-forms, commit 63267ca) | Full sha256 | Boxes | Bindings | Unmapped |
|---|---|---|---|---|---|---|
| 2551Q | January 2018 (ENCS) | `crates/bir-core/data/agent-forms/2551q-v2018.json` | `a206416cabb09c5e1e0ebedf9f7ac635a46856a139ffbbde5f7be88f89005788` | 105 | 39 | 43 |
| 1601C | January 2018 (ENCS) | `crates/bir-core/data/agent-forms/1601c-v2018.json` | `07ca2f04da430efa12b9fa30ca4ab4fa37bb6a226c397783a3d1e4358e7bc251` | 113 | 57 | 42 |
| 1701MS | August 2024 (08/24) | `crates/bir-core/data/agent-forms/1701ms-v2024.json` | `6cf45329d89a09d6436de58df4b0e65acb7d68d91a91bf299c3b3b635dcaab84` | 230 | 119 | 58 |

- **Observed (CONFIRMED):** the pinned commit is `63267ca3d4d526a8ec490394470f026de82ac385`. It is a commit in the local `OSS/buwiz-forms` object store. The checked-out HEAD `301f9a0f` is stale and was not used (verify.md s1).
- **Observed (CONFIRMED):** the buwiz-oss copies (`0f34106`) of these three files are byte-identical to the pin (sha256 match).
- **Observed (CONFIRMED):** the stale local 2551Q (`67188ca6966901b9`) and 1701-MS (`85a4fd3d9f2d7f17`) files differ from the pin. The local 2551Q leaves items 20-21 unbound and binds item 22 to `/compromise` (verify.md s2 #5). Do not use the local checkout.
- **Observed:** every binding and every unmapped key is a box key in the pin. No box key is duplicated. The generator checked every key, path and lock against the file (see section 8).

## 2. How to read the tables

| Column | Meaning |
|---|---|
| Sheet ID | The ID used in the workbook field tab. It is a spreadsheet label, not an API key. Rule IDs use it (for example `2551Q.II.15`). |
| Printed item | The item number in the pinned schema box. For 1701-MS, items repeat across parts, so use the Sheet ID. |
| Spreadsheet label | The BIR field text on the workbook row with that Sheet ID. |
| ACTUAL API KEY | The exact key in the pinned schema (`bindings` or `boxes`). The fill tool may refuse it (see Fill key). |
| Typed alias | A typed name the fill code accepts, traced in verify.md s3-s4. Not a schema key. `[UNCONFIRMED]` marks a write target that was not traced. |
| Draft path | Where the value lands in the BIR draft (from the pin). |
| Fill class | `input`, `computed`, `locked_profile`, `locked_election`, `locked_identity` or `not_fillable`. Locks come from the pin. |
| Books source / policy | Where Books would get the value, and whether Books may send it. Policy values: `fill_mvp` (Slice A/B), `fill_p6`, `fill_p7`, `cross_check` (Forms computes; Books compares), `never_fill`, `refuse_mvp_spouse`, `never_touch`. |
| Fill key | The exact string Books may put in `fill_args`. The official key when accepted; the typed alias when only that is accepted; empty when no key is accepted. |

**Observed (CONFIRMED):** the fill tool accepts exact official keys, including the `frm<Form>:` prefix. Bare keys such as `txt15` are refused. The bare keys that exist in the pin (for example `txtATCAmt1`, `txtEmail#occurrence-1` and `txtEmail`) are accepted. `drpATC1` is refused because the app derives it. For 2551Q and 1601-C, one bad key fails the whole call (verify.md s1 #1, s3).

## 3. Verified crosswalk: Form 2551Q (quarterly percentage tax)

94 schema fields (all boxes, group members listed under their parent). Source: `data/crosswalk/2551q-v2018.json`.

| Sheet ID (workbook) | Printed item (schema) | Spreadsheet label (workbook text) | ACTUAL API KEY (pinned schema) | Typed alias (code, verify s3-s4) | Draft path (pinned) | Fill class | Books source / policy | Status | Scope | Fill key Books may send |
|---|---|---|---|---|---|---|---|---|---|---|
| H.1 | 1 | Calendar or fiscal year | frm2551Qv2018:forThe; members: forThe_1, forThe_2 | tax_period_basis | /tax_period_basis | input | profile / never_fill | CONFIRMED | none | frm2551Qv2018:forThe |
| H.2 | 2 | Year ended (MM/YYYY) | frm2551Qv2018:rtnMonth | year_end_month | /year_end_month | input | profile / never_fill | CONFIRMED | none | frm2551Qv2018:rtnMonth |
| H.2 | 2 | Year ended (MM/YYYY) | frm2551Qv2018:txtYear | - | /taxable_year | locked_identity lock=identity | none / never_fill | CONFIRMED | none | - |
| H.3 | 3 | Quarter | frm2551Qv2018:qtr_1 | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| H.3 | 3 | Quarter | frm2551Qv2018:qtr_2 | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| H.3 | 3 | Quarter | frm2551Qv2018:qtr_3 | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| H.3 | 3 | Quarter | frm2551Qv2018:qtr_4 | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| H.4 | 4 | Amended return? | frm2551Qv2018:amendedRtn; members: amendedRtn_1, amendedRtn_2 | is_amended | /is_amended | input | user_input / never_fill | CONFIRMED | none | frm2551Qv2018:amendedRtn |
| H.5 | 5 | Number of sheets attached | frm2551Qv2018:txtSheets | number_of_attached_sheets | /number_of_attached_sheets | input | user_input / never_fill | CONFIRMED | none | frm2551Qv2018:txtSheets |
| I.6 | 6 | Taxpayer identification number | frm2551Qv2018:txtTIN1 | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| I.6 | 6 | Taxpayer identification number | frm2551Qv2018:txtTIN2 | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| I.6 | 6 | Taxpayer identification number | frm2551Qv2018:txtTIN3 | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| I.6 | 6 | Taxpayer identification number | frm2551Qv2018:txtBranchCode | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| I.8 | 8 | Taxpayer name | frm2551Qv2018:registeredName | - | /taxpayer_name | locked_profile lock=profile | profile / never_fill | CONFIRMED | taxpayer | - |
| I.9 | 9 | Registered address | frm2551Qv2018:registeredAddress | - | /registered_address | locked_profile lock=profile | profile / never_fill | CONFIRMED | none | - |
| I.9A | 9 | ZIP code | frm2551Qv2018:zipCode | - | /zip_code | locked_profile lock=profile | profile / never_fill | CONFIRMED | none | - |
| I.10 | 10 | Contact number | frm2551Qv2018:telNo | - | /contact_number | locked_profile lock=profile | profile / never_fill | CONFIRMED | none | - |
| I.11 | 11 | Email address | txtEmail#occurrence-1 | - | /email | locked_profile lock=profile | profile / never_fill | CONFIRMED | none | - |
| I.12 | 12 | Relief under special law or treaty? | frm2551Qv2018:taxTreaty; members: taxTreaty_1, taxTreaty_2 | tax_relief | /tax_relief | input | user_input / never_fill | CONFIRMED | none | frm2551Qv2018:taxTreaty |
| I.12A | 12 | Specify relief | frm2551Qv2018:txtTaxReliefSpecify | tax_relief_specification | /tax_relief_specification | input | user_input / never_fill | CONFIRMED | none | frm2551Qv2018:txtTaxReliefSpecify |
| I.13 | 13 | Income-tax rates availed | frm2551Qv2018:taxRate; members: taxRate1, taxRate2, taxRateNA | - | /item_13_election | locked_election lock=election | profile / never_fill | CONFIRMED | none | - |
| II.14 | 14 | Total tax due | frm2551Qv2018:txt14 | - | - | computed | none / cross_check | CONFIRMED | none | - |
| II.15 | 15 | Creditable percentage tax withheld per 2307 | frm2551Qv2018:txt15 | creditable_tax_withheld (also: form-2551q-creditable) | /creditable_tax_withheld | input | evidence / fill_mvp | CONFIRMED | none | frm2551Qv2018:txt15 |
| II.16 | 16 | Tax paid in previously filed return | frm2551Qv2018:txt16 | tax_paid_previous | /tax_paid_previous | input | prior_filing / never_fill | CONFIRMED | none | frm2551Qv2018:txt16 |
| II.17 | 17 | Other tax credit/payment + description | frm2551Qv2018:txt17Specify | other_tax_credit_description | /other_tax_credit_description | input | user_input / never_fill | CONFIRMED | none | frm2551Qv2018:txt17Specify |
| II.17 | 17 | Other tax credit/payment + description | frm2551Qv2018:txt17 | other_tax_credit (also: form-2551q-other-credit) | /other_tax_credit | input | user_input / never_fill | CONFIRMED | none | frm2551Qv2018:txt17 |
| II.18 | 18 | Total tax credits/payments | frm2551Qv2018:txt18 | - | - | computed | none / cross_check | CONFIRMED | none | - |
| II.19 | 19 | Tax still payable / overpayment | frm2551Qv2018:txt19 | - | - | computed | none / cross_check | CONFIRMED | none | - |
| II.20 | 20 | Surcharge | frm2551Qv2018:txt20 | - | /penalty_overrides/surcharge | input | user_input / never_fill | CONFIRMED | none | frm2551Qv2018:txt20 |
| II.21 | 21 | Interest | frm2551Qv2018:txt21 | - | /penalty_overrides/interest | input | user_input / never_fill | CONFIRMED | none | frm2551Qv2018:txt21 |
| II.22 | 22 | Compromise | frm2551Qv2018:txt22 | compromise [UNCONFIRMED] | /penalty_overrides/compromise | input | user_input / never_fill | CONFIRMED | none | frm2551Qv2018:txt22 |
| II.23 | 23 | Total penalties | frm2551Qv2018:txt23 | - | - | computed | none / cross_check | CONFIRMED | none | - |
| II.24 | 24 | Total amount payable / overpayment | frm2551Qv2018:txt24 | - | - | computed | none / cross_check | CONFIRMED | none | - |
| II.overpayment | - | Refund or tax-credit-certificate selection | frm2551Qv2018:overPayment; members: overPayment1, overPayment2 | overpayment_disposition | /overpayment_disposition | input | user_input / never_fill | CONFIRMED | none | frm2551Qv2018:overPayment |
| DECL.agent | - | Tax agent accreditation / attorney roll, issue and expiry | txtTaxAgentNo | - | - | not_fillable | user_input / never_fill | CONFIRMED | none | - |
| DECL.agent | - | Tax agent accreditation / attorney roll, issue and expiry | txtDateIssue | - | - | not_fillable | user_input / never_fill | CONFIRMED | none | - |
| DECL.agent | - | Tax agent accreditation / attorney roll, issue and expiry | txtDateExpiry | - | - | not_fillable | user_input / never_fill | CONFIRMED | none | - |
| III.25.bank | - | Cash / bank debit: Drawee bank / agency | frm2551Qv2018:txtAgency25 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.25.number | - | Cash / bank debit: Reference number | frm2551Qv2018:txtNumber25 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.25.date | - | Cash / bank debit: Payment date | frm2551Qv2018:txtDate25 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.25.amount | - | Cash / bank debit: Amount | frm2551Qv2018:txtAmount25 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.26.bank | - | Check: Drawee bank / agency | frm2551Qv2018:txtAgency26 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.26.number | - | Check: Reference number | frm2551Qv2018:txtNumber26 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.26.date | - | Check: Payment date | frm2551Qv2018:txtDate26 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.26.amount | - | Check: Amount | frm2551Qv2018:txtAmount26 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.27.bank | - | Tax debit memo: Drawee bank / agency | frm2551Qv2018:txtAgency27 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.27.number | - | Tax debit memo: Reference number | frm2551Qv2018:txtNumber27 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.27.date | - | Tax debit memo: Payment date | frm2551Qv2018:txtDate27 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.27.amount | - | Tax debit memo: Amount | frm2551Qv2018:txtAmount27 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.28.particular | - | Other payment: particulars (Particular 28; key txtParticular28) (added by correction 29) | frm2551Qv2018:txtParticular28 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.28.bank | - | Other payment: Drawee bank / agency | frm2551Qv2018:txtAgency28 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.28.number | - | Other payment: Reference number | frm2551Qv2018:txtNumber28 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.28.date | - | Other payment: Payment date | frm2551Qv2018:txtDate28 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.28.amount | - | Other payment: Amount | frm2551Qv2018:txtAmount28 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| page2-mirror | - | (page-2 mirror; derived by BIR; no workbook row) | frm2551Qv2018:txtPg2TIN1 | - | - | computed | none / never_touch | CONFIRMED | none | - |
| page2-mirror | - | (page-2 mirror; derived by BIR; no workbook row) | frm2551Qv2018:txtPg2TIN2 | - | - | computed | none / never_touch | CONFIRMED | none | - |
| page2-mirror | - | (page-2 mirror; derived by BIR; no workbook row) | frm2551Qv2018:txtPg2TIN3 | - | - | computed | none / never_touch | CONFIRMED | none | - |
| page2-mirror | - | (page-2 mirror; derived by BIR; no workbook row) | frm2551Qv2018:txtPg2BranchCode | - | - | computed | none / never_touch | CONFIRMED | none | - |
| page2-mirror | - | (page-2 mirror; derived by BIR; no workbook row) | frm2551Qv2018:txtPg2TaxpayerName | - | - | computed | none / never_touch | CONFIRMED | none | - |
| S1.1.ATC | - | Row 1: ATC | drpATC1 | schedule_1.0.atc | - | input | ledger / fill_mvp | CONFIRMED | none | schedule_1.0.atc |
| S1.1.base | - | Row 1: taxable amount | txtATCAmt1 | schedule_1.0.taxable_amount (also: taxable_amount, form-2551q-taxable-0) | /schedule_1/0/taxable_amount | input | ledger / fill_mvp | CONFIRMED | row:1 | txtATCAmt1 |
| S1.1.rate | - | Row 1: tax rate | txtATCRate1 | - | - | computed | none / cross_check | CONFIRMED | none | - |
| S1.1.tax | - | Row 1: tax due | txtATCDue1 | - | - | computed | none / cross_check | CONFIRMED | none | - |
| S1.2.ATC | - | Row 2: ATC | drpATC2 | schedule_1.1.atc | - | input | ledger / never_fill | CONFIRMED | none | schedule_1.1.atc |
| S1.2.base | - | Row 2: taxable amount | txtATCAmt2 | schedule_1.1.taxable_amount | /schedule_1/1/taxable_amount | input | ledger / never_fill | CONFIRMED | row:2 | txtATCAmt2 |
| S1.2.rate | - | Row 2: tax rate | txtATCRate2 | - | - | computed | none / cross_check | CONFIRMED | none | - |
| S1.2.tax | - | Row 2: tax due | txtATCDue2 | - | - | computed | none / cross_check | CONFIRMED | none | - |
| S1.3.ATC | - | Row 3: ATC | drpATC3 | schedule_1.2.atc | - | input | ledger / never_fill | CONFIRMED | none | schedule_1.2.atc |
| S1.3.base | - | Row 3: taxable amount | txtATCAmt3 | schedule_1.2.taxable_amount | /schedule_1/2/taxable_amount | input | ledger / never_fill | CONFIRMED | row:3 | txtATCAmt3 |
| S1.3.rate | - | Row 3: tax rate | txtATCRate3 | - | - | computed | none / cross_check | CONFIRMED | none | - |
| S1.3.tax | - | Row 3: tax due | txtATCDue3 | - | - | computed | none / cross_check | CONFIRMED | none | - |
| S1.4.ATC | - | Row 4: ATC | drpATC4 | schedule_1.3.atc | - | input | ledger / never_fill | CONFIRMED | none | schedule_1.3.atc |
| S1.4.base | - | Row 4: taxable amount | txtATCAmt4 | schedule_1.3.taxable_amount | /schedule_1/3/taxable_amount | input | ledger / never_fill | CONFIRMED | row:4 | txtATCAmt4 |
| S1.4.rate | - | Row 4: tax rate | txtATCRate4 | - | - | computed | none / cross_check | CONFIRMED | none | - |
| S1.4.tax | - | Row 4: tax due | txtATCDue4 | - | - | computed | none / cross_check | CONFIRMED | none | - |
| S1.5.ATC | - | Row 5: ATC | drpATC5 | schedule_1.4.atc | - | input | ledger / never_fill | CONFIRMED | none | schedule_1.4.atc |
| S1.5.base | - | Row 5: taxable amount | txtATCAmt5 | schedule_1.4.taxable_amount | /schedule_1/4/taxable_amount | input | ledger / never_fill | CONFIRMED | row:5 | txtATCAmt5 |
| S1.5.rate | - | Row 5: tax rate | txtATCRate5 | - | - | computed | none / cross_check | CONFIRMED | none | - |
| S1.5.tax | - | Row 5: tax due | txtATCDue5 | - | - | computed | none / cross_check | CONFIRMED | none | - |
| S1.6.ATC | - | Row 6: ATC | drpATC6 | schedule_1.5.atc | - | input | ledger / never_fill | CONFIRMED | none | schedule_1.5.atc |
| S1.6.base | - | Row 6: taxable amount | txtATCAmt6 | schedule_1.5.taxable_amount | /schedule_1/5/taxable_amount | input | ledger / never_fill | CONFIRMED | row:6 | txtATCAmt6 |
| S1.6.rate | - | Row 6: tax rate | txtATCRate6 | - | - | computed | none / cross_check | CONFIRMED | none | - |
| S1.6.tax | - | Row 6: tax due | txtATCDue6 | - | - | computed | none / cross_check | CONFIRMED | none | - |
| S1.7 | - | Total tax due | txtTotalSched1 | - | - | computed | none / cross_check | CONFIRMED | none | - |
| app | - | (app-written by BIR; no workbook row) | frm2551Qv2018:txtCurrentPage | - | - | not_fillable | none / never_touch | CONFIRMED | none | - |
| app | - | (app-written by BIR; no workbook row) | frm2551Qv2018:txtMaxPage | - | - | not_fillable | none / never_touch | CONFIRMED | none | - |
| app | - | (app-written by BIR; no workbook row) | txtFinalFlag | - | - | not_fillable | none / never_touch | CONFIRMED | none | - |
| app | - | (app-written by BIR; no workbook row) | txtEnroll | - | - | not_fillable | none / never_touch | CONFIRMED | none | - |
| app-credential | - | (BIR online credential field; Books never records a value) | ebirOnlineConfirmUsername | - | - | not_fillable | none / never_touch | CONFIRMED | none | - |
| app-credential | - | (BIR online credential field; Books never records a value) | ebirOnlineUsername | - | - | not_fillable | none / never_touch | CONFIRMED | none | - |
| app-credential | - | (BIR online credential field; Books never records a value) | ebirOnlineSecret | - | - | not_fillable | none / never_touch | CONFIRMED | none | - |
| I.11 | 11 | Email address | txtEmail#occurrence-2 | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| app | - | (app-written by BIR; no workbook row) | driveSelectTPExport | - | - | not_fillable | none / never_touch | CONFIRMED | none | - |
| I.7 | 7 | RDO code | frm2551Qv2018:txtRDOCode | - | /rdo_code | locked_profile lock=profile | profile / never_fill | CONFIRMED | none | - |

Notes for 2551Q:
- Items 20-22 are bound to `/penalty_overrides/*` in the pin. Books never fills them (D14).
- `frm2551Qv2018:taxRate` (item 13, with its `_1`, `_2` and `NA` variants) is election-locked. The pin also has `taxRateNA`. The printed NA option is UNCONFIRMED (bir.md s2 lists two options).
- Typed `compromise` writes the root field, which the penalty recompute overwrites in auto mode (verify.md s2 #9). The official `txt22` is the durable input.
- The Schedule 1 ATC key `drpATC1` is refused. Books uses the typed alias `schedule_1.0.atc`.
- `original_return_filed_and_paid_on_time` is a typed-only input with no official box (assist.rs:182). Its effect is not traced; Books does not use it (UNCONFIRMED).

## 4. Verified crosswalk: Form 1601-C (monthly compensation withholding)

105 schema fields. Source: `data/crosswalk/1601c-v2018.json`. Payroll fields are P6 (not in MVP).

| Sheet ID (workbook) | Printed item (schema) | Spreadsheet label (workbook text) | ACTUAL API KEY (pinned schema) | Typed alias (code, verify s3-s4) | Draft path (pinned) | Fill class | Books source / policy | Status | Scope | Fill key Books may send |
|---|---|---|---|---|---|---|---|---|---|---|
| H.1 | 1 | For the month (MM/YYYY) | frm1601c:txtMonth | - | /month | locked_identity lock=identity | none / never_fill | CONFIRMED | none | - |
| H.1 | 1 | For the month (MM/YYYY) | frm1601c:txtYear | - | /taxable_year | locked_identity lock=identity | none / never_fill | CONFIRMED | none | - |
| H.2 | 2 | Amended return? | frm1601c:AmendedRtn; members: AmendedRtn_1, AmendedRtn_2 | is_amended | /is_amended | input | user_input / never_fill | CONFIRMED | none | frm1601c:AmendedRtn |
| H.3 | 3 | Any taxes withheld? | frm1601c:TaxWithheld; members: TaxWithheld_1, TaxWithheld_2 | any_taxes_withheld (also: form-1601c-withheld) | /any_taxes_withheld | input | payroll / fill_p6 | CONFIRMED | none | frm1601c:TaxWithheld |
| H.4 | 4 | Number of sheets attached | frm1601c:txtSheets | number_of_sheets | /number_of_sheets | input | user_input / never_fill | CONFIRMED | none | frm1601c:txtSheets |
| H.5 | 5 | ATC | frm1601c:txtATC | atc | /atc | input | none / fill_p6 | CONFIRMED | none | frm1601c:txtATC |
| I.6 | 6 | Taxpayer identification number | frm1601c:txtTIN1 | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| I.6 | 6 | Taxpayer identification number | frm1601c:txtTIN2 | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| I.6 | 6 | Taxpayer identification number | frm1601c:txtTIN3 | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| I.6 | 6 | Taxpayer identification number | frm1601c:txtBranchCode | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| I.7 | 7 | RDO code | frm1601c:txtRDOCode | - | /rdo_code | locked_profile lock=profile | profile / never_fill | CONFIRMED | none | - |
| I.8 | 8 | Withholding agent name | frm1601c:txtTaxpayerName | - | /taxpayer_name | locked_profile lock=profile | profile / never_fill | CONFIRMED | taxpayer | - |
| I.9 | 9 | Registered address | frm1601c:txtAddress | - | /registered_address | locked_profile lock=profile | profile / never_fill | CONFIRMED | none | - |
| I.9 | 9 | Registered address | frm1601c:txtAddress2 | registered_address_2 | /registered_address_2 | input | user_input / never_fill | CONFIRMED | none | frm1601c:txtAddress2 |
| I.9A | 9A | ZIP code | frm1601c:txtZipCode | - | /zip_code | locked_profile lock=profile | profile / never_fill | CONFIRMED | none | - |
| I.10 | 10 | Contact number | frm1601c:txtTelNum | - | /contact_number | locked_profile lock=profile | profile / never_fill | CONFIRMED | none | - |
| I.11 | 11 | Category of withholding agent | frm1601c:CatAgent; members: CatAgent_P, CatAgent_G | category_of_agent (also: form-1601c-category) | /category_of_agent | input | user_input / never_fill | CONFIRMED | none | frm1601c:CatAgent |
| I.12 | 12 | Email address | txtEmail | - | /email_address | locked_profile lock=profile | profile / never_fill | CONFIRMED | none | - |
| I.13 | 13 | Payees with relief under law / treaty? | frm1601c:SpecialTax; members: SpecialTax_1, SpecialTax_2 | tax_relief | /tax_relief | input | user_input / never_fill | CONFIRMED | none | frm1601c:SpecialTax |
| I.13A | 13 | Specify relief | frm1601c:selTreaty | tax_relief_specification | /tax_relief_specification | input | user_input / never_fill | CONFIRMED | none | frm1601c:selTreaty |
| II.14 | 14 | Total amount of compensation | frm1601c:txtTax14 | tax_14_total_compensation (also: tax_14, form-1601c-tax-14) | /tax_14_total_compensation | input | payroll / fill_p6 | CONFIRMED | none | frm1601c:txtTax14 |
| II.15 | 15 | Statutory minimum wage for MWEs | frm1601c:txtTax15 | tax_15_statutory_minimum_wage | /tax_15_statutory_minimum_wage | input | payroll / fill_p6 | CONFIRMED | none | frm1601c:txtTax15 |
| II.16 | 16 | MWE holiday / overtime / night / hazard pay | frm1601c:txtTax16 | tax_16_holiday_pay | /tax_16_holiday_pay | input | payroll / fill_p6 | CONFIRMED | none | frm1601c:txtTax16 |
| II.17 | 17 | 13th month pay and other benefits | frm1601c:txtTax17 | tax_17_13th_month_pay | /tax_17_13th_month_pay | input | payroll / fill_p6 | CONFIRMED | none | frm1601c:txtTax17 |
| II.18 | 18 | De minimis benefits | frm1601c:txtTax18 | tax_18_de_minimis | /tax_18_de_minimis | input | payroll / fill_p6 | CONFIRMED | none | frm1601c:txtTax18 |
| II.19 | 19 | Mandatory contributions and union dues, employee share only | frm1601c:txtTax19 | tax_19_sss_gsis | /tax_19_sss_gsis | input | payroll / fill_p6 | CONFIRMED | none | frm1601c:txtTax19 |
| II.20 | 20 | Other non-taxable compensation + description | frm1601c:txt20Other | tax_20_other_name | /tax_20_other_name | input | payroll / fill_p6 | CONFIRMED | none | frm1601c:txt20Other |
| II.20 | 20 | Other non-taxable compensation + description | frm1601c:txtTax20 | tax_20_other_amount | /tax_20_other_amount | input | payroll / fill_p6 | CONFIRMED | none | frm1601c:txtTax20 |
| II.21 | 21 | Total non-taxable compensation | frm1601c:txtTax21 | - | - | computed | none / cross_check | CONFIRMED | none | - |
| II.22 | 22 | Total taxable compensation | frm1601c:txtTax22 | - | - | computed | none / cross_check | CONFIRMED | none | - |
| II.23 | 23 | Taxable compensation not subject to withholding | frm1601c:txtTax23 | tax_23_not_subject | /tax_23_not_subject | input | payroll / fill_p6 | CONFIRMED | none | frm1601c:txtTax23 |
| II.24 | 24 | Net taxable compensation | frm1601c:txtTax24 | - | - | computed | none / cross_check | CONFIRMED | none | - |
| II.25 | 25 | Total taxes withheld | frm1601c:txtTax25 | tax_25_total_taxes_withheld | /tax_25_total_taxes_withheld | input | payroll / fill_p6 | CONFIRMED | none | frm1601c:txtTax25 |
| II.26 | 26 | Adjustment from previous months | frm1601c:txtTax26 | - | - | computed | none / cross_check | CONFIRMED | none | - |
| II.27 | 27 | Taxes withheld for remittance | frm1601c:txtTax27 | - | - | computed | none / cross_check | CONFIRMED | none | - |
| II.28 | 28 | Previously remitted in amended return | frm1601c:txtTax28 | tax_28_tax_remitted_previously | /tax_28_tax_remitted_previously | input | prior_filing / never_fill | CONFIRMED | none | frm1601c:txtTax28 |
| II.29 | 29 | Other remittances + description | frm1601c:txt29Other | tax_29_other_remittances_name | /tax_29_other_remittances_name | input | evidence / never_fill | CONFIRMED | none | frm1601c:txt29Other |
| II.29 | 29 | Other remittances + description | frm1601c:txtTax29 | tax_29_other_remittances_amount | /tax_29_other_remittances_amount | input | evidence / never_fill | CONFIRMED | none | frm1601c:txtTax29 |
| II.30 | 30 | Total remittances made | frm1601c:txtTax30 | - | - | computed | none / cross_check | CONFIRMED | none | - |
| II.31 | 31 | Tax still due / overremittance | frm1601c:txtTax31 | - | - | computed | none / cross_check | CONFIRMED | none | - |
| II.32 | 32 | Surcharge | frm1601c:txtTax32 | tax_32_surcharge [UNCONFIRMED] | /penalty_overrides/surcharge | input | user_input / never_fill | CONFIRMED | none | frm1601c:txtTax32 |
| II.33 | 33 | Interest | frm1601c:txtTax33 | tax_33_interest [UNCONFIRMED] | /penalty_overrides/interest | input | user_input / never_fill | CONFIRMED | none | frm1601c:txtTax33 |
| II.34 | 34 | Compromise | frm1601c:txtTax34 | tax_34_compromise [UNCONFIRMED] | /penalty_overrides/compromise | input | user_input / never_fill | CONFIRMED | none | frm1601c:txtTax34 |
| II.35 | 35 | Total penalties | frm1601c:txtTax35 | - | - | computed | none / cross_check | CONFIRMED | none | - |
| II.36 | 36 | Total amount still due / overremittance | frm1601c:txtTax36 | - | - | computed | none / cross_check | CONFIRMED | none | - |
| DECL.signature | - | Signer, title, TIN, agent credentials and dates | txtTaxAgentNo | - | - | not_fillable | user_input / never_fill | CONFIRMED | none | - |
| DECL.signature | - | Signer, title, TIN, agent credentials and dates | txtDateIssue | - | - | not_fillable | user_input / never_fill | CONFIRMED | none | - |
| DECL.signature | - | Signer, title, TIN, agent credentials and dates | txtDateExpiry | - | - | not_fillable | user_input / never_fill | CONFIRMED | none | - |
| III.37.bank | 37 | Cash / bank debit: Drawee bank / agency | frm1601c:txtAgency37 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.37.number | 37 | Cash / bank debit: Reference number | frm1601c:txtNumber37 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.37.date | 37 | Cash / bank debit: Payment date | frm1601c:txtDate37 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.37.amount | 37 | Cash / bank debit: Amount | frm1601c:txtAmount37 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.38.bank | 38 | Check: Drawee bank / agency | frm1601c:txtAgency38 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.38.number | 38 | Check: Reference number | frm1601c:txtNumber38 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.38.date | 38 | Check: Payment date | frm1601c:txtDate38 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.38.amount | 38 | Check: Amount | frm1601c:txtAmount38 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.39.number | 39 | Tax debit memo: Reference number | frm1601c:txtNumber39 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.39.date | 39 | Tax debit memo: Payment date | frm1601c:txtDate39 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.39.amount | 39 | Tax debit memo: Amount | frm1601c:txtAmount39 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.40.particular | 40 | Other payment: particulars (txtParticular40) (added by correction 31) | frm1601c:txtParticular40 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.40.bank | 40 | Other payment: Drawee bank / agency | frm1601c:txtAgency40 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.40.number | 40 | Other payment: Reference number | frm1601c:txtNumber40 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.40.date | 40 | Other payment: Payment date | frm1601c:txtDate40 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.40.amount | 40 | Other payment: Amount | frm1601c:txtAmount40 | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| page2-mirror | - | (page-2 mirror; derived by BIR; no workbook row) | frm1601c:txtPg2TIN1 | - | - | not_fillable | none / never_touch | CONFIRMED | none | - |
| page2-mirror | - | (page-2 mirror; derived by BIR; no workbook row) | frm1601c:txtPg2TIN2 | - | - | not_fillable | none / never_touch | CONFIRMED | none | - |
| page2-mirror | - | (page-2 mirror; derived by BIR; no workbook row) | frm1601c:txtPg2TIN3 | - | - | not_fillable | none / never_touch | CONFIRMED | none | - |
| page2-mirror | - | (page-2 mirror; derived by BIR; no workbook row) | frm1601c:txtPg2BranchCode | - | - | not_fillable | none / never_touch | CONFIRMED | none | - |
| page2-mirror | - | (page-2 mirror; derived by BIR; no workbook row) | frm1601c:txtPg2TaxpayerName | - | /taxpayer_name | locked_profile lock=profile | none / never_touch | CONFIRMED | taxpayer | - |
| app | - | (app-written by BIR; no workbook row) | chkScheduleDelete0 | - | - | not_fillable | none / never_touch | CONFIRMED | none | - |
| S1.1.1 | Schedule I | Adjustment row 1: Previous month | frm1601c:sched1:txtMonthYear0 | schedule_1.0.previous_month | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | schedule_1.0.previous_month |
| S1.1.2 | Schedule I | Adjustment row 1: Date paid | frm1601c:sched1:txtDatePaid0 | schedule_1.0.date_paid | /schedule_1/0/date_paid | input | prior_filing / never_fill | CONFIRMED (sheet ID by label) | row:1 | frm1601c:sched1:txtDatePaid0 |
| S1.1.3 | Schedule I | Adjustment row 1: Drawee bank / code / agency | frm1601c:sched1:txtBankCode0 | schedule_1.0.drawee_bank_code_or_agency | /schedule_1/0/drawee_bank_code_or_agency | input | prior_filing / never_fill | CONFIRMED (sheet ID by label) | row:1 | frm1601c:sched1:txtBankCode0 |
| S1.1.4 | Schedule I | Adjustment row 1: Number | frm1601c:sched1:txtNumber0 | schedule_1.0.payment_number | /schedule_1/0/payment_number | input | prior_filing / never_fill | CONFIRMED (sheet ID by label) | row:1 | frm1601c:sched1:txtNumber0 |
| S1.1.5 | Schedule I | Adjustment row 1: Tax paid excluding penalties | frm1601c:sched1:txtTaxPaid0 | schedule_1.0.tax_paid | /schedule_1/0/tax_paid | input | prior_filing / never_fill | CONFIRMED (sheet ID by label) | row:1 | frm1601c:sched1:txtTaxPaid0 |
| app | - | (app-written by BIR; no workbook row) | chkScheduleDelete1 | - | - | not_fillable | none / never_touch | CONFIRMED | none | - |
| S1.2.1 | Schedule I | Adjustment row 2: Previous month | frm1601c:sched1:txtMonthYear1 | schedule_1.1.previous_month | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | schedule_1.1.previous_month |
| S1.2.2 | Schedule I | Adjustment row 2: Date paid | frm1601c:sched1:txtDatePaid1 | schedule_1.1.date_paid | /schedule_1/1/date_paid | input | prior_filing / never_fill | CONFIRMED (sheet ID by label) | row:2 | frm1601c:sched1:txtDatePaid1 |
| S1.2.3 | Schedule I | Adjustment row 2: Drawee bank / code / agency | frm1601c:sched1:txtBankCode1 | schedule_1.1.drawee_bank_code_or_agency | /schedule_1/1/drawee_bank_code_or_agency | input | prior_filing / never_fill | CONFIRMED (sheet ID by label) | row:2 | frm1601c:sched1:txtBankCode1 |
| S1.2.4 | Schedule I | Adjustment row 2: Number | frm1601c:sched1:txtNumber1 | schedule_1.1.payment_number | /schedule_1/1/payment_number | input | prior_filing / never_fill | CONFIRMED (sheet ID by label) | row:2 | frm1601c:sched1:txtNumber1 |
| S1.2.5 | Schedule I | Adjustment row 2: Tax paid excluding penalties | frm1601c:sched1:txtTaxPaid1 | schedule_1.1.tax_paid | /schedule_1/1/tax_paid | input | prior_filing / never_fill | CONFIRMED (sheet ID by label) | row:2 | frm1601c:sched1:txtTaxPaid1 |
| app | - | (app-written by BIR; no workbook row) | chkScheduleDelete2 | - | - | not_fillable | none / never_touch | CONFIRMED | none | - |
| S1.3.1 | Schedule I | Adjustment row 3: Previous month | frm1601c:sched1:txtMonthYear2 | schedule_1.2.previous_month | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | schedule_1.2.previous_month |
| S1.3.2 | Schedule I | Adjustment row 3: Date paid | frm1601c:sched1:txtDatePaid2 | schedule_1.2.date_paid | /schedule_1/2/date_paid | input | prior_filing / never_fill | CONFIRMED (sheet ID by label) | row:3 | frm1601c:sched1:txtDatePaid2 |
| S1.3.3 | Schedule I | Adjustment row 3: Drawee bank / code / agency | frm1601c:sched1:txtBankCode2 | schedule_1.2.drawee_bank_code_or_agency | /schedule_1/2/drawee_bank_code_or_agency | input | prior_filing / never_fill | CONFIRMED (sheet ID by label) | row:3 | frm1601c:sched1:txtBankCode2 |
| S1.3.4 | Schedule I | Adjustment row 3: Number | frm1601c:sched1:txtNumber2 | schedule_1.2.payment_number | /schedule_1/2/payment_number | input | prior_filing / never_fill | CONFIRMED (sheet ID by label) | row:3 | frm1601c:sched1:txtNumber2 |
| S1.3.5 | Schedule I | Adjustment row 3: Tax paid excluding penalties | frm1601c:sched1:txtTaxPaid2 | schedule_1.2.tax_paid | /schedule_1/2/tax_paid | input | prior_filing / never_fill | CONFIRMED (sheet ID by label) | row:3 | frm1601c:sched1:txtTaxPaid2 |
| S1.1.6 | Schedule I | Adjustment row 1: Should be tax due | frm1601c:sched1:txtShouldTaxDue0 | schedule_1.0.should_be_tax_due | /schedule_1/0/should_be_tax_due | input | prior_filing / never_fill | CONFIRMED (sheet ID by label) | row:1 | frm1601c:sched1:txtShouldTaxDue0 |
| S1.1.7 | Schedule I | Adjustment row 1: Adjustment = column 6 less 5 | frm1601c:sched1:txtAdjustments0 | - | - | computed | none / cross_check | CONFIRMED (sheet ID by label) | none | - |
| S1.2.6 | Schedule I | Adjustment row 2: Should be tax due | frm1601c:sched1:txtShouldTaxDue1 | schedule_1.1.should_be_tax_due | /schedule_1/1/should_be_tax_due | input | prior_filing / never_fill | CONFIRMED (sheet ID by label) | row:2 | frm1601c:sched1:txtShouldTaxDue1 |
| S1.2.7 | Schedule I | Adjustment row 2: Adjustment = column 6 less 5 | frm1601c:sched1:txtAdjustments1 | - | - | computed | none / cross_check | CONFIRMED (sheet ID by label) | none | - |
| S1.3.6 | Schedule I | Adjustment row 3: Should be tax due | frm1601c:sched1:txtShouldTaxDue2 | schedule_1.2.should_be_tax_due | /schedule_1/2/should_be_tax_due | input | prior_filing / never_fill | CONFIRMED (sheet ID by label) | row:3 | frm1601c:sched1:txtShouldTaxDue2 |
| S1.3.7 | Schedule I | Adjustment row 3: Adjustment = column 6 less 5 | frm1601c:sched1:txtAdjustments2 | - | - | computed | none / cross_check | CONFIRMED (sheet ID by label) | none | - |
| S1.4 | Schedule I | Total adjustment | frm1601c:sched1:txtTotal1 | - | - | computed | none / cross_check | CONFIRMED (sheet ID by label) | none | - |
| app | - | (app-written by BIR; no workbook row) | frm1601c:txtCurrentPage | - | - | not_fillable | none / never_touch | CONFIRMED | none | - |
| app | - | (app-written by BIR; no workbook row) | frm1601c:txtMaxPage | - | - | not_fillable | none / never_touch | CONFIRMED | none | - |
| I.8 | 8 | Withholding agent name | frm1601c:txtLineBus | - | /line_of_business | locked_profile lock=profile | profile / never_fill | CONFIRMED | none | - |
| app | Schedule I | (app-written by BIR; no workbook row) | chkScheduleDelete{N>=0} | - | - | not_fillable | none / never_touch | CONFIRMED | none | - |
| S1.N | Schedule I | (schedule I row family pattern; no fixed workbook row) | frm1601c:sched1:txtMonthYear{N>=0} | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |
| S1.N | Schedule I | (schedule I row family pattern; no fixed workbook row) | frm1601c:sched1:txtDatePaid{N>=0} | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |
| S1.N | Schedule I | (schedule I row family pattern; no fixed workbook row) | frm1601c:sched1:txtBankCode{N>=0} | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |
| S1.N | Schedule I | (schedule I row family pattern; no fixed workbook row) | frm1601c:sched1:txtNumber{N>=0} | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |
| S1.N | Schedule I | (schedule I row family pattern; no fixed workbook row) | frm1601c:sched1:txtTaxPaid{N>=0} | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |
| S1.N | Schedule I | (schedule I row family pattern; no fixed workbook row) | frm1601c:sched1:txtShouldTaxDue{N>=0} | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |
| S1.N | Schedule I | (schedule I row family pattern; no fixed workbook row) | frm1601c:sched1:txtAdjustments{N>=0} | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |

Notes for 1601-C:
- No drawee key exists for item 39 (verify.md s2 #13). Items 37, 38 and 40 are unmapped in the pin.
- Schedule I official keys cover rows 0-2 only. Typed rows N>=3 were not traced (verify.md s2 #17, UNCONFIRMED).
- Item 19 binds to `/tax_19_sss_gsis`, narrower than its label. It is the employee share only (RR 11-2018 s2.78.1(B)(12); statute UNVERIFIED).
- Typed write targets for items 32-34 are UNCONFIRMED (verify.md s4).

## 5. Verified crosswalk: Form 1701-MS (annual return, micro and small)

169 taxpayer-side schema fields. Spouse (B-side) fields are summarised in section 6 and are refused in MVP. Source: `data/crosswalk/1701ms-v2024.json` (all 206 entries).

| Sheet ID (workbook) | Printed item (schema) | Spreadsheet label (workbook text) | ACTUAL API KEY (pinned schema) | Typed alias (code, verify s3-s4) | Draft path (pinned) | Fill class | Books source / policy | Status | Scope | Fill key Books may send |
|---|---|---|---|---|---|---|---|---|---|---|
| H.1 | 1 | For the year (MM/YYYY) | frm1701MS:txtMonthNo1 | - | /month | input | user_input / never_fill | CONFIRMED | none | frm1701MS:txtMonthNo1 |
| H.1 | 1 | For the year (MM/YYYY) | frm1701MS:txtYearNo1 | - | /taxable_year | locked_identity lock=identity | none / never_fill | CONFIRMED | none | - |
| H.2 | 2 | Amended return? | frm1701MS:txtAmended; members: txtAmendedYesNo2, txtAmendedNoNo2 | - | /is_amended | input | user_input / never_fill | CONFIRMED | none | frm1701MS:txtAmended |
| H.3 | 3 | Short period return? | frm1701MS:txtReturnPeriod; members: txtReturnPeriodYesNo3, txtReturnPeriodNoNo3 | - | /is_short_period | input | user_input / never_fill | CONFIRMED | none | frm1701MS:txtReturnPeriod |
| H.4 | - | Civil status | frm1701MS:civilStatus | - | /civil_status | input | user_input / never_fill | CONFIRMED | none | frm1701MS:civilStatus |
| H.5 | 5 | If married, filing jointly / separately / spouse no income | frm1701MS:txtIfMarried; members: txtIfMarriedJointlyNo5, txtIfMarriedSeparatelyNo5, txtIfMarriedNotApplicableNo5 | - | /filing | input | user_input / never_fill | CONFIRMED | none | frm1701MS:txtIfMarried |
| I.6 | - | TIN | frm1701MS:txtPg1TIN1 | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| I.6 | - | TIN | frm1701MS:txtPg1TIN2 | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| I.6 | - | TIN | frm1701MS:txtPg1TIN3 | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| I.6 | - | TIN | frm1701MS:txtPg1BranchCode | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| I.6 | - | TIN | frm1701MS:txtPg2TIN1Spouse | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| I.6 | - | TIN | frm1701MS:txtPg2TIN2Spouse | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| I.6 | - | TIN | frm1701MS:txtPg2TIN3Spouse | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| I.6 | - | TIN | frm1701MS:txtPg2BranchCodeSpouse | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| I.7 | - | RDO code | frm1701MS:txtRDOCodea | - | /rdo_code | locked_profile lock=profile | profile / never_fill | CONFIRMED | none | - |
| I.8 | 8 | Name | frm1701MS:txtTaxpayerNo8a | - | /taxpayer_name | locked_profile lock=profile | profile / never_fill | CONFIRMED | taxpayer | - |
| I.9 | - | Email address | frm1701MS:txtEmaila | - | /email | locked_profile lock=profile | profile / never_fill | CONFIRMED | none | - |
| I.10 | - | Contact number | frm1701MS:txtTelNum10a | - | /contact_number | locked_profile lock=profile | profile / never_fill | CONFIRMED | none | - |
| I.11 | 11 | Source of income | frm1701MS:txtTaxpayerNo11group; members: txtTaxpayerNo11a, txtTaxpayerNo11b | - | /taxpayer/source | input | profile / never_fill | CONFIRMED | taxpayer | frm1701MS:txtTaxpayerNo11group |
| I.11 | - | Source of income | frm1701MS:txtTaxpayerNo11c | - | /taxpayer/source | input | profile / never_fill | CONFIRMED | taxpayer | frm1701MS:txtTaxpayerNo11c |
| I.12 | 12 | Income subject to | frm1701MS:txtTaxpayerNo12b | - | /taxpayer/tax_option | input | profile / never_fill | CONFIRMED RISK B7 | taxpayer | frm1701MS:txtTaxpayerNo12b |
| I.12 | - | Income subject to | frm1701MS:txtTaxpayerNo12; members: txtTaxpayerNo12b1, txtTaxpayerNo12c1, txtTaxpayerNo12d1, txtTaxpayerNo12e1, txtTaxpayerNo12c, txtTaxpayerNo12d, t… | - | /spouse/tax_option, /taxpayer/tax_option | input | profile / never_fill | CONFIRMED RISK B7 | taxpayer/spou… | frm1701MS:txtTaxpayerNo12 |
| I.13 | 13 | Legal basis of relief / exemption | frm1701MS:txtTaxpayerNo13a | - | /taxpayer/legal_basis | input | user_input / never_fill | CONFIRMED | taxpayer | frm1701MS:txtTaxpayerNo13a |
| I.14 | 14 | Investment promotion / government agency | frm1701MS:txtTaxpayerNo14a | - | /taxpayer/promotion_agency | input | user_input / never_fill | CONFIRMED | taxpayer | frm1701MS:txtTaxpayerNo14a |
| I.15 | 15 | Registered activity / program number | frm1701MS:txtTaxpayerNo15a | - | /taxpayer/registered_activity | input | user_input / never_fill | CONFIRMED | taxpayer | frm1701MS:txtTaxpayerNo15a |
| I.16 | 16 | Relief effective from / to | frm1701MS:txtTaxpayerNo16a | - | /taxpayer/effectivity_from | input | user_input / never_fill | CONFIRMED | taxpayer | frm1701MS:txtTaxpayerNo16a |
| I.16 | - | Relief effective from / to | frm1701MS:txtTaxpayerNo16a1 | - | /taxpayer/effectivity_to | input | user_input / never_fill | CONFIRMED | taxpayer | frm1701MS:txtTaxpayerNo16a1 |
| I.17 | 17 | Method of deduction | frm1701MS:txtTaxpayerNo17; members: txtTaxpayerNo17a, txtTaxpayerNo17b | - | /taxpayer/deduction | input | profile / never_fill | CONFIRMED RISK B7 | taxpayer | frm1701MS:txtTaxpayerNo17 |
| I.18 | - | Alphanumeric tax code | taxCodeDropdown | - | /taxpayer/source, /taxpayer/tax_option | input | profile / never_fill | CONFIRMED RISK B7 | taxpayer | taxCodeDropdown |
| II.19 | 19 | Income tax due – special / preferential | frm1701MS:txtIncomeTaxDueNo19a | - | - | computed | none / cross_check | CONFIRMED | none | - |
| II.19 | 19 | Income tax due – special / preferential | frm1701MS:txtIncomeTaxDueNo19b | - | - | computed | none / cross_check | CONFIRMED (sheet ID by label) | none | - |
| II.20 | 20 | Share remitted to other government agency | frm1701MS:txtShareOfOtherNo20a | - | /taxpayer/share_of_other_agencies | input | evidence / never_fill | CONFIRMED | taxpayer | frm1701MS:txtShareOfOtherNo20a |
| II.21 | 21 | Net special / preferential tax | frm1701MS:txtNetSpecialRateNo21a | - | - | computed | none / cross_check | CONFIRMED | none | - |
| II.21 | 21 | Net special / preferential tax | frm1701MS:txtNetSpecialRateNo21b | - | - | computed | none / cross_check | CONFIRMED (sheet ID by label) | none | - |
| II.22 | 22 | Income tax due – regular | frm1701MS:txtIncomeTaxRegularNo22a | - | - | computed | none / cross_check | CONFIRMED | none | - |
| II.22 | 22 | Income tax due – regular | frm1701MS:txtIncomeTaxRegularNo22b | - | - | computed | none / cross_check | CONFIRMED (sheet ID by label) | none | - |
| II.23 | 23 | Total income tax due | frm1701MS:txtTotalIncomeTaxDueNo23a | - | - | computed | none / cross_check | CONFIRMED | none | - |
| II.23 | 23 | Total income tax due | frm1701MS:txtTotalIncomeTaxDueNo23b | - | - | computed | none / cross_check | CONFIRMED (sheet ID by label) | none | - |
| II.24 | 24 | Total tax credits / payments | frm1701MS:txtTotalTaxCreditsPartIINo24a | - | - | computed | none / cross_check | CONFIRMED | none | - |
| II.24 | 24 | Total tax credits / payments | frm1701MS:txtTotalTaxCreditsPartIINo24b | - | - | computed | none / cross_check | CONFIRMED (sheet ID by label) | none | - |
| II.25 | 25 | Tax payable / overpayment | frm1701MS:txtTaxPayableNo25a | - | - | computed | none / cross_check | CONFIRMED | none | - |
| II.25 | 25 | Tax payable / overpayment | frm1701MS:txtTaxPayableNo25b | - | - | computed | none / cross_check | CONFIRMED (sheet ID by label) | none | - |
| II.26 | 26 | Tax due allowed for second installment | frm1701MS:txtTaxDueAllowedNo26a | - | /taxpayer/second_installment | input | user_input / never_fill | CONFIRMED | taxpayer | frm1701MS:txtTaxDueAllowedNo26a |
| II.27 | 27 | Amount payable / overpayment | frm1701MS:txtAmountPayableNo27a | - | - | computed | none / cross_check | CONFIRMED | none | - |
| II.27 | 27 | Amount payable / overpayment | frm1701MS:txtAmountPayableNo27b | - | - | computed | none / cross_check | CONFIRMED (sheet ID by label) | none | - |
| II.28A | 28 | Surcharge | frm1701MS:txtSurchangeNo28a | - | /taxpayer/penalty_overrides/surcharge | input | user_input / never_fill | CONFIRMED | taxpayer | frm1701MS:txtSurchangeNo28a |
| II.28B | 28 | Interest | frm1701MS:txtInterestNo28a | - | /taxpayer/penalty_overrides/interest | input | user_input / never_fill | CONFIRMED | taxpayer | frm1701MS:txtInterestNo28a |
| II.28C | - | Compromise | frm1701MS:txtCompromise28a | - | /taxpayer/penalty_overrides/compromise | input | user_input / never_fill | CONFIRMED | taxpayer | frm1701MS:txtCompromise28a |
| II.28D | 28 | Total penalties | frm1701MS:txtTotalPenaltiesNo28a | - | - | computed | none / cross_check | CONFIRMED | none | - |
| II.28D | 28 | Total penalties | frm1701MS:txtTotalPenaltiesNo28b | - | - | computed | none / cross_check | CONFIRMED (sheet ID by label) | none | - |
| II.29 | 29 | Total amount payable / overpayment | frm1701MS:txtTotalAmountPayableNo29a | - | - | computed | none / cross_check | CONFIRMED | none | - |
| II.29 | 29 | Total amount payable / overpayment | frm1701MS:txtTotalAmountPayableNo29b | - | - | computed | none / cross_check | CONFIRMED (sheet ID by label) | none | - |
| II.30 | - | Aggregate amount payable / overpayment | frm1701MS:txtAggregateAmountPayable30a | - | - | computed | none / cross_check | CONFIRMED | none | - |
| II.31 | - | Overpayment options and amounts | frm1701MS:txtToBeRefunded | - | /to_be_refunded | input | user_input / never_fill | CONFIRMED | none | frm1701MS:txtToBeRefunded |
| II.31 | - | Overpayment options and amounts | frm1701MS:txtToBeIssued | - | /to_be_issued_tcc | input | user_input / never_fill | CONFIRMED | none | frm1701MS:txtToBeIssued |
| II.31 | - | Overpayment options and amounts | frm1701MS:txtToBeCarried | - | /to_be_carried_over | input | user_input / never_fill | CONFIRMED | none | frm1701MS:txtToBeCarried |
| II.31 | - | Overpayment options and amounts | frm1701MS:txtToBeRefunded1 | - | /refund_amount | input | user_input / never_fill | CONFIRMED | none | frm1701MS:txtToBeRefunded1 |
| II.31 | - | Overpayment options and amounts | frm1701MS:txtToBeIssued1 | - | /tcc_amount | input | user_input / never_fill | CONFIRMED | none | frm1701MS:txtToBeIssued1 |
| II.31 | - | Overpayment options and amounts | frm1701MS:txtToBeCarried1 | - | /carry_over_amount | input | user_input / never_fill | CONFIRMED | none | frm1701MS:txtToBeCarried1 |
| II.32 | - | Declaration / agreement | frm1701MS:perjuryClause | - | /perjury_agreed | input | user_input / never_fill | CONFIRMED | none | frm1701MS:perjuryClause |
| II.33 | - | Number of attachments (schema: 'Page 1 Item 33'; added as II.33 by correction 32) | frm1701MS:printedName | - | /signatory_name | input | user_input / never_fill | CONFIRMED | none | frm1701MS:printedName |
| II.33 | - | Number of attachments (schema: 'Page 1 Item 33'; added as II.33 by correction 32) | frm1701MS:authorizeRepresentative | - | /authorized_representative | input | user_input / never_fill | CONFIRMED | none | frm1701MS:authorizeRepresentative |
| II.35 | - | Number of attachments | frm1701MS:txtPg1I33NumberOfAttachments | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| III.36.bank | - | Cash / bank debit: Drawee bank / agency | frm1701MS:txtCashBankDrawee | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.36.number | - | Cash / bank debit: Reference number | frm1701MS:txtCashBankNumber | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.36.date | - | Cash / bank debit: Payment date | frm1701MS:txtCashBankDate | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.36.amount | - | Cash / bank debit: Amount | frm1701MS:txtCashBankAmount | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.37.bank | - | Check: Drawee bank / agency | frm1701MS:txtCheckDrawee | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.37.number | - | Check: Reference number | frm1701MS:txtCheckNumber | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.37.date | - | Check: Payment date | frm1701MS:txtCheckDate | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.37.amount | - | Check: Amount | frm1701MS:txtCheckAmount | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.38.number | - | Tax debit memo: Reference number | frm1701MS:txtTaxDebitMemoNumber | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.38.date | - | Tax debit memo: Payment date | frm1701MS:txtTaxDebitMemoDate | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.38.amount | - | Tax debit memo: Amount | frm1701MS:txtTaxDebitMemoAmount | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.39.particular | - | Other payment: other particulars (txtOtherParticulars) (added by correction 33) | frm1701MS:txtOtherParticulars | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.39.bank | - | Other payment: Drawee bank / agency | frm1701MS:txtOtherDrawee | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.39.number | - | Other payment: Reference number | frm1701MS:txtOtherNumber | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.39.date | - | Other payment: Payment date | frm1701MS:txtOtherDate | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| III.39.amount | - | Other payment: Amount | frm1701MS:txtOtherAmount | - | - | not_fillable | evidence / never_fill | CONFIRMED | none | - |
| I.6 | - | TIN | frm1701MS:txtPg2TIN1 | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| I.6 | - | TIN | frm1701MS:txtPg2TIN2 | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| I.6 | - | TIN | frm1701MS:txtPg2TIN3 | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| I.6 | - | TIN | frm1701MS:txtPg2BranchCode | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| I.8 | - | Name | frm1701MS:Pg2TaxPayer | - | /taxpayer_name | locked_profile lock=profile | profile / never_fill | CONFIRMED | taxpayer | - |
| IV.1 | - | Gross compensation income | frm1701MS:txtGrossa | - | /taxpayer/gross_compensation | input | evidence / never_fill | CONFIRMED | taxpayer | frm1701MS:txtGrossa |
| IV.2 | - | Non-taxable / exempt compensation | frm1701MS:txtLessNonTaxablea | - | /taxpayer/non_taxable_compensation | input | evidence / never_fill | CONFIRMED | taxpayer | frm1701MS:txtLessNonTaxablea |
| IV.3 | - | Taxable compensation | frm1701MS:txtTaxableCompensationIncomea | - | - | computed | none / cross_check | CONFIRMED | none | - |
| IV.3 | - | Taxable compensation | frm1701MS:txtTaxableCompensationIncomeb | - | - | computed | none / cross_check | CONFIRMED (sheet ID by label) | none | - |
| IV.1 | 4 | Gross compensation income | frm1701MS:txtTaxpayerNo4a | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| IV.1 | 4 | Gross compensation income | frm1701MS:txtTaxpayerNo4b | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| IV.5 | 5 | Sales / revenues / fees | frm1701MS:txtSalesRevenueNo5a | - | /taxpayer/sales | input | ledger / fill_p7 | CONFIRMED | taxpayer | frm1701MS:txtSalesRevenueNo5a |
| IV.6 | 6 | Sales returns, allowances and discounts | frm1701MS:txtSalesReturnsNo6a | - | /taxpayer/sales_returns | input | ledger / fill_p7 | CONFIRMED | taxpayer | frm1701MS:txtSalesReturnsNo6a |
| IV.7 | 7 | Net sales / revenues / fees | frm1701MS:txtTaxableCompensationIncomeNo7a | - | - | computed | none / cross_check | CONFIRMED | none | - |
| IV.7 | 7 | Net sales / revenues / fees | frm1701MS:txtTaxableCompensationIncomeNo7b | - | - | computed | none / cross_check | CONFIRMED (sheet ID by label) | none | - |
| IV.8 | 8 | Cost of sales / services | frm1701MS:txtCostofSalesNo8a | - | /taxpayer/cost_of_sales | input | ledger / fill_p7 | CONFIRMED | taxpayer | frm1701MS:txtCostofSalesNo8a |
| IV.9 | 9 | Gross income / loss from operation | frm1701MS:txtGrossIncomeNo9a | - | - | computed | none / cross_check | CONFIRMED | none | - |
| IV.9 | 9 | Gross income / loss from operation | frm1701MS:txtGrossIncomeNo9b | - | - | computed | none / cross_check | CONFIRMED (sheet ID by label) | none | - |
| IV.10A | 10 | Ordinary allowable itemized deductions | frm1701MS:txtAllowableItemizedDeductionsNo10a | - | /taxpayer/itemized_deductions | input | ledger / fill_p7 | CONFIRMED | taxpayer | frm1701MS:txtAllowableItemizedDeductionsNo10a |
| IV.10B | 10 | Special allowable itemized deductions | frm1701MS:txtSpecialAllowableNo10a | - | /taxpayer/special_allowable_deductions | input | user_input / never_fill | CONFIRMED | taxpayer | frm1701MS:txtSpecialAllowableNo10a |
| IV.10C | 10 | Net operating loss carryover | frm1701MS:txtNetOperatingLossNo10a | - | /taxpayer/nolco | input | prior_filing / never_fill | CONFIRMED | taxpayer | frm1701MS:txtNetOperatingLossNo10a |
| IV.10D | 10 | Total allowable itemized deductions | frm1701MS:txtTotalAllowableItemixedDeductionsNo10a | - | - | computed | none / cross_check | CONFIRMED | none | - |
| IV.10D | 10 | Total allowable itemized deductions | frm1701MS:txtTotalAllowableItemixedDeductionsNo10b | - | - | computed | none / cross_check | CONFIRMED (sheet ID by label) | none | - |
| IV.11 | 11 | Optional standard deduction | frm1701MS:txtOptionalDeductionNo11a | - | - | computed | none / cross_check | CONFIRMED | none | - |
| IV.11 | 11 | Optional standard deduction | frm1701MS:txtOptionalDeductionNo11b | - | - | computed | none / cross_check | CONFIRMED (sheet ID by label) | none | - |
| IV.12 | 12 | Net income / loss | frm1701MS:txtNetIncomeLossNo12a | - | - | computed | none / cross_check | CONFIRMED | none | - |
| IV.12 | 12 | Net income / loss | frm1701MS:txtNetIncomeLossNo12b | - | - | computed | none / cross_check | CONFIRMED (sheet ID by label) | none | - |
| IV.13 | 13 | Other non-operating income | frm1701MS:txtNonOperatingIncomeNo13a | - | /taxpayer/non_operating_income | input | ledger / fill_p7 | CONFIRMED | taxpayer | frm1701MS:txtNonOperatingIncomeNo13a |
| IV.14 | 14 | Taxable income - business | frm1701MS:txtTaxableIncomeBusinesNo14a | - | - | computed | none / cross_check | CONFIRMED | none | - |
| IV.14 | 14 | Taxable income - business | frm1701MS:txtTaxableIncomeBusinesNo14b | - | - | computed | none / cross_check | CONFIRMED (sheet ID by label) | none | - |
| IV.15 | 15 | Taxable compensation and business income | frm1701MS:txtCompensationBusinessNo15a | - | - | computed | none / cross_check | CONFIRMED | none | - |
| IV.15 | 15 | Taxable compensation and business income | frm1701MS:txtCompensationBusinessNo15b | - | - | computed | none / cross_check | CONFIRMED (sheet ID by label) | none | - |
| IV.16 | - | Special / preferential rate | frm1701MS:txtTaxRatea | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| IV.16 | - | Special / preferential rate | frm1701MS:txtTaxRateb | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |
| IV.17 | - | Special / preferential / exempt tax due | frm1701MS:txtTaxDueSpeciala | - | - | computed | none / cross_check | CONFIRMED | none | - |
| IV.17 | - | Special / preferential / exempt tax due | frm1701MS:txtTaxDueSpecialb | - | - | computed | none / cross_check | CONFIRMED (sheet ID by label) | none | - |
| IV.18A | - | Total special / exempt + compensation tax | frm1701MS:TotalTaxDueCompensationATP | - | - | computed | none / cross_check | UNCONFIRMED | none | - |
| IV.18A | - | Total special / exempt + compensation tax | frm1701MS:TotalTaxDueCompensationASP | - | - | computed | none / cross_check | UNCONFIRMED | none | - |
| IV.18B | - | Total tax due under graduated rates | frm1701MS:TotalTaxDueCompensationBTP | - | - | computed | none / cross_check | CONFIRMED | none | - |
| IV.18B | - | Total tax due under graduated rates | frm1701MS:TotalTaxDueCompensationBSP | - | - | computed | none / cross_check | CONFIRMED | none | - |
| IV.19 | 19 | 8%: net sales / revenues / fees | frm1701MS:txtSalesRevenuesFeesANo19a | - | /taxpayer/eight_sales | input | ledger / fill_p7 | CONFIRMED | taxpayer | frm1701MS:txtSalesRevenuesFeesANo19a |
| IV.20 | 20 | 8%: other non-operating income | frm1701MS:txtOtherNonOperatingIncomeNo20a | - | /taxpayer/eight_other_income | input | ledger / fill_p7 | CONFIRMED | taxpayer | frm1701MS:txtOtherNonOperatingIncomeNo20a |
| IV.21 | 21 | 8%: total income | frm1701MS:txtTotalIncomeNo21a | - | - | computed | none / cross_check | CONFIRMED | none | - |
| IV.21 | 21 | 8%: total income | frm1701MS:txtTotalIncomeNo21b | - | - | computed | none / cross_check | CONFIRMED (sheet ID by label) | none | - |
| IV.22 | - | 8%: pure-business reduction | frm1701MS:txtLess22A | - | - | not_fillable | none / never_fill | CONFIRMED | none | - |
| IV.22 | - | 8%: pure-business reduction | frm1701MS:txtLess22B | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |
| IV.23 | - | 8%: taxable income / loss | frm1701MS:txtTaxableIncomeLoss23A | - | - | computed | none / cross_check | CONFIRMED | none | - |
| IV.23 | - | 8%: taxable income / loss | frm1701MS:txtTaxableIncomeLoss23B | - | - | computed | none / cross_check | CONFIRMED (sheet ID by label) | none | - |
| IV.24 | - | 8%: business income tax | frm1701MS:txtTaxDueBusinessIncome24A | - | - | computed | none / cross_check | CONFIRMED | none | - |
| IV.24 | - | 8%: business income tax | frm1701MS:txtTaxDueBusinessIncome24B | - | - | computed | none / cross_check | CONFIRMED (sheet ID by label) | none | - |
| IV.25 | 25 | 8%: total compensation + business tax | frm1701MS:txtCompensationBusinessNo25a | - | - | computed | none / cross_check | CONFIRMED | none | - |
| IV.25 | 25 | 8%: total compensation + business tax | frm1701MS:txtCompensationBusinessNo25b | - | - | computed | none / cross_check | CONFIRMED (sheet ID by label) | none | - |
| V.1 | 1 | Prior year excess credits | frm1701MS:txtPriorYearsNo1a | - | /taxpayer/prior_year_excess | input | prior_filing / never_fill | CONFIRMED | taxpayer | frm1701MS:txtPriorYearsNo1a |
| V.2 | 2 | Tax payments for first three quarters | frm1701MS:txtTaxPaymentsNo2a | - | /taxpayer/quarterly_payments | input | prior_filing / never_fill | CONFIRMED | taxpayer | frm1701MS:txtTaxPaymentsNo2a |
| V.3 | 3 | Creditable tax withheld for first three quarters | frm1701MS:txtCreditableTaxNo3a | - | /taxpayer/cwt_q1_q3 | input | evidence / never_fill | CONFIRMED | taxpayer | frm1701MS:txtCreditableTaxNo3a |
| V.4 | 4 | Creditable tax withheld for fourth quarter | frm1701MS:txtCreditableFourthQuarterNo4a | - | /taxpayer/cwt_q4 | input | evidence / never_fill | CONFIRMED | taxpayer | frm1701MS:txtCreditableFourthQuarterNo4a |
| V.5 | 5 | Creditable tax withheld per 2316 | frm1701MS:txtCreditableForm2316No5a | - | /taxpayer/cwt_2316 | input | evidence / never_fill | CONFIRMED | taxpayer | frm1701MS:txtCreditableForm2316No5a |
| V.6 | 6 | Tax paid on prior return if amended | frm1701MS:txtTaxPaidPreviouslyFiledNo6a | - | /taxpayer/previously_filed | input | prior_filing / never_fill | CONFIRMED | taxpayer | frm1701MS:txtTaxPaidPreviouslyFiledNo6a |
| V.7 | - | Foreign tax credits + foreign tax number | frm1701MS:addForeignTaxCredits | - | /foreign_tax_credits_description | input | evidence / never_fill | CONFIRMED | none | frm1701MS:addForeignTaxCredits |
| V.7 | 7 | Foreign tax credits + foreign tax number | frm1701MS:txtForeignTaxCreditsNo7a | - | /taxpayer/foreign_tax_credits | input | evidence / never_fill | CONFIRMED | taxpayer | frm1701MS:txtForeignTaxCreditsNo7a |
| V.8 | 8 | Special tax credits | frm1701MS:txtSpecialTaxCreditsNo8a | - | /taxpayer/special_tax_credits | input | user_input / never_fill | CONFIRMED | taxpayer | frm1701MS:txtSpecialTaxCreditsNo8a |
| V.9 | - | Other tax credits/payments + description | frm1701MS:addOtherCreditsPayments | - | /other_credits_description | input | user_input / never_fill | CONFIRMED | none | frm1701MS:addOtherCreditsPayments |
| V.9 | 9 | Other tax credits/payments + description | frm1701MS:txtOtherCreditsPaymentsNo9a | - | /taxpayer/other_credits | input | user_input / never_fill | CONFIRMED | taxpayer | frm1701MS:txtOtherCreditsPaymentsNo9a |
| V.10 | 10 | Total tax credits and payments | frm1701MS:txtTotalTaxCreditsPaymentsPartVNo10a | - | - | computed | none / cross_check | CONFIRMED | none | - |
| V.10 | 10 | Total tax credits and payments | frm1701MS:txtTotalTaxCreditsPaymentsPartVNo10b | - | - | computed | none / cross_check | CONFIRMED (sheet ID by label) | none | - |
| VI.1 | - | Total tax relief availment | frm1701MS:totalTaxReliefAvailmentA | - | - | computed | none / cross_check | CONFIRMED | none | - |
| VI.1 | - | Total tax relief availment | frm1701MS:totalTaxReliefAvailmentB | - | - | computed | none / cross_check | CONFIRMED (sheet ID by label) | none | - |
| app | - | (app-written by BIR; no workbook row) | frm1701MS:txtCurrentPage | - | - | not_fillable | none / never_touch | CONFIRMED | none | - |
| app | - | (app-written by BIR; no workbook row) | frm1701MS:txtMaxPage | - | - | not_fillable | none / never_touch | CONFIRMED | none | - |
| IV.10A | - | Ordinary allowable itemized deductions | frm1701MS:sched410A:Description{N>=0} | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |
| IV.10A | - | Ordinary allowable itemized deductions | frm1701MS:sched410A:Amount{N>=0} | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |
| IV.10A | - | Ordinary allowable itemized deductions | frm1701MS:sched410aSpouse:Description{N>=0} | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |
| IV.10A | - | Ordinary allowable itemized deductions | frm1701MS:sched410aSpouse:Amount{N>=0} | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |
| IV.10A | - | Ordinary allowable itemized deductions | frm1701MS:sched410A1:Description{N>=0} | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |
| IV.10A | - | Ordinary allowable itemized deductions | frm1701MS:sched410A1:LegalBasis{N>=0} | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |
| IV.10A | - | Ordinary allowable itemized deductions | frm1701MS:sched410A1:Amount{N>=0} | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |
| IV.10A | - | Ordinary allowable itemized deductions | frm1701MS:sched410A2:Description{N>=0} | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |
| IV.10A | - | Ordinary allowable itemized deductions | frm1701MS:sched410A2:LegalBasis{N>=0} | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |
| IV.10A | - | Ordinary allowable itemized deductions | frm1701MS:sched410A2:Amount{N>=0} | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |
| IV.13 | - | Other non-operating income | frm1701MS:sched413A:Description{N>=0} | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |
| IV.13 | - | Other non-operating income | frm1701MS:sched413A:Amount{N>=0} | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |
| IV.13 | - | Other non-operating income | frm1701MS:Sched413B:Description{N>=0} | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |
| IV.13 | - | Other non-operating income | frm1701MS:Sched413B:Amount{N>=0} | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |
| IV.20 | - | 8%: other non-operating income | frm1701MS:sched420A:Description{N>=0} | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |
| IV.20 | - | 8%: other non-operating income | frm1701MS:sched420A:Amount{N>=0} | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |
| IV.20 | - | 8%: other non-operating income | frm1701MS:sched42OB:Description{N>=0} | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |
| IV.20 | - | 8%: other non-operating income | frm1701MS:sched420B:Amount{N>=0} | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |
| V.9 | - | Other tax credits/payments + description | frm1701MS:sched5No9:Description{N>=0} | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |
| V.9 | - | Other tax credits/payments + description | frm1701MS:sched5No9:Amount{N>=0} | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |
| V.9 | - | Other tax credits/payments + description | frm1701MS:sched5No9:Spouse{N>=0} | - | - | not_fillable | none / never_fill | CONFIRMED (sheet ID by label) | none | - |

Notes for 1701-MS:
- **RISK (blocker B7, CONFIRMED):** items 12 (`txtTaxpayerNo12`, value `c` = 8%), 17 (`txtTaxpayerNo17`, `a` itemized, `b` OSD) and the `taxCodeDropdown` values II015-II017 have no lock. Nothing compares them with the profile election (verify.md s1 #3, s2 #10).
- `txtTaxableCompensationIncomeNo7a` is a computed display box for IV.7 (net sales). Its name says 'taxable compensation'. It is not bound (verify.md s2 #15).
- The attachments box reads 'Page 1 Item 33'. The workbook had it as II.35. Corrected by correction 32.
- No key exists for II.34 (TIN of authorized representative) (verify.md s2 #14).
- `TotalTaxDueCompensationATP` and `ASP` are mapped to IV.18A by pattern (UNCONFIRMED). No pinned box maps to IV.4.
- Schedules 4.10A, 4.13A/B, 4.20A/B and 5 No. 9 are row families with no row template. They are not fillable.

## 6. Spouse (B-side) fields, 1701-MS

37 spouse entries mirror the taxpayer side (for example `frm1701MS:txtGrossb` = `/spouse/gross_compensation`). Policy: `refuse_mvp_spouse`. Spouse is phase P7 at the earliest.

## 7. Fields by source (who populates each field)

Counts exclude spouse entries. The lists give workbook Sheet IDs. A field may appear under only one source.

### Form 2551Q (quarterly percentage tax), January 2018 (ENCS)

| Books source | Books policy | Count | Sheet IDs |
|---|---|---|---|
| evidence | fill_mvp | 1 | II.15 |
| evidence | never_fill | 17 | III.25.amount, III.25.bank, III.25.date, III.25.number, III.26.amount, III.26.bank, III.26.date, III.26.number, III.27.amount, III.27.bank, III.27.date, III.27.number, III.28.amount, III.28.bank, III.28.date, III.28.number, III.28.particular |
| ledger | fill_mvp | 2 | S1.1.ATC, S1.1.base |
| ledger | never_fill | 10 | S1.2.ATC, S1.2.base, S1.3.ATC, S1.3.base, S1.4.ATC, S1.4.base, S1.5.ATC, S1.5.base, S1.6.ATC, S1.6.base |
| none | cross_check | 18 | II.14, II.18, II.19, II.23, II.24, S1.1.rate, S1.1.tax, S1.2.rate, S1.2.tax, S1.3.rate, S1.3.tax, S1.4.rate, S1.4.tax, S1.5.rate, S1.5.tax, S1.6.rate, S1.6.tax, S1.7 |
| none | never_fill | 10 | H.2, H.3, I.11, I.6 |
| none | never_touch | 13 | app, app-credential, page2-mirror |
| prior_filing | never_fill | 1 | II.16 |
| profile | never_fill | 9 | H.1, H.2, I.10, I.11, I.13, I.7, I.8, I.9, I.9A |
| user_input | never_fill | 13 | DECL.agent, H.4, H.5, I.12, I.12A, II.17, II.20, II.21, II.22, II.overpayment |

### Form 1601-C (monthly compensation withholding), January 2018 (ENCS)

| Books source | Books policy | Count | Sheet IDs |
|---|---|---|---|
| evidence | never_fill | 18 | II.29, III.37.amount, III.37.bank, III.37.date, III.37.number, III.38.amount, III.38.bank, III.38.date, III.38.number, III.39.amount, III.39.date, III.39.number, III.40.amount, III.40.bank, III.40.date, III.40.number, III.40.particular |
| none | cross_check | 13 | II.21, II.22, II.24, II.26, II.27, II.30, II.31, II.35, II.36, S1.1.7, S1.2.7, S1.3.7, S1.4 |
| none | fill_p6 | 1 | H.5 |
| none | never_fill | 16 | H.1, I.6, S1.1.1, S1.2.1, S1.3.1, S1.N |
| none | never_touch | 11 | app, page2-mirror |
| payroll | fill_p6 | 11 | H.3, II.14, II.15, II.16, II.17, II.18, II.19, II.20, II.23, II.25 |
| prior_filing | never_fill | 16 | II.28, S1.1.2, S1.1.3, S1.1.4, S1.1.5, S1.1.6, S1.2.2, S1.2.3, S1.2.4, S1.2.5, S1.2.6, S1.3.2, S1.3.3, S1.3.4, S1.3.5, S1.3.6 |
| profile | never_fill | 7 | I.10, I.12, I.7, I.8, I.9, I.9A |
| user_input | never_fill | 12 | DECL.signature, H.2, H.4, I.11, I.13, I.13A, I.9, II.32, II.33, II.34 |

### Form 1701-MS (annual return, micro and small), August 2024

| Books source | Books policy | Count | Sheet IDs |
|---|---|---|---|
| evidence | never_fill | 24 | II.20, III.36.amount, III.36.bank, III.36.date, III.36.number, III.37.amount, III.37.bank, III.37.date, III.37.number, III.38.amount, III.38.date, III.38.number, III.39.amount, III.39.bank, III.39.date, III.39.number, III.39.particular, IV.1, IV.2, V.3, V.4, V.5, V.7 |
| ledger | fill_p7 | 7 | IV.10A, IV.13, IV.19, IV.20, IV.5, IV.6, IV.8 |
| none | cross_check | 53 | II.19, II.21, II.22, II.23, II.24, II.25, II.27, II.28D, II.29, II.30, IV.10D, IV.11, IV.12, IV.14, IV.15, IV.17, IV.18A, IV.18B, IV.21, IV.23, IV.24, IV.25, IV.3, IV.7, IV.9, V.10, VI.1 |
| none | never_fill | 41 | H.1, I.6, II.35, IV.1, IV.10A, IV.13, IV.16, IV.20, IV.22, V.9 |
| none | never_touch | 2 | app |
| prior_filing | never_fill | 4 | IV.10C, V.1, V.2, V.6 |
| profile | never_fill | 11 | I.10, I.11, I.12, I.17, I.18, I.7, I.8, I.9 |
| user_input | never_fill | 27 | H.1, H.2, H.3, H.4, H.5, I.13, I.14, I.15, I.16, II.26, II.28A, II.28B, II.28C, II.31, II.32, II.33, IV.10B, V.8, V.9 |

Plain reading, by source (PROPOSAL):

| Source | Meaning in this crosswalk | Examples |
|---|---|---|
| ledger | Books computes it from posted lines and tax attributes | 2551Q S1 row 1 base; 1701-MS IV.5, IV.6, IV.8, IV.10A, IV.13, IV.19, IV.20 |
| payroll | Books computes it from payroll runs (P6) | 1601-C II.14-II.20, II.23, II.25 |
| evidence | A certificate or a payment record, with proof | 2551Q II.15 (2307 WB, received only); 1701-MS V.3-V.5; payment rows |
| profile | Owned by the BIR profile; Books never sends it | TIN, RDO, name, address, email, election, year-end month |
| prior_filing | Comes from an earlier return or payment | 2551Q II.16 (amended); 1701-MS V.1, V.2, V.6, IV.10C |
| user_input | A person decides or enters it | amended flag, relief, penalties, declarations, overpayment choice |
| none | Identity, a constant or a Forms computation | period identity; computed totals (cross-check only) |

**Not supported in MVP:** payroll (1601-C, P6), 1701-MS (P7), evidence and 2307 import (P4), spouse, credit notes, VAT (2550Q), 1700, 1702, 0619-E, 1601-EQ, 1604-C.

## 8. Observed: checks performed on the crosswalk

- Pinned file sha256 equals the digest recorded in each JSON (`schema_sha256`), and the buwiz-oss copy is equal.
- Every entry key is a pinned box key. Every draft path equals the path or option sets in the pin. Every lock equals the pin's lock.
- Every typed alias is found in verify.md or forms.md (schedule-row aliases are checked as patterns).
- Every pinned key is covered by an entry, a key pattern or a group member (0 uncovered keys in all three forms).
- Every rule ID referenced by an entry is defined in `data/rules/` (242 distinct IDs, 0 missing, 0 duplicates).
- Every Sheet ID is on its field tab, except the rows added by corrections 29, 31 and 33, the family pattern `S1.N`, and the internal labels `app`, `app-credential` and `page2-mirror`.

## 9. Unresolved discrepancies (list)

| # | Discrepancy | Sources | Status | Owner or decision |
|---|---|---|---|---|
| D-01 | Local checkout 301f9a0f is stale; the pin is 63267ca (2551Q and 1701-MS differ). | verify s1 #1, s2 #3 | CONFIRMED | Forms owner (use the pin) |
| D-02 | Items 20-22: the stale local file leaves 20-21 unbound and binds 22 to /compromise; the pin binds /penalty_overrides/*. | verify s2 #5 | CONFIRMED (L stale) | Forms owner |
| D-03 | Typed 2551Q 'compromise' writes the root; auto mode overwrites it. Official txt22 is durable. | verify s2 #9 | CONFIRMED (overwrite); typed write path PARTIAL | Forms owner |
| D-04 | 1701-MS items 12, 17 and II015-II017 have no lock; nothing compares them with the profile election. | verify s1 #3, s2 #10, s3 | CONFIRMED | Forms owner (locks before any 1701-MS fill: B7) |
| D-05 | 1701-MS key IV.7 'txtTaxableCompensationIncomeNo7a' is computed display; its name suggests compensation. | verify s2 #15; sheets s3 | CONFIRMED (REFUTED as 'bound') | No change to the official key |
| D-06 | Attachments: schema item 33 vs workbook II.35; II.34 has no key. | verify s4; #14; sheets s3 | CONFIRMED | Workbook fixed (correction 32) |
| D-07 | 1601-C item 39 has no drawee key; items 37, 38, 40 unmapped; typed rows N>=3 not traced. | verify s2 #13, #17 | CONFIRMED (missing); N>=3 UNCONFIRMED | Forms owner |
| D-08 | 1601-C item 19 path /tax_19_sss_gsis is narrower than the printed label. | forms s3b; verify s3b | CONFIRMED | Do not rename without the PDF (verify s2 #16) |
| D-09 | 1601-C employee contributions are dropped in BK-TS (II.19 absent; II.22 overstated by 2,000 in the probe). | verify s2 #28; books s1 #1 | CONFIRMED defect | Books owner (regression test in P6) |
| D-10 | Schedule I rows 0-2 only; typed N>=3 not traced. | verify s2 #17 | UNCONFIRMED | Forms owner |
| D-11 | 2551Q item 13 'Not applicable' (taxRateNA) is in the pin; printed form lists two options. | verify s2 #7; bir s2 | UNCONFIRMED (printed NA) | Re-read the printed form |
| D-12 | 1701-MS I.18 has II011-II017 as box values; the taxpayer binding has no II011. | pinned schema | CONFIRMED | No action (documented) |
| D-13 | 1701-MS IV.4 has no pinned box; ATP/ASP mapped to IV.18A by pattern. | W2 check of pin | UNCONFIRMED | Forms owner |
| D-14 | 1701-MS printed rounding ('49 centavos down') is image-only and UNCONFIRMED here; the design lists it as CONFIRMED. | verify s2 #83; design s7/s8 | UNCONFIRMED | Design correction (docs/07) |
| D-15 | 1701-MS 15 October second installment: design says CONFIRMED (bir D-4), verify says UNCONFIRMED (#83). | bir D-4; verify #83 | UNCONFIRMED | Re-read the printed form |
| D-16 | 1601-C printed due-date text: UNCONFIRMED here; code CONFIRMED. | verify #83; books s4a; bir D-3 | UNCONFIRMED (printed) | Re-read the printed form |
| D-17 | OSD base: printed Part I item 17 says 40% of gross sales; Part IV item 11 and the code say 40% of net sales. | bir s2; sheets s5; forms | UNCONFIRMED (conflict) | Re-read the printed form; tax adviser |
| D-18 | 8% year with 2551Q: Books blocks; buwiz-oss requires NIL PT010; RMO 23-2018 says not required. | verify #73, #84; books s4d | CONFLICT CONFIRMED; resolution UNCONFIRMED | Owner decision D5; read RR 8-2018 full text |
| D-19 | 1% PT claim to 2025-12-31: UNCONFIRMED. Books and buwiz-oss use 3% after 2023-06-30. | verify #62, #63 | UNCONFIRMED | Owner decision D6; primary text |
| D-20 | Credit notes and sales returns in the 2551Q base: bir.md PT-8 finds no printed deduction; the workbook example deducts 10,000. | bir PT-8; workbook worked example | UNCONFIRMED | Owner decision D20; tax adviser |
| D-21 | CPI-adjusted VAT threshold: statute says indexed; no figure found. | bir PT-7 | UNKNOWN | Keep 3,000,000.00 as a dated fact; review yearly |
| D-22 | Annex C (RMC 20-2026) is the TY2025 FAQ. TY2026 applicability UNKNOWN. | verify #80 | UNKNOWN | Owner: confirm the TY2026 basis |
| D-23 | 1601-C monthly withholding method: workbook 2,808.4 vs annualized 2,808.33; method UNCONFIRMED. | verify #49 | UNCONFIRMED | Payroll owner |
| D-24 | Interest final-tax attribute: not stored in Books (UNKNOWN). | sheets s10 | UNKNOWN | Books owner |
| D-25 | Books (percentage-tax.ts:65-68) and buwiz-oss (atc.rs:140-165) hard-code the 1% window. Rates should come from rates-v1.json rows instead. | verify #35 | CONFIRMED | Books and buwiz-oss owners |

## 10. Recommended

- **Recommended:** keep the pin as the only Forms reference until the Forms owner publishes a newer one. Re-run the crosswalk check on every pin change (PROPOSAL).
- **Recommended:** add locks for 1701-MS items 12 and 17 and the tax code dropdown in the Forms repo before any 1701-MS fill (blocker B7; Forms owner).
- **Recommended:** keep the sheet IDs as labels. Do not rename official keys (verify s2 #16).
- **Recommended:** use the fill keys in the tables for `fill_args`. Never use a bare key that the pin does not list.
