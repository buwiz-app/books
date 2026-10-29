import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { CASES, type CaseSpec, HAND_GRADUATED, HAND_PT_VECTORS, HAND_THIRTEENTH, HAND_VAT } from "./cases.ts";
import { prettyJson, sha256Of } from "./canonical.ts";
import type { LoadedData } from "./load.ts";
import { buildProposal, onlyProfileId, scopeInput, type Proposal } from "./proposal.ts";
import { VECTORS_DIR } from "./paths.ts";
import { graduatedTaxCentavos, ptRateFor, vatThresholdCentavos } from "./rates.ts";
import { percentOf } from "./money.ts";
import { thirteenthMonthSplit } from "./forms.ts";
import type { NormInput } from "./types.ts";

export const VECTOR_FORMAT = "buwiz-poc-vector/1";

/** A language-neutral proposal vector: the normalized, profile-scoped input and the expected proposal. */
export function buildCaseVector(data: LoadedData, spec: CaseSpec, input: NormInput): { vector: Record<string, unknown>; proposal: Proposal } {
  const pid = onlyProfileId(input);
  const scope = scopeInput(input, pid);
  const proposal = buildProposal(data, scope, spec.form, spec.period, pid);
  return {
    proposal,
    vector: {
      vector_format: VECTOR_FORMAT,
      vector_id: spec.id,
      description: spec.description,
      fixture: spec.fixture,
      form: spec.form,
      period: spec.period,
      fixture_sha256: sha256Of(scope),
      input: scope,
      expected: proposal,
    },
  };
}

export function buildMoneyVectors(data: LoadedData): Record<string, unknown> {
  const pt = HAND_PT_VECTORS.map((c) => ({
    id: c.id,
    op: "percent_of",
    base_centavos: c.base_centavos,
    rate_bps: c.rate_bps,
    expected_centavos: c.expected_centavos,
    note: c.note,
  }));
  const grad = HAND_GRADUATED.map((c) => ({
    id: c.id,
    op: "graduated_tax_2023",
    base_centavos: c.base_centavos,
    expected_centavos: c.expected_centavos,
  }));
  const thirteenth = HAND_THIRTEENTH.map((c) => ({
    id: c.id,
    op: "thirteenth_month_split",
    amount_centavos: c.amount_centavos,
    ytd_exempt_centavos: c.ytd_exempt_centavos,
    exempt_centavos: c.exempt_centavos,
    taxable_centavos: c.taxable_centavos,
  }));
  const vat = HAND_VAT.map((c) => ({ id: c.id, op: "vat_threshold_breached", ytd_centavos: c.ytd_centavos, expected: c.breached }));
  const rateLookups = [
    { id: "pt-rate-2020-07-01", date: "2020-07-01", expected_bps: 100 },
    { id: "pt-rate-2023-06-30", date: "2023-06-30", expected_bps: 100 },
    { id: "pt-rate-2023-07-01", date: "2023-07-01", expected_bps: 300 },
    { id: "pt-rate-2024-04-01-disputed", date: "2024-04-01", expected_bps: 300 },
    { id: "pt-rate-2026-09-30", date: "2026-09-30", expected_bps: 300 },
    { id: "pt-rate-2026-10-01", date: "2026-10-01", expected_bps: 300 },
  ].map((c) => ({ ...c, op: "pt_rate_for" }));
  return {
    vector_format: VECTOR_FORMAT,
    vector_id: "money-and-rules",
    description:
      "Hand-written expectations (not computed by the engine): half-away rounding at output (DS-C), graduated tax (rates.v1 brackets), 13th-month split (1601C.II.17), VAT gate (rates.v1), PT rate lookup (rates.v1). Each case also carries the rule set that produced its expectation.",
    rules_sources: {
      rates: "data/rules/rates.v1.json",
      rate_rules_version: data.rates.rule_set_id,
      vat_threshold_amount_text: data.rates.vat_threshold.amount_text,
    },
    cases: [...pt, ...grad, ...thirteenth, ...vat, ...rateLookups],
  };
}

/**
 * Recomputes one money-and-rules case with the engine. Money results come back as decimal centavo strings,
 * never as JS numbers, so the comparison is exact.
 */
export function runMoneyCase(data: LoadedData, c: Record<string, any>): unknown {
  switch (c.op) {
    case "percent_of":
      return percentOf(BigInt(c.base_centavos), c.rate_bps).toString();
    case "graduated_tax_2023":
      return graduatedTaxCentavos(data.rates, BigInt(c.base_centavos)).toString();
    case "thirteenth_month_split": {
      const split = thirteenthMonthSplit(BigInt(c.amount_centavos), BigInt(c.ytd_exempt_centavos));
      return { exempt_centavos: split.exempt.toString(), taxable_centavos: split.taxable.toString() };
    }
    case "vat_threshold_breached":
      return BigInt(c.ytd_centavos) > vatThresholdCentavos(data.rates);
    case "pt_rate_for": {
      const found = ptRateFor(data.rates, c.date);
      return found ? found.row.rate_bps : null;
    }
    default:
      throw new Error(`unknown money case op ${c.op}`);
  }
}

export interface WrittenVector {
  file: string;
  bytes: number;
}

/** Writes every case vector and the money-and-rules vector. Returns what was written. */
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
  const moneyFile = join(dir, "money-and-rules.json");
  const moneyText = prettyJson(money);
  writeFileSync(moneyFile, moneyText);
  written.push({ file: moneyFile, bytes: moneyText.length });
  return written;
}
