import { describe, expect, test } from "bun:test";
import { parseMoney } from "../src/money.ts";
import { data, fixture, proposal, sumText } from "./support.ts";

/*
 * Expected values come from the workbook worked example after the corrections in
 * research/spreadsheet-corrections.md (items 1-7 and 10). The checks below use the workbook's own cells:
 *   2551Q  C7 base 290,000; C9 tax 8,700; C17 4,000 (II.21 chain); C11 credits 0; C12 payable 8,700
 *   1601-C II.14 42,000; II.18 2,000; II.19 2,000; II.21 4,000; II.22 38,000; II.25 2,808.40; II.36 2,808.40
 *   1701-MS C29 net sales 1,190,000; C31 OSD 476,000; C33 tax 85,300; V.10 52,300; II.29 33,000
 * Fixture a reproduces them from ledger lines. The figures are synthetic.
 */

const input = fixture("a-sole-prop-2026");

describe("workbook worked example (fixture a)", () => {
  test("2551Q quarter 3: base 290000.00 (300000 sales less the acknowledged 10000 return), tax 8700.00, payable 8700.00", () => {
    const p = proposal(input, "2551Q", "2026-Q3");
    expect(p.status).toBe("ready");
    expect(p.fill_args["txtATCAmt1"]).toBe("290000.00");
    expect(p.fill_args["schedule_1.0.atc"]).toBe("PT010");
    expect(p.fill_args["frm2551Qv2018:txt15"]).toBe("0.00");
    expect(p.expected_readback["frm2551Qv2018:txt14"]).toBe("8700.00");
    expect(p.expected_readback["txtATCDue1"]).toBe("8700.00");
    expect(p.expected_readback["frm2551Qv2018:txt19"]).toBe("8700.00");
    expect(p.expected_readback["frm2551Qv2018:txt24"]).toBe("8700.00");
  });

  test("2551Q lineage sums to the base and lists the three sales and the credit note", () => {
    const p = proposal(input, "2551Q", "2026-Q3");
    const lines = (p.lineage["txtATCAmt1"] as { lines: Array<{ entry_id: string; amount: string }> }).lines;
    expect(lines.map((l) => l.entry_id).sort()).toEqual(["A-009", "A-010", "A-011", "A-013"]);
    expect(sumText(lines.map((l) => l.amount))).toBe("290000.00");
    expect(lines.find((l) => l.entry_id === "A-013")?.amount).toBe("-10000.00");
  });

  test("1601-C September: gross 42000.00, de minimis 2000.00, employee share 2000.00, taxable 38000.00", () => {
    const p = proposal(input, "1601C", "2026-09");
    expect(p.status).toBe("ready");
    expect(p.fill_args["frm1601c:txtTax14"]).toBe("42000.00");
    expect(p.fill_args["frm1601c:txtTax18"]).toBe("2000.00");
    expect(p.fill_args["frm1601c:txtTax19"]).toBe("2000.00");
    expect(p.fill_args["frm1601c:TaxWithheld"]).toBe("1");
    expect(p.fill_args["frm1601c:txtATC"]).toBe("WW010");
    expect(p.expected_readback["frm1601c:txtTax21"]).toBe("4000.00"); // workbook C17 chain
    expect(p.expected_readback["frm1601c:txtTax22"]).toBe("38000.00");
    expect(p.expected_readback["frm1601c:txtTax24"]).toBe("38000.00");
  });

  test("1601-C tax withheld 2808.40 comes from payroll; amount still due equals it", () => {
    const p = proposal(input, "1601C", "2026-09");
    expect(p.fill_args["frm1601c:txtTax25"]).toBe("2808.40");
    expect(p.expected_readback["frm1601c:txtTax27"]).toBe("2808.40");
    expect(p.expected_readback["frm1601c:txtTax36"]).toBe("2808.40");
  });

  test("1701-MS annual with OSD: sales 1200000.00, returns 10000.00, net 1190000.00, OSD 476000.00, taxable 714000.00, tax 85300.00", () => {
    const p = proposal(input, "1701MS", "2026");
    expect(p.status).toBe("ready");
    expect(p.fill_args["frm1701MS:txtSalesRevenueNo5a"]).toBe("1200000.00");
    expect(p.fill_args["frm1701MS:txtSalesReturnsNo6a"]).toBe("10000.00");
    expect(p.expected_readback["frm1701MS:txtTaxableCompensationIncomeNo7a"]).toBe("1190000.00");
    expect(p.expected_readback["frm1701MS:txtOptionalDeductionNo11a"]).toBe("476000.00");
    expect(p.expected_readback["frm1701MS:txtTaxableIncomeBusinesNo14a"]).toBe("714000.00");
    expect(p.expected_readback["frm1701MS:TotalTaxDueCompensationBTP"]).toBe("85300.00");
  });

  test("1701-MS credits 52300.00 (40400 + 8900 + 3000) and payable 33000.00", () => {
    const p = proposal(input, "1701MS", "2026");
    expect(p.expected_readback["frm1701MS:txtTotalTaxCreditsPaymentsPartVNo10a"]).toBe("52300.00");
    expect(p.expected_readback["frm1701MS:txtTotalTaxCreditsPartIINo24a"]).toBe("52300.00");
    expect(p.expected_readback["frm1701MS:txtTaxPayableNo25a"]).toBe("33000.00");
    expect(p.expected_readback["frm1701MS:txtTotalAmountPayableNo29a"]).toBe("33000.00");
    expect(p.reference_values["frm1701MS:txtTaxPaymentsNo2a"]).toBe("40400.00");
    expect(p.reference_values["frm1701MS:txtCreditableTaxNo3a"]).toBe("8900.00");
    expect(p.reference_values["frm1701MS:txtCreditableFourthQuarterNo4a"]).toBe("3000.00");
  });

  test("the 1701-MS prior-filing and certificate values are reference only, never fill arguments", () => {
    const p = proposal(input, "1701MS", "2026");
    for (const key of ["frm1701MS:txtTaxPaymentsNo2a", "frm1701MS:txtCreditableTaxNo3a", "frm1701MS:txtCreditableFourthQuarterNo4a"]) {
      expect(p.fill_args[key]).toBeUndefined();
      expect(p.reference_values[key]).toBeDefined();
    }
  });

  test("the 1601-C taxable amount is not reduced by the employer share (the BK-TS defect would give 40000.00)", () => {
    const p = proposal(input, "1601C", "2026-09");
    expect(parseMoney(p.expected_readback["frm1601c:txtTax22"]!)).toBe(parseMoney("38000.00"));
    expect(p.warnings.some((w) => w.code === "EXC.EMPLOYER_SHARE_NOT_19" && (w.ids ?? []).length === 2)).toBe(true);
  });

  test("the ledger reproduces the rules-file test vectors for 1601-C II.14 and II.22", () => {
    const rules = data.rules["1601C"].rules;
    const ii14 = rules.find((r) => r.id === "1601C.II.14")?.test_vectors?.[0] as { expected_text: string };
    const ii22 = rules.find((r) => r.id === "1601C.II.22")?.test_vectors?.[0] as { expected_text: string };
    const p = proposal(input, "1601C", "2026-09");
    expect(p.fill_args["frm1601c:txtTax14"]).toBe(ii14.expected_text);
    expect(p.expected_readback["frm1601c:txtTax22"]).toBe(ii22.expected_text);
  });
});
