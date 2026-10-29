import { readFileSync } from "node:fs";
import { join } from "node:path";
import { DATA_DIR, SYNTHETIC_DIR } from "./paths.ts";
import { isValidDate } from "./dates.ts";
import { parseMoney } from "./money.ts";
import type { FormCode, NormEntry, NormInput, NormLine, NormProfile } from "./types.ts";
import { FORM_CODES } from "./types.ts";

export class InputError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "InputError";
  }
}

export const RULE_FILE: Record<FormCode, string> = {
  "2551Q": "rules/2551q-v2018.rules.json",
  "1601C": "rules/1601c-v2018.rules.json",
  "1701MS": "rules/1701ms-v2024.rules.json",
};

export const CROSSWALK_FILE: Record<FormCode, string> = {
  "2551Q": "crosswalk/2551q-v2018.json",
  "1601C": "crosswalk/1601c-v2018.json",
  "1701MS": "crosswalk/1701ms-v2024.json",
};

/**
 * Closed vocabularies. Taken from tax-attributes.v1.json, with labels such as "(later P6) comp_income"
 * reduced to their codes. The ATC WI010 is a PoC extension (income-tax series label, bir.md W-3),
 * used only to exercise credit separation.
 */
export interface Vocab {
  tax_type: Set<string>;
  treatment: Set<string>;
  income_class: Set<string>;
  atc: Set<string>;
  document_kind: Set<string>;
  credit_kind: Set<string>;
  certificate_status: Set<string>;
  person_scope: Set<string>;
  comp_component: Set<string>;
  contribution_side: Set<string>;
  account: Set<string>;
}

export interface Field {
  /** Official key, or null for a runtime-indexed row family (then key_pattern names it). */
  key: string | null;
  key_pattern: string | null;
  label: string;
  printed_item: string;
  fill_key: string | null;
  typed_alias: string | null;
  typed_alias_status: string | null;
  fill_class: string;
  books_policy: string;
  books_source: string;
  status: string;
  lock: string | null;
  rule_ids: string[];
  value_type: string;
  options: Array<{ value: string }> | null;
}

export interface Crosswalk {
  form: FormCode;
  schema_file: string;
  schema_sha256: string;
  schema_pin: string;
  fields: Field[];
  byKey: Map<string, Field>;
  byFillKey: Map<string, Field>;
}

export interface Rule {
  id: string;
  method: string;
  books_policy: string;
  books_source: string;
  status: string;
  gates?: string[];
  sign?: string;
  rounding?: string;
  formula: string;
  test_vectors?: unknown[];
}

export interface RuleSet {
  rule_set_id: string;
  version: number;
  schema_sha256: string;
  schema_pin: string;
  rules: Rule[];
}

export interface LoadedData {
  dataDir: string;
  rates: any;
  common: { rules: Rule[]; rule_set_id: string; version: number };
  categories: { categories: Array<{ id: string }> };
  attributes: any;
  rules: Record<FormCode, RuleSet>;
  crosswalk: Record<FormCode, Crosswalk>;
  vocab: Vocab;
}

function readJson(path: string): any {
  return JSON.parse(readFileSync(path, "utf8"));
}

const LABEL_PREFIX = /^\((?:later|reserved)[^)]*\)\s*/;
const codeOf = (label: string): string => label.replace(LABEL_PREFIX, "").trim();

function vocabFrom(attributes: any, categories: { categories: Array<{ id: string }> }): Vocab {
  const allowed = (name: string): Set<string> => {
    const spec = attributes.line_attributes.find((a: { name: string }) => a.name === name);
    if (!spec) throw new Error(`tax-attributes.v1.json has no attribute ${name}`);
    return new Set(
      spec.allowed_values.map(codeOf).filter((v: string) => /^[A-Za-z0-9_]+$/.test(v) && v !== "null"),
    );
  };
  const atc = allowed("atc");
  atc.add("WI010");
  return {
    tax_type: allowed("tax_type"),
    treatment: allowed("treatment"),
    income_class: allowed("income_class"),
    atc,
    document_kind: allowed("document_kind"),
    credit_kind: allowed("credit_kind"),
    certificate_status: allowed("certificate_status"),
    person_scope: allowed("person_scope"),
    comp_component: allowed("comp_component"),
    contribution_side: allowed("contribution_side"),
    account: new Set(categories.categories.map((c) => c.id)),
  };
}

function indexCrosswalk(form: FormCode, raw: any): Crosswalk {
  const fields: Field[] = raw.fields;
  const byKey = new Map<string, Field>();
  const byFillKey = new Map<string, Field>();
  for (const field of fields) {
    const id = field.key ?? field.key_pattern;
    if (id === null || id === undefined) throw new Error(`${form}: crosswalk entry has neither key nor key_pattern`);
    if (byKey.has(id)) throw new Error(`${form}: duplicate crosswalk key ${id}`);
    byKey.set(id, field);
    if (field.fill_key !== null) {
      if (byFillKey.has(field.fill_key)) throw new Error(`${form}: duplicate fill_key ${field.fill_key}`);
      byFillKey.set(field.fill_key, field);
    }
  }
  return {
    form,
    schema_file: raw.schema_file,
    schema_sha256: raw.schema_sha256,
    schema_pin: raw.source_commit,
    fields,
    byKey,
    byFillKey,
  };
}

/** Loads every data file the PoC reads and checks that the crosswalk and rule pins agree. */
export function loadData(dataDir: string = DATA_DIR): LoadedData {
  const rates = readJson(join(dataDir, "rules/rates.v1.json"));
  const common = readJson(join(dataDir, "rules/common.rules.json"));
  const categories = readJson(join(dataDir, "categories.v1.json"));
  const attributes = readJson(join(dataDir, "tax-attributes.v1.json"));
  const rules = {} as Record<FormCode, RuleSet>;
  const crosswalk = {} as Record<FormCode, Crosswalk>;
  for (const form of FORM_CODES) {
    const ruleSet = readJson(join(dataDir, RULE_FILE[form]));
    const cw = indexCrosswalk(form, readJson(join(dataDir, CROSSWALK_FILE[form])));
    if (ruleSet.schema_sha256 !== cw.schema_sha256) {
      throw new Error(`${form}: rules and crosswalk point at different schema digests`);
    }
    rules[form] = ruleSet;
    crosswalk[form] = cw;
  }
  return {
    dataDir,
    rates,
    common,
    categories,
    attributes,
    rules,
    crosswalk,
    vocab: vocabFrom(attributes, categories),
  };
}

const TREATMENT_SIDE: Record<string, "debit" | "credit"> = {
  sale_pt: "credit",
  comp_income: "debit",
  contribution: "credit",
  expense: "debit",
  cwt_credit: "debit",
};

const DECIMAL_KEYS = new Set(["1701q_q1_q3_tax_payments"]);

function requireIn(set: Set<string>, value: unknown, label: string): string {
  if (typeof value !== "string" || !set.has(value)) {
    throw new InputError(`${label}: "${String(value)}" is not in the closed vocabulary`);
  }
  return value;
}

function optionalIn(set: Set<string>, value: unknown, label: string): string | null {
  if (value === undefined || value === null) return null;
  return requireIn(set, value, label);
}

export function normalizeLine(raw: any, vocab: Vocab): NormLine {
  const id = typeof raw?.line_id === "string" ? raw.line_id : "(no line_id)";
  requireIn(vocab.account, raw.account, `${id}.account`);
  const debit = raw.debit ?? "0.00";
  const credit = raw.credit ?? "0.00";
  parseMoney(debit, `${id}.debit`);
  parseMoney(credit, `${id}.credit`);
  const taxTypes: string[] = [...new Set<string>((raw.tax_types ?? []) as string[])]
    .map((t) => requireIn(vocab.tax_type, t, `${id}.tax_types`))
    .sort();
  const treatment = requireIn(vocab.treatment, raw.treatment ?? "none", `${id}.treatment`);
  const taxSide = raw.tax_side ?? null;
  if (taxSide !== null && taxSide !== "debit" && taxSide !== "credit") {
    throw new InputError(`${id}.tax_side must be debit or credit`);
  }
  const incomeClass = requireIn(vocab.income_class, raw.income_class ?? "none", `${id}.income_class`);
  const atc = optionalIn(vocab.atc, raw.atc, `${id}.atc`);
  const documentKind = requireIn(vocab.document_kind, raw.document_kind ?? "other", `${id}.document_kind`);
  const creditKind = requireIn(vocab.credit_kind, raw.credit_kind ?? "none", `${id}.credit_kind`);
  const certificateStatus = optionalIn(vocab.certificate_status, raw.certificate_status, `${id}.certificate_status`);
  const periodStart = raw.period_start ?? null;
  const periodEnd = raw.period_end ?? null;
  if (periodStart !== null && !isValidDate(periodStart)) throw new InputError(`${id}.period_start is not a date`);
  if (periodEnd !== null && !isValidDate(periodEnd)) throw new InputError(`${id}.period_end is not a date`);
  const personScope = requireIn(vocab.person_scope, raw.person_scope ?? "self", `${id}.person_scope`);
  const compComponent = optionalIn(vocab.comp_component, raw.comp_component, `${id}.comp_component`);
  const contributionSide = requireIn(
    vocab.contribution_side,
    raw.contribution_side ?? "none",
    `${id}.contribution_side`,
  );
  const expectedSide = TREATMENT_SIDE[treatment];
  if (expectedSide !== undefined && taxSide !== expectedSide) {
    throw new InputError(`${id}: treatment ${treatment} needs tax_side ${expectedSide}, got ${String(taxSide)}`);
  }
  if (taxTypes.length > 0 && taxSide === null) {
    throw new InputError(`${id}: a tax-relevant line (tax_types set) needs tax_side`);
  }
  return {
    line_id: id,
    account: raw.account,
    debit,
    credit,
    tax_types: taxTypes,
    treatment,
    tax_side: taxSide,
    income_class: incomeClass,
    atc,
    document_kind: documentKind,
    credit_kind: creditKind,
    certificate_status: certificateStatus,
    period_start: periodStart,
    period_end: periodEnd,
    person_scope: personScope,
    comp_component: compComponent,
    contribution_side: contributionSide,
  };
}

export function normalizeEntry(raw: any, profileIds: Set<string>, vocab: Vocab): NormEntry {
  const id = raw?.entry_id;
  if (typeof id !== "string" || !/^[A-Za-z0-9._-]{1,40}$/.test(id)) throw new InputError(`bad entry_id ${String(id)}`);
  if (!profileIds.has(raw.tax_profile_id)) throw new InputError(`${id}: tax_profile_id is not a profile in this input`);
  if (!isValidDate(raw.entry_date)) throw new InputError(`${id}: entry_date must be YYYY-MM-DD`);
  if (!Array.isArray(raw.lines) || raw.lines.length < 2) throw new InputError(`${id}: needs at least two lines`);
  const reverses = raw.reverses_entry_id ?? null;
  if (reverses !== null && typeof reverses !== "string") throw new InputError(`${id}: reverses_entry_id must be a string`);
  return {
    entry_id: id,
    tax_profile_id: raw.tax_profile_id,
    entry_date: raw.entry_date,
    reverses_entry_id: reverses,
    lines: raw.lines.map((line: unknown) => normalizeLine(line, vocab)),
  };
}

const TAX_PROFILE_ID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
const PERIOD_KEY = /^\d{4}(-Q[1-4]|-\d{2})?$/;
const FACT_VALUES: Record<string, Set<string>> = {
  regime: new Set(["graduated", "eight_percent", "special_exempt", "unknown"]),
  vat_status: new Set(["non_vat", "vat_registered", "unknown"]),
  deduction_method: new Set(["osd", "itemized"]),
  owner_taxable_compensation_2316: new Set(),
};

function normalizeProfile(raw: any): NormProfile {
  if (typeof raw?.tax_profile_id !== "string" || !TAX_PROFILE_ID.test(raw.tax_profile_id)) {
    throw new InputError("tax_profile_id must be an opaque UUID");
  }
  if (typeof raw.display !== "string" || !/^\*\*\*-\*\*\*-\*\*\*-\d{5}$/.test(raw.display)) {
    throw new InputError("display must be the masked form ***-***-***-NNNNN");
  }
  const facts: NormProfile["facts"] = {};
  for (const [year, values] of Object.entries<Record<string, string>>(raw.facts ?? {})) {
    facts[year] = {};
    for (const [key, value] of Object.entries(values)) {
      const allowed = FACT_VALUES[key];
      if (!allowed) throw new InputError(`unknown fact ${key}`);
      if (key === "owner_taxable_compensation_2316") parseMoney(value, `${year}.${key}`);
      else if (!allowed.has(value)) throw new InputError(`fact ${key}=${value} is not allowed`);
      facts[year][key] = value;
    }
  }
  const periods: Record<string, string> = {};
  for (const [key, status] of Object.entries<string>(raw.periods ?? {})) {
    if (!PERIOD_KEY.test(key)) throw new InputError(`bad period key ${key}`);
    if (status !== "open" && status !== "closed") throw new InputError(`period ${key} must be open or closed`);
    periods[key] = status;
  }
  const prior: NormProfile["prior_filings"] = {};
  for (const [year, values] of Object.entries<Record<string, string>>(raw.prior_filings ?? {})) {
    prior[year] = {};
    for (const [key, value] of Object.entries(values)) {
      if (!DECIMAL_KEYS.has(key)) throw new InputError(`unknown prior filing input ${key}`);
      parseMoney(value, `${year}.${key}`);
      prior[year][key] = value;
    }
  }
  return { tax_profile_id: raw.tax_profile_id, facts, periods, prior_filings: prior };
}

/** Turns a fixture file into the normalized input. Memos and labels are dropped here on purpose. */
export function normalizeFixture(raw: any, vocab: Vocab): NormInput {
  if (raw?.fixture_format !== "buwiz-poc-fixture/1") throw new InputError("fixture_format must be buwiz-poc-fixture/1");
  if (!isValidDate(raw.as_of)) throw new InputError("as_of must be YYYY-MM-DD");
  const profiles = (raw.profiles as unknown[]).map((p) => normalizeProfile(p));
  const profileIds = new Set(profiles.map((p) => p.tax_profile_id));
  if (profileIds.size !== profiles.length) throw new InputError("duplicate tax_profile_id");
  const entries = (raw.entries as unknown[]).map((e) => normalizeEntry(e, profileIds, vocab));
  const entryIds = new Set(entries.map((e) => e.entry_id));
  if (entryIds.size !== entries.length) throw new InputError("duplicate entry_id");
  const acks: string[] = [...(raw.acknowledged_entry_ids ?? [])].sort();
  for (const id of acks) if (!entryIds.has(id)) throw new InputError(`acknowledged entry ${id} does not exist`);
  return { input_format: "buwiz-poc-input/1", as_of: raw.as_of, profiles, acknowledged_entry_ids: acks, entries };
}

export function loadFixture(name: string, vocab: Vocab, dir: string = SYNTHETIC_DIR): NormInput {
  return normalizeFixture(readJson(join(dir, `${name}.json`)), vocab);
}
