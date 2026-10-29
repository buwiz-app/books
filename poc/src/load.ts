import { readFileSync } from "node:fs";
import { join } from "node:path";
import { DATA_DIR, SYNTHETIC_DIR } from "./paths.ts";
import { isValidDate } from "./dates.ts";
import { parseMoney } from "./money.ts";
import type { AccountingBasis, FormCode, NormEntry, NormInput, NormLine, NormProfile } from "./types.ts";
import { ACCOUNTING_BASES, DEFAULT_ACCOUNTING_BASIS, FORM_CODES } from "./types.ts";

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

/** Closed vocabularies, taken from data/tax-attributes.v1.json (accounting attributes) and the category catalogue. */
export interface Vocab {
  measure_side: Set<string>;
  income_class: Set<string>;
  contra_kind: Set<string>;
  document_kind: Set<string>;
  person_scope: Set<string>;
  comp_component: Set<string>;
  contribution_side: Set<string>;
  contribution_scheme: Set<string>;
  credit_kind: Set<string>;
  certificate_atc: Set<string>;
  certificate_status: Set<string>;
  final_tax_withheld: Set<string>;
  account: Set<string>;
  /** Trade receivable and payable accounts (categories with accrual_only). A cash-basis book refuses them. */
  accrual_only_account: Set<string>;
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
  draft_paths: string[] | null;
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

export interface FormContextCondition {
  read: string;
  equals: string;
}

/** One measure-to-input mapping (data/rules/*.rules.json mappings[]). */
export interface Mapping {
  id: string;
  printed_item: string;
  mode: "fill" | "reference";
  target_key: string;
  measures: string[];
  aggregation: "sum" | "choice_yes_if_nonzero" | "nontaxable_13th_month_cap";
  aggregation_params?: Record<string, string>;
  period_basis: string;
  person_scope: string;
  row?: number;
  sub_window?: "Q1-Q3" | "Q4";
  form_context?: FormContextCondition[];
  non_negative?: boolean;
  gates: string[];
  review: { status: "open" | "settled"; note: string };
  poc: "implemented" | "fail_closed";
  fail_closed_code?: string;
  input_level_tax_rule?: string;
  test_vectors?: Array<Record<string, string>>;
  status: string;
  citations: string[];
}

export interface FormContextInput {
  path: string;
  source: string;
  supported_values: string[];
  required?: boolean;
  required_when?: Record<string, string>;
  why: string;
}

export interface RuleSet {
  rule_set_id: string;
  version: number;
  schema_sha256: string;
  schema_pin: string;
  measures_catalog: string;
  form_context_inputs: FormContextInput[];
  mappings: Mapping[];
  not_mapped: Array<{ id: string; printed_item: string; owner: string; note?: string }>;
}

export interface CommonRule {
  id: string;
  method: string;
  formula: string;
  status: string;
}

export interface MeasureSpec {
  id: string;
  description: string;
  unit: string;
  sign_convention: string;
  period_basis: string[];
  person_scope: string[];
  derived_from: Record<string, unknown>;
  lineage: string;
  poc: "implemented" | "planned";
}

export interface LoadedData {
  dataDir: string;
  measures: { catalog_id: string; version: number; measures: MeasureSpec[] };
  common: { rules: CommonRule[]; rule_set_id: string; version: number };
  categories: { categories: Array<{ id: string; accrual_only?: boolean }> };
  attributes: any;
  rules: Record<FormCode, RuleSet>;
  crosswalk: Record<FormCode, Crosswalk>;
  vocab: Vocab;
}

function readJson(path: string): any {
  return JSON.parse(readFileSync(path, "utf8"));
}

function vocabFrom(attributes: any, categories: { categories: Array<{ id: string; accrual_only?: boolean }> }): Vocab {
  const allowed = (name: string): Set<string> => {
    const spec = attributes.line_attributes.find((a: { name: string }) => a.name === name);
    if (!spec) throw new Error(`tax-attributes.v1.json has no attribute ${name}`);
    return new Set(spec.allowed_values.filter((v: string) => /^[A-Za-z0-9_]+$/.test(v)));
  };
  return {
    measure_side: allowed("measure_side"),
    income_class: allowed("income_class"),
    contra_kind: allowed("contra_kind"),
    document_kind: allowed("document_kind"),
    person_scope: allowed("person_scope"),
    comp_component: allowed("comp_component"),
    contribution_side: allowed("contribution_side"),
    contribution_scheme: allowed("contribution_scheme"),
    credit_kind: allowed("credit_kind"),
    certificate_atc: allowed("certificate_atc"),
    certificate_status: allowed("certificate_status"),
    final_tax_withheld: allowed("final_tax_withheld"),
    account: new Set(categories.categories.map((c) => c.id)),
    accrual_only_account: new Set(categories.categories.filter((c) => c.accrual_only === true).map((c) => c.id)),
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

/**
 * Loads every data file the PoC reads and checks that the crosswalk and rule pins agree. There is no rate,
 * threshold or due-date file: Books loads no compliance data (ADR-0007).
 */
export function loadData(dataDir: string = DATA_DIR): LoadedData {
  const measures = readJson(join(dataDir, "measures.v1.json"));
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
    if (ruleSet.measures_catalog !== measures.catalog_id) {
      throw new Error(`${form}: rule set reads ${ruleSet.measures_catalog}, catalog is ${measures.catalog_id}`);
    }
    rules[form] = ruleSet;
    crosswalk[form] = cw;
  }
  return {
    dataDir,
    measures,
    common,
    categories,
    attributes,
    rules,
    crosswalk,
    vocab: vocabFrom(attributes, categories),
  };
}

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

const PARTY_REF = /^[A-Za-z0-9_-]{1,40}$/;

export function normalizeLine(raw: any, vocab: Vocab): NormLine {
  const id = typeof raw?.line_id === "string" ? raw.line_id : "(no line_id)";
  requireIn(vocab.account, raw.account, `${id}.account`);
  const debit = raw.debit ?? "0.00";
  const credit = raw.credit ?? "0.00";
  parseMoney(debit, `${id}.debit`);
  parseMoney(credit, `${id}.credit`);
  const measureSide = optionalIn(vocab.measure_side, raw.measure_side, `${id}.measure_side`) as "debit" | "credit" | null;
  const incomeClass = requireIn(vocab.income_class, raw.income_class ?? "none", `${id}.income_class`);
  const contraKind = optionalIn(vocab.contra_kind, raw.contra_kind, `${id}.contra_kind`);
  const documentKind = requireIn(vocab.document_kind, raw.document_kind ?? "other", `${id}.document_kind`);
  const personScope = requireIn(vocab.person_scope, raw.person_scope ?? "self", `${id}.person_scope`);
  const partyRef = raw.party_ref ?? null;
  if (partyRef !== null && (typeof partyRef !== "string" || !PARTY_REF.test(partyRef))) {
    throw new InputError(`${id}.party_ref must be an opaque id`);
  }
  const compComponent = optionalIn(vocab.comp_component, raw.comp_component, `${id}.comp_component`);
  const contributionSide = requireIn(vocab.contribution_side, raw.contribution_side ?? "none", `${id}.contribution_side`);
  const contributionScheme = optionalIn(vocab.contribution_scheme, raw.contribution_scheme, `${id}.contribution_scheme`);
  const creditKind = requireIn(vocab.credit_kind, raw.credit_kind ?? "none", `${id}.credit_kind`);
  const certificateAtc = optionalIn(vocab.certificate_atc, raw.certificate_atc, `${id}.certificate_atc`);
  const certificateStatus = optionalIn(vocab.certificate_status, raw.certificate_status, `${id}.certificate_status`);
  const finalTax = requireIn(vocab.final_tax_withheld, raw.final_tax_withheld ?? "unknown", `${id}.final_tax_withheld`);
  const periodStart = raw.period_start ?? null;
  const periodEnd = raw.period_end ?? null;
  if (periodStart !== null && !isValidDate(periodStart)) throw new InputError(`${id}.period_start is not a date`);
  if (periodEnd !== null && !isValidDate(periodEnd)) throw new InputError(`${id}.period_end is not a date`);

  for (const banned of ["tax_types", "treatment", "atc", "tax_side", "regime", "vat_status"]) {
    if (raw[banned] !== undefined) {
      throw new InputError(`${id}.${banned} is a compliance attribute; Books lines carry accounting facts only (ADR-0007)`);
    }
  }
  const isCertificate = documentKind.startsWith("certificate_");
  if ((incomeClass !== "none" || compComponent !== null || isCertificate) && measureSide === null) {
    throw new InputError(`${id}: a measured line (income_class, comp_component or certificate) needs measure_side`);
  }
  if (incomeClass === "contra_sales" && contraKind === null) {
    throw new InputError(`${id}: a contra_sales line needs contra_kind (return, allowance or discount)`);
  }
  if (incomeClass !== "contra_sales" && contraKind !== null) {
    throw new InputError(`${id}: contra_kind is only for contra_sales lines`);
  }
  return {
    line_id: id,
    account: raw.account,
    debit,
    credit,
    measure_side: measureSide,
    income_class: incomeClass,
    contra_kind: contraKind,
    document_kind: documentKind,
    person_scope: personScope,
    party_ref: partyRef,
    comp_component: compComponent,
    contribution_side: contributionSide,
    contribution_scheme: contributionScheme,
    credit_kind: creditKind,
    certificate_atc: certificateAtc,
    certificate_status: certificateStatus,
    period_start: periodStart,
    period_end: periodEnd,
    final_tax_withheld: finalTax,
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

function normalizeProfile(raw: any): NormProfile {
  if (typeof raw?.tax_profile_id !== "string" || !TAX_PROFILE_ID.test(raw.tax_profile_id)) {
    throw new InputError("tax_profile_id must be an opaque UUID");
  }
  if (typeof raw.display !== "string" || !/^\*\*\*-\*\*\*-\*\*\*-\d{5}$/.test(raw.display)) {
    throw new InputError("display must be the masked form ***-***-***-NNNNN");
  }
  for (const banned of ["facts", "regime", "elections", "prior_filings"]) {
    if (raw[banned] !== undefined) {
      throw new InputError(`profile.${banned}: elections, tax facts and prior filings are Forms' (ADR-0007)`);
    }
  }
  const basis = raw.accounting_basis ?? DEFAULT_ACCOUNTING_BASIS;
  if (!ACCOUNTING_BASES.includes(basis)) throw new InputError("accounting_basis must be cash or accrual");
  const periods: Record<string, string> = {};
  for (const [key, status] of Object.entries<string>(raw.periods ?? {})) {
    if (!PERIOD_KEY.test(key)) throw new InputError(`bad period key ${key}`);
    if (status !== "open" && status !== "closed") throw new InputError(`period ${key} must be open or closed`);
    periods[key] = status;
  }
  return { tax_profile_id: raw.tax_profile_id, accounting_basis: basis, periods };
}

/**
 * The cash-basis posting rule (ADR 0008). A cash-basis book posts a sale on the day the money is received and an
 * expense on the day it is paid, so it has no trade receivable or payable lines. Its sales line carries the invoice it
 * settles (sale_invoice); a sales line tagged payment_receipt would be dropped by the sales measures, so it is refused
 * here instead of being lost silently. P1 enforces the same rule at post time.
 */
function checkCashBasis(entry: NormEntry, basis: AccountingBasis, vocab: Vocab): void {
  if (basis !== "cash") return;
  for (const line of entry.lines) {
    if (vocab.accrual_only_account.has(line.account)) {
      throw new InputError(
        `${line.line_id}: ${line.account} is an accrual account; a cash-basis book posts the sale when the money is received and the expense when it is paid`,
      );
    }
    if (line.income_class === "business_sales" && line.document_kind === "payment_receipt") {
      throw new InputError(
        `${line.line_id}: on a cash-basis book the sales line carries the invoice it settles (sale_invoice), not payment_receipt`,
      );
    }
  }
}

/** Turns a fixture file into the normalized input. Memos and labels are dropped here on purpose. */
export function normalizeFixture(raw: any, vocab: Vocab): NormInput {
  if (raw?.fixture_format !== "buwiz-poc-fixture/3") throw new InputError("fixture_format must be buwiz-poc-fixture/3");
  if (!isValidDate(raw.as_of)) throw new InputError("as_of must be YYYY-MM-DD");
  const profiles = (raw.profiles as unknown[]).map((p) => normalizeProfile(p));
  const profileIds = new Set(profiles.map((p) => p.tax_profile_id));
  if (profileIds.size !== profiles.length) throw new InputError("duplicate tax_profile_id");
  const entries = (raw.entries as unknown[]).map((e) => normalizeEntry(e, profileIds, vocab));
  const basisOf = new Map(profiles.map((p) => [p.tax_profile_id, p.accounting_basis]));
  for (const entry of entries) checkCashBasis(entry, basisOf.get(entry.tax_profile_id)!, vocab);
  const entryIds = new Set(entries.map((e) => e.entry_id));
  if (entryIds.size !== entries.length) throw new InputError("duplicate entry_id");
  const acks: string[] = [...(raw.acknowledged_entry_ids ?? [])].sort();
  for (const id of acks) if (!entryIds.has(id)) throw new InputError(`acknowledged entry ${id} does not exist`);
  return { input_format: "buwiz-poc-input/3", as_of: raw.as_of, profiles, acknowledged_entry_ids: acks, entries };
}

export function loadFixture(name: string, vocab: Vocab, dir: string = SYNTHETIC_DIR): NormInput {
  return normalizeFixture(readJson(join(dir, `${name}.json`)), vocab);
}
