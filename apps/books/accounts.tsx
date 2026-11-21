// Accounts (Category Manager): ported from buwiz-books CategoryManagerLayout
// with teal Category Manager header, search + filter bar, scrollable category
// list, and right-hand Filter by Type / Category Detail side panel.

const GROUP_CHIPS: [string, string][] = [
  ["all", "All"],
  ["Assets", "Assets"],
  ["Liabilities", "Liabilities"],
  ["Equity", "Equity"],
  ["Revenue", "Revenue"],
  ["Expenses", "Expenses"],
];

/** The account whose activity drawer is open (Accounts page). */
const accountDrawer = signal("");

function AccountsPage(p: { go: (page: string, filter?: string) => void }) {
  const group = signal("all");
  const find = signal("");
  const selected = signal(null as Any);
  const data = resource(
    () => ({ v: ver(), g: group() }),
    (k: Any) => buwiz.call("books_accounts_list", { group: k.g }),
  );
  const d = () => data() as Any;
  const shown = () => {
    const q = find().trim().toLowerCase();
    const rows = (data() as Any)?.rows || [];
    return q ? rows.filter((a: Any) => `${a.number} ${a.name} ${a.tag_label || ""}`.toLowerCase().includes(q)) : rows;
  };
  const imported = resource(
    () => ({ v: ver() }),
    () => buwiz.call("books_import_get", {}),
  );
  const waiting = () => {
    const c = (imported() as Any)?.counts;
    return c ? c.to_confirm + c.not_matched : 0;
  };
  const hasFilters = () => group() !== "all" || find().trim().length > 0;
  const open = (a: Any) => {
    selected.set(a);
  };
  return (
    <View class="flex-1 flex-col overflow-hidden">
      {/* Top Navigation — matches CategoryManagerLayout */}
      <View class="shrink-0 flex-row items-center justify-between px-6 py-3.5">
        <View
          class="flex-row items-center gap-1.5 py-1.5 pr-2"
          onPress={() => p.go("today")}
          activeStyle={{ opacity: 0.7 }}
        >
          <Text class={`text-[14px] font-medium ${C.ink}`}>‹ Back to Dashboard</Text>
        </View>
        <View class="flex-row items-center gap-2">
          <If when={waiting() > 0}>
            <View
              class="h-[36px] px-4 rounded-full border border-[#0d9488] items-center justify-center"
              onPress={() => p.go("import")}
              activeStyle={{ opacity: 0.8 }}
            >
              <Text class="text-[13px] font-medium text-[#0d9488] dark:text-[#2dd4bf]">
                {`Match imported (${String(waiting())})`}
              </Text>
            </View>
          </If>
          <View
            class="h-[36px] px-4 rounded-full bg-[#0d9488] items-center justify-center"
            onPress={() => p.go("import")}
            activeStyle={{ opacity: 0.85 }}
          >
            <Text class="text-[13px] font-semibold text-white">Import Categories</Text>
          </View>
        </View>
      </View>

      {/* Two-Panel Layout — fills remaining height */}
      <View class="flex-1 min-h-0 flex-row gap-6 px-6 pb-6 overflow-hidden">
        {/* Main Panel — rounded-2xl card with teal Category Manager header */}
        <View class={`flex-1 min-h-0 rounded-[16px] border overflow-hidden flex-col bg-white dark:bg-slate-800 ${C.line}`}>
          <View class="shrink-0 bg-[#0d9488] px-5 py-4 flex-row items-center justify-between">
            <Text class="text-[16px] font-semibold text-white">Category Manager</Text>
            <Text class="text-[12px] text-white/80">
              {`${String(shown().length)} ${shown().length === 1 ? "category" : "categories"}`}
            </Text>
          </View>

          {/* Search Bar + Filter Chips */}
          <View class={`shrink-0 flex-row items-center gap-2 px-4 py-2.5 border-b bg-white dark:bg-slate-800 ${C.line}`}>
            <View class="flex-1">
              <TextBox
                label="Lookup category"
                value={find()}
                onChange={(v) => find.set(v)}
                placeholder="Lookup category..."
                entry
              />
            </View>
            <If when={hasFilters()}>
              <View
                class="px-2 py-1"
                onPress={() => {
                  group.set("all");
                  find.set("");
                }}
                activeStyle={{ opacity: 0.7 }}
              >
                <Text class="text-[12px] font-semibold text-[#0d9488] dark:text-[#2dd4bf]">Clear</Text>
              </View>
            </If>
          </View>

          <View class={`shrink-0 flex-row items-center gap-1.5 flex-wrap px-4 py-2 border-b bg-[#f8f9fb] dark:bg-slate-900 ${C.line}`}>
            <Each of={GROUP_CHIPS}>
              {(g: Any) => <Chip label={g()[1]} on={group() === g()[0]} onPress={() => group.set(g()[0])} />}
            </Each>
          </View>

          <Loading res={data} />
          <If when={Boolean(d())}>
            <Scroll class="flex-1 min-h-0">
              <View class={`flex-row gap-3 px-4 py-2 border-b bg-[#f8f9fb] dark:bg-slate-900 ${C.line}`}>
                <Text class={`w-[64px] text-[11px] font-semibold uppercase tracking-wider ${C.dim}`}>No.</Text>
                <Text class={`flex-1 text-[11px] font-semibold uppercase tracking-wider ${C.dim}`}>Category</Text>
                <Text class={`w-[88px] text-[11px] font-semibold uppercase tracking-wider ${C.dim}`}>Type</Text>
                <Text class={`w-[120px] text-right text-[11px] font-semibold uppercase tracking-wider ${C.dim}`}>Balance</Text>
              </View>
              <Each of={shown()} key={rowKey}>
                {(a: Any) => (
                  <View
                    class={`flex-row items-center gap-3 px-4 py-2.5 border-b ${C.line} ${
                      selected()?.ticket === a().ticket ? "bg-[#f0fdfa] dark:bg-slate-700" : ""
                    }`}
                    onPress={() => open(a())}
                    activeStyle={{ opacity: 0.75 }}
                  >
                    <Text class={`w-[64px] text-[12.5px] font-mono ${a().off ? C.dim : C.mute}`}>{a().number}</Text>
                    <View class="flex-1 gap-0.5">
                      <Text class={`text-[13.5px] font-medium ${a().off ? C.dim : C.ink}`} numberOfLines={1}>
                        {a().name}
                      </Text>
                      <If when={a().off}>
                        <Text class={`text-[11px] ${C.dim}`}>Accrual books only</Text>
                      </If>
                    </View>
                    <View class="w-[88px]">
                      <View class={`rounded-[5px] px-1.5 py-0.5 self-start ${TAG[a().tag] || PILL.mute}`}>
                        <Text class={`text-[10px] font-bold uppercase tracking-wider ${TAG[a().tag] || PILL.mute}`}>
                          {a().tag_label}
                        </Text>
                      </View>
                    </View>
                    <Text class={`w-[120px] text-right text-[12.5px] font-mono ${a().off ? C.dim : C.ink}`}>
                      {a().balance_shown}
                    </Text>
                  </View>
                )}
              </Each>
              <If when={Boolean(d()?.note)}>
                <Text class={`text-[12px] px-4 py-3 ${C.dim}`}>{d()?.note || ""}</Text>
              </If>
            </Scroll>
          </If>
        </View>

        {/* Right Side Panel — Category Detail or Filter Summary (w-[340px]) */}
        <View class={`w-[340px] shrink-0 rounded-[16px] border overflow-hidden flex-col bg-white dark:bg-slate-800 ${C.line}`}>
          <If when={Boolean(selected())}>
            <View class="flex-1 flex-col">
              <View class={`flex-row items-center justify-between px-5 py-4 border-b ${C.line}`}>
                <View class="flex-1 gap-0.5">
                  <Text class={`text-[15px] font-semibold ${C.ink}`} numberOfLines={1}>
                    {selected()?.name || ""}
                  </Text>
                  <Text class={`text-[12px] font-mono ${C.dim}`}>{selected()?.number || ""}</Text>
                </View>
                <Btn label="✕" small onPress={() => selected.set(null)} />
              </View>
              <View class="p-5 gap-4 flex-1">
                <View class="gap-1">
                  <Text class={`text-[11px] font-semibold uppercase tracking-wider ${C.dim}`}>Current Balance</Text>
                  <Text class={`text-[24px] font-semibold font-mono ${C.ink}`}>{selected()?.balance_shown || "₱ 0.00"}</Text>
                </View>
                <View class="flex-row items-center gap-2">
                  <View class={`rounded-[5px] px-2 py-0.5 ${TAG[selected()?.tag] || PILL.mute}`}>
                    <Text class={`text-[11px] font-bold uppercase tracking-wider ${TAG[selected()?.tag] || PILL.mute}`}>
                      {selected()?.tag_label || ""}
                    </Text>
                  </View>
                  <If when={Boolean(selected()?.off)}>
                    <Pill tone="mute" label="Accrual only" />
                  </If>
                </View>
                <If when={!selected()?.off}>
                  <View class="gap-2 pt-2">
                    <View
                      class="h-[38px] rounded-[10px] bg-[#0d9488] items-center justify-center"
                      onPress={() => accountDrawer.set(selected()?.ticket || "")}
                      activeStyle={{ opacity: 0.85 }}
                    >
                      <Text class="text-[13px] font-semibold text-white">View Monthly Activity</Text>
                    </View>
                    <Btn
                      label="Open in Ledger"
                      onPress={() => openLedger({ account: selected()?.ticket || "" })}
                    />
                  </View>
                </If>
              </View>
            </View>
          </If>
          <If when={!selected()}>
            <View class="p-5 gap-4">
              <View class="gap-1">
                <Text class={`text-[15px] font-semibold ${C.ink}`}>Category Overview</Text>
                <Text class={`text-[12.5px] ${C.mute}`}>
                  Select a category from the list to inspect its balance, monthly activity, or open its ledger.
                </Text>
              </View>
              <View class={`border-t pt-3 gap-2 ${C.line}`}>
                <Text class={`text-[11px] font-semibold uppercase tracking-wider ${C.dim}`}>Filter by Type</Text>
                <Each of={GROUP_CHIPS}>
                  {(g: Any) => (
                    <View
                      class={`flex-row items-center justify-between px-3 py-2 rounded-[10px] border ${
                        group() === g()[0]
                          ? "bg-[#f0fdfa] dark:bg-slate-700 border-[#0d9488]"
                          : `bg-transparent ${C.line}`
                      }`}
                      onPress={() => group.set(g()[0])}
                      activeStyle={{ opacity: 0.75 }}
                    >
                      <Text class={`text-[13px] font-medium ${C.ink}`}>{g()[1]}</Text>
                      <Text class={`text-[12px] ${C.dim}`}>{group() === g()[0] ? "Active" : ""}</Text>
                    </View>
                  )}
                </Each>
              </View>
            </View>
          </If>
        </View>
      </View>
    </View>
  );
}
