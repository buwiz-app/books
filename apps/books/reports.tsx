// Reports: P&L, balance sheet and cash flow (each beside the previous
// period, the same period last year, or alone), trial balance, and aging on
// accrual books. A tag narrows the P&L and cash flow. Click a line to see its
// entries in the Ledger. Reports follow the book's basis (ADR 0008).

const REPORT_TABS: [string, string][] = [
  ["pl", "P&L"],
  ["bs", "Balance Sheet"],
  ["cf", "Cash Flow"],
  ["tb", "Trial Balance"],
  ["ag", "Aging"],
];
const repKind = signal("pl");
const repPeriodKind = signal("month");
const repPeriod = signal("");
const repCompare = signal("none");
const repTag = signal("");
// aging: the old app's Accounts Payable / Accounts Receivable toggle
const repSide = signal("pay");
// one Excel export at a time: a press while one is in flight does nothing
const xlsxSending = signal(false);
// The old app shows the compare toggle on P&L and the balance sheet only.
const canCompare = () => repKind() === "pl" || repKind() === "bs";
const REP_MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
// Point-in-time tabs (balance sheet, trial balance, aging) read "As of
// Oct 31, 2026" in the old app: the period's last day.
const repAsOf = (key: string, kind: string): string => {
  const y = parseInt(key.slice(0, 4), 10);
  let m = 12;
  if (kind === "month") m = parseInt(key.slice(5, 7), 10) || 12;
  if (kind === "quarter") m = (parseInt(key.slice(6, 7), 10) || 4) * 3;
  const last = [31, y % 4 === 0 && (y % 100 !== 0 || y % 400 === 0) ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][m - 1];
  return `As of ${REP_MONTHS[m - 1]} ${last}, ${y}`;
};
const canTag = () => repKind() === "pl" || repKind() === "cf";
const reportArgs = (kind: string, pk: string, per: string, cmp: string, tag: string, side: string) => {
  const args: Record<string, string> = { kind, period_kind: pk };
  if (per) args.period = per;
  if (kind === "pl" || kind === "bs" || kind === "cf") args.compare_to = cmp;
  if (tag && (kind === "pl" || kind === "cf")) args.tag = tag;
  if (kind === "ag") args.side = side;
  return args;
};

// Two-decimal amount text ("1,234.56", "−1,234.56", "(1,234.56)") as centavos.
const repCents = (t: Any): bigint => {
  const s = String(t ?? "");
  const neg = /[-−(]/.test(s);
  const clean = s.replace(/[^0-9.]/g, "");
  if (!clean) return 0n;
  const dot = clean.indexOf(".");
  const whole = (dot < 0 ? clean : clean.slice(0, dot)) || "0";
  const frac = (dot < 0 ? "00" : (clean.slice(dot + 1) + "00").slice(0, 2)) || "00";
  const c = BigInt(whole) * 100n + BigInt(frac);
  return neg ? -c : c;
};

// Centavos as grouped two-decimal text.
const repMoney = (c: bigint): string => {
  const neg = c < 0n;
  const a = neg ? -c : c;
  const whole = String(a / 100n).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  return `${neg ? "−" : ""}${whole}.${String(a % 100n).padStart(2, "0")}`;
};

// The old app's ChangeBadge: an arrow, the absolute change, and the percent
// beside it. No badge when there is no prior figure to compare to.
function RepChange(p: { cur: Any; prev: Any }) {
  if (p.prev == null || p.prev === "") return null;
  const cur = repCents(p.cur);
  const prev = repCents(p.prev);
  if (prev === 0n && cur === 0n) return null;
  const diff = cur - prev;
  const up = diff > 0n;
  const down = diff < 0n;
  const color = up ? "text-[#16a34a]" : down ? "text-[#dc2626]" : "text-[#6b7280]";
  const arrow = up ? "▲ " : down ? "▼ " : "";
  const abs = diff < 0n ? -diff : diff;
  const pct = prev !== 0n ? (abs * 1000n) / (prev < 0n ? -prev : prev) : null;
  return (
    <Text class={`text-[11px] font-semibold ${color} whitespace-nowrap`}>
      {arrow}
      {repMoney(abs)}
      {pct !== null ? ` (${pct / 10n}.${pct % 10n}%)` : ""}
    </Text>
  );
}

// A pill on the green toolbar: the compare toggles and the AP/AR toggle.
function RepPill(p: { label: string; on: boolean; onPress: () => void }) {
  return (
    <View
      class={`group px-3 py-1 rounded-md text-center border transition-all ${p.on ? "bg-white/20 border-white/20" : "border-transparent hover:bg-white/10"}`}
      onPress={p.onPress}
      activeStyle={{ opacity: 0.8 }}
      hoverStyle={{ backgroundColor: p.on ? undefined : "rgba(255,255,255,0.1)" }}
    >
      <Text class={`text-[12px] font-medium ${p.on ? "text-white" : "text-white/50 group-hover:text-white"}`}>{p.label}</Text>
    </View>
  );
}

// The green band's small light button: the exports and the month pack.
function RepBandBtn(p: { label: string; onPress: () => void; disabled?: boolean }) {
  return (
    <View
      class={`flex-row items-center gap-1.5 px-3 py-1.5 rounded-lg bg-white/10 transition-colors ${p.disabled ? "opacity-40" : "hover:bg-white/15"}`}
      onPress={p.disabled ? () => {} : p.onPress}
      activeStyle={{ opacity: 0.8 }}
      hoverStyle={{ backgroundColor: p.disabled ? undefined : "rgba(255,255,255,0.15)" }}
    >
      <Text class="text-xs font-medium text-white">{p.label}</Text>
    </View>
  );
}

// A report card: the white rounded card the old app wraps each report in.
function RepCard(p: { title: string; sub: string; children?: Any }) {
  return (
    <View class={`rounded-xl ${C.panel} shadow-[0_1px_3px_rgba(0,0,0,0.08),0_1px_2px_rgba(0,0,0,0.04)]`}>
      <View class="px-3 sm:px-6 pt-5 pb-3 border-b border-[#f3f4f6] dark:border-white/8">
        <Text class="text-lg font-bold text-[#111827] dark:text-white">{p.title}</Text>
        <If when={Boolean(p.sub)}>
          <Text class="text-[13px] text-[#6b7280] dark:text-white/50 mt-1">{p.sub}</Text>
        </If>
      </View>
      <View class="px-3 sm:px-6 pb-5">{p.children}</View>
    </View>
  );
}

// A report table's header row: the thin muted labels over a heavy rule.
function RepHead(p: { cells: { label: string; right?: boolean; wide?: boolean }[] }) {
  return (
    <View class="flex-row border-b-2 border-[#e5e7eb] dark:border-white/10">
      {p.cells.map((c: Any) => (
        <Text class={`${c.wide ? "flex-1" : "w-[150px]"} py-2 px-3 ${c.right ? "text-right" : "text-left"} text-xs font-semibold text-[#6b7280] dark:text-white/40`}>{c.label}</Text>
      ))}
    </View>
  );
}

// A section title inside the table ("Revenue", "Assets", ...).
function RepSectionTitle(p: { label: string }) {
  return <Text class="pt-3.5 px-3 pb-1.5 font-bold text-xs uppercase tracking-wider text-[#6b7280] dark:text-white/40">{p.label}</Text>;
}

function ReportsPage(p: { go: (page: string, filter?: string) => void }) {
  const data = resource(
    () => ({ v: ver(), k: repKind(), pk: repPeriodKind(), per: repPeriod(), cmp: repCompare(), tag: repTag(), side: repSide() }),
    (k: Any) => buwiz.call("books_report", reportArgs(k.k, k.pk, k.per, k.cmp, k.tag, k.side)),
  );
  const tags = resource(
    () => ({ v: ver() }),
    () =>
      Promise.all([buwiz.call("books_tags_list", { kind: "department" }), buwiz.call("books_tags_list", { kind: "location" })]).then((all: Any[]) => [
        { ticket: "", name: "All tags" },
        ...all.flatMap((t: Any) => (t?.rows || []).map((r: Any) => ({ ticket: r.ticket, name: r.name }))),
      ]),
  );
  const d = () => data() as Any;
  const two = () => Boolean(d()?.compare);
  const exportCsv = () => {
    const args = reportArgs(repKind(), repPeriodKind(), repPeriod(), repCompare(), repTag(), repSide());
    buwiz.call("books_report_csv", args).then(
      (r: Any) => {
        if (buwiz.copy) {
          buwiz.copy(String(r.csv));
          toast(`${String(r.file_name)} copied · ${String(r.rows)} rows`);
        } else toast("Copying is not available in this Buwiz Bot.");
      },
      (e: Any) => toast(errText(e)),
    );
  };
  // the Bot copies the file to Downloads and says how it went in its own toast;
  // the call's answer carries no outcome of that, so this line claims none
  const exportXlsx = () => {
    if (xlsxSending()) return;
    xlsxSending.set(true);
    const args = reportArgs(repKind(), repPeriodKind(), repPeriod(), repCompare(), repTag(), repSide());
    buwiz.call("books_app_report_xlsx", { client_request_id: rid(), ...args }).then(
      (r: Any) => {
        xlsxSending.set(false);
        toast(`${r.already_sent ? "Already sent to" : "Sent to"} Buwiz Bot to save in Downloads · ${String(r.file_name)}`);
      },
      (e: Any) => {
        xlsxSending.set(false);
        toast(errText(e));
      },
    );
  };
  const monthPack = () => {
    if (buwiz.preview)
      buwiz
        .preview({ tool: "books_app_month_pack", args: { month: d()?.period.key }, title: "Month pack" })
        .catch((e: Any) => toast(e.code === "not_supported" ? "Print preview needs Buwiz Bot on a Mac." : errText(e)));
    else toast("Print preview is not available in this Buwiz Bot.");
  };

  // One statement line: name left, current (and prior + change) right. A row
  // with a ticket opens the Ledger on a press.
  const row = (r: Any, withNumber: boolean, signed: boolean) => {
    const open = () => {
      if (!r.ticket) return;
      openLedger({ account: r.ticket, month: r.month || "" });
    };
    const neg = repCents(r.shown) < 0n;
    return (
      <View class="flex-row items-center border-b border-[#f3f4f6] dark:border-white/5" onPress={open} activeStyle={{ opacity: r.ticket ? 0.75 : 1 }}>
        <View class="flex-1 flex-row items-center py-2 px-3 gap-2">
          <Text class="text-[13px] text-[#374151] dark:text-white/70 truncate">
            {withNumber && r.number ? `${r.number}  ` : ""}
            {r.name}
          </Text>
          <If when={Boolean(r.review)}>
            <Pill tone="rev" label={r.review || ""} />
          </If>
        </View>
        <Text class={`w-[150px] py-2 px-3 text-right text-[13px] font-medium tabular-nums ${neg && signed ? "text-[#dc2626] dark:text-red-400" : "text-[#111827] dark:text-white"}`}>{r.shown || ""}</Text>
        <If when={two()}>
          {(() => {
            // a row absent in the prior period carries no prev_shown at all;
            // "0.00" is a real figure: the account was live and netted to zero.
            const had = r.prev_shown != null && r.prev_shown !== "";
            return (
              <>
                <Text class="w-[150px] py-2 px-3 text-right text-[13px] tabular-nums text-[#6b7280] dark:text-white/50">{had ? r.prev_shown : "—"}</Text>
                <View class="w-[150px] py-2 px-3 items-end">
                  <RepChange cur={r.shown} prev={had ? r.prev_shown : ""} />
                </View>
              </>
            );
          })()}
        </If>
      </View>
    );
  };

  // A section's total row: semibold like the old SectionRow, or the heavier
  // band the balance sheet uses.
  const totalRow = (t: Any, bold: boolean) => (
    <View class={`flex-row items-center ${bold ? "bg-[#f8fafc] dark:bg-white/[0.03]" : ""}`}>
      <Text class={`flex-1 py-2.5 px-3 ${bold ? "font-bold text-sm" : "font-semibold text-[13px]"} text-[#111827] dark:text-white`}>{t?.label || ""}</Text>
      <Text class={`w-[150px] py-2.5 px-3 text-right ${bold ? "font-bold" : "font-semibold"} text-[13px] tabular-nums text-[#111827] dark:text-white`}>{t?.shown || ""}</Text>
      <If when={two()}>
        <Text class="w-[150px] py-2.5 px-3 text-right text-[13px] tabular-nums text-[#6b7280] dark:text-white/50">{t?.prev_shown || "—"}</Text>
        <View class="w-[150px] py-2.5 px-3 items-end">
          <RepChange cur={t?.shown} prev={t?.prev_shown} />
        </View>
      </If>
    </View>
  );

  // The closing row: P&L's Net Income reads like a section total; the other
  // statements close on the blue rule.
  const netRow = () => {
    const n = d()?.net;
    if (!n) return null;
    if (repKind() === "pl") return totalRow(n, true);
    const neg = repCents(n.shown) < 0n;
    return (
      <View class="flex-row items-center border-t-2 border-[#1a7fc4]">
        <Text class="flex-1 p-3 font-bold text-sm text-[#1a7fc4]">{n.label || ""}</Text>
        <Text class={`w-[150px] p-3 text-right font-bold text-sm tabular-nums ${neg ? "text-[#dc2626] dark:text-red-400" : "text-[#1a7fc4]"}`}>{n.shown || ""}</Text>
        <If when={two()}>
          <Text class="w-[150px] p-3 text-right text-[13px] tabular-nums text-[#6b7280] dark:text-white/50">{n.prev_shown || "—"}</Text>
          <View class="w-[150px] p-3" />
        </If>
      </View>
    );
  };

  const headCells = () => {
    const kind = repKind();
    if (kind === "tb")
      return [
        { label: "Acct #", wide: true },
        { label: "Account Name", wide: true },
        { label: "Debit", right: true },
        { label: "Credit", right: true },
      ];
    const first = kind === "cf" ? "Activity" : "Account";
    const cur = kind === "bs" ? "Balance" : kind === "cf" ? "Amount" : "Current";
    const cells: Any[] = [
      { label: first, wide: true },
      { label: cur, right: true },
    ];
    if (two()) {
      cells.push({ label: "Prior", right: true });
      cells.push({ label: "Change", right: true });
    }
    return cells;
  };

  const cardTitle = () => {
    const kind = repKind();
    if (kind === "pl") return "Profit & Loss";
    if (kind === "bs") return "Balance Sheet";
    if (kind === "cf") return "Cash Flow Statement";
    if (kind === "tb") return "Trial Balance";
    return repSide() === "pay" ? "Accounts Payable Aging" : "Accounts Receivable Aging";
  };

  const cardSub = () => {
    const base = d()?.columns?.[0] || d()?.period?.label || "";
    return base ? `${base} • ${d()?.basis_label || ""}` : "";
  };

  // pl + bs: titled sections of account rows; cf: one untitled list; each ends
  // in its closing row.
  const statementTable = () => (
    <View class="mt-3">
      <RepHead cells={headCells()} />
      <Each of={d()?.sections || []}>
        {(s: Any, si: Any) => (
          <View>
            <If when={Boolean(s().title)}>
              <RepSectionTitle label={s().title} />
            </If>
            <Each of={s().rows}>{(r: Any) => row(r(), repKind() === "bs" || repKind() === "tb", repKind() === "cf")}</Each>
            <If when={Boolean(s().total)}>
              {totalRow(s().total, repKind() === "bs")}
            </If>
            <If when={repKind() === "pl" && Boolean(d()?.totals?.[si()])}>
              {totalRow(d()?.totals?.[si()], false)}
            </If>
          </View>
        )}
      </Each>
      {netRow()}
    </View>
  );

  const tbTable = () => (
    <View class="mt-3">
      <RepHead cells={headCells()} />
      <Each of={d()?.sections || []}>
        {(s: Any) => (
          <Each of={s().rows}>
            {(r: Any) => (
              <View class="flex-row items-center border-b border-[#f3f4f6] dark:border-white/5" onPress={() => r().ticket && openLedger({ account: r().ticket, month: r().month || "" })} activeStyle={{ opacity: r().ticket ? 0.75 : 1 }}>
                <Text class="flex-1 py-2 px-3 text-xs text-[#9ca3af] dark:text-white/30 tabular-nums">{r().number || "—"}</Text>
                <Text class="flex-1 py-2 px-3 text-[13px] text-[#374151] dark:text-white/70 truncate">{r().name}</Text>
                <Text class={`w-[150px] py-2 px-3 text-right text-[13px] font-medium tabular-nums ${r().debit_shown ? "text-[#111827] dark:text-white" : "text-[#d1d5db] dark:text-white/20"}`}>{r().debit_shown || "—"}</Text>
                <Text class={`w-[150px] py-2 px-3 text-right text-[13px] font-medium tabular-nums ${r().credit_shown ? "text-[#111827] dark:text-white" : "text-[#d1d5db] dark:text-white/20"}`}>{r().credit_shown || "—"}</Text>
              </View>
            )}
          </Each>
        )}
      </Each>
      <View class="flex-row items-center border-t-2 border-[#e5e7eb] dark:border-white/10 bg-[#f8fafc] dark:bg-white/[0.03]">
        <View class="flex-1" />
        <Text class="flex-1 p-3 font-bold text-sm text-[#111827] dark:text-white">Total</Text>
        <Text class="w-[150px] p-3 text-right font-bold text-sm tabular-nums text-[#111827] dark:text-white">{d()?.net?.debit_shown || ""}</Text>
        <Text class="w-[150px] p-3 text-right font-bold text-sm tabular-nums text-[#111827] dark:text-white">{d()?.net?.credit_shown || ""}</Text>
      </View>
      <View class={`mt-3 py-2 px-3 rounded-md text-xs font-medium flex-row items-center gap-1.5 ${d()?.balanced ? "bg-[#f0fdf4] dark:bg-emerald-900/20 text-[#16a34a] dark:text-emerald-400" : "bg-[#fef2f2] dark:bg-red-900/20 text-[#dc2626] dark:text-red-400"}`}>
        <Text class={`text-xs font-medium ${d()?.balanced ? "text-[#16a34a] dark:text-emerald-400" : "text-[#dc2626] dark:text-red-400"}`}>
          {d()?.balanced ? "✓ Trial balance is in balance" : `⚠ Out of balance by ${repMoney(repCents(d()?.net?.debit_shown) - repCents(d()?.net?.credit_shown)).replace("−", "")}`}
        </Text>
      </View>
    </View>
  );

  const agTable = () => {
    const cols: Any[] = d()?.columns || [];
    const bucketCols = cols.slice(1);
    const rows: Any[] = d()?.rows || [];
    const cellCents = (r: Any, j: number) => repCents(r.cells?.[j] || "0");
    const rowTotal = (r: Any) => bucketCols.reduce((a: bigint, _c: Any, j: number) => a + cellCents(r, j), 0n);
    const colTotal = (j: number) => rows.reduce((a: bigint, r: Any) => a + cellCents(r, j), 0n);
    const grand = rows.reduce((a: bigint, r: Any) => a + rowTotal(r), 0n);
    return (
      <View class="mt-3">
        <View class="flex-row border-b-2 border-[#e5e7eb] dark:border-white/10">
          <Text class="flex-1 py-2 px-3 text-left text-xs font-semibold text-[#6b7280] dark:text-white/40">{repSide() === "pay" ? "Vendor" : "Customer"}</Text>
          {bucketCols.map((c: Any) => (
            <Text class="w-[110px] py-2 px-3 text-right text-xs font-semibold text-[#6b7280] dark:text-white/40">{c}</Text>
          ))}
          <Text class="w-[130px] py-2 px-3 text-right text-xs font-semibold text-[#6b7280] dark:text-white/40">Total</Text>
        </View>
        <If when={rows.length === 0}>
          <Text class="p-6 text-center text-[13px] text-[#9ca3af] dark:text-white/30">{`No outstanding ${repSide() === "pay" ? "payables" : "receivables"}`}</Text>
        </If>
        <Each of={rows}>
          {(r: Any) => (
            <View class="flex-row border-b border-[#f3f4f6] dark:border-white/5">
              <Text class="flex-1 py-2 px-3 text-[13px] font-medium text-[#374151] dark:text-white/70 truncate">{r().name}</Text>
              {bucketCols.map((_c: Any, j: number) => {
                const v = cellCents(r(), j);
                const cls = v > 0n ? (j >= 3 ? "text-[#dc2626] dark:text-red-400" : "text-[#111827] dark:text-white") : "text-[#d1d5db] dark:text-white/20";
                return <Text class={`w-[110px] py-2 px-3 text-right text-[13px] tabular-nums ${cls}`}>{v > 0n ? repMoney(v) : "—"}</Text>;
              })}
              <Text class="w-[130px] py-2 px-3 text-right text-[13px] font-semibold tabular-nums text-[#111827] dark:text-white">{repMoney(rowTotal(r()))}</Text>
            </View>
          )}
        </Each>
        <If when={rows.length > 0}>
          <View class="flex-row border-t-2 border-[#e5e7eb] dark:border-white/10 bg-[#f8fafc] dark:bg-white/[0.03]">
            <Text class="flex-1 p-3 font-bold text-[13px] text-[#111827] dark:text-white">Total</Text>
            {bucketCols.map((_c: Any, j: number) => {
              const v = colTotal(j);
              const cls = v > 0n ? (j >= 3 ? "text-[#dc2626] dark:text-red-400" : "text-[#111827] dark:text-white") : "text-[#d1d5db] dark:text-white/20";
              return <Text class={`w-[110px] p-3 text-right font-bold text-[13px] tabular-nums ${cls}`}>{v > 0n ? repMoney(v) : "—"}</Text>;
            })}
            <Text class="w-[130px] p-3 text-right font-bold text-sm tabular-nums text-[#1a7fc4]">{repMoney(grand)}</Text>
          </View>
        </If>
      </View>
    );
  };

  const comparePill = (mode: string, label: string) => (
    <RepPill label={label} on={repCompare() === mode} onPress={() => repCompare.set(repCompare() === mode ? "none" : mode)} />
  );

  return (
    <View class="gap-3.5">
      <View class="rounded-xl" style={{ backgroundColor: acc() }}>
        <View class="flex-col px-5 py-3 gap-3">
          <View class="flex-row items-center gap-3 flex-wrap">
            <View
              class="h-8 px-2 rounded-lg flex-row items-center justify-center transition-colors hover:bg-white/10 cursor-pointer"
              onPress={() => p.go("today")}
              activeStyle={{ opacity: 0.75 }}
            >
              <Text class="text-[13px] font-medium text-white/90">‹ Back</Text>
            </View>
            <View class="flex-row items-end gap-[2px] shrink-0">
              <View class="w-[3px] h-[14px] rounded-[1px] bg-white" />
              <View class="w-[3px] h-[20px] rounded-[1px] bg-white" />
              <View class="w-[3px] h-[9px] rounded-[1px] bg-white" />
            </View>
            <Text class="text-[15px] font-semibold text-white">Financials</Text>
            <View class="flex-1" />
            <RepBandBtn label="↓ Export CSV" onPress={exportCsv} />
            {["pl", "bs", "cf"].includes(repKind()) ? <RepBandBtn label="↓ Excel" onPress={exportXlsx} disabled={xlsxSending()} /> : null}
            {repPeriodKind() === "month" ? <RepBandBtn label="↓ Month pack" onPress={monthPack} /> : null}
          </View>
          {/* the tabs take their own row and wrap, so all five stay reachable
              on a phone-width window instead of running off the right edge */}
          <View class="flex-row items-center flex-wrap gap-1 p-1 bg-white/10 rounded-full self-start">
            <Each of={REPORT_TABS}>
              {(t: Any) => (
                <View
                  class={`group px-4 py-1 rounded-full transition-all ${repKind() === t()[0] ? "bg-white/20 shadow-sm" : "hover:bg-white/10"}`}
                  onPress={() => {
                    repKind.set(t()[0]);
                    repCompare.set("none");
                  }}
                  activeStyle={{ opacity: 0.8 }}
                  hoverStyle={{ backgroundColor: repKind() === t()[0] ? undefined : "rgba(255,255,255,0.1)" }}
                >
                  <Text class={`text-[13px] font-medium whitespace-nowrap ${repKind() === t()[0] ? "text-white" : "text-white/50 group-hover:text-white"}`}>{t()[1]}</Text>
                </View>
              )}
            </Each>
          </View>
        </View>
        <View class="flex-row items-center justify-between gap-2 px-5 pb-3 flex-wrap">
          <View class="flex-row items-center gap-2">
            <View class="flex-row items-center bg-white/10 rounded-lg shrink-0">
              <View
                class={`group w-8 h-8 items-center justify-center rounded-l-lg transition-colors ${d()?.period.prev ? "" : "opacity-40"}`}
                onPress={() => d()?.period.prev && repPeriod.set(d().period.prev)}
                activeStyle={{ opacity: 0.7 }}
                hoverStyle={{ backgroundColor: d()?.period.prev ? "rgba(255,255,255,0.1)" : undefined }}
              >
                <Avatar src={ICONS.chevronLeft} class="w-3.5 h-3.5 opacity-70 group-hover:opacity-100" imageClass="tint-white" fallbackDelay={Infinity} accessibilityLabel="" />
              </View>
              <Text class="px-2 text-xs font-medium text-white/70">
                {["bs", "tb", "ag"].includes(repKind()) && d()?.period?.key ? repAsOf(d().period.key, repPeriodKind()) : d()?.period.label || ""}
              </Text>
              <View
                class={`group w-8 h-8 items-center justify-center rounded-r-lg transition-colors ${d()?.period.next ? "" : "opacity-40"}`}
                onPress={() => d()?.period.next && repPeriod.set(d().period.next)}
                activeStyle={{ opacity: 0.7 }}
                hoverStyle={{ backgroundColor: d()?.period.next ? "rgba(255,255,255,0.1)" : undefined }}
              >
                <Avatar src={ICONS.chevronRight} class="w-3.5 h-3.5 opacity-70 group-hover:opacity-100" imageClass="tint-white" fallbackDelay={Infinity} accessibilityLabel="" />
              </View>
            </View>
            <View class="flex-row items-center bg-white/10 rounded-lg shrink-0">
              <Each of={[["month", "Month"], ["quarter", "Quarter"], ["year", "Year"]]}>
                {(o: Any) => (
                  <View
                    class={`group px-3 h-8 items-center justify-center transition-colors ${repPeriodKind() === o()[0] ? "bg-white/20 rounded-lg" : ""}`}
                    onPress={() => {
                      repPeriodKind.set(o()[0]);
                      repPeriod.set("");
                    }}
                    activeStyle={{ opacity: 0.8 }}
                    hoverStyle={{ backgroundColor: repPeriodKind() === o()[0] ? undefined : "rgba(255,255,255,0.1)" }}
                  >
                    <Text class={`text-[12px] font-medium ${repPeriodKind() === o()[0] ? "text-white" : "text-white/50 group-hover:text-white"}`}>{o()[1]}</Text>
                  </View>
                )}
              </Each>
            </View>
          </View>
          <View class="flex-row items-center gap-2">
            <If when={canCompare()}>
              {comparePill("previous", "vs Prior Period")}
              {comparePill("last_year", "vs Last Year")}
            </If>
            <If when={repKind() === "ag"}>
              <RepPill label="Accounts Payable" on={repSide() === "pay"} onPress={() => repSide.set("pay")} />
              <RepPill label="Accounts Receivable" on={repSide() === "collect"} onPress={() => repSide.set("collect")} />
            </If>
            <If when={canTag()}>
              <View class="w-[180px]">
                <Pick optionSaid="Choose this tag" value={repTag()} options={(tags() as Any) || []} onPick={(o: Any) => repTag.set(o.ticket)} placeholder="All tags" />
              </View>
            </If>
          </View>
        </View>
      </View>
      <If when={Boolean(d()?.tag_note)}>
        <Text class={`text-[12px] ${C.dim}`}>{d()?.tag_note || ""}</Text>
      </If>
      <Loading res={data} />
      <If when={Boolean(d()) && repKind() === "ag" && d()?.cash_book}>
        <RepCard title={d()?.title || ""} sub="">
          <View class="gap-2 pt-1">
            <Pill tone="in" label="Cash basis" />
            <Text class={`text-[12px] ${C.dim}`}>{d()?.note || ""}</Text>
            <Btn label="Open Unpaid" onPress={() => p.go("unpaid")} />
          </View>
        </RepCard>
      </If>
      <If when={Boolean(d()) && repKind() === "ag" && !d()?.cash_book}>
        <RepCard title={cardTitle()} sub={cardSub()}>
          {agTable()}
        </RepCard>
      </If>
      <If when={Boolean(d()) && repKind() === "tb"}>
        <RepCard title={cardTitle()} sub={cardSub()}>
          {tbTable()}
        </RepCard>
      </If>
      <If when={Boolean(d()) && ["pl", "bs", "cf"].includes(repKind())}>
        <RepCard title={cardTitle()} sub={cardSub()}>
          {statementTable()}
          <If when={repKind() === "bs" && d()?.balanced === false}>
            <View class="mt-3 py-2 px-3 bg-[#fef2f2] dark:bg-red-900/20 rounded-md flex-row">
              <Text class="text-xs text-[#dc2626] dark:text-red-400 font-medium">{`⚠ Balance sheet is out of balance. ${d()?.check || ""}`}</Text>
            </View>
          </If>
        </RepCard>
      </If>
      <Text class={`text-[12px] ${C.dim}`}>Click a line to see its transactions in the Ledger.</Text>
      <LedgerBulk />
    </View>
  );
}
