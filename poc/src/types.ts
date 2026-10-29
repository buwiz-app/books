export type FormCode = "2551Q" | "1601C" | "1701MS";

export const FORM_CODES: readonly FormCode[] = ["2551Q", "1601C", "1701MS"];

/** Filing period kind per form: 2551Q quarterly, 1601-C monthly, 1701-MS annual. */
export const FORM_PERIOD_KIND: Record<FormCode, "quarter" | "month" | "year"> = {
  "2551Q": "quarter",
  "1601C": "month",
  "1701MS": "year",
};

/** One journal line after normalization. Amounts stay decimal text until the ledger parses them. */
export interface NormLine {
  line_id: string;
  account: string;
  debit: string;
  credit: string;
  tax_types: string[];
  treatment: string;
  tax_side: "debit" | "credit" | null;
  income_class: string;
  atc: string | null;
  document_kind: string;
  credit_kind: string;
  certificate_status: string | null;
  period_start: string | null;
  period_end: string | null;
  person_scope: string;
  comp_component: string | null;
  contribution_side: string;
}

export interface NormEntry {
  entry_id: string;
  tax_profile_id: string;
  entry_date: string;
  reverses_entry_id: string | null;
  lines: NormLine[];
}

export interface NormProfile {
  tax_profile_id: string;
  /** Per tax year: regime, vat_status, deduction_method, owner_taxable_compensation_2316. */
  facts: Record<string, Record<string, string>>;
  /** Period key to "open" or "closed". */
  periods: Record<string, string>;
  /** Per tax year: prior-filing inputs that Books never sends (for example 1701Q payments). */
  prior_filings: Record<string, Record<string, string>>;
}

/** The normalized, tax-relevant input. Memos and labels are dropped here, so they cannot reach a hash or a proposal. */
export interface NormInput {
  input_format: "buwiz-poc-input/1";
  as_of: string;
  profiles: NormProfile[];
  acknowledged_entry_ids: string[];
  entries: NormEntry[];
}
