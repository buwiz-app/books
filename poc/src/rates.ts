import { divRoundHalfAway, parseMoney } from "./money.ts";

export interface PtRateRow {
  code: string;
  rate_bps: number;
  effective_from: string;
  effective_to: string | null;
  default_on: boolean;
  disputed?: boolean;
  status: string;
  citations: string[];
}

export interface PtRateLookup {
  row: PtRateRow;
  warnings: string[];
}

/**
 * Percentage-tax row for a filing period, looked up by the period start (rates.v1 pt_rate_lookup).
 * Only default_on rows are used. A disputed row inside its window produces a warning (decision D6).
 * Returns null when no row covers the date; the caller then blocks (fail closed).
 */
export function ptRateFor(rates: any, date: string): PtRateLookup | null {
  const rows: PtRateRow[] = rates.pt_rate_rows;
  const matches = rows.filter(
    (r) => r.default_on && date >= r.effective_from && (r.effective_to === null || date <= r.effective_to),
  );
  if (matches.length > 1) throw new Error(`ambiguous PT rate rows for ${date}`);
  if (matches.length === 0) return null;
  const warnings: string[] = [];
  for (const r of rows) {
    if (!r.default_on && r.disputed && date >= r.effective_from && (r.effective_to === null || date <= r.effective_to)) {
      warnings.push(
        `PT_RATE_DISPUTED_WINDOW: ${r.code} ${r.rate_bps} bps for ${r.effective_from}..${r.effective_to} is ${r.status} and OFF; ${matches[0].rate_bps} bps applies (decision D6)`,
      );
    }
  }
  return { row: matches[0], warnings };
}

export function vatThresholdCentavos(rates: any): bigint {
  return parseMoney(rates.vat_threshold.amount_text, "vat_threshold");
}

export function osdFractionBps(rates: any): number {
  return rates.osd.fraction_bps;
}

/**
 * Graduated income tax (2023+ table, rates.v1). Brackets are (over, not_over]; the base tax plus
 * the rate on the excess over the lower bound, rounded half away from zero once at the end.
 * Non-positive bases give zero. rates.v1 test_vector: 714000.00 gives 85300.00.
 */
export function graduatedTaxCentavos(rates: any, baseCentavos: bigint): bigint {
  if (baseCentavos <= 0n) return 0n;
  for (const bracket of rates.graduated_income_tax_2023.brackets) {
    const over = parseMoney(bracket.over_text, "bracket.over");
    const notOver = bracket.not_over_text === null ? null : parseMoney(bracket.not_over_text, "bracket.not_over");
    if (baseCentavos > over && (notOver === null || baseCentavos <= notOver)) {
      const baseTax = parseMoney(bracket.base_tax_text, "bracket.base_tax");
      const excess = baseCentavos - over;
      return baseTax + divRoundHalfAway(excess * BigInt(bracket.rate_bps), 10_000n);
    }
  }
  return 0n;
}
