import { describe, expect, test } from "bun:test";
import { CASES } from "../src/cases.ts";
import { EVALUATED_RULE_IDS, IMPLEMENTED_GATES } from "../src/forms.ts";
import { CrosswalkViolation, assertFillKey, buildProposal } from "../src/proposal.ts";
import { data, fixture, PROFILE_A, clone, proposal, rawFixture } from "./support.ts";
import type { FormCode } from "../src/types.ts";

const ALL_FIXTURES = ["a-sole-prop-2026", "b-eight-percent-2026", "c-reversal-credit-note-acknowledged-2026", "d-spouse-scope-2026", "e-second-profile-2026", "f-slice-a-2026", "g-pt-withholding-2026", "g-overpayment-2026"];

function allProposals(): Array<{ id: string; form: FormCode; p: ReturnType<typeof proposal> }> {
  return CASES.map((spec) => ({ id: spec.id, form: spec.form, p: proposal(fixture(spec.fixture), spec.form, spec.period) }));
}

describe("only sendable crosswalk keys ever carry a value", () => {
  test("every fill key in every case is a sendable input key with a fill policy", () => {
    for (const { id, form, p } of allProposals()) {
      const cw = data.crosswalk[form];
      for (const key of Object.keys(p.fill_args)) {
        const field = cw.byFillKey.get(key);
        expect(field, `${id}: ${key}`).toBeDefined();
        expect(field!.fill_class, `${id}: ${key}`).toBe("input");
        expect(["fill_mvp", "fill_p6", "fill_p7"], `${id}: ${key}`).toContain(field!.books_policy);
      }
    }
  });

  test("no computed, locked or not_fillable key is ever in fill_args", () => {
    for (const { id, form, p } of allProposals()) {
      const cw = data.crosswalk[form];
      for (const key of Object.keys(p.fill_args)) {
        const field = cw.byFillKey.get(key)!;
        expect(["computed", "locked_profile", "locked_election", "locked_identity", "not_fillable"], `${id}: ${key}`).not.toContain(field.fill_class);
      }
    }
  });

  test("read-back keys are computed cross-check keys, and never also fill keys", () => {
    for (const { id, form, p } of allProposals()) {
      const cw = data.crosswalk[form];
      for (const key of Object.keys(p.expected_readback)) {
        const field = cw.byKey.get(key);
        expect(field, `${id}: ${key}`).toBeDefined();
        expect(field!.books_policy === "cross_check" || field!.fill_class === "computed", `${id}: ${key}`).toBe(true);
        expect(key in p.fill_args, `${id}: ${key}`).toBe(false);
      }
    }
  });

  test("reference values are evidence inputs with the never_fill policy, and never in fill_args", () => {
    for (const { id, form, p } of allProposals()) {
      const cw = data.crosswalk[form];
      for (const key of Object.keys(p.reference_values)) {
        const field = cw.byKey.get(key)!;
        expect(field.books_policy, `${id}: ${key}`).toBe("never_fill");
        expect(key in p.fill_args, `${id}: ${key}`).toBe(false);
      }
    }
  });

  test("the election boxes (1701-MS I.12, I.17, I.18; 2551Q item 13) are never filled", () => {
    for (const { p } of allProposals()) {
      for (const key of ["frm1701MS:txtTaxpayerNo12b", "frm1701MS:txtTaxpayerNo12", "frm1701MS:txtTaxpayerNo17", "taxCodeDropdown", "frm2551Qv2018:taxRate"]) {
        expect(key in p.fill_args).toBe(false);
      }
    }
  });

  test("spouse (B-side) keys are never filled, and any that the crosswalk names appear only as refused", () => {
    for (const { p } of allProposals()) {
      for (const key of Object.keys(p.fill_args)) {
        expect(key).not.toMatch(/No\d+b$|Spouse|NoB/i);
      }
    }
    const refused = proposal(fixture("a-sole-prop-2026"), "1701MS", "2026").refused_keys;
    expect(refused.some((r) => r.reason.includes("spouse"))).toBe(true);
    expect(refused.some((r) => r.reason.includes("election box") && r.key === "frm1701MS:txtTaxpayerNo17")).toBe(true);
  });

  test("a forged fill of a computed key is refused by the crosswalk gate", () => {
    const cw = data.crosswalk["2551Q"];
    expect(() => assertFillKey(cw, "frm2551Qv2018:txt14", "8700.00", "2551Q.II.14")).toThrow(CrosswalkViolation);
    expect(() => assertFillKey(cw, "frm2551Qv2018:taxRate", "3", "2551Q.I.13")).toThrow(CrosswalkViolation);
    expect(() => assertFillKey(cw, "frm2551Qv2018:txtATC", "PT010", "2551Q.S1.1.ATC")).toThrow(CrosswalkViolation);
  });

  test("a fill with the wrong rule, or a value of the wrong type, is refused", () => {
    const cw = data.crosswalk["2551Q"];
    expect(() => assertFillKey(cw, "txtATCAmt1", "290000.00", "2551Q.II.15")).toThrow(/governed by/);
    expect(() => assertFillKey(cw, "txtATCAmt1", "290000", "2551Q.S1.1.base")).toThrow(/two-place decimal/);
    expect(() => assertFillKey(cw, "txtATCAmt1", "1,000.00", "2551Q.S1.1.base")).toThrow(CrosswalkViolation);
    expect(assertFillKey(cw, "txtATCAmt1", "290000.00", "2551Q.S1.1.base").fill_class).toBe("input");
  });

  test("the choice value for 1601-C TaxWithheld must be one of the printed options", () => {
    const cw = data.crosswalk["1601C"];
    expect(() => assertFillKey(cw, "frm1601c:TaxWithheld", "1", "1601C.H.3")).not.toThrow();
    expect(() => assertFillKey(cw, "frm1601c:TaxWithheld", "yes", "1601C.H.3")).toThrow(/printed option/);
  });
});

describe("rule and gate coverage", () => {
  test("every rule the PoC evaluates exists in its rules file", () => {
    for (const form of ["2551Q", "1601C", "1701MS"] as const) {
      const ids = new Set(data.rules[form].rules.map((r) => r.id));
      for (const id of EVALUATED_RULE_IDS[form]) expect(ids.has(id), id).toBe(true);
    }
  });

  test("every gate that an evaluated rule cites is implemented (fail closed for anything else)", () => {
    for (const form of ["2551Q", "1601C", "1701MS"] as const) {
      const byId = new Map(data.rules[form].rules.map((r) => [r.id, r]));
      for (const id of EVALUATED_RULE_IDS[form]) {
        for (const gate of byId.get(id)?.gates ?? []) {
          expect(IMPLEMENTED_GATES, `${id} cites ${gate}`).toContain(gate);
        }
      }
    }
  });

  test("the proposal cites the rule that governs each value (checked by the assembler on every build)", () => {
    for (const { p } of allProposals()) {
      for (const [key, lineage] of Object.entries(p.lineage)) {
        expect(typeof (lineage as { rule_id: string }).rule_id, key).toBe("string");
      }
    }
  });
});

describe("an account is not tax meaning (invariant 8)", () => {
  test("renaming the sales account on every sale line leaves the proposal values unchanged", () => {
    const base = fixture("a-sole-prop-2026");
    const renamed = clone(base);
    for (const entry of renamed.entries) {
      for (const line of entry.lines) {
        if (line.account === "sales_revenue") line.account = "subscription_revenue";
      }
    }
    const before = proposal(base, "2551Q", "2026-Q3");
    const after = proposal(renamed, "2551Q", "2026-Q3");
    expect(after.fill_args).toEqual(before.fill_args);
    expect(after.expected_readback).toEqual(before.expected_readback);
    expect(after.blocking).toEqual(before.blocking);
    expect(after.warnings).toEqual(before.warnings);
    const lineBefore = before.lineage["txtATCAmt1"] as { lines: unknown; netted_pairs: number; rule_id: string };
    const lineAfter = after.lineage["txtATCAmt1"] as { lines: unknown; netted_pairs: number; rule_id: string };
    expect(lineAfter.lines).toEqual(lineBefore.lines);
    expect(lineAfter.netted_pairs).toBe(lineBefore.netted_pairs);
    expect(lineAfter.rule_id).toBe(lineBefore.rule_id);
  });

  test("the category catalogue defaults never drive a proposal: rewriting every default leaves the output unchanged", () => {
    const mutated = structuredClone(data) as typeof data;
    for (const category of mutated.categories.categories as Array<Record<string, unknown>>) {
      category.default_tax_attributes = { tax_type: "none", atc: null, treatment: "none", income_class: "none", person_scope: "self", credit_kind: "none", contribution_side: "none", comp_component: null, counterparty_kind: null };
    }
    const input = fixture("a-sole-prop-2026");
    const original = buildProposal(data, input, "2551Q", "2026-Q3");
    const rewritten = buildProposal(mutated, input, "2551Q", "2026-Q3");
    expect(rewritten.output_sha256).toBe(original.output_sha256);
  });

  test("the account keys used in fixtures all exist in the category catalogue", () => {
    const ids = new Set(data.categories.categories.map((c) => c.id));
    for (const name of ALL_FIXTURES) {
      for (const entry of rawFixture(name).entries as Array<{ lines: Array<{ account: string }> }>) {
        for (const line of entry.lines) expect(ids.has(line.account), `${name}: ${line.account}`).toBe(true);
      }
    }
  });

  test("buildProposal refuses an unknown period kind (a quarter key for the annual form)", () => {
    expect(() => buildProposal(data, fixture("a-sole-prop-2026"), "1701MS", "2026-Q3", PROFILE_A)).toThrow(/period_kind/);
  });
});
