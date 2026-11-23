// Transactions: the old app's /transactions ledger (routes/transactions.tsx):
// COA sidebar, green header with View/Edit pill, month navigator + tabs, the
// LedgerSearchBar tag-chip input, and the list — a flat grid when an account is
// picked, the AccountGroupedList (one group per account the month touches)
// when it is not. Row press opens the window's entry drawer. Edit mode keeps
// select + Void (posted) / Discard (drafts) — the only batch writes Books tools
// offer; inline edits, batch match and CSV export have no tool, so they're
// left out.

const txMonth = signal(String(PROPS.month || "")); // "YYYY-MM", "" = the tool's current month
const txAccount = signal(""); // account ticket, "" = All accounts
const txChecked = signal<string[]>([]); // checked account tickets in TxSidebar (filters grouped view)
const txFilter = signal("all"); // tool enum: all/review/draft/posted/checks
const txSearch = signal(""); // live text in the search input
const txQueryChips = signal<string[]>([]); // queries the person committed with Enter
const txChipsOpen = signal(false); // chip bar overflow expanded
const txFiltersOpen = signal(false);
const txSplitOpen = signal(false);
const txMode = signal("view"); // "view" | "edit"
const txPicked = signal<string[]>([]); // selected entry tickets in edit mode
const txSideOpen = signal(false); // accounts sheet below lg
const txConfirm = signal<"" | "void" | "discard">("");
const txBusy = signal(false);
// expand/collapse (AccountGroupedList's expandedGroups + allCollapsed)
const txAllCollapsed = signal(false);
const txExpandOver = signal<Record<string, boolean>>({});
// client-side filters (the tool's own enum stays on `txFilter`)
const txTypes = signal<string[]>([]);
const txStatuses = signal<string[]>([]);
const txSources = signal<string[]>([]);
const txMatched = signal<string>(""); // "" | "matched" | "unmatched"
const txDeptText = signal("");
const txLocText = signal("");
const txAmountMin = signal("");
const txAmountMax = signal("");
const txPartyText = signal("");
// LedgerFilters multi-selects (names, the hay is text anyway)
const txParties = signal<string[]>([]);
const txDepts = signal<string[]>([]);
const txLocs = signal<string[]>([]);
const txSecOpen = signal<Record<string, boolean>>({ Party: true, Amount: true, "Transaction Type": true });

// The server `search` arg: the text being typed, else the first query chip
// (the rest of the chips AND client-side in txFiltered).
const txServerSearch = () => txSearch().trim() || txQueryChips()[0] || "";

const txData = resource(
  () => ({ v: ver(), m: txMonth(), a: txAccount(), f: txFilter(), s: txServerSearch(), q: txQueryChips().length }),
  (k: Any) =>
    buwiz.call("books_ledger_list", {
      ...(k.m ? { month: k.m } : {}),
      ...(k.a ? { account: k.a } : {}),
      ...(k.s ? { search: k.s } : {}),
      filter: k.f,
    }),
);

// Paging: the tool answers 200 rows + next_cursor; "Load more" appends pages.
const txCursor = signal(""); // next_cursor of the last page fetched
const txMoreRows = signal<Any[]>([]);
const txMoreFor = signal(""); // the page key the extra rows belong to
const txLoadingMore = signal(false);
const txPageKey = () => [ver(), txMonth(), txAccount(), txFilter(), txServerSearch(), txQueryChips().join("\u001f")].join("|");
const txRowsAll = () => (txMoreFor() === txPageKey() ? [...txRows(), ...txMoreRows()] : txRows());
const txNextCursor = () => (txMoreFor() === txPageKey() ? txCursor() : String(txD()?.next_cursor || ""));
const txLoadMore = async () => {
  const cursor = txNextCursor();
  if (!cursor || txLoadingMore()) return;
  txLoadingMore.set(true);
  try {
    const res: Any = await buwiz.call("books_ledger_list", {
      ...(txMonth() ? { month: txMonth() } : {}),
      ...(txAccount() ? { account: txAccount() } : {}),
      filter: txFilter(),
      ...(txServerSearch() ? { search: txServerSearch() } : {}),
      cursor,
    });
    txMoreRows.set([...(txMoreFor() === txPageKey() ? txMoreRows() : []), ...(((res?.rows as Any[]) || []) as Any[])]);
    txCursor.set(String(res?.next_cursor || ""));
    txMoreFor.set(txPageKey());
  } finally {
    txLoadingMore.set(false);
  }
};
const txAccounts = resource(
  () => ({ v: ver() }),
  () => buwiz.call("books_accounts_list", {}),
);
const txPeople = resource(
  () => ({ v: ver() }),
  () => buwiz.call("books_people_list", {}),
);
const txTags = resource(
  () => ({ v: ver() }),
  () => buwiz.call("books_tags_list", {}),
);
const txPartyOpts = () => (((txPeople() as Any)?.rows || (txPeople() as Any)?.parties || []) as Any[]).map((r) => String(r.name || "")).filter(Boolean).sort();
const txTagOpts = (kind: string) =>
  (((txTags() as Any)?.rows || (txTags() as Any)?.tags || []) as Any[])
    .filter((r) => !kind || String(r.kind || r.group || "") === kind)
    .map((r) => String(r.name || ""))
    .filter(Boolean)
    .sort();

const txD = () => txData() as Any;
const txRows = () => ((txD()?.rows as Any[]) || []) as Any[];
const txGroups = () => ((txD()?.groups as Any[]) || []) as Any[];
const txVisibleGroups = () => {
  const checked = txChecked();
  const all = txGroups();
  return checked.length ? all.filter((g) => checked.includes(String(g.ticket))) : all;
};

// Grouped view: each group in the ledger payload already carries its rows
// for the month (views::ledger embeds them), so the app draws the whole
// grouped ledger from one call.
const txGroupRows = (g: Any): Any[] => ((g?.rows as Any[]) || []) as Any[];


// Kit ICONS has no search/x/funnel/grid yet (kit.tsx is agent B's); same svgIcon helper.
const TX_ICONS = {
  search: svgIcon('<circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>', 2),
  x: svgIcon('<path d="M18 6 6 18"/><path d="m6 6 12 12"/>', 2),
  funnel: svgIcon('<polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3"/>', 2),
  grid: svgIcon('<rect x="3" y="3" width="18" height="18" rx="2" ry="2"/><line x1="3" y1="9" x2="21" y2="9"/><line x1="3" y1="15" x2="21" y2="15"/><line x1="9" y1="3" x2="9" y2="21"/>', 2),
  chevronUp: svgIcon('<path d="m18 15-6-6-6 6"/>', 2),
  eye: svgIcon('<path d="M2 12s3-7 10-7 10 7 10 7-3 7-10 7-10-7-10-7Z"/><circle cx="12" cy="12" r="3"/>', 2),
  pencil: svgIcon('<path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"/>', 2),
  panel: svgIcon('<rect x="3" y="3" width="18" height="18" rx="2" ry="2"/><line x1="9" y1="3" x2="9" y2="21"/>', 1.5),
  plus: svgIcon('<line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>', 2.5),
  upload: svgIcon('<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/>', 1.5),
  doc: svgIcon('<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/>', 1.5),
  fParty: svgIcon('<path d="M17 21v-2a4 4 0 00-4-4H5a4 4 0 00-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 00-3-3.87"/><path d="M16 3.13a4 4 0 010 7.75"/>', 1.5),
  fAmount: svgIcon('<line x1="12" y1="1" x2="12" y2="23"/><path d="M17 5H9.5a3.5 3.5 0 000 7h5a3.5 3.5 0 010 7H6"/>', 1.5),
  fSource: svgIcon('<path d="M21 16V8a2 2 0 00-1-1.73l-7-4a2 2 0 00-2 0l-7 4A2 2 0 003 8v8a2 2 0 001 1.73l7 4a2 2 0 002 0l7-4A2 2 0 0021 16z"/>', 1.5),
  fMatched: svgIcon('<path d="M22 11.08V12a10 10 0 11-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/>', 1.5),
  fDept: svgIcon('<rect x="2" y="7" width="20" height="14" rx="2" ry="2"/><path d="M16 21V5a2 2 0 00-2-2h-4a2 2 0 00-2 2v16"/>', 1.5),
  fLoc: svgIcon('<path d="M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0118 0z"/><circle cx="12" cy="10" r="3"/>', 1.5),
  fType: svgIcon('<polyline points="22 12 18 12 15 21 9 3 6 12 2 12"/>', 1.5),
};
const MONTH_KEYS = MONTH_NAMES;

/** Add `n` months to a YYYY-MM key. */
function monthAdd(key: string, n: number): string {
  const y = Number(key.slice(0, 4));
  const m = Number(key.slice(5, 7)) - 1 + n;
  const yy = y + Math.floor(m / 12);
  const mm = ((m % 12) + 12) % 12;
  return `${yy}-${String(mm + 1).padStart(2, "0")}`;
}
/** Compare two two-place decimal texts without floating point: -1 | 0 | 1. */
function cmpDec(a: string, b: string): number {
  const na = a.replace(/,/g, "").trim();
  const nb = b.replace(/,/g, "").trim();
  const sa = na.startsWith("-") ? -1 : 1;
  const sb = nb.startsWith("-") ? -1 : 1;
  if (sa !== sb) return sa < sb ? -1 : 1;
  const [ai, af = "00"] = na.replace("-", "").split(".");
  const [bi, bf = "00"] = nb.replace("-", "").split(".");
  const cmp =
    ai.length !== bi.length
      ? ai.length - bi.length
      : ai !== bi
        ? ai < bi
          ? -1
          : 1
        : af.padEnd(2, "0") === bf.padEnd(2, "0")
          ? 0
          : af.padEnd(2, "0") < bf.padEnd(2, "0")
            ? -1
            : 1;
  return sa < 0 ? -cmp : cmp;
}

/** The fields the tool's `matches()` haystack covers, for client chip terms. */
function txHay(r: Any): string {
  return [r.payee, r.title, r.sub, r.category, r.no, r.date_short, r.date, r.amount_shown, r.amount]
    .filter(Boolean)
    .join(" ")
    .toLowerCase();
}

/** Rows after search + the client-side filters (type, status, amount, party) + extra query chips. */
function txFiltered(rowsIn?: Any[]): Any[] {
  const types = txTypes();
  const statuses = txStatuses();
  const sources = txSources();
  const matched = txMatched();
  const dept = txDeptText().trim().toLowerCase();
  const loc = txLocText().trim().toLowerCase();
  // Compare unsigned amounts on digits only ("+40.00" and "1,200" both read).
  const min = txAmountMin().replace(/[^0-9.]/g, "");
  const max = txAmountMax().replace(/[^0-9.]/g, "");
  const party = txPartyText().trim().toLowerCase();
  // Query chips still needing a client-side match: all of them while the input
  // has text (the server filters on the text instead), else chips past the
  // first (the server filtered on chip[0]).
  const terms = txSearch().trim() ? txQueryChips() : txQueryChips().slice(1);
  return (rowsIn || txRowsAll()).filter((r) => {
    if (types.length && !types.includes(String(r.form || ""))) return false;
    if (statuses.length) {
      const rev = String(r.pill || "") === "rev";
      const st = String(r.status || "");
      if (!statuses.some((s) => (s === "review" ? rev : st === s))) return false;
    }
    if (sources.length) {
      const src = String(r.source || (r.evidence ? "document" : "manual")).toLowerCase();
      if (!sources.includes(src)) return false;
    }
    if (matched) {
      const isM = Boolean(r.matched || r.reconciled);
      if (matched === "matched" && !isM) return false;
      if (matched === "unmatched" && isM) return false;
    }
    if (dept && !String(r.department || "").toLowerCase().includes(dept)) return false;
    if (loc && !String(r.location || "").toLowerCase().includes(loc)) return false;
    if (txDepts().length && !txDepts().some((x) => String(r.department || "").toLowerCase() === x.toLowerCase())) return false;
    if (txLocs().length && !txLocs().some((x) => String(r.location || "").toLowerCase() === x.toLowerCase())) return false;
    const amt = String(r.amount_shown || r.amount || "0").replace(/[^0-9.]/g, "");
    if (min && cmpDec(amt, min) < 0) return false;
    if (max && cmpDec(amt, max) > 0) return false;
    const hay = (terms.length || party || txParties().length) && txHay(r);
    if (party && !hay.includes(party)) return false;
    if (txParties().length && !txParties().some((x) => hay && hay.includes(x.toLowerCase()))) return false;
    for (const t of terms) if (!hay.includes(t.toLowerCase())) return false;
    return true;
  });
}

const TX_TYPE_OPTS: [string, string][] = [
  ["money_in", "Pay In"],
  ["money_out", "Pay Out"],
  ["journal", "Journal"],
  ["transfer", "Transfer"],
];
const TX_STATUS_OPTS: [string, string][] = [
  ["draft", "Draft"],
  ["posted", "Posted"],
  ["voided", "Voided"],
  ["review", "Needs review"],
];
const TX_SOURCE_OPTS: [string, string][] = [
  ["manual", "Manual"],
  ["bank", "Bank"],
  ["document", "Document"],
];
const TX_MATCHED_OPTS: [string, string][] = [
  ["matched", "Matched"],
  ["unmatched", "Unmatched"],
];
const TX_FORMS: Record<string, string> = {
  money_in: "Pay In",
  money_out: "Pay Out",
  transfer: "Transfer",
  journal: "Journal",
};

function txToggleIn(sig: Any, v: string) {
  const list = sig() as string[];
  sig.set(list.includes(v) ? list.filter((x: string) => x !== v) : [...list, v]);
}
function txHasFilters(): boolean {
  return Boolean(
    txTypes().length ||
      txStatuses().length ||
      txSources().length ||
      txMatched() ||
      txDeptText().trim() ||
      txLocText().trim() ||
      txAmountMin().trim() ||
      txAmountMax().trim() ||
      txPartyText().trim() ||
      txParties().length ||
      txDepts().length ||
      txLocs().length ||
      txQueryChips().length ||
      txSearch().trim(),
  );
}
function txClearFilters() {
  txTypes.set([]);
  txStatuses.set([]);
  txSources.set([]);
  txMatched.set("");
  txDeptText.set("");
  txLocText.set("");
  txAmountMin.set("");
  txAmountMax.set("");
  txPartyText.set("");
  txParties.set([]);
  txDepts.set([]);
  txLocs.set([]);
  txQueryChips.set([]);
  txSearch.set("");
}
/** Enter in the search input commits the query as a chip (LedgerSearchBar). */
function txCommitQuery() {
  const q = txSearch().trim();
  if (q) txQueryChips.set([...txQueryChips(), q]);
  txSearch.set("");
}
function txDropQuery(q: string) {
  txQueryChips.set(txQueryChips().filter((x) => x !== q));
}

/** Group icon-box colours per account type (AccountGroupedList's TYPE_COLORS). */
const TX_TYPE_COLORS: Record<string, [string, string]> = {
  asset: ["#dcfce7", "#16a34a"],
  liability: ["#fef3c7", "#d97706"],
  equity: ["#dbeafe", "#2563eb"],
  revenue: ["#d1fae5", "#059669"],
  cost_of_revenue: ["#fce7f3", "#db2777"],
  expense: ["#fee2e2", "#dc2626"],
  other_income: ["#e0e7ff", "#4f46e5"],
  other_expense: ["#fef9c3", "#ca8a04"],
};

const STATUS_BAR: Record<string, string> = {
  draft: "#fbbf24",
  posted: "#22c55e",
  voided: "#ef4444",
};

/** Party initials + stable color (LedgerRow's getPartyInitials / stringToColor). */
const TX_PARTY_COLORS = [
  "#ef4444", "#f97316", "#f59e0b", "#84cc16", "#22c55e", "#14b8a6",
  "#06b6d4", "#3b82f6", "#6366f1", "#a855f7", "#ec4899", "#f43f5e",
];
function txPartyAvatar(name: string): [string, string] {
  const words = String(name || "?").trim().split(/\s+/);
  const initials =
    words.length >= 2
      ? (words[0][0] + words[1][0]).toUpperCase()
      : String(name || "?").slice(0, 2).toUpperCase();
  let hash = 0;
  for (let i = 0; i < String(name).length; i++) hash = name.charCodeAt(i) + ((hash << 5) - hash);
  return [initials, TX_PARTY_COLORS[Math.abs(hash) % TX_PARTY_COLORS.length]];
}

/** Date cell: "Oct 29" + "2026" under it (LedgerRow's formatDate). */
function txDayParts(iso: string): [string, string] {
  const d = String(iso || "");
  const day = String(Number(d.slice(8, 10) || 0));
  const mon = MONTH_KEYS[Number(d.slice(5, 7)) - 1]?.slice(0, 3) || "";
  return [`${mon} ${day}`, d.slice(0, 4)];
}

/** The checkbox shared by grid rows, cards and group headers. */
function TxCheck(p: { checked: boolean }) {
  return (
    <View
      class={`w-4 h-4 rounded border items-center justify-center transition-all ${
        p.checked
          ? "bg-[#2dd4bf] border-[#2dd4bf] dark:bg-teal-500 dark:border-teal-500"
          : "bg-white dark:bg-slate-800 border-gray-300 dark:border-slate-600 group-hover:border-gray-400 dark:group-hover:border-slate-500"
      }`}
    >
      <If when={p.checked}>
        <Avatar src={ICONS.check} class="w-2.5 h-2.5" imageClass="tint-white" fallbackDelay={Infinity} accessibilityLabel="" />
      </If>
    </View>
  );
}

/** One ledger row (LedgerRow.tsx grid mode): bar|date|party|src|category|dept|loc|debit|credit|balance|› */
function TxRow(p: {
  r: Any;
  balance?: boolean;
  edit?: boolean;
  checked?: boolean;
  inGroup?: boolean;
  onCheck: () => void;
  onOpen: () => void;
}) {
  const r = () => p.r as Any;
  const [md, yr] = txDayParts(String(r().date || ""));
  const title = String(r().payee || r().title || "—");
  const sub = String(r().payee ? r().title || r().sub || "" : r().sub || "");
  const bar = String(r().pill || "") === "rev" ? "#fbbf24" : STATUS_BAR[String(r().status || "")] || "#cbd5e1";
  const unposted = String(r().status || "") !== "posted" || String(r().pill || "") === "rev";
  const [initials, color] = txPartyAvatar(String(r().payee || r().title || "?"));
  const cat = String(r().category || "");
  const catColor = String(r().category_color || "#94a3b8");
  const debit = String(r().debit_shown || "");
  const credit = String(r().credit_shown || "");
  return (
    <View
      class={`group flex-row items-stretch cursor-pointer border-b transition-colors ${
        p.checked
          ? "bg-[#eff6ff] dark:bg-blue-950/60 border-[#dbeafe] dark:border-blue-900/40"
          : `border-[#f1f5f9] dark:border-slate-800 ${p.inGroup ? "hover:bg-[#f8fafc] dark:hover:bg-slate-800/70" : "hover:bg-[#fafbfc] dark:hover:bg-slate-800"}`
      }`}
      onPress={p.edit ? p.onCheck : p.onOpen}
    >
      <View class="w-1 shrink-0 self-stretch">
        <If when={unposted}>
          <View class="w-1 h-full" style={{ backgroundColor: bar }} />
        </If>
      </View>
      <If when={p.edit}>
        <View class="pl-3 pr-1 items-center justify-center shrink-0">
          <TxCheck checked={Boolean(p.checked)} />
        </View>
      </If>
      <View class="w-[72px] py-2.5 px-2 items-center shrink-0">
        <Text class="text-[13px] font-semibold text-[#1e293b] dark:text-slate-100 leading-tight">{md}</Text>
        <Text class="text-[10px] text-[#94a3b8] dark:text-slate-500">{yr}</Text>
      </View>
      <View class="flex-1 min-w-0 flex-row items-center gap-2.5 py-2.5 px-2">
        <View class="w-7 h-7 rounded-full items-center justify-center shrink-0" style={{ backgroundColor: color }}>
          <Text class="text-[10px] font-bold text-white">{initials}</Text>
        </View>
        <View class="flex-1 min-w-0">
          <Text class="text-[13px] font-medium text-[#1e293b] dark:text-slate-100 truncate">{title}</Text>
          <If when={Boolean(sub)}>
            <Text class="text-[11px] text-[#94a3b8] dark:text-slate-500 truncate">{sub}</Text>
          </If>
        </View>
      </View>
      <View class="w-10 py-2.5 items-center justify-center shrink-0">
        <View class="w-5 h-5 rounded-full items-center justify-center" style={{ backgroundColor: "#64748b" }}>
          <Text class="text-[8px] font-bold text-white">M</Text>
        </View>
      </View>
      <View class="w-[160px] py-2.5 px-2 shrink-0 min-w-0 hidden md:block">
        <If when={Boolean(cat)}>
          <View class="flex-row items-center gap-1.5">
            <View class="w-1.5 h-1.5 rounded-full shrink-0" style={{ backgroundColor: catColor }} />
            <Text class="text-[12px] text-[#475569] dark:text-slate-300 truncate">{cat}</Text>
          </View>
        </If>
        <If when={!cat}>
          <Text class="text-[11px] text-[#cbd5e1] dark:text-slate-600 italic">—</Text>
        </If>
      </View>
      <View class="w-[130px] py-2.5 px-2 shrink-0 hidden xl:block">
        <Text class="text-[11px] text-[#cbd5e1] dark:text-slate-600 italic">None</Text>
      </View>
      <View class="w-[130px] py-2.5 px-2 shrink-0 hidden xl:block">
        <Text class="text-[11px] text-[#cbd5e1] dark:text-slate-600 italic">None</Text>
      </View>
      <View class="w-[100px] py-2.5 px-2 items-end justify-center shrink-0">
        <If when={Boolean(debit)}>
          <Text class="text-[13px] font-semibold tabular-nums text-[#1e293b] dark:text-slate-100">{debit}</Text>
        </If>
      </View>
      <View class="w-[100px] py-2.5 px-2 items-end justify-center shrink-0">
        <If when={Boolean(credit)}>
          <Text class="text-[13px] font-semibold tabular-nums text-[#16a34a] dark:text-emerald-400">{credit}</Text>
        </If>
      </View>
      <If when={p.balance}>
        <View class="w-[110px] py-2.5 px-2 items-end justify-center shrink-0">
          <Text class="tabular-nums text-[12px] text-[#64748b] dark:text-slate-400">{String(r().balance_shown || "")}</Text>
        </View>
      </If>
      <View class="w-9 items-center justify-center shrink-0">
        <Avatar
          src={ICONS.chevronRight}
          class="w-4 h-4 opacity-0 group-hover:opacity-100 transition-opacity"
          imageClass="tint-[#cbd5e1] dark:tint-slate-600"
          fallbackDelay={Infinity}
          accessibilityLabel=""
        />
      </View>
    </View>
  );
}

/** Card-mode row below md (LedgerRow's card: avatar, title, sub, chips, Dr/Cr + Bal). */
function TxCard(p: { r: Any; edit?: boolean; checked?: boolean; onCheck?: () => void; onOpen: () => void }) {
  const r = () => p.r as Any;
  const [initials, color] = txPartyAvatar(String(r().payee || r().title || "?"));
  const [md, yr] = txDayParts(String(r().date || ""));
  const unposted = String(r().status || "") !== "posted" || String(r().pill || "") === "rev";
  const debit = String(r().debit_shown || "");
  const credit = String(r().credit_shown || "");
  return (
    <View
      class={`border-b px-4 py-3 transition-colors cursor-pointer ${
        p.checked
          ? "bg-[#eff6ff] dark:bg-blue-950/60 border-[#dbeafe] dark:border-blue-900/40"
          : "border-[#f1f5f9] dark:border-slate-800 hover:bg-[#fafbfc] dark:hover:bg-slate-800"
      }`}
      onPress={p.edit ? p.onCheck : p.onOpen}
    >
      <View class="flex-row gap-3">
        <If when={p.edit}>
          <View class="w-11 h-11 -ml-2 -my-1 items-center justify-center shrink-0">
            <TxCheck checked={Boolean(p.checked)} />
          </View>
        </If>
        <View class="mt-0.5 h-8 w-8 shrink-0 items-center justify-center rounded-full" style={{ backgroundColor: color }}>
          <Text class="text-[11px] font-bold text-white">{initials}</Text>
        </View>
        <View class="flex-1 min-w-0">
          <Text class="truncate text-sm font-semibold text-[#1e293b] dark:text-slate-100">{String(r().payee || r().title || "—")}</Text>
          <Text class="mt-0.5 truncate text-xs text-[#94a3b8] dark:text-slate-500">
            {`${md}, ${yr}${r().category ? ` · ${String(r().category)}` : ""}`}
          </Text>
          <View class="mt-1 flex-row flex-wrap items-center gap-1.5">
            <Text class="text-[10px] font-medium uppercase tracking-wider text-[#94a3b8] dark:text-slate-500">
              {TX_FORMS[String(r().form)] || String(r().form || "")}
            </Text>
            <If when={unposted}>
              <Pill tone={String(r().pill || "ok")} label={String(r().status_label || r().status || "")} />
            </If>
          </View>
        </View>
        <View class="shrink-0 items-end gap-0.5">
          <If when={Boolean(debit || credit)}>
            <Text class={`tabular-nums text-[15px] font-semibold ${debit ? "text-[#1e293b] dark:text-slate-100" : "text-[#16a34a] dark:text-emerald-400"}`}>
              {debit ? `Dr ${debit}` : `Cr ${credit}`}
            </Text>
          </If>
          <If when={!debit && !credit}>
            <Text class={`tabular-nums text-[15px] font-semibold ${String(r().tone) === "neg" ? "text-[#1e293b] dark:text-slate-100" : "text-[#16a34a] dark:text-emerald-400"}`}>
              {String(r().amount_shown || r().amount || "")}
            </Text>
          </If>
          <If when={Boolean(r().balance_shown)}>
            <Text class="text-xs tabular-nums text-[#94a3b8] dark:text-slate-500">{`Bal ${String(r().balance_shown)}`}</Text>
          </If>
        </View>
      </View>
    </View>
  );
}

/** One account group in the All-accounts view (AccountGroupedList's header). */
function TxGroup(p: {
  g: Any;
  expanded: boolean;
  edit: boolean;
  onToggle: () => void;
  onToggleAll: (on: boolean) => void;
  onOpen: (r: Any) => void;
  onCheck: (r: Any) => void;
}) {
  const g = () => p.g as Any;
  const [bg, fg] = TX_TYPE_COLORS[String(g().type || "")] || ["#f1f5f9", "#64748b"];
  const rows = () => txFiltered(txGroupRows(g()));
  const some = () => rows().some((r) => txPicked().includes(String(r.ticket)));
  const neg = () => String(g().balance_shown || "").startsWith("\u2212") || String(g().balance_shown || "").startsWith("-");
  return (
    <View>
      <View
        class={`group flex-row items-center gap-2 px-3 py-2 border-b border-[#f1f5f9] dark:border-slate-800 cursor-pointer transition-colors bg-[#fafbfc] dark:bg-slate-800/50 hover:bg-[#f1f5f9] dark:hover:bg-slate-800`}
        onPress={p.onToggle}
      >
        <If when={p.edit}>
          <View class="w-11 h-11 -ml-2 -my-1 md:w-auto md:h-auto md:ml-0 md:my-0 items-center justify-center shrink-0" onPress={() => p.onToggleAll(!some())}>
            <TxCheck checked={some()} />
          </View>
        </If>
        <View class="w-7 h-7 rounded-lg items-center justify-center shrink-0" style={{ backgroundColor: bg }}>
          {/* expanded: the type icon, a chevron-down on hover (always below md) */}
          <If when={p.expanded}>
            <View class="hidden md:flex items-center justify-center group-hover:hidden">
              <Avatar src={ICONS.journal} class="w-4 h-4" imageClass={`tint-[${fg}]`} fallbackDelay={Infinity} accessibilityLabel="" />
            </View>
            <View class="md:hidden flex items-center justify-center md:group-hover:flex">
              <Avatar src={ICONS.chevronDown} class="w-4 h-4" imageClass={`tint-[${fg}]`} fallbackDelay={Infinity} accessibilityLabel="" />
            </View>
          </If>
          <If when={!p.expanded}>
            <Avatar src={ICONS.chevronRight} class="w-4 h-4" imageClass={`tint-[${fg}]`} fallbackDelay={Infinity} accessibilityLabel="" />
          </If>
        </View>
        <View class="flex-1 min-w-0">
          <View class="flex-row items-baseline gap-2 min-w-0">
            <Text class="text-[13px] font-semibold text-[#1e293b] dark:text-slate-100 truncate">{String(g().name || "")}</Text>
            <Text class="text-[11px] tabular-nums text-[#94a3b8] dark:text-slate-500 shrink-0">{String(g().number || "")}</Text>
          </View>
          <Text class="text-[11px] text-[#94a3b8] dark:text-slate-500">{`${Number(g().count || 0)} Transactions`}</Text>
        </View>
        <View class="w-[100px] items-end shrink-0 hidden md:flex">
          <Text class="tabular-nums text-[12px] text-[#1e293b] dark:text-slate-100">{String(g().debit_shown || "")}</Text>
        </View>
        <View class="w-[100px] items-end shrink-0 hidden md:flex">
          <Text class="tabular-nums text-[12px] text-[#16a34a] dark:text-emerald-400">{String(g().credit_shown || "")}</Text>
        </View>
        <View class="w-[110px] items-end shrink-0">
          <Text class={`tabular-nums text-[12px] font-semibold ${neg() ? "text-[#dc2626] dark:text-red-400" : "text-[#1e293b] dark:text-slate-100"}`}>
            {String(g().balance_shown || "")}
          </Text>
        </View>
      </View>
      <If when={p.expanded}>
        <View>
          <TxCols edit={p.edit} balance />
          <View class="hidden md:block">
            {rows().map((r) => (
              <TxRow r={r} balance inGroup edit={p.edit} checked={txPicked().includes(String(r.ticket))} onCheck={() => p.onCheck(r)} onOpen={() => p.onOpen(r)} />
            ))}
          </View>
          <View class="md:hidden">
            {rows().map((r) => (
              <TxCard r={r} edit={p.edit} checked={txPicked().includes(String(r.ticket))} onCheck={() => p.onCheck(r)} onOpen={() => p.onOpen(r)} />
            ))}
          </View>
        </View>
      </If>
    </View>
  );
}

/** Column labels over the grid (flat list header or a group's own header). */
function TxCols(p: { edit?: boolean; balance?: boolean }) {
  const colHead = (label: string, cls: string) => (
    <Text class={`${cls} px-2 text-[10px] font-semibold text-slate-500 dark:text-slate-400 uppercase tracking-wider`}>{label}</Text>
  );
  return (
    <View class="hidden md:block">
      <View class="flex-row items-center py-1.5 border-b border-[#f1f5f9] dark:border-slate-800 bg-white dark:bg-[#0f172a]">
        <View class="w-1" />
        <If when={p.edit}>
          <View class="w-8" />
        </If>
        {colHead("Date", "w-[72px] text-center")}
        {colHead("Party", "flex-1 min-w-0")}
        {colHead("Src", "w-10 text-center")}
        {colHead("Category", "w-[160px]")}
        {colHead("Department", "w-[130px] hidden xl:block")}
        {colHead("Location", "w-[130px] hidden xl:block")}
        {colHead("Debit", "w-[100px] text-right")}
        {colHead("Credit", "w-[100px] text-right")}
        <If when={p.balance}>
          {colHead("Balance", "w-[110px] text-right")}
        </If>
        <View class="w-9" />
      </View>
    </View>
  );
}

/* ── TxSide: wires TxSidebar (transactions_side.tsx) to TransactionsPage ─── */
function TxSide() {
  const accts = () => (((txAccounts() as Any)?.rows as Any[]) || []).filter((a) => !a.off);
  return (
    <TxSidebar
      accounts={accts()}
      selectedId={txAccount()}
      onSelect={(ticket) => {
        txAccount.set(ticket);
        txChecked.set([]);
        txSideOpen.set(false);
      }}
      checkedIds={txChecked()}
      onToggleAccount={(_id, node) => {
        const next = txSideToggled(txSideNodeIds(node), txChecked());
        txChecked.set(next);
        if (next.length) txAccount.set("");
      }}
      onSelectAll={() => {
        txChecked.set(txSideAllIds(accts()));
        txAccount.set("");
      }}
      onClearSelection={() => txChecked.set([])}
    />
  );
}
/* ── end TxSide ─────────────────────────────────────────────────────────── */

/** LedgerFilters' collapsible section: chevron + icon + title + count badge. */
function TxFilterSection(p: { title: string; icon: string; count: number; children: Any }) {
  const open = () => Boolean(txSecOpen()[p.title]);
  return (
    <View class="border-b border-[#f1f5f9] dark:border-slate-700">
      <View
        class="w-full flex-row items-center gap-2 px-4 py-2.5 cursor-pointer transition-colors hover:bg-[#fafbfc] dark:hover:bg-slate-800"
        onPress={() => txSecOpen.set({ ...txSecOpen(), [p.title]: !open() })}
      >
        <Avatar src={open() ? ICONS.chevronDown : ICONS.chevronRight} class="w-2.5 h-2.5 shrink-0" imageClass="tint-[#94a3b8]" fallbackDelay={Infinity} accessibilityLabel="" />
        <Avatar src={p.icon} class="w-4 h-4 shrink-0" imageClass="tint-[#64748b] dark:tint-slate-400" fallbackDelay={Infinity} accessibilityLabel="" />
        <Text class="flex-1 text-[13px] font-semibold text-[#1e293b] dark:text-slate-100">{p.title}</Text>
        <If when={p.count > 0}>
          <View class="w-5 h-5 rounded-full items-center justify-center shrink-0" style={{ backgroundColor: "#2a9d8f" }}>
            <Text class="text-[10px] font-bold text-white">{p.count}</Text>
          </View>
        </If>
      </View>
      <If when={open()}>
        <View class="px-4 pb-3">{p.children}</View>
      </If>
    </View>
  );
}

/** LedgerFilters' checkbox row: optional icon/color dot + label + 18px teal check. */
function TxCheckRow(p: { label: string; on: boolean; icon?: string; color?: string; onPress: () => void }) {
  return (
    <View class="flex-row items-center gap-2.5 py-1.5 px-1 -mx-1 rounded-md cursor-pointer group" onPress={p.onPress}>
      <If when={Boolean(p.icon)}>
        <Avatar src={p.icon} class="w-3.5 h-3.5 shrink-0" imageClass="tint-[#64748b] dark:tint-slate-400" fallbackDelay={Infinity} accessibilityLabel="" />
      </If>
      <If when={Boolean(p.color) && !p.icon}>
        <View class="w-2 h-2 rounded-full shrink-0" style={{ backgroundColor: p.color }} />
      </If>
      <Text class="flex-1 text-[13px] text-[#374151] dark:text-slate-300 group-hover:text-[#1e293b] dark:group-hover:text-slate-100 transition-colors">{p.label}</Text>
      <TxSideCheck state={p.on ? "all" : "none"} onPress={p.onPress} />
    </View>
  );
}

/** The small search input each checkbox-list section carries (LedgerFilters). */
function TxListSearch(p: { value: string; onChange: (v: string) => void; placeholder: string }) {
  return (
    <View class="relative mb-2">
      <View class="absolute left-2 top-0 bottom-0 items-center justify-center">
        <Avatar src={TX_ICONS.search} class="w-3 h-3" imageClass="tint-[#94a3b8] dark:tint-slate-500" fallbackDelay={Infinity} accessibilityLabel="" />
      </View>
      <Input
        value={p.value}
        placeholder={p.placeholder}
        onChange={(e: Any) => p.onChange(typeof e === "string" ? e : e.value)}
        class="w-full pl-7 pr-2 py-1.5 rounded-lg border border-[#e2e8f0] dark:border-slate-700 text-[12px] text-[#1e293b] dark:text-slate-100 placeholder-[#cbd5e1] dark:placeholder-slate-600 bg-white dark:bg-slate-800 focus:outline-none"
      />
    </View>
  );
}

/** Searchable checkbox list (SearchableCheckboxList): search + filtered rows. */
function TxCheckList(p: {
  options: string[];
  picked: string[];
  search: () => string;
  onSearch: (v: string) => void;
  onToggle: (v: string) => void;
  icon?: (v: string) => string | undefined;
  color?: (v: string) => string | undefined;
  ph: string;
  empty: string;
}) {
  const q = () => p.search().trim().toLowerCase();
  const shown = () => (q() ? p.options.filter((o) => o.toLowerCase().includes(q())) : p.options);
  return (
    <View>
      <TxListSearch value={p.search()} onChange={p.onSearch} placeholder={p.ph} />
      <View class="max-h-[200px] overflow-hidden">
        <If when={shown().length === 0}>
          <Text class="text-[11px] text-[#94a3b8] dark:text-slate-500 italic py-2">{p.empty}</Text>
        </If>
        {shown()
          .slice(0, 40)
          .map((o) => (
            <TxCheckRow
              label={o}
              on={p.picked.includes(o)}
              icon={p.icon ? p.icon(o) : undefined}
              color={p.color ? p.color(o) : undefined}
              onPress={() => p.onToggle(o)}
            />
          ))}
      </View>
    </View>
  );
}

/** A filter/query chip in the search bar (LedgerSearchBar's tag chips). */
function TxChip(p: { label: string; clear: () => void }) {
  return (
    <View class="flex-row items-center gap-1 px-2 py-0.5 rounded-full bg-[#e6f7f5] dark:bg-teal-950/60 border border-[#b2e0db] dark:border-teal-800 shrink-0">
      <Text class="text-[11px] font-medium text-[#0f766e] dark:text-teal-400">{p.label}</Text>
      <View class="w-3.5 h-3.5 items-center justify-center rounded-full cursor-pointer hover:bg-[#ccece8] dark:hover:bg-teal-900/40" onPress={p.clear}>
        <Avatar src={TX_ICONS.x} class="w-2 h-2" imageClass="tint-[#7cc8bf] dark:tint-teal-500" fallbackDelay={Infinity} accessibilityLabel="" />
      </View>
    </View>
  );
}

function TransactionsPage(p: { go: (id: string) => void; sheet: (mode: string) => void }) {
  const d = () => txD();
  const month = () => (d()?.month || {}) as Any;
  const rows = () => txFiltered();
  const tabKey = () => String(month().key || "");
  // Old app tabs: a window of month pills around the selected month, newest at today.
  const tabs = () => {
    const key = tabKey() || "0000-00";
    const latest = d()?.month?.next ? monthAdd(String(month().key), 0) : key; // response month is the cap
    const cur = String(month().key || key);
    const out: string[] = [];
    for (let i = -3; i <= 3; i++) {
      const k = monthAdd(cur, i);
      if (latest && cmpDec(k.replace("-", "") + ".00", latest.replace("-", "") + ".00") <= 0) out.push(k);
    }
    return out;
  };
  const goMonth = (k: string) => txMonth.set(k);
  const selRows = () => {
    // group rows can repeat an entry across accounts: act on each once.
    const seen: Record<string, Any> = {};
    const pool: Any[] = !txAccount()
      ? txVisibleGroups().flatMap((g) => txFiltered(txGroupRows(g)))
      : rows();
    for (const r of pool) if (txPicked().includes(String(r.ticket))) seen[String(r.ticket)] = r;
    return Object.values(seen);
  };
  // Voidable = posted and not itself a reversal (the entry drawer's rule);
  // drafts go through one atomic books_app_batch "discard".
  const voidables = () => selRows().filter((r) => r.status === "posted" && r.form !== "reversal" && !r.reversal);
  const drafts = () => selRows().filter((r) => r.status === "draft");
  const actBatch = async (mode: "void" | "discard") => {
    txBusy.set(true);
    try {
      if (mode === "discard") {
        const tickets = drafts().map((r) => String(r.ticket));
        if (tickets.length) await act("books_app_batch", { action: "discard", entries: tickets });
        toast("Discarded.");
      } else {
        const list = voidables();
        let done = 0;
        for (const r of list) {
          try {
            await act("books_app_void_entry", { entry: String(r.ticket) });
            done++;
          } catch (e: Any) {
            toast(done ? `Voided ${done} of ${list.length}; the rest failed: ${errText(e)}` : errText(e));
            break;
          }
        }
        if (done === list.length) toast("Voided.");
      }
      const acted = new Set(
        mode === "discard"
          ? drafts().map((r) => String(r.ticket))
          : voidables().map((r) => String(r.ticket)),
      );
      txPicked.set(txPicked().filter((t) => !acted.has(t)));
      txConfirm.set("");
      bump();
    } catch (e: Any) {
      toast(errText(e));
    } finally {
      txBusy.set(false);
    }
  };

  const chips = () => {
    const out: { label: string; clear: () => void }[] = [];
    for (const t of txTypes()) out.push({ label: TX_FORMS[t] || t, clear: () => txToggleIn(txTypes, t) });
    for (const s of txStatuses()) out.push({ label: (TX_STATUS_OPTS.find(([k]) => k === s) || ["", s])[1], clear: () => txToggleIn(txStatuses, s) });
    for (const sr of txSources()) out.push({ label: (TX_SOURCE_OPTS.find(([k]) => k === sr) || ["", sr])[1], clear: () => txToggleIn(txSources, sr) });
    if (txMatched()) out.push({ label: txMatched() === "matched" ? "Matched" : "Unmatched", clear: () => txMatched.set("") });
    if (txDeptText().trim()) out.push({ label: `Dept: ${txDeptText().trim()}`, clear: () => txDeptText.set("") });
    if (txLocText().trim()) out.push({ label: `Location: ${txLocText().trim()}`, clear: () => txLocText.set("") });
    if (txAmountMin().trim() || txAmountMax().trim())
      out.push({
        label: `${txAmountMin().trim() ? `\u2265 ${txAmountMin().trim()}` : ""}${txAmountMin().trim() && txAmountMax().trim() ? " – " : ""}${txAmountMax().trim() ? (txAmountMin().trim() ? "" : "\u2264 ") + txAmountMax().trim() : ""}` || `${txAmountMin().trim()} – ${txAmountMax().trim()}`,
        clear: () => { txAmountMin.set(""); txAmountMax.set(""); },
      });
    if (txPartyText().trim()) out.push({ label: `Party: ${txPartyText().trim()}`, clear: () => txPartyText.set("") });
    for (const n of txParties()) out.push({ label: `Party: ${n}`, clear: () => txParties.set(txParties().filter((x) => x !== n)) });
    for (const n of txDepts()) out.push({ label: `Dept: ${n}`, clear: () => txDepts.set(txDepts().filter((x) => x !== n)) });
    for (const n of txLocs()) out.push({ label: `Location: ${n}`, clear: () => txLocs.set(txLocs().filter((x) => x !== n)) });
    for (const q of txQueryChips()) out.push({ label: q, clear: () => txDropQuery(q) });
    return out;
  };
  // Old app measures the wrap; approximate: show the expand chevron when the
  // chips plausibly reach a second row.
  const chipsWrap = () => {
    const cs = chips();
    return cs.length > 4 || cs.reduce((n, c) => n + c.label.length, 0) > 80;
  };
  const filterOn = () => txFiltersOpen() || txHasFilters();

  const togglePill = (
    <View class="flex-row items-center gap-1.5 shrink-0">
      {/* ViewToggle: gray segmented control, active = white card + shadow */}
      <View class="flex-row items-center bg-[#f1f5f9] dark:bg-slate-800 rounded-lg p-0.5">
        <View
          accessibilityLabel="Expand all"
          class={`w-7 h-7 items-center justify-center rounded-md cursor-pointer transition-all ${
            !txAllCollapsed() ? "bg-white dark:bg-slate-700 shadow-sm" : ""
          }`}
          onPress={() => { txAllCollapsed.set(false); txExpandOver.set({}); }}
        >
          <Avatar src={ICONS.list} class="w-3.5 h-3.5" imageClass={!txAllCollapsed() ? "tint-[#1e293b] dark:tint-slate-100" : "tint-[#94a3b8] dark:tint-slate-500"} fallbackDelay={Infinity} accessibilityLabel="" />
        </View>
        <View
          accessibilityLabel="Collapse all"
          class={`w-7 h-7 items-center justify-center rounded-md cursor-pointer transition-all ${
            txAllCollapsed() ? "bg-white dark:bg-slate-700 shadow-sm" : ""
          }`}
          onPress={() => { txAllCollapsed.set(true); txExpandOver.set({}); }}
        >
          <Avatar src={TX_ICONS.grid} class="w-3.5 h-3.5" imageClass={txAllCollapsed() ? "tint-[#1e293b] dark:tint-slate-100" : "tint-[#94a3b8] dark:tint-slate-500"} fallbackDelay={Infinity} accessibilityLabel="" />
        </View>
      </View>
      <View
        accessibilityLabel="Filters"
        class={`w-8 h-8 items-center justify-center rounded-lg cursor-pointer transition-colors ${
          filterOn() ? "bg-[#ccece8] dark:bg-teal-900/40" : "hover:bg-[#f1f5f9] dark:hover:bg-slate-800"
        }`}
        onPress={() => txFiltersOpen.set(!txFiltersOpen())}
      >
        <Avatar src={TX_ICONS.funnel} class="w-3.5 h-3.5" imageClass={filterOn() ? "tint-[#0f766e] dark:tint-teal-400" : "tint-[#94a3b8] dark:tint-slate-500"} fallbackDelay={Infinity} accessibilityLabel="" />
      </View>
    </View>
  );

  return (
    <View class="flex-1 min-h-0 flex-row bg-white dark:bg-slate-900">
      {/* COA sidebar: fixed 240px column at lg+, an overlay sheet below */}
      <View class="hidden lg:flex w-[240px] shrink-0 flex-col border-r border-[#e5e7eb] dark:border-slate-700 overflow-hidden">
        <TxSide />
      </View>
      <View class="flex-1 min-w-0 flex flex-col">
        <View class="flex-1 min-h-0 flex flex-col md:mx-3 md:my-3 md:rounded-lg md:border border-[#e5e7eb] dark:border-slate-700 md:shadow-sm overflow-hidden bg-white dark:bg-slate-900">
          {/* Header — LedgerHeader */}
          <View class="flex-row items-center px-5 py-3 bg-gradient-to-r from-[#1a6b3c] to-[#27ae60] dark:from-[#145a30] dark:to-[#1e8c4c] rounded-t-lg">
            <View class="flex-row items-center gap-3 shrink-0">
              <View
                accessibilityLabel="Accounts"
                class="w-8 h-8 rounded-lg items-center justify-center transition-colors hover:bg-white/10 cursor-pointer"
                onPress={() => txSideOpen.set(true)}
              >
                <Avatar src={TX_ICONS.panel} class="w-4 h-4" imageClass="tint-white opacity-70" fallbackDelay={Infinity} accessibilityLabel="" />
              </View>
              <View
                accessibilityLabel="Back"
                class="flex-row items-center gap-1 px-2 py-1 rounded-lg cursor-pointer transition-colors hover:bg-white/10"
                onPress={() => p.go("today")}
              >
                <Text class="text-[13px] font-medium text-white/80">‹ Back</Text>
              </View>
              <Text class="text-[15px] font-semibold text-white">Ledger</Text>
            </View>
            <View class="flex-1 flex-row justify-center">
              <View class="flex-row items-center p-1 bg-white/10 rounded-full">
                {(["view", "edit"] as const).map((m) => (
                  <View
                    class={`group flex-row items-center gap-1.5 px-3 py-1 rounded-full cursor-pointer transition-all ${
                      txMode() === m ? "bg-white/20 shadow-sm" : "hover:bg-white/10"
                    }`}
                    onPress={() => { txMode.set(m); if (m === "view") txPicked.set([]); }}
                  >
                    <Avatar
                      src={m === "view" ? TX_ICONS.eye : TX_ICONS.pencil}
                      class="w-3.5 h-3.5"
                      imageClass={txMode() === m ? "tint-white" : "tint-white opacity-50 group-hover:opacity-100"}
                      fallbackDelay={Infinity}
                      accessibilityLabel=""
                    />
                    <Text class={`text-[13px] font-medium ${txMode() === m ? "text-white" : "text-white/50 group-hover:text-white"}`}>
                      {m === "view" ? "View" : "Edit"}
                    </Text>
                  </View>
                ))}
              </View>
            </View>
            <View class="w-8" />
          </View>

          {/* Date navigator — DateRangeNavigator */}
          <View class="flex-row items-center gap-2 bg-gradient-to-r from-[#1a6b3c] to-[#27ae60] px-2 py-2.5 sm:px-5 dark:from-[#145a30] dark:to-[#1e8c4c]">
            <View class="flex-row items-center rounded-lg bg-white/10 shrink-0">
              <View
                class={`w-8 h-8 items-center justify-center rounded-l-lg transition-colors ${month().prev ? "cursor-pointer hover:bg-white/10" : ""}`}
                onPress={() => month().prev && goMonth(String(month().prev))}
              >
                <Avatar src={ICONS.chevronLeft} class="w-3.5 h-3.5" imageClass={`tint-white ${month().prev ? "opacity-70" : "opacity-20"}`} fallbackDelay={Infinity} accessibilityLabel="" />
              </View>
              <Text class="px-2 text-xs font-medium text-white/70 truncate">{String(month().range || month().label || "")}</Text>
              <View
                class={`w-8 h-8 items-center justify-center rounded-r-lg transition-colors ${month().next ? "cursor-pointer hover:bg-white/10" : ""}`}
                onPress={() => month().next && goMonth(String(month().next))}
              >
                <Avatar src={ICONS.chevronRight} class="w-3.5 h-3.5" imageClass={`tint-white ${month().next ? "opacity-70" : "opacity-20"}`} fallbackDelay={Infinity} accessibilityLabel="" />
              </View>
            </View>
            <View class="flex-1 min-w-0 flex-row items-center gap-1 overflow-hidden p-1">
              {tabs().map((k) => (
                <View
                  class={`group shrink-0 flex-row items-center gap-1 px-3 py-1 rounded-md cursor-pointer transition-all ${
                    k === tabKey() ? "bg-white/20 shadow-sm border border-white/20" : "border border-transparent hover:bg-white/10"
                  }`}
                  onPress={() => goMonth(k)}
                >
                  <Text class={`text-[13px] font-medium whitespace-nowrap ${k === tabKey() ? "text-white" : "text-white/50 group-hover:text-white"}`}>
                    {MONTH_KEYS[Number(k.slice(5, 7)) - 1]?.slice(0, 3)}
                  </Text>
                  <If when={k === tabs()[tabs().length - 1]}>
                    <Text class="text-[9px] text-white/60 italic">Today</Text>
                  </If>
                  <Text class="text-[10px] text-white/40">{k.slice(0, 4)}</Text>
                </View>
              ))}
            </View>
            <View class="relative flex-row items-stretch shrink-0 bg-white/10 rounded-lg h-8">
              <View
                class="flex-row items-center gap-1.5 px-2.5 sm:px-3 rounded-l-lg cursor-pointer transition-colors hover:bg-white/10"
                onPress={() => { txSplitOpen.set(false); p.sheet("journal"); }}
              >
                <Avatar src={TX_ICONS.plus} class="w-3 h-3" imageClass="tint-white" fallbackDelay={Infinity} accessibilityLabel="" />
                <Text class="text-xs font-medium text-white whitespace-nowrap hidden sm:block">Add Transaction</Text>
              </View>
              <View
                accessibilityLabel="More add options"
                class="w-7 items-center justify-center border-l border-white/20 rounded-r-lg cursor-pointer transition-colors hover:bg-white/10"
                onPress={() => txSplitOpen.set(!txSplitOpen())}
              >
                <Avatar src={ICONS.chevronDown} class="w-3 h-3" imageClass="tint-white" fallbackDelay={Infinity} accessibilityLabel="" />
              </View>
              <If when={txSplitOpen()}>
                <View class="absolute right-0 top-full mt-1 w-52 bg-white dark:bg-slate-900 border border-[#e5e7eb] dark:border-slate-700 rounded-lg shadow-lg z-50 py-1">
                  <View
                    class="w-full flex-row items-center gap-2 px-3 py-2 cursor-pointer transition-colors hover:bg-[#f9fafb] dark:hover:bg-slate-800 rounded-t-lg"
                    onPress={() => { txSplitOpen.set(false); p.go("reconcile"); }}
                  >
                    <Avatar src={TX_ICONS.upload} class="w-3.5 h-3.5 shrink-0" imageClass="tint-[#94a3b8] dark:tint-slate-500" fallbackDelay={Infinity} accessibilityLabel="" />
                    <Text class="text-xs text-[#374151] dark:text-slate-300">Import Bank Transactions…</Text>
                  </View>
                  <View
                    class="w-full flex-row items-center gap-2 px-3 py-2 cursor-pointer transition-colors hover:bg-[#f9fafb] dark:hover:bg-slate-800 rounded-b-lg"
                    onPress={() => { txSplitOpen.set(false); p.go("oldbooks"); }}
                  >
                    <Avatar src={TX_ICONS.doc} class="w-3.5 h-3.5 shrink-0" imageClass="tint-[#94a3b8] dark:tint-slate-500" fallbackDelay={Infinity} accessibilityLabel="" />
                    <Text class="text-xs text-[#374151] dark:text-slate-300">Import Journal Entries…</Text>
                  </View>
                </View>
              </If>
            </View>
          </View>

          {/* Search + filters — LedgerSearchBar: one bordered input until the
              first chip, then chips + a bare inline input in one wrap row. */}
          <View class="relative z-40">
          <If when={chips().length === 0}>
            <View class="flex-row items-center gap-2 px-5 py-2 border-b border-[#e5e7eb] dark:border-slate-700">
              <View class="flex-1 min-w-0 flex-row items-center gap-2 rounded-lg border border-[#e2e8f0] bg-[#f8fafc] px-2.5 h-9 dark:border-slate-700 dark:bg-slate-800 focus-within:ring-1 focus-within:ring-[#0d9488] focus-within:border-[#0d9488] dark:focus-within:ring-teal-500 dark:focus-within:border-teal-500 transition-all">
                <Avatar src={TX_ICONS.search} class="w-3.5 h-3.5 shrink-0" imageClass="tint-[#94a3b8] dark:tint-slate-500" fallbackDelay={Infinity} accessibilityLabel="" />
                <Input
                  value={txSearch()}
                  placeholder={`Search ${txRows().length || Number(txD()?.total || 0)} transactions…`}
                  onChange={(e: Any) => txSearch.set(typeof e === "string" ? e : e.value)}
                  onSubmit={() => txCommitQuery()}
                  class="flex-1 min-w-0 bg-transparent border-none outline-none text-[13px] text-[#1e293b] dark:text-slate-100 placeholder:text-[#94a3b8] focus:ring-0"
                />
              </View>
              {togglePill}
            </View>
          </If>
          <If when={chips().length > 0}>
            <View class="flex-row items-start gap-2 px-5 py-2 border-b border-[#e5e7eb] dark:border-slate-700">
              <View class="h-8 items-center justify-center shrink-0">
                <Avatar src={TX_ICONS.search} class="w-3.5 h-3.5" imageClass="tint-[#94a3b8] dark:tint-slate-500" fallbackDelay={Infinity} accessibilityLabel="" />
              </View>
              <View class={`flex-1 min-w-0 flex-row flex-wrap items-center gap-1.5 ${txChipsOpen() ? "max-h-[128px]" : "max-h-[32px]"} overflow-hidden`}>
                {chips().map((c) => (
                  <TxChip label={c.label} clear={c.clear} />
                ))}
                <Input
                  value={txSearch()}
                  placeholder="Search or add filter…"
                  onChange={(e: Any) => txSearch.set(typeof e === "string" ? e : e.value)}
                  onSubmit={() => txCommitQuery()}
                  class="flex-1 min-w-[120px] py-0.5 bg-transparent border-none outline-none text-[13px] text-[#1e293b] dark:text-slate-100 placeholder:text-[#94a3b8] focus:ring-0"
                />
              </View>
              <If when={chipsWrap()}>
                <View class="w-6 h-6 items-center justify-center rounded cursor-pointer hover:bg-[#f1f5f9] dark:hover:bg-slate-800 shrink-0" onPress={() => txChipsOpen.set(!txChipsOpen())}>
                  <Avatar src={txChipsOpen() ? TX_ICONS.chevronUp : ICONS.chevronDown} class="w-3.5 h-3.5" imageClass="tint-[#94a3b8]" fallbackDelay={Infinity} accessibilityLabel="" />
                </View>
              </If>
              <View class="cursor-pointer shrink-0" onPress={txClearFilters}>
                <Text class="text-[12px] font-medium text-[#0f766e] dark:text-teal-400 hover:underline">Clear</Text>
              </View>
              {togglePill}
            </View>
          </If>

          {/* Filter dropdown — LedgerFilters: right-anchored w-[280px] panel of
              collapsible sections, each a searchable checkbox list. */}
          <If when={txFiltersOpen()}>
            <View class="absolute right-5 top-full mt-1 w-[280px] max-w-[calc(100%-24px)] bg-white dark:bg-slate-900 border border-[#e5e7eb] dark:border-slate-700 rounded-xl shadow-xl z-50 overflow-hidden">
              <View class="flex-row items-center justify-between px-4 py-3 border-b border-[#f1f5f9] dark:border-slate-800">
                <Text class="text-[13px] font-semibold text-[#1e293b] dark:text-slate-100">Filters</Text>
                <View class="flex-row items-center gap-2">
                  <If when={txHasFilters()}>
                    <View class="cursor-pointer" onPress={txClearFilters}>
                      <Text class="text-[11px] font-medium text-[#2a9d8f] hover:text-[#248f82]">Clear all</Text>
                    </View>
                  </If>
                  <View
                    accessibilityLabel="Close filters"
                    class="w-6 h-6 rounded items-center justify-center cursor-pointer transition-colors hover:bg-[#f1f5f9] dark:hover:bg-slate-800"
                    onPress={() => txFiltersOpen.set(false)}
                  >
                    <Avatar src={TX_ICONS.x} class="w-3 h-3" imageClass="tint-[#94a3b8] dark:tint-slate-500" fallbackDelay={Infinity} accessibilityLabel="" />
                  </View>
                </View>
              </View>
              <Scroll class="max-h-[420px]">
                <View>
                  <TxFilterSection title="Party" icon={TX_ICONS.fParty} count={txParties().length + (txPartyText().trim() ? 1 : 0)}>
                    <TxCheckList
                      options={txPartyOpts()}
                      picked={txParties()}
                      search={txPartyText}
                      onSearch={(v) => txPartyText.set(v)}
                      onToggle={(v) => txToggleIn(txParties, v)}
                      ph="Search parties…"
                      empty="No parties found"
                    />
                  </TxFilterSection>
                  <TxFilterSection title="Amount" icon={TX_ICONS.fAmount} count={txAmountMin().trim() || txAmountMax().trim() ? 1 : 0}>
                    <View class="flex-row items-center gap-2 py-1">
                      <View class="flex-1">
                        <Text class="text-[10px] text-[#94a3b8] dark:text-slate-500 font-medium mb-0.5">Min</Text>
                        <View class="relative">
                          <View class="absolute left-2 top-0 bottom-0 items-center justify-center">
                            <Text class="text-[11px] text-[#94a3b8] dark:text-slate-500">$</Text>
                          </View>
                          <Input
                            value={txAmountMin()}
                            placeholder="Min 0.00"
                            onChange={(e: Any) => txAmountMin.set((typeof e === "string" ? e : e.value).replace(/[^0-9.,]/g, ""))}
                            class="w-full pl-5 pr-2 py-1.5 rounded-lg border border-[#e2e8f0] dark:border-slate-700 text-[12px] text-[#1e293b] dark:text-slate-100 placeholder-[#cbd5e1] dark:placeholder-slate-600 bg-white dark:bg-slate-800 focus:outline-none"
                          />
                        </View>
                      </View>
                      <Text class="text-[12px] text-[#94a3b8] mt-4">—</Text>
                      <View class="flex-1">
                        <Text class="text-[10px] text-[#94a3b8] dark:text-slate-500 font-medium mb-0.5">Max</Text>
                        <View class="relative">
                          <View class="absolute left-2 top-0 bottom-0 items-center justify-center">
                            <Text class="text-[11px] text-[#94a3b8] dark:text-slate-500">$</Text>
                          </View>
                          <Input
                            value={txAmountMax()}
                            placeholder="Max 0.00"
                            onChange={(e: Any) => txAmountMax.set((typeof e === "string" ? e : e.value).replace(/[^0-9.,]/g, ""))}
                            class="w-full pl-5 pr-2 py-1.5 rounded-lg border border-[#e2e8f0] dark:border-slate-700 text-[12px] text-[#1e293b] dark:text-slate-100 placeholder-[#cbd5e1] dark:placeholder-slate-600 bg-white dark:bg-slate-800 focus:outline-none"
                          />
                        </View>
                      </View>
                    </View>
                  </TxFilterSection>
                  <TxFilterSection title="Source" icon={TX_ICONS.fSource} count={txSources().length}>
                    <View>
                      {TX_SOURCE_OPTS.map(([k, label]) => (
                        <TxCheckRow label={label} on={txSources().includes(k)} onPress={() => txToggleIn(txSources, k)} />
                      ))}
                    </View>
                  </TxFilterSection>
                  <TxFilterSection title="Matched Status" icon={TX_ICONS.fMatched} count={txMatched() ? 1 : 0}>
                    <View>
                      {TX_MATCHED_OPTS.map(([k, label]) => (
                        <TxCheckRow label={label} on={txMatched() === k} onPress={() => txMatched.set(txMatched() === k ? "" : k)} />
                      ))}
                    </View>
                  </TxFilterSection>
                  <TxFilterSection title="Department" icon={TX_ICONS.fDept} count={txDepts().length + (txDeptText().trim() ? 1 : 0)}>
                    <TxCheckList
                      options={txTagOpts("department")}
                      picked={txDepts()}
                      search={txDeptText}
                      onSearch={(v) => txDeptText.set(v)}
                      onToggle={(v) => txToggleIn(txDepts, v)}
                      ph="Search departments…"
                      empty="No departments found"
                    />
                  </TxFilterSection>
                  <TxFilterSection title="Location" icon={TX_ICONS.fLoc} count={txLocs().length + (txLocText().trim() ? 1 : 0)}>
                    <TxCheckList
                      options={txTagOpts("location")}
                      picked={txLocs()}
                      search={txLocText}
                      onSearch={(v) => txLocText.set(v)}
                      onToggle={(v) => txToggleIn(txLocs, v)}
                      ph="Search locations…"
                      empty="No locations found"
                    />
                  </TxFilterSection>
                  <TxFilterSection title="Transaction Type" icon={TX_ICONS.fType} count={txTypes().length}>
                    <View>
                      {TX_TYPE_OPTS.map(([k, label]) => (
                        <TxCheckRow
                          label={label}
                          on={txTypes().includes(k)}
                          icon={k === "money_in" ? ICONS.payIn : k === "money_out" ? ICONS.payOut : k === "transfer" ? ICONS.transfer : ICONS.journal}
                          onPress={() => txToggleIn(txTypes, k)}
                        />
                      ))}
                    </View>
                  </TxFilterSection>
                  <TxFilterSection title="Status" icon={TX_ICONS.fMatched} count={txStatuses().length}>
                    <View>
                      {TX_STATUS_OPTS.map(([k, label]) => (
                        <TxCheckRow
                          label={label}
                          on={txStatuses().includes(k)}
                          color={STATUS_BAR[k] || "#f59e0b"}
                          onPress={() => txToggleIn(txStatuses, k)}
                        />
                      ))}
                    </View>
                  </TxFilterSection>
                </View>
              </Scroll>
            </View>
          </If>
          </View>

          {/* List: flat grid when an account is picked, account groups otherwise */}
          <Scroll class="flex-1 min-h-0">
            <If when={!txData() && !txData.error()}>
              <View class="items-center px-4 py-12">
                <Text class="text-sm text-slate-500 dark:text-slate-400">Loading transactions…</Text>
              </View>
            </If>
            <If when={Boolean(txData()) && !txAccount() && txVisibleGroups().length === 0}>
              <View class="items-center gap-4 px-4 py-12">
                <Text class="text-sm text-slate-500 dark:text-slate-400">No transactions found matching your filters.</Text>
                <View class="flex-row items-center gap-2">
                  <If when={txHasFilters()}>
                    <Btn label="Clear filters" onPress={txClearFilters} />
                  </If>
                  <If when={txChecked().length > 0}>
                    <Btn label="Clear accounts" onPress={() => txChecked.set([])} />
                  </If>
                </View>
              </View>
            </If>
            <If when={Boolean(txData()) && Boolean(txAccount()) && rows().length === 0}>
              <View class="items-center gap-4 px-4 py-12">
                <Text class="text-sm text-slate-500 dark:text-slate-400">No transactions found matching your filters.</Text>
                <If when={txHasFilters()}>
                  <Btn label="Clear filters" onPress={txClearFilters} />
                </If>
              </View>
            </If>

            {/* Grouped — AccountGroupedList */}
            <If when={!txAccount() && txVisibleGroups().length > 0}>
              {() =>
                txVisibleGroups().map((g) => {
                  const t = String(g.ticket);
                  const expanded = txExpandOver()[t] ?? !txAllCollapsed();
                  return (
                    <TxGroup
                      g={g}
                      expanded={expanded}
                      edit={txMode() === "edit"}
                      onToggle={() => txExpandOver.set({ ...txExpandOver(), [t]: !expanded })}
                      onToggleAll={(on) => {
                        const ids = txFiltered(txGroupRows(g)).map((r) => String(r.ticket));
                        txPicked.set(on ? [...new Set([...txPicked(), ...ids])] : txPicked().filter((x) => !ids.includes(x)));
                      }}
                      onCheck={(r) =>
                        txPicked.set(
                          txPicked().includes(String(r.ticket))
                            ? txPicked().filter((x) => x !== String(r.ticket))
                            : [...txPicked(), String(r.ticket)],
                        )
                      }
                      onOpen={(r) => drawerEntry.set(String(r.ticket))}
                    />
                  );
                })
              }
            </If>

            {/* Flat — LedgerRows for the picked account */}
            <If when={Boolean(txAccount()) && rows().length > 0}>
              <TxCols edit={txMode() === "edit"} balance={Boolean(txD()?.running_balance)} />
              <View class="hidden md:block">
                {rows().map((r) => (
                  <TxRow
                    r={r}
                    balance={Boolean(txD()?.running_balance)}
                    edit={txMode() === "edit"}
                    checked={txPicked().includes(String(r.ticket))}
                    onCheck={() =>
                      txPicked.set(
                        txPicked().includes(String(r.ticket))
                          ? txPicked().filter((t) => t !== String(r.ticket))
                          : [...txPicked(), String(r.ticket)],
                      )
                    }
                    onOpen={() => drawerEntry.set(String(r.ticket))}
                  />
                ))}
              </View>
              <View class="md:hidden">
                {rows().map((r) => (
                  <TxCard
                    r={r}
                    edit={txMode() === "edit"}
                    checked={txPicked().includes(String(r.ticket))}
                    onCheck={() =>
                      txPicked.set(
                        txPicked().includes(String(r.ticket))
                          ? txPicked().filter((t) => t !== String(r.ticket))
                          : [...txPicked(), String(r.ticket)],
                      )
                    }
                    onOpen={() => drawerEntry.set(String(r.ticket))}
                  />
                ))}
              </View>
              {/* "Showing N of M" + Load more while the tool has another page */}
              <View class="flex-row items-center justify-between px-3 py-2">
                <Text class="text-[11px] text-slate-500 dark:text-slate-400">
                  {`Showing ${rows().length} of ${Number(txD()?.total || rows().length)}`}
                </Text>
                <If when={Boolean(txNextCursor())}>
                  <View class="cursor-pointer" onPress={txLoadMore}>
                    <Text class="text-[12px] font-medium text-[#0f766e] dark:text-teal-300 hover:underline">
                      {txLoadingMore() ? "Loading…" : "Load more"}
                    </Text>
                  </View>
                </If>
              </View>
            </If>
          </Scroll>

          {/* Account summary card — AccountSummaryCard */}
          <If when={Boolean(txAccount()) && Boolean(d()?.account)}>
            <View class="flex-row justify-end px-3 pb-2">
              <View class="rounded-lg border border-[#e5e7eb] dark:border-slate-700 bg-white dark:bg-slate-900 px-4 py-2.5 shadow-sm">
                <Text class="text-[12px] text-[#94a3b8] dark:text-slate-500">
                  {String(d()?.account?.number || "")} {String(d()?.account?.name || "")}
                </Text>
                <Text class="tabular-nums text-[15px] font-bold text-[#1e293b] dark:text-slate-100">
                  {String(d()?.account?.balance_shown || "")}
                </Text>
              </View>
            </View>
          </If>

          {/* Batch bar (edit mode) — BatchActionBar */}
          <If when={txMode() === "edit" && txPicked().length > 0}>
            <View class="flex-row flex-wrap items-center gap-3 min-h-[52px] md:min-h-0 border-t border-[#e5e7eb] dark:border-slate-700 bg-white dark:bg-slate-900 px-4 py-2.5 shadow">
              <Text class="text-[13px] font-medium text-[#334155] dark:text-slate-300">{txPicked().length} selected</Text>
              <View class="flex-1" />
              <If when={voidables().length > 0}>
                <Btn label={`Void ${voidables().length} posted`} tone="vi" disabled={txBusy()} onPress={() => txConfirm.set("void")} />
              </If>
              <If when={drafts().length > 0}>
                <Btn label={`Discard ${drafts().length} drafts`} disabled={txBusy()} onPress={() => txConfirm.set("discard")} />
              </If>
              <Btn label="Clear" onPress={() => txPicked.set([])} />
            </View>
          </If>
        </View>
      </View>

      {/* Accounts sheet below lg — fixed + sibling backdrop (kit's sheet pattern);
          Overlay+absolute mis-positions on the native host. */}
      <If when={txSideOpen()}>
        <View class="fixed inset-0 lg:hidden">
          <View class="absolute inset-0 bg-black/30" onPress={() => txSideOpen.set(false)} />
          <View class="absolute left-0 top-0 bottom-0 w-[280px] bg-white dark:bg-slate-900 shadow-2xl border-r border-[#e5e7eb] dark:border-slate-700">
            <View class="flex-row items-center justify-between border-b border-[#e5e7eb] dark:border-slate-700 pl-4 pr-2 py-2">
              <Text class="text-sm font-semibold text-[#1e293b] dark:text-slate-100">Accounts</Text>
              <View class="w-8 h-8 items-center justify-center rounded-lg cursor-pointer hover:bg-[#f1f5f9] dark:hover:bg-slate-700" onPress={() => txSideOpen.set(false)}>
                <Avatar src={TX_ICONS.x} class="w-4 h-4" imageClass="tint-[#94a3b8]" fallbackDelay={Infinity} accessibilityLabel="" />
              </View>
            </View>
            <TxSide />
          </View>
        </View>
      </If>

      {/* Confirm modal — ConfirmModal (Delete Transactions → Books void/discard) */}
      <If when={Boolean(txConfirm())}>
        <View class="fixed inset-0 items-center justify-center">
          <View class="absolute inset-0 bg-black/30" onPress={() => txConfirm.set("")} />
          <View class="w-[380px] rounded-xl bg-white dark:bg-slate-900 p-5 gap-3 shadow-xl border border-[#e5e7eb] dark:border-slate-700">
            <Text class="text-[15px] font-semibold text-[#1e293b] dark:text-slate-100">
              {txConfirm() === "void" ? "Void transactions" : "Discard drafts"}
            </Text>
            <Text class="text-[13px] text-[#475569] dark:text-slate-400">
              {txConfirm() === "void"
                ? `Void ${voidables().length} posted transaction(s)? A reversing entry is recorded — it cannot be undone.`
                : `Discard ${drafts().length} draft(s)? This cannot be undone.`}
            </Text>
            <View class="flex-row justify-end gap-2">
              <Btn label="Cancel" disabled={txBusy()} onPress={() => txConfirm.set("")} />
              <Btn
                label={txConfirm() === "void" ? "Void" : "Discard"}
                tone="vi"
                disabled={txBusy()}
                onPress={() => actBatch(txConfirm() === "void" ? "void" : "discard")}
              />
            </View>
          </View>
        </View>
      </If>
    </View>
  );
}
