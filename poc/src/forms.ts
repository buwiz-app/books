import { dueDate, inRange, type PeriodRange } from "./dates.ts";
import { LedgerError, sameWindowReversalEntries, sumMeasure, type Ledger, type PostedLine } from "./ledger.ts";
import type { LoadedData } from "./load.ts";
import { divRoundHalfAway, formatMoney, parseMoney, percentOf } from "./money.ts";
import { graduatedTaxCentavos, osdFractionBps, ptRateFor, vatThresholdCentavos } from "./rates.ts";
import type { FormCode, NormProfile } from "./types.ts";

/**
 * The three form computations. Each returns an Outcome. The proposal assembler (proposal.ts) checks every
 * key against the crosswalk and decides what is sent. When a blocking item exists, no values are kept.
 *
 * Gate IDs in the codes come from data/rules/common.rules.json. Gates the PoC does not implement cause a
 * block, not a silent pass (fail closed). IMPLEMENTED_GATES lists what is real here.
 */

export const IMPLEMENTED_GATES: readonly string[] = [
  "GATE.VAT_THRESHOLD",
  "GATE.8PCT_YEAR_2551Q",
  "GATE.RETURNS_ACK",
  "GATE.CERT_STATUS",
  "GATE.PERIOD_LOCK",
  "EXC.VAT_NOT_IN_SALES",
  "EXC.BANK_DEPOSIT_NOT_SALES",
  "EXC.IT_CWT_NOT_PT",
  "EXC.PT_CWT_NOT_IT",
  "EXC.WB084_FINAL",
  "EXC.UNRECEIVED_CERT",
  "EXC.EMPLOYER_SHARE_NOT_19",
  "EXC.SPOUSE_MVP",
  "EXC.FINAL_TAX_INTEREST",
  "REV.VOID_OPEN_PERIOD",
  "REV.COUNT_ONCE",
  "REV.LATER_PERIOD_RETURN_EVENT",
  "REV.CREDIT_NOTE_DOCUMENT",
];

/** Rule IDs the PoC evaluates (fills, read-backs and reference values). The test suite checks that they exist. */
export const EVALUATED_RULE_IDS: Record<FormCode, string[]> = {
  "2551Q": [
    "2551Q.S1.1.ATC",
    "2551Q.S1.1.base",
    "2551Q.S1.1.tax",
    "2551Q.S1.7",
    "2551Q.II.14",
    "2551Q.II.15",
    "2551Q.II.18",
    "2551Q.II.19",
    "2551Q.II.24",
  ],
  "1601C": [
    "1601C.H.3",
    "1601C.H.5",
    "1601C.II.14",
    "1601C.II.18",
    "1601C.II.19",
    "1601C.II.21",
    "1601C.II.22",
    "1601C.II.24",
    "1601C.II.25",
    "1601C.II.27",
    "1601C.II.30",
    "1601C.II.31",
    "1601C.II.36",
  ],
  "1701MS": [
    "1701MS.IV.3",
    "1701MS.IV.5",
    "1701MS.IV.6",
    "1701MS.IV.7",
    "1701MS.IV.9",
    "1701MS.IV.11",
    "1701MS.IV.12",
    "1701MS.IV.13",
    "1701MS.IV.14",
    "1701MS.IV.15",
    "1701MS.IV.18B",
    "1701MS.II.22",
    "1701MS.II.23",
    "1701MS.II.24",
    "1701MS.II.25",
    "1701MS.II.27",
    "1701MS.II.29",
    "1701MS.V.2",
    "1701MS.V.3",
    "1701MS.V.4",
    "1701MS.V.10",
  ],
};

export interface Item {
  code: string;
  detail: string;
  ids?: string[];
}

export interface Disc {
  id: string;
  status: "needs_review";
  summary: string;
  citations: string[];
}

export interface Lineage {
  line_id: string;
  entry_id: string;
  amount: string;
}

export interface Val {
  key: string;
  value: string;
  rule_id: string;
  lines: Lineage[];
  netted_pairs: number;
  note?: string;
}

export interface Outcome {
  blocking: Item[];
  warnings: Item[];
  discrepancies: Disc[];
  fills: Val[];
  readback: Val[];
  reference: Val[];
  excludedReasons: Record<string, string>;
  due: { date: string; rule_status: string; rule: string } | null;
  rate: { code: string; rate_bps: number; effective_from: string; effective_to: string | null } | null;
}

export interface FormCtx {
  data: LoadedData;
  profile: NormProfile;
  ledger: Ledger;
  period: PeriodRange;
  acks: Set<string>;
}

const DISC_8PCT_2551Q: Disc = {
  id: "DISC-8PCT-2551Q",
  status: "needs_review",
  summary:
    "8% election year. RMO 23-2018 (bir.md 8-6; verify.md s2 #73) says 8% electors do not file 2551Q, while buwiz-oss form_2551q.rs requires NIL PT010 rows (verify.md s2 #84). The conflict is CONFIRMED; its resolution needs the RR 8-2018 text, which was not read (blocker B5; decision D5). The PoC proposes no PT010 row and no NIL row.",
  citations: [
    "research/traces/bir.md section 8-6",
    "research/traces/verify.md section 2 items 73 and 84",
    "data/rules/common.rules.json GATE.8PCT_YEAR_2551Q",
  ],
};

const DISC_OSD_BASE: Disc = {
  id: "DISC-OSD-BASE-1701MS",
  status: "needs_review",
  summary:
    "OSD base conflict: printed Part I item 17 says 40% of gross sales; Part IV item 11 says 40% of IV.7 (net sales). The PoC follows the rules file and the workbook worked example (net sales). UNCONFIRMED.",
  citations: [
    "research/traces/bir.md section 2",
    "data/rules/rates.v1.json osd",
    "research/spreadsheet-corrections.md item 38",
  ],
};

/** Exempt 13th-month and other benefits per employee per calendar year (1601C.II.17; bir.md 3.4). */
export const THIRTEENTH_MONTH_CAP = 9_000_000n; // 90,000.00 in centavos

/** Splits a 13th-month amount into the exempt part (up to the cap, net of prior exempt amounts) and the excess. */
export function thirteenthMonthSplit(amount: bigint, ytdExempt = 0n): { exempt: bigint; taxable: bigint } {
  const room = THIRTEENTH_MONTH_CAP - ytdExempt > 0n ? THIRTEENTH_MONTH_CAP - ytdExempt : 0n;
  const exempt = amount < room ? amount : room;
  return { exempt, taxable: amount - exempt };
}

const PT_WITHHOLDING_ATC = new Set(["WB080", "WB082"]); // WB084 is final (2306) and is never credited
const IT_WITHHOLDING_ATC = /^W[IC]\d{3}$/;
const UNIMPLEMENTED_COMPENSATION = new Set([
  "statutory_minimum_wage",
  "holiday_ot_nsd_hazard",
  "thirteenth_month_and_benefits",
  "other_non_taxable",
]);

export function emptyOutcome(): Outcome {
  return {
    blocking: [],
    warnings: [],
    discrepancies: [],
    fills: [],
    readback: [],
    reference: [],
    excludedReasons: {},
    due: null,
    rate: null,
  };
}

const compareText = (a: string, b: string): number => (a < b ? -1 : a > b ? 1 : 0);

function lineage(lines: PostedLine[], sign: 1n | -1n = 1n): Lineage[] {
  return lines
    .map((l) => ({ line_id: l.line_id, entry_id: l.entry_id, amount: formatMoney(sign * l.measure) }))
    .sort((a, b) => compareText(a.line_id, b.line_id));
}

const uniqueSorted = (values: string[]): string[] => [...new Set(values)].sort(compareText);

function block(o: Outcome, code: string, detail: string, ids?: string[]): void {
  o.blocking.push(ids && ids.length ? { code, detail, ids: uniqueSorted(ids) } : { code, detail });
}

function warn(o: Outcome, code: string, detail: string, ids?: string[]): void {
  o.warnings.push(ids && ids.length ? { code, detail, ids: uniqueSorted(ids) } : { code, detail });
}

/** Warning strings from rates.ts look like "CODE: detail". */
function warningFromText(o: Outcome, text: string): void {
  const cut = text.indexOf(": ");
  warn(o, text.slice(0, cut), text.slice(cut + 2));
}

/** A posted business sale (income-tax or percentage-tax family). Used for the VAT gate and the 1701-MS sales line. */
function isBusinessSale(l: PostedLine): boolean {
  return (
    l.income_class === "business_sales" &&
    (l.tax_types.includes("percentage_tax") || l.tax_types.includes("income_tax")) &&
    !l.tax_types.includes("vat") &&
    l.document_kind !== "payment_receipt"
  );
}

/** A 2551Q sales-family line: sales or sales adjustments on the PT010 row. Payments, VAT and other ATCs are excluded. */
function isPtSale(l: PostedLine): boolean {
  return (
    l.treatment === "sale_pt" &&
    l.tax_types.includes("percentage_tax") &&
    !l.tax_types.includes("vat") &&
    l.atc === "PT010" &&
    (l.income_class === "business_sales" || l.income_class === "contra_sales") &&
    l.document_kind !== "payment_receipt"
  );
}

/** Why a sales-family line is a return event that must be acknowledged, or null if it is not one. */
function returnEventReason(l: PostedLine, period: PeriodRange): string | null {
  if (l.income_class === "contra_sales") return "contra_sales";
  if (l.document_kind === "credit_note") return "credit_note";
  if (l.reverses_entry_id !== null && !inRange(l.original_entry_date, period.start, period.end)) {
    return "later_period_reversal";
  }
  return null;
}

function gatePeriod(ctx: FormCtx, o: Outcome): void {
  const status = ctx.profile.periods[ctx.period.key];
  if (status === undefined) block(o, "period_unknown", `no status recorded for ${ctx.period.key}`);
  else if (status !== "closed") {
    block(o, "GATE.PERIOD_LOCK", `${ctx.period.key} is ${status}; proposals need a closed period`);
  }
}

function gateVat(ctx: FormCtx, o: Outcome, yearStart: string, end: string): void {
  const ytd = sumMeasure(ctx.ledger.lines.filter((l) => isBusinessSale(l) && inRange(l.entry_date, yearStart, end)));
  const threshold = vatThresholdCentavos(ctx.data.rates);
  if (ytd > threshold) {
    block(
      o,
      "GATE.VAT_THRESHOLD",
      `posted sales year-to-date ${formatMoney(ytd)} are above ${formatMoney(threshold)}; VAT status is forced (strictly greater; rates.v1 vat_threshold)`,
    );
  }
}

function gateSpouse(ctx: FormCtx, o: Outcome, start: string, end: string): void {
  const spouse = ctx.ledger.lines.filter(
    (l) => l.person_scope === "spouse" && l.tax_types.length > 0 && inRange(l.entry_date, start, end),
  );
  if (spouse.length) {
    block(o, "EXC.SPOUSE_MVP", "spouse-scoped tax lines are refused in MVP (phase P7)", spouse.map((l) => l.line_id));
  }
}

function ids(lines: PostedLine[]): string[] {
  return lines.map((l) => l.line_id);
}

// ---------------------------------------------------------------------------------------------------------------
// 2551Q: quarterly percentage tax, Section 116, PT010 row 1 (rates-v1 PT row by period start).
// ---------------------------------------------------------------------------------------------------------------

export function form2551Q(ctx: FormCtx): Outcome {
  const o = emptyOutcome();
  const { period: p, profile, ledger, data } = ctx;
  const facts = profile.facts[String(p.year)] ?? {};

  gatePeriod(ctx, o);
  if (facts.regime === "eight_percent") {
    block(o, "GATE.8PCT_YEAR_2551Q", `${p.year} is an 8% year (regime fact); no 2551Q values until decision D5`);
    o.discrepancies.push(DISC_8PCT_2551Q);
  } else if (facts.regime !== "graduated") {
    block(o, "regime_not_graduated", `regime fact is ${facts.regime ?? "unset"}`);
  }
  if (facts.vat_status !== "non_vat") block(o, "vat_status_not_non_vat", `vat_status fact is ${facts.vat_status ?? "unset"}`);
  gateVat(ctx, o, `${p.year}-01-01`, p.end);
  gateSpouse(ctx, o, p.start, p.end);

  const window = ledger.lines.filter((l) => inRange(l.entry_date, p.start, p.end));
  const unclassified = window.filter(
    (l) =>
      l.tax_types.includes("percentage_tax") &&
      (l.income_class === "business_sales" || l.income_class === "contra_sales") &&
      !isPtSale(l),
  );
  if (unclassified.length) {
    block(o, "unclassified_sales_line", "sales-class lines must carry treatment sale_pt and ATC PT010", ids(unclassified));
  }

  // Sales family: sales and sales adjustments on the PT010 row, dated in the quarter.
  const saleLines = window.filter(isPtSale);
  const returnEvents = saleLines.filter((l) => returnEventReason(l, p) !== null);
  const unacked = uniqueSorted(returnEvents.filter((l) => !ctx.acks.has(l.entry_id)).map((l) => l.entry_id));
  if (unacked.length) {
    block(
      o,
      "GATE.RETURNS_ACK",
      "sales returns, credit notes or later-period reversals in this quarter need a person's acknowledgement",
      unacked,
    );
  }
  if (returnEvents.some((l) => l.document_kind === "credit_note")) {
    warn(o, "CREDIT_NOTE_TREATMENT_UNCONFIRMED", "credit-note treatment on the 2551Q base is UNCONFIRMED (bir.md PT-8)");
  }

  // Same-quarter voids net to zero: the original and its reversal are both left out of lineage.
  const netted = sameWindowReversalEntries(saleLines, p.start, p.end);
  const included = saleLines.filter((l) => !netted.has(l.entry_id));
  const base = sumMeasure(saleLines);
  if (sumMeasure(included) !== base) {
    throw new LedgerError("netting_mismatch", "netted lines do not sum to zero; the reversal is not an exact mirror");
  }
  const nettedPairs = new Set(
    saleLines.filter((l) => l.reverses_entry_id !== null && netted.has(l.entry_id)).map((l) => l.entry_id),
  ).size;
  if (base < 0n) block(o, "negative_base", `the PT base ${formatMoney(base)} is negative; no negative percentage tax (DS-J)`);

  const rate = ptRateFor(data.rates, p.start);
  if (!rate) block(o, "pt_rate_row_missing", `no PT rate row covers ${p.start}`);
  for (const text of rate?.warnings ?? []) warningFromText(o, text);
  if (rate) {
    o.rate = {
      code: rate.row.code,
      rate_bps: rate.row.rate_bps,
      effective_from: rate.row.effective_from,
      effective_to: rate.row.effective_to,
    };
  }

  // Percentage-tax certificates: only received WB080/WB082 with a period end in the quarter credit item 15.
  const certWindow = ledger.lines.filter((l) => l.treatment === "cwt_credit" && inRange(l.period_end, p.start, p.end));
  const credited = certWindow.filter(
    (l) => l.credit_kind === "pt" && PT_WITHHOLDING_ATC.has(l.atc ?? "") && l.certificate_status === "received",
  );
  for (const l of certWindow) {
    if (credited.includes(l)) continue;
    if (l.credit_kind !== "pt") warn(o, "EXC.IT_CWT_NOT_PT", "income-tax certificate; never a 2551Q credit", [l.line_id]);
    else if (l.atc === "WB084") warn(o, "EXC.WB084_FINAL", "final withholding (WB084); never credited", [l.line_id]);
    else if (l.certificate_status !== "received") {
      warn(o, "EXC.UNRECEIVED_CERT", `certificate status is ${l.certificate_status ?? "unset"}; no credit`, [l.line_id]);
    }
  }
  const creditTotal = sumMeasure(credited);

  const pt = rate ? percentOf(base, rate.row.rate_bps) : 0n;
  const ii14 = pt;
  const ii18 = creditTotal; // II.16 and II.17 have no source in the PoC
  const ii19 = ii14 - ii18; // signed; overpayment stays negative
  const ii24 = ii19; // II.20-II.23 penalties are never computed (D14)

  if (o.blocking.length === 0) {
    o.fills = [
      { key: "schedule_1.0.atc", value: "PT010", rule_id: "2551Q.S1.1.ATC", lines: [], netted_pairs: 0 },
      {
        key: "txtATCAmt1",
        value: formatMoney(base),
        rule_id: "2551Q.S1.1.base",
        lines: lineage(included),
        netted_pairs: nettedPairs,
      },
      {
        key: "frm2551Qv2018:txt15",
        value: formatMoney(creditTotal),
        rule_id: "2551Q.II.15",
        lines: lineage(credited),
        netted_pairs: 0,
      },
    ];
    o.readback = [
      { key: "frm2551Qv2018:txt14", value: formatMoney(ii14), rule_id: "2551Q.II.14", lines: [], netted_pairs: 0 },
      { key: "txtTotalSched1", value: formatMoney(pt), rule_id: "2551Q.S1.7", lines: [], netted_pairs: 0 },
      {
        key: "txtATCDue1",
        value: formatMoney(pt),
        rule_id: "2551Q.S1.1.tax",
        lines: [],
        netted_pairs: 0,
        note: "row 1 tax due: base x rate, half away from zero, once (D7)",
      },
      { key: "frm2551Qv2018:txt18", value: formatMoney(ii18), rule_id: "2551Q.II.18", lines: [], netted_pairs: 0 },
      {
        key: "frm2551Qv2018:txt19",
        value: formatMoney(ii19),
        rule_id: "2551Q.II.19",
        lines: [],
        netted_pairs: 0,
        note: "signed; an overpayment is negative and never clamped",
      },
      { key: "frm2551Qv2018:txt24", value: formatMoney(ii24), rule_id: "2551Q.II.24", lines: [], netted_pairs: 0 },
    ];
  }
  o.due = dueDate("2551Q", p);
  return o;
}

// ---------------------------------------------------------------------------------------------------------------
// 1601-C: monthly withholding on compensation (payroll path, RR 11-2018 as cited in the rules file).
// ---------------------------------------------------------------------------------------------------------------

export function form1601C(ctx: FormCtx): Outcome {
  const o = emptyOutcome();
  const { period: p, ledger } = ctx;

  gatePeriod(ctx, o);
  const window = ledger.lines.filter((l) => inRange(l.entry_date, p.start, p.end));
  const compensation = window.filter((l) => l.treatment === "comp_income" && l.person_scope === "employee");
  const contributions = window.filter((l) => l.treatment === "contribution");
  const withheld = window.filter(
    (l) =>
      l.tax_types.includes("compensation_withholding") &&
      l.atc === "WW010" &&
      l.treatment === "none" &&
      l.tax_side === "credit",
  );
  if (compensation.length === 0 && withheld.length === 0) {
    block(o, "no_payroll_lines", `no employee compensation or WW010 withholding is posted in ${p.key}; zero payroll is not confirmed`);
  }
  const strayScope = window.filter((l) => l.treatment === "comp_income" && l.person_scope !== "employee");
  if (strayScope.length) {
    block(o, "comp_scope_not_employee", "compensation lines must carry person_scope employee; a stray line would be dropped silently", ids(strayScope));
  }
  const unmapped = window.filter((l) => l.comp_component !== null && UNIMPLEMENTED_COMPENSATION.has(l.comp_component));
  if (unmapped.length) {
    block(
      o,
      "comp_component_not_implemented",
      "II.15-II.17 and II.20 need MWE flags or a YTD cap, which the PoC does not implement",
      ids(unmapped),
    );
  }
  const employerShares = window.filter((l) => l.contribution_side === "employer");
  if (employerShares.length) {
    warn(
      o,
      "EXC.EMPLOYER_SHARE_NOT_19",
      "employer contributions never go to II.19 and never reduce taxable compensation",
      ids(employerShares),
    );
  }

  const deMinimis = compensation.filter((l) => l.comp_component === "de_minimis");
  const employeeShares = contributions.filter(
    (l) =>
      l.contribution_side === "employee" &&
      (l.comp_component === "mandatory_contributions_employee" || l.comp_component === "union_dues"),
  );
  const ii14 = sumMeasure(compensation); // basic gross plus exempt benefits; no employer share
  const ii18 = sumMeasure(deMinimis);
  const ii19 = sumMeasure(employeeShares);
  const ii21 = ii18 + ii19; // II.15-II.17 and II.20 are zero (see the block above)
  const ii22 = ii14 - ii21;
  const ii24 = ii22; // II.23 is not produced (UNCONFIRMED rule, no source lines)
  const ii25 = sumMeasure(withheld);
  const ii27 = ii25; // II.26 adjustments are zero
  const ii30 = 0n; // II.28 and II.29 are zero (prior filings and evidence are never sent)
  const ii31 = ii27 - ii30;
  const ii36 = ii31; // II.32-II.35 penalties are never computed (D14)

  if (o.blocking.length === 0) {
    o.fills = [
      {
        key: "frm1601c:TaxWithheld",
        value: ii25 > 0n ? "1" : "2",
        rule_id: "1601C.H.3",
        lines: [],
        netted_pairs: 0,
        note: "option 1 = any taxes withheld (yes); option 2 = no",
      },
      { key: "frm1601c:txtATC", value: "WW010", rule_id: "1601C.H.5", lines: [], netted_pairs: 0 },
      { key: "frm1601c:txtTax14", value: formatMoney(ii14), rule_id: "1601C.II.14", lines: lineage(compensation), netted_pairs: 0 },
      { key: "frm1601c:txtTax18", value: formatMoney(ii18), rule_id: "1601C.II.18", lines: lineage(deMinimis), netted_pairs: 0 },
      { key: "frm1601c:txtTax19", value: formatMoney(ii19), rule_id: "1601C.II.19", lines: lineage(employeeShares), netted_pairs: 0 },
      { key: "frm1601c:txtTax25", value: formatMoney(ii25), rule_id: "1601C.II.25", lines: lineage(withheld), netted_pairs: 0 },
    ];
    o.readback = [
      { key: "frm1601c:txtTax21", value: formatMoney(ii21), rule_id: "1601C.II.21", lines: [], netted_pairs: 0 },
      { key: "frm1601c:txtTax22", value: formatMoney(ii22), rule_id: "1601C.II.22", lines: [], netted_pairs: 0 },
      { key: "frm1601c:txtTax24", value: formatMoney(ii24), rule_id: "1601C.II.24", lines: [], netted_pairs: 0 },
      { key: "frm1601c:txtTax27", value: formatMoney(ii27), rule_id: "1601C.II.27", lines: [], netted_pairs: 0 },
      { key: "frm1601c:txtTax30", value: formatMoney(ii30), rule_id: "1601C.II.30", lines: [], netted_pairs: 0 },
      { key: "frm1601c:txtTax31", value: formatMoney(ii31), rule_id: "1601C.II.31", lines: [], netted_pairs: 0 },
      {
        key: "frm1601c:txtTax36",
        value: formatMoney(ii36),
        rule_id: "1601C.II.36",
        lines: [],
        netted_pairs: 0,
        note: "assumes no penalty overrides: Books computes none and never fills II.32-II.34 (D14)",
      },
    ];
  }
  o.excludedReasons = {
    "frm1601c:txtTax15": "no statutory-minimum-wage lines; II.15 needs the MWE flag (not implemented)",
    "frm1601c:txtTax16": "no MWE holiday, overtime, night or hazard lines; II.16 is MWE-only (not implemented)",
    "frm1601c:txtTax17": "no 13th-month lines; II.17 needs the 90,000 YTD cap (not implemented; blocks if present)",
    "frm1601c:txtTax20": "no other non-taxable compensation lines; II.20 is not implemented (blocks if present)",
    "frm1601c:txt20Other": "no other non-taxable compensation lines; II.20 is not implemented (blocks if present)",
    "frm1601c:txtTax23": "II.23 rule is UNCONFIRMED (1601C.II.23); no source lines",
  };
  o.due = dueDate("1601C", p);
  if (o.due.rule_status === "UNCONFIRMED") {
    warn(o, "PRINTED_DUE_DATE_UNCONFIRMED", "the printed 1601-C due-date text is UNCONFIRMED (rates.v1 due_dates.1601C)");
  }
  return o;
}

// ---------------------------------------------------------------------------------------------------------------
// 1701-MS: annual income tax, graduated regime, OSD path only.
// ---------------------------------------------------------------------------------------------------------------

export function form1701MS(ctx: FormCtx): Outcome {
  const o = emptyOutcome();
  const { period: p, ledger, profile, data } = ctx;
  const year = String(p.year);
  const facts = profile.facts[year] ?? {};
  const prior = profile.prior_filings[year] ?? {};
  const yearStart = `${p.year}-01-01`;
  const q3End = `${p.year}-09-30`;

  gatePeriod(ctx, o);
  if (facts.regime === "eight_percent") {
    block(o, "eight_percent_1701ms_not_implemented", "the 8% pathway (IV.19, IV.20, IV.25) is not implemented in the PoC");
  } else if (facts.regime !== "graduated") {
    block(o, "regime_not_graduated", `regime fact is ${facts.regime ?? "unset"}`);
  }
  if (facts.deduction_method !== "osd") {
    block(o, "deduction_method_not_osd", "only the OSD path is implemented; IV.8 and IV.10A are itemized only");
  }
  if (facts.vat_status !== "non_vat") block(o, "vat_status_not_non_vat", `vat_status fact is ${facts.vat_status ?? "unset"}`);
  gateVat(ctx, o, yearStart, p.end);
  gateSpouse(ctx, o, p.start, p.end);

  const window = ledger.lines.filter((l) => inRange(l.entry_date, p.start, p.end));
  const sales = window.filter(
    (l) =>
      l.income_class === "business_sales" &&
      l.tax_types.includes("income_tax") &&
      !l.tax_types.includes("vat") &&
      l.document_kind !== "payment_receipt",
  );
  const contra = window.filter((l) => l.income_class === "contra_sales" && l.tax_types.includes("income_tax"));
  const nonOperating = window.filter((l) => l.income_class === "non_operating_income" && l.tax_types.includes("income_tax"));
  if (nonOperating.length) {
    block(
      o,
      "EXC.FINAL_TAX_INTEREST",
      "the final-tax attribute is not stored (UNKNOWN), so non-operating income cannot be split safely",
      ids(nonOperating),
    );
  }
  const unacked = uniqueSorted(contra.filter((l) => !ctx.acks.has(l.entry_id)).map((l) => l.entry_id));
  if (unacked.length) block(o, "GATE.RETURNS_ACK", "sales returns in the year need a person's acknowledgement", unacked);

  const netted = sameWindowReversalEntries(sales, p.start, p.end);
  const salesIncluded = sales.filter((l) => !netted.has(l.entry_id));
  const nettedPairs = new Set(sales.filter((l) => l.reverses_entry_id !== null && netted.has(l.entry_id)).map((l) => l.entry_id)).size;
  const iv5 = sumMeasure(sales);
  const iv6 = -sumMeasure(contra); // a return is entered as a positive amount and subtracted by the form
  if (iv6 < 0n) block(o, "negative_returns", "IV.6 would be negative");
  const iv13 = sumMeasure(nonOperating);

  const ownerFact = facts.owner_taxable_compensation_2316;
  if (ownerFact === undefined) {
    block(o, "owner_compensation_fact_missing", "IV.3 comes from the owner's 2316 (evidence, never sent); supply it as a fact");
  }
  const priorPayments = prior["1701q_q1_q3_tax_payments"];
  if (priorPayments === undefined) {
    block(o, "prior_payments_fact_missing", "V.2 (1701Q payments) is prior-filing evidence; supply it as a fact");
  }

  const iv3 = ownerFact === undefined ? 0n : parseMoney(ownerFact, "owner_taxable_compensation_2316");
  const v2 = priorPayments === undefined ? 0n : parseMoney(priorPayments, "1701q_q1_q3_tax_payments");
  const iv7 = iv5 - iv6; // the display box for net sales (verify.md s2 #15)
  const iv9 = iv7; // OSD: IV.8 is omitted
  const iv11 = divRoundHalfAway(iv7 * BigInt(osdFractionBps(data.rates)), 10_000n); // OSD base UNCONFIRMED
  const iv12 = iv9 - iv11;
  const iv14 = iv12 + iv13;
  const iv15 = iv3 + iv14;
  const iv18b = graduatedTaxCentavos(data.rates, iv15);
  const ii22 = iv18b;
  const ii19 = 0n; // IV.18A (special or exempt tax) is UNCONFIRMED; no input exists, so zero
  const ii21 = ii19;
  const ii23 = ii21 + ii22;

  // Income-tax certificates only (WI/WC series). Percentage-tax (WB) certificates never reach this form.
  const itWindow = ledger.lines.filter((l) => l.treatment === "cwt_credit" && inRange(l.period_end, yearStart, p.end));
  const itCredited = itWindow.filter(
    (l) => l.credit_kind === "it" && IT_WITHHOLDING_ATC.test(l.atc ?? "") && l.certificate_status === "received",
  );
  for (const l of itWindow) {
    if (itCredited.includes(l)) continue;
    if (l.credit_kind === "pt") warn(o, "EXC.PT_CWT_NOT_IT", "percentage-tax certificate; never an income-tax credit", [l.line_id]);
    else if (l.certificate_status !== "received") {
      warn(o, "EXC.UNRECEIVED_CERT", `certificate status is ${l.certificate_status ?? "unset"}; no credit`, [l.line_id]);
    }
  }
  const v3Lines = itCredited.filter((l) => (l.period_end ?? "") <= q3End);
  const v4Lines = itCredited.filter((l) => (l.period_end ?? "") > q3End);
  const v3 = sumMeasure(v3Lines);
  const v4 = sumMeasure(v4Lines);
  const v10 = v2 + v3 + v4; // V.1 and V.5-V.9 are zero in the PoC
  const ii24 = v10;
  const ii25 = ii23 - ii24;
  const ii27 = ii25; // II.26 installment is never chosen by Books
  const ii29 = ii27; // II.28 penalties are zero (D14)

  o.discrepancies.push(DISC_OSD_BASE);
  warn(o, "V10_SUMMATION_UNCONFIRMED", "V.10 sums V.1 to V.9 per the field tab; the printed summation is UNCONFIRMED (verify.md s2 #54)");
  warn(
    o,
    "ELECTION_BOXES_NOT_LOCKED",
    "items I.12, I.17 and I.18 have no Forms lock and are not compared with the profile election (blocker B7); Books never fills them",
  );
  warn(o, "II19_IV18A_UNCONFIRMED", "II.19 (IV.18A) is not produced; zero because no special-rate or direct-remittance input exists");

  if (o.blocking.length === 0) {
    o.fills = [
      { key: "frm1701MS:txtSalesRevenueNo5a", value: formatMoney(iv5), rule_id: "1701MS.IV.5", lines: lineage(salesIncluded), netted_pairs: nettedPairs },
      { key: "frm1701MS:txtSalesReturnsNo6a", value: formatMoney(iv6), rule_id: "1701MS.IV.6", lines: lineage(contra, -1n), netted_pairs: 0 },
      { key: "frm1701MS:txtNonOperatingIncomeNo13a", value: formatMoney(iv13), rule_id: "1701MS.IV.13", lines: lineage(nonOperating), netted_pairs: 0 },
    ];
    o.readback = [
      { key: "frm1701MS:txtTaxableCompensationIncomea", value: formatMoney(iv3), rule_id: "1701MS.IV.3", lines: [], netted_pairs: 0, note: "owner compensation, from the 2316 fact" },
      {
        key: "frm1701MS:txtTaxableCompensationIncomeNo7a",
        value: formatMoney(iv7),
        rule_id: "1701MS.IV.7",
        lines: [],
        netted_pairs: 0,
        note: "display box; the key name says compensation, the value is net sales (verify.md s2 #15)",
      },
      { key: "frm1701MS:txtGrossIncomeNo9a", value: formatMoney(iv9), rule_id: "1701MS.IV.9", lines: [], netted_pairs: 0 },
      {
        key: "frm1701MS:txtOptionalDeductionNo11a",
        value: formatMoney(iv11),
        rule_id: "1701MS.IV.11",
        lines: [],
        netted_pairs: 0,
        note: "40% of IV.7 (OSD); the base is UNCONFIRMED (rates.v1 osd)",
      },
      { key: "frm1701MS:txtNetIncomeLossNo12a", value: formatMoney(iv12), rule_id: "1701MS.IV.12", lines: [], netted_pairs: 0 },
      { key: "frm1701MS:txtTaxableIncomeBusinesNo14a", value: formatMoney(iv14), rule_id: "1701MS.IV.14", lines: [], netted_pairs: 0 },
      { key: "frm1701MS:txtCompensationBusinessNo15a", value: formatMoney(iv15), rule_id: "1701MS.IV.15", lines: [], netted_pairs: 0 },
      { key: "frm1701MS:TotalTaxDueCompensationBTP", value: formatMoney(iv18b), rule_id: "1701MS.IV.18B", lines: [], netted_pairs: 0, note: "graduated tax on IV.15 (2023+ table)" },
      { key: "frm1701MS:txtIncomeTaxRegularNo22a", value: formatMoney(ii22), rule_id: "1701MS.II.22", lines: [], netted_pairs: 0 },
      { key: "frm1701MS:txtTotalIncomeTaxDueNo23a", value: formatMoney(ii23), rule_id: "1701MS.II.23", lines: [], netted_pairs: 0 },
      {
        key: "frm1701MS:txtTotalTaxCreditsPaymentsPartVNo10a",
        value: formatMoney(v10),
        rule_id: "1701MS.V.10",
        lines: [],
        netted_pairs: 0,
        note: "V.2 + V.3 + V.4 in the PoC; the summation is UNCONFIRMED",
      },
      { key: "frm1701MS:txtTotalTaxCreditsPartIINo24a", value: formatMoney(ii24), rule_id: "1701MS.II.24", lines: [], netted_pairs: 0 },
      { key: "frm1701MS:txtTaxPayableNo25a", value: formatMoney(ii25), rule_id: "1701MS.II.25", lines: [], netted_pairs: 0 },
      { key: "frm1701MS:txtAmountPayableNo27a", value: formatMoney(ii27), rule_id: "1701MS.II.27", lines: [], netted_pairs: 0 },
      { key: "frm1701MS:txtTotalAmountPayableNo29a", value: formatMoney(ii29), rule_id: "1701MS.II.29", lines: [], netted_pairs: 0 },
    ];
    o.reference = [
      { key: "frm1701MS:txtTaxPaymentsNo2a", value: formatMoney(v2), rule_id: "1701MS.V.2", lines: [], netted_pairs: 0, note: "prior-filing input; never sent" },
      { key: "frm1701MS:txtCreditableTaxNo3a", value: formatMoney(v3), rule_id: "1701MS.V.3", lines: lineage(v3Lines), netted_pairs: 0, note: "income-tax certificates, first three quarters; never sent" },
      { key: "frm1701MS:txtCreditableFourthQuarterNo4a", value: formatMoney(v4), rule_id: "1701MS.V.4", lines: lineage(v4Lines), netted_pairs: 0, note: "income-tax certificates, fourth quarter; never sent" },
    ];
  }
  o.excludedReasons = {
    "frm1701MS:txtCostofSalesNo8a": "IV.8 is itemized-only; OSD path",
    "frm1701MS:txtAllowableItemizedDeductionsNo10a": "IV.10A is itemized-only; OSD path",
    "frm1701MS:txtSalesRevenuesFeesANo19a": "IV.19 is the 8% pathway only",
    "frm1701MS:txtOtherNonOperatingIncomeNo20a": "IV.20 is the 8% pathway only",
  };
  o.due = dueDate("1701MS", p);
  return o;
}

