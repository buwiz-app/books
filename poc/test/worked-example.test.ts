import { describe, expect, test } from "bun:test";
import { CASES } from "../src/cases.ts";
import { postLedger } from "../src/ledger.ts";
import { computeMeasure } from "../src/measures.ts";
import { formatMoney } from "../src/money.ts";
import { EIGHT_PERCENT, GRADUATED_OSD, data, fixture, lineageOf, proposal, sumText } from "./support.ts";

/*
 * The workbook worked example (research/spreadsheet-corrections.md, items 1-7 and 10), reproduced as ACCOUNTING
 * MEASURES and the fill arguments the bridge derives from them. Tax amounts (2551Q 8,700.00, 1701-MS 85,300.00, ...)
 * are Forms' output: they live only in the vectors' expected_forms_computed documentation and are never asserted
 * against Books code here (ADR-0007). The figures are synthetic.
 */

const input = fixture("a-sole-prop-2026");

describe("worked example, 2551Q quarter 3 (fixture a)", () => {
  const p = proposal(input, "2551Q", "2026-Q3");

  test("measures: gross sales 300000.00, returns 10000.00, net sales 290000.00", () => {
    expect(p.status).toBe("ready");
    expect(p.measures["gross_sales"]).toBeUndefined(); // the 2551Q mapping reads net_sales only
    expect(p.measures["net_sales"]?.value).toBe("290000.00");
    const full = proposal(input, "1701MS", "2026");
    expect(full.measures["gross_sales"]?.value).toBe("1200000.00");
  });

  test("the mapping uses net sales, so the txtATCAmt1 input is 290000.00; item 15 has no percentage-tax certificate", () => {
    expect(p.fill_args["txtATCAmt1"]).toBe("290000.00");
    expect(p.fill_args["frm2551Qv2018:txt15"]).toBe("0.00");
    expect(lineageOf(p, "txtATCAmt1").mapping_id).toBe("2551Q.S1.1.base");
    expect(lineageOf(p, "txtATCAmt1").measures).toEqual(["net_sales"]);
  });

  test("net-sales lineage sums to the value and lists the three sales and the return", () => {
    const lines = lineageOf(p, "txtATCAmt1").lines;
    expect(lines.map((l) => l.entry_id).sort()).toEqual(["A-009", "A-010", "A-011", "A-013"]);
    expect(sumText(lines.map((l) => l.amount))).toBe("290000.00");
    expect(lines.find((l) => l.entry_id === "A-013")?.amount).toBe("-10000.00");
  });

  test("the returns measure is separate and positive: 10000.00 in quarter 3", () => {
    const q3 = proposal(input, "1701MS", "2026");
    expect(q3.measures["sales_returns"]?.value).toBe("10000.00");
    expect(q3.measures["sales_allowances"]?.value).toBe("0.00");
    expect(q3.measures["sales_discounts"]?.value).toBe("0.00");
  });

  test("no ATC, rate, tax, total or penalty key is proposed: Forms chooses the ATC and computes the tax", () => {
    for (const key of ["schedule_1.0.atc", "drpATC1", "txtATCRate1", "txtATCDue1", "txtTotalSched1", "frm2551Qv2018:txt14", "frm2551Qv2018:txt19", "frm2551Qv2018:txt24", "frm2551Qv2018:taxRate"]) {
      expect(key in p.fill_args, key).toBe(false);
    }
    expect(p.refused_keys.some((r) => r.key === "schedule_1.0.atc" && r.reason.includes("Forms decides"))).toBe(true);
  });

  test("Forms' tax for this case is documentation in the vector, not a Books output", () => {
    const spec = CASES.find((c) => c.id === "a-2551q-2026-Q3")!;
    expect(spec.expected_forms_computed?.values["frm2551Qv2018:txt14"]).toBe("8700.00");
    expect(JSON.stringify(p)).not.toContain("8700.00");
  });
});

describe("worked example, 1601-C September (fixture a)", () => {
  const p = proposal(input, "1601C", "2026-09");

  test("measures: gross compensation 42000.00, de minimis 2000.00, employee contributions 2000.00, employer share 1500.00 kept apart", () => {
    expect(p.status).toBe("ready");
    expect(p.measures["gross_compensation"]?.value).toBe("42000.00");
    expect(p.measures["de_minimis_paid"]?.value).toBe("2000.00");
    expect(p.measures["employee_mandatory_contributions"]?.value).toBe("2000.00");
    expect(p.measures["employee_mandatory_contributions"]?.breakdown).toEqual({ sss_gsis: "2000.00" });
    expect(p.measures["tax_withheld_from_employees"]?.value).toBe("2808.40");
  });

  test("fill args: item 14 42000.00, item 18 2000.00, item 19 2000.00 (frm1601c:txtTax19), item 25 2808.40", () => {
    expect(p.fill_args["frm1601c:txtTax14"]).toBe("42000.00");
    expect(p.fill_args["frm1601c:txtTax18"]).toBe("2000.00");
    expect(p.fill_args["frm1601c:txtTax19"]).toBe("2000.00");
    expect(p.fill_args["frm1601c:txtTax25"]).toBe("2808.40");
    expect(p.fill_args["frm1601c:TaxWithheld"]).toBe("1");
  });

  test("employer shares never reach item 19, and are listed as excluded", () => {
    expect(lineageOf(p, "frm1601c:txtTax19").lines.map((l) => l.line_id)).toEqual(["A-015.4"]);
    expect(p.warnings.find((w) => w.code === "EXC.EMPLOYER_SHARE_NOT_19")?.ids).toEqual(["A-015.3", "A-015.5"]);
  });

  test("no ATC, computed total or penalty key is proposed", () => {
    for (const key of ["frm1601c:txtATC", "frm1601c:txtTax21", "frm1601c:txtTax22", "frm1601c:txtTax24", "frm1601c:txtTax36"]) {
      expect(key in p.fill_args, key).toBe(false);
    }
  });

  test("the mapping vectors in the rules file agree with the measures", () => {
    const ii14 = data.rules["1601C"].mappings.find((m) => m.id === "1601C.II.14")!.test_vectors![0];
    expect(p.fill_args["frm1601c:txtTax14"]).toBe(ii14.expected_text);
  });
});

describe("worked example, 1701-MS annual: the Forms-provided tax option picks the box (fixture a)", () => {
  test("net sales measure for the year is 1190000.00 (gross 1200000.00 less returns 10000.00), whatever the option", () => {
    const ledger = postLedger(input.entries);
    const year = { start: "2026-01-01", end: "2026-12-31" };
    expect(formatMoney(computeMeasure("net_sales", ledger, year).value)).toBe("1190000.00");
    const graduated = proposal(input, "1701MS", "2026", GRADUATED_OSD);
    expect(graduated.measures["gross_sales"]?.value).toBe("1200000.00");
    expect(graduated.measures["sales_returns"]?.value).toBe("10000.00");
    expect(proposal(input, "1701MS", "2026", EIGHT_PERCENT).measures["net_sales"]?.value).toBe("1190000.00");
  });

  test("graduated (read from Forms): sales go to /taxpayer/sales (IV.5) and the return to IV.6; IV.19 is not proposed", () => {
    const p = proposal(input, "1701MS", "2026", GRADUATED_OSD);
    expect(p.status).toBe("ready");
    expect(p.fill_args["frm1701MS:txtSalesRevenueNo5a"]).toBe("1200000.00");
    expect(p.fill_args["frm1701MS:txtSalesReturnsNo6a"]).toBe("10000.00");
    expect(sumText([p.fill_args["frm1701MS:txtSalesRevenueNo5a"]!, `-${p.fill_args["frm1701MS:txtSalesReturnsNo6a"]}`])).toBe("1190000.00");
    expect("frm1701MS:txtSalesRevenuesFeesANo19a" in p.fill_args).toBe(false);
    expect(data.crosswalk["1701MS"].byFillKey.get("frm1701MS:txtSalesRevenueNo5a")!.draft_paths).toEqual(["/taxpayer/sales"]);
  });

  test("eight_percent (read from Forms): net sales 1190000.00 go to /taxpayer/eight_sales (IV.19); IV.5 and IV.6 are not proposed", () => {
    const p = proposal(input, "1701MS", "2026", EIGHT_PERCENT);
    expect(p.status).toBe("ready");
    expect(p.fill_args["frm1701MS:txtSalesRevenuesFeesANo19a"]).toBe("1190000.00");
    expect("frm1701MS:txtSalesRevenueNo5a" in p.fill_args).toBe(false);
    expect("frm1701MS:txtSalesReturnsNo6a" in p.fill_args).toBe(false);
    expect(data.crosswalk["1701MS"].byFillKey.get("frm1701MS:txtSalesRevenuesFeesANo19a")!.draft_paths).toEqual(["/taxpayer/eight_sales"]);
    expect(p.excluded_keys.find((e) => e.key === "frm1701MS:txtSalesRevenueNo5a")?.reason).toContain("/taxpayer/tax_option is eight_percent");
  });

  test("the OSD choice is read too: under OSD cost of sales is not proposed; under itemized it would be", () => {
    const osd = proposal(input, "1701MS", "2026", GRADUATED_OSD);
    expect("frm1701MS:txtCostofSalesNo8a" in osd.fill_args).toBe(false);
    expect(osd.excluded_keys.find((e) => e.key === "frm1701MS:txtCostofSalesNo8a")?.reason).toContain("/taxpayer/deduction is osd");
  });

  test("the income-tax certificates are reference values (V.3 8900.00, V.4 3000.00), never fill arguments", () => {
    const p = proposal(input, "1701MS", "2026", GRADUATED_OSD);
    expect(p.reference_values["frm1701MS:txtCreditableTaxNo3a"]).toBe("8900.00");
    expect(p.reference_values["frm1701MS:txtCreditableFourthQuarterNo4a"]).toBe("3000.00");
    expect("frm1701MS:txtCreditableTaxNo3a" in p.fill_args).toBe(false);
  });

  test("no OSD, taxable income, tax due, credit total or payable key is proposed", () => {
    const p = proposal(input, "1701MS", "2026", GRADUATED_OSD);
    for (const key of [
      "frm1701MS:txtTaxableCompensationIncomeNo7a",
      "frm1701MS:txtOptionalDeductionNo11a",
      "frm1701MS:txtTaxableIncomeBusinesNo14a",
      "frm1701MS:TotalTaxDueCompensationBTP",
      "frm1701MS:txtTaxPayableNo25a",
      "frm1701MS:txtTotalAmountPayableNo29a",
      "frm1701MS:txtTaxpayerNo12b",
      "frm1701MS:txtTaxpayerNo17",
      "taxCodeDropdown",
    ]) {
      expect(key in p.fill_args, key).toBe(false);
    }
  });
});
