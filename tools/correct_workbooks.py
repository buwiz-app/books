"""Apply the verified corrections from research/spreadsheet-corrections.md to a copy of a research workbook.

Usage (from the repo root):
    uv run -q --with openpyxl python -I tools/correct_workbooks.py <input.xlsx> <output.xlsx>

The input file is only read. The output is a new workbook that keeps every original sheet,
applies only corrections whose verdict is CONFIRMED (or the W2 re-read noted in the log), and
adds a 'Corrections log' sheet that lists every item from sheets.md s6, including the ones
NOT applied (REFUTED, UNCONFIRMED or owner decisions). No field-key cell is replaced: none of
the workbook's cells claims an API key (see the log).
"""
import re
import sys
from copy import copy

import openpyxl
from openpyxl.styles import Alignment, Font, PatternFill
from openpyxl.workbook.properties import CalcProperties

ORIGIN_NOTE = ("Corrections are applied from research/spreadsheet-corrections.md (W2, 2026-10-29). "
               "Workbook labels: Form-map = Buwiz_Tax_Profiles_Categories_Form_Fields.xlsx; Form-map-2 = Philippine_Bookkeeping_and_BIR_Form_Map.xlsx.")

# (id, workbook, sheet, cells, correction text (sheets.md s6), verify verdict, action, note)
LOG = [
 (1, "Form-map", "Worked example", "A12:B12, C12", "Relabel II.24 as II.19 'Tax still payable' (II.14 - II.18). Add II.14 and II.24 rows.", "CONFIRMED (#46)", "APPLIED", "Relabelled A12 to '2551Q II.19'. Added II.14 (row 40) and II.24 (row 41) with formulas."),
 (2, "Form-map", "Worked example", "C11, A11:B11", "II.18 must be II.15 + II.16 + II.17 (value 0 unchanged).", "CONFIRMED (#45)", "APPLIED", "C11 is now '=C10+0+0'. II.16 and II.17 have no rows here and are zero."),
 (3, "Form-map", "Worked example", "A25:B25, C25", "Relabel II.36 as II.31 (II.27 - II.30). Add II.36 = II.31 + II.35.", "CONFIRMED (#48)", "APPLIED", "A25 = '1601-C II.31'. II.36 added as row 42 (=C25+0; II.35 is zero)."),
 (4, "Form-map", "Worked example", "C17, D17", "II.21 must sum II.15 to II.20 (value 4,000 unchanged).", "CONFIRMED (#47)", "APPLIED", "C17 is now '=0+0+0+C15+C16+0' (II.15-II.17 and II.20 are zero here)."),
 (5, "Form-map", "Worked example", "C33, B33, D33", "Graduated tax applies to IV.15 = IV.3 + IV.14. Add IV.3 (= 0). Value unchanged (85,300).", "CONFIRMED (#50)", "APPLIED", "C33 applies to (C43+C32). IV.3 added as row 43 = 0."),
 (6, "Form-map", "Worked example", "C37, A37:B37", "V.10 must sum V.1 to V.9 (value 52,300 unchanged).", "UNCONFIRMED (#54)", "NOT APPLIED (needs review)", "Left as is. The worked example sums V.2 to V.4 only. The E99 formula in the field tab reads 'Sum V.1 through V.9' (text)."),
 (7, "Form-map", "Worked example", "A39:B39, C39, D39", "Show the chain II.23 -> II.25 -> II.27 -> II.29 (value 33,000 unchanged).", "CONFIRMED (#53)", "APPLIED", "A39 = '1701-MS II.25'; C39 = C44 - C38. II.23, II.27, II.29 added as rows 44-46."),
 (8, "Form-map", "Example data", "H17", "Change 2808.4 to 2,808.33 under the annualized method, or document the payroll source.", "UNCONFIRMED (#49): arithmetic CONFIRMED, payroll method UNCONFIRMED", "NOT APPLIED (needs review)", "Left at 2808.4. The annualized figure is arithmetic only; the payroll withholding method is not verified."),
 (9, "Form-map", "Example data", "H8", "Confirm the INV-Q3 2307 amount (2,900 vs 3,000 peers). If 3,000, the worked example changes.", "UNCONFIRMED (#52)", "NOT APPLIED (needs review)", "Left at 2900. Whether 2,900 is an error is not established."),
 (10, "Form-map", "Example data", "A12, I12", "PAY-Q3 holds one month of payroll (September). Relabel as September or add July and August.", "CONFIRMED (#55)", "APPLIED", "A12 = 'PAY-SEP'. I12 notes 'September 2026 only; July and August absent'."),
 (11, "Form-map", "(none)", "-", "No change: Worked example C8 and C30 are consistent with code.", "n/a (sheets.md s9)", "NO CHANGE", "No action."),
 (12, "Form-map", "Form catalog C10; Profile A18", "C10 (1701-MS scope)", "Change scope 'Individual/Fiduciary' to 'Individual' (Annex C A7).", "REFUTED as stated (#60)", "NOT APPLIED (do not change)", "RMC 34-2025 Annex A item 3 includes trustees, guardians and executors where the estate or minor is in trade. The label 'Individual/Fiduciary' stays."),
 (13, "Form-map", "Profile A18:A21 (checks a-e)", "A30:A33 (added)", "Add review checks (a)-(e) to A18:A21.", "PARTIAL: (c) CONFIRMED (#75); (a) REFUTED as worded (#60); (e) conflict CONFIRMED (#73, #84); (b) and (d) not verified", "APPLIED in part", "Added check (c) (mixed earners, no 250,000) and check (e) (8% and 2551Q conflict) as new rows 32 and 33. (a), (b), (d) not added."),
 (14, "Form-map", "Profile C12; Form catalog A5", "-", "Wire Profile C12 into Form catalog A5 (1700 row).", "C12 unused CONFIRMED (#58); target REFUTED (#59: A5 is the header 'Form')", "NOT APPLIED (do not change)", "The target cell is the header, not the 1700 row. Profile C12 stays unwired until a 1700 row exists."),
 (15, "Form-map-2", "8 percent example", "B14:E14, A15:E15", "Replace MAX(0, ...) with a signed formula; show overpayment on a separate line.", "CONFIRMED (#57)", "APPLIED", "B14:E14 = B11-B12-B13 (signed). Row 15 'Overpayment (separate line)' = MAX(0,-row 14). Values unchanged."),
 (16, "Form-map", "Category field rules A7:B8, A16:A17", "-", "Add a regime filter (1701-MS I.12) to sales and refund rules for IV.5, IV.19, IV.6.", "Not in verify.md; sheets.md says PARTIAL", "NOT APPLIED (needs review)", "Rule text carries the filter in the rules file (1701MS.IV.5 selector). The workbook text is not changed until the engine's 8% behaviour is checked."),
 (17, "Form-map", "Category field rules A42", "-", "depreciation_expense source should be the tax-depreciation schedule, not the GL account.", "Not in verify.md", "NOT APPLIED (needs review)", "Catalog row 43 already says 'Use fixed asset schedule and tax adjustment'. Rule text unchanged pending review."),
 (18, "Form-map", "Category field rules A18:A21", "-", "Final-tax exclusion needs a tax attribute. Whether Books stores one is UNKNOWN.", "UNKNOWN (sheets.md s10); not in verify.md", "NOT APPLIED (open)", "Tax attribute final_tax_flag = unknown_not_stored (tax-attributes.v1.json)."),
 (19, "Form-map", "Category catalog C18, C19, C20, C21, C22, C28, C29, C42, C47", "Account type column", "Map labels to Books enum values: contra revenue -> revenue (subtype refunds_discounts); other income -> other_income; cost -> cost_of_revenue; other expense -> other_expense.", "Not in verify.md. W2 re-read of account-constants.ts:9-18 confirms the enum (asset, liability, equity, revenue, expense, cost_of_revenue, other_income, other_expense).", "APPLIED (W2 re-read)", "Nine cells changed. Contra-revenue detail moved to the checks column (I18)."),
 (20, "Form-map", "Category catalog A13 (owner_drawings)", "A13", "Decide reuse of owners_equity 32000 or keep a new key.", "Owner decision (sheets.md s6 #20)", "NOT APPLIED (owner decision)", "Recorded as PENDING_DECISION in categories.v1.json."),
 (21, "Form-map", "Category catalog rows 65-67 (added)", "A65:I67", "Add rows for ph_employee_advances (12700), ph_employee_tax_advanced (12800) and ph_unrecovered_employee_tax (61950).", "CONFIRMED (#40)", "APPLIED", "Names and types from Books ph-chart.ts:84-104 and 232-245. Activation set to PAYROLL. Table range extended."),
 (22, "Form-map-2", "Forms and timing D18", "-", "Add 'December -> 15 January regardless of channel' and the eFPS group days to the 1601-C timing.", "UNCONFIRMED (#83: printed due text image-only). Code CONFIRMED (books.md s4a), not in verify.md", "NOT APPLIED (needs review)", "Due-date rule kept in rates-v1.json with status UNCONFIRMED for the printed text."),
 (23, "Form-map-2", "Filer combinations F9", "-", "Change 'OSD is generally 40% of individual gross sales' to 40% of net sales.", "Not in verify.md. Printed Part I item 17 wording conflicts with Part IV item 11 (bir.md s2)", "NOT APPLIED (needs review)", "Conflict recorded in rates-v1.json (osd UNCONFIRMED)."),
 (24, "Form-map", "Build notes and sources C21", "C21", "State the 8% and 2551Q conflict: Books blocks 2551Q; buwiz-oss requires NIL PT010 rows.", "CONFLICT CONFIRMED (#73, #84); RR 8-2018 full text UNCONFIRMED", "APPLIED", "Note appended. Conflict stated, no side chosen."),
 (25, "Form-map and Form-map-2", "Build notes and sources C16; Sources C8", "C16 (wb1), C8 (wb2)", "Caveat: Annex C covers the 2025 annual return filed in 2026.", "CONFIRMED (#80; #76)", "APPLIED", "Caveat appended to both cells. TY2026 applicability stays UNKNOWN."),
 (26, "Form-map", "Build notes and sources C20", "C20", "Note that the 8% test includes other non-operating income in the Forms engine, and that Books omits it.", "CONFIRMED for Books (#36, #75). The Forms-engine line (form_1701ms.rs:49) is from sheets.md s5 only", "APPLIED (Books part only)", "Note appended for the Books omission. The Forms-engine line is not stated because verify.md did not check it."),
 (27, "Form-map", "Build notes and sources C22", "C22", "'ComputedReturnForm lacks 1701-MS' is Books-only; buwiz-oss has form_1701ms.rs.", "CONFIRMED (#34); buwiz-oss copy of the schema is byte-identical (verify s3)", "APPLIED", "Note appended."),
 (28, "Form-map", "2551Q fields C20 (I.13)", "C20", "Add the 'Not applicable' option (buwiz-oss taxRateNA).", "CONFIRMED that the pin has taxRateNA (#7); printed NA option UNCONFIRMED (bir.md s2)", "APPLIED (with caveat)", "Option text added to C20 with the caveat."),
 (29, "Form-map", "2551Q fields, row 77 (added)", "A77:H77", "Add a 'Particular 28' row (txtParticular28) after III.28.", "CONFIRMED (#20)", "APPLIED", "Row appended after the last row (keeps IDs unique). Table range extended."),
 (30, "Form-map", "1601-C fields B52 (III.39.bank)", "F52", "No drawee key for item 39. Drop the field or add the key in the Forms repo.", "CONFIRMED (#13)", "APPLIED as annotation (row kept)", "Row kept and annotated. Adding a Forms key is a Forms-repo change and was not made."),
 (31, "Form-map", "1601-C fields, row 84 (added)", "A84:H84", "Add a 'Particular 40' row (txtParticular40).", "CONFIRMED (#13; verify s4: txtParticular40 unmapped)", "APPLIED", "Row appended. Table range extended."),
 (32, "Form-map", "1701-MS fields B41, B42, B43", "B41:B43, C43", "II.35 should be II.33 per the schema (Item 33). II.34 has no schema key.", "CONFIRMED (verify s4 item 33; #14 II.34 missing)", "APPLIED", "B43 = 'II.33' (attachments). B41 marked 'II.33 (name, signature; unnumbered in schema)'. F42 notes the missing II.34 key."),
 (33, "Form-map", "1701-MS fields B52 (III.38.bank) and row 101 (added)", "F52, A101:H101", "No tax-debit-memo drawee key. Add a row for III.39 'Other particulars' (txtOtherParticulars).", "CONFIRMED (verify s4 payment lines)", "APPLIED", "Annotation on F52. Row appended for txtOtherParticulars. Table range extended."),
 (34, "Form-map", "1701-MS fields B22 -> row 23 (I.18)", "F23", "List the official values II011 to II017.", "CONFIRMED (pinned schema box options). Sheets cites B22; the I.18 row is row 23", "APPLIED", "F23 lists II011-II017 and the binding detail. The off-by-one reference is recorded here."),
 (35, "Form-map", "1601-C fields B26; 1701-MS fields B67", "-", "Keep the IDs. Rename /tax_19_sss_gsis and txtTaxableCompensationIncomeNo7a in the Forms repo.", "UNCONFIRMED (#16: renaming official keys is risky); #15 REFUTED ('bound' claim)", "NOT APPLIED (IDs kept; no Forms rename)", "No cell changed. The renames are not made."),
 (36, "Form-map", "Worked example C7 (290,000 base after a 10,000 return)", "C7", "Extra (not in sheets.md s6): bir.md PT-8 finds no printed deduction for sales returns in the 2551Q instructions read.", "Not in verify.md; bir.md PT-8 UNVERIFIED", "NOT APPLIED (needs review)", "Base 290,000 comes from SUMIFS minus the return. Treatment of returns is open (decision D20)."),
 (37, "Form-map", "1701-MS fields E99 / Worked example V.10", "-", "Extra: the printed Part V item 10 total is V.10. The workbook's worked example does not sum V.1 to V.9.", "UNCONFIRMED (#54)", "NOT APPLIED (needs review)", "See item 6."),
 (38, "Form-map", "1701-MS fields row 33 (IV.8) and row 35 (IV.10A)", "-", "Extra: the printed Part I item 17 '40% of gross sales' wording conflicts with IV.11 '40% of IV.7' (bir.md s2).", "Not in verify.md", "NOT APPLIED (needs review)", "Recorded in rates-v1.json (osd UNCONFIRMED)."),
 (39, "Form-map", "Form-map 2551Q: txt15, II.15", "-", "Extra: the verified schema key frm2551Qv2018:txt15 is accepted; bare txt15 is refused. No workbook cell claims a bare key.", "CONFIRMED (verify s1 #1, s2 #4)", "NO CHANGE", "Recorded so no one 'fixes' the sheet IDs into keys without the prefix."),
 (40, "Both", "All field-tab ID cells (H.1 ... III.40.bank)", "-", "Field-key replacement: replace any cell that claims an API key with the verified key.", "Checked: no cell in the workbook claims an API key. Field IDs are spreadsheet labels (sheets.md s3)", "NO CHANGE", "Verified official keys are in data/crosswalk/*.json. The sheet IDs stay as labels."),
]

def find_table(ws, name):
    for t in ws.tables.values():
        if t.name == name:
            return t
    return None

def extend_table(ws, name, new_last_row):
    t = find_table(ws, name)
    if t is None:
        raise SystemExit(f"table {name} not found")
    first, last = t.ref.split(":")
    col_last = re.match(r"([A-Z]+)", last).group(1)
    t.ref = f"{first}:{col_last}{new_last_row}"
    if t.autoFilter is not None:
        t.autoFilter.ref = t.ref

def expect(ws, addr, old):
    cur = ws[addr].value
    if cur != old:
        raise SystemExit(f"{ws.title}!{addr}: expected {old!r}, found {cur!r}")

def set_value(ws, addr, new, old=None):
    if old is not None:
        expect(ws, addr, old)
    ws[addr].value = new

def copy_style(src, dst):
    dst._style = copy(src._style)

def apply_wb1(wb):
    ws = wb["Worked example"]
    set_value(ws, "B11", "Total credits / payments (II.15 + II.16 + II.17; II.16 and II.17 are zero here)", "Total credits / payments")
    set_value(ws, "C11", "=C10+0+0", "=C10")
    set_value(ws, "D11", "Only percentage-tax credit in this example; II.16 and II.17 are zero (correction 2)", "Only percentage-tax credit in this example")
    set_value(ws, "A12", "2551Q II.19", "2551Q II.24")
    set_value(ws, "B12", "Tax still payable (II.14 - II.18)", "Amount payable")
    set_value(ws, "D12", "Tax due minus applicable credits (correction 1: was labelled II.24)", "Tax due minus applicable credits")
    set_value(ws, "B17", "Total non-taxable compensation (II.15 to II.20)", "Total non-taxable compensation")
    set_value(ws, "C17", "=0+0+0+C15+C16+0", "=C15+C16")
    set_value(ws, "D17", "II.15, II.16, II.17 and II.20 are zero here; II.18 = C15 and II.19 = C16 (correction 4)", "Only II.18 and II.19 in this example")
    set_value(ws, "A25", "1601-C II.31", "1601-C II.36")
    set_value(ws, "B25", "Tax still due / over-remittance (II.27 - II.30)", "Amount still due")
    set_value(ws, "D25", "No penalties in this example (correction 3: was labelled II.36)", "No penalties in this example")
    set_value(ws, "B33", "Graduated income tax due (applied to IV.15 = IV.3 + IV.14)", "Graduated income tax due")
    old33 = ws["C33"].value
    assert "C32" in old33
    ws["C33"].value = re.sub(r"\bC32\b", "(C43+C32)", old33)
    set_value(ws, "D33", "Applied to IV.15 = IV.3 + IV.14; IV.3 is zero here (correction 5). Illustration: current progressive brackets", "Illustration: current progressive brackets")
    set_value(ws, "A39", "1701-MS II.25", "1701-MS II.29")
    set_value(ws, "B39", "Tax payable / overpayment (II.23 - II.24)", "Amount payable")
    set_value(ws, "C39", "=C44-C38", "=C33-C38")
    set_value(ws, "D39", "Equals II.27 and II.29 here: no second installment or penalties (correction 7). Chain in rows 43-46.", "Assumes no installment, penalties or other credits")
    # added rows (labels in A:B, formulas in C, notes in D; styles copied from row 38)
    added = [
        (40, "2551Q II.14", "Total tax due (= Schedule 1 item 7)", "=C9", "Added by correction 1."),
        (41, "2551Q II.24", "Total amount payable (II.19 + II.23)", "=C12+0", "Added by correction 1. II.23 (penalties) is zero here."),
        (42, "1601-C II.36", "Amount still due (II.31 + II.35)", "=C25+0", "Added by correction 3. II.35 (penalties) is zero here."),
        (43, "1701-MS IV.3", "Taxable compensation (IV.1 - IV.2)", 0, "Added by correction 5. Zero in this example (no compensation)."),
        (44, "1701-MS II.23", "Total income tax due (II.21 + II.22)", "=0+C33", "Added by correction 7. II.21 is zero here."),
        (45, "1701-MS II.27", "Amount payable (II.25 - II.26)", "=C39-0", "Added by correction 7. II.26 (second installment) is zero here."),
        (46, "1701-MS II.29", "Total amount payable (II.27 + II.28D)", "=C45+0", "Added by correction 7. II.28D (penalties) is zero here."),
    ]
    for r, a, b, c, d in added:
        for col, v in (("A", a), ("B", b), ("C", c), ("D", d)):
            cell = ws[f"{col}{r}"]
            cell.value = v
            copy_style(ws[f"{col}38"], cell)
    ws = wb["Example data"]
    set_value(ws, "A12", "PAY-SEP", "PAY-Q3")
    set_value(ws, "I12", "Gross employee payroll, September 2026 only (one month; July and August absent) (correction 10)", "Gross employee payroll")
    ws = wb["Category catalog"]
    relabel = {18: ("contra revenue", "revenue"), 19: ("other income", "other_income"), 20: ("other income", "other_income"),
               21: ("cost", "cost_of_revenue"), 22: ("cost", "cost_of_revenue"), 28: ("cost", "cost_of_revenue"),
               29: ("cost", "cost_of_revenue"), 42: ("other expense", "other_expense"), 47: ("other expense", "other_expense")}
    for r, (old, new) in relabel.items():
        set_value(ws, f"C{r}", new, old)
    ws["I18"].value = str(ws["I18"].value) + " [Books: revenue + subtype refunds_discounts (correction 19)]"
    # three PH accounts (correction 21), formulas copied from row 64 with the row number changed
    rows = [
        ("ph_employee_advances", "Employee Advances & Receivables", "asset", "12700", "Employee advances and receivables", "Present in Books ph-chart.ts (stage 5b). Not in the catalog before correction 21."),
        ("ph_employee_tax_advanced", "Employee Receivable - Tax Advanced", "asset", "12800", "Employer-advanced tax recoverable from employees", "Referenced by Books annualization-posting.ts."),
        ("ph_unrecovered_employee_tax", "Unrecovered Employee Tax", "expense", "61950", "Employer-borne tax not recovered from employees", "Matters for 1601-C reconciliation. Referenced by annualization-posting.ts."),
    ]
    template = ws["G64"].value
    assert "F64" in template
    for i, (key, name, typ, num, purpose, checks) in enumerate(rows):
        r = 65 + i
        vals = {"A": key, "B": name, "C": typ, "D": num, "E": "Existing PH control", "F": "PAYROLL",
                "G": template.replace("F64", f"F{r}"), "H": purpose, "I": checks}
        for col, v in vals.items():
            cell = ws[f"{col}{r}"]
            cell.value = v
            copy_style(ws[f"{col}64"], cell)
    extend_table(ws, "CategoryCatalog", 67)
    ws = wb["2551Q fields"]
    set_value(ws, "C20", "Income-tax rates availed (options: graduated; 8%; not applicable: pinned key taxRateNA; printed NA option UNCONFIRMED)", "Income-tax rates availed")
    _append_row(ws, 77, ["III", "III.28.particular", "Other payment: particulars (Particular 28; key txtParticular28)", "Payment", "Payment evidence",
                         "Record only if method 28 is used; unmapped in pinned schema (verify s4)", "Payment row", "Specified; added by correction 29"], "Fields2551Q")
    ws = wb["1601-C fields"]
    expect(ws, "B52", "III.39.bank")
    ws["F52"].value = str(ws["F52"].value) + " [No drawee key for item 39 in the pinned schema (txtNumber39, txtDate39, txtAmount39 only). Not fillable; row kept (correction 30).]"
    _append_row(ws, 84, ["III", "III.40.particular", "Other payment: particulars (txtParticular40)", "Payment", "Payment evidence",
                         "Record if method 40 is used; unmapped in pinned schema", "Payment row", "Specified; added by correction 31"], "Fields1601C")
    ws = wb["1701-MS fields"]
    expect(ws, "B41", "II.33")
    ws["B41"].value = "II.33 (name, signature; unnumbered in schema)"
    expect(ws, "B43", "II.35")
    ws["B43"].value = "II.33"
    ws["C43"].value = "Number of attachments (schema: 'Page 1 Item 33 Number Of Attachments', constant) (correction 32)"
    ws["F42"].value = str(ws["F42"].value) + " No key exists for II.34 (TIN of authorized representative) in the pinned schema (verify s2 #14)."
    expect(ws, "B52", "III.38.bank")
    ws["F52"].value = str(ws["F52"].value) + " No tax-debit-memo drawee key in the pinned schema (number, date and amount only). Not fillable (correction 33)."
    expect(ws, "B23", "I.18")
    ws["F23"].value = str(ws["F23"].value) + " Official box values (pinned schema): II011 to II017. Taxpayer binding maps II012 to II017 (no II011 binding); the spouse dropdown maps II011 (correction 34)."
    _append_row(ws, 101, ["III", "III.39.particular", "Other payment: other particulars (txtOtherParticulars)", "Payment", "Payment evidence",
                          "Record if method 39 is used; unmapped in pinned schema", "Payment row", "Specified; added by correction 33"], "Fields1701MS")
    ws = wb["Build notes and sources"]
    ws["C16"].value = str(ws["C16"].value) + " Annex C (RMC 20-2026) is the TY2025 FAQ ('on filing the 2025 annual income tax return'); TY2026 applicability is UNKNOWN (verify s2 #80) (correction 25)."
    ws["C20"].value = str(ws["C20"].value) + " Books omits other non-operating income from the 8% base (percentage-tax.ts:167-191; verify s2 #36). RMO 23-2018 includes it (verify s2 #75) (correction 26)."
    ws["C21"].value = str(ws["C21"].value) + " CONFLICT (CONFIRMED): Books blocks 2551Q in an 8% year (percentage-tax.ts:354-358; deadlines.ts:122); buwiz-oss requires NIL PT010 rows (form_2551q.rs:2217-2226); RMO 23-2018 says an 8% elector does not file 2551Q (verify s2 #73). RR 8-2018 full text not read: UNCONFIRMED (decision D5) (correction 24)."
    ws["C22"].value = str(ws["C22"].value) + " Books ComputedReturnForm (tax-stage-remainder.ts:21) does not list 1701-MS (verify s2 #34). buwiz-oss carries the 1701-MS schema and engine; the schema bytes are identical (verify s3) (correction 27)."
    ws = wb["Profile"]
    ws["A31"].value = "Added review checks (correction 13; (c) and (e) only)"
    ws["A31"].font = Font(bold=True)
    ws["A32"].value = '=IF(AND(C10="8%",C12="Yes"),"Review: mixed earners get no 250,000 reduction on the 8% base (RMO 23-2018; verify s2 #75).","No mixed-earner 8% reduction issue flagged by this check.")'
    ws["A33"].value = '=IF(AND(C10="8%",C7="Yes"),"Open discrepancy: an 8% year with 2551Q selected. Books blocks 2551Q; buwiz-oss requires NIL PT010; RMO 23-2018 says not required (verify s2 #73, #84). Do not file either way until RR 8-2018 is read.","No 8% and 2551Q conflict flagged.")'
    for a in ("A32", "A33"):
        ws[a].alignment = Alignment(wrap_text=True, vertical="top")

def _append_row(ws, row, values, table_name):
    for i, v in enumerate(values):
        col = chr(ord("A") + i)
        cell = ws[f"{col}{row}"]
        cell.value = v
        copy_style(ws[f"{col}{row - 1}"], cell)
    extend_table(ws, table_name, row)

def apply_wb2(wb):
    ws = wb["8 percent example"]
    for col in "BCDE":
        old = ws[f"{col}14"].value
        assert old.startswith("=MAX(0,"), old
        inner = old[len("=MAX(0,"):-1]
        ws[f"{col}14"].value = "=" + inner
        ws[f"A15"].value = "Overpayment (separate line, shown positive; correction 15)"
        ws[f"{col}15"].value = f"=MAX(0,-{col}14)"
        copy_style(ws[f"{col}14"], ws[f"{col}15"])
    copy_style(ws["A14"], ws["A15"])
    ws = wb["Sources"]
    ws["C8"].value = str(ws["C8"].value) + " Annex C is the TY2025 FAQ ('on filing the 2025 annual income tax return'); TY2026 applicability is UNKNOWN (verify s2 #80) (correction 25)."

def add_log(wb, which):
    ws = wb.create_sheet("Corrections log")
    ws["A1"].value = "Corrections log: research workbook (applied from research/spreadsheet-corrections.md)"
    ws["A1"].font = Font(bold=True, size=12)
    ws["A2"].value = ORIGIN_NOTE + " Items are numbered as in sheets.md section 6; extra items are numbered 36 onward."
    head = ["#", "Workbook", "Sheet", "Cell(s)", "Correction", "verify.md verdict", "Action", "What changed or why"]
    for i, h in enumerate(head):
        c = ws.cell(row=4, column=i + 1, value=h)
        c.font = Font(bold=True, color="FFFFFF")
        c.fill = PatternFill("solid", fgColor="1F4E78")
        c.alignment = Alignment(wrap_text=True, vertical="top")
    for r, item in enumerate(LOG, start=5):
        for i, v in enumerate(item):
            c = ws.cell(row=r, column=i + 1, value=v)
            c.alignment = Alignment(wrap_text=True, vertical="top")
    widths = [5, 14, 26, 24, 48, 36, 26, 60]
    for i, w in enumerate(widths):
        ws.column_dimensions[chr(ord("A") + i)].width = w
    ws.freeze_panes = "A5"
    return ws

def main(src, dst):
    wb = openpyxl.load_workbook(src)
    names = set(wb.sheetnames)
    if "Category catalog" in names:
        apply_wb1(wb)
        kind = "Buwiz_Tax_Profiles_Categories_Form_Fields"
    elif "8 percent example" in names:
        apply_wb2(wb)
        kind = "Philippine_Bookkeeping_and_BIR_Form_Map"
    else:
        raise SystemExit("unrecognised workbook")
    add_log(wb, kind)
    wb.calculation = CalcProperties(fullCalcOnLoad=True)
    wb.save(dst)
    print("saved", dst, "sheets:", wb.sheetnames)

if __name__ == "__main__":
    if len(sys.argv) != 3:
        raise SystemExit("usage: correct_workbooks.py <input.xlsx> <output.xlsx>")
    main(sys.argv[1], sys.argv[2])
