import { addDays, inRange, type PeriodRange } from "./dates.ts";
import type { Ledger, PostedLine } from "./ledger.ts";
import type { LoadedData, Mapping, RuleSet } from "./load.ts";
import { COMPENSATION_COMPONENTS, computeMeasure, type MeasureResult, type Window } from "./measures.ts";
import { formatMoney, parseMoney } from "./money.ts";
import type { FormCode, FormContext, NormProfile } from "./types.ts";

/**
 * The bridge: measures to Forms input boxes, with lineage. It reads Forms-provided choices (form_context) to pick
 * between alternative boxes, and never decides them. It holds no rate, election, threshold, applicability test,
 * ATC choice, due date or penalty (ADR-0007). The one labelled input-level tax rule is the 1601-C item 17 cap.
 */

export interface Item {
  code: string;
  detail: string;
  ids?: string[];
}

export interface Lineage {
  line_id: string;
  entry_id: string;
  amount: string;
}

export interface Val {
  key: string;
  value: string;
  mapping_id: string;
  mode: "fill" | "reference";
  measures: string[];
  lines: Lineage[];
  netted_pairs: number;
  note?: string;
}

export interface Outcome {
  blocking: Item[];
  warnings: Item[];
  values: Val[];
  measures: Record<string, MeasureResult>;
  excludedReasons: Record<string, string>;
}

export interface BridgeCtx {
  data: LoadedData;
  form: FormCode;
  profile: NormProfile;
  ledger: Ledger;
  period: PeriodRange;
  acks: Set<string>;
  formContext: FormContext;
}

/** REV.COUNT_ONCE: a line may feed only one value of its group. */
export const COUNTED_ONCE: Record<FormCode, string[]> = {
  "2551Q": ["txtATCAmt1", "frm2551Qv2018:txt15"],
  "1601C": ["frm1601c:txtTax14", "frm1601c:txtTax19", "frm1601c:txtTax25"],
  "1701MS": [
    "frm1701MS:txtSalesRevenueNo5a",
    "frm1701MS:txtSalesReturnsNo6a",
    "frm1701MS:txtCostofSalesNo8a",
    "frm1701MS:txtNonOperatingIncomeNo13a",
    "frm1701MS:txtSalesRevenuesFeesANo19a",
    "frm1701MS:txtOtherNonOperatingIncomeNo20a",
  ],
};

const compareText = (a: string, b: string): number => (a < b ? -1 : a > b ? 1 : 0);
const uniqueSorted = (values: string[]): string[] => [...new Set(values)].sort(compareText);

function push(list: Item[], code: string, detail: string, ids?: string[]): void {
  list.push(ids && ids.length ? { code, detail, ids: uniqueSorted(ids) } : { code, detail });
}

function lineage(rows: Array<{ line: PostedLine; amount: bigint }>): Lineage[] {
  return rows
    .map(({ line, amount }) => ({ line_id: line.line_id, entry_id: line.entry_id, amount: formatMoney(amount) }))
    .sort((a, b) => compareText(a.line_id, b.line_id));
}

/** Validates the Forms context against the rule set's declared inputs. Returns blocking items. */
export function checkFormContext(rs: RuleSet, ctx: FormContext): Item[] {
  const items: Item[] = [];
  const declared = new Set(rs.form_context_inputs.map((i) => i.path));
  for (const key of Object.keys(ctx)) {
    if (!declared.has(key)) items.push({ code: "form_context_undeclared", detail: `${key} is not a Forms input this rule set reads` });
  }
  for (const input of rs.form_context_inputs) {
    const needed =
      input.required === true ||
      (input.required_when !== undefined &&
        Object.entries(input.required_when).every(([path, value]) => ctx[path] === value));
    const value = ctx[input.path];
    if (value === undefined) {
      if (needed) {
        items.push({
          code: "form_context_missing",
          detail: `${input.path} must be read from the Forms draft (${input.source}); the bridge never decides or defaults it`,
        });
      }
      continue;
    }
    if (!input.supported_values.includes(value)) {
      items.push({ code: "form_context_unsupported", detail: `${input.path} is ${value}; this rule set maps ${input.supported_values.join(", ")}` });
    }
  }
  return items;
}

function selected(m: Mapping, ctx: FormContext): boolean {
  return (m.form_context ?? []).every((c) => ctx[c.read] === c.equals);
}

function windowFor(m: Mapping, period: PeriodRange): Window {
  if (m.sub_window === "Q1-Q3") return { start: period.start, end: `${period.year}-09-30` };
  if (m.sub_window === "Q4") return { start: `${period.year}-10-01`, end: period.end };
  return { start: period.start, end: period.end };
}

const measureKey = (id: string, m: Mapping): string => (m.sub_window ? `${id}[${m.sub_window}]` : id);

/**
 * 1601C.II.17 for one employee: min(paid in the month, max(0, cap - paid earlier in the year)). The one input-level
 * tax rule in the bridge (see data/rules/1601c-v2018.rules.json and research/handoff-to-forms/compliance-notes.md).
 */
export function nontaxableThirteenthCentavos(paid: bigint, paidEarlierInYear: bigint, cap: bigint): bigint {
  const room = cap - paidEarlierInYear;
  const allowed = room > 0n ? room : 0n;
  return paid < allowed ? paid : allowed;
}

/** 1601C.II.17 summed over employees (grouped by party_ref). */
function nontaxable13th(m: Mapping, month: MeasureResult, ytd: MeasureResult, o: Outcome): bigint | null {
  const cap = parseMoney(m.aggregation_params?.annual_cap_text, `${m.id}.annual_cap_text`);
  const missing = [...month.lines, ...ytd.lines].filter((r) => r.line.party_ref === null).map((r) => r.line.line_id);
  if (missing.length) {
    push(o.blocking, "employee_ref_missing", `${m.id}: 13th-month lines need the employee's party_ref so the annual cap is applied per employee`, missing);
    return null;
  }
  const sumBy = (rows: Array<{ line: PostedLine; amount: bigint }>): Map<string, bigint> => {
    const out = new Map<string, bigint>();
    for (const { line, amount } of rows) out.set(line.party_ref!, (out.get(line.party_ref!) ?? 0n) + amount);
    return out;
  };
  const paid = sumBy(month.lines);
  const before = sumBy(ytd.lines);
  let total = 0n;
  for (const [who, amount] of paid) total += nontaxableThirteenthCentavos(amount, before.get(who) ?? 0n, cap);
  return total;
}

export function runBridge(ctx: BridgeCtx): Outcome {
  const { data, form, profile, ledger, period, acks, formContext } = ctx;
  const rs = data.rules[form];
  const o: Outcome = { blocking: [], warnings: [], values: [], measures: {}, excludedReasons: {} };

  // GATE.PERIOD_LOCK: measures are proposed from closed periods only.
  const status = profile.periods[period.key];
  if (status === undefined) push(o.blocking, "period_unknown", `no status recorded for ${period.key}`);
  else if (status !== "closed") push(o.blocking, "GATE.PERIOD_LOCK", `${period.key} is ${status}; proposals need a closed period`);

  // BRIDGE.FORM_CONTEXT_READ_ONLY: the Forms choices are inputs; missing or unsupported values block.
  o.blocking.push(...checkFormContext(rs, formContext));

  const fills = rs.mappings.filter((m) => m.mode === "fill" || m.mode === "reference");
  const active = fills.filter((m) => selected(m, formContext));
  for (const m of fills) {
    if (active.includes(m)) continue;
    const why = (m.form_context ?? []).map((c) => `${c.read} is ${formContext[c.read] ?? "unset"} (mapping needs ${c.equals})`).join("; ");
    o.excludedReasons[m.target_key] = `not selected by the Forms context: ${why}`;
  }

  const inPeriod = ledger.lines.filter((l) => inRange(l.entry_date, period.start, period.end));

  // EXC.SPOUSE_MVP: spouse lines are measured but the bridge refuses to map them in MVP.
  if (active.some((m) => m.person_scope === "taxpayer")) {
    const spouse = inPeriod.filter((l) => l.person_scope === "spouse" && l.measure !== 0n);
    if (spouse.length) push(o.blocking, "EXC.SPOUSE_MVP", "spouse-scoped lines are refused in MVP (phase P7)", spouse.map((l) => l.line_id));
  }

  // Payroll completeness: Books cannot tell an unrecorded payroll from zero payroll, so it proposes nothing.
  if (active.some((m) => m.person_scope === "employee")) {
    const payroll = inPeriod.filter((l) => l.comp_component !== null);
    if (payroll.length === 0) {
      push(o.blocking, "no_payroll_lines", `no payroll lines are posted in ${period.key}; Books does not assert zero payroll`);
    }
    const stray = inPeriod.filter((l) => l.comp_component !== null && COMPENSATION_COMPONENTS.has(l.comp_component) && l.person_scope !== "employee");
    if (stray.length) {
      push(o.blocking, "comp_scope_not_employee", "compensation lines must carry person_scope employee; a stray line would be dropped silently", stray.map((l) => l.line_id));
    }
  }

  const cache = new Map<string, MeasureResult>();
  const measure = (id: string, window: Window): MeasureResult => {
    const key = `${id}@${window.start}..${window.end}`;
    let found = cache.get(key);
    if (!found) {
      found = computeMeasure(id, ledger, window);
      cache.set(key, found);
    }
    return found;
  };

  const excludedByCode = new Map<string, Set<string>>();
  for (const m of active) {
    const window = windowFor(m, period);
    const results = m.measures.map((id) => measure(id, window));
    results.forEach((r, i) => (o.measures[measureKey(m.measures[i], m)] = r));

    for (const r of results) {
      for (const e of r.excluded) {
        if (!excludedByCode.has(e.code)) excludedByCode.set(e.code, new Set());
        excludedByCode.get(e.code)!.add(e.line_id);
      }
    }

    const events = results.flatMap((r) => r.return_events);
    const unacked = uniqueSorted(events.filter((l) => !acks.has(l.entry_id)).map((l) => l.entry_id));
    if (unacked.length) {
      push(o.blocking, "GATE.RETURNS_ACK", `${m.id}: return events (returns, allowances, discounts, credit notes, later-period reversals) need a person's acknowledgement`, unacked);
    }

    if (m.review.status === "open") push(o.warnings, "MAPPING_REVIEW_OPEN", `${m.id}: ${m.review.note}`);
    if (m.input_level_tax_rule) push(o.warnings, "INPUT_LEVEL_TAX_RULE", `${m.id}: ${m.input_level_tax_rule}`);

    const rows = results.flatMap((r) => r.lines);
    if (m.poc === "fail_closed") {
      if (rows.length) {
        push(o.blocking, m.fail_closed_code ?? "mapping_open", `${m.id}: ${m.review.note}`, rows.map((r) => r.line.line_id));
      } else {
        o.excludedReasons[m.target_key] = `no source lines; mapping open for review (${m.fail_closed_code ?? "mapping_open"})`;
      }
      continue;
    }

    const sum = results.reduce((total, r) => total + r.value, 0n);
    let value: string;
    let note: string | undefined;
    if (m.aggregation === "sum") {
      value = formatMoney(sum);
    } else if (m.aggregation === "choice_yes_if_nonzero") {
      value = sum !== 0n ? m.aggregation_params!.yes : m.aggregation_params!.no;
      note = `${m.aggregation_params!.yes} when ${m.measures.join(" + ")} is not zero, else ${m.aggregation_params!.no}`;
    } else {
      const ytdWindow: Window = { start: `${period.year}-01-01`, end: addDays(period.start, -1) };
      const ytd = measure(m.aggregation_params!.ytd_measure, ytdWindow);
      o.measures[`${m.aggregation_params!.ytd_measure}[ytd_before_period]`] = ytd;
      const split = nontaxable13th(m, results[0], ytd, o);
      if (split === null) continue;
      value = formatMoney(split);
      note = `non-taxable part after the annual cap of ${m.aggregation_params!.annual_cap_text} per employee; the one input-level tax rule in the bridge`;
    }
    if (m.non_negative && sum < 0n) {
      push(o.blocking, "negative_measure", `${m.id}: ${m.measures.join(" + ")} is ${formatMoney(sum)}; the bridge does not propose a negative amount for this box`);
    }
    const nettedPairs = results.reduce((total, r) => total + r.netted_pairs, 0);
    o.values.push({
      key: m.target_key,
      value,
      mapping_id: m.id,
      mode: m.mode,
      measures: m.measures,
      lines: lineage(rows),
      netted_pairs: nettedPairs,
      ...(note ? { note } : {}),
    });
  }
  for (const [code, ids] of excludedByCode) {
    push(o.warnings, code, "excluded from the measure by rule; listed, never silently dropped", [...ids]);
  }
  return o;
}
