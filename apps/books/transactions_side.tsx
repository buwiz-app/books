// Transactions COA sidebar: a 1:1 port of the old app's ledger/AccountSidebar.tsx.
// Books' chart of accounts is flat (no parent accounts), so the tree is two
// levels: a type-group root row carries the old root-node look (colour tile,
// bold name, icon->chevron swap on hover, tri-state cascade checkbox) and the
// group's accounts are the child rows (dot bullet, name + number, badge).
// The old per-account transaction count has no Books tool; the badge slot
// carries the account's balance_shown in the same 10px tabular style.
//
// Wire-in (Agent D, in transactions.tsx — replace each <TxSide />):
//   <TxSidebar
//     accounts={accts}                       // books_accounts_list rows (active)
//     selectedId={txAccount()}               // "" = All accounts
//     onSelect={(t) => { txAccount.set(t); txSideOpen.set(false); }}
//     checkedIds={txChecked()}               // string[] of checked tickets
//     onToggleAccount={(id, node) => txCheckedSet(txSideToggled(txSideNodeIds(node), txChecked()))}
//     onSelectAll={() => txCheckedSet(txSideAllIds(accts))}
//     onClearSelection={() => txCheckedSet([])}
//   />
// txSideNodeIds(node) is the old getAllDescendantIds: a group node yields its
// member tickets, an account row its own. txSideToggled is the old
// handleToggleAccount cascade (all checked -> remove all, else add all).
// The old sidebar has no Select all/Clear row of its own (its "Select all"
// lives in the MultiCheckboxCombobox footer); pass onSelectAll/onClearSelection
// to draw that same footer here.

const txSideCollapsed = signal<string[]>([]); // collapsed group names; groups start open like the old depth-0 nodes

const TX_SIDE_ICONS = {
  search: svgIcon('<circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>', 2),
  check: svgIcon('<polyline points="20 6 9 17 4 12"/>', 3),
  minus: svgIcon('<line x1="5" y1="12" x2="19" y2="12"/>', 3),
  chevronDown: svgIcon('<path d="M6 9l6 6 6-6"/>', 2.5),
  chevronRight: svgIcon('<path d="M9 18l6-6-6-6"/>', 2.5),
};

// The old TYPE_COLORS for the account types Books' groups cover.
const TX_SIDE_TYPES: Record<string, { bg: string; text: string; label: string }> = {
  Assets: { bg: "#dcfce7", text: "#16a34a", label: "Assets" },
  Liabilities: { bg: "#fef3c7", text: "#d97706", label: "Liabilities" },
  Equity: { bg: "#dbeafe", text: "#2563eb", label: "Equity" },
  Revenue: { bg: "#d1fae5", text: "#059669", label: "Revenue" },
  Expenses: { bg: "#fee2e2", text: "#dc2626", label: "Expenses" },
};
const TX_SIDE_GROUPS = ["Assets", "Liabilities", "Equity", "Revenue", "Expenses"];
const txSideConf = (group: string) =>
  TX_SIDE_TYPES[group] || { bg: "#f1f5f9", text: "#64748b", label: group || "Accounts" };

/** Tickets a node press cascades over (the old getAllDescendantIds). */
function txSideNodeIds(node: Any): string[] {
  if (node && node.accounts) return (node.accounts as Any[]).map((a) => String(a.ticket));
  return [String(node.ticket)];
}
/** Every checkable ticket (Select all). */
function txSideAllIds(accounts: Any[]): string[] {
  return accounts.map((a) => String(a.ticket));
}
/** The old handleToggleAccount cascade: all ids checked -> remove, else add. */
function txSideToggled(ids: string[], checked: string[]): string[] {
  const all = ids.every((id) => checked.includes(id));
  const out = checked.filter((id) => !ids.includes(id));
  return all ? out : [...out, ...ids];
}
/** Tri-state of a node against the checked set (the old getCheckState). */
function txSideCheckState(node: Any, checked: string[]): "none" | "some" | "all" {
  const ids = txSideNodeIds(node);
  const n = ids.filter((id) => checked.includes(id)).length;
  if (n === 0) return "none";
  return n === ids.length ? "all" : "some";
}

/** The old SidebarCheckbox: 18px box, #2a9d8f fill, white check or minus. */
function TxSideCheck(p: { state: "none" | "some" | "all"; onPress: () => void }) {
  const on = () => p.state !== "none";
  return (
    <View
      class="w-[18px] h-[18px] rounded border items-center justify-center shrink-0 cursor-pointer transition-all"
      style={{
        borderColor: on() ? "#2a9d8f" : "#cbd5e1",
        backgroundColor: on() ? "#2a9d8f" : "transparent",
      }}
      onPress={p.onPress}
      accessibilityLabel={on() ? "Uncheck category" : "Check category"}
    >
      <If when={p.state === "all"}>
        <Avatar src={TX_SIDE_ICONS.check} class="w-2.5 h-2.5" imageClass="tint-white" fallbackDelay={Infinity} accessibilityLabel="" />
      </If>
      <If when={p.state === "some"}>
        <Avatar src={TX_SIDE_ICONS.minus} class="w-2.5 h-2.5" imageClass="tint-white" fallbackDelay={Infinity} accessibilityLabel="" />
      </If>
    </View>
  );
}

/** Right slot: hover/checked shows the checkbox, else the badge (the old
 *  transaction count; here balance_shown — same slot and style). */
function TxSideRight(p: {
  node: Any;
  badge: string;
  checked: string[];
  onToggle?: (id: string, node: Any) => void;
}) {
  const state = () => txSideCheckState(p.node, p.checked || []);
  const toggle = () => p.onToggle?.(String(p.node.group ? `group:${p.node.group}` : p.node.ticket), p.node);
  if (!p.onToggle) {
    return p.badge ? <Text class="text-[10px] font-medium text-[#94a3b8] dark:text-slate-500 tabular-nums shrink-0">{p.badge}</Text> : <View />;
  }
  return (
    <>
      <If when={state() !== "none"}>
        <TxSideCheck state={state()} onPress={toggle} />
      </If>
      <If when={state() === "none"}>
        <View class="hidden group-hover:flex">
          <TxSideCheck state="none" onPress={toggle} />
        </View>
        <If when={Boolean(p.badge)}>
          <Text class="group-hover:hidden text-[10px] font-medium text-[#94a3b8] dark:text-slate-500 tabular-nums shrink-0">{p.badge}</Text>
        </If>
      </If>
    </>
  );
}

/** One row of the tree (the old AccountNode). `node` is a group object
 *  {group, accounts} or an account row; `depth` indents 8 + depth*14 px. */
function TxSideNode(p: {
  node: Any;
  depth: number;
  selectedId: string;
  onSelect: (id: string) => void;
  badge?: (a: Any) => string;
  checkedIds?: string[];
  onToggleAccount?: (id: string, node: Any) => void;
  forceOpen?: boolean;
}) {
  const node = () => p.node as Any;
  const isRoot = () => p.depth === 0;
  const conf = () => txSideConf(String(node().group || ""));
  const expanded = () => Boolean(p.forceOpen) || !txSideCollapsed().includes(String(node().group || ""));
  const toggleExpand = () => {
    const g = String(node().group || "");
    txSideCollapsed.set(expanded() ? [...txSideCollapsed(), g] : txSideCollapsed().filter((x) => x !== g));
  };
  const checked = () => p.checkedIds || [];
  const selected = () => !isRoot() && String(node().ticket) === p.selectedId;
  const badge = () => (isRoot() ? "" : p.badge ? p.badge(node()) : String(node().balance_shown || ""));
  return (
    <View>
      <View
        class={`group flex-row items-center gap-2 px-3 py-[7px] cursor-pointer transition-colors ${
          selected()
            ? "bg-[#e6f7f5] dark:bg-slate-700 border-r-2 border-[#2a9d8f]"
            : "hover:bg-[#f8fafc] dark:hover:bg-slate-800"
        }`}
        style={{ paddingLeft: 8 + p.depth * 14 }}
      >
        {/* Icon tile — doubles as expand/collapse (icon shows, chevron on row hover) */}
        {isRoot() ? (
          <View
            class="w-7 h-7 rounded-lg items-center justify-center shrink-0 cursor-pointer transition-colors"
            style={{ backgroundColor: conf().bg }}
            onPress={toggleExpand}
            accessibilityLabel={expanded() ? "Collapse" : "Expand"}
          >
            <View class="items-center justify-center group-hover:hidden">
              <Text class="text-[11px] font-bold" style={{ color: conf().text }}>{conf().label.slice(0, 1)}</Text>
            </View>
            <View class="hidden group-hover:flex items-center justify-center">
              <Avatar
                src={expanded() ? TX_SIDE_ICONS.chevronDown : TX_SIDE_ICONS.chevronRight}
                class="w-3.5 h-3.5"
                imageClass={`tint-[${conf().text}]`}
                fallbackDelay={Infinity}
                accessibilityLabel=""
              />
            </View>
          </View>
        ) : (
          <View class="w-6 h-6 rounded-md items-center justify-center shrink-0">
            <View class="w-1.5 h-1.5 rounded-full opacity-40" style={{ backgroundColor: conf().text }} />
          </View>
        )}

        {/* Name + account number — the old name span: root toggles, leaf selects */}
        <View
          class="flex-1 min-w-0"
          onPress={() => (isRoot() ? toggleExpand() : p.onSelect(String(node().ticket)))}
        >
          <Text
            class={`block truncate ${
              isRoot()
                ? "text-[13px] font-semibold text-[#1e293b] dark:text-slate-100"
                : "text-[12px] text-[#475569] dark:text-slate-400"
            }`}
          >
            {isRoot() ? conf().label : String(node().name || "")}
          </Text>
          <If when={!isRoot() && Boolean(node().number)}>
            <Text class="block text-[10px] text-[#94a3b8] dark:text-slate-500 leading-tight">{String(node().number)}</Text>
          </If>
        </View>

        <TxSideRight node={node()} badge={badge()} checked={checked()} onToggle={p.onToggleAccount} />
      </View>

      {/* Children */}
      <If when={isRoot() && expanded()}>
        <View>
          {(node().accounts as Any[]).map((a) => (
            <TxSideNode
              node={a}
              depth={p.depth + 1}
              selectedId={p.selectedId}
              onSelect={p.onSelect}
              badge={p.badge}
              checkedIds={p.checkedIds}
              onToggleAccount={p.onToggleAccount}
            />
          ))}
        </View>
      </If>
    </View>
  );
}

/** The COA sidebar (the old AccountSidebar): lookup + tree + tri-state checks. */
function TxSidebar(p: {
  accounts: Any[];
  selectedId: string;
  onSelect: (id: string) => void;
  checkedIds?: string[];
  onToggleAccount?: (id: string, node: Any) => void;
  onSelectAll?: () => void;
  onClearSelection?: () => void;
  badge?: (a: Any) => string;
}) {
  const search = signal("");
  /** Groups -> member rows, lookup-filtered by name or account number. */
  const tree = () => {
    const q = search().trim().toLowerCase();
    const out: Any[] = [];
    for (const g of TX_SIDE_GROUPS) {
      const rows = p.accounts.filter((a) => String(a.group) === g);
      if (!rows.length) continue;
      if (!q) {
        out.push({ group: g, accounts: rows });
        continue;
      }
      const matched = rows.filter(
        (a) =>
          String(a.name || "").toLowerCase().includes(q) ||
          String(a.number || "").toLowerCase().includes(q),
      );
      // the old filterTree keeps a node on its own match (all children) or on a matched child
      if (g.toLowerCase().includes(q)) out.push({ group: g, accounts: rows });
      else if (matched.length) out.push({ group: g, accounts: matched });
    }
    return out;
  };
  const searching = () => Boolean(search().trim());
  const leafCount = () => p.accounts.length;
  const checkedCount = () => (p.checkedIds || []).length;
  return (
    <View class="flex-1 flex-col bg-[#f8fafc] dark:bg-slate-900">
      {/* Lookup */}
      <View class="px-3 py-2.5 border-b border-[#e5e7eb] dark:border-slate-700">
        <View class="relative">
          <View class="absolute left-2.5 top-0 bottom-0 items-center justify-center">
            <Avatar src={TX_SIDE_ICONS.search} class="w-[13px] h-[13px]" imageClass="tint-[#94a3b8]" fallbackDelay={Infinity} accessibilityLabel="" />
          </View>
          <Input
            accessibilityLabel="Lookup accounts"
            value={search()}
            placeholder="Lookup..."
            onChange={(e: Any) => search.set(typeof e === "string" ? e : e.value)}
            class="w-full pl-8 pr-3 py-1.5 rounded-md bg-white dark:bg-slate-800 border border-[#e2e8f0] dark:border-slate-600 text-[12px] text-[#1e293b] dark:text-slate-200 placeholder:text-[#94a3b8] dark:placeholder:text-slate-500 focus:outline-none focus:border-[#2a9d8f] dark:focus:border-teal-500 focus:ring-1 focus:ring-[#2a9d8f] dark:focus:ring-teal-500 transition-all"
          />
        </View>
      </View>

      {/* Account tree (a search keeps every matched parent open, like the old expanded depth-0 nodes) */}
      <View class="flex-1 min-h-0">
        <Scroll class="flex-1">
          {tree().map((g) =>
            <TxSideNode
              node={g}
              depth={0}
              selectedId={p.selectedId}
              onSelect={p.onSelect}
              badge={p.badge}
              checkedIds={p.checkedIds}
              onToggleAccount={p.onToggleAccount}
              forceOpen={searching()}
            />,
          )}
        </Scroll>
      </View>

      {/* Select all / Clear — the old MultiCheckboxCombobox footer, drawn only
          when the caller wires the callbacks (the sidebar itself has none). */}
      <If when={Boolean(p.onSelectAll || p.onClearSelection)}>
        <View class="flex-row items-center justify-between border-t border-[#e5e7eb] dark:border-slate-700 px-3 py-2.5">
          <Text class="text-[11px] text-slate-500 dark:text-slate-400">
            {`${checkedCount()} of ${leafCount()} selected`}
          </Text>
          <View class="flex-row items-center gap-1">
            <If when={checkedCount() > 0 && Boolean(p.onClearSelection)}>
              <View
                class="rounded-md px-2 py-1 cursor-pointer transition hover:bg-emerald-50 dark:hover:bg-emerald-400/10"
                onPress={() => p.onClearSelection?.()}
              >
                <Text class="text-xs font-semibold text-emerald-700 dark:text-emerald-300">Clear</Text>
              </View>
            </If>
            <If when={checkedCount() < leafCount() && Boolean(p.onSelectAll)}>
              <View
                class="rounded-md px-2 py-1 cursor-pointer transition hover:bg-emerald-50 dark:hover:bg-emerald-400/10"
                onPress={() => p.onSelectAll?.()}
              >
                <Text class="text-xs font-semibold text-emerald-700 dark:text-emerald-300">Select all</Text>
              </View>
            </If>
          </View>
        </View>
      </If>
    </View>
  );
}
