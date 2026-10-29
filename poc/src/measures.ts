import { inRange } from "./dates.ts";
import { LedgerError, sameWindowReversalEntries, type Ledger, type PostedLine } from "./ledger.ts";

/**
 * Accounting measures (data/measures.v1.json). Each measure selects posted lines by their accounting attributes,
 * never by account, and sums their signed centavos exactly. No rate, election, threshold or rounding is applied
 * here (ADR-0007). Every value carries its lineage: the line ids and signed amounts that sum to it.
 */

export interface Window {
  start: string;
  end: string;
}

export interface Excluded {
  code: string;
  line_id: string;
}

export interface MeasureResult {
  id: string;
  window: Window;
  value: bigint;
  /** Lines that make up the value, with their signed contribution. Same-window void pairs are netted out. */
  lines: Array<{ line: PostedLine; amount: bigint }>;
  netted_pairs: number;
  breakdown: Record<string, bigint> | null;
  /** Lines in the window that the measure deliberately leaves out, with the rule that excludes them. */
  excluded: Excluded[];
  /** Sales-family lines that are return events (GATE.RETURNS_ACK). */
  return_events: PostedLine[];
}

export const COMPENSATION_COMPONENTS = new Set([
  "basic_pay",
  "statutory_minimum_wage",
  "holiday_ot_nsd_hazard",
  "thirteenth_month_and_benefits",
  "de_minimis",
]);

interface Spec {
  /** "entry": the entry date is in the window. "certificate": the certificate's period_end is in the window. */
  basis: "entry" | "certificate";
  select: (l: PostedLine) => boolean;
  /** Signed contribution of a selected line. Default: the line's measure. */
  amount?: (l: PostedLine) => bigint;
  breakdownBy?: (l: PostedLine) => string;
  /** Lines in the window that this measure leaves out on purpose, and why. */
  exclude?: (l: PostedLine) => string | null;
  salesFamily?: boolean;
}

const self = (l: PostedLine): boolean => l.person_scope === "self";
const employee = (l: PostedLine): boolean => l.person_scope === "employee";
const isSale = (l: PostedLine): boolean =>
  l.income_class === "business_sales" && l.document_kind !== "payment_receipt" && self(l);
const isContra = (kind: string) => (l: PostedLine): boolean =>
  l.income_class === "contra_sales" && l.contra_kind === kind && self(l);
const isAnyContra = (l: PostedLine): boolean => l.income_class === "contra_sales" && self(l);
const component = (name: string) => (l: PostedLine): boolean => l.comp_component === name && employee(l);

function certificateExclusion(kind: "pt" | "it") {
  return (l: PostedLine): string | null => {
    if (!l.document_kind.startsWith("certificate_")) return null;
    if (l.document_kind === "certificate_2306") return "EXC.WB084_FINAL";
    if (l.document_kind !== "certificate_2307") return null;
    if (l.credit_kind !== kind) return kind === "pt" ? "EXC.IT_CWT_NOT_PT" : "EXC.PT_CWT_NOT_IT";
    if (l.certificate_status !== "received") return "EXC.UNRECEIVED_CERT";
    return null;
  };
}

function certificate(kind: "pt" | "it") {
  return (l: PostedLine): boolean =>
    l.document_kind === "certificate_2307" && l.credit_kind === kind && l.certificate_status === "received";
}

const SPECS: Record<string, Spec> = {
  gross_sales: { basis: "entry", select: isSale, salesFamily: true },
  sales_returns: { basis: "entry", select: isContra("return"), amount: (l) => -l.measure, salesFamily: true },
  sales_allowances: { basis: "entry", select: isContra("allowance"), amount: (l) => -l.measure, salesFamily: true },
  sales_discounts: { basis: "entry", select: isContra("discount"), amount: (l) => -l.measure, salesFamily: true },
  net_sales: { basis: "entry", select: (l) => isSale(l) || isAnyContra(l), salesFamily: true },
  cost_of_sales: { basis: "entry", select: (l) => l.income_class === "cost_of_sales" && self(l) },
  operating_expenses_by_category: {
    basis: "entry",
    select: (l) => l.income_class === "operating_expense" && self(l),
    breakdownBy: (l) => l.account,
  },
  other_non_operating_income: { basis: "entry", select: (l) => l.income_class === "non_operating_income" && self(l) },
  gross_compensation: {
    basis: "entry",
    select: (l) => employee(l) && l.comp_component !== null && COMPENSATION_COMPONENTS.has(l.comp_component),
  },
  basic_pay: { basis: "entry", select: component("basic_pay") },
  mwe_basic_pay: { basis: "entry", select: component("statutory_minimum_wage") },
  holiday_ot_nsd_hazard_pay: { basis: "entry", select: component("holiday_ot_nsd_hazard") },
  thirteenth_month_and_benefits_paid: { basis: "entry", select: component("thirteenth_month_and_benefits") },
  de_minimis_paid: { basis: "entry", select: component("de_minimis") },
  employee_mandatory_contributions: {
    basis: "entry",
    select: (l) => l.comp_component === "contribution" && l.contribution_side === "employee",
    breakdownBy: (l) => l.contribution_scheme ?? "unspecified",
    exclude: (l) =>
      l.comp_component === "contribution" && l.contribution_side === "employer" ? "EXC.EMPLOYER_SHARE_NOT_19" : null,
  },
  employer_mandatory_contributions: {
    basis: "entry",
    select: (l) => l.comp_component === "contribution" && l.contribution_side === "employer" && l.measure_side === "debit",
    breakdownBy: (l) => l.contribution_scheme ?? "unspecified",
  },
  tax_withheld_from_employees: { basis: "entry", select: (l) => l.comp_component === "tax_withheld" },
  pt_withheld_2307_received: { basis: "certificate", select: certificate("pt"), exclude: certificateExclusion("pt") },
  it_withheld_2307_received: { basis: "certificate", select: certificate("it"), exclude: certificateExclusion("it") },
};

export const IMPLEMENTED_MEASURES: readonly string[] = Object.keys(SPECS);

function returnEventReason(l: PostedLine, window: Window): string | null {
  if (l.income_class === "contra_sales") return "contra_sales";
  if (l.document_kind === "credit_note") return "credit_note";
  if (l.reverses_entry_id !== null && !inRange(l.original_entry_date, window.start, window.end)) return "later_period_reversal";
  return null;
}

/** Computes one measure over one window. Throws for a measure the PoC does not implement (fail closed). */
export function computeMeasure(id: string, ledger: Ledger, window: Window): MeasureResult {
  const spec = SPECS[id];
  if (!spec) throw new LedgerError("measure_not_implemented", `${id} is not implemented in the PoC`);
  const dateOf = (l: PostedLine): string | null => (spec.basis === "entry" ? l.entry_date : l.period_end);
  const inWindow = ledger.lines.filter((l) => inRange(dateOf(l), window.start, window.end));
  const selected = inWindow.filter(spec.select);
  const amountOf = spec.amount ?? ((l: PostedLine) => l.measure);
  const value = selected.reduce((total, l) => total + amountOf(l), 0n);

  // Same-window voids net to zero: both sides are left out of lineage, and the value is unchanged.
  const netted = spec.basis === "entry" ? sameWindowReversalEntries(selected, window.start, window.end) : new Set<string>();
  const kept = selected.filter((l) => !netted.has(l.entry_id));
  const keptValue = kept.reduce((total, l) => total + amountOf(l), 0n);
  if (keptValue !== value) {
    throw new LedgerError("netting_mismatch", `${id}: netted lines do not sum to zero; a reversal is not an exact mirror`);
  }
  const nettedPairs = new Set(selected.filter((l) => l.reverses_entry_id !== null && netted.has(l.entry_id)).map((l) => l.entry_id)).size;

  let breakdown: Record<string, bigint> | null = null;
  if (spec.breakdownBy) {
    breakdown = {};
    for (const l of selected) {
      const key = spec.breakdownBy(l);
      breakdown[key] = (breakdown[key] ?? 0n) + amountOf(l);
    }
  }

  const excluded: Excluded[] = [];
  if (spec.exclude) {
    for (const l of inWindow) {
      if (selected.includes(l)) continue;
      const code = spec.exclude(l);
      if (code) excluded.push({ code, line_id: l.line_id });
    }
  }

  return {
    id,
    window,
    value,
    lines: kept.map((line) => ({ line, amount: amountOf(line) })),
    netted_pairs: nettedPairs,
    breakdown,
    excluded,
    return_events: spec.salesFamily ? selected.filter((l) => returnEventReason(l, window) !== null) : [],
  };
}
