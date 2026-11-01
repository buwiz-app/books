// Reports: P&L, balance sheet and cash flow (each beside the previous
// period, the same period last year, or alone), trial balance, and aging on
// accrual books. A tag narrows the P&L and cash flow. Click a line to see its
// entries in the Ledger. Reports follow the book's basis (ADR 0008).

const REPORT_TABS: [string, string][] = [
  ["pl", "P&L"],
  ["bs", "Balance sheet"],
  ["cf", "Cash flow"],
  ["tb", "Trial balance"],
  ["ag", "Aging"],
];
const repKind = signal("pl");
const repPeriodKind = signal("month");
const repPeriod = signal("");
const repCompare = signal("previous");
const repTag = signal("");
const COMPARE_TABS: [string, string][] = [
  ["previous", "Previous"],
  ["last_year", "Last year"],
  ["none", "None"],
];
const canCompare = () => repKind() === "pl" || repKind() === "bs" || repKind() === "cf";
const canTag = () => repKind() === "pl" || repKind() === "cf";
const reportArgs = (kind: string, pk: string, per: string, cmp: string, tag: string) => {
  const args: Record<string, string> = { kind, period_kind: pk };
  if (per) args.period = per;
  if (kind === "pl" || kind === "bs" || kind === "cf") args.compare_to = cmp;
  if (tag && (kind === "pl" || kind === "cf")) args.tag = tag;
  return args;
};

function RepRow(p: { r: Any; cols: number; go: (page: string, filter?: string) => void }) {
  const r = p.r;
  const open = () => {
    if (!r.ticket) return;
    openLedger({ account: r.ticket, month: r.month || "" });
  };
  return (
    <View class={`flex-row items-center gap-3 px-3 py-2 border-b ${C.line}`} onPress={open} activeStyle={{ opacity: r.ticket ? 0.75 : 1 }}>
      <View class="flex-1 flex-row items-center gap-2">
        <Text class={`text-[13px] ${C.ink}`} numberOfLines={1}>
          {r.name}
        </Text>
        <If when={Boolean(r.review)}>
          <Pill tone="rev" label={r.review || ""} />
        </If>
      </View>
      <If when={p.cols === 3}>
        <Text class={`w-[130px] text-right text-[12.5px] font-mono ${C.ink}`}>{r.debit_shown ?? ""}</Text>
        <Text class={`w-[130px] text-right text-[12.5px] font-mono ${C.ink}`}>{r.credit_shown ?? ""}</Text>
      </If>
      <If when={p.cols !== 3}>
        <Text class={`w-[130px] text-right text-[12.5px] font-mono ${r.tone === "pos" ? C.green : C.ink}`}>{r.shown ?? ""}</Text>
        <If when={p.cols === 2}>
          <Text class={`w-[130px] text-right text-[12.5px] font-mono ${C.mute}`}>{r.prev_shown ?? ""}</Text>
        </If>
      </If>
    </View>
  );
}

function ReportsPage(p: { go: (page: string, filter?: string) => void }) {
  const data = resource(
    () => ({ v: ver(), k: repKind(), pk: repPeriodKind(), per: repPeriod(), cmp: repCompare(), tag: repTag() }),
    (k: Any) => buwiz.call("books_report", reportArgs(k.k, k.pk, k.per, k.cmp, k.tag)),
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
  const two = () => (d()?.columns?.length || 0) > 1 && Boolean(d()?.compare);
  const cols = () => (repKind() === "tb" ? 3 : two() ? 2 : 1);
  const exportCsv = () => {
    const args = reportArgs(repKind(), repPeriodKind(), repPeriod(), repCompare(), repTag());
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
    <View class="gap-3.5">
      <Head title="Reports" sub={d()?.basis_label || ""}>
        <View class="flex-row gap-2 items-center">
          <Seg
            options={[
              ["month", "Month"],
              ["quarter", "Quarter"],
              ["year", "Year"],
            ]}
            value={repPeriodKind()}
            onPick={(v: string) => {
              repPeriodKind.set(v);
              repPeriod.set("");
            }}
          />
          <Btn label="Export CSV" onPress={exportCsv} />
        </View>
      </Head>
      <View class="flex-row items-center gap-2">
        <Seg
          options={REPORT_TABS}
          value={repKind()}
          onPick={(v: string) => {
            repKind.set(v);
            repCompare.set(v === "pl" ? "previous" : "none");
          }}
        />
        <View class="flex-1" />
        <View class="flex-row items-center gap-1">
          <Btn label="‹" small onPress={() => repPeriod.set(d()?.period.prev || "")} />
          <Text class={`text-[12.5px] px-1 ${C.ink}`}>{d()?.period.label || ""}</Text>
          <Btn label="›" small onPress={() => repPeriod.set(d()?.period.next || "")} />
        </View>
      </View>
      <If when={canCompare() || canTag()}>
        <View class="flex-row items-center gap-2">
          <If when={canCompare()}>
            <Text class={`text-[12px] ${C.dim}`}>Compare</Text>
            <Seg options={COMPARE_TABS} value={repCompare()} onPick={(v: string) => repCompare.set(v)} />
          </If>
          <View class="flex-1" />
          <If when={canTag()}>
            <View class="w-[200px]">
              <Pick value={repTag()} options={(tags() as Any) || []} onPick={(o: Any) => repTag.set(o.ticket)} placeholder="All tags" />
            </View>
          </If>
        </View>
      </If>
      <If when={Boolean(d()?.tag_note)}>
        <Text class={`text-[12px] ${C.dim}`}>{d()?.tag_note || ""}</Text>
      </If>
      <Loading res={data} />
      <If when={Boolean(d()) && repKind() === "ag" && d()?.cash_book}>
        <Panel pad class="gap-2">
          <Pill tone="in" label="Cash basis" />
          <Text class={`text-[13px] font-bold ${C.ink}`}>{d()?.title || ""}</Text>
          <Text class={`text-[12px] ${C.dim}`}>{d()?.note || ""}</Text>
          <Btn label="Open Unpaid" onPress={() => p.go("unpaid")} />
        </Panel>
      </If>
      <If when={Boolean(d()) && repKind() === "ag" && !d()?.cash_book}>
        <Panel>
          <View class={`flex-row gap-3 px-3 py-2 border-b ${C.line}`}>
            <Each of={d()?.columns || []}>
              {(c: Any, i: Any) => <Text class={`${i() === 0 ? "flex-1" : "w-[110px] text-right"} text-[10.5px] uppercase tracking-wider ${C.dim}`}>{c()}</Text>}
            </Each>
          </View>
          <Each of={d()?.rows || []}>
            {(r: Any) => (
              <View class={`flex-row gap-3 px-3 py-2 border-b ${C.line}`}>
                <Text class={`flex-1 text-[13px] ${C.ink}`}>{r().name}</Text>
                <Each of={r().cells}>{(c: Any) => <Text class={`w-[110px] text-right text-[12.5px] font-mono ${C.ink}`}>{c()}</Text>}</Each>
              </View>
            )}
          </Each>
        </Panel>
      </If>
      <If when={Boolean(d()) && repKind() !== "ag"}>
        <Panel>
          <View class={`flex-row gap-3 px-3 py-2 border-b ${C.line}`}>
            <Text class={`flex-1 text-[10.5px] uppercase tracking-wider ${C.dim}`}>{repKind() === "pl" || two() ? "Account" : d()?.columns?.[0] || ""}</Text>
            <If when={repKind() === "pl" || two()}>
              <Text class={`w-[130px] text-right text-[10.5px] uppercase tracking-wider ${C.dim}`}>{d()?.columns?.[0] || ""}</Text>
            </If>
            <If when={two()}>
              <Text class={`w-[130px] text-right text-[10.5px] uppercase tracking-wider ${C.dim}`}>{d()?.columns?.[1] || ""}</Text>
            </If>
            <If when={repKind() === "tb"}>
              <Text class={`w-[130px] text-right text-[10.5px] uppercase tracking-wider ${C.dim}`}>Debit</Text>
              <Text class={`w-[130px] text-right text-[10.5px] uppercase tracking-wider ${C.dim}`}>Credit</Text>
            </If>
            <If when={(repKind() === "bs" || repKind() === "cf") && !two()}>
              <Text class={`w-[130px] text-right text-[10.5px] uppercase tracking-wider ${C.dim}`}>{repKind() === "bs" ? "Balance" : "Amount"}</Text>
            </If>
          </View>
          <Each of={d()?.sections || []}>
            {(s: Any) => (
              <View>
                <If when={Boolean(s().title)}>
                  <Text class={`text-[10.5px] uppercase tracking-wider px-3 pt-3 pb-1 ${C.dim}`}>{s().title}</Text>
                </If>
                <Each of={s().rows}>{(r: Any) => <RepRow r={r()} cols={cols()} go={p.go} />}</Each>
                <If when={Boolean(s().total)}>
                  <View class={`flex-row gap-3 px-3 py-2 border-b ${C.line} ${C.raise}`}>
                    <Text class={`flex-1 text-[13px] font-bold ${C.ink}`}>{s().total?.label || ""}</Text>
                    <Text class={`w-[130px] text-right text-[12.5px] font-bold font-mono ${C.ink}`}>{s().total?.shown || ""}</Text>
                    <If when={two()}>
                      <Text class={`w-[130px] text-right text-[12.5px] font-bold font-mono ${C.mute}`}>{s().total?.prev_shown || ""}</Text>
                    </If>
                  </View>
                </If>
              </View>
            )}
          </Each>
          <View class={`flex-row gap-3 px-3 py-2.5 rounded-b-xl ${C.raise}`}>
            <Text class={`flex-1 text-[13px] font-bold ${C.ink}`}>{d()?.net?.label || ""}</Text>
            <If when={repKind() === "tb"}>
              <Text class={`w-[130px] text-right text-[12.5px] font-bold font-mono ${C.ink}`}>{d()?.net?.debit_shown || ""}</Text>
              <Text class={`w-[130px] text-right text-[12.5px] font-bold font-mono ${C.ink}`}>{d()?.net?.credit_shown || ""}</Text>
            </If>
            <If when={repKind() !== "tb"}>
              <Text class={`w-[130px] text-right text-[12.5px] font-bold font-mono ${C.ink}`}>{d()?.net?.shown || ""}</Text>
              <If when={two()}>
                <Text class={`w-[130px] text-right text-[12.5px] font-bold font-mono ${C.mute}`}>{d()?.net?.prev_shown || ""}</Text>
              </If>
            </If>
          </View>
        </Panel>
        <If when={repKind() === "bs"}>
          <Text class={`text-[12px] ${d()?.balanced ? C.green : C.red}`}>{d()?.check || ""}</Text>
        </If>
      </If>
      <Text class={`text-[12px] ${C.dim}`}>Click a line to see its transactions in the Ledger.</Text>
    </View>
  );
}
