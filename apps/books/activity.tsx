// Account activity: the drawer the Accounts page opens over the right side
// of the window. One account's month, oldest first, with the running
// balance in the account's own sense (a card shows owed rising). Rows carry
// no payee or memo; a row opens the entry in the Ledger's drawer.

const acMonth = signal("");

/** The account drawer: opening, rows with a running balance, closing. */
function AccountDrawer(p: { ticket: string; onClose: () => void; go: (page: string, filter?: string) => void }) {
  const data = resource(
    () => ({ v: ver(), a: accountDrawer() || p.ticket, m: acMonth() }),
    (k: Any) => {
      if (!k.a) return Promise.resolve(null);
      const args: Record<string, string> = { kind: "ac", account: k.a, period_kind: "month" };
      if (k.m) args.period = k.m;
      return buwiz.call("books_report", args);
    },
  );
  const d = () => data() as Any;
  const ledger = (entry?: string) => {
    ledgerAccount.set(d()?.account.ticket || "");
    ledgerMonth.set(d()?.period.key || "");
    acMonth.set("");
    p.onClose();
    p.go("ledger");
    if (entry) drawerEntry.set(entry);
  };
  const copyCsv = () => {
    const args: Record<string, string> = { kind: "ac", account: d()?.account.ticket || "", period_kind: "month" };
    if (d()?.period.key) args.period = d()?.period.key;
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
  const close = () => {
    acMonth.set("");
    p.onClose();
  };
  return (
    <View class={`absolute top-0 right-0 bottom-0 w-[380px] border-l ${C.panel} ${C.line}`}>
      <View class={`flex-row items-center gap-2 px-3.5 py-3 border-b ${C.line}`}>
        <View class="flex-1 gap-0.5">
          <Text class={`text-[14px] font-bold ${C.ink}`} numberOfLines={1}>
            {d()?.account.name || "Account"}
          </Text>
          <Text class={`text-[11.5px] font-mono ${C.dim}`}>{d()?.account.number || ""}</Text>
        </View>
        <Btn label="✕" small onPress={close} />
      </View>
      <View class={`flex-row items-center gap-1 px-3.5 py-2 border-b ${C.line}`}>
        <Btn label="‹" small onPress={() => acMonth.set(d()?.period.prev || "")} />
        <Text class={`flex-1 text-center text-[12.5px] ${C.ink}`}>{d()?.period.label || ""}</Text>
        <Btn label="›" small onPress={() => acMonth.set(d()?.period.next || "")} />
      </View>
      <Loading res={data} />
      <If when={Boolean(d())}>
        <Scroll class="flex-1">
          <View class={`flex-row gap-2 px-3.5 py-2 border-b ${C.line}`}>
            <Text class={`flex-1 text-[12.5px] font-semibold ${C.ink}`}>Opening</Text>
            <Text class={`w-[110px] text-right text-[12.5px] font-mono font-semibold ${C.ink}`}>{d()?.opening.shown || ""}</Text>
          </View>
          <View class={`flex-row gap-2 px-3.5 py-1.5 border-b ${C.line}`}>
            <Text class={`w-[52px] text-[10.5px] uppercase tracking-wider ${C.dim}`}>Date</Text>
            <Text class={`flex-1 text-[10.5px] uppercase tracking-wider ${C.dim}`}>What</Text>
            <Text class={`w-[90px] text-right text-[10.5px] uppercase tracking-wider ${C.dim}`}>Change</Text>
            <Text class={`w-[100px] text-right text-[10.5px] uppercase tracking-wider ${C.dim}`}>Balance</Text>
          </View>
          <Each of={d()?.rows || []} key={(r: Any) => String(r.entry)}>
            {(r: Any) => (
              <View class={`flex-row items-center gap-2 px-3.5 py-2 border-b ${C.line}`} onPress={() => ledger(r().entry)} activeStyle={{ opacity: 0.75 }}>
                <Text class={`w-[52px] text-[12px] ${C.mute}`}>{r().date_shown}</Text>
                <Text class={`flex-1 text-[12.5px] ${r().reversal ? C.dim : C.ink}`} numberOfLines={1}>
                  {r().what}
                </Text>
                <Text class={`w-[90px] text-right text-[12px] font-mono ${C.mute}`}>{r().change_shown}</Text>
                <Text class={`w-[100px] text-right text-[12.5px] font-mono ${C.ink}`}>{r().balance_shown}</Text>
              </View>
            )}
          </Each>
          <If when={Boolean(d()?.more)}>
            <Text class={`px-3.5 py-2 text-[12px] ${C.dim}`}>Showing the first 500 entries. The closing counts them all.</Text>
          </If>
          <View class={`flex-row gap-2 px-3.5 py-2 border-b ${C.line}`}>
            <Text class={`flex-1 text-[12.5px] font-semibold ${C.ink}`}>Closing</Text>
            <Text class={`w-[110px] text-right text-[12.5px] font-mono font-semibold ${C.ink}`}>{d()?.closing.shown || ""}</Text>
          </View>
        </Scroll>
        <View class={`flex-row gap-2 px-3.5 py-3 border-t ${C.line}`}>
          <Btn label="Open in Ledger" tone="go" onPress={() => ledger()} />
          <Btn label="Copy CSV" onPress={copyCsv} />
        </View>
      </If>
    </View>
  );
}
