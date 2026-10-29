export type FormCode = "2551Q" | "1601C" | "1701MS";

export const FORM_CODES: readonly FormCode[] = ["2551Q", "1601C", "1701MS"];

/** Filing period kind per form: 2551Q quarterly, 1601-C monthly, 1701-MS annual. */
export const FORM_PERIOD_KIND: Record<FormCode, "quarter" | "month" | "year"> = {
  "2551Q": "quarter",
  "1601C": "month",
  "1701MS": "year",
};

/**
 * One journal line after normalization. Attributes are accounting and evidence facts only (ADR-0007):
 * no tax type, ATC selection, treatment, regime or election. Amounts stay decimal text until the ledger parses them.
 */
export interface NormLine {
  line_id: string;
  account: string;
  debit: string;
  credit: string;
  measure_side: "debit" | "credit" | null;
  income_class: string;
  contra_kind: string | null;
  document_kind: string;
  person_scope: string;
  party_ref: string | null;
  comp_component: string | null;
  contribution_side: string;
  contribution_scheme: string | null;
  credit_kind: string;
  certificate_atc: string | null;
  certificate_status: string | null;
  period_start: string | null;
  period_end: string | null;
  final_tax_withheld: string;
}

export interface NormEntry {
  entry_id: string;
  tax_profile_id: string;
  entry_date: string;
  reverses_entry_id: string | null;
  lines: NormLine[];
}

/**
 * When the book recognizes income and expenses (ADR 0008). An accounting policy of the business profile, not a tax
 * election. "cash" (the default): income is posted when the money is received and expenses when they are paid, so an
 * entry's date is the day money moved. "accrual": income is posted at invoice and expenses at bill.
 */
export type AccountingBasis = "cash" | "accrual";

export const ACCOUNTING_BASES: readonly AccountingBasis[] = ["cash", "accrual"];

/** The basis a profile gets when none is chosen (owner decision, 2026-10-29). */
export const DEFAULT_ACCOUNTING_BASIS: AccountingBasis = "cash";

/** A business profile as Books sees it: an opaque link id, its accounting basis and period statuses. No tax facts, no elections. */
export interface NormProfile {
  tax_profile_id: string;
  accounting_basis: AccountingBasis;
  /** Period key to "open" or "closed". */
  periods: Record<string, string>;
}

/** The normalized input. Memos and labels are dropped here, so they cannot reach a hash or a proposal. */
export interface NormInput {
  input_format: "buwiz-poc-input/3";
  as_of: string;
  profiles: NormProfile[];
  acknowledged_entry_ids: string[];
  entries: NormEntry[];
}

/**
 * Values the bridge READS from the Forms draft or profile (for example /taxpayer/tax_option). Books never
 * decides, infers or defaults them (BRIDGE.FORM_CONTEXT_READ_ONLY).
 */
export type FormContext = Record<string, string>;
