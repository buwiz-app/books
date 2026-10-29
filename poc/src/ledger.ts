import { canonical } from "./canonical.ts";
import { inRange } from "./dates.ts";
import { formatMoney, parseMoney } from "./money.ts";
import type { NormEntry, NormLine } from "./types.ts";

export class LedgerError extends Error {
  readonly code: string;
  constructor(code: string, detail: string) {
    super(`${code}: ${detail}`);
    this.name = "LedgerError";
    this.code = code;
  }
}

/**
 * A posted line. `measure` is the line's signed effect on its tax meaning:
 * (credit - debit) when tax_side is credit, (debit - credit) when it is debit.
 * A reversal carries the mirrored sides, so its measure is the exact negative of the original.
 */
export interface PostedLine extends NormLine {
  entry_id: string;
  entry_date: string;
  reverses_entry_id: string | null;
  original_entry_date: string | null;
  dr: bigint;
  cr: bigint;
  measure: bigint;
}

export interface Ledger {
  lines: PostedLine[];
  entries: Map<string, NormEntry>;
  reversedBy: Map<string, string>;
}

const MIRROR_ATTRIBUTES = [
  "account",
  "tax_types",
  "treatment",
  "tax_side",
  "income_class",
  "atc",
  "document_kind",
  "credit_kind",
  "certificate_status",
  "period_start",
  "period_end",
  "person_scope",
  "comp_component",
  "contribution_side",
] as const;

function mirrors(original: NormLine, reversal: NormLine): boolean {
  if (!MIRROR_ATTRIBUTES.every((key) => canonical(original[key]) === canonical(reversal[key]))) return false;
  return (
    parseMoney(original.debit) === parseMoney(reversal.credit) &&
    parseMoney(original.credit) === parseMoney(reversal.debit)
  );
}

/**
 * Validates the entries and posts their lines. Throws LedgerError with a code:
 * unbalanced, line_sides, duplicate_id, orphan_reversal, reversed_twice, reversal_not_mirror,
 * reversal_before_original.
 */
export function postLedger(entries: NormEntry[]): Ledger {
  const byId = new Map<string, NormEntry>();
  const lineIds = new Set<string>();
  for (const entry of entries) {
    if (byId.has(entry.entry_id)) throw new LedgerError("duplicate_id", entry.entry_id);
    byId.set(entry.entry_id, entry);
    let debits = 0n;
    let credits = 0n;
    for (const line of entry.lines) {
      if (lineIds.has(line.line_id)) throw new LedgerError("duplicate_id", line.line_id);
      lineIds.add(line.line_id);
      const dr = parseMoney(line.debit, `${line.line_id}.debit`);
      const cr = parseMoney(line.credit, `${line.line_id}.credit`);
      if (dr < 0n || cr < 0n) throw new LedgerError("line_sides", `${line.line_id}: negative side`);
      if ((dr > 0n) === (cr > 0n)) {
        throw new LedgerError("line_sides", `${line.line_id}: exactly one of debit or credit must be non-zero`);
      }
      debits += dr;
      credits += cr;
    }
    if (debits !== credits) {
      throw new LedgerError(
        "unbalanced",
        `${entry.entry_id}: debits ${formatMoney(debits)} do not equal credits ${formatMoney(credits)}`,
      );
    }
  }

  const reversedBy = new Map<string, string>();
  for (const entry of byId.values()) {
    if (entry.reverses_entry_id === null) continue;
    const original = byId.get(entry.reverses_entry_id);
    if (!original) throw new LedgerError("orphan_reversal", `${entry.entry_id} reverses ${entry.reverses_entry_id}, which is not in scope`);
    if (reversedBy.has(original.entry_id)) {
      throw new LedgerError("reversed_twice", `${original.entry_id} is reversed by ${reversedBy.get(original.entry_id)} and ${entry.entry_id}`);
    }
    if (original.lines.length !== entry.lines.length || !original.lines.every((l, i) => mirrors(l, entry.lines[i]))) {
      throw new LedgerError("reversal_not_mirror", `${entry.entry_id} does not mirror ${original.entry_id} line by line`);
    }
    if (entry.entry_date < original.entry_date) {
      throw new LedgerError("reversal_before_original", `${entry.entry_id} is dated before ${original.entry_id}`);
    }
    reversedBy.set(original.entry_id, entry.entry_id);
  }

  const lines: PostedLine[] = [];
  for (const entry of byId.values()) {
    const original = entry.reverses_entry_id === null ? null : byId.get(entry.reverses_entry_id) ?? null;
    for (const line of entry.lines) {
      const dr = parseMoney(line.debit);
      const cr = parseMoney(line.credit);
      const measure =
        line.tax_side === null ? 0n : line.tax_side === "credit" ? cr - dr : dr - cr;
      lines.push({
        ...line,
        entry_id: entry.entry_id,
        entry_date: entry.entry_date,
        reverses_entry_id: entry.reverses_entry_id,
        original_entry_date: original ? original.entry_date : null,
        dr,
        cr,
        measure,
      });
    }
  }
  return { lines, entries: byId, reversedBy };
}

/**
 * Entry ids whose lines net to zero inside a window: a reversal dated in the window whose original is
 * also dated in the window. Both sides are excluded from lineage, and the sum is unchanged.
 */
export function sameWindowReversalEntries(lines: PostedLine[], start: string, end: string): Set<string> {
  const netted = new Set<string>();
  for (const line of lines) {
    if (
      line.reverses_entry_id !== null &&
      inRange(line.entry_date, start, end) &&
      inRange(line.original_entry_date, start, end)
    ) {
      netted.add(line.entry_id);
      netted.add(line.reverses_entry_id);
    }
  }
  return netted;
}

export function sumMeasure(lines: PostedLine[]): bigint {
  return lines.reduce((total, line) => total + line.measure, 0n);
}
