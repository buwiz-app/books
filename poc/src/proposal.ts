import { sha256Hex, sha256Of } from "./canonical.ts";
import { dueDate, parsePeriod, type PeriodRange } from "./dates.ts";
import { form1601C, form1701MS, form2551Q, type FormCtx, type Outcome, type Val } from "./forms.ts";
import { LedgerError, postLedger } from "./ledger.ts";
import type { Crosswalk, Field, LoadedData } from "./load.ts";
import { FORM_PERIOD_KIND, type FormCode, type NormEntry, type NormInput } from "./types.ts";

export const PROPOSAL_FORMAT = "buwiz-poc-proposal/1";

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

export interface Proposal {
  proposal_format: typeof PROPOSAL_FORMAT;
  form_code: FormCode;
  tax_year: number;
  period_key: string;
  period_start: string;
  period_end: string;
  as_of: string;
  profile_digest: string;
  status: "ready" | "blocked";
  rule_set: { rule_set_id: string; rule_set_version: number; schema_sha256: string; schema_pin: string };
  fixture_sha256: string;
  inputs_hash: string;
  due: { date: string; rule_status: string; rule: string; overdue: boolean; penalties_computed: false };
  blocking: Array<{ code: string; detail: string; ids?: string[] }>;
  warnings: Array<{ code: string; detail: string; ids?: string[] }>;
  discrepancies: Array<{ id: string; status: string; summary: string; citations: string[] }>;
  rate: { code: string; rate_bps: number; effective_from: string; effective_to: string | null } | null;
  fill_args: Record<string, string>;
  expected_readback: Record<string, string>;
  reference_values: Record<string, string>;
  lineage: Record<string, unknown>;
  excluded_keys: Array<{ key: string; reason: string }>;
  refused_keys: Array<{ key: string; reason: string }>;
  output_sha256: string;
}

const FORM_FN: Record<FormCode, (ctx: FormCtx) => Outcome> = {
  "2551Q": form2551Q,
  "1601C": form1601C,
  "1701MS": form1701MS,
};

/** Fill policies: the only crosswalk policies that may produce a sent value. */
const FILL_POLICIES = new Set(["fill_mvp", "fill_p6", "fill_p7"]);

/** Election boxes with no Forms lock (verify.md s2 #3, #10; blocker B7). */
const ELECTION_RULES = new Set(["1701MS.I.12", "1701MS.I.17", "1701MS.I.18"]);

/**
 * REV.COUNT_ONCE: a sales-family line may feed only one field of its group. The 1601-C exempt portion
 * (II.18) is a component of gross compensation (II.14) by design, so it is not in a group.
 */
const COUNTED_ONCE: Record<FormCode, string[]> = {
  "2551Q": ["txtATCAmt1", "frm2551Qv2018:txt15"],
  "1601C": ["frm1601c:txtTax14", "frm1601c:txtTax19", "frm1601c:txtTax25"],
  "1701MS": ["frm1701MS:txtSalesRevenueNo5a", "frm1701MS:txtSalesReturnsNo6a", "frm1701MS:txtNonOperatingIncomeNo13a"],
};

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
 * The period-scoped inputs covered by staleness: the profile's facts for the year, its period status, the
 * prior-filing inputs, the entries that can affect this period (dated up to the period end, or certificates
 * whose period ends in it, plus the originals of reversals), the rule set, the rates and the crosswalk.
 */
function hashedInputs(data: LoadedData, scope: NormInput, form: FormCode, period: PeriodRange): unknown {
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
    period_status: profile.periods[period.key] ?? "unset",
    facts_for_year: profile.facts[String(period.year)] ?? {},
    prior_for_year: profile.prior_filings[String(period.year)] ?? {},
    acknowledged_in_scope: scope.acknowledged_entry_ids.filter((id) => scopedIds.has(id)),
    entries,
    rule_set: {
      rule_set_id: rs.rule_set_id,
      rule_set_version: rs.version,
      schema_sha256: rs.schema_sha256,
      rules_sha256: sha256Of(rs.rules),
    },
    rates_sha256: sha256Of(data.rates),
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
    case "select":
      if (!/^[A-Z]{2}\d{3}$/.test(value)) fail("is not an ATC code");
      return;
    case "text":
      if (value.length === 0) fail("is empty");
      return;
    default:
      fail(`has unknown value_type ${field.value_type}`);
  }
}

/**
 * The crosswalk gate for anything Books sends: the key must be a sendable fill key, input-class, with a fill policy,
 * cited by the rule that governs it, and the value must match the printed type. Throws CrosswalkViolation otherwise.
 */
export function assertFillKey(cw: Crosswalk, key: string, value: string, ruleId: string): Field {
  const field = cw.byFillKey.get(key);
  if (!field) throw new CrosswalkViolation(`${cw.form}: ${key} is not a crosswalk fill key`);
  if (field.fill_class !== "input") throw new CrosswalkViolation(`${cw.form}: ${key} is ${field.fill_class}; never proposed`);
  if (!FILL_POLICIES.has(field.books_policy)) {
    throw new CrosswalkViolation(`${cw.form}: ${key} policy is ${field.books_policy}; not sendable`);
  }
  checkCited(cw, key, field, ruleId);
  checkValue(cw, key, field, value);
  return field;
}

function checkCited(cw: Crosswalk, key: string, field: Field, ruleId: string): void {
  if (!field.rule_ids.includes(ruleId)) {
    throw new CrosswalkViolation(`${cw.form}: ${key} is governed by ${field.rule_ids.join("|")}, not ${ruleId}`);
  }
}

function assemble(
  data: LoadedData,
  scope: NormInput,
  form: FormCode,
  period: PeriodRange,
  outcome: Outcome,
  inputsHash: string,
): Proposal {
  const cw = data.crosswalk[form];
  const rs = data.rules[form];
  const profile = scope.profiles[0];
  const blocked = outcome.blocking.length > 0;
  const fills: Val[] = blocked ? [] : outcome.fills;
  const readback: Val[] = blocked ? [] : outcome.readback;
  const reference: Val[] = blocked ? [] : outcome.reference;
  const inputSha = sha256Of(scope);

  const fillArgs: Record<string, string> = {};
  const lineage: Record<string, unknown> = {};
  const lineOwner = new Map<string, string>();
  for (const v of fills) {
    assertFillKey(cw, v.key, v.value, v.rule_id);
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
    lineage[v.key] = {
      rule_id: v.rule_id,
      rule_set_version: rs.version,
      fixture_sha256: inputSha,
      netted_pairs: v.netted_pairs,
      lines: v.lines,
    };
  }

  const readbackValues: Record<string, string> = {};
  for (const v of readback) {
    const field = cw.byKey.get(v.key);
    if (!field) throw new CrosswalkViolation(`${form}: ${v.key} is not a crosswalk key`);
    if (!(field.books_policy === "cross_check" || field.fill_class === "computed")) {
      throw new CrosswalkViolation(`${form}: ${v.key} is not a computed key; it cannot be a read-back expectation`);
    }
    checkCited(cw, v.key, field, v.rule_id);
    checkValue(cw, v.key, field, v.value);
    readbackValues[v.key] = v.value;
  }

  const referenceValues: Record<string, string> = {};
  for (const v of reference) {
    const field = cw.byKey.get(v.key);
    if (!field) throw new CrosswalkViolation(`${form}: ${v.key} is not a crosswalk key`);
    if (field.books_policy !== "never_fill" || field.fill_class !== "input") {
      throw new CrosswalkViolation(`${form}: ${v.key} is not an evidence input; reference values are never sent`);
    }
    checkCited(cw, v.key, field, v.rule_id);
    checkValue(cw, v.key, field, v.value);
    referenceValues[v.key] = v.value;
  }

  for (const key of Object.keys(readbackValues)) {
    if (key in fillArgs || key in referenceValues) throw new CrosswalkViolation(`${form}: ${key} appears in two groups`);
  }

  const excluded: Array<{ key: string; reason: string }> = [];
  for (const field of cw.fields) {
    if (field.fill_key === null || !FILL_POLICIES.has(field.books_policy) || field.fill_key in fillArgs) continue;
    const reason = blocked
      ? `proposal blocked: ${outcome.blocking.map((b) => b.code).join(", ")}`
      : (outcome.excludedReasons[field.fill_key] ??
        "not produced for this period by the PoC (no source lines, or the rule is not implemented)");
    excluded.push({ key: field.fill_key, reason });
  }

  const refused: Array<{ key: string; reason: string }> = [];
  for (const field of cw.fields) {
    if (field.fill_key === null || FILL_POLICIES.has(field.books_policy) || field.fill_key in fillArgs) continue;
    let reason: string | null = null;
    if (field.rule_ids.some((r) => ELECTION_RULES.has(r))) {
      reason = "election box with no Forms lock; Books never fills it (blocker B7)";
    } else if (field.books_policy === "refuse_mvp_spouse") {
      reason = "spouse (B-side) key; refused in MVP (phase P7)";
    } else if (field.fill_class.startsWith("locked_")) {
      reason = `Forms ${field.fill_class.slice(7)} lock; Books never fills it`;
    }
    if (reason === null) continue;
    refused.push({ key: field.fill_key, reason });
  }

  const dueBase = outcome.due ?? dueDate(form, period);
  const body = {
    proposal_format: PROPOSAL_FORMAT,
    form_code: form,
    tax_year: period.year,
    period_key: period.key,
    period_start: period.start,
    period_end: period.end,
    as_of: scope.as_of,
    profile_digest: sha256Hex(profile.tax_profile_id),
    status: blocked ? ("blocked" as const) : ("ready" as const),
    rule_set: {
      rule_set_id: rs.rule_set_id,
      rule_set_version: rs.version,
      schema_sha256: rs.schema_sha256,
      schema_pin: rs.schema_pin,
    },
    fixture_sha256: inputSha,
    inputs_hash: inputsHash,
    due: {
      date: dueBase.date,
      rule_status: dueBase.rule_status,
      rule: dueBase.rule,
      overdue: scope.as_of > dueBase.date,
      penalties_computed: false as const,
    },
    blocking: [...outcome.blocking].sort((a, b) => compareText(a.code, b.code)),
    warnings: [...outcome.warnings].sort((a, b) => compareText(a.code, b.code) || compareText(a.detail, b.detail)),
    discrepancies: [...outcome.discrepancies].sort((a, b) => compareText(a.id, b.id)),
    rate: outcome.rate,
    fill_args: fillArgs,
    expected_readback: readbackValues,
    reference_values: referenceValues,
    lineage,
    excluded_keys: excluded.sort(byKey),
    refused_keys: refused.sort(byKey),
  };
  return { ...body, output_sha256: sha256Of(body) };
}

interface Built {
  scope: NormInput;
  period: PeriodRange;
  ctx: FormCtx;
  inputsHash: string;
}

function build(data: LoadedData, input: NormInput, form: FormCode, periodKey: string, taxProfileId?: string): Built {
  const pid = taxProfileId ?? onlyProfileId(input);
  const scope = scopeInput(input, pid);
  const period = parsePeriod(periodKey);
  if (period.kind !== FORM_PERIOD_KIND[form]) {
    throw new ProposalError("period_kind", `${form} needs a ${FORM_PERIOD_KIND[form]} period; got ${periodKey}`);
  }
  const inputsHash = sha256Of(hashedInputs(data, scope, form, period));
  const ledger = postLedger(scope.entries);
  const ctx: FormCtx = {
    data,
    profile: scope.profiles[0],
    ledger,
    period,
    acks: new Set(scope.acknowledged_entry_ids),
  };
  return { scope, period, ctx, inputsHash };
}

/** Builds a proposal. Same inputs give byte-identical canonical JSON. */
export function buildProposal(
  data: LoadedData,
  input: NormInput,
  form: FormCode,
  periodKey: string,
  taxProfileId?: string,
): Proposal {
  const built = build(data, input, form, periodKey, taxProfileId);
  const outcome = FORM_FN[form](built.ctx);
  if (outcome.blocking.length === 0 && outcome.fills.length === 0) {
    throw new ProposalError("no_outcome", "a ready outcome must produce fills");
  }
  return assemble(data, built.scope, form, built.period, outcome, built.inputsHash);
}

/** The period-scoped input hash alone. It does not validate the ledger, so it works on a changed input too. */
export function computeInputsHash(
  data: LoadedData,
  input: NormInput,
  form: FormCode,
  periodKey: string,
  taxProfileId?: string,
): string {
  const pid = taxProfileId ?? onlyProfileId(input);
  return sha256Of(hashedInputs(data, scopeInput(input, pid), form, parsePeriod(periodKey)));
}

/** "fresh" when the inputs still match the stored proposal; "stale" when any covered input changed. */
export function freshness(
  stored: Proposal,
  data: LoadedData,
  input: NormInput,
  taxProfileId?: string,
): "fresh" | "stale" {
  const current = computeInputsHash(data, input, stored.form_code, stored.period_key, taxProfileId);
  return current === stored.inputs_hash ? "fresh" : "stale";
}

/** Fill is refused unless the proposal is ready and fresh. Returns the fill args when it is allowed. */
export function assertFillable(
  stored: Proposal,
  data: LoadedData,
  input: NormInput,
  taxProfileId?: string,
): Record<string, string> {
  if (stored.status !== "ready") throw new ProposalError("not_ready", `proposal is ${stored.status}; fill refused`);
  if (freshness(stored, data, input, taxProfileId) !== "fresh") {
    throw new ProposalError("stale", "the inputs changed after this proposal was made; regenerate it before any fill");
  }
  return stored.fill_args;
}

