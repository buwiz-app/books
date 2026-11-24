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
  const color = up ? C.green : down ? C.red : C.dim;
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

// A report card: the white rounded card the old app wraps each report in. The
// optional `right` slot holds the tag picker on P&L and cash flow, next to the
// title — the opaque picker sits on the card, not the green toolbar.
function RepCard(p: { title: string; sub: string; right?: Any; children?: Any }) {
  return (
    <View class={`${C.panel} border ${C.line} rounded-xl overflow-hidden shadow-[0_1px_3px_rgba(0,0,0,0.08)]`}>
      <View class={`px-4 sm:px-6 pt-5 pb-3.5 border-b ${C.lineSoft} flex-row items-start justify-between gap-3`}>
        <View class="flex-1">
          <Text class={`text-lg font-bold ${C.ink}`}>{p.title}</Text>
          <If when={Boolean(p.sub)}>
            <Text class={`text-[13px] ${C.mute} mt-1`}>{p.sub}</Text>
          </If>
        </View>
        {p.right || null}
      </View>
      <View class="px-4 sm:px-6 pb-5">{p.children}</View>
    </View>
  );
}

// A report table's header row: the thin muted labels over a heavy rule.
function RepHead(p: { cells: { label: string; right?: boolean; wide?: boolean; width?: string }[] }) {
  return (
    <View class={`flex-row border-b-2 ${C.line}`}>
      {p.cells.map((c: Any) => (
        <Text class={`${c.width ? c.width : c.wide ? "flex-1" : "w-[150px]"} py-2 px-3 ${c.right ? "text-right" : "text-left"} text-xs font-semibold ${C.dim}`}>{c.label}</Text>
      ))}
    </View>
  );
}

// A section title inside the table ("Revenue", "Assets", ...).
function RepSectionTitle(p: { label: string }) {
  return <Text class={`pt-3.5 px-3 pb-1.5 font-bold text-xs uppercase tracking-wider ${C.dim}`}>{p.label}</Text>;
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

  const openInTransactions = (account: string, month: string) => {
    if (!account) return;
    if (typeof txAccount !== "undefined") txAccount.set(account);
    if (typeof txMonth !== "undefined") txMonth.set(month || "");
    if (typeof txFilter !== "undefined") txFilter.set("all");
    p.go("transactions");
  };

  // One statement line: name left, current (and prior + change) right. A row
  // with a ticket opens Transactions filtered to that account and period;
  // expense rows indent like the old app's P&L.
  const row = (r: Any, withNumber: boolean, signed: boolean, indent: boolean) => {
    const open = () => openInTransactions(r.ticket || "", r.month || "");
    const neg = repCents(r.shown) < 0n;
    return (
      <View class={`flex-row items-center border-b ${C.lineSoft} ${r.ticket ? C.hoverBg : ""}`} onPress={open} activeStyle={{ opacity: r.ticket ? 0.75 : 1 }}>
        <View class={`flex-1 flex-row items-center py-2 px-3 gap-2 ${indent ? "pl-8" : ""}`}>
          <Text class={`text-[13px] ${C.mute} truncate`}>
            {withNumber && r.number ? `${r.number}  ` : ""}
            {r.name}
          </Text>
          <If when={Boolean(r.review)}>
            <Pill tone="rev" label={r.review || ""} />
          </If>
        </View>
        <Text class={`w-[150px] py-2 px-3 text-right text-[13px] font-medium tabular-nums ${neg && signed ? C.red : C.ink}`}>{r.shown || ""}</Text>
        <If when={two()}>
          {(() => {
            // a row absent in the prior period carries no prev_shown at all;
            // "0.00" is a real figure: the account was live and netted to zero.
            const had = r.prev_shown != null && r.prev_shown !== "";
            return (
              <>
                <Text class={`w-[150px] py-2 px-3 text-right text-[13px] tabular-nums ${C.dim}`}>{had ? r.prev_shown : "—"}</Text>
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
    <View class={`flex-row items-center ${bold ? C.subtle : ""}`}>
      <Text class={`flex-1 py-2.5 px-3 ${bold ? "font-bold text-sm" : "font-semibold text-[13px]"} ${C.ink}`}>{t?.label || ""}</Text>
      <Text class={`w-[150px] py-2.5 px-3 text-right ${bold ? "font-bold" : "font-semibold"} text-[13px] tabular-nums ${C.ink}`}>{t?.shown || ""}</Text>
      <If when={two()}>
        <Text class={`w-[150px] py-2.5 px-3 text-right text-[13px] tabular-nums ${C.dim}`}>{t?.prev_shown || "—"}</Text>
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
      <View class="flex-row items-center border-t-2 border-[#2563eb] dark:border-[#60a5fa]">
        <Text class={`flex-1 p-3 font-bold text-sm ${C.blue}`}>{n.label || ""}</Text>
        <Text class={`w-[150px] p-3 text-right font-bold text-sm tabular-nums ${neg ? C.red : C.blue}`}>{n.shown || ""}</Text>
        <If when={two()}>
          <Text class={`w-[150px] p-3 text-right text-[13px] tabular-nums ${C.dim}`}>{n.prev_shown || "—"}</Text>
          <View class="w-[150px] p-3" />
        </If>
      </View>
    );
  };

  const headCells = () => {
    const kind = repKind();
    if (kind === "tb")
      return [
        { label: "Acct #", width: "w-[100px]" },
        { label: "Account Name", wide: true },
        { label: "Type", width: "w-[140px]" },
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

  // The tag filter sits on the card's header row, not the green toolbar.
  const tagPick = () =>
    canTag() ? (
      <View class="w-[180px] shrink-0">
        <Pick optionSaid="Choose this tag" value={repTag()} options={(tags() as Any) || []} onPick={(o: Any) => repTag.set(o.ticket)} placeholder="All tags" />
      </View>
    ) : null;

  // pl + bs: titled sections of account rows; cf: one untitled list; each ends
  // in its closing row. P&L expense sections indent their rows like the old app.
  const statementTable = () => (
    <View class="mt-3">
      <RepHead cells={headCells()} />
      <Each of={d()?.sections || []}>
        {(s: Any, si: Any) => (
          <View>
            <If when={Boolean(s().title)}>
              <RepSectionTitle label={s().title} />
            </If>
            <Each of={s().rows}>{(r: Any) => row(r(), repKind() === "bs" || repKind() === "tb", repKind() === "cf", repKind() === "pl" && /expense/i.test(s().title || ""))}</Each>
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
              <View class={`flex-row items-center border-b ${C.lineSoft} ${r().ticket ? C.hoverBg : ""}`} onPress={() => openInTransactions(r().ticket || "", r().month || "")} activeStyle={{ opacity: r().ticket ? 0.75 : 1 }}>
                <Text class={`w-[100px] py-2 px-3 text-xs ${C.dim} tabular-nums`}>{r().number || "—"}</Text>
                <Text class={`flex-1 py-2 px-3 text-[13px] ${C.mute} truncate`}>{r().name}</Text>
                <Text class={`w-[140px] py-2 px-3 text-xs ${C.dim} capitalize`}>{String(r().type || "—").replace(/_/g, " ")}</Text>
                <Text class={`w-[150px] py-2 px-3 text-right text-[13px] font-medium tabular-nums ${r().debit_shown ? C.ink : C.dim}`}>{r().debit_shown || "—"}</Text>
                <Text class={`w-[150px] py-2 px-3 text-right text-[13px] font-medium tabular-nums ${r().credit_shown ? C.ink : C.dim}`}>{r().credit_shown || "—"}</Text>
              </View>
            )}
          </Each>
        )}
      </Each>
      <View class={`flex-row items-center border-t-2 ${C.line} ${C.subtle}`}>
        <Text class={`flex-1 p-3 font-bold text-sm ${C.ink}`}>Total</Text>
        <Text class={`w-[150px] p-3 text-right font-bold text-sm tabular-nums ${C.ink}`}>{d()?.net?.debit_shown || ""}</Text>
        <Text class={`w-[150px] p-3 text-right font-bold text-sm tabular-nums ${C.ink}`}>{d()?.net?.credit_shown || ""}</Text>
      </View>
      <View class={`mt-3 py-2 px-3 rounded-md text-xs font-medium flex-row items-center gap-1.5 ${d()?.balanced ? "bg-[#f0fdf4] dark:bg-emerald-500/10" : "bg-[#fef2f2] dark:bg-red-500/10"}`}>
        <Text class={`text-xs font-medium ${d()?.balanced ? C.green : C.red}`}>
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
        <View class={`flex-row border-b-2 ${C.line}`}>
          <Text class={`flex-1 py-2 px-3 text-left text-xs font-semibold ${C.dim}`}>{repSide() === "pay" ? "Vendor" : "Customer"}</Text>
          {bucketCols.map((c: Any) => (
            <Text class={`w-[110px] py-2 px-3 text-right text-xs font-semibold ${C.dim}`}>{c}</Text>
          ))}
          <Text class={`w-[130px] py-2 px-3 text-right text-xs font-semibold ${C.dim}`}>Total</Text>
        </View>
        <If when={rows.length === 0}>
          <Text class={`p-6 text-center text-[13px] ${C.dim}`}>{`No outstanding ${repSide() === "pay" ? "payables" : "receivables"}`}</Text>
        </If>
        <Each of={rows}>
          {(r: Any) => (
            <View class={`flex-row border-b ${C.lineSoft}`}>
              <Text class={`flex-1 py-2 px-3 text-[13px] font-medium ${C.mute} truncate`}>{r().name}</Text>
              {bucketCols.map((_c: Any, j: number) => {
                const v = cellCents(r(), j);
                const cls = v > 0n ? (j >= 3 ? C.red : C.ink) : C.dim;
                return <Text class={`w-[110px] py-2 px-3 text-right text-[13px] tabular-nums ${cls}`}>{v > 0n ? repMoney(v) : "—"}</Text>;
              })}
              <Text class={`w-[130px] py-2 px-3 text-right text-[13px] font-semibold tabular-nums ${C.ink}`}>{repMoney(rowTotal(r()))}</Text>
            </View>
          )}
        </Each>
        <If when={rows.length > 0}>
          <View class={`flex-row border-t-2 ${C.line} ${C.subtle}`}>
            <Text class={`flex-1 p-3 font-bold text-[13px] ${C.ink}`}>Total</Text>
            {bucketCols.map((_c: Any, j: number) => {
              const v = colTotal(j);
              const cls = v > 0n ? (j >= 3 ? C.red : C.ink) : C.dim;
              return <Text class={`w-[110px] p-3 text-right font-bold text-[13px] tabular-nums ${cls}`}>{v > 0n ? repMoney(v) : "—"}</Text>;
            })}
            <Text class={`w-[130px] p-3 text-right font-bold text-sm tabular-nums ${C.blue}`}>{repMoney(grand)}</Text>
          </View>
        </If>
      </View>
    );
  };

  const comparePill = (mode: string, label: string) => (
    <RepPill label={label} on={repCompare() === mode} onPress={() => repCompare.set(repCompare() === mode ? "none" : mode)} />
  );

  return (
    <View class={`flex-1 flex-col overflow-hidden ${C.bg}`}>
      {/* header: back + title left, tab pill nav centre, exports right — inner row matches report max-width */}
      <View class={`${C.emeraldBar} px-5 py-3 shrink-0`} style={{ backgroundColor: acc() }}>
        <View class="w-full max-w-[1152px] mx-auto flex-col md:flex-row md:items-center md:justify-between gap-2 md:gap-3">
          <View class="flex-row items-center gap-3">
            <View
              class="h-8 px-2 rounded-lg flex-row items-center justify-center transition-colors hover:bg-white/10 cursor-pointer"
              onPress={() => p.go("today")}
              activeStyle={{ opacity: 0.75 }}
            >
              <Text class="text-[13px] font-medium text-white/90">‹ Back</Text>
            </View>
            <Avatar src={ICONS.barChart} class="w-[22px] h-[22px]" imageClass="tint-white" fallbackDelay={Infinity} accessibilityLabel="" />
            <Text class="text-[15px] font-semibold text-white">Financials</Text>
          </View>
          <View class="flex-row items-center flex-wrap gap-0.5 p-1 bg-white/10 rounded-full self-start md:self-center">
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
                  <Text class={`text-[13px] font-medium whitespace-nowrap ${repKind() === t()[0] ? "text-white" : "text-white/60 group-hover:text-white"}`}>{t()[1]}</Text>
                </View>
              )}
            </Each>
          </View>
          <View class="flex-row items-center gap-2">
            <RepBandBtn label="↓ Export CSV" onPress={exportCsv} />
            {["pl", "bs", "cf"].includes(repKind()) ? <RepBandBtn label="↓ Excel" onPress={exportXlsx} disabled={xlsxSending()} /> : null}
            {repPeriodKind() === "month" ? <RepBandBtn label="↓ Month pack" onPress={monthPack} /> : null}
          </View>
        </View>
      </View>
      {/* toolbar: period nav + month/quarter/year left, compare pills right — aligned to report max-width */}
      <View class={`${C.emeraldBar} border-t border-white/10 px-5 py-2.5 shrink-0`} style={{ backgroundColor: acc() }}>
        <View class="w-full max-w-[1152px] mx-auto flex-row flex-wrap items-center justify-between gap-2">
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
          </View>
        </View>
      </View>
      <Scroll class="flex-1">
        <View class="w-full max-w-[1152px] mx-auto px-4 sm:px-5 py-4 sm:py-6 gap-4">
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
            <RepCard title={cardTitle()} sub={cardSub()} right={tagPick()}>
              {statementTable()}
              <If when={repKind() === "bs" && d()?.balanced === false}>
                <View class="mt-3 py-2 px-3 bg-[#fef2f2] dark:bg-red-500/10 rounded-md flex-row">
                  <Text class={`text-xs ${C.red} font-medium`}>{`⚠ Balance sheet is out of balance. ${d()?.check || ""}`}</Text>
                </View>
              </If>
            </RepCard>
          </If>
        </View>
      </Scroll>
    </View>
  );
}
