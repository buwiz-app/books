// Accounts (Category Manager): ported from buwiz-books' accounts route —
// Back to Dashboard + "+ New Category" split pill, a rounded-2xl panel with
// the teal header, the search row (status chip + Lookup input + Clear),
// the icon-tile Filter by Type side panel, and the New Category modal
// (NewCategoryModal/NewCategoryForm).

/** The eight category types, with the glyph and tone the old app's icon tiles use. */
const CAT_TYPES: [string, string, string, string][] = [
  ["asset", "Asset", "▤", "#14b8a6"],
  ["revenue", "Revenue", "↗", "#3b82f6"],
  ["other_income", "Other Income", "◍", "#0ea5e9"],
  ["liability", "Liability", "▭", "#fb923c"],
  ["expense", "Operating Expenses", "◫", "#f87171"],
  ["other_expense", "Other Expenses", "×", "#fb7185"],
  ["equity", "Equity", "◆", "#a855f7"],
  ["cost_of_revenue", "Cost of Revenue", "⬡", "#f97316"],
];
const catTypeOf = (row: Any) => String(row.type || "");
const catGlyph = (row: Any) => {
  const t = CAT_TYPES.find((x) => x[0] === catTypeOf(row));
  return { glyph: t ? t[2] : "▤", color: t ? t[3] : "#94a3b8" };
};

/** The account whose activity drawer is open (Accounts page). */
const accountDrawer = signal("");

function AccountsPage(p: { go: (page: string, filter?: string) => void }) {
  const typeFilter = signal("");
  const activeOnly = signal(true);
  const find = signal("");
  const selected = signal(null as Any);
  const catMenu = signal(false);
  const newCat = signal(false);
  const mFilter = signal(false);
  const ncName = signal("");
  const ncNumber = signal("");
  const ncType = signal("asset");
  const ncBusy = signal(false);
  let catMenuAnchor: ViewRef | undefined;
  const data = resource(
    () => ({ v: ver() }),
    () => buwiz.call("books_accounts_list", { group: "all" }),
  );
  const d = () => data() as Any;
  const shown = () => {
    const q = find().trim().toLowerCase();
    let rows = ((data() as Any)?.rows || []) as Any[];
    if (activeOnly()) rows = rows.filter((a: Any) => !a.off);
    if (typeFilter()) rows = rows.filter((a: Any) => catTypeOf(a) === typeFilter());
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
  const hasFilters = () => typeFilter() !== "" || find().trim().length > 0 || !activeOnly();
  const open = (a: Any) => {
    selected.set(a);
  };
  /** Next free account number when the field is left blank: highest + 10. */
  const nextNumber = () => {
    let hi = 0;
    for (const a of (d()?.rows || []) as Any[]) {
      const n = parseInt(String(a.number || "0"), 10);
      if (!isNaN(n) && n > hi) hi = n;
    }
    return String(hi + 10).slice(0, 12);
  };
  const createCategory = async () => {
    if (ncBusy() || !ncName().trim()) return;
    ncBusy.set(true);
    const name = ncName().trim();
    const number = ncNumber().trim() || nextNumber();
    try {
      const r = await act("books_app_account_create", {
        number,
        name,
        kind: "unsure",
        type: ncType(),
      });
      newCat.set(false);
      ncName.set("");
      ncNumber.set("");
      ncType.set("asset");
      const acct = r?.account || {};
      if (acct.ticket) selected.set({ ...acct, name, number });
    } catch {
      // act() already toasts the refusal.
    } finally {
      ncBusy.set(false);
    }
  };
  const closeSide = () => {
    selected.set(null);
    mFilter.set(false);
  };
  const sideInner = () => (
    <View class="flex-1 flex-col">
      <If when={Boolean(selected())}>
        <View class="flex-1 flex-col">
          <View class={`flex-row items-center justify-between px-5 py-4 border-b ${C.line}`}>
            <View class="flex-1 gap-0.5">
              <Text class={`text-[15px] font-semibold ${C.ink} truncate`}>
                {selected()?.name || ""}
              </Text>
              <Text class={`text-[12px] font-mono ${C.dim}`}>{selected()?.number || ""}</Text>
            </View>
            <Btn label="✕" small onPress={closeSide} />
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
                  class="h-[38px] rounded-[10px] bg-[#0d9488] items-center justify-center transition-colors hover:bg-[#0f766e]"
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
        <View class="flex-1 flex-col">
          <View class={`shrink-0 px-5 py-4 border-b ${C.line}`}>
            <Text class={`text-[15px] font-semibold ${C.ink}`}>Filter by Type</Text>
          </View>
          <View class="p-4 gap-2 flex-row flex-wrap">
            <Each of={CAT_TYPES}>
              {(t: Any) => {
                const on = () => typeFilter() === t()[0];
                return (
                  <View
                    class={`w-[calc(50%-4px)] rounded-[10px] border items-center gap-1.5 px-2 py-3 transition-colors ${
                      on()
                        ? "border-[#0d9488] bg-[#f0fdfa] dark:bg-[#134e4a]/30"
                        : `${C.line} hover:border-[#cbd5e1] dark:hover:border-[#3f3f46] hover:bg-[#f8f9fb] dark:hover:bg-[#26262a]`
                    }`}
                    onPress={() => typeFilter.set(on() ? "" : t()[0])}
                    activeStyle={{ opacity: 0.7 }}
                  >
                    <View class="w-8 h-8 rounded-lg items-center justify-center" style={{ backgroundColor: `${t()[3]}1c` }}>
                      <Text class="text-[15px]" style={{ color: t()[3] }}>{t()[2]}</Text>
                    </View>
                    <Text class={`text-[11px] font-medium text-center ${on() ? "text-[#0d9488] dark:text-[#2dd4bf]" : C.mute}`}>{t()[1]}</Text>
                  </View>
                );
              }}
            </Each>
          </View>
        </View>
      </If>
    </View>
  );
  return (
    <View class="flex-1 flex-col overflow-hidden relative">
      {/* Top Navigation — Back + New Category split pill */}
      <View class="shrink-0 flex-row items-center justify-between px-3 sm:px-6 py-2 sm:py-3">
        <View
          class="flex-row items-center gap-1.5 py-1.5 pr-2"
          onPress={() => p.go("today")}
          activeStyle={{ opacity: 0.7 }}
        >
          <Text class={`text-[14px] font-medium transition-colors ${C.ink} hover:text-[#0d9488] dark:hover:text-[#2dd4bf]`}>‹</Text>
          <Text class={`hidden sm:inline text-[14px] font-medium transition-colors ${C.ink} hover:text-[#0d9488] dark:hover:text-[#2dd4bf]`}>Back to Dashboard</Text>
        </View>
        <View ref={catMenuAnchor} class="relative flex-row items-stretch rounded-full bg-[#0d9488] shadow-sm">
          <View
            class="px-4 pl-5 h-[38px] flex-row items-center gap-1.5 rounded-l-full transition-colors hover:bg-[#0f766e]"
            onPress={() => newCat.set(true)}
            activeStyle={{ opacity: 0.85 }}
          >
            <Text class="text-[14px] font-bold text-white">+</Text>
            <Text class="text-[13px] font-semibold text-white">New Category</Text>
          </View>
          <View class="w-px my-2 bg-white/30" />
          <View
            class="px-3 h-[38px] items-center justify-center rounded-r-full transition-colors hover:bg-[#0f766e]"
            onPress={() => catMenu.set(!catMenu())}
            activeStyle={{ opacity: 0.85 }}
          >
            <Text class="text-[12px] text-white">⌄</Text>
          </View>
        </View>
      </View>

      {/* Two-Panel Layout — fills remaining height */}
      <View class="flex-1 min-h-0 flex-row gap-6 px-3 pb-3 sm:px-6 sm:pb-6 overflow-hidden">
        {/* Main Panel — rounded-2xl card with teal Category Manager header */}
        <View class={`flex-1 min-h-0 rounded-[16px] border overflow-hidden flex-col bg-white dark:bg-slate-800 ${C.line}`}>
          <View class="shrink-0 bg-[#2a9d8f] px-5 py-4 flex-row items-center justify-between">
            <Text class="text-[16px] font-semibold text-white">Category Manager</Text>
            <Text class="text-[12px] text-white/80">
              {`${String(shown().length)} ${shown().length === 1 ? "category" : "categories"}`}
            </Text>
          </View>

          {/* Search row — funnel, status chip, Lookup input, Clear */}
          <View class={`shrink-0 flex-row items-center gap-2 px-4 py-2.5 border-b bg-white dark:bg-slate-800 ${C.line}`}>
            <View class="hidden lg:flex">
              <Text class={`text-[13px] ${C.dim}`}>⏷</Text>
            </View>
            <View
              class="lg:hidden h-9 w-9 items-center justify-center rounded-md"
              onPress={() => mFilter.set(true)}
              activeStyle={{ opacity: 0.7 }}
            >
              <Text class={`text-[13px] transition-colors ${C.dim} hover:text-[#0d9488]`}>⏷</Text>
            </View>
            <View
              class={`flex-row items-center gap-1 rounded-full border px-2 py-0.5 transition-colors ${activeOnly() ? "border-[#0d9488] bg-[#f0fdfa] dark:bg-[#134e4a]/30" : `${C.line} hover:border-[#cbd5e1] dark:hover:border-[#3f3f46]`}`}
              onPress={() => activeOnly.set(!activeOnly())}
              activeStyle={{ opacity: 0.7 }}
            >
              <View class={`w-1.5 h-1.5 rounded-full ${activeOnly() ? "bg-[#0d9488]" : "bg-[#94a3b8]"}`} />
              <Text class={`text-[11px] font-semibold ${activeOnly() ? "text-[#0d9488] dark:text-[#2dd4bf]" : C.dim}`}>Active</Text>
              <If when={activeOnly()}>
                <Text class="text-[11px] text-[#0d9488] dark:text-[#2dd4bf]">×</Text>
              </If>
            </View>
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
                  typeFilter.set("");
                  find.set("");
                  activeOnly.set(true);
                }}
                activeStyle={{ opacity: 0.7 }}
              >
                <Text class="text-[12px] font-semibold text-[#0d9488] transition-colors hover:text-[#0f766e] dark:text-[#2dd4bf] dark:hover:text-[#5eead4]">Clear</Text>
              </View>
            </If>
          </View>

          <Loading res={data} />
          <If when={Boolean(d())}>
            <Scroll class="flex-1 min-h-0">
              <View class={`flex-row gap-3 px-4 py-2 border-b bg-[#f8f9fb] dark:bg-slate-900 ${C.line}`}>
                <Text class={`flex-1 text-[11px] font-semibold uppercase tracking-wider ${C.dim}`}>Category</Text>
                <Text class={`w-[110px] text-right text-[11px] font-semibold uppercase tracking-wider ${C.dim}`}>Balance</Text>
              </View>
              <Each of={shown()} key={rowKey}>
                {(a: Any) => {
                  const g = catGlyph(a());
                  return (
                    <View
                      class={`flex-row items-center gap-3 px-4 py-3 border-b transition-colors border-b-[#f0f1f3] dark:border-slate-700 ${
                        selected()?.ticket === a().ticket ? "bg-[#e8f4fe] dark:bg-slate-700" : "hover:bg-[#f8f9fb] dark:hover:bg-[#1c1c1f]"
                      }`}
                      onPress={() => open(a())}
                      activeStyle={{ opacity: 0.75 }}
                    >
                      <View class="w-9 h-9 rounded-lg items-center justify-center shrink-0" style={{ backgroundColor: `${g.color}1c` }}>
                        <Text class="text-[15px]" style={{ color: g.color }}>{g.glyph}</Text>
                      </View>
                      <View class="flex-1 min-w-0">
                        <View class="flex-row items-baseline gap-1.5 min-w-0">
                          <Text class={`text-[13.5px] font-semibold ${a().off ? C.dim : C.ink} truncate`}>{a().name}</Text>
                          <Text class={`text-[11.5px] font-mono ${C.dim}`}>{a().number}</Text>
                        </View>
                        <Text class={`text-[11.5px] truncate ${a().off ? C.dim : C.mute}`}>
                          {a().off ? "Accrual books only" : `${a().tag_label || ""} · ${a().kind || "account"}`}
                        </Text>
                      </View>
                      <Text class={`w-[110px] text-right text-[12.5px] font-mono ${a().off ? C.dim : C.ink}`}>
                        {a().balance_shown}
                      </Text>
                    </View>
                  );
                }}
              </Each>
              <If when={shown().length === 0}>
                <View class="items-center gap-1.5 py-10">
                  <Text class="text-[22px] opacity-40">▤</Text>
                  <Text class={`text-[13px] ${C.mute}`}>No categories match.</Text>
                  <Text class={`text-[12px] ${C.dim}`}>Try a different search or clear the filters.</Text>
                </View>
              </If>
              <If when={Boolean(d()?.note)}>
                <Text class={`text-[12px] px-4 py-3 ${C.dim}`}>{d()?.note || ""}</Text>
              </If>
              {/* Extra room below lg so the last row scrolls clear of the FAB */}
              <View class="h-24 lg:hidden" />
            </Scroll>
          </If>
        </View>

        {/* Right Side Panel — Filter by Type tiles or Category Detail:
            lg+ a quarter column, below lg a full-screen sheet (owner rule) */}
        <SidePanel
          open={() => Boolean(selected()) || mFilter()}
          onBack={closeSide}
          card
          class={`rounded-[16px] border overflow-hidden flex-col bg-white dark:bg-slate-800 ${C.line}`}
          body={sideInner}
        />
        <If when={!Boolean(selected()) && !mFilter()}>
          <PanelFab label="Open details" glyph="☰" onPress={() => mFilter.set(true)} />
        </If>
      </View>

      {/* "+ New Category" chevron menu */}
      <If when={catMenu()}>
        <Overlay
          anchor={catMenuAnchor}
          placement="bottom"
          align="end"
          offset={2}
          margin={8}
          onPressOutside={() => catMenu.set(false)}
          onCancel={() => catMenu.set(false)}
        >
          <View class={`w-[210px] rounded-xl border py-1 shadow-lg ${C.panel} ${C.line}`}>
            <View
              class="flex-row items-center gap-2.5 px-4 py-3 transition-colors hover:bg-[#f8fafc] dark:hover:bg-[#1c1c1f]"
              activeStyle={{ opacity: 0.7 }}
              onPress={() => {
                catMenu.set(false);
                p.go("import");
              }}
            >
              <Text class="text-[13px] text-[#94a3b8]">⇪</Text>
              <Text class={`text-[13px] ${C.ink}`}>Import Categories</Text>
            </View>
            <If when={waiting() > 0}>
              <View
                class={`flex-row items-center gap-2.5 px-4 py-3 border-t transition-colors hover:bg-[#f8fafc] dark:hover:bg-[#1c1c1f] ${C.line}`}
                activeStyle={{ opacity: 0.7 }}
                onPress={() => {
                  catMenu.set(false);
                  p.go("import");
                }}
              >
                <Text class="text-[13px] text-[#94a3b8]">⚯</Text>
                <Text class={`text-[13px] ${C.ink}`}>{`Match imported (${String(waiting())})`}</Text>
              </View>
            </If>
          </View>
        </Overlay>
      </If>

      {/* New Category modal — NewCategoryModal + NewCategoryForm */}
      <If when={newCat()}>
        <Overlay
          anchor={{ x: 320, y: 110 }}
          placement="bottom"
          align="start"
          offset={4}
          margin={8}
          onCancel={() => newCat.set(false)}
          onPressOutside={() => newCat.set(false)}
        >
          <View class={`w-[360px] rounded-[16px] border overflow-hidden shadow-xl ${C.panel} ${C.line}`}>
            <View class="bg-[#2a9d8f] px-5 py-4 flex-row items-center gap-2.5">
              <Text class="text-[16px] font-semibold text-white">▤</Text>
              <Text class="text-[15px] font-semibold text-white">New Category</Text>
            </View>
            <View class="p-5 gap-3.5">
              <View class="gap-1.5">
                <Text class={`text-[11px] font-medium ${C.mute}`}>Category Name</Text>
                <TextBox label="Category name" value={ncName()} onChange={(v) => ncName.set(v)} placeholder="e.g., Office Supplies" entry />
              </View>
              <View class="gap-1.5">
                <Text class={`text-[11px] font-medium ${C.mute}`}>Account Number</Text>
                <TextBox label="Account number" value={ncNumber()} onChange={(v) => ncNumber.set(v.replace(/[^0-9]/g, "").slice(0, 12))} placeholder="e.g., 61000" entry />
              </View>
              <View class="gap-1.5">
                <Text class={`text-[11px] font-medium ${C.mute}`}>Category Type</Text>
                <View class="flex-row flex-wrap gap-1.5">
                  <Each of={CAT_TYPES}>
                    {(t: Any) => (
                      <View
                        class={`flex-row items-center gap-1.5 rounded-lg border px-2.5 py-1.5 transition-colors ${
                          ncType() === t()[0]
                            ? "border-[#0d9488] bg-[#f0fdfa] dark:bg-[#134e4a]/30"
                            : `${C.line} hover:border-[#cbd5e1] dark:hover:border-[#3f3f46]`
                        }`}
                        onPress={() => ncType.set(t()[0])}
                        activeStyle={{ opacity: 0.7 }}
                      >
                        <Text class="text-[12px]" style={{ color: t()[3] }}>{t()[2]}</Text>
                        <Text class={`text-[11.5px] ${ncType() === t()[0] ? `font-semibold ${C.ink}` : C.mute}`}>{t()[1]}</Text>
                      </View>
                    )}
                  </Each>
                </View>
              </View>
              <View class="flex-row gap-2 pt-1">
                <View class="flex-1">
                  <Btn label="Cancel" onPress={() => newCat.set(false)} />
                </View>
                <View
                  class={`flex-1 h-[34px] rounded-lg items-center justify-center transition-colors ${ncName().trim() ? "bg-[#0d9488] hover:bg-[#0f766e]" : "bg-[#0d9488] opacity-40"}`}
                  onPress={createCategory}
                  activeStyle={{ opacity: 0.85 }}
                >
                  <Text class="text-[13px] font-semibold text-white">{ncBusy() ? "Creating…" : "Create Category"}</Text>
                </View>
              </View>
            </View>
          </View>
        </Overlay>
      </If>
    </View>
  );
}
