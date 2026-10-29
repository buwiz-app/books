import { describe, expect, test } from "bun:test";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { DATA_DIR, POC_DIR } from "../src/paths.ts";
import {
  EIGHT_PERCENT,
  GRADUATED_ITEMIZED,
  clone,
  creditNoteEntry,
  fixture,
  lineageOf,
  memoryInput,
  proposal,
  PROFILE_A,
  reversalEntry,
  saleEntry,
} from "./support.ts";

describe("return events need acknowledgement; each is counted once (fixture c, REV.*)", () => {
  const c = fixture("c-reversal-credit-note-2026");
  const cAck = fixture("c-reversal-credit-note-acknowledged-2026");

  test("Q3 is held until the later-period reversal and the credit note are acknowledged", () => {
    const p = proposal(c, "2551Q", "2026-Q3");
    expect(p.status).toBe("blocked");
    expect(p.blocking.find((b) => b.code === "GATE.RETURNS_ACK")?.ids).toEqual(["C-006", "C-007"]);
    expect(p.fill_args).toEqual({});
  });

  test("after acknowledgement, net sales are 155000.00", () => {
    const p = proposal(cAck, "2551Q", "2026-Q3");
    expect(p.status).toBe("ready");
    expect(p.fill_args["txtATCAmt1"]).toBe("155000.00");
  });

  test("the same-quarter void nets to zero and is left out of lineage", () => {
    const lin = lineageOf(proposal(cAck, "2551Q", "2026-Q3"), "txtATCAmt1");
    expect(lin.lines.map((l) => l.entry_id).sort()).toEqual(["C-002", "C-005", "C-006", "C-007"]);
    expect(lin.netted_pairs).toBe(1);
  });

  test("the later-period reversal counts once, as a negative line in Q3, and Q2 is unchanged", () => {
    const lines = lineageOf(proposal(cAck, "2551Q", "2026-Q3"), "txtATCAmt1").lines;
    expect(lines.find((l) => l.entry_id === "C-006")?.amount).toBe("-40000.00");
    expect(lines.find((l) => l.entry_id === "C-007")?.amount).toBe("-5000.00");
    expect(proposal(c, "2551Q", "2026-Q2").fill_args["txtATCAmt1"]).toBe("40000.00");
  });

  test("a void posted after a closed quarter does not change that quarter", () => {
    const original = saleEntry(PROFILE_A, "L-1", "2026-06-10", "70000.00");
    const periods = { "2026-Q2": "closed", "2026-Q3": "closed" };
    const before = memoryInput({ entries: [original], periods });
    const after = memoryInput({ entries: [original, reversalEntry("L-2", original, "2026-07-20")], acks: ["L-2"], periods });
    expect(proposal(after, "2551Q", "2026-Q2").fill_args).toEqual(proposal(before, "2551Q", "2026-Q2").fill_args);
    expect(proposal(after, "2551Q", "2026-Q3").blocking.map((b) => b.code)).toContain("negative_measure");
  });

  test("returns, allowances and discounts are separate measures; 1701-MS item 6 sums them once", () => {
    const input = memoryInput({
      entries: [
        saleEntry(PROFILE_A, "S-1", "2026-07-02", "10000.00"),
        creditNoteEntry(PROFILE_A, "R-1", "2026-07-09", "100.00", "return"),
        creditNoteEntry(PROFILE_A, "R-2", "2026-07-10", "200.00", "allowance"),
        creditNoteEntry(PROFILE_A, "R-3", "2026-07-11", "300.00", "discount"),
      ],
      acks: ["R-1", "R-2", "R-3"],
    });
    const p = proposal(input, "1701MS", "2026");
    expect(p.measures["sales_returns"]?.value).toBe("100.00");
    expect(p.measures["sales_allowances"]?.value).toBe("200.00");
    expect(p.measures["sales_discounts"]?.value).toBe("300.00");
    expect(p.fill_args["frm1701MS:txtSalesRevenueNo5a"]).toBe("10000.00");
    expect(p.fill_args["frm1701MS:txtSalesReturnsNo6a"]).toBe("600.00");
    expect(proposal(input, "2551Q", "2026-Q3").fill_args["txtATCAmt1"]).toBe("9400.00");
  });
});

describe("the bridge reads Forms choices and never decides them (BRIDGE.FORM_CONTEXT_READ_ONLY)", () => {
  const a = fixture("a-sole-prop-2026");

  test("1701-MS without the Forms tax option blocks: the bridge never defaults it", () => {
    const p = proposal(a, "1701MS", "2026", {});
    expect(p.status).toBe("blocked");
    expect(p.blocking.find((b) => b.code === "form_context_missing")?.detail).toContain("/taxpayer/tax_option");
    expect(p.fill_args).toEqual({});
  });

  test("graduated without the deduction method blocks; eight_percent does not need it", () => {
    expect(proposal(a, "1701MS", "2026", { "/taxpayer/tax_option": "graduated" }).blocking.map((b) => b.code)).toContain("form_context_missing");
    expect(proposal(a, "1701MS", "2026", EIGHT_PERCENT).status).toBe("ready");
  });

  test("an option this rule set does not map, or an undeclared context key, blocks", () => {
    expect(proposal(a, "1701MS", "2026", { "/taxpayer/tax_option": "unanswered" }).blocking.map((b) => b.code)).toContain("form_context_unsupported");
    expect(proposal(a, "2551Q", "2026-Q3", { "/taxpayer/tax_option": "graduated" }).blocking.map((b) => b.code)).toContain("form_context_undeclared");
  });

  test("under itemized (read from Forms) cost of sales is proposed for IV.8", () => {
    const input = clone(a);
    input.entries.push({
      entry_id: "CS-1",
      tax_profile_id: PROFILE_A,
      entry_date: "2026-05-05",
      reverses_entry_id: null,
      lines: [
        { ...input.entries[0].lines[0], line_id: "CS-1.1", account: "cost_of_goods_sold", debit: "5000.00", credit: "0.00", measure_side: "debit", income_class: "cost_of_sales" },
        { ...input.entries[0].lines[0], line_id: "CS-1.2", account: "bank_accounts", debit: "0.00", credit: "5000.00" },
      ],
    });
    expect(proposal(input, "1701MS", "2026", GRADUATED_ITEMIZED).fill_args["frm1701MS:txtCostofSalesNo8a"]).toBe("5000.00");
  });

  test("the 8% Forms context changes only the 1701-MS mapping: 2551Q and 1601-C outputs equal fixture a's", () => {
    const b = fixture("b-eight-percent-2026");
    expect(proposal(b, "2551Q", "2026-Q3").fill_args).toEqual(proposal(a, "2551Q", "2026-Q3").fill_args);
    expect(proposal(b, "1601C", "2026-09").fill_args).toEqual(proposal(a, "1601C", "2026-09").fill_args);
  });
});

describe("spouse-scoped lines are refused in MVP (fixture d)", () => {
  test("2551Q and 1701-MS block, and the spouse line is named", () => {
    const d = fixture("d-spouse-scope-2026");
    for (const [form, period] of [["2551Q", "2026-Q3"], ["1701MS", "2026"]] as const) {
      const p = proposal(d, form, period);
      expect(p.status).toBe("blocked");
      expect(p.blocking.find((b) => b.code === "EXC.SPOUSE_MVP")?.ids).toEqual(["D-002.2"]);
      expect(p.fill_args).toEqual({});
    }
  });
});

describe("Books contains no compliance logic (ADR-0007)", () => {
  const COMPLIANCE = /rate_bps|pt_rate|vat_threshold|eight_percent_year|GATE\.8PCT|GATE\.VAT_THRESHOLD|GATE\.OVERDUE|graduatedTax|percentOf|dueDate|penalt(y|ies)_computed/;

  test("no PoC source file holds a rate, threshold, graduated table, due date or penalty helper", () => {
    for (const dir of ["src", "tools"]) {
      for (const name of readdirSync(join(POC_DIR, dir)).filter((n) => n.endsWith(".ts"))) {
        expect(readFileSync(join(POC_DIR, dir, name), "utf8"), `${dir}/${name}`).not.toMatch(COMPLIANCE);
      }
    }
  });

  test("the data directory holds no rate file, and the rules hold no compliance gate", () => {
    expect(readdirSync(join(DATA_DIR, "rules")).sort()).toEqual([
      "1601c-v2018.rules.json",
      "1701ms-v2024.rules.json",
      "2551q-v2018.rules.json",
      "common.rules.json",
    ]);
    const rules = readdirSync(join(DATA_DIR, "rules")).map((n) => readFileSync(join(DATA_DIR, "rules", n), "utf8")).join("\n");
    expect(rules).not.toMatch(/GATE\.(VAT_THRESHOLD|8PCT|MIXED_INCOME|OVERDUE)|"ELIG\.|rate_bps|"brackets"/);
  });
});
