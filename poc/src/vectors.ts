import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { nontaxableThirteenthCentavos } from "./bridge.ts";
import { CASES, type CaseSpec, HAND_MONEY, HAND_THIRTEENTH } from "./cases.ts";
import { prettyJson, sha256Of } from "./canonical.ts";
import type { LoadedData } from "./load.ts";
import { formatMoney, parseMoney } from "./money.ts";
import { buildProposal, onlyProfileId, scopeInput, type Proposal } from "./proposal.ts";
import { VECTORS_DIR } from "./paths.ts";
import type { NormInput } from "./types.ts";

export const VECTOR_FORMAT = "buwiz-poc-vector/2";
export const MONEY_VECTOR_FILE = "money-and-mapping.json";

/**
 * A language-neutral proposal vector: the normalized, profile-scoped input, the Forms context the bridge read, and the
 * expected proposal (measures and fill arguments). `expected_forms_computed` is documentation of Forms' expected
 * output; it is not produced by Books and no test compares Books code against it.
 */
export function buildCaseVector(data: LoadedData, spec: CaseSpec, input: NormInput): { vector: Record<string, unknown>; proposal: Proposal } {
  const pid = onlyProfileId(input);
  const scope = scopeInput(input, pid);
  const proposal = buildProposal(data, scope, spec.form, spec.period, spec.form_context, pid);
  return {
    proposal,
    vector: {
      vector_format: VECTOR_FORMAT,
      vector_id: spec.id,
      description: spec.description,
      fixture: spec.fixture,
      form: spec.form,
      period: spec.period,
      form_context: spec.form_context,
      fixture_sha256: sha256Of(scope),
      input: scope,
      expected: proposal,
      ...(spec.expected_forms_computed
        ? { expected_forms_computed: { ...spec.expected_forms_computed, asserted_against_books: false } }
        : {}),
    },
  };
}

export function buildMoneyVectors(data: LoadedData): Record<string, unknown> {
  const rule = data.rules["1601C"].mappings.find((m) => m.id === "1601C.II.17")!;
  const cap = parseMoney(rule.aggregation_params?.annual_cap_text, "annual_cap_text");
  const money = HAND_MONEY.map((c) => ({ id: c.id, op: "money_text_round_trip", text: c.text, value_centavos: c.value_centavos }));
  const thirteenth = HAND_THIRTEENTH.map((c) => ({
    id: c.id,
    op: "nontaxable_13th_month_cap",
    cap_centavos: cap.toString(),
    paid_centavos: c.paid_centavos,
    paid_earlier_centavos: c.paid_earlier_centavos,
    nontaxable_centavos: c.nontaxable_centavos,
  }));
  return {
    vector_format: VECTOR_FORMAT,
    vector_id: "money-and-mapping",
    description:
      "Hand-written expectations (not computed by the engine): money text to integer centavos and back, and the 1601-C item 17 non-taxable split (the one input-level tax rule in the bridge, annual cap read from data/rules/1601c-v2018.rules.json). No rate, rounding, threshold or graduated-tax vector: those are Forms' (ADR-0007).",
    rules_sources: {
      mapping: "data/rules/1601c-v2018.rules.json 1601C.II.17",
      rule_set_id: data.rules["1601C"].rule_set_id,
    },
    cases: [...money, ...thirteenth],
  };
}

/** Recomputes one money-and-mapping case with the engine. Results are decimal strings, never JS numbers. */
export function runMoneyCase(_data: LoadedData, c: Record<string, any>): unknown {
  switch (c.op) {
    case "money_text_round_trip": {
      const centavos = parseMoney(c.text, c.id);
      return { value_centavos: centavos.toString(), text: formatMoney(centavos) };
    }
    case "nontaxable_13th_month_cap":
      return nontaxableThirteenthCentavos(BigInt(c.paid_centavos), BigInt(c.paid_earlier_centavos), BigInt(c.cap_centavos)).toString();
    default:
      throw new Error(`unknown money case op ${c.op}`);
  }
}

export interface WrittenVector {
  file: string;
  bytes: number;
}

/** Writes every case vector and the money-and-mapping vector. Returns what was written. */
export function writeVectors(data: LoadedData, loadInput: (fixture: string) => NormInput, dir: string = VECTORS_DIR): WrittenVector[] {
  const written: WrittenVector[] = [];
  for (const spec of CASES) {
    const { vector } = buildCaseVector(data, spec, loadInput(spec.fixture));
    const file = join(dir, `${spec.id}.json`);
    const text = prettyJson(vector);
    writeFileSync(file, text);
    written.push({ file, bytes: text.length });
  }
  const money = buildMoneyVectors(data);
  const moneyFile = join(dir, MONEY_VECTOR_FILE);
  const moneyText = prettyJson(money);
  writeFileSync(moneyFile, moneyText);
  written.push({ file: moneyFile, bytes: moneyText.length });
  return written;
}
