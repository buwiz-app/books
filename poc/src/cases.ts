import type { FormCode } from "./types.ts";

/**
 * The proposal cases. Each one is a fixture file, a form and a period. The vector files in
 * fixtures/vectors/ are written from these cases, and a test recomputes each one and compares.
 */
export interface CaseSpec {
  id: string;
  fixture: string;
  form: FormCode;
  period: string;
  description: string;
}

export const CASES: CaseSpec[] = [
  {
    id: "a-2551q-2026-Q3",
    fixture: "a-sole-prop-2026",
    form: "2551Q",
    period: "2026-Q3",
    description:
      "Workbook worked example, quarter 3: sales 300000.00, eligible return 10000.00 (acknowledged), base 290000.00, PT010 3% gives 8700.00. Ready.",
  },
  {
    id: "a-1601c-2026-09",
    fixture: "a-sole-prop-2026",
    form: "1601C",
    period: "2026-09",
    description:
      "Workbook worked example, September: gross compensation 42000.00, de minimis 2000.00, employee share 2000.00, employer share 1500.00 excluded; taxable 38000.00; tax withheld 2808.40. Ready.",
  },
  {
    id: "a-1701ms-2026",
    fixture: "a-sole-prop-2026",
    form: "1701MS",
    period: "2026",
    description:
      "Workbook worked example, annual 1701-MS with OSD: net sales 1190000.00, OSD 476000.00, taxable 714000.00, tax 85300.00, credits 52300.00, payable 33000.00. Ready.",
  },
  {
    id: "b-2551q-2026-Q3",
    fixture: "b-eight-percent-2026",
    form: "2551Q",
    period: "2026-Q3",
    description: "8%-elector year: no PT010 proposal; a needs-review discrepancy citing the RMO 23-2018 vs buwiz-oss conflict. Blocked.",
  },
  {
    id: "b-1601c-2026-09",
    fixture: "b-eight-percent-2026",
    form: "1601C",
    period: "2026-09",
    description: "Payroll is independent of the 8% election. Ready, same figures as the workbook worked example.",
  },
  {
    id: "b-1701ms-2026",
    fixture: "b-eight-percent-2026",
    form: "1701MS",
    period: "2026",
    description: "8% pathway is not implemented in the PoC; fails closed. Blocked.",
  },
  {
    id: "c-2551q-2026-Q2",
    fixture: "c-reversal-credit-note-2026",
    form: "2551Q",
    period: "2026-Q2",
    description: "Quarter 2 is unaffected by a reversal posted in quarter 3. Base 40000.00, PT 1200.00. Ready.",
  },
  {
    id: "c-2551q-2026-Q3",
    fixture: "c-reversal-credit-note-2026",
    form: "2551Q",
    period: "2026-Q3",
    description:
      "Quarter 3 with a same-quarter void (nets to zero), a later-period reversal and a credit note, none acknowledged. Blocked on GATE.RETURNS_ACK.",
  },
  {
    id: "c-ack-2551q-2026-Q3",
    fixture: "c-reversal-credit-note-acknowledged-2026",
    form: "2551Q",
    period: "2026-Q3",
    description: "Same ledger, returns acknowledged. Base 155000.00, PT 4650.00. Ready.",
  },
  {
    id: "d-2551q-2026-Q3",
    fixture: "d-spouse-scope-2026",
    form: "2551Q",
    period: "2026-Q3",
    description: "A spouse-scoped sales line is refused in MVP. Blocked, no values.",
  },
  {
    id: "d-1701ms-2026",
    fixture: "d-spouse-scope-2026",
    form: "1701MS",
    period: "2026",
    description: "A spouse-scoped sales line is refused in MVP. Blocked, no values.",
  },
  {
    id: "e-2551q-2026-Q3",
    fixture: "e-second-profile-2026",
    form: "2551Q",
    period: "2026-Q3",
    description: "Second business profile (B). Base 999000.00, PT 29970.00. Ready. Used for isolation tests.",
  },
  {
    id: "f-2551q-2026-Q3",
    fixture: "f-slice-a-2026",
    form: "2551Q",
    period: "2026-Q3",
    description:
      "Design Slice A numbers (DS-A): sales 120000.00 and 100000.00, the 100000.00 sale voided by a reversal dated 2026-07-15, sale 80000.00. Base 200000.00, PT 6000.00, due 2026-10-25, overdue as of 2026-10-29. Ready.",
  },
  {
    id: "g-2551q-2026-Q3",
    fixture: "g-pt-withholding-2026",
    form: "2551Q",
    period: "2026-Q3",
    description:
      "Journey worked example with a percentage-tax credit (DS-H): sales 300000.00, PT 9000.00, received WB082 credit 3000.00; a pending WB082 and a received WB084 are not credited. Payable 6000.00. Ready.",
  },
  {
    id: "g-overpayment-2551q-2026-Q3",
    fixture: "g-overpayment-2026",
    form: "2551Q",
    period: "2026-Q3",
    description: "Same ledger with a further received WB082 credit of 7000.00: total credit 10000.00 against PT 9000.00. Overpayment -1000.00, signed and not clamped. Ready.",
  },
];

/**
 * Expected values written by hand from docs/06 DS-C and data/rules/rates.v1.json. They are not computed by the
 * engine, so tests that compare against them are not circular.
 */
export const HAND_PT_VECTORS: Array<{ id: string; base_centavos: string; rate_bps: number; expected_centavos: string; note: string }> = [
  { id: "pt-150-300", base_centavos: "150", rate_bps: 300, expected_centavos: "5", note: "1.50 at 3% is 0.045, which rounds half away to 0.05 (not banker's 0.04)" },
  { id: "pt-149-300", base_centavos: "149", rate_bps: 300, expected_centavos: "4", note: "1.49 at 3% is 0.0447, which rounds to 0.04" },
  { id: "pt-minus150-300", base_centavos: "-150", rate_bps: 300, expected_centavos: "-5", note: "-1.50 at 3% is -0.05" },
  { id: "pt-1-300", base_centavos: "1", rate_bps: 300, expected_centavos: "0", note: "0.01 at 3% is 0.0003, which rounds to 0.00" },
  { id: "pt-50-300", base_centavos: "50", rate_bps: 300, expected_centavos: "2", note: "0.50 at 3% is 0.015, which rounds to 0.02" },
  { id: "pt-49-300", base_centavos: "49", rate_bps: 300, expected_centavos: "1", note: "0.49 at 3% is 0.0147, which rounds to 0.01" },
  { id: "pt-minus50-300", base_centavos: "-50", rate_bps: 300, expected_centavos: "-2", note: "-0.50 at 3% is -0.015, which rounds half away to -0.02" },
  { id: "pt-3000000.00-300", base_centavos: "300000000", rate_bps: 300, expected_centavos: "9000000", note: "3000000.00 at 3% is 90000.00" },
  { id: "pt-150-100", base_centavos: "150", rate_bps: 100, expected_centavos: "2", note: "1.50 at 1% is 0.015, which rounds to 0.02" },
  { id: "pt-149-100", base_centavos: "149", rate_bps: 100, expected_centavos: "1", note: "1.49 at 1% is 0.0149, which rounds to 0.01" },
  { id: "pt-minus150-100", base_centavos: "-150", rate_bps: 100, expected_centavos: "-2", note: "-1.50 at 1% is -0.015, which rounds to -0.02" },
  { id: "pt-1-100", base_centavos: "1", rate_bps: 100, expected_centavos: "0", note: "0.01 at 1% is 0.0001, which rounds to 0.00" },
  { id: "pt-50-100", base_centavos: "50", rate_bps: 100, expected_centavos: "1", note: "0.50 at 1% is 0.005, which rounds half away to 0.01" },
  { id: "pt-49-100", base_centavos: "49", rate_bps: 100, expected_centavos: "0", note: "0.49 at 1% is 0.0049, which rounds to 0.00" },
  { id: "pt-minus50-100", base_centavos: "-50", rate_bps: 100, expected_centavos: "-1", note: "-0.50 at 1% is -0.005, which rounds to -0.01" },
  { id: "pt-3000000.00-100", base_centavos: "300000000", rate_bps: 100, expected_centavos: "3000000", note: "3000000.00 at 1% is 30000.00" },
  { id: "pt-200000.00-300", base_centavos: "20000000", rate_bps: 300, expected_centavos: "600000", note: "200000.00 at 3% is 6000.00 (Slice A)" },
  { id: "pt-290000.00-300", base_centavos: "29000000", rate_bps: 300, expected_centavos: "870000", note: "290000.00 at 3% is 8700.00 (workbook worked example)" },
];

/** Graduated tax (1701-MS IV.18B) and 13th-month split (1601-C II.17) expectations, written by hand. */
export const HAND_GRADUATED: Array<{ id: string; base_centavos: string; expected_centavos: string }> = [
  { id: "grad-0", base_centavos: "0", expected_centavos: "0" },
  { id: "grad-250000.00", base_centavos: "25000000", expected_centavos: "0" },
  { id: "grad-250000.01", base_centavos: "25000001", expected_centavos: "0" },
  { id: "grad-400000.00", base_centavos: "40000000", expected_centavos: "2250000" },
  { id: "grad-714000.00", base_centavos: "71400000", expected_centavos: "8530000" },
  { id: "grad-800000.00", base_centavos: "80000000", expected_centavos: "10250000" },
  { id: "grad-2000000.00", base_centavos: "200000000", expected_centavos: "40250000" },
  { id: "grad-8000000.00", base_centavos: "800000000", expected_centavos: "220250000" },
  { id: "grad-8000000.01", base_centavos: "800000001", expected_centavos: "220250000" },
];

export const HAND_THIRTEENTH: Array<{ id: string; amount_centavos: string; ytd_exempt_centavos: string; exempt_centavos: string; taxable_centavos: string }> = [
  { id: "13th-95000-ytd0", amount_centavos: "9500000", ytd_exempt_centavos: "0", exempt_centavos: "9000000", taxable_centavos: "500000" },
  { id: "13th-80000-ytd0", amount_centavos: "8000000", ytd_exempt_centavos: "0", exempt_centavos: "8000000", taxable_centavos: "0" },
  { id: "13th-80000-ytd20000", amount_centavos: "8000000", ytd_exempt_centavos: "2000000", exempt_centavos: "7000000", taxable_centavos: "1000000" },
];

/** VAT gate (rates.v1 vat_threshold, strictly greater). */
export const HAND_VAT: Array<{ id: string; ytd_centavos: string; breached: boolean }> = [
  { id: "vat-2999999.99", ytd_centavos: "299999999", breached: false },
  { id: "vat-3000000.00", ytd_centavos: "300000000", breached: false },
  { id: "vat-3000000.01", ytd_centavos: "300000001", breached: true },
];
