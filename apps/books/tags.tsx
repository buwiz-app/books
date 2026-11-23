// Tags: departments and locations — the shared DimensionListPage layout from
// the old app's /departments and /locations routes: Back link + teal "New …"
// pill, left card (teal header, lookup bar, icon rows) and a right card that
// swaps between the empty, detail and New/Edit states. Books tags are flat
// and have no code/description/isActive, so the reference's parent tree,
// code line and deactivate affordances drop away; Spent/Transactions ride in
// the "Source" column.

const tagKind = signal("department");

// Page icon = the old app's DeptIcon (building) / MapPinIcon; the accent
// band shows departments' table glyph, locations keep the pin.
const tagIcon = (kind: string, accent = false) =>
  accent ? (kind === "department" ? "table" : "location") : kind === "department" ? "building" : "location";
const tagAvatar = (kind: string, cls: string, tint: string, accent = false) => (
  <Avatar src={ICONS[tagIcon(kind, accent)]} class={cls} imageClass={tint} fallbackDelay={Infinity} accessibilityLabel="" />
);
const tagNouns = (kind: string) =>
  kind === "department"
    ? {
        plural: "Departments",
        noun: "Department",
        newEntity: "New Department",
        searchPlaceholder: "Lookup department…",
        columnHeader: "Department",
        emptySearch: "No departments match your search",
        emptyList: "No departments yet",
        addFirst: "Add your first department",
        nameLabel: "Department Name",
        namePlaceholder: "Enter department name",
        selectPrompt: "Select a department",
        selectHint: "Click on a department to view details or edit",
      }
    : {
        plural: "Locations",
        noun: "Location",
        newEntity: "New Location",
        searchPlaceholder: "Lookup location…",
        columnHeader: "Location",
        emptySearch: "No locations match your search",
        emptyList: "No locations yet",
        addFirst: "Add your first location",
        nameLabel: "Location Name",
        namePlaceholder: "Enter location name",
        selectPrompt: "Select a location",
        selectHint: "Click on a location to view details or edit",
      };

function TagsPage(p: { go: (page: string, filter?: string) => void }) {
  const find = signal("");
  const selected = signal(null as Any);
  const mode = signal("" as string); // "" | "new" — Books has no rename/deactivate, so the form is New only
  const fname = signal("");
  const busy = signal(false);
  const data = resource(
    () => ({ v: ver(), k: tagKind() }),
    (k: Any) => buwiz.call("books_tags_list", { kind: k.k }),
  );
  const d = () => data() as Any;
  const L = () => tagNouns(tagKind());
  const rows = () => {
    const q = find().trim().toLowerCase();
    const list = (d()?.rows || []) as Any[];
    return q ? list.filter((r) => String(r.name || "").toLowerCase().includes(q)) : list;
  };
  const pick = (r: Any) => {
    selected.set(selected()?.ticket === r.ticket ? null : r);
    mode.set("");
  };
  const openNew = () => {
    fname.set("");
    mode.set("new");
    selected.set(null);
  };
  const save = async () => {
    const name = fname().trim();
    if (busy() || !name) return;
    busy.set(true);
    try {
      const r = await act("books_app_tag_create", { kind: tagKind(), name });
      const ticket = r?.tag || "";
      mode.set("");
      fname.set("");
      if (ticket) selected.set({ ticket, name });
    } catch {
      // act() already toasts the refusal.
    } finally {
      busy.set(false);
    }
  };
  const switchKind = (k: string) => {
    tagKind.set(k);
    selected.set(null);
    mode.set("");
    find.set("");
  };
  const sidePanel = () => {
    if (mode() === "new") return "form";
    if (selected()) return "view";
    return "empty";
  };
  const closeSide = () => {
    selected.set(null);
    mode.set("");
  };
  const sideCards = () => (
    <View class="flex-col gap-3">
      {/* Empty state */}
      <If when={sidePanel() === "empty"}>
        <View class={`rounded-[16px] border overflow-hidden items-center justify-center py-12 px-8 bg-white dark:bg-slate-800 ${C.line}`}>
          <View class="w-16 h-16 rounded-2xl items-center justify-center bg-[#e8f5f0] dark:bg-[#134e4a]/40 mb-4">
            {tagAvatar(tagKind(), "w-[30px] h-[30px]", "tint-[#2a9d8f] dark:tint-[#2dd4bf]")}
          </View>
          <Text class={`text-[14px] font-medium ${C.ink}`}>{L().selectPrompt}</Text>
          <Text class={`text-[12px] mt-1 text-center ${C.dim}`}>{L().selectHint}</Text>
        </View>
      </If>

      {/* Detail card */}
      <If when={sidePanel() === "view"}>
        <View class={`rounded-[16px] border overflow-hidden bg-white dark:bg-slate-800 ${C.line}`}>
          {/* Teal accent band */}
          <View class="bg-[#2a9d8f] items-center pt-5 pb-1">
            <View class="w-10 h-10 rounded-lg items-center justify-center bg-white/20">
              {tagAvatar(tagKind(), "w-[18px] h-[18px]", "tint-white", true)}
            </View>
          </View>
          {/* Header: teal icon + name + edit */}
          <View class="px-5 pt-3 pb-4">
            <View class="flex-row items-start gap-3">
              <View class="w-10 h-10 rounded-lg items-center justify-center bg-[#2a9d8f] shrink-0">
                {tagAvatar(tagKind(), "w-[16px] h-[16px]", "tint-white")}
              </View>
              <View class="flex-1 min-w-0">
                <Text class={`text-[17px] font-bold ${C.ink}`}>{selected()?.name || ""}</Text>
                <Text class={`text-[11px] mt-0.5 uppercase tracking-wider ${C.dim}`}>{L().noun}</Text>
              </View>
            </View>
          </View>
          {/* Activity box */}
          <View class="mx-4 mb-4 rounded-xl bg-[#f5f7fb] dark:bg-slate-900 p-4">
            <View class="flex-row items-center gap-2">
              {tagAvatar(tagKind(), "w-[13px] h-[13px]", "tint-[#2a9d8f] dark:tint-[#2dd4bf]")}
              <Text class={`text-[13px] font-semibold ${C.ink}`}>{selected()?.name || ""}</Text>
            </View>
            <Text class={`text-[11px] mt-0.5 ${C.dim}`}>{d()?.month.label || ""}</Text>
            <View class="flex-row items-baseline gap-4 mt-3">
              <View>
                <Text class={`text-[10px] font-semibold uppercase tracking-wider ${C.dim}`}>Spent</Text>
                <Text class={`text-[16px] font-semibold font-mono ${C.ink}`}>{selected()?.spent_shown || "₱ 0.00"}</Text>
              </View>
              <View>
                <Text class={`text-[10px] font-semibold uppercase tracking-wider ${C.dim}`}>Transactions</Text>
                <Text class={`text-[16px] font-semibold font-mono ${C.ink}`}>{String(selected()?.count ?? 0)}</Text>
              </View>
            </View>
          </View>
          {/* Footer — the reference deactivates here; Books links the ledger instead */}
          <View class="px-5 pb-4">
            <View
              class="h-[38px] rounded-lg border border-[#0d9488]/30 items-center justify-center transition-colors hover:bg-[#f0fdfa] dark:hover:bg-[#134e4a]/20"
              onPress={() => p.go("ledger")}
              activeStyle={{ opacity: 0.75 }}
            >
              <Text class="text-[13px] font-medium text-[#0d9488] dark:text-[#2dd4bf]">View in Ledger</Text>
            </View>
          </View>
        </View>
      </If>

      {/* New form card */}
      <If when={sidePanel() === "form"}>
        <View class={`rounded-[16px] border overflow-hidden bg-white dark:bg-slate-800 ${C.line}`}>
          <View class="bg-[#2a9d8f] px-5 py-4 flex-row items-center gap-3">
            <Text class="text-[15px] font-bold text-white">+</Text>
            <Text class="text-[15px] font-semibold text-white">{L().newEntity}</Text>
          </View>
          <View class="p-5 gap-4">
            <View class="gap-1.5">
              <Text class={`text-[11px] font-semibold uppercase tracking-wider ${C.dim}`}>{L().nameLabel}</Text>
              <TextBox label={L().nameLabel} value={fname()} onChange={(v) => fname.set(v)} placeholder={L().namePlaceholder} entry />
            </View>
            <View class={`flex-row items-center justify-end gap-3 pt-4 border-t ${C.line}`}>
              <View
                class={`h-[38px] px-5 rounded-lg border items-center justify-center transition-colors ${C.line} hover:bg-[#f8fafc] dark:hover:bg-slate-700`}
                onPress={closeSide}
                activeStyle={{ opacity: 0.75 }}
              >
                <Text class={`text-[13px] font-medium ${C.ink}`}>Cancel</Text>
              </View>
              <View
                class={`h-[38px] px-5 rounded-lg items-center justify-center transition-colors ${fname().trim() ? "bg-[#2a9d8f] hover:bg-[#0f766e]" : "bg-[#2a9d8f] opacity-50"}`}
                onPress={save}
                activeStyle={{ opacity: 0.85 }}
              >
                <Text class="text-[13px] font-semibold text-white">{busy() ? "Saving…" : "Save"}</Text>
              </View>
            </View>
          </View>
        </View>
      </If>
    </View>
  );
  return (
    <View class="flex-1 flex-col overflow-hidden relative">
      {/* Top Navigation — Back + New pill */}
      <View class="shrink-0 flex-row items-center justify-between px-3 sm:px-6 py-2 sm:py-3">
        <View
          class="flex-row items-center gap-1.5 py-1.5 pr-2"
          onPress={() => p.go("today")}
          activeStyle={{ opacity: 0.7 }}
        >
          <Text class={`text-[14px] font-medium transition-colors ${C.ink} hover:text-[#0d9488] dark:hover:text-[#2dd4bf]`}>‹ Back</Text>
        </View>
        <View
          class="h-[38px] flex-row items-center gap-1.5 rounded-full bg-[#0d9488] px-4 shadow-sm transition-colors hover:bg-[#0f766e]"
          onPress={openNew}
          activeStyle={{ opacity: 0.85 }}
        >
          <Text class="text-[14px] font-bold text-white">+</Text>
          <Text class="sm:hidden text-[13px] font-semibold text-white">New</Text>
          <Text class="hidden sm:inline text-[13px] font-semibold text-white">{L().newEntity}</Text>
        </View>
      </View>

      {/* Segment — Departments | Locations */}
      <View class="shrink-0 px-3 sm:px-6 pb-3">
        <Seg
          options={[
            ["department", "Departments"],
            ["location", "Locations"],
          ]}
          value={tagKind()}
          onPick={switchKind}
        />
      </View>

      {/* Two-Panel Layout */}
      <View class="flex-1 min-h-0 flex-row gap-6 px-3 pb-3 sm:px-6 sm:pb-6 overflow-hidden">
        {/* ── Left Card ── */}
        <View class={`flex-1 min-h-0 rounded-[16px] border overflow-hidden flex-col bg-white dark:bg-slate-800 ${C.line}`}>
          <View class="shrink-0 bg-[#2a9d8f] px-5 py-4 flex-row items-center gap-3">
            {tagAvatar(tagKind(), "w-[16px] h-[16px]", "tint-white", true)}
            <Text class="text-[16px] font-semibold text-white">{L().plural}</Text>
          </View>

          {/* Search bar — funnel, status chip, lookup, Clear */}
          <View class={`shrink-0 flex-row items-center gap-2 px-4 py-2.5 border-b bg-white dark:bg-slate-800 ${C.line}`}>
            <Text class={`text-[13px] ${C.dim}`}>⏷</Text>
            <View class="flex-row items-center gap-1 rounded-full border border-[#0d9488] bg-[#f0fdfa] px-2 py-0.5 dark:bg-[#134e4a]/30">
              <View class="w-1.5 h-1.5 rounded-full bg-[#0d9488]" />
              <Text class="text-[11px] font-semibold text-[#0d9488] dark:text-[#2dd4bf]">Active</Text>
            </View>
            <View class="flex-1">
              <TextBox
                label={L().noun}
                value={find()}
                onChange={(v) => find.set(v)}
                placeholder={L().searchPlaceholder}
                entry
              />
            </View>
            <If when={find().trim().length > 0}>
              <View
                class="px-2 py-1"
                onPress={() => find.set("")}
                activeStyle={{ opacity: 0.7 }}
              >
                <Text class="text-[12px] font-semibold text-[#0d9488] transition-colors hover:text-[#0f766e] dark:text-[#2dd4bf] dark:hover:text-[#5eead4]">Clear</Text>
              </View>
            </If>
          </View>

          {/* Column headers */}
          <View class={`flex-row gap-3 px-4 py-2 border-b bg-[#f8f9fb] dark:bg-slate-900 ${C.line}`}>
            <Text class={`flex-1 text-[10px] font-semibold uppercase tracking-wider ${C.dim}`}>{L().columnHeader}</Text>
            <Text class={`w-[120px] text-right text-[10px] font-semibold uppercase tracking-wider ${C.dim}`}>{`Spent in ${d()?.month.name || ""}`}</Text>
            <Text class={`w-[80px] text-right text-[10px] font-semibold uppercase tracking-wider ${C.dim}`}>Uses</Text>
          </View>

          <Loading res={data} />
          <If when={Boolean(d())}>
            <Scroll class="flex-1 min-h-0">
              <Each of={rows()} key={rowKey}>
                {(r: Any) => (
                  <View
                    class={`flex-row items-center gap-2.5 px-4 py-4 md:py-3 border-b transition-colors border-b-[#f0f1f3] dark:border-slate-700 ${
                      selected()?.ticket === r().ticket ? "bg-[#e8f4fe] dark:bg-slate-700" : "hover:bg-[#f8f9fb] dark:hover:bg-slate-800"
                    }`}
                    onPress={() => pick(r())}
                    activeStyle={{ opacity: 0.75 }}
                  >
                    <View class="w-9 h-9 rounded-lg items-center justify-center shrink-0 bg-[#e8f5f0] dark:bg-[#134e4a]/40">
                      {tagAvatar(tagKind(), "w-[14px] h-[14px]", "tint-[#2a9d8f] dark:tint-[#2dd4bf]")}
                    </View>
                    <View class="flex-1 min-w-0">
                      <Text class={`text-[14px] font-semibold truncate ${C.ink}`}>{r().name}</Text>
                    </View>
                    <Text class={`w-[120px] text-right text-[12.5px] font-mono ${C.ink}`}>{r().spent_shown}</Text>
                    <Text class={`w-[80px] text-right text-[12.5px] font-mono ${C.mute}`}>{String(r().count)}</Text>
                    <Text class={`w-[14px] text-right ${C.dim}`}>›</Text>
                  </View>
                )}
              </Each>
              <If when={rows().length === 0}>
                <View class="items-center gap-1.5 py-16">
                  {tagAvatar(tagKind(), "w-[40px] h-[40px]", "tint-[#e2e8f0] dark:tint-slate-700")}
                  <Text class={`text-[14px] font-medium ${C.mute}`}>
                    {find().trim() ? L().emptySearch : L().emptyList}
                  </Text>
                  <If when={!find().trim()}>
                    <View onPress={openNew} activeStyle={{ opacity: 0.7 }} class="mt-2 px-3 py-1.5">
                      <Text class="text-[14px] font-medium text-[#2a9d8f] hover:underline">{L().addFirst}</Text>
                    </View>
                  </If>
                </View>
              </If>
              {/* Extra room below lg so the last row scrolls clear of the FAB */}
              <View class="h-24 lg:hidden" />
            </Scroll>
          </If>
        </View>

        {/* ── Right Card: lg+ a quarter column, below lg a full-screen
              sheet (owner rule) ── */}
        <SidePanel
          open={() => sidePanel() !== "empty"}
          onBack={closeSide}
          body={sideCards}
        />
        <If when={sidePanel() === "empty"}>
          <PanelFab label="Open details" icon="panelRight" onPress={openNew} />
        </If>
      </View>
    </View>
  );
}
