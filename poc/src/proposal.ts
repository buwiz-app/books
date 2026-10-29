import { sha256Hex, sha256Of } from "./canonical.ts";
import { parsePeriod, type PeriodRange } from "./dates.ts";
import { COUNTED_ONCE, runBridge, type BridgeCtx, type Outcome, type Val } from "./bridge.ts";
import { LedgerError, postLedger } from "./ledger.ts";
import type { Crosswalk, Field, LoadedData } from "./load.ts";
import type { MeasureResult } from "./measures.ts";
import { formatMoney } from "./money.ts";
import { FORM_PERIOD_KIND, type AccountingBasis, type FormCode, type FormContext, type NormEntry, type NormInput } from "./types.ts";

export const PROPOSAL_FORMAT = "buwiz-poc-proposal/3";

export class CrosswalkViolation extends Error {
  constructor(message: string) {
    super(message);
    this.name = "CrosswalkViolation";
  }
}

export class ProposalError extends Error {
  readonly code: string;
  constructor(code: string, message: string) {
    super(`${code}: ${message}`);
    this.name = "ProposalError";
    this.code = code;
  }
}

export interface MeasureOut {
  value: string;
  window: { start: string; end: string };
  netted_pairs: number;
  breakdown?: Record<string, string>;
  lines: Array<{ line_id: string; entry_id: string; amount: string }>;
}

/**
 * A proposal: accounting measures with lineage, and the fill arguments the bridge derived from them for Forms
 * input boxes. It carries no tax amount, rate, due date or penalty: Forms computes those.
 */
export interface Proposal {
  proposal_format: typeof PROPOSAL_FORMAT;
  form_code: FormCode;
  tax_year: number;
  period_key: string;
  period_start: string;
  period_end: string;
  as_of: string;
  profile_digest: string;
  /** The book's accounting basis (ADR 0008). On a cash-basis book the sales measures are amounts received in the period. */
  accounting_basis: AccountingBasis;
  status: "ready" | "blocked";
  rule_set: { rule_set_id: string; rule_set_version: number; schema_sha256: string; schema_pin: string };
  measures_catalog: { catalog_id: string; version: number };
  form_context: FormContext;
  fixture_sha256: string;
  inputs_hash: string;
  measures: Record<string, MeasureOut>;
  blocking: Array<{ code: string; detail: string; ids?: string[] }>;
  warnings: Array<{ code: string; detail: string; ids?: string[] }>;
  fill_args: Record<string, string>;
  reference_values: Record<string, string>;
  lineage: Record<string, unknown>;
  excluded_keys: Array<{ key: string; reason: string }>;
  refused_keys: Array<{ key: string; reason: string }>;
  output_sha256: string;
}

/** Fill policies: the only crosswalk policies that may produce a sent value. */
const FILL_POLICIES = new Set(["fill_mvp", "fill_p6", "fill_p7"]);

/** Election inputs that Forms owns and has not locked against the profile election (verify.md s2 #3, #10; B7). */
const ELECTION_RULES = new Set(["1701MS.I.12", "1701MS.I.17", "1701MS.I.18", "2551Q.I.13"]);

const compareText = (a: string, b: string): number => (a < b ? -1 : a > b ? 1 : 0);
const byKey = <T extends { key: string }>(a: T, b: T): number => compareText(a.key, b.key);

/** The input restricted to one profile. Everything a proposal may read comes through this. */
export function scopeInput(input: NormInput, taxProfileId: string): NormInput {
  const profile = input.profiles.find((p) => p.tax_profile_id === taxProfileId);
  if (!profile) throw new ProposalError("no_profile", `profile ${taxProfileId} is not in the input`);
  const entries = input.entries.filter((e) => e.tax_profile_id === taxProfileId);
  const ids = new Set(entries.map((e) => e.entry_id));
  return {
    input_format: input.input_format,
    as_of: input.as_of,
    profiles: [profile],
    acknowledged_entry_ids: input.acknowledged_entry_ids.filter((id) => ids.has(id)),
    entries,
  };
}

export function onlyProfileId(input: NormInput): string {
  if (input.profiles.length !== 1) throw new ProposalError("profile_required", "pass taxProfileId when the input has several profiles");
  return input.profiles[0].tax_profile_id;
}

/**
 * The period-scoped inputs covered by staleness: the accounting basis, the period status, the acknowledgements, the entries that can
 * affect this period (dated from 1 January up to the period end, or certificates whose period ends in that range,
 * plus the originals of reversals), the Forms context, the rule set, the measures catalogue and the crosswalk.
 */
function hashedInputs(data: LoadedData, scope: NormInput, form: FormCode, period: PeriodRange, formContext: FormContext): unknown {
  const profile = scope.profiles[0];
  const yearStart = `${period.year}-01-01`;
  const inWindow = (e: NormEntry): boolean =>
    (e.entry_date >= yearStart && e.entry_date <= period.end) ||
    e.lines.some((l) => l.period_end !== null && l.period_end >= yearStart && l.period_end <= period.end);
  const scopedSet = new Set<NormEntry>(scope.entries.filter(inWindow));
  for (const entry of [...scopedSet]) {
    if (entry.reverses_entry_id === null) continue;
    const original = scope.entries.find((e) => e.entry_id === entry.reverses_entry_id);
    if (original) scopedSet.add(original);
  }
  const entries = [...scopedSet].sort((a, b) => compareText(a.entry_id, b.entry_id));
  const scopedIds = new Set(entries.map((e) => e.entry_id));
  const rs = data.rules[form];
  return {
    form,
    tax_year: period.year,
    period_key: period.key,
    period_start: period.start,
    period_end: period.end,
    profile_digest: sha256Hex(profile.tax_profile_id),
    accounting_basis: profile.accounting_basis,
    period_status: profile.periods[period.key] ?? "unset",
    acknowledged_in_scope: scope.acknowledged_entry_ids.filter((id) => scopedIds.has(id)),
    entries,
    form_context: formContext,
    rule_set: {
      rule_set_id: rs.rule_set_id,
      rule_set_version: rs.version,
      schema_sha256: rs.schema_sha256,
      mappings_sha256: sha256Of(rs.mappings),
      form_context_inputs_sha256: sha256Of(rs.form_context_inputs),
    },
    measures_catalog_sha256: sha256Of(data.measures),
    crosswalk_sha256: sha256Of(data.crosswalk[form].fields),
  };
}

function checkValue(cw: Crosswalk, key: string, field: Field, value: string): void {
  const fail = (why: string): never => {
    throw new CrosswalkViolation(`${cw.form}: ${key} value ${JSON.stringify(value)} ${why}`);
  };
  switch (field.value_type) {
    case "money":
      if (!/^-?\d+\.\d{2}$/.test(value)) fail("is not two-place decimal text");
      return;
    case "whole":
      if (!/^\d+$/.test(value)) fail("is not a whole number");
      return;
    case "choice":
      if (!field.options?.some((o) => o.value === value)) fail("is not one of the printed option values");
      return;
    case "text":
      if (value.length === 0) fail("is empty");
      return;
    default:
      fail(`has value_type ${field.value_type}, which the bridge never sends`);
  }
}

function checkCited(cw: Crosswalk, key: string, field: Field, mappingId: string): void {
  if (!field.rule_ids.includes(mappingId)) {
    throw new CrosswalkViolation(`${cw.form}: ${key} is governed by ${field.rule_ids.join("|")}, not ${mappingId}`);
  }
}

/**
 * The crosswalk gate for anything Books sends (BRIDGE.INPUT_ONLY): the key must be a sendable fill key, input-class,
 * with a fill policy, cited by the mapping that governs it, and the value must match the printed type.
 */
export function assertFillKey(cw: Crosswalk, key: string, value: string, mappingId: string): Field {
  const field = cw.byFillKey.get(key);
  if (!field) throw new CrosswalkViolation(`${cw.form}: ${key} is not a crosswalk fill key`);
  if (field.fill_class !== "input") throw new CrosswalkViolation(`${cw.form}: ${key} is ${field.fill_class}; never proposed`);
  if (!FILL_POLICIES.has(field.books_policy)) {
    throw new CrosswalkViolation(`${cw.form}: ${key} policy is ${field.books_policy}; not sendable`);
  }
  checkCited(cw, key, field, mappingId);
  checkValue(cw, key, field, value);
  return field;
}

function measureOut(r: MeasureResult): MeasureOut {
  const out: MeasureOut = {
    value: formatMoney(r.value),
    window: r.window,
    netted_pairs: r.netted_pairs,
    lines: r.lines
      .map(({ line, amount }) => ({ line_id: line.line_id, entry_id: line.entry_id, amount: formatMoney(amount) }))
      .sort((a, b) => compareText(a.line_id, b.line_id)),
  };
  if (r.breakdown) {
    out.breakdown = Object.fromEntries(Object.keys(r.breakdown).sort().map((k) => [k, formatMoney(r.breakdown![k])]));
  }
  return out;
}

function assemble(
  data: LoadedData,
  scope: NormInput,
  form: FormCode,
  period: PeriodRange,
  formContext: FormContext,
  outcome: Outcome,
  inputsHash: string,
): Proposal {
  const cw = data.crosswalk[form];
  const rs = data.rules[form];
  const profile = scope.profiles[0];
  const blocked = outcome.blocking.length > 0;
  const values: Val[] = blocked ? [] : outcome.values;
  const inputSha = sha256Of(scope);

  const fillArgs: Record<string, string> = {};
  const referenceValues: Record<string, string> = {};
  const lineage: Record<string, unknown> = {};
  const lineOwner = new Map<string, string>();
  for (const v of values) {
    if (v.mode === "fill") {
      assertFillKey(cw, v.key, v.value, v.mapping_id);
      if (COUNTED_ONCE[form].includes(v.key)) {
        for (const line of v.lines) {
          const owner = lineOwner.get(line.line_id);
          if (owner !== undefined && owner !== v.key) {
            throw new LedgerError("counted_twice", `${line.line_id} feeds both ${owner} and ${v.key}`);
          }
          lineOwner.set(line.line_id, v.key);
        }
      }
      fillArgs[v.key] = v.value;
    } else {
      const field = cw.byKey.get(v.key);
      if (!field || field.fill_class !== "input" || FILL_POLICIES.has(field.books_policy)) {
        throw new CrosswalkViolation(`${form}: ${v.key} is not an evidence input; reference values are never sent`);
      }
      checkCited(cw, v.key, field, v.mapping_id);
      checkValue(cw, v.key, field, v.value);
      referenceValues[v.key] = v.value;
    }
    lineage[v.key] = {
      mapping_id: v.mapping_id,
      measures: v.measures,
      rule_set_version: rs.version,
      fixture_sha256: inputSha,
      netted_pairs: v.netted_pairs,
      lines: v.lines,
      ...(v.note ? { note: v.note } : {}),
    };
  }

  const excluded: Array<{ key: string; reason: string }> = [];
  for (const field of cw.fields) {
    if (field.fill_key === null || !FILL_POLICIES.has(field.books_policy) || field.fill_key in fillArgs) continue;
    const reason = blocked
      ? `proposal blocked: ${uniqueCodes(outcome)}`
      : (outcome.excludedReasons[field.fill_key] ?? "not produced for this period");
    excluded.push({ key: field.fill_key, reason });
  }

  const refused: Array<{ key: string; reason: string }> = [];
  for (const field of cw.fields) {
    if (field.fill_key === null || FILL_POLICIES.has(field.books_policy) || field.fill_key in fillArgs) continue;
    let reason: string | null = null;
    if (field.rule_ids.some((r) => ELECTION_RULES.has(r))) {
      reason = "Forms-owned election input: the bridge may read it as form_context, never fills or decides it (B7: not locked against the profile election)";
    } else if (field.books_policy === "refuse_mvp_spouse") {
      reason = "spouse (B-side) key; refused in MVP (phase P7)";
    } else if (field.fill_class.startsWith("locked_")) {
      reason = `Forms ${field.fill_class.slice(7)} lock; Books never fills it`;
    } else if (field.books_source === "forms") {
      reason = "Forms decides it (ATC selection, threshold or classification); Books never sends it";
    }
    if (reason === null) continue;
    refused.push({ key: field.fill_key, reason });
  }

  const measures: Record<string, MeasureOut> = {};
  for (const key of Object.keys(outcome.measures).sort()) measures[key] = measureOut(outcome.measures[key]);

  const body = {
    proposal_format: PROPOSAL_FORMAT as typeof PROPOSAL_FORMAT,
    form_code: form,
    tax_year: period.year,
    period_key: period.key,
    period_start: period.start,
    period_end: period.end,
    as_of: scope.as_of,
    profile_digest: sha256Hex(profile.tax_profile_id),
    accounting_basis: profile.accounting_basis,
    status: blocked ? ("blocked" as const) : ("ready" as const),
    rule_set: {
      rule_set_id: rs.rule_set_id,
      rule_set_version: rs.version,
      schema_sha256: rs.schema_sha256,
      schema_pin: rs.schema_pin,
    },
    measures_catalog: { catalog_id: data.measures.catalog_id, version: data.measures.version },
    form_context: formContext,
    fixture_sha256: inputSha,
    inputs_hash: inputsHash,
    measures: blocked ? {} : measures,
    blocking: [...outcome.blocking].sort((a, b) => compareText(a.code, b.code) || compareText(a.detail, b.detail)),
    warnings: [...outcome.warnings].sort((a, b) => compareText(a.code, b.code) || compareText(a.detail, b.detail)),
    fill_args: fillArgs,
    reference_values: referenceValues,
    lineage,
    excluded_keys: excluded.sort(byKey),
    refused_keys: refused.sort(byKey),
  };
  return { ...body, output_sha256: sha256Of(body) };
}

function uniqueCodes(outcome: Outcome): string {
  return [...new Set(outcome.blocking.map((b) => b.code))].sort().join(", ");
}

function build(data: LoadedData, input: NormInput, form: FormCode, periodKey: string, formContext: FormContext, taxProfileId?: string) {
  const pid = taxProfileId ?? onlyProfileId(input);
  const scope = scopeInput(input, pid);
  const period = parsePeriod(periodKey);
  if (period.kind !== FORM_PERIOD_KIND[form]) {
    throw new ProposalError("period_kind", `${form} needs a ${FORM_PERIOD_KIND[form]} period; got ${periodKey}`);
  }
  const inputsHash = sha256Of(hashedInputs(data, scope, form, period, formContext));
  const ledger = postLedger(scope.entries);
  const ctx: BridgeCtx = {
    data,
    form,
    profile: scope.profiles[0],
    ledger,
    period,
    acks: new Set(scope.acknowledged_entry_ids),
    formContext,
  };
  return { scope, period, ctx, inputsHash };
}

/**
 * Builds a proposal. Same inputs give byte-identical canonical JSON. `formContext` holds the values the bridge reads
 * from the Forms draft (for example the 1701-MS tax option); Books never supplies them itself.
 */
export function buildProposal(
  data: LoadedData,
  input: NormInput,
  form: FormCode,
  periodKey: string,
  formContext: FormContext = {},
  taxProfileId?: string,
): Proposal {
  const built = build(data, input, form, periodKey, formContext, taxProfileId);
  const outcome = runBridge(built.ctx);
  return assemble(data, built.scope, form, built.period, formContext, outcome, built.inputsHash);
}

/** The period-scoped input hash alone. It does not validate the ledger, so it works on a changed input too. */
export function computeInputsHash(
  data: LoadedData,
  input: NormInput,
  form: FormCode,
  periodKey: string,
  formContext: FormContext = {},
  taxProfileId?: string,
): string {
  const pid = taxProfileId ?? onlyProfileId(input);
  return sha256Of(hashedInputs(data, scopeInput(input, pid), form, parsePeriod(periodKey), formContext));
}

/**
 * "fresh" when the inputs still match the stored proposal; "stale" when any covered input changed. `formContext` is
 * the Forms draft's current context; it defaults to the one the proposal was built with.
 */
export function freshness(
  stored: Proposal,
  data: LoadedData,
  input: NormInput,
  formContext: FormContext = stored.form_context,
  taxProfileId?: string,
): "fresh" | "stale" {
  const current = computeInputsHash(data, input, stored.form_code, stored.period_key, formContext, taxProfileId);
  return current === stored.inputs_hash ? "fresh" : "stale";
}

/** Fill is refused unless the proposal is ready and fresh. Returns the fill args when it is allowed. */
export function assertFillable(
  stored: Proposal,
  data: LoadedData,
  input: NormInput,
  formContext: FormContext = stored.form_context,
  taxProfileId?: string,
): Record<string, string> {
  if (stored.status !== "ready") throw new ProposalError("not_ready", `proposal is ${stored.status}; fill refused`);
  if (freshness(stored, data, input, formContext, taxProfileId) !== "fresh") {
    throw new ProposalError("stale", "the inputs changed after this proposal was made; regenerate it before any fill");
  }
  return stored.fill_args;
}
