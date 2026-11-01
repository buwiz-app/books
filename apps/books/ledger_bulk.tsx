// Ledger multi-select on the Reports page: tick rows, then Post, Discard
// or give them one category in one click (books_app_batch, all or
// nothing), and the ledger as CSV (books_report_csv kind "ledger"). The
// Ledger itself is a native screen; only a click here changes entries
// (DD-20).

const BULK_FILTERS = [
  ["draft", "Drafts"],
  ["posted", "Posted"],
  ["review", "Needs review"],
  ["all", "All"],
];

/** The ticked entry tickets. */
const bulkSel = signal(new Set<string>());

function bulkToggle(t: string) {
  const s = new Set(bulkSel());
  if (s.has(t)) s.delete(t);
  else s.add(t);
  bulkSel.set(s);
}

/** Last day of a month, for the CSV range: integer dates, no money. */
function bulkLastDay(key: string): string {
  const y = Number(key.slice(0, 4));
  const m = Number(key.slice(5, 7));
  const leap = (y % 4 === 0 && y % 100 !== 0) || y % 400 === 0;
  const days = [31, leap ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][m - 1] || 31;
  return `${key}-${String(days)}`;
}

/** The tap target in front of a ledger row. */
function BulkBox(p: { r: Any }) {
  return (
    <View class={`pl-3 py-2.5 border-b ${C.line}`} onPress={() => bulkToggle(p.r.ticket)} activeStyle={{ opacity: 0.6 }}>
      <Text class={`text-[14px] ${C.ink}`}>{bulkSel().has(p.r.ticket) ? "◼" : "◻"}</Text>
    </View>
  );
}

/** A ledger row with its tap target in front. */
function BulkRow(p: { r: Any }) {
  const r = p.r;
  return (
    <View class="flex-row">
      <BulkBox r={r} />
      <View class={`flex-1 flex-row items-center gap-3 px-3 py-2.5 border-b ${C.line}`}>
        <Text class={`w-[52px] text-[12.5px] font-mono ${C.ink}`}>{r.date_short}</Text>
        <Text class={`flex-1 text-[13px] font-semibold ${C.ink}`} numberOfLines={1}>
          {r.title}
        </Text>
        <View class="w-[150px] flex-row items-center gap-1.5">
          <Dot color={r.category_color} />
          <Text class={`text-[12.5px] ${C.ink}`} numberOfLines={1}>
            {r.category || ""}
          </Text>
        </View>
        <View class="w-[96px]">
          <Pill tone={r.pill} label={r.status_label} />
        </View>
        <Text class={`w-[104px] text-right text-[12.5px] font-mono ${AMT[r.tone] || C.ink}`}>{r.amount_shown}</Text>
      </View>
    </View>
  );
}

/** "N selected · Post · Discard · Category…", Select all and Export CSV. */
function BulkBar(p: { rows: Any[]; account: string; month: string; filter: string }) {
  const busy = signal(false);
  const picking = signal(false);
  const options = resource(
    () => ver(),
    () => buwiz.call("books_entry_options", {}),
  );
  const cats = () => {
    const o = options() as Any;
    const seen = new Set<string>();
    return [...((o && o.expense) || []), ...((o && o.income) || [])].filter((c: Any) => (seen.has(c.ticket) ? false : (seen.add(c.ticket), true)));
  };
  const picked = () => p.rows.filter((r: Any) => bulkSel().has(r.ticket));
  const all = (status: string) => picked().length > 0 && picked().every((r: Any) => r.status === status);
  const run = (action: string, extra: Record<string, Any>) => {
    busy.set(true);
    act("books_app_batch", { action, entries: picked().map((r: Any) => r.ticket), ...extra }).then(
      (r: Any) => {
        busy.set(false);
        picking.set(false);
        bulkSel.set(new Set());
        toast(`${String(r.done)} entries done`);
      },
      () => busy.set(false),
    );
  };
  const exportCsv = () => {
    const args: Record<string, string> = { kind: "ledger" };
    if (p.filter !== "all") args.filter = p.filter;
    if (p.month) {
      args.from = `${p.month}-01`;
      args.to = bulkLastDay(p.month);
    }
    if (p.account) args.account = p.account;
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
  return (
    <View class={`gap-2 px-3 py-2 border-t ${C.line}`}>
      <View class="flex-row items-center gap-2 flex-wrap">
        <Btn small label="Select all on screen" onPress={() => bulkSel.set(new Set(p.rows.map((r: Any) => String(r.ticket))))} />
        <If when={picked().length > 0}>
          <Text class={`text-[12.5px] font-semibold ${C.ink}`}>{`${String(picked().length)} selected`}</Text>
          <Btn small label="Post" tone="go" disabled={busy() || !all("draft")} onPress={() => run("post", {})} />
          <Btn small label="Discard" disabled={busy() || !all("draft")} onPress={() => run("discard", {})} />
          <Btn small label="Category…" disabled={busy() || !all("posted")} onPress={() => picking.set(!picking())} />
          <Btn small label="Clear" onPress={() => bulkSel.set(new Set())} />
        </If>
        <View class="flex-1" />
        <Btn small label="Export CSV" onPress={exportCsv} />
      </View>
      <If when={picking() && all("posted")}>
        <Pick value="" options={cats()} placeholder="Pick a category" onPick={(c: Any) => run("recategorize", { category: c.ticket })} />
      </If>
    </View>
  );
}

/** A month of ledger entries to tick, with the bulk bar (Reports page). */
function LedgerBulk() {
  const filter = signal("draft");
  const month = signal("");
  const data = resource(
    () => ({ v: ver(), f: filter(), m: month() }),
    (k: Any) => {
      const args: Record<string, string> = { filter: k.f };
      if (k.m) args.month = k.m;
      return buwiz.call("books_ledger_list", args);
    },
  );
  const d = () => data() as Any;
  return (
    <Panel class="gap-0">
      <View class="flex-row items-center gap-2 flex-wrap px-3 py-2.5">
        <Text class={`text-[13px] font-semibold ${C.ink}`}>Ledger entries</Text>
        <View class="px-1.5 py-1" onPress={() => month.set(d()?.month.prev || "")}>
          <Text class={`text-[12px] ${C.ink}`}>‹</Text>
        </View>
        <Text class={`text-[12px] ${C.mute}`}>{d()?.month.label || ""}</Text>
        <View class="px-1.5 py-1" onPress={() => month.set(d()?.month.next || "")}>
          <Text class={`text-[12px] ${C.ink}`}>›</Text>
        </View>
        <View class="flex-1" />
        <Each of={BULK_FILTERS}>
          {(f: Any) => <Chip label={f()[1]} count={d()?.counts[f()[0]]} on={filter() === f()[0]} onPress={() => filter.set(f()[0])} />}
        </Each>
      </View>
      <Loading res={data} />
      <BulkBar rows={d()?.rows || []} account="" month={d()?.month.key || ""} filter={filter()} />
      <If when={(d()?.rows || []).length === 0}>
        <Text class={`text-[12.5px] px-3 py-4 ${C.dim}`}>No matching entries this month.</Text>
      </If>
      <Each of={d()?.rows || []} key={rowKey}>
        {(r: Any) => <BulkRow r={r()} />}
      </Each>
    </Panel>
  );
}
