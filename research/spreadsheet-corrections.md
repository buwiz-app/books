# Spreadsheet corrections (W2)

Owner: W2 (tax mapping data). Status: research. Scope: the two research workbooks in `research/inputs/`. The corrected copies are in `research/corrected/`, and the script that makes them is `tools/correct_workbooks.py`.

Labels: CONFIRMED, PARTIAL, MISSING, UNKNOWN, UNCONFIRMED. INFERENCE and PROPOSAL are marked where used. **Observed** = read in a workbook cell, schema or code. **Recommended** = a proposal.

## 1. Observed: what the workbooks are and what was checked

- **Observed (CONFIRMED):** `Buwiz_Tax_Profiles_Categories_Form_Fields.xlsx` has 10 sheets and 7 Excel tables. The log calls it `Form-map`. `Philippine_Bookkeeping_and_BIR_Form_Map.xlsx` has 5 sheets and no tables. The log calls it `Form-map-2`.
- **Observed:** the source of truth for each correction is `sheets.md` section 6 (35 items). Each item is checked against the verdict in `verify.md` section 2 (items 44-61 cover the workbook). verify.md governs conflicts.
- **Observed:** no cell in either workbook claims to hold an API key. The field-tab IDs (for example `II.14`, `S1.1.base`) are spreadsheet labels (sheets.md s3). So no field-key cell was replaced. Verified keys are in `data/crosswalk/*.json`.
- **Observed:** the field tabs are Excel tables (`Fields2551Q`, `Fields1601C`, `Fields1701MS`, `CategoryCatalog`). Rows appended to them extend the table range.
- **Observed:** sheets.md cites `1701-MS fields!B22` for I.18; the I.18 row is row 23. Recorded as item 34.

## 2. Result

- Items in the log: 40 (35 from sheets.md s6, plus 5 extra items found during the check).
- Applied (including the W2 re-read in item 19): 22. No change needed: 3. Not applied (refuted, UNCONFIRMED, open or owner decision): 15.
- Every applied change keeps the original value unless the item says otherwise. Two formula changes (8% sheet and worked-example chain) were checked by recalculation: the figures are unchanged (section 6).

## 3. Item-by-item

| # | Workbook | Sheet and cell(s) | Correction (sheets.md s6 or extra) | verify.md verdict | Action | What changed or why |
|---|---|---|---|---|---|---|
| 1 | Form-map | Worked example: A12:B12, C12 | Relabel II.24 as II.19 'Tax still payable' (II.14 - II.18). Add II.14 and II.24 rows. | CONFIRMED (#46) | APPLIED | Relabelled A12 to '2551Q II.19'. Added II.14 (row 40) and II.24 (row 41) with formulas. |
| 2 | Form-map | Worked example: C11, A11:B11 | II.18 must be II.15 + II.16 + II.17 (value 0 unchanged). | CONFIRMED (#45) | APPLIED | C11 is now '=C10+0+0'. II.16 and II.17 have no rows here and are zero. |
| 3 | Form-map | Worked example: A25:B25, C25 | Relabel II.36 as II.31 (II.27 - II.30). Add II.36 = II.31 + II.35. | CONFIRMED (#48) | APPLIED | A25 = '1601-C II.31'. II.36 added as row 42 (=C25+0; II.35 is zero). |
| 4 | Form-map | Worked example: C17, D17 | II.21 must sum II.15 to II.20 (value 4,000 unchanged). | CONFIRMED (#47) | APPLIED | C17 is now '=0+0+0+C15+C16+0' (II.15-II.17 and II.20 are zero here). |
| 5 | Form-map | Worked example: C33, B33, D33 | Graduated tax applies to IV.15 = IV.3 + IV.14. Add IV.3 (= 0). Value unchanged (85,300). | CONFIRMED (#50) | APPLIED | C33 applies to (C43+C32). IV.3 added as row 43 = 0. |
| 6 | Form-map | Worked example: C37, A37:B37 | V.10 must sum V.1 to V.9 (value 52,300 unchanged). | UNCONFIRMED (#54) | NOT APPLIED (needs review) | Left as is. The worked example sums V.2 to V.4 only. The E99 formula in the field tab reads 'Sum V.1 through V.9' (text). |
| 7 | Form-map | Worked example: A39:B39, C39, D39 | Show the chain II.23 -> II.25 -> II.27 -> II.29 (value 33,000 unchanged). | CONFIRMED (#53) | APPLIED | A39 = '1701-MS II.25'; C39 = C44 - C38. II.23, II.27, II.29 added as rows 44-46. |
| 8 | Form-map | Example data: H17 | Change 2808.4 to 2,808.33 under the annualized method, or document the payroll source. | UNCONFIRMED (#49): arithmetic CONFIRMED, payroll method UNCONFIRMED | NOT APPLIED (needs review) | Left at 2808.4. The annualized figure is arithmetic only; the payroll withholding method is not verified. |
| 9 | Form-map | Example data: H8 | Confirm the INV-Q3 2307 amount (2,900 vs 3,000 peers). If 3,000, the worked example changes. | UNCONFIRMED (#52) | NOT APPLIED (needs review) | Left at 2900. Whether 2,900 is an error is not established. |
| 10 | Form-map | Example data: A12, I12 | PAY-Q3 holds one month of payroll (September). Relabel as September or add July and August. | CONFIRMED (#55) | APPLIED | A12 = 'PAY-SEP'. I12 notes 'September 2026 only; July and August absent'. |
| 11 | Form-map | (none): - | No change: Worked example C8 and C30 are consistent with code. | n/a (sheets.md s9) | NO CHANGE | No action. |
| 12 | Form-map | Form catalog C10; Profile A18: C10 (1701-MS scope) | Change scope 'Individual/Fiduciary' to 'Individual' (Annex C A7). | REFUTED as stated (#60) | NOT APPLIED (do not change) | RMC 34-2025 Annex A item 3 includes trustees, guardians and executors where the estate or minor is in trade. The label 'Individual/Fiduciary' stays. |
| 13 | Form-map | Profile A18:A21 (checks a-e): A30:A33 (added) | Add review checks (a)-(e) to A18:A21. | PARTIAL: (c) CONFIRMED (#75); (a) REFUTED as worded (#60); (e) conflict CONFIRMED (#73, #84); (b) and (d) not verified | APPLIED in part | Added check (c) (mixed earners, no 250,000) and check (e) (8% and 2551Q conflict) as new rows 32 and 33. (a), (b), (d) not added. |
| 14 | Form-map | Profile C12; Form catalog A5: - | Wire Profile C12 into Form catalog A5 (1700 row). | C12 unused CONFIRMED (#58); target REFUTED (#59: A5 is the header 'Form') | NOT APPLIED (do not change) | The target cell is the header, not the 1700 row. Profile C12 stays unwired until a 1700 row exists. |
| 15 | Form-map-2 | 8 percent example: B14:E14, A15:E15 | Replace MAX(0, ...) with a signed formula; show overpayment on a separate line. | CONFIRMED (#57) | APPLIED | B14:E14 = B11-B12-B13 (signed). Row 15 'Overpayment (separate line)' = MAX(0,-row 14). Values unchanged. |
| 16 | Form-map | Category field rules A7:B8, A16:A17: - | Add a regime filter (1701-MS I.12) to sales and refund rules for IV.5, IV.19, IV.6. | Not in verify.md; sheets.md says PARTIAL | NOT APPLIED (needs review) | Rule text carries the filter in the rules file (1701MS.IV.5 selector). The workbook text is not changed until the engine's 8% behaviour is checked. |
| 17 | Form-map | Category field rules A42: - | depreciation_expense source should be the tax-depreciation schedule, not the GL account. | Not in verify.md | NOT APPLIED (needs review) | Catalog row 43 already says 'Use fixed asset schedule and tax adjustment'. Rule text unchanged pending review. |
| 18 | Form-map | Category field rules A18:A21: - | Final-tax exclusion needs a tax attribute. Whether Books stores one is UNKNOWN. | UNKNOWN (sheets.md s10); not in verify.md | NOT APPLIED (open) | Tax attribute final_tax_flag = unknown_not_stored (tax-attributes.v1.json). |
| 19 | Form-map | Category catalog C18, C19, C20, C21, C22, C28, C29, C42, C47: Account type column | Map labels to Books enum values: contra revenue -> revenue (subtype refunds_discounts); other income -> other_income; cost -> cost_of_revenue; other expense -> other_expense. | Not in verify.md. W2 re-read of account-constants.ts:9-18 confirms the enum (asset, liability, equity, revenue, expense, cost_of_revenue, other_income, other_expense). | APPLIED (W2 re-read) | Nine cells changed. Contra-revenue detail moved to the checks column (I18). |
| 20 | Form-map | Category catalog A13 (owner_drawings): A13 | Decide reuse of owners_equity 32000 or keep a new key. | Owner decision (sheets.md s6 #20) | NOT APPLIED (owner decision) | Recorded as PENDING_DECISION in categories.v1.json. |
| 21 | Form-map | Category catalog rows 65-67 (added): A65:I67 | Add rows for ph_employee_advances (12700), ph_employee_tax_advanced (12800) and ph_unrecovered_employee_tax (61950). | CONFIRMED (#40) | APPLIED | Names and types from Books ph-chart.ts:84-104 and 232-245. Activation set to PAYROLL. Table range extended. |
| 22 | Form-map-2 | Forms and timing D18: - | Add 'December -> 15 January regardless of channel' and the eFPS group days to the 1601-C timing. | UNCONFIRMED (#83: printed due text image-only). Code CONFIRMED (books.md s4a), not in verify.md | NOT APPLIED (needs review) | Due-date rule kept in rates-v1.json with status UNCONFIRMED for the printed text. |
| 23 | Form-map-2 | Filer combinations F9: - | Change 'OSD is generally 40% of individual gross sales' to 40% of net sales. | Not in verify.md. Printed Part I item 17 wording conflicts with Part IV item 11 (bir.md s2) | NOT APPLIED (needs review) | Conflict recorded in rates-v1.json (osd UNCONFIRMED). |
| 24 | Form-map | Build notes and sources C21: C21 | State the 8% and 2551Q conflict: Books blocks 2551Q; buwiz-oss requires NIL PT010 rows. | CONFLICT CONFIRMED (#73, #84); RR 8-2018 full text UNCONFIRMED | APPLIED | Note appended. Conflict stated, no side chosen. |
| 25 | Form-map and Form-map-2 | Build notes and sources C16; Sources C8: C16 (wb1), C8 (wb2) | Caveat: Annex C covers the 2025 annual return filed in 2026. | CONFIRMED (#80; #76) | APPLIED | Caveat appended to both cells. TY2026 applicability stays UNKNOWN. |
| 26 | Form-map | Build notes and sources C20: C20 | Note that the 8% test includes other non-operating income in the Forms engine, and that Books omits it. | CONFIRMED for Books (#36, #75). The Forms-engine line (form_1701ms.rs:49) is from sheets.md s5 only | APPLIED (Books part only) | Note appended for the Books omission. The Forms-engine line is not stated because verify.md did not check it. |
| 27 | Form-map | Build notes and sources C22: C22 | 'ComputedReturnForm lacks 1701-MS' is Books-only; buwiz-oss has form_1701ms.rs. | CONFIRMED (#34); buwiz-oss copy of the schema is byte-identical (verify s3) | APPLIED | Note appended. |
| 28 | Form-map | 2551Q fields C20 (I.13): C20 | Add the 'Not applicable' option (buwiz-oss taxRateNA). | CONFIRMED that the pin has taxRateNA (#7); printed NA option UNCONFIRMED (bir.md s2) | APPLIED (with caveat) | Option text added to C20 with the caveat. |
| 29 | Form-map | 2551Q fields, row 77 (added): A77:H77 | Add a 'Particular 28' row (txtParticular28) after III.28. | CONFIRMED (#20) | APPLIED | Row appended after the last row (keeps IDs unique). Table range extended. |
| 30 | Form-map | 1601-C fields B52 (III.39.bank): F52 | No drawee key for item 39. Drop the field or add the key in the Forms repo. | CONFIRMED (#13) | APPLIED as annotation (row kept) | Row kept and annotated. Adding a Forms key is a Forms-repo change and was not made. |
| 31 | Form-map | 1601-C fields, row 84 (added): A84:H84 | Add a 'Particular 40' row (txtParticular40). | CONFIRMED (#13; verify s4: txtParticular40 unmapped) | APPLIED | Row appended. Table range extended. |
| 32 | Form-map | 1701-MS fields B41, B42, B43: B41:B43, C43 | II.35 should be II.33 per the schema (Item 33). II.34 has no schema key. | CONFIRMED (verify s4 item 33; #14 II.34 missing) | APPLIED | B43 = 'II.33' (attachments). B41 marked 'II.33 (name, signature; unnumbered in schema)'. F42 notes the missing II.34 key. |
| 33 | Form-map | 1701-MS fields B52 (III.38.bank) and row 101 (added): F52, A101:H101 | No tax-debit-memo drawee key. Add a row for III.39 'Other particulars' (txtOtherParticulars). | CONFIRMED (verify s4 payment lines) | APPLIED | Annotation on F52. Row appended for txtOtherParticulars. Table range extended. |
| 34 | Form-map | 1701-MS fields B22 -> row 23 (I.18): F23 | List the official values II011 to II017. | CONFIRMED (pinned schema box options). Sheets cites B22; the I.18 row is row 23 | APPLIED | F23 lists II011-II017 and the binding detail. The off-by-one reference is recorded here. |
| 35 | Form-map | 1601-C fields B26; 1701-MS fields B67: - | Keep the IDs. Rename /tax_19_sss_gsis and txtTaxableCompensationIncomeNo7a in the Forms repo. | UNCONFIRMED (#16: renaming official keys is risky); #15 REFUTED ('bound' claim) | NOT APPLIED (IDs kept; no Forms rename) | No cell changed. The renames are not made. |
| 36 | Form-map | Worked example C7 (290,000 base after a 10,000 return): C7 | Extra (not in sheets.md s6): bir.md PT-8 finds no printed deduction for sales returns in the 2551Q instructions read. | Not in verify.md; bir.md PT-8 UNVERIFIED | NOT APPLIED (needs review) | Base 290,000 comes from SUMIFS minus the return. Treatment of returns is open (decision D20). |
| 37 | Form-map | 1701-MS fields E99 / Worked example V.10: - | Extra: the printed Part V item 10 total is V.10. The workbook's worked example does not sum V.1 to V.9. | UNCONFIRMED (#54) | NOT APPLIED (needs review) | See item 6. |
| 38 | Form-map | 1701-MS fields row 33 (IV.8) and row 35 (IV.10A): - | Extra: the printed Part I item 17 '40% of gross sales' wording conflicts with IV.11 '40% of IV.7' (bir.md s2). | Not in verify.md | NOT APPLIED (needs review) | Recorded in rates-v1.json (osd UNCONFIRMED). |
| 39 | Form-map | Form-map 2551Q: txt15, II.15: - | Extra: the verified schema key frm2551Qv2018:txt15 is accepted; bare txt15 is refused. No workbook cell claims a bare key. | CONFIRMED (verify s1 #1, s2 #4) | NO CHANGE | Recorded so no one 'fixes' the sheet IDs into keys without the prefix. |
| 40 | Both | All field-tab ID cells (H.1 ... III.40.bank): - | Field-key replacement: replace any cell that claims an API key with the verified key. | Checked: no cell in the workbook claims an API key. Field IDs are spreadsheet labels (sheets.md s3) | NO CHANGE | Verified official keys are in data/crosswalk/*.json. The sheet IDs stay as labels. |

## 4. Do not change (refuted or deliberately kept)

- **1701-MS scope stays "Individual/Fiduciary"** (item 12). verify.md s2 #60: RMC 34-2025 Annex A item 3 includes trustees, guardians and executors where the estate or minor is in trade. The sheets.md change to "Individual" is refuted.
- **Form catalog A5 is the header, not the 1700 row** (item 14, verify.md s2 #59). Profile C12 stays unwired until a 1700 row exists.
- **IV.7 is not a bound key** (sheets.md s3 #15, verify.md s2 #15). The computed box feeds a display map. The official key name is misleading and stays.
- **Official key renames are not made** (item 35, verify.md s2 #16: UNCONFIRMED and risky). Fix labels and paths, not official keys, unless the printed PDF field list is checked.
- **No bare keys in the workbook** (item 39). Forms accepts exact prefixed keys (for example `frm2551Qv2018:txt15`). A bare `txt15` is refused (verify.md s1 #1).

## 5. Needs review (UNCONFIRMED, PARTIAL or open)

- **Item 6** V.10 summation: verify.md s2 #54 UNCONFIRMED.
- **Item 8** monthly 1601-C figure 2,808.4 vs 2,808.33: arithmetic CONFIRMED, payroll method UNCONFIRMED (verify.md s2 #49).
- **Item 9** 2,900 vs 3,000 CWT on INV-Q3: values CONFIRMED, error UNCONFIRMED (verify.md s2 #52).
- **Items 16, 17, 18** category rules: regime filter, depreciation source and final-tax attribute. Not in verify.md; sheets.md marks them PARTIAL or UNKNOWN.
- **Item 20** owner decision on `owner_drawings`.
- **Item 22** printed 1601-C due-date text (verify.md s2 #83 UNCONFIRMED) and item 23 OSD wording conflict (bir.md s2).
- **Items 36-38** extra: the 10,000 return deduction in the 2551Q example (bir.md PT-8), the V.10 sum, and the OSD wording conflict.

## 6. Verification performed

- Reopened both corrected workbooks with openpyxl. Sheets, tables and the `Corrections log` sheet are present. Table ranges now include the appended rows.
- Recalculated the worked-example formulas in a pure-Python evaluator (`formulas` library, run through uv). All 16 checked cells match the expected figures (for example C9 = 8,700; C17 = 4,000; C33 = 85,300; C39 = 33,000). Figures did not change.
- The workbook is saved with full recalculation on load, because openpyxl does not write cached values. Opening it in Excel or Numbers recalculates the formulas.
- The script does not change the input workbook. It loads the input, applies the listed changes (failing if an expected old value is missing), and saves a new file.

## 7. Outputs

- `research/inputs/Buwiz_Tax_Profiles_Categories_Form_Fields.xlsx`, `research/inputs/Philippine_Bookkeeping_and_BIR_Form_Map.xlsx`, `research/inputs/Buwiz_Books_Rox_Claude_Handover.md` (copies).
- `research/corrected/Buwiz_Tax_Profiles_Categories_Form_Fields.xlsx`, `research/corrected/Philippine_Bookkeeping_and_BIR_Form_Map.xlsx`.
- `tools/correct_workbooks.py`. Run it with: `uv run -q --with openpyxl python -I tools/correct_workbooks.py <in> <out>`.

**Recommended:** owner review of items 6, 8, 9, 16-18, 20, 22, 23 and 36-38 before the next research round. Adopt the corrected workbooks only after the review.
