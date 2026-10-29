/** Calendar dates are ISO days (Asia/Manila). Period keys: 2026-Q3, 2026-09, 2026. Due dates are Forms' (ADR-0007). */

export type PeriodKind = "quarter" | "month" | "year";

export interface PeriodRange {
  key: string;
  kind: PeriodKind;
  year: number;
  start: string;
  end: string;
}

const DATE_TEXT = /^(\d{4})-(\d{2})-(\d{2})$/;
const pad2 = (n: number): string => String(n).padStart(2, "0");

export function isValidDate(text: unknown): text is string {
  if (typeof text !== "string") return false;
  const match = DATE_TEXT.exec(text);
  if (!match) return false;
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const probe = new Date(Date.UTC(year, month - 1, day));
  return probe.getUTCFullYear() === year && probe.getUTCMonth() === month - 1 && probe.getUTCDate() === day;
}

export function addDays(date: string, days: number): string {
  const [year, month, day] = date.split("-").map(Number);
  return new Date(Date.UTC(year, month - 1, day + days)).toISOString().slice(0, 10);
}

export function monthEnd(year: number, month: number): string {
  const last = new Date(Date.UTC(year, month, 0)).getUTCDate();
  return `${year}-${pad2(month)}-${pad2(last)}`;
}

export function parsePeriod(key: string): PeriodRange {
  let match = /^(\d{4})-Q([1-4])$/.exec(key);
  if (match) {
    const year = Number(match[1]);
    const firstMonth = (Number(match[2]) - 1) * 3 + 1;
    return { key, kind: "quarter", year, start: `${year}-${pad2(firstMonth)}-01`, end: monthEnd(year, firstMonth + 2) };
  }
  match = /^(\d{4})-(\d{2})$/.exec(key);
  if (match) {
    const year = Number(match[1]);
    const month = Number(match[2]);
    if (month < 1 || month > 12) throw new Error(`bad period key ${key}`);
    return { key, kind: "month", year, start: `${year}-${pad2(month)}-01`, end: monthEnd(year, month) };
  }
  match = /^(\d{4})$/.exec(key);
  if (match) {
    const year = Number(match[1]);
    return { key, kind: "year", year, start: `${year}-01-01`, end: `${year}-12-31` };
  }
  throw new Error(`bad period key ${key}`);
}

export function periodKeyOf(date: string, kind: PeriodKind): string {
  const [year, month] = date.split("-").map(Number);
  if (kind === "quarter") return `${year}-Q${Math.floor((month - 1) / 3) + 1}`;
  if (kind === "month") return `${year}-${pad2(month)}`;
  return String(year);
}

export function inRange(date: string | null, start: string, end: string): boolean {
  return date !== null && date >= start && date <= end;
}

/** Calendar day in Asia/Manila for an instant such as 2026-09-30T23:30:00+08:00. */
export function manilaDay(instant: string): string {
  return new Intl.DateTimeFormat("en-CA", {
    timeZone: "Asia/Manila",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).format(new Date(instant));
}
