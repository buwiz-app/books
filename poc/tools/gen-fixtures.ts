#!/usr/bin/env bun
/**
 * Writes the synthetic fixtures to fixtures/synthetic/. Run from poc/: bun run fixtures
 *
 * Every value is invented. Profiles carry an opaque UUID, a masked display and period statuses only: no tax facts,
 * elections or prior filings (those are Forms', ADR-0007). Lines carry accounting attributes only. Money is decimal
 * text. Memos carry a MEMO-SYNTH- marker so the privacy tests can prove they never reach a proposal.
 */
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { prettyJson } from "../src/canonical.ts";
import { SYNTHETIC_DIR } from "../src/paths.ts";

type Obj = Record<string, unknown>;

const AS_OF = "2026-10-29";
const MASK = "***-***-***-00000";

const PROFILE_A = "6b1d2f3a-4c5e-4f60-8a71-92b3c4d5e6f7";
const PROFILE_B = "2f4e6d8c-1b3a-4c5d-8e7f-0a9b8c7d6e5f";
const PROFILE_C = "3e5f7a9b-2c4d-4e6f-9a8b-7c6d5e4f3a2b";
const PROFILE_D = "4f6a8b0c-3d5e-4f7a-8b9c-0d1e2f3a4b5c";
const PROFILE_E = "c0a8e3f1-2b7d-4e9a-9f10-5d6c7b8a9e0f";
const PROFILE_F = "5a7b9c1d-4e6f-4a8b-9c0d-1e2f3a4b5c6d";
const PROFILE_G = "7c9d1e3f-5a7b-4c9d-8e1f-2a3b4c5d6e7f";

function profile(id: string, label: string, periods: Record<string, string>): Obj {
  return { tax_profile_id: id, display: MASK, label, periods };
}

function fixture(id: string, status: string, description: string, profiles: Obj[], acks: string[], entries: Obj[]): Obj {
  return {
    fixture_format: "buwiz-poc-fixture/2",
    fixture_id: id,
    status,
    description,
    as_of: AS_OF,
    profiles,
    acknowledged_entry_ids: acks,
    entries,
  };
}

// ---- line and entry builders (amounts are decimal text) ----------------------------------------------------------

function entry(profileId: string, entryId: string, date: string, memo: string, lines: Obj[], reverses: string | null = null): Obj {
  return { entry_id: entryId, tax_profile_id: profileId, entry_date: date, memo, reverses_entry_id: reverses, lines };
}

const arDebit = (lineId: string, amount: string): Obj => ({ line_id: lineId, account: "accounts_receivable_gross", debit: amount, credit: "0.00" });
const arCredit = (lineId: string, amount: string): Obj => ({ line_id: lineId, account: "accounts_receivable_gross", debit: "0.00", credit: amount });

function saleLine(lineId: string, amount: string, personScope = "self"): Obj {
  return {
    line_id: lineId,
    account: "sales_revenue",
    debit: "0.00",
    credit: amount,
    measure_side: "credit",
    income_class: "business_sales",
    document_kind: "sale_invoice",
    person_scope: personScope,
  };
}

function sale(profileId: string, entryId: string, date: string, amount: string, personScope = "self"): Obj {
  return entry(profileId, entryId, date, `MEMO-SYNTH-${entryId} synthetic sales invoice`, [
    arDebit(`${entryId}.1`, amount),
    saleLine(`${entryId}.2`, amount, personScope),
  ]);
}

function itCert(profileId: string, entryId: string, date: string, amount: string, periodStart: string, periodEnd: string): Obj {
  return entry(profileId, entryId, date, `MEMO-SYNTH-${entryId} synthetic income-tax 2307`, [
    {
      line_id: `${entryId}.1`,
      account: "ph_cwt_receivable",
      debit: amount,
      credit: "0.00",
      measure_side: "debit",
      certificate_atc: "WI010",
      credit_kind: "it",
      certificate_status: "received",
      period_start: periodStart,
      period_end: periodEnd,
      document_kind: "certificate_2307",
    },
    arCredit(`${entryId}.2`, amount),
  ]);
}

function ptCert(
  profileId: string,
  entryId: string,
  date: string,
  amount: string,
  atc: string,
  status: string,
  periodStart: string,
  periodEnd: string,
): Obj {
  // A WB084 certificate is final withholding: the document is a 2306, not a 2307 (evidence fact).
  const documentKind = atc === "WB084" ? "certificate_2306" : "certificate_2307";
  return entry(profileId, entryId, date, `MEMO-SYNTH-${entryId} synthetic percentage-tax certificate (${status})`, [
    {
      line_id: `${entryId}.1`,
      account: "pct_tax_credit_receivable",
      debit: amount,
      credit: "0.00",
      measure_side: "debit",
      certificate_atc: atc,
      credit_kind: "pt",
      certificate_status: status,
      period_start: periodStart,
      period_end: periodEnd,
      document_kind: documentKind,
    },
    arCredit(`${entryId}.2`, amount),
  ]);
}

function creditNote(profileId: string, entryId: string, date: string, amount: string): Obj {
  return entry(profileId, entryId, date, `MEMO-SYNTH-${entryId} synthetic credit note against a Q3 sale`, [
    {
      line_id: `${entryId}.1`,
      account: "refunds_and_discounts",
      debit: amount,
      credit: "0.00",
      measure_side: "credit",
      income_class: "contra_sales",
      contra_kind: "return",
      document_kind: "credit_note",
    },
    arCredit(`${entryId}.2`, amount),
  ]);
}

function expense(profileId: string, entryId: string, date: string, amount: string, account: string): Obj {
  return entry(profileId, entryId, date, `MEMO-SYNTH-${entryId} synthetic supplier invoice`, [
    {
      line_id: `${entryId}.1`,
      account,
      debit: amount,
      credit: "0.00",
      measure_side: "debit",
      income_class: "operating_expense",
      document_kind: "expense_document",
    },
    { line_id: `${entryId}.2`, account: "bank_accounts", debit: "0.00", credit: amount },
  ]);
}

/** September payroll for one employee. Balanced: 43500.00 each side. */
function payroll(profileId: string, entryId: string, date: string): Obj {
  const who = { person_scope: "employee", party_ref: "EMP-1", document_kind: "payroll_register" };
  return entry(profileId, entryId, date, "MEMO-SYNTH-PAYROLL one synthetic employee, September", [
    { line_id: `${entryId}.1`, account: "salaries", debit: "40000.00", credit: "0.00", measure_side: "debit", comp_component: "basic_pay", ...who },
    { line_id: `${entryId}.2`, account: "employee_benefits", debit: "2000.00", credit: "0.00", measure_side: "debit", comp_component: "de_minimis", ...who },
    {
      line_id: `${entryId}.3`,
      account: "employer_payroll_taxes",
      debit: "1500.00",
      credit: "0.00",
      measure_side: "debit",
      comp_component: "contribution",
      contribution_side: "employer",
      contribution_scheme: "sss_gsis",
      ...who,
    },
    {
      line_id: `${entryId}.4`,
      account: "ph_sss_payable",
      debit: "0.00",
      credit: "2000.00",
      measure_side: "credit",
      comp_component: "contribution",
      contribution_side: "employee",
      contribution_scheme: "sss_gsis",
      ...who,
    },
    {
      line_id: `${entryId}.5`,
      account: "ph_sss_payable",
      debit: "0.00",
      credit: "1500.00",
      measure_side: "credit",
      comp_component: "contribution",
      contribution_side: "employer",
      contribution_scheme: "sss_gsis",
      ...who,
    },
    { line_id: `${entryId}.6`, account: "ph_wtc_payable", debit: "0.00", credit: "2808.40", measure_side: "credit", comp_component: "tax_withheld", ...who },
    { line_id: `${entryId}.7`, account: "ph_net_pay_payable", debit: "0.00", credit: "37191.60" },
  ]);
}

function reversalOf(profileId: string, entryId: string, original: Obj, date: string): Obj {
  const originalLines = original.lines as Obj[];
  return entry(
    profileId,
    entryId,
    date,
    `MEMO-SYNTH-${entryId} synthetic void of ${String(original.entry_id)}`,
    originalLines.map((line, index) => ({
      ...line,
      line_id: `${entryId}.${index + 1}`,
      debit: line.credit,
      credit: line.debit,
    })),
    String(original.entry_id),
  );
}

// ---- the standard year used by fixtures a (sole proprietor) and b (8% elector) -----------------------------------

function standardYear(profileId: string, prefix: string): Obj[] {
  const id = (n: number): string => `${prefix}-${String(n).padStart(3, "0")}`;
  return [
    sale(profileId, id(1), "2026-01-15", "120000.00"),
    sale(profileId, id(2), "2026-02-10", "100000.00"),
    sale(profileId, id(3), "2026-03-20", "80000.00"),
    itCert(profileId, id(4), "2026-03-31", "3000.00", "2026-01-01", "2026-03-31"),
    sale(profileId, id(5), "2026-04-10", "120000.00"),
    sale(profileId, id(6), "2026-05-15", "100000.00"),
    sale(profileId, id(7), "2026-06-20", "80000.00"),
    itCert(profileId, id(8), "2026-06-30", "3000.00", "2026-04-01", "2026-06-30"),
    sale(profileId, id(9), "2026-07-01", "120000.00"),
    sale(profileId, id(10), "2026-07-15", "100000.00"),
    sale(profileId, id(11), "2026-08-20", "80000.00"),
    expense(profileId, id(12), "2026-08-05", "20000.00", "office_supplies"),
    creditNote(profileId, id(13), "2026-09-10", "10000.00"),
    itCert(profileId, id(14), "2026-09-30", "2900.00", "2026-07-01", "2026-09-30"),
    payroll(profileId, id(15), "2026-09-30"),
    sale(profileId, id(16), "2026-10-05", "120000.00"),
    sale(profileId, id(17), "2026-10-20", "100000.00"),
    sale(profileId, id(18), "2026-10-28", "80000.00"),
    itCert(profileId, id(19), "2026-10-28", "3000.00", "2026-10-01", "2026-10-28"),
  ];
}

const STANDARD_PERIODS = { "2026-Q1": "closed", "2026-Q2": "closed", "2026-Q3": "closed", "2026-09": "closed", "2026": "closed" };

export function buildFixtures(): Map<string, Obj> {
  const out = new Map<string, Obj>();

  out.set(
    "a-sole-prop-2026",
    fixture(
      "a-sole-prop-2026",
      "SYNTHETIC: no real person, TIN, amount or filing. Modelled on the workbook worked example.",
      "Fictional sole proprietor with sales, a sales return, expenses, income-tax and payroll records and one employee. Q4 invoices are hypothetical, as in the workbook. The year is marked closed so the annual measures run. The credit note A-013 (a return) is acknowledged. Elections are not in the fixture: the cases pass the Forms context (tax option, deduction method).",
      [profile(PROFILE_A, "A: sole proprietor", STANDARD_PERIODS)],
      ["A-013"],
      standardYear(PROFILE_A, "A"),
    ),
  );

  out.set(
    "b-eight-percent-2026",
    fixture(
      "b-eight-percent-2026",
      "SYNTHETIC: second copy of fixture a's ledger, used with an 8% Forms context.",
      "The same ledger as fixture a under its own profile. The 8% election lives in the Forms tax profile, not in Books: the cases read /taxpayer/tax_option = eight_percent from the Forms context. Books measures are identical to fixture a; only the 1701-MS mapping changes (IV.19 instead of IV.5 and IV.6). Whether a 2551Q is required in an 8% year is Forms' decision.",
      [profile(PROFILE_B, "B: same ledger, 8% Forms context", STANDARD_PERIODS)],
      ["EL-013"],
      standardYear(PROFILE_B, "EL"),
    ),
  );

  const cEntries: Obj[] = [];
  const c1 = sale(PROFILE_C, "C-001", "2026-06-20", "40000.00");
  const c3 = sale(PROFILE_C, "C-003", "2026-07-15", "100000.00");
  const c6 = reversalOf(PROFILE_C, "C-006", c1, "2026-09-02");
  cEntries.push(
    c1,
    sale(PROFILE_C, "C-002", "2026-07-01", "120000.00"),
    c3,
    reversalOf(PROFILE_C, "C-004", c3, "2026-07-15"),
    sale(PROFILE_C, "C-005", "2026-08-20", "80000.00"),
    c6,
    creditNote(PROFILE_C, "C-007", "2026-09-15", "5000.00"),
  );
  const cProfile = profile(PROFILE_C, "C: reversals and a credit note", { "2026-Q2": "closed", "2026-Q3": "closed" });
  out.set(
    "c-reversal-credit-note-2026",
    fixture(
      "c-reversal-credit-note-2026",
      "SYNTHETIC: reversal and credit-note case.",
      "Q2 sale 40000.00 (C-001) is voided by a reversal dated in Q3 (C-006), which is a return event. Q3 sale 100000.00 (C-003) is voided in the same quarter (C-004), which nets to zero. A credit note of 5000.00 (C-007) is a return event. None is acknowledged, so Q3 is blocked.",
      [cProfile],
      [],
      cEntries,
    ),
  );
  out.set(
    "c-reversal-credit-note-acknowledged-2026",
    fixture(
      "c-reversal-credit-note-acknowledged-2026",
      "SYNTHETIC: the same ledger as c, with the two return events acknowledged.",
      "Acknowledging C-006 and C-007 regenerates the Q3 proposal: net sales 155000.00.",
      [cProfile],
      ["C-006", "C-007"],
      cEntries,
    ),
  );

  out.set(
    "d-spouse-scope-2026",
    fixture(
      "d-spouse-scope-2026",
      "SYNTHETIC: spouse-scope refusal case.",
      "One self-scoped sale and one spouse-scoped sale in Q3. Spouse lines are refused in MVP, so both forms block and produce no values.",
      [profile(PROFILE_D, "D: spouse-scoped sale", { "2026-Q3": "closed", "2026": "closed" })],
      [],
      [sale(PROFILE_D, "D-001", "2026-07-10", "100000.00"), sale(PROFILE_D, "D-002", "2026-08-10", "50000.00", "spouse")],
    ),
  );

  out.set(
    "e-second-profile-2026",
    fixture(
      "e-second-profile-2026",
      "SYNTHETIC: second business profile for isolation tests.",
      "Profile B. Its amounts are chosen so that any leak into profile A would be visible.",
      [profile(PROFILE_E, "E: second business profile", { "2026-Q3": "closed" })],
      [],
      [sale(PROFILE_E, "E-001", "2026-07-20", "999000.00"), expense(PROFILE_E, "E-002", "2026-07-25", "30000.00", "rent")],
    ),
  );

  out.set(
    "f-slice-a-2026",
    fixture(
      "f-slice-a-2026",
      "SYNTHETIC: design Slice A numbers (docs/06 DS-A).",
      "Sales 120000.00 (2026-07-01), 100000.00 (2026-07-15), 80000.00 (2026-08-20), rent 25000.00 (2026-08-05). The 100000.00 sale is voided by a reversal dated 2026-07-15. The unbalanced entry N1 is not in the fixture; the ledger test builds it in memory.",
      [profile(PROFILE_F, "F: Slice A", { "2026-Q3": "closed" })],
      [],
      [
        sale(PROFILE_F, "F-001", "2026-07-01", "120000.00"),
        sale(PROFILE_F, "F-002", "2026-07-15", "100000.00"),
        sale(PROFILE_F, "F-003", "2026-08-20", "80000.00"),
        expense(PROFILE_F, "F-004", "2026-08-05", "25000.00", "rent"),
        reversalOf(PROFILE_F, "F-005", sale(PROFILE_F, "F-002", "2026-07-15", "100000.00"), "2026-07-15"),
      ],
    ),
  );

  const gBase: Obj[] = [
    sale(PROFILE_G, "G-001", "2026-07-10", "200000.00"),
    sale(PROFILE_G, "G-002", "2026-08-12", "100000.00"),
    ptCert(PROFILE_G, "G-003", "2026-09-30", "3000.00", "WB082", "received", "2026-07-01", "2026-09-30"),
    ptCert(PROFILE_G, "G-004", "2026-09-28", "2000.00", "WB082", "pending", "2026-07-01", "2026-09-30"),
    ptCert(PROFILE_G, "G-005", "2026-09-29", "1500.00", "WB084", "received", "2026-07-01", "2026-09-30"),
  ];
  const gProfile = profile(PROFILE_G, "G: percentage-tax credit (DS-H)", { "2026-Q3": "closed" });
  out.set(
    "g-pt-withholding-2026",
    fixture(
      "g-pt-withholding-2026",
      "SYNTHETIC: journey worked example (docs/06 DS-H) with percentage-tax credits.",
      "Sales 300000.00. A received WB082 2307 of 3000.00 counts in the percentage-tax certificate measure. A pending WB082 (2000.00) and a received WB084 2306 (1500.00, final withholding) do not.",
      [gProfile],
      [],
      gBase,
    ),
  );
  out.set(
    "g-overpayment-2026",
    fixture(
      "g-overpayment-2026",
      "SYNTHETIC: overpayment variant of fixture g.",
      "A further received WB082 credit of 7000.00 makes the certificate measure 10000.00. Forms computes the payable; Books never clamps or nets it.",
      [gProfile],
      [],
      [...gBase, ptCert(PROFILE_G, "G-006", "2026-09-30", "7000.00", "WB082", "received", "2026-07-01", "2026-09-30")],
    ),
  );

  return out;
}

if (import.meta.main) {
  const fixtures = buildFixtures();
  for (const [name, body] of fixtures) {
    const file = join(SYNTHETIC_DIR, `${name}.json`);
    writeFileSync(file, prettyJson(body));
    console.log(`wrote ${file}`);
  }
}
