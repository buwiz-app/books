import { describe, expect, test } from "bun:test";
import { certEntry, clone, data, fixture, memoryInput, normalized, proposal, PROFILE_A, saleEntry, thirteenthEntry } from "./support.ts";

describe("employee versus employer shares (1601-C item 19, EXC.EMPLOYER_SHARE_NOT_19)", () => {
  const a = fixture("a-sole-prop-2026");

  test("employer shares are a separate measure and never reach item 19", () => {
    const p = proposal(a, "1601C", "2026-09");
    expect(p.fill_args["frm1601c:txtTax19"]).toBe("2000.00");
    expect(p.measures["employer_mandatory_contributions"]).toBeUndefined(); // no 1601-C mapping reads it
    expect(p.warnings.find((w) => w.code === "EXC.EMPLOYER_SHARE_NOT_19")?.ids).toEqual(["A-015.3", "A-015.5"]);
  });

  test("a larger employer share changes neither gross compensation nor item 19", () => {
    const input = clone(a);
    const payroll = input.entries.find((e) => e.entry_id === "A-015")!;
    payroll.lines.find((l) => l.line_id === "A-015.3")!.debit = "9000.00";
    payroll.lines.find((l) => l.line_id === "A-015.5")!.credit = "9000.00";
    const p = proposal(input, "1601C", "2026-09");
    expect(p.status).toBe("ready");
    expect(p.fill_args["frm1601c:txtTax14"]).toBe("42000.00");
    expect(p.fill_args["frm1601c:txtTax19"]).toBe("2000.00");
  });

  test("contribution_side decides whose share it is: an employer line tagged with an employee scheme still never reaches item 19", () => {
    const input = clone(a);
    const employer = input.entries.find((e) => e.entry_id === "A-015")!.lines.find((l) => l.line_id === "A-015.5")!;
    employer.contribution_scheme = "philhealth";
    const p = proposal(input, "1601C", "2026-09");
    expect(p.fill_args["frm1601c:txtTax19"]).toBe("2000.00");
    expect(p.warnings.find((w) => w.code === "EXC.EMPLOYER_SHARE_NOT_19")?.ids).toContain("A-015.5");
  });

  test("tax withheld is the amount payroll recorded (a fact), not a computation", () => {
    const input = clone(a);
    const line = input.entries.find((e) => e.entry_id === "A-015")!.lines.find((l) => l.line_id === "A-015.6")!;
    line.credit = "2808.41";
    input.entries.find((e) => e.entry_id === "A-015")!.lines.find((l) => l.line_id === "A-015.7")!.credit = "37191.59";
    expect(proposal(input, "1601C", "2026-09").fill_args["frm1601c:txtTax25"]).toBe("2808.41");
  });
});

describe("1601-C item 17: the one input-level tax rule in the bridge (non-taxable 13th month, annual cap per employee)", () => {
  test("95000.00 paid in December with nothing earlier gives 90000.00 non-taxable; the gross stays in item 14", () => {
    const input = memoryInput({ entries: [thirteenthEntry(PROFILE_A, "T-1", "2026-12-15", "95000.00")] });
    const p = proposal(input, "1601C", "2026-12");
    expect(p.status).toBe("ready");
    expect(p.fill_args["frm1601c:txtTax17"]).toBe("90000.00");
    expect(p.fill_args["frm1601c:txtTax14"]).toBe("95000.00");
    expect(p.measures["thirteenth_month_and_benefits_paid"]?.value).toBe("95000.00");
    expect(p.warnings.some((w) => w.code === "INPUT_LEVEL_TAX_RULE")).toBe(true);
  });

  test("benefits paid earlier in the year use up the cap (year-to-date payroll only Books has)", () => {
    const input = memoryInput({
      periods: { "2026-12": "closed" },
      entries: [thirteenthEntry(PROFILE_A, "T-1", "2026-06-15", "20000.00"), thirteenthEntry(PROFILE_A, "T-2", "2026-12-15", "80000.00")],
    });
    const p = proposal(input, "1601C", "2026-12");
    expect(p.fill_args["frm1601c:txtTax17"]).toBe("70000.00");
    expect(p.measures["thirteenth_month_and_benefits_paid[ytd_before_period]"]?.value).toBe("20000.00");
  });

  test("the cap is per employee", () => {
    const input = memoryInput({
      periods: { "2026-12": "closed" },
      entries: [
        thirteenthEntry(PROFILE_A, "T-1", "2026-06-15", "90000.00", "EMP-1"),
        thirteenthEntry(PROFILE_A, "T-2", "2026-12-15", "10000.00", "EMP-1"),
        thirteenthEntry(PROFILE_A, "T-3", "2026-12-15", "10000.00", "EMP-2"),
      ],
    });
    expect(proposal(input, "1601C", "2026-12").fill_args["frm1601c:txtTax17"]).toBe("10000.00");
  });

  test("a 13th-month line without an employee reference blocks rather than guessing whose cap it uses", () => {
    const entry = thirteenthEntry(PROFILE_A, "T-1", "2026-12-15", "1000.00");
    delete (entry.lines as Array<Record<string, unknown>>)[0].party_ref;
    const p = proposal(memoryInput({ entries: [entry] }), "1601C", "2026-12");
    expect(p.blocking.map((b) => b.code)).toContain("employee_ref_missing");
  });

  test("the rules file labels it and carries the cap; the hand vector matches", () => {
    const m = data.rules["1601C"].mappings.find((x) => x.id === "1601C.II.17")!;
    expect(m.input_level_tax_rule).toContain("THE ONE INPUT-LEVEL TAX RULE IN THE BRIDGE");
    expect(m.aggregation_params?.annual_cap_text).toBe("90000.00");
    expect(m.test_vectors?.[0].expected_text).toBe("90000.00");
  });
});

describe("payroll lines that cannot be placed fail closed", () => {
  test("a month without payroll blocks: Books does not assert zero payroll", () => {
    const p = proposal(memoryInput({ entries: [saleEntry(PROFILE_A, "S-9", "2026-09-10", "1000.00")] }), "1601C", "2026-09");
    expect(p.blocking.map((b) => b.code)).toContain("no_payroll_lines");
  });

  test("a compensation line not tagged to an employee blocks instead of being dropped", () => {
    const input = clone(fixture("a-sole-prop-2026"));
    input.entries.find((e) => e.entry_id === "A-015")!.lines.find((l) => l.line_id === "A-015.1")!.person_scope = "self";
    const p = proposal(input, "1601C", "2026-09");
    expect(p.blocking.map((b) => b.code)).toContain("comp_scope_not_employee");
    expect(p.fill_args).toEqual({});
  });

  test("holiday/overtime pay blocks item 16 until the per-employee MWE split exists (P6)", () => {
    const input = clone(fixture("a-sole-prop-2026"));
    input.entries.find((e) => e.entry_id === "A-015")!.lines.find((l) => l.line_id === "A-015.2")!.comp_component = "holiday_ot_nsd_hazard";
    const p = proposal(input, "1601C", "2026-09");
    expect(p.blocking.map((b) => b.code)).toContain("mwe_split_not_implemented");
  });
});

describe("percentage-tax and income-tax certificates never cross (EXC.PT_CWT_NOT_IT, EXC.IT_CWT_NOT_PT)", () => {
  test("a received percentage-tax 2307 feeds 2551Q item 15 and never 1701-MS V.3/V.4", () => {
    const input = clone(fixture("a-sole-prop-2026"));
    input.entries.push(
      normalized(PROFILE_A, certEntry(PROFILE_A, "PT-1", "2026-09-30", "3000.00", { atc: "WB082", creditKind: "pt", status: "received", periodStart: "2026-07-01", periodEnd: "2026-09-30" })),
    );
    expect(proposal(input, "2551Q", "2026-Q3").fill_args["frm2551Qv2018:txt15"]).toBe("3000.00");
    const annual = proposal(input, "1701MS", "2026");
    expect(annual.reference_values["frm1701MS:txtCreditableTaxNo3a"]).toBe("8900.00");
    expect(annual.reference_values["frm1701MS:txtCreditableFourthQuarterNo4a"]).toBe("3000.00");
    expect(annual.warnings.find((w) => w.code === "EXC.PT_CWT_NOT_IT")?.ids).toEqual(["PT-1.1"]);
  });

  test("an income-tax 2307 never feeds 2551Q item 15", () => {
    const p = proposal(fixture("a-sole-prop-2026"), "2551Q", "2026-Q3");
    expect(p.fill_args["frm2551Qv2018:txt15"]).toBe("0.00");
    expect(p.warnings.find((w) => w.code === "EXC.IT_CWT_NOT_PT")?.ids).toEqual(["A-014.1"]);
  });

  test("a 2306 (final withholding, WB084) and a pending 2307 never count", () => {
    const g = proposal(fixture("g-pt-withholding-2026"), "2551Q", "2026-Q3");
    expect(g.fill_args["frm2551Qv2018:txt15"]).toBe("3000.00");
    expect(g.warnings.find((w) => w.code === "EXC.UNRECEIVED_CERT")?.ids).toEqual(["G-004.1"]);
    expect(g.warnings.find((w) => w.code === "EXC.WB084_FINAL")?.ids).toEqual(["G-005.1"]);
  });

  test("lost and disputed certificates do not count either", () => {
    const base = fixture("g-pt-withholding-2026");
    const input = clone(base);
    const pid = base.profiles[0].tax_profile_id;
    input.entries.push(
      normalized(pid, certEntry(pid, "G-L1", "2026-09-30", "1200.00", { atc: "WB082", creditKind: "pt", status: "lost", periodStart: "2026-07-01", periodEnd: "2026-09-30" })),
      normalized(pid, certEntry(pid, "G-D1", "2026-09-30", "1300.00", { atc: "WB082", creditKind: "pt", status: "disputed", periodStart: "2026-07-01", periodEnd: "2026-09-30" })),
    );
    expect(proposal(input, "2551Q", "2026-Q3").fill_args["frm2551Qv2018:txt15"]).toBe("3000.00");
  });

  test("certificates are windowed by their own period end, not the posting date", () => {
    const input = memoryInput({
      entries: [
        saleEntry(PROFILE_A, "S-1", "2026-07-10", "1000.00"),
        certEntry(PROFILE_A, "C-LATE", "2026-10-05", "30.00", { atc: "WB082", creditKind: "pt", status: "received", periodStart: "2026-07-01", periodEnd: "2026-09-30" }),
      ],
    });
    expect(proposal(input, "2551Q", "2026-Q3").fill_args["frm2551Qv2018:txt15"]).toBe("30.00");
  });

  test("an overpayment is Forms' to compute: Books proposes the certificate measure as recorded (10000.00)", () => {
    const p = proposal(fixture("g-overpayment-2026"), "2551Q", "2026-Q3");
    expect(p.fill_args["frm2551Qv2018:txt15"]).toBe("10000.00");
    expect("frm2551Qv2018:txt19" in p.fill_args).toBe(false);
  });
});
