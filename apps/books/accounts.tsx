// Accounts: the chart with balances, by group. On a cash book the
// receivable and payable accounts show as accrual-only and dim.

const GROUP_CHIPS: [string, string][] = [
  ["all", "All"],
  ["Assets", "Assets"],
  ["Liabilities", "Liabilities"],
  ["Equity", "Equity"],
  ["Revenue", "Revenue"],
  ["Expenses", "Expenses"],
];

function AccountsPage(p: { go: (page: string, filter?: string) => void }) {
  const group = signal("all");
  const data = resource(
    () => ({ v: ver(), g: group() }),
    (k: Any) => buwiz.call("books_accounts_list", { group: k.g }),
  );
  const d = () => data() as Any;
  const imported = resource(
    () => ({ v: ver() }),
    () => buwiz.call("books_import_get", {}),
  );
  const waiting = () => {
    const c = (imported() as Any)?.counts;
    return c ? c.to_confirm + c.not_matched : 0;
  };
  const open = (a: Any) => {
    if (a.off) return;
    openLedger({ account: a.ticket });
  };
  return (
    <View class="gap-3.5">
      <Head title="Accounts" sub="chart of accounts · Philippines small business" />
      <If when={waiting() > 0}>
        <Panel pad class="flex-row items-center gap-2.5">
          <Pill tone="draft" label={`${String(waiting())} to match`} />
          <Text class={`flex-1 text-[12.5px] ${C.mute}`}>Imported accounts wait for your match. Boxes their lines could feed are held until then.</Text>
          <Btn label="Match your accounts" tone="go" onPress={() => p.go("import")} />
        </Panel>
      </If>
      <View class="flex-row gap-1.5 flex-wrap">
        <Each of={GROUP_CHIPS}>{(g: Any) => <Chip label={g()[1]} on={group() === g()[0]} onPress={() => group.set(g()[0])} />}</Each>
      </View>
      <Loading res={data} />
      <If when={Boolean(d())}>
        <Panel>
          <View class={`flex-row gap-3 px-3 py-2 border-b ${C.line}`}>
            <Text class={`w-[64px] text-[10.5px] uppercase tracking-wider ${C.dim}`}>No.</Text>
            <Text class={`flex-1 text-[10.5px] uppercase tracking-wider ${C.dim}`}>Account</Text>
            <Text class={`w-[84px] text-[10.5px] uppercase tracking-wider ${C.dim}`}>Type</Text>
            <Text class={`w-[120px] text-right text-[10.5px] uppercase tracking-wider ${C.dim}`}>Balance</Text>
            <View class="w-[14px]" />
          </View>
          <Each of={d()?.rows || []} key={rowKey}>
            {(a: Any) => (
              <View class={`flex-row items-center gap-3 px-3 py-2 border-b ${C.line}`} onPress={() => open(a())} activeStyle={{ opacity: a().off ? 1 : 0.75 }}>
                <Text class={`w-[64px] text-[12.5px] font-mono ${a().off ? C.dim : C.ink}`}>{a().number}</Text>
                <View class="flex-1 gap-0.5">
                  <Text class={`text-[13px] font-semibold ${a().off ? C.dim : C.ink}`} numberOfLines={1}>
                    {a().name}
                  </Text>
                  <If when={a().off}>
                    <Text class={`text-[11.5px] ${C.dim}`}>Accrual books only</Text>
                  </If>
                </View>
                <View class="w-[84px]">
                  <View class={`rounded-[5px] px-1.5 py-0.5 self-start ${TAG[a().tag] || PILL.mute}`}>
                    <Text class={`text-[10px] font-bold uppercase tracking-wider ${TAG[a().tag] || PILL.mute}`}>{a().tag_label}</Text>
                  </View>
                </View>
                <Text class={`w-[120px] text-right text-[12.5px] font-mono ${a().off ? C.dim : C.ink}`}>{a().balance_shown}</Text>
                <Text class={`w-[14px] text-right ${C.dim}`}>{a().off ? "" : "›"}</Text>
              </View>
            )}
          </Each>
        </Panel>
        <If when={Boolean(d()?.note)}>
          <Text class={`text-[12px] ${C.dim}`}>{d()?.note || ""}</Text>
        </If>
      </If>
    </View>
  );
}
