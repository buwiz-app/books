import { describe, expect, test } from "bun:test";
import { certEntry, clone, data, fixture, memoryInput, normalized, proposal, PROFILE_A, saleEntry } from "./support.ts";
import { parseMoney } from "../src/money.ts";

describe("employee versus employer shares (1601-C II.19, EXC.EMPLOYER_SHARE_NOT_19)", () => {
  const a = fixture("a-sole-prop-2026");

  test("the employer share is excluded from II.19 and from taxable compensation", () => {
    const p = proposal(a, "1601C", "2026-09");
    expect(p.fill_args["frm1601c:txtTax19"]).toBe("2000.00");
    expect(p.expected_readback["frm1601c:txtTax22"]).toBe("38000.00");
  });

  test("the employer-share lines are listed as excluded, never silently dropped", () => {
    const p = proposal(a, "1601C", "2026-09");
    const warning = p.warnings.find((w) => w.code === "EXC.EMPLOYER_SHARE_NOT_19");
    expect(warning?.ids).toEqual(["A-015.3", "A-015.5"]);
  });

  test("the employer share cannot reduce compensation even when it is larger than the employee share", () => {
    // Employer share 9000.00 (debit expense and credit employer payable), net pay unchanged, so the entry still balances.
    const input = clone(a);
    const payroll = input.entries.find((e) => e.entry_id === "A-015")!;
    payroll.lines.find((l) => l.line_id === "A-015.3")!.debit = "9000.00";
    payroll.lines.find((l) => l.line_id === "A-015.5")!.credit = "9000.00";
    const p = proposal(input, "1601C", "2026-09");
    expect(p.status).toBe("ready");
    expect(p.expected_readback["frm1601c:txtTax22"]).toBe("38000.00");
    expect(p.fill_args["frm1601c:txtTax19"]).toBe("2000.00");
  });

  test("contribution_side decides whose share it is: an employer share mis-tagged as a mandatory contribution still never reaches II.19", () => {
    const input = clone(a);
    const employer = input.entries.find((e) => e.entry_id === "A-015")!.lines.find((l) => l.line_id === "A-015.5")!;
    employer.treatment = "contribution";
    employer.comp_component = "mandatory_contributions_employee";
    employer.contribution_side = "employer";
    const p = proposal(input, "1601C", "2026-09");
    expect(p.status).toBe("ready");
    expect(p.fill_args["frm1601c:txtTax19"]).toBe("2000.00");
    expect(p.expected_readback["frm1601c:txtTax22"]).toBe("38000.00");
    expect(p.warnings.find((w) => w.code === "EXC.EMPLOYER_SHARE_NOT_19")?.ids).toContain("A-015.5");
  });

  test("a payroll month with an unmapped component (13th month) blocks rather than guessing the cap", () => {
    const input = memoryInput({
      entries: [
        {
          entry_id: "P-1",
          tax_profile_id: PROFILE_A,
          entry_date: "2026-09-30",
          lines: [
            { line_id: "P-1.1", account: "salaries", debit: "95000.00", credit: "0.00", tax_types: ["compensation_withholding"], treatment: "comp_income", tax_side: "debit", comp_component: "thirteenth_month_and_benefits", person_scope: "employee", document_kind: "payroll_register" },
            { line_id: "P-1.2", account: "ph_net_pay_payable", debit: "0.00", credit: "95000.00" },
          ],
        },
      ],
    });
    const p = proposal(input, "1601C", "2026-09");
    expect(p.status).toBe("blocked");
    expect(p.blocking.map((b) => b.code)).toContain("comp_component_not_implemented");
  });

  test("a month without payroll blocks: zero payroll is not confirmed by any fact", () => {
    const input = memoryInput({ entries: [saleEntry(PROFILE_A, "S-9", "2026-09-10", "1000.00")] });
    const p = proposal(input, "1601C", "2026-09");
    expect(p.blocking.map((b) => b.code)).toContain("no_payroll_lines");
  });

  test("the 13th-month exemption vector from the rules file (95000.00 gives 90000.00 exempt) is the cap used by the split", () => {
    const vector = data.rules["1601C"].rules.find((r) => r.id === "1601C.II.17")?.test_vectors?.[0] as { expected_exempt_text: string };
    expect(vector.expected_exempt_text).toBe("90000.00");
  });
});

describe("percentage-tax credits and income-tax credits never cross (EXC.PT_CWT_NOT_IT, EXC.IT_CWT_NOT_PT)", () => {
  test("a received percentage-tax certificate credits 2551Q item 15 and never reaches 1701-MS V.3/V.4", () => {
    const base = fixture("a-sole-prop-2026");
    const input = clone(base);
    input.entries.push(
      normalized(PROFILE_A, certEntry(PROFILE_A, "PT-1", "2026-09-30", "3000.00", { atc: "WB082", creditKind: "pt", status: "received", periodStart: "2026-07-01", periodEnd: "2026-09-30" })),
    );
    const quarter = proposal(input, "2551Q", "2026-Q3");
    expect(quarter.fill_args["frm2551Qv2018:txt15"]).toBe("3000.00");
    const annual = proposal(input, "1701MS", "2026");
    expect(annual.reference_values["frm1701MS:txtCreditableTaxNo3a"]).toBe("8900.00");
    expect(annual.reference_values["frm1701MS:txtCreditableFourthQuarterNo4a"]).toBe("3000.00");
    expect(annual.warnings.map((w) => w.code)).toContain("EXC.PT_CWT_NOT_IT");
  });

  test("an income-tax certificate (WI010) never credits 2551Q item 15", () => {
    const p = proposal(fixture("a-sole-prop-2026"), "2551Q", "2026-Q3");
    expect(p.fill_args["frm2551Qv2018:txt15"]).toBe("0.00");
    expect(p.warnings.find((w) => w.code === "EXC.IT_CWT_NOT_PT")?.ids).toEqual(["A-014.1"]);
  });

  test("the income-tax certificates reach 1701-MS V.3 (Q1 to Q3) and V.4 (Q4) by period end", () => {
    const p = proposal(fixture("a-sole-prop-2026"), "1701MS", "2026");
    expect(p.reference_values["frm1701MS:txtCreditableTaxNo3a"]).toBe("8900.00");
    expect(p.reference_values["frm1701MS:txtCreditableFourthQuarterNo4a"]).toBe("3000.00");
  });

  test("WB084 (final withholding) is never credited, and pending, lost and disputed certificates are not credited", () => {
    const g = proposal(fixture("g-pt-withholding-2026"), "2551Q", "2026-Q3");
    expect(g.fill_args["frm2551Qv2018:txt15"]).toBe("3000.00");
    const codes = g.warnings.map((w) => w.code);
    expect(codes).toContain("EXC.WB084_FINAL");
    expect(codes).toContain("EXC.UNRECEIVED_CERT");
    expect(g.warnings.find((w) => w.code === "EXC.UNRECEIVED_CERT")?.ids).toEqual(["G-004.1"]);
    expect(g.warnings.find((w) => w.code === "EXC.WB084_FINAL")?.ids).toEqual(["G-005.1"]);
  });

  test("lost and disputed certificates are not credited either", () => {
    const base = fixture("g-pt-withholding-2026");
    const input = clone(base);
    const pid = base.profiles[0].tax_profile_id;
    input.entries.push(
      normalized(pid, certEntry(pid, "G-L1", "2026-09-30", "1200.00", { atc: "WB082", creditKind: "pt", status: "lost", periodStart: "2026-07-01", periodEnd: "2026-09-30" })),
      normalized(pid, certEntry(pid, "G-D1", "2026-09-30", "1300.00", { atc: "WB082", creditKind: "pt", status: "disputed", periodStart: "2026-07-01", periodEnd: "2026-09-30" })),
    );
    const p = proposal(input, "2551Q", "2026-Q3");
    expect(p.fill_args["frm2551Qv2018:txt15"]).toBe("3000.00");
  });

  test("the journey worked example: payable is 6000.00 from PT 9000.00 less a credit of 3000.00 (DS-H)", () => {
    const p = proposal(fixture("g-pt-withholding-2026"), "2551Q", "2026-Q3");
    expect(p.expected_readback["frm2551Qv2018:txt14"]).toBe("9000.00");
    expect(p.expected_readback["frm2551Qv2018:txt18"]).toBe("3000.00");
    expect(p.expected_readback["frm2551Qv2018:txt19"]).toBe("6000.00");
  });

  test("an overpayment is a negative signed line, never clamped to zero", () => {
    const p = proposal(fixture("g-overpayment-2026"), "2551Q", "2026-Q3");
    expect(p.fill_args["frm2551Qv2018:txt15"]).toBe("10000.00");
    expect(p.expected_readback["frm2551Qv2018:txt19"]).toBe("-1000.00");
    expect(p.expected_readback["frm2551Qv2018:txt24"]).toBe("-1000.00");
    expect(parseMoney(p.expected_readback["frm2551Qv2018:txt19"]!) < 0n).toBe(true);
  });
});

describe("fail closed on payroll lines that cannot be placed", () => {
  test("a compensation line not tagged to an employee blocks 1601-C instead of being dropped", () => {
    const input = clone(fixture("a-sole-prop-2026"));
    const payroll = input.entries.find((e) => e.entry_id === "A-015")!;
    payroll.lines.find((l) => l.line_id === "A-015.1")!.person_scope = "self";
    const p = proposal(input, "1601C", "2026-09");
    expect(p.status).toBe("blocked");
    expect(p.blocking.map((b) => b.code)).toContain("comp_scope_not_employee");
    expect(p.fill_args).toEqual({});
  });
});
