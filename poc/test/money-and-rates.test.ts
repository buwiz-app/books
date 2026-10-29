import { describe, expect, test } from "bun:test";
import { HAND_GRADUATED, HAND_PT_VECTORS, HAND_THIRTEENTH, HAND_VAT } from "../src/cases.ts";
import { addDays, dueDate, manilaDay, parsePeriod, periodKeyOf } from "../src/dates.ts";
import { thirteenthMonthSplit } from "../src/forms.ts";
import { divRoundHalfAway, formatMoney, MoneyError, parseMoney, percentOf } from "../src/money.ts";
import { graduatedTaxCentavos, ptRateFor, vatThresholdCentavos } from "../src/rates.ts";
import { data } from "./support.ts";

describe("money is integer centavos and decimal text", () => {
  test("parses two-place decimal text only", () => {
    expect(parseMoney("200000.00")).toBe(20000000n);
    expect(parseMoney("-1.50")).toBe(-150n);
    expect(parseMoney("0.00")).toBe(0n);
    for (const bad of ["1.5", "1.499", "1,000.00", "1e3", "-0.00", " 1.00", "1.00 "]) {
      expect(() => parseMoney(bad)).toThrow(MoneyError);
    }
  });

  test("refuses JS numbers, so no float can enter a money path", () => {
    expect(() => parseMoney(1.5)).toThrow(/decimal text/);
    expect(() => parseMoney(200000)).toThrow(/decimal text/);
  });

  test("formats centavos back to two-place text", () => {
    expect(formatMoney(20000000n)).toBe("200000.00");
    expect(formatMoney(-5n)).toBe("-0.05");
    expect(formatMoney(0n)).toBe("0.00");
  });

  test("half away from zero, once: 1.50 at 3% is 0.05, 1.49 is 0.04, -1.50 is -0.05", () => {
    expect(formatMoney(percentOf(150n, 300))).toBe("0.05");
    expect(formatMoney(percentOf(149n, 300))).toBe("0.04");
    expect(formatMoney(percentOf(-150n, 300))).toBe("-0.05");
  });

  test("divRoundHalfAway rounds exact halves away from zero (banker's rounding would give 0.04)", () => {
    expect(divRoundHalfAway(45000n, 10000n)).toBe(5n);
    expect(divRoundHalfAway(-45000n, 10000n)).toBe(-5n);
    expect(divRoundHalfAway(44999n, 10000n)).toBe(4n);
  });

  for (const c of HAND_PT_VECTORS) {
    test(`percentage tax ${c.id}: ${c.note}`, () => {
      expect(percentOf(BigInt(c.base_centavos), c.rate_bps).toString()).toBe(String(c.expected_centavos));
    });
  }

  test("the rules files carry their own PT vectors; the engine reproduces them", () => {
    const vectors = data.rules["2551Q"].rules.find((r) => r.id === "2551Q.S1.1.tax")?.test_vectors as Array<{
      base_text: string;
      rate_bps: number;
      tax_text: string;
    }>;
    expect(vectors.length).toBe(4);
    for (const v of vectors) {
      const base = parseMoney(v.base_text);
      expect(formatMoney(percentOf(base, v.rate_bps))).toBe(v.tax_text);
    }
  });
});

describe("dates, periods and the Asia/Manila calendar day", () => {
  test("an instant is taken on the Asia/Manila calendar day (DS-D)", () => {
    expect(manilaDay("2026-09-30T23:30:00+08:00")).toBe("2026-09-30");
    expect(manilaDay("2026-10-01T00:30:00+08:00")).toBe("2026-10-01");
    // 20:00 UTC on 30 Sep is 04:00 on 1 Oct in Manila: the quarter is Q4, not Q3.
    expect(manilaDay("2026-09-30T20:00:00Z")).toBe("2026-10-01");
    expect(periodKeyOf(manilaDay("2026-09-30T20:00:00Z"), "quarter")).toBe("2026-Q4");
  });

  test("periods resolve to calendar ranges", () => {
    expect(parsePeriod("2026-Q3")).toMatchObject({ start: "2026-07-01", end: "2026-09-30" });
    expect(parsePeriod("2026-09")).toMatchObject({ start: "2026-09-01", end: "2026-09-30" });
    expect(parsePeriod("2026")).toMatchObject({ start: "2026-01-01", end: "2026-12-31" });
    expect(() => parsePeriod("2026-Q5")).toThrow();
  });

  test("due dates: 2551Q is 25 days after quarter end; 1601-C is the 10th of the next month; 1701-MS is 15 April", () => {
    expect(dueDate("2551Q", parsePeriod("2026-Q3"))).toMatchObject({ date: "2026-10-25", rule_status: "CONFIRMED" });
    expect(dueDate("1601C", parsePeriod("2026-09"))).toMatchObject({ date: "2026-10-10", rule_status: "UNCONFIRMED" });
    expect(dueDate("1601C", parsePeriod("2026-12"))).toMatchObject({ date: "2027-01-15" });
    expect(dueDate("1701MS", parsePeriod("2026"))).toMatchObject({ date: "2027-04-15", rule_status: "CONFIRMED" });
    expect(addDays("2026-09-30", 25)).toBe("2026-10-25");
  });
});

describe("percentage-tax rate rows (rates.v1)", () => {
  const lookups: Array<[string, number | null]> = [
    ["2020-06-30", null],
    ["2020-07-01", 100],
    ["2023-06-30", 100],
    ["2023-07-01", 300],
    ["2026-09-30", 300],
    ["2026-10-01", 300],
  ];
  for (const [date, bps] of lookups) {
    test(`rate for ${date} is ${bps === null ? "absent (fail closed)" : `${bps} bps`}`, () => {
      expect(ptRateFor(data.rates, date)?.row.rate_bps ?? null).toBe(bps);
    });
  }

  test("the disputed 1% extension is OFF and warned for 2023-07-01..2025-12-31 (decision D6)", () => {
    const warned = ptRateFor(data.rates, "2024-04-01");
    expect(warned?.row.rate_bps).toBe(300);
    expect(warned?.warnings.some((w) => w.startsWith("PT_RATE_DISPUTED_WINDOW"))).toBe(true);
    expect(ptRateFor(data.rates, "2026-07-01")?.warnings).toEqual([]);
  });

  test("the VAT threshold is 3000000.00 pesos (300000000 centavos)", () => {
    expect(vatThresholdCentavos(data.rates)).toBe(300000000n);
  });
});

describe("graduated tax (1701-MS IV.18B, 2023+ table) and the 13th-month split", () => {
  for (const c of HAND_GRADUATED) {
    test(`graduated ${c.id}`, () => {
      expect(graduatedTaxCentavos(data.rates, BigInt(c.base_centavos)).toString()).toBe(String(c.expected_centavos));
    });
  }

  test("714000.00 gives 85300.00 (rates.v1 test_vector and workbook C33)", () => {
    const vector = data.rates.graduated_income_tax_2023.test_vector;
    expect(formatMoney(graduatedTaxCentavos(data.rates, parseMoney(vector.input_text)))).toBe(vector.expected_tax_text);
  });

  for (const c of HAND_THIRTEENTH) {
    test(`13th-month split ${c.id}`, () => {
      const split = thirteenthMonthSplit(BigInt(c.amount_centavos), BigInt(c.ytd_exempt_centavos));
      expect(split.exempt.toString()).toBe(String(c.exempt_centavos));
      expect(split.taxable.toString()).toBe(String(c.taxable_centavos));
    });
  }

  test("the rules file vector (95000.00 gives 90000.00 exempt and 5000.00 excess) matches the split", () => {
    const vector = data.rules["1601C"].rules.find((r) => r.id === "1601C.II.17")?.test_vectors?.[0] as {
      expected_exempt_text: string;
      expected_taxable_excess_text: string;
    };
    const split = thirteenthMonthSplit(parseMoney("95000.00"));
    expect(formatMoney(split.exempt)).toBe(vector.expected_exempt_text);
    expect(formatMoney(split.taxable)).toBe(vector.expected_taxable_excess_text);
  });

  test("the hand VAT vectors are the strict > test (DS-B)", () => {
    const threshold = vatThresholdCentavos(data.rates);
    for (const c of HAND_VAT) {
      expect(BigInt(c.ytd_centavos) > threshold).toBe(c.breached);
    }
  });
});
