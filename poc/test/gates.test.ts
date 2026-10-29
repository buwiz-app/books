import { describe, expect, test } from "bun:test";
import { HAND_VAT } from "../src/cases.ts";
import { formatMoney } from "../src/money.ts";
import { data, fixture, memoryInput, proposal, saleEntry, reversalEntry, creditNoteEntry, PROFILE_A, clone } from "./support.ts";

describe("returns, credit notes and reversals across periods (fixture c)", () => {
  const c = fixture("c-reversal-credit-note-2026");
  const cAck = fixture("c-reversal-credit-note-acknowledged-2026");

  test("Q3 is blocked until the two return events are acknowledged: a later-period reversal and a credit note", () => {
    const p = proposal(c, "2551Q", "2026-Q3");
    expect(p.status).toBe("blocked");
    const block = p.blocking.find((b) => b.code === "GATE.RETURNS_ACK");
    expect(block?.ids).toEqual(["C-006", "C-007"]);
    expect(p.fill_args).toEqual({});
    expect(p.expected_readback).toEqual({});
  });

  test("acknowledging the return events regenerates Q3: base 155000.00 and PT 4650.00", () => {
    const p = proposal(cAck, "2551Q", "2026-Q3");
    expect(p.status).toBe("ready");
    expect(p.fill_args["txtATCAmt1"]).toBe("155000.00");
    expect(p.expected_readback["frm2551Qv2018:txt14"]).toBe("4650.00");
  });

  test("the same-quarter void (C-003 and C-004) nets to zero: it is not a return event and is left out of lineage", () => {
    const p = proposal(cAck, "2551Q", "2026-Q3");
    const lines = (p.lineage["txtATCAmt1"] as { lines: Array<{ entry_id: string; amount: string }>; netted_pairs: number });
    const entries = lines.lines.map((l) => l.entry_id).sort();
    expect(entries).toEqual(["C-002", "C-005", "C-006", "C-007"]);
    expect(lines.netted_pairs).toBe(1);
  });

  test("the later-period reversal counts once, as a negative sales line in Q3 (not in Q2)", () => {
    const p = proposal(cAck, "2551Q", "2026-Q3");
    const lines = (p.lineage["txtATCAmt1"] as { lines: Array<{ entry_id: string; amount: string }> }).lines;
    expect(lines.find((l) => l.entry_id === "C-006")?.amount).toBe("-40000.00");
    expect(lines.find((l) => l.entry_id === "C-007")?.amount).toBe("-5000.00");
  });

  test("Q2 is unaffected by the Q3 reversal: base 40000.00, PT 1200.00 (no double counting across periods)", () => {
    const p = proposal(c, "2551Q", "2026-Q2");
    expect(p.status).toBe("ready");
    expect(p.fill_args["txtATCAmt1"]).toBe("40000.00");
    expect(p.expected_readback["frm2551Qv2018:txt14"]).toBe("1200.00");
  });

  test("the credit-note treatment warning is raised (UNCONFIRMED, bir.md PT-8)", () => {
    expect(proposal(cAck, "2551Q", "2026-Q3").warnings.map((w) => w.code)).toContain("CREDIT_NOTE_TREATMENT_UNCONFIRMED");
  });
});

describe("eight-percent election (fixture b and in-memory A6)", () => {
  const b = fixture("b-eight-percent-2026");

  test("an 8% year produces no 2551Q values and no PT010 anywhere in the output", () => {
    const p = proposal(b, "2551Q", "2026-Q3");
    expect(p.status).toBe("blocked");
    expect(p.blocking.map((x) => x.code)).toContain("GATE.8PCT_YEAR_2551Q");
    expect(p.fill_args).toEqual({});
    expect(p.expected_readback).toEqual({});
    expect(p.reference_values).toEqual({});
    expect(JSON.stringify({ fill: p.fill_args, read: p.expected_readback, ref: p.reference_values })).not.toContain("PT010");
  });

  test("the needs-review discrepancy cites the RMO 23-2018 vs buwiz-oss conflict", () => {
    const p = proposal(b, "2551Q", "2026-Q3");
    const d = p.discrepancies.find((x) => x.id === "DISC-8PCT-2551Q");
    expect(d?.status).toBe("needs_review");
    expect(d?.summary).toContain("RMO 23-2018");
    expect(d?.summary).toContain("buwiz-oss");
    expect(d?.citations.join(" ")).toContain("bir.md section 8-6");
    expect(d?.citations.join(" ")).toContain("verify.md section 2 items 73 and 84");
  });

  test("the 1601-C payroll is unaffected by the 8% election and still produces its values", () => {
    const p = proposal(b, "1601C", "2026-09");
    expect(p.status).toBe("ready");
    expect(p.fill_args["frm1601c:txtTax14"]).toBe("42000.00");
  });

  test("the 1701-MS 8% pathway is not implemented and fails closed", () => {
    const p = proposal(b, "1701MS", "2026");
    expect(p.status).toBe("blocked");
    expect(p.blocking.map((x) => x.code)).toContain("eight_percent_1701ms_not_implemented");
    expect(p.fill_args).toEqual({});
  });

  test("A6 (in memory): recording an 8% event on a ledger that is otherwise ready produces no values and a blocking reason", () => {
    const base = fixture("a-sole-prop-2026");
    const elected = clone(base);
    elected.profiles[0].facts["2026"].regime = "eight_percent";
    const p = proposal(elected, "2551Q", "2026-Q3");
    expect(p.status).toBe("blocked");
    expect(p.fill_args).toEqual({});
  });
});

describe("VAT threshold gate (DS-B): strictly greater than 3000000.00 of posted sales year-to-date", () => {
  for (const c of HAND_VAT) {
    test(`${c.id}: ${c.breached ? "blocks" : "does not block"} the 2551Q`, () => {
      const amount = formatMoney(BigInt(c.ytd_centavos));
      const input = memoryInput({ entries: [saleEntry(PROFILE_A, "V-1", "2026-07-01", amount)] });
      const p = proposal(input, "2551Q", "2026-Q3");
      const blocked = p.blocking.some((b) => b.code === "GATE.VAT_THRESHOLD");
      expect(blocked).toBe(c.breached);
      if (c.breached) expect(p.fill_args).toEqual({});
    });
  }

  test("the 1701-MS annual uses the same gate on year-to-date sales", () => {
    const amount = "3000000.01";
    const input = memoryInput({ entries: [saleEntry(PROFILE_A, "V-2", "2026-03-01", amount)] });
    const p = proposal(input, "1701MS", "2026");
    expect(p.blocking.map((b) => b.code)).toContain("GATE.VAT_THRESHOLD");
  });

  test("the threshold figure comes from the dated fact in rates.v1, not a constant in the engine", () => {
    expect(data.rates.vat_threshold.amount_text).toBe("3000000.00");
  });
});

describe("spouse-scoped lines are refused in MVP (fixture d)", () => {
  test("2551Q and 1701-MS block and produce no values, and no spouse (B-side) key is ever filled", () => {
    const d = fixture("d-spouse-scope-2026");
    for (const form of ["2551Q", "1701MS"] as const) {
      const p = proposal(d, form, form === "2551Q" ? "2026-Q3" : "2026");
      expect(p.status).toBe("blocked");
      expect(p.blocking.map((b) => b.code)).toContain("EXC.SPOUSE_MVP");
      expect(p.fill_args).toEqual({});
    }
  });

  test("the spouse line id is reported in the blocking reason", () => {
    const p = proposal(fixture("d-spouse-scope-2026"), "2551Q", "2026-Q3");
    expect(p.blocking.find((b) => b.code === "EXC.SPOUSE_MVP")?.ids).toEqual(["D-002.2"]);
  });
});

describe("reversal in a later period is not a silent change to the earlier period", () => {
  test("a void posted after a filed quarter does not change the filed quarter's proposal", () => {
    const original = saleEntry(PROFILE_A, "L-1", "2026-06-10", "70000.00");
    const before = memoryInput({ entries: [original], periods: { "2026-Q2": "closed", "2026-Q3": "closed" } });
    const after = memoryInput({
      entries: [original, reversalEntry("L-2", original, "2026-07-20")],
      acks: ["L-2"],
      periods: { "2026-Q2": "closed", "2026-Q3": "closed" },
    });
    const q2Before = proposal(before, "2551Q", "2026-Q2");
    const q2After = proposal(after, "2551Q", "2026-Q2");
    expect(q2After.fill_args).toEqual(q2Before.fill_args);
    expect(q2After.fill_args["txtATCAmt1"]).toBe("70000.00");
    const q3 = proposal(after, "2551Q", "2026-Q3");
    expect(q3.status).toBe("blocked");
    expect(q3.blocking.map((b) => b.code)).toContain("negative_base");
    expect(q3.fill_args).toEqual({});
  });

  test("a credit note on its own is blocked until acknowledged, then counted once", () => {
    const sale = saleEntry(PROFILE_A, "K-1", "2026-07-02", "10000.00");
    const note = creditNoteEntry(PROFILE_A, "K-2", "2026-07-09", "1000.00");
    const pending = proposal(memoryInput({ entries: [sale, note] }), "2551Q", "2026-Q3");
    expect(pending.status).toBe("blocked");
    const acked = proposal(memoryInput({ entries: [sale, note], acks: ["K-2"] }), "2551Q", "2026-Q3");
    expect(acked.fill_args["txtATCAmt1"]).toBe("9000.00");
    expect(acked.expected_readback["frm2551Qv2018:txt14"]).toBe("270.00");
  });
});
