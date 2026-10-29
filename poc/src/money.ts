/**
 * Money is integer centavos (bigint). JSON carries decimal text with exactly two places.
 * No JS number, float or parseFloat touches a money value. Rounding is half away from zero, once,
 * at output (decision D7; research: rates.v1.json rounding.pt_example).
 */

export class MoneyError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "MoneyError";
  }
}

const MONEY_TEXT = /^-?(0|[1-9][0-9]*)\.[0-9]{2}$/;
const I64_LIMIT = 9_223_372_036_854_775_807n;

export function parseMoney(text: unknown, label = "amount"): bigint {
  if (typeof text !== "string") {
    throw new MoneyError(`${label}: money must be decimal text, got a JS ${typeof text}`);
  }
  if (!MONEY_TEXT.test(text) || text === "-0.00") {
    throw new MoneyError(`${label}: "${text}" is not decimal text with exactly two places`);
  }
  const negative = text.startsWith("-");
  const [whole, cents] = (negative ? text.slice(1) : text).split(".");
  const magnitude = BigInt(whole) * 100n + BigInt(cents);
  const value = negative ? -magnitude : magnitude;
  if (value > I64_LIMIT || value < -I64_LIMIT) throw new MoneyError(`${label}: outside i64 range`);
  return value;
}

export function formatMoney(value: bigint): string {
  const negative = value < 0n;
  const magnitude = negative ? -value : value;
  const cents = (magnitude % 100n).toString().padStart(2, "0");
  return `${negative ? "-" : ""}${magnitude / 100n}.${cents}`;
}

/** round(numerator / denominator), halves away from zero. */
export function divRoundHalfAway(numerator: bigint, denominator: bigint): bigint {
  if (denominator <= 0n) throw new MoneyError("denominator must be positive");
  const negative = numerator < 0n;
  const magnitude = negative ? -numerator : numerator;
  let quotient = magnitude / denominator;
  const remainder = magnitude % denominator;
  if (2n * remainder >= denominator) quotient += 1n;
  return negative ? -quotient : quotient;
}

/** Tax of a centavo base at an integer basis-point rate (300 = 3.00%). */
export function percentOf(baseCentavos: bigint, rateBps: number): bigint {
  return divRoundHalfAway(baseCentavos * BigInt(rateBps), 10_000n);
}
