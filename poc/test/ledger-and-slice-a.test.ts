import { describe, expect, test } from "bun:test";
import { postLedger, LedgerError } from "../src/ledger.ts";
import { normalizeEntry, normalizeFixture } from "../src/load.ts";
import { sumText, data, fixture, lineageOf, memoryInput, proposal, saleEntry, creditNoteEntry, reversalEntry, PROFILE_A } from "./support.ts";

/* Design Slice A (docs/04, acceptance A3-A5) on fixture f, which carries DS-A exactly. */

const f = fixture("f-slice-a-2026");

describe("Slice A ledger (A3, A4)", () => {
  test("A3: an entry with debit 100.00 and credit 99.99 is refused as unbalanced", () => {
    const bad = normalizeEntry(
      {
        entry_id: "N1",
        tax_profile_id: PROFILE_A,
        entry_date: "2026-07-20",
        lines: [
          { line_id: "N1.1", account: "bank_accounts", debit: "100.00", credit: "0.00" },
          { line_id: "N1.2", account: "sales_revenue", debit: "0.00", credit: "99.99", measure_side: "credit", income_class: "business_sales" },
        ],
      },
      new Set([PROFILE_A]),
      data.vocab,
    );
    expect(() => postLedger([bad])).toThrow(LedgerError);
    try {
      postLedger([bad]);
    } catch (error) {
      expect((error as LedgerError).code).toBe("unbalanced");
    }
  });

  test("A3: a line with both debit and credit is refused", () => {
    const bad = normalizeEntry(
      {
        entry_id: "N2",
        tax_profile_id: PROFILE_A,
        entry_date: "2026-07-20",
        lines: [
          { line_id: "N2.1", account: "bank_accounts", debit: "10.00", credit: "10.00" },
          { line_id: "N2.2", account: "sales_revenue", debit: "0.00", credit: "0.00" },
        ],
      },
      new Set([PROFILE_A]),
      data.vocab,
    );
    expect(() => postLedger([bad])).toThrow(/line_sides|exactly one/);
  });

  test("A4: voiding the 100000.00 sale posts a reversal dated the original entry date, and Q3 gross is 200000.00", () => {
    const { lines, reversedBy } = postLedger(f.entries);
    expect(reversedBy.get("F-002")).toBe("F-005");
    const reversal = f.entries.find((e) => e.entry_id === "F-005");
    expect(reversal?.entry_date).toBe("2026-07-15");
    const q3Sales = lines.filter((l) => l.income_class === "business_sales" && l.entry_date >= "2026-07-01" && l.entry_date <= "2026-09-30");
    const gross = q3Sales.reduce((total, l) => total + l.measure, 0n);
    expect(gross.toString()).toBe("20000000"); // 200000.00 in centavos
  });
});

describe("Slice A proposal (A5)", () => {
  test("A5: 2026-Q3 is ready; net sales 200000.00 to txtATCAmt1; no tax, due date or overdue status in Books", () => {
    const p = proposal(f, "2551Q", "2026-Q3");
    expect(p.status).toBe("ready");
    expect(p.measures["net_sales"]?.value).toBe("200000.00");
    expect(p.fill_args["txtATCAmt1"]).toBe("200000.00");
    for (const field of ["due", "rate", "expected_readback", "discrepancies"]) expect(field in p).toBe(false);
  });

  test("A5: lineage lists the 120000.00 and 80000.00 sales and not the voided pair", () => {
    const p = proposal(f, "2551Q", "2026-Q3");
    const lin = lineageOf(p, "txtATCAmt1");
    expect(lin.lines.map((l) => l.entry_id).sort()).toEqual(["F-001", "F-003"]);
    expect(sumText(lin.lines.map((l) => l.amount))).toBe("200000.00");
    expect(lin.netted_pairs).toBe(1);
  });

  test("the rent expense does not reach the 2551Q base or the output", () => {
    const p = proposal(f, "2551Q", "2026-Q3");
    expect(JSON.stringify(p)).not.toContain("F-004");
  });
});

describe("reversal rules: mirror, orphan and count-once", () => {
  const original = saleEntry(PROFILE_A, "R-1", "2026-07-10", "50000.00");

  test("a reversal must mirror its original line by line", () => {
    const wrong = reversalEntry("R-2", original, "2026-07-10");
    (wrong.lines as Array<Record<string, string>>)[1].debit = "49999.00";
    const input = memoryInput({ entries: [original, wrong] });
    expect(() => postLedger(input.entries)).toThrow(/reversal_not_mirror|unbalanced/);
  });

  test("an orphan reversal (its original is not in scope) is refused: no double counting by omission", () => {
    const orphan = reversalEntry("R-3", original, "2026-07-10");
    const input = memoryInput({ entries: [orphan] });
    expect(() => postLedger(input.entries)).toThrow(/orphan_reversal/);
  });

  test("a second reversal of the same entry is refused", () => {
    const first = reversalEntry("R-4", original, "2026-07-10");
    const second = reversalEntry("R-5", original, "2026-07-11");
    const input = memoryInput({ entries: [original, first, second] });
    expect(() => postLedger(input.entries)).toThrow(/reversed_twice/);
  });

  test("a reversal dated before its original is refused", () => {
    const early = reversalEntry("R-6", original, "2026-07-01");
    const input = memoryInput({ entries: [original, early] });
    expect(() => postLedger(input.entries)).toThrow(/reversal_before_original/);
  });
});

describe("a negative sales measure is not proposed, never clamped (DS-J)", () => {
  test("returns larger than sales in one quarter block with negative_measure", () => {
    const input = memoryInput({
      entries: [saleEntry(PROFILE_A, "S-1", "2026-07-10", "1000.00"), creditNoteEntry(PROFILE_A, "CN-1", "2026-07-20", "2000.00")],
      acks: ["CN-1"],
    });
    const p = proposal(input, "2551Q", "2026-Q3");
    expect(p.status).toBe("blocked");
    expect(p.blocking.map((b) => b.code)).toContain("negative_measure");
    expect(p.fill_args).toEqual({});
  });
});

describe("the input is validated before any proposal (privacy and shape)", () => {
  test("a money field given as a JS number is refused at normalization", () => {
    expect(() =>
      normalizeEntry(
        { entry_id: "X", tax_profile_id: PROFILE_A, entry_date: "2026-07-01", lines: [{ line_id: "X.1", account: "bank_accounts", debit: 100, credit: "0.00" }, { line_id: "X.2", account: "bank_accounts", debit: "0.00", credit: "100.00" }] },
        new Set([PROFILE_A]),
        data.vocab,
      ),
    ).toThrow(/decimal text/);
  });

  test("a compliance attribute on a line is refused: lines carry accounting facts only (ADR-0007)", () => {
    for (const banned of [{ atc: "PT010" }, { treatment: "sale_pt" }, { tax_types: ["percentage_tax"] }, { regime: "graduated" }]) {
      expect(() =>
        normalizeEntry(
          { entry_id: "X", tax_profile_id: PROFILE_A, entry_date: "2026-07-01", lines: [{ line_id: "X.1", account: "bank_accounts", debit: "1.00", credit: "0.00" }, { line_id: "X.2", account: "sales_revenue", debit: "0.00", credit: "1.00", measure_side: "credit", income_class: "business_sales", ...banned }] },
          new Set([PROFILE_A]),
          data.vocab,
        ),
      ).toThrow(/compliance attribute/);
    }
  });

  test("a profile carrying elections or tax facts is refused (elections are Forms')", () => {
    const raw = { fixture_format: "buwiz-poc-fixture/2", as_of: "2026-10-29", profiles: [{ tax_profile_id: PROFILE_A, display: "***-***-***-00000", facts: { "2026": { regime: "eight_percent" } } }], entries: [] };
    expect(() => normalizeFixture(raw, data.vocab)).toThrow(/Forms/);
  });

  test("a contra-sales line must say whether it is a return, an allowance or a discount", () => {
    const entry = creditNoteEntry(PROFILE_A, "CK-1", "2026-07-20", "10.00");
    delete ((entry.lines as Array<Record<string, unknown>>)[0]).contra_kind;
    expect(() => normalizeEntry(entry, new Set([PROFILE_A]), data.vocab)).toThrow(/contra_kind/);
  });
});
