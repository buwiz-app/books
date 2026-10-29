import { describe, expect, test } from "bun:test";
import { nontaxableThirteenthCentavos } from "../src/bridge.ts";
import { HAND_MONEY, HAND_THIRTEENTH } from "../src/cases.ts";
import { manilaDay, parsePeriod, periodKeyOf } from "../src/dates.ts";
import { formatMoney, MoneyError, parseMoney } from "../src/money.ts";

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

  for (const c of HAND_MONEY) {
    test(`round trip ${c.id}`, () => {
      expect(parseMoney(c.text).toString()).toBe(c.value_centavos);
      expect(formatMoney(BigInt(c.value_centavos))).toBe(c.text);
    });
  }
});

describe("the 1601-C item 17 split (hand vectors)", () => {
  for (const c of HAND_THIRTEENTH) {
    test(c.id, () => {
      expect(nontaxableThirteenthCentavos(BigInt(c.paid_centavos), BigInt(c.paid_earlier_centavos), 9000000n).toString()).toBe(c.nontaxable_centavos);
    });
  }
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
});
