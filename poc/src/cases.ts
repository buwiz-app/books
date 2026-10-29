import type { FormCode, FormContext } from "./types.ts";

/**
 * The proposal cases. Each one is a fixture file, a form, a period and the Forms context the bridge reads. The
 * vector files in fixtures/vectors/ are written from these cases, and a test recomputes each one and compares.
 *
 * `expected_forms_computed` is DOCUMENTATION ONLY: what Buwiz Forms is expected to compute from these inputs with
 * its own rates and rules (for example the 2551Q tax at 3%). Books has no code that produces these values and no
 * test asserts them against Books code (ADR-0007).
 */
export interface CaseSpec {
  id: string;
  fixture: string;
  form: FormCode;
  period: string;
  form_context: FormContext;
  description: string;
  expected_forms_computed?: { values: Record<string, string>; basis: string };
}

const GRADUATED_OSD: FormContext = { "/taxpayer/tax_option": "graduated", "/taxpayer/deduction": "osd" };
const EIGHT_PERCENT: FormContext = { "/taxpayer/tax_option": "eight_percent" };
const FORMS_DOC = "Forms' output with Forms' own rate table and rules; recorded for reviewers, never asserted against Books code.";

export const CASES: CaseSpec[] = [
  {
    id: "a-2551q-2026-Q3",
    fixture: "a-sole-prop-2026",
    form: "2551Q",
    period: "2026-Q3",
    form_context: {},
    description:
      "Workbook worked example, quarter 3: gross sales 300000.00, returns 10000.00 (acknowledged), net sales 290000.00 to txtATCAmt1 (the mapping uses net_sales; open for tax review). Ready.",
    expected_forms_computed: {
      values: { txtATCDue1: "8700.00", "frm2551Qv2018:txt14": "8700.00", "frm2551Qv2018:txt19": "8700.00" },
      basis: `290000.00 at the PT010 3% rate. ${FORMS_DOC}`,
    },
  },
  {
    id: "a-1601c-2026-09",
    fixture: "a-sole-prop-2026",
    form: "1601C",
    period: "2026-09",
    form_context: {},
    description:
      "Workbook worked example, September: gross compensation 42000.00, de minimis 2000.00, employee contributions 2000.00 to item 19, employer share 1500.00 kept separate, tax withheld 2808.40 as recorded by payroll. Ready.",
    expected_forms_computed: {
      values: { "frm1601c:txtTax21": "4000.00", "frm1601c:txtTax22": "38000.00", "frm1601c:txtTax36": "2808.40" },
      basis: `Item 21 sums items 15-20; item 22 is 14 less 21; no penalties. ${FORMS_DOC}`,
    },
  },
  {
    id: "a-1701ms-2026",
    fixture: "a-sole-prop-2026",
    form: "1701MS",
    period: "2026",
    form_context: GRADUATED_OSD,
    description:
      "Workbook worked example, annual 1701-MS, Forms context graduated + OSD: gross sales 1200000.00 to IV.5 (/taxpayer/sales) and returns 10000.00 to IV.6; net sales measure 1190000.00. Ready.",
    expected_forms_computed: {
      values: {
        "frm1701MS:txtTaxableCompensationIncomeNo7a": "1190000.00",
        "frm1701MS:txtOptionalDeductionNo11a": "476000.00",
        "frm1701MS:txtTaxableIncomeBusinesNo14a": "714000.00",
        "frm1701MS:TotalTaxDueCompensationBTP": "85300.00",
      },
      basis: `OSD at 40% of net sales and the 2023 graduated table. ${FORMS_DOC}`,
    },
  },
  {
    id: "a-1701ms-2026-eight",
    fixture: "a-sole-prop-2026",
    form: "1701MS",
    period: "2026",
    form_context: EIGHT_PERCENT,
    description:
      "Same ledger, Forms context eight_percent: net sales 1190000.00 to IV.19 (/taxpayer/eight_sales); IV.5 and IV.6 are not selected. Ready.",
    expected_forms_computed: {
      values: {},
      basis: `Forms applies the 8% rate, the 250,000 reduction and the eligibility tests. ${FORMS_DOC}`,
    },
  },
  {
    id: "b-2551q-2026-Q3",
    fixture: "b-eight-percent-2026",
    form: "2551Q",
    period: "2026-Q3",
    form_context: {},
    description:
      "Same ledger under a profile whose Forms tax profile elected 8%. Books output equals fixture a's: whether a 2551Q is required in an 8% year (RMO 23-2018 vs buwiz-oss NIL PT010) is Forms' decision. Ready.",
  },
  {
    id: "b-1601c-2026-09",
    fixture: "b-eight-percent-2026",
    form: "1601C",
    period: "2026-09",
    form_context: {},
    description: "Payroll measures do not depend on any election. Ready, same figures as fixture a.",
  },
  {
    id: "b-1701ms-2026",
    fixture: "b-eight-percent-2026",
    form: "1701MS",
    period: "2026",
    form_context: EIGHT_PERCENT,
    description: "Forms context eight_percent: net sales 1190000.00 to IV.19. Ready.",
  },
  {
    id: "c-2551q-2026-Q2",
    fixture: "c-reversal-credit-note-2026",
    form: "2551Q",
    period: "2026-Q2",
    form_context: {},
    description: "Quarter 2 is unaffected by a reversal posted in quarter 3. Net sales 40000.00. Ready.",
  },
  {
    id: "c-2551q-2026-Q3",
    fixture: "c-reversal-credit-note-2026",
    form: "2551Q",
    period: "2026-Q3",
    form_context: {},
    description:
      "Quarter 3 with a same-quarter void (nets to zero), a later-period reversal and a credit note, none acknowledged. Blocked on GATE.RETURNS_ACK.",
  },
  {
    id: "c-ack-2551q-2026-Q3",
    fixture: "c-reversal-credit-note-acknowledged-2026",
    form: "2551Q",
    period: "2026-Q3",
    form_context: {},
    description: "Same ledger, return events acknowledged. Net sales 155000.00. Ready.",
  },
  {
    id: "d-2551q-2026-Q3",
    fixture: "d-spouse-scope-2026",
    form: "2551Q",
    period: "2026-Q3",
    form_context: {},
    description: "A spouse-scoped sales line is refused in MVP. Blocked, no values.",
  },
  {
    id: "d-1701ms-2026",
    fixture: "d-spouse-scope-2026",
    form: "1701MS",
    period: "2026",
    form_context: GRADUATED_OSD,
    description: "A spouse-scoped sales line is refused in MVP. Blocked, no values.",
  },
  {
    id: "e-2551q-2026-Q3",
    fixture: "e-second-profile-2026",
    form: "2551Q",
    period: "2026-Q3",
    form_context: {},
    description: "Second business profile (B). Net sales 999000.00. Ready. Used for isolation tests.",
  },
  {
    id: "f-2551q-2026-Q3",
    fixture: "f-slice-a-2026",
    form: "2551Q",
    period: "2026-Q3",
    form_context: {},
    description:
      "Design Slice A numbers (DS-A): sales 120000.00 and 100000.00, the 100000.00 sale voided by a reversal dated 2026-07-15, sale 80000.00. Net sales 200000.00. Ready.",
    expected_forms_computed: {
      values: { "frm2551Qv2018:txt14": "6000.00" },
      basis: `200000.00 at 3%; Forms also owns the due date (2026-10-25) and any overdue status. ${FORMS_DOC}`,
    },
  },
  {
    id: "g-2551q-2026-Q3",
    fixture: "g-pt-withholding-2026",
    form: "2551Q",
    period: "2026-Q3",
    form_context: {},
    description:
      "Journey worked example (DS-H): net sales 300000.00; received WB082 2307 of 3000.00 to item 15; a pending WB082 and a WB084 2306 are excluded. Ready.",
    expected_forms_computed: {
      values: { "frm2551Qv2018:txt14": "9000.00", "frm2551Qv2018:txt18": "3000.00", "frm2551Qv2018:txt19": "6000.00" },
      basis: `300000.00 at 3%, less the 3000.00 credit. ${FORMS_DOC}`,
    },
  },
  {
    id: "g-overpayment-2551q-2026-Q3",
    fixture: "g-overpayment-2026",
    form: "2551Q",
    period: "2026-Q3",
    form_context: {},
    description: "Same ledger with a further received WB082 2307 of 7000.00: item 15 is 10000.00. Ready.",
    expected_forms_computed: {
      values: { "frm2551Qv2018:txt19": "-1000.00" },
      basis: `9000.00 less 10000.00; an overpayment stays signed. ${FORMS_DOC}`,
    },
  },
];

/**
 * 1601-C item 17 split, written by hand (the one input-level tax rule in the bridge). Annual cap 90,000.00 per
 * employee. Not computed by the engine, so tests that compare against them are not circular.
 */
export const HAND_THIRTEENTH: Array<{ id: string; paid_centavos: string; paid_earlier_centavos: string; nontaxable_centavos: string }> = [
  { id: "13th-95000-earlier0", paid_centavos: "9500000", paid_earlier_centavos: "0", nontaxable_centavos: "9000000" },
  { id: "13th-80000-earlier0", paid_centavos: "8000000", paid_earlier_centavos: "0", nontaxable_centavos: "8000000" },
  { id: "13th-80000-earlier20000", paid_centavos: "8000000", paid_earlier_centavos: "2000000", nontaxable_centavos: "7000000" },
  { id: "13th-5000-earlier95000", paid_centavos: "500000", paid_earlier_centavos: "9500000", nontaxable_centavos: "0" },
];

/** Money text round trips, written by hand: decimal text in, integer centavos out, and back. */
export const HAND_MONEY: Array<{ id: string; text: string; value_centavos: string }> = [
  { id: "money-290000.00", text: "290000.00", value_centavos: "29000000" },
  { id: "money-2808.40", text: "2808.40", value_centavos: "280840" },
  { id: "money-minus-1.50", text: "-1.50", value_centavos: "-150" },
  { id: "money-0.01", text: "0.01", value_centavos: "1" },
  { id: "money-0.00", text: "0.00", value_centavos: "0" },
  { id: "money-3000000.00", text: "3000000.00", value_centavos: "300000000" },
];
