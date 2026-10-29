import { readFileSync } from "node:fs";
import { join } from "node:path";
import { normalizeEntry, normalizeFixture, loadData, loadFixture, type LoadedData } from "../src/load.ts";
import { SYNTHETIC_DIR } from "../src/paths.ts";
import { buildProposal, type Proposal } from "../src/proposal.ts";
import type { FormCode, FormContext, NormInput } from "../src/types.ts";

/** Shared fixtures for the suite. Loaded once per test process. */
export const data: LoadedData = loadData();

const cache = new Map<string, NormInput>();

export function fixture(name: string): NormInput {
  let input = cache.get(name);
  if (!input) {
    input = loadFixture(name, data.vocab);
    cache.set(name, input);
  }
  return structuredClone(input);
}

export function rawFixture(name: string): any {
  return JSON.parse(readFileSync(join(SYNTHETIC_DIR, `${name}.json`), "utf8"));
}

export const PROFILE_A = "6b1d2f3a-4c5e-4f60-8a71-92b3c4d5e6f7";
export const PROFILE_B = "c0a8e3f1-2b7d-4e9a-9f10-5d6c7b8a9e0f";

/** Forms contexts the bridge reads (never decides). */
export const GRADUATED_OSD: FormContext = { "/taxpayer/tax_option": "graduated", "/taxpayer/deduction": "osd" };
export const GRADUATED_ITEMIZED: FormContext = { "/taxpayer/tax_option": "graduated", "/taxpayer/deduction": "itemized" };
export const EIGHT_PERCENT: FormContext = { "/taxpayer/tax_option": "eight_percent" };

/** The default Forms context per form: none for 2551Q and 1601-C, graduated + OSD for 1701-MS. */
export function defaultContext(form: FormCode): FormContext {
  return form === "1701MS" ? GRADUATED_OSD : {};
}

export function proposal(input: NormInput, form: FormCode, period: string, formContext?: FormContext, profileId?: string): Proposal {
  return buildProposal(data, input, form, period, formContext ?? defaultContext(form), profileId);
}

export function clone<T>(value: T): T {
  return structuredClone(value);
}

/** Sum of decimal-text values as a decimal string (two places), without floats. */
export function sumText(values: string[]): string {
  const cents = values.reduce((total, text) => {
    const negative = text.startsWith("-");
    const unsigned = negative ? text.slice(1) : text;
    const [whole, frac] = unsigned.split(".");
    const magnitude = BigInt(whole) * 100n + BigInt(frac);
    return total + (negative ? -magnitude : magnitude);
  }, 0n);
  const negative = cents < 0n;
  const magnitude = negative ? -cents : cents;
  return `${negative ? "-" : ""}${magnitude / 100n}.${(magnitude % 100n).toString().padStart(2, "0")}`;
}

export type LineageEntry = { line_id: string; entry_id: string; amount: string };
export function lineageOf(p: Proposal, key: string): { lines: LineageEntry[]; netted_pairs: number; mapping_id: string; measures: string[] } {
  return p.lineage[key] as { lines: LineageEntry[]; netted_pairs: number; mapping_id: string; measures: string[] };
}

// ---- in-memory inputs for scenarios not worth a fixture file -------------------------------------------------------

type Obj = Record<string, unknown>;

export function memoryInput(opts: { profileId?: string; periods?: Obj; acks?: string[]; entries: Obj[]; asOf?: string }): NormInput {
  const profileId = opts.profileId ?? PROFILE_A;
  const raw = {
    fixture_format: "buwiz-poc-fixture/2",
    fixture_id: "in-memory",
    as_of: opts.asOf ?? "2026-10-29",
    profiles: [
      {
        tax_profile_id: profileId,
        display: "***-***-***-00000",
        periods: opts.periods ?? { "2026-Q3": "closed", "2026-09": "closed", "2026-12": "closed", "2026": "closed" },
      },
    ],
    acknowledged_entry_ids: opts.acks ?? [],
    entries: opts.entries,
  };
  return normalizeFixture(raw, data.vocab);
}

/** A plain sale: receivable debit, sales credit. */
export function saleEntry(profileId: string, entryId: string, date: string, amount: string): Obj {
  return {
    entry_id: entryId,
    tax_profile_id: profileId,
    entry_date: date,
    memo: `in-memory sale ${entryId}`,
    lines: [
      { line_id: `${entryId}.1`, account: "accounts_receivable_gross", debit: amount, credit: "0.00" },
      {
        line_id: `${entryId}.2`,
        account: "sales_revenue",
        debit: "0.00",
        credit: amount,
        measure_side: "credit",
        income_class: "business_sales",
        document_kind: "sale_invoice",
      },
    ],
  };
}

/** A contra-sales debit (return by default), acknowledged by entry id. */
export function creditNoteEntry(profileId: string, entryId: string, date: string, amount: string, contraKind = "return"): Obj {
  return {
    entry_id: entryId,
    tax_profile_id: profileId,
    entry_date: date,
    memo: `in-memory credit note ${entryId}`,
    lines: [
      {
        line_id: `${entryId}.1`,
        account: "refunds_and_discounts",
        debit: amount,
        credit: "0.00",
        measure_side: "credit",
        income_class: "contra_sales",
        contra_kind: contraKind,
        document_kind: "credit_note",
      },
      { line_id: `${entryId}.2`, account: "accounts_receivable_gross", debit: "0.00", credit: amount },
    ],
  };
}

/** Certificate posting: receivable debit with the certificate's evidence attributes. */
export function certEntry(
  profileId: string,
  entryId: string,
  date: string,
  amount: string,
  opts: { atc: string; creditKind: "pt" | "it"; status: string; periodStart: string; periodEnd: string; documentKind?: string },
): Obj {
  const account = opts.creditKind === "pt" ? "pct_tax_credit_receivable" : "ph_cwt_receivable";
  return {
    entry_id: entryId,
    tax_profile_id: profileId,
    entry_date: date,
    memo: `in-memory certificate ${entryId}`,
    lines: [
      {
        line_id: `${entryId}.1`,
        account,
        debit: amount,
        credit: "0.00",
        measure_side: "debit",
        certificate_atc: opts.atc,
        credit_kind: opts.creditKind,
        certificate_status: opts.status,
        period_start: opts.periodStart,
        period_end: opts.periodEnd,
        document_kind: opts.documentKind ?? "certificate_2307",
      },
      { line_id: `${entryId}.2`, account: "accounts_receivable_gross", debit: "0.00", credit: amount },
    ],
  };
}

/** One employee's 13th-month payment (balanced against net pay). */
export function thirteenthEntry(profileId: string, entryId: string, date: string, amount: string, employee = "EMP-1"): Obj {
  return {
    entry_id: entryId,
    tax_profile_id: profileId,
    entry_date: date,
    lines: [
      {
        line_id: `${entryId}.1`,
        account: "employee_benefits",
        debit: amount,
        credit: "0.00",
        measure_side: "debit",
        comp_component: "thirteenth_month_and_benefits",
        person_scope: "employee",
        party_ref: employee,
        document_kind: "payroll_register",
      },
      { line_id: `${entryId}.2`, account: "ph_net_pay_payable", debit: "0.00", credit: amount },
    ],
  };
}

/** Reverses an entry: mirrored sides, same attributes, dated as given. */
export function reversalEntry(entryId: string, original: Obj, date: string): Obj {
  return {
    entry_id: entryId,
    tax_profile_id: original.tax_profile_id,
    entry_date: date,
    memo: `in-memory void of ${String(original.entry_id)}`,
    reverses_entry_id: original.entry_id,
    lines: (original.lines as Obj[]).map((line, index) => ({
      ...line,
      line_id: `${entryId}.${index + 1}`,
      debit: line.credit,
      credit: line.debit,
    })),
  };
}

/** Normalizes a raw entry the same way the loader does, for tests that append to a normalized input. */
export function normalized(profileId: string, raw: Obj): NormInput["entries"][number] {
  return normalizeEntry(raw, new Set([profileId]), data.vocab);
}
