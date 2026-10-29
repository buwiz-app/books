import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { CASES } from "../src/cases.ts";
import { IMPLEMENTED_MEASURES } from "../src/measures.ts";
import { CrosswalkViolation, assertFillKey, buildProposal } from "../src/proposal.ts";
import { FORM_CODES, type FormCode } from "../src/types.ts";
import { data, fixture, PROFILE_A, clone, proposal, rawFixture } from "./support.ts";

const ALL_FIXTURES = ["a-sole-prop-2026", "b-eight-percent-2026", "c-reversal-credit-note-acknowledged-2026", "d-spouse-scope-2026", "e-second-profile-2026", "f-slice-a-2026", "g-pt-withholding-2026", "g-overpayment-2026"];
const FILL_POLICIES = ["fill_mvp", "fill_p6", "fill_p7"];

function allProposals(): Array<{ id: string; form: FormCode; p: ReturnType<typeof proposal> }> {
  return CASES.map((spec) => ({ id: spec.id, form: spec.form, p: proposal(fixture(spec.fixture), spec.form, spec.period, spec.form_context) }));
}

describe("only input keys are ever proposed (BRIDGE.INPUT_ONLY)", () => {
  test("every fill key in every case is an input key with a fill policy", () => {
    for (const { id, form, p } of allProposals()) {
      for (const key of Object.keys(p.fill_args)) {
        const field = data.crosswalk[form].byFillKey.get(key);
        expect(field, `${id}: ${key}`).toBeDefined();
        expect(field!.fill_class, `${id}: ${key}`).toBe("input");
        expect(FILL_POLICIES, `${id}: ${key}`).toContain(field!.books_policy);
      }
    }
  });

  test("no computed, locked or not_fillable key, and no Forms-owned key, is ever in fill_args or reference_values", () => {
    for (const { id, form, p } of allProposals()) {
      for (const key of [...Object.keys(p.fill_args), ...Object.keys(p.reference_values)]) {
        const field = data.crosswalk[form].byFillKey.get(key) ?? data.crosswalk[form].byKey.get(key)!;
        expect(field.fill_class, `${id}: ${key}`).toBe("input");
        expect(field.books_policy, `${id}: ${key}`).not.toBe("forms_computes");
        expect(field.books_source, `${id}: ${key}`).not.toBe("forms");
      }
    }
  });

  test("every mapping in every rule set targets an input key; fill mappings target a fill-policy key", () => {
    for (const form of FORM_CODES) {
      const cw = data.crosswalk[form];
      for (const m of data.rules[form].mappings) {
        const field = cw.byFillKey.get(m.target_key) ?? cw.byKey.get(m.target_key);
        expect(field?.fill_class, m.id).toBe("input");
        if (m.mode === "fill") expect(FILL_POLICIES, m.id).toContain(field!.books_policy);
        else expect(field!.books_policy, m.id).toBe("never_fill");
        expect(field!.rule_ids, m.id).toContain(m.id);
      }
    }
  });

  test("every fill-policy key has exactly one fill mapping, and every crosswalk rule id resolves", () => {
    for (const form of FORM_CODES) {
      const cw = data.crosswalk[form];
      const rs = data.rules[form];
      const fillTargets = rs.mappings.filter((m) => m.mode === "fill").map((m) => m.target_key);
      const sendable = cw.fields.filter((f) => f.fill_key !== null && FILL_POLICIES.includes(f.books_policy)).map((f) => f.fill_key!);
      expect([...fillTargets].sort(), form).toEqual([...sendable].sort());
      const ids = new Set([...rs.mappings.map((m) => m.id), ...rs.not_mapped.map((n) => n.id), ...data.common.rules.map((r) => r.id)]);
      for (const field of cw.fields) for (const id of field.rule_ids) expect(ids.has(id), `${form}: ${id}`).toBe(true);
    }
  });

  test("Forms-owned boxes (computed totals, tax due, rates) have no Books rule, only a pointer to Forms", () => {
    for (const form of FORM_CODES) {
      const cw = data.crosswalk[form];
      const mapped = new Set(data.rules[form].mappings.map((m) => m.id));
      const owner = new Map(data.rules[form].not_mapped.map((n) => [n.id, n.owner]));
      for (const field of cw.fields.filter((f) => f.books_policy === "forms_computes")) {
        for (const id of field.rule_ids) {
          expect(mapped.has(id), `${form}: ${id}`).toBe(false);
          expect(owner.get(id), `${form}: ${id}`).toBe("forms");
        }
      }
    }
  });

  test("forged fills of a computed, election or ATC key are refused by the crosswalk gate", () => {
    const cw = data.crosswalk["2551Q"];
    expect(() => assertFillKey(cw, "frm2551Qv2018:txt14", "8700.00", "2551Q.II.14")).toThrow(CrosswalkViolation);
    expect(() => assertFillKey(cw, "frm2551Qv2018:taxRate", "3", "2551Q.I.13")).toThrow(CrosswalkViolation);
    expect(() => assertFillKey(cw, "schedule_1.0.atc", "PT010", "2551Q.S1.1.ATC")).toThrow(/not sendable/);
    expect(() => assertFillKey(data.crosswalk["1601C"], "frm1601c:txtATC", "WW010", "1601C.H.5")).toThrow(/not sendable/);
  });

  test("a fill with the wrong mapping, or a value of the wrong type, is refused", () => {
    const cw = data.crosswalk["2551Q"];
    expect(() => assertFillKey(cw, "txtATCAmt1", "290000.00", "2551Q.II.15")).toThrow(/governed by/);
    expect(() => assertFillKey(cw, "txtATCAmt1", "290000", "2551Q.S1.1.base")).toThrow(/two-place decimal/);
    expect(assertFillKey(cw, "txtATCAmt1", "290000.00", "2551Q.S1.1.base").fill_class).toBe("input");
    expect(() => assertFillKey(data.crosswalk["1601C"], "frm1601c:TaxWithheld", "yes", "1601C.H.3")).toThrow(/printed option/);
  });

  test("election inputs and spouse keys are never filled, and appear only as refused", () => {
    for (const { p } of allProposals()) {
      for (const key of ["frm1701MS:txtTaxpayerNo12b", "frm1701MS:txtTaxpayerNo12", "frm1701MS:txtTaxpayerNo17", "taxCodeDropdown", "frm2551Qv2018:taxRate"]) {
        expect(key in p.fill_args).toBe(false);
      }
      for (const key of Object.keys(p.fill_args)) expect(key).not.toMatch(/No\d+b$|Spouse|NoB/i);
    }
    const refused = proposal(fixture("a-sole-prop-2026"), "1701MS", "2026").refused_keys;
    expect(refused.some((r) => r.reason.includes("spouse"))).toBe(true);
    expect(refused.some((r) => r.key === "frm1701MS:txtTaxpayerNo17" && r.reason.includes("form_context"))).toBe(true);
  });

  test("every Forms-context condition reads a declared input whose value is a draft path option", () => {
    for (const form of FORM_CODES) {
      const rs = data.rules[form];
      const declared = new Map(rs.form_context_inputs.map((i) => [i.path, i.supported_values]));
      const draftPaths = new Set(data.crosswalk[form].fields.flatMap((f) => f.draft_paths ?? []));
      for (const input of rs.form_context_inputs) expect(draftPaths.has(input.path), input.path).toBe(true);
      for (const m of rs.mappings) {
        for (const c of m.form_context ?? []) expect(declared.get(c.read), `${m.id}: ${c.read}`).toContain(c.equals);
      }
    }
  });
});

describe("measures catalogue and mapping coverage", () => {
  test("every measure a mapping reads is in the catalogue and implemented in the PoC", () => {
    const catalogue = new Map(data.measures.measures.map((m) => [m.id, m]));
    for (const form of FORM_CODES) {
      for (const m of data.rules[form].mappings) {
        for (const id of m.measures) {
          expect(catalogue.has(id), `${m.id}: ${id}`).toBe(true);
          expect(IMPLEMENTED_MEASURES, `${m.id}: ${id}`).toContain(id);
        }
      }
    }
  });

  test("the catalogue and the engine agree on what is implemented", () => {
    const implementedInCatalogue = data.measures.measures.filter((m) => m.poc === "implemented").map((m) => m.id).sort();
    expect([...IMPLEMENTED_MEASURES].sort()).toEqual(implementedInCatalogue);
  });

  test("every measure has a unit, a sign convention, a period basis, a person scope and a lineage requirement", () => {
    for (const m of data.measures.measures) {
      expect(m.unit, m.id).toBe("centavos");
      expect(["credit_positive", "debit_positive", "reduction_positive"], m.id).toContain(m.sign_convention);
      expect(m.period_basis.length, m.id).toBeGreaterThan(0);
      expect(m.person_scope.length, m.id).toBeGreaterThan(0);
      expect(m.lineage, m.id).toMatch(/^required/);
    }
  });

  test("the coverage matrix lists exactly the mappings, by input key", () => {
    const coverage = JSON.parse(readFileSync(`${data.dataDir}/coverage-matrix.v1.json`, "utf8"));
    for (const form of FORM_CODES) {
      const entry = coverage.forms.find((f: { form: string }) => f.form === form);
      expect(entry.fed_by_measures.map((r: { input_key: string }) => r.input_key).sort()).toEqual(data.rules[form].mappings.map((m) => m.target_key).sort());
    }
  });

  test("each proposal value cites its mapping and the measures it read", () => {
    for (const { p } of allProposals()) {
      for (const [key, lineage] of Object.entries(p.lineage)) {
        const l = lineage as { mapping_id: string; measures: string[] };
        expect(typeof l.mapping_id, key).toBe("string");
        expect(l.measures.length, key).toBeGreaterThan(0);
      }
    }
  });
});

describe("an account is not meaning (invariant 8)", () => {
  test("renaming the sales account on every sale line leaves the proposal values unchanged", () => {
    const base = fixture("a-sole-prop-2026");
    const renamed = clone(base);
    for (const entry of renamed.entries) for (const line of entry.lines) if (line.account === "sales_revenue") line.account = "subscription_revenue";
    const before = proposal(base, "2551Q", "2026-Q3");
    const after = proposal(renamed, "2551Q", "2026-Q3");
    expect(after.fill_args).toEqual(before.fill_args);
    expect(after.blocking).toEqual(before.blocking);
    expect(after.warnings).toEqual(before.warnings);
    expect(after.lineage["txtATCAmt1"]).toMatchObject({ lines: (before.lineage["txtATCAmt1"] as { lines: unknown }).lines });
  });

  test("the category catalogue defaults never drive a proposal: rewriting every default leaves the output unchanged", () => {
    const mutated = structuredClone(data) as typeof data;
    for (const category of mutated.categories.categories as Array<Record<string, unknown>>) {
      category.default_line_attributes = { income_class: "none", measure_side: null, person_scope: "self" };
    }
    const input = fixture("a-sole-prop-2026");
    expect(buildProposal(mutated, input, "2551Q", "2026-Q3").output_sha256).toBe(buildProposal(data, input, "2551Q", "2026-Q3").output_sha256);
  });

  test("category defaults carry accounting facts only: no tax type, ATC or treatment", () => {
    for (const category of data.categories.categories as Array<Record<string, any>>) {
      expect(category.default_tax_attributes, category.id).toBeUndefined();
      for (const banned of ["tax_type", "atc", "treatment", "regime", "vat_status"]) {
        expect(banned in category.default_line_attributes, `${category.id}.${banned}`).toBe(false);
      }
    }
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
    expect(() => buildProposal(data, fixture("a-sole-prop-2026"), "1701MS", "2026-Q3", {}, PROFILE_A)).toThrow(/period_kind/);
  });
});
