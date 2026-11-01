// Ledger multi-select: tick rows, then Post, Discard or give them one
// category in one click (books_app_batch, all or nothing), and the
// ledger as CSV (books_report_csv kind "ledger"). Only a click here
// changes entries (DD-20).

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
  return (
    <View class="flex-row">
      <BulkBox r={p.r} />
      <View class="flex-1">
        <TxRow r={p.r} />
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
    const args: Record<string, string> = { kind: "ledger", filter: p.filter };
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
