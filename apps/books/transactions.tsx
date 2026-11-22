// Transactions: the old app's /transactions 3-panel ledger (routes/transactions.tsx):
// COA sidebar, green header with View/Edit pill, month navigator + tabs, search
// + filter chips, grouped list. Books' tools answer one account per month, so
// the list is the month's rows (grouped by account only in the old app; the
// tool's rows carry no per-row account). Row press opens the window's entry
// drawer, the way the native ledger's drawer does. Edit mode keeps select +
// Void (posted) / Discard (drafts) — the only batch writes Books tools offer;
// inline edits, batch match and CSV export have no tool, so they're left out.

const txMonth = signal(String(PROPS.month || "")); // "YYYY-MM", "" = the tool's current month
const txAccount = signal(""); // account ticket, "" = All accounts
const txFilter = signal("all"); // tool enum: all/review/draft/posted/checks
const txSearch = signal("");
const txFiltersOpen = signal(false);
const txMode = signal("view"); // "view" | "edit"
const txPicked = signal<string[]>([]); // selected entry tickets in edit mode
const txSideOpen = signal(false); // accounts sheet below lg
const txConfirm = signal<"" | "void" | "discard">("");
const txBusy = signal(false);
// client-side filters (the tool's own enum stays on `txFilter`)
const txTypes = signal<string[]>([]);
const txStatuses = signal<string[]>([]);
const txAmountMin = signal("");
const txAmountMax = signal("");
const txPartyText = signal("");

const txData = resource(
  () => ({ v: ver(), m: txMonth(), a: txAccount(), f: txFilter() }),
  (k: Any) =>
    buwiz.call("books_ledger_list", {
      ...(k.m ? { month: k.m } : {}),
      ...(k.a ? { account: k.a } : {}),
      filter: k.f,
    }),
);
const txAccounts = resource(
  () => ({ v: ver() }),
  () => buwiz.call("books_accounts_list", {}),
);

const txD = () => txData() as Any;
const txRows = () => ((txD()?.rows as Any[]) || []) as Any[];
// Kit ICONS has no search or x yet (kit.tsx is agent B's); same svgIcon helper.
const TX_ICONS = {
  search: svgIcon('<circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>', 2),
  x: svgIcon('<path d="M18 6 6 18"/><path d="m6 6 12 12"/>', 2),
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

/** Rows after search + the client-side filters (type, status, amount, party). */
function txFiltered(): Any[] {
  const q = txSearch().trim().toLowerCase();
  const types = txTypes();
  const statuses = txStatuses();
  const min = txAmountMin().trim();
  const max = txAmountMax().trim();
  const party = txPartyText().trim().toLowerCase();
  return txRows().filter((r) => {
    if (types.length && !types.includes(String(r.form || ""))) return false;
    if (statuses.length) {
      const rev = String(r.pill || "") === "rev";
      const st = String(r.status || "");
      if (!statuses.some((s) => (s === "review" ? rev : st === s))) return false;
    }
    const amt = String(r.amount_shown || r.amount || "0").replace("-", "");
    if (min && cmpDec(amt, min) < 0) return false;
    if (max && cmpDec(amt, max) > 0) return false;
    if (party && !String(r.payee || r.title || "").toLowerCase().includes(party)) return false;
    if (!q) return true;
    return [r.title, r.payee, r.category, r.sub, r.no]
      .some((x) => String(x || "").toLowerCase().includes(q));
  });
}

const TX_TYPE_OPTS: [string, string][] = [
  ["money_in", "Money in"],
  ["money_out", "Money out"],
  ["transfer", "Transfer"],
  ["journal", "Journal"],
];
const TX_STATUS_OPTS: [string, string][] = [
  ["draft", "Draft"],
  ["posted", "Posted"],
  ["voided", "Voided"],
  ["review", "Needs review"],
];
const TX_FORMS: Record<string, string> = {
  money_in: "Money in",
  money_out: "Money out",
  transfer: "Transfer",
  journal: "Journal",
};

function txToggleIn(list: () => string[], set: (v: string[]) => void, v: string) {
  set(list().includes(v) ? list().filter((x) => x !== v) : [...list(), v]);
}
function txHasFilters(): boolean {
  return Boolean(
    txTypes().length ||
      txStatuses().length ||
      txAmountMin().trim() ||
      txAmountMax().trim() ||
      txPartyText().trim() ||
      txSearch().trim(),
  );
}
function txClearFilters() {
  txTypes.set([]);
  txStatuses.set([]);
  txAmountMin.set("");
  txAmountMax.set("");
  txPartyText.set("");
  txSearch.set("");
}

/** The old app's per-row source/dimension columns Books has no data for. */
const TX_DIMS: Record<string, string> = { department: "—", location: "—" };

/** Date cell: "29" + "Oct 2026" under it (LedgerRow's formatDate). */
function txDayParts(iso: string): [string, string] {
  const d = String(iso || "");
  const day = String(Number(d.slice(8, 10) || 0));
  const mon = MONTH_KEYS[Number(d.slice(5, 7)) - 1]?.slice(0, 3) || "";
  return [day, `${mon} ${d.slice(0, 4)}`];
}

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

const STATUS_BAR: Record<string, string> = {
  draft: "#fbbf24",
  posted: "#22c55e",
  voided: "#ef4444",
};

/** One ledger row (LedgerRow.tsx grid mode): bar | date | party | src | category | dept | loc | amount | balance? | › */
function TxRow(p: { r: Any; balance: boolean; edit: boolean; checked: boolean; onCheck: () => void; onOpen: () => void }) {
  const r = () => p.r as Any;
  const [day, my] = txDayParts(String(r().date || ""));
  const title = String(r().title || r().payee || "—");
  const sub = String(r().sub || "");
  const pill = String(r().pill || "");
  const bar = STATUS_BAR[String(r().status || "")] || "#cbd5e1";
  return (
    <View
      class={`flex-row items-stretch cursor-pointer border-b transition-colors ${
        p.checked
          ? "bg-[#eff6ff] dark:bg-blue-950/60 border-[#dbeafe] dark:border-blue-900/40"
          : "border-[#f1f5f9] dark:border-slate-800 hover:bg-[#fafbfc] dark:hover:bg-slate-800"
      }`}
      onPress={p.edit ? p.onCheck : p.onOpen}
    >
      <View class="w-1 shrink-0" style={{ backgroundColor: pill === "rev" ? "#fbbf24" : bar }} />
      <If when={p.edit}>
        <View class="pl-3 pr-1 py-3 items-center justify-center shrink-0">
          <View
            class={`w-4 h-4 rounded border items-center justify-center transition-all ${
              p.checked ? "bg-[#2dd4bf] border-[#2dd4bf]" : "bg-white dark:bg-slate-800 border-gray-300 dark:border-slate-600"
            }`}
          >
            <If when={p.checked}>
              <Avatar src={ICONS.check} class="w-2.5 h-2.5" imageClass="tint-white" fallbackDelay={Infinity} accessibilityLabel="" />
            </If>
          </View>
        </View>
      </If>
      <View class="w-[72px] py-2.5 px-2 items-center shrink-0">
        <Text class="text-[13px] font-semibold text-[#1e293b] dark:text-slate-100 leading-tight">{day}</Text>
        <Text class="text-[10px] text-[#94a3b8] dark:text-slate-500">{my}</Text>
      </View>
      <View class="flex-1 min-w-0 flex-row items-center gap-2.5 py-2.5 px-2">
        <View class="min-w-0">
          <Text class="text-[13px] font-medium text-[#1e293b] dark:text-slate-100 truncate">{title}</Text>
          <If when={Boolean(sub)}>
            <Text class="text-[11px] text-[#94a3b8] dark:text-slate-500 truncate">{sub}</Text>
          </If>
        </View>
      </View>
      <View class="w-10 py-2.5 px-1 items-center shrink-0">
        <Text class="text-[11px] text-[#cbd5e1] dark:text-slate-600 italic">—</Text>
      </View>
      <View class="w-[160px] py-2.5 px-2 shrink-0 min-w-0">
        <Text class="text-[12px] text-[#475569] dark:text-slate-300 truncate">{String(r().category || "") || "—"}</Text>
      </View>
      <View class="w-[130px] py-2.5 px-2 shrink-0 hidden xl:block">
        <Text class="text-[11px] text-[#cbd5e1] dark:text-slate-600 italic">{TX_DIMS.department}</Text>
      </View>
      <View class="w-[130px] py-2.5 px-2 shrink-0 hidden xl:block">
        <Text class="text-[11px] text-[#cbd5e1] dark:text-slate-600 italic">{TX_DIMS.location}</Text>
      </View>
      <View class="w-[100px] py-2.5 px-2 items-end shrink-0">
        <Text class={`tabular-nums whitespace-nowrap text-[13px] font-semibold ${String(r().tone) === "neg" ? "text-[#1e293b] dark:text-slate-100" : "text-[#16a34a] dark:text-emerald-400"}`}>
          {String(r().amount_shown || r().amount || "")}
        </Text>
      </View>
      <If when={p.balance}>
        <View class="w-[110px] py-2.5 px-2 items-end shrink-0">
          <Text class="tabular-nums text-[12px] text-[#64748b] dark:text-slate-400">{String(r().balance_shown || "")}</Text>
        </View>
      </If>
      <View class="w-9 items-center justify-center shrink-0">
        <Avatar src={ICONS.chevronRight} class="w-4 h-4" imageClass="tint-[#cbd5e1] dark:tint-slate-600" fallbackDelay={Infinity} accessibilityLabel="" />
      </View>
    </View>
  );
}

/** Card-mode row below md (LedgerRow's card: avatar, title, sub, chips, amount). */
function TxCard(p: { r: Any; onOpen: () => void }) {
  const r = () => p.r as Any;
  const [initials, color] = txPartyAvatar(String(r().payee || r().title || "?"));
  return (
    <View
      class="border-b border-[#f1f5f9] dark:border-slate-800 px-4 py-3 transition-colors cursor-pointer hover:bg-[#fafbfc] dark:hover:bg-slate-800"
      onPress={p.onOpen}
    >
      <View class="flex-row gap-3">
        <View class="mt-0.5 h-8 w-8 shrink-0 items-center justify-center rounded-full" style={{ backgroundColor: color }}>
          <Text class="text-[11px] font-bold text-white">{initials}</Text>
        </View>
        <View class="flex-1 min-w-0">
          <Text class="truncate text-sm font-semibold text-[#1e293b] dark:text-slate-100">{String(r().title || r().payee || "—")}</Text>
          <If when={Boolean(r().sub)}>
            <Text class="mt-0.5 truncate text-xs text-[#94a3b8] dark:text-slate-500">{String(r().sub)}</Text>
          </If>
          <View class="mt-1 flex-row flex-wrap items-center gap-1.5">
            <Text class="text-[10px] font-medium uppercase tracking-wider text-[#94a3b8] dark:text-slate-500">
              {TX_FORMS[String(r().form)] || String(r().form || "")}
            </Text>
            <Pill tone={String(r().pill || "ok")} label={String(r().status_label || r().status || "")} />
            <If when={Boolean(r().category)}>
              <Text class="rounded-full bg-[#f8fafc] px-2 py-0.5 text-xs font-medium text-[#475569] dark:bg-slate-800 dark:text-slate-300">
                {String(r().category)}
              </Text>
            </If>
          </View>
        </View>
        <View class="shrink-0 items-end gap-0.5">
          <Text class={`tabular-nums text-[15px] font-semibold ${String(r().tone) === "neg" ? "text-[#1e293b] dark:text-slate-100" : "text-[#16a34a] dark:text-emerald-400"}`}>
            {String(r().amount_shown || r().amount || "")}
          </Text>
          <Text class="text-xs font-medium text-[#94a3b8] dark:text-slate-500">{String(r().date_short || "")}</Text>
        </View>
      </View>
    </View>
  );
}

/** Sidebar account row (AccountSidebar): number · name · balance. */
function TxSideRow(p: { a: Any; on: boolean; onPress: () => void }) {
  const a = () => p.a as Any;
  return (
    <View
      class={`flex-row items-center gap-2 px-3 py-2 cursor-pointer transition-colors ${
        p.on ? "bg-[#f0fdfa] dark:bg-teal-950/40" : "hover:bg-[#f8fafc] dark:hover:bg-slate-800"
      }`}
      onPress={p.onPress}
    >
      <Text class={`text-[11px] tabular-nums w-[52px] shrink-0 ${p.on ? "text-[#0f766e] dark:text-teal-400" : "text-[#94a3b8] dark:text-slate-500"}`}>
        {String(a().number || "")}
      </Text>
      <Text class={`flex-1 min-w-0 truncate text-[13px] ${p.on ? "font-medium text-[#0f766e] dark:text-teal-300" : "text-[#334155] dark:text-slate-300"}`}>
        {String(a().name || "")}
      </Text>
      <Text class={`tabular-nums text-[11px] shrink-0 ${p.on ? "text-[#0f766e] dark:text-teal-400" : "text-[#94a3b8] dark:text-slate-500"}`}>
        {String(a().balance_shown || "")}
      </Text>
    </View>
  );
}

/** The COA sidebar contents, shared by the lg column and the sheet. */
function TxSide() {
  const accts = () => (((txAccounts() as Any)?.rows as Any[]) || []).filter((a) => !a.off);
  const groups = () => {
    const order = ["Assets", "Liabilities", "Equity", "Revenue", "Expenses"];
    const out: { g: string; rows: Any[] }[] = [];
    for (const g of order) {
      const rows = accts().filter((a) => a.group === g);
      if (rows.length) out.push({ g, rows });
    }
    return out;
  };
  const pick = (ticket: string) => {
    txAccount.set(ticket);
    txSideOpen.set(false);
  };
  return (
    <View class="flex-1">
      <View class="px-3 pt-3 pb-1">
        <Text class="text-[11px] font-semibold uppercase tracking-wider text-[#94a3b8] dark:text-slate-500">Accounts</Text>
      </View>
      <TxSideRow a={{ number: "", name: "All accounts", balance_shown: txD()?.account?.ticket ? "" : String(txD()?.account?.balance_shown || "") }} on={!txAccount()} onPress={() => pick("")} />
      {groups().map((g) => (
        <View>
          <Text class="px-3 pt-2 pb-1 text-[10px] font-semibold uppercase tracking-wider text-[#cbd5e1] dark:text-slate-600">{g.g}</Text>
          {g.rows.map((a) => (
            <TxSideRow a={a} on={txAccount() === a.ticket} onPress={() => pick(a.ticket)} />
          ))}
        </View>
      ))}
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
  const selRows = () => rows().filter((r) => txPicked().includes(String(r.ticket)));
  const actBatch = async (tool: string) => {
    txBusy.set(true);
    try {
      for (const r of selRows()) {
        await act(tool, { entry: String(r.ticket) });
      }
      toast(tool === "books_app_void_entry" ? "Voided." : "Discarded.");
      txPicked.set([]);
      txConfirm.set("");
      bump();
    } catch (e: Any) {
      toast(errText(e));
    } finally {
      txBusy.set(false);
    }
  };
  const voidables = () => selRows().filter((r) => r.status === "posted");
  const drafts = () => selRows().filter((r) => r.status === "draft");

  const colHead = (label: string, cls: string) => (
    <Text class={`${cls} px-2 text-[11px] font-semibold text-slate-500 dark:text-slate-400 uppercase tracking-wider`}>{label}</Text>
  );
  const header = (
    <View class="hidden md:flex flex-row items-center px-0 py-2 border-b border-[#f1f5f9] dark:border-slate-800 bg-white dark:bg-[#0f172a]">
      <View class="w-1" />
      <If when={txMode() === "edit"}>
        <View class="w-8" />
      </If>
      {colHead("Date", "w-[72px] text-center")}
      {colHead("Party", "flex-1 min-w-0")}
      {colHead("Src", "w-10 text-center")}
      {colHead("Category", "w-[160px]")}
      {colHead("Department", "w-[130px] hidden xl:block")}
      {colHead("Location", "w-[130px] hidden xl:block")}
      {colHead("Amount", "w-[100px] text-right")}
      <If when={Boolean(txAccount())}>
        {colHead("Balance", "w-[110px] text-right")}
      </If>
      <View class="w-9" />
    </View>
  );

  const chips = () => {
    const out: { label: string; clear: () => void }[] = [];
    for (const t of txTypes()) out.push({ label: TX_FORMS[t] || t, clear: () => txToggleIn(txTypes, txTypes.set, t) });
    for (const s of txStatuses()) out.push({ label: (TX_STATUS_OPTS.find(([k]) => k === s) || ["", s])[1], clear: () => txToggleIn(txStatuses, txStatuses.set, s) });
    if (txAmountMin().trim() || txAmountMax().trim())
      out.push({ label: `Amount ${txAmountMin().trim() || "0"}–${txAmountMax().trim() || "∞"}`, clear: () => { txAmountMin.set(""); txAmountMax.set(""); } });
    if (txPartyText().trim()) out.push({ label: `Party: ${txPartyText().trim()}`, clear: () => txPartyText.set("") });
    return out;
  };

  return (
    <View class="flex-1 min-h-0 flex-row bg-white dark:bg-slate-900">
      {/* COA sidebar: fixed 240px column at lg+, an overlay sheet below */}
      <View class="hidden lg:flex w-[240px] shrink-0 flex-col border-r border-[#e5e7eb] dark:border-slate-700 overflow-hidden">
        <Scroll class="flex-1"><TxSide /></Scroll>
      </View>
      <View class="flex-1 min-w-0 flex flex-col">
        <View class="flex-1 min-h-0 flex flex-col md:m-3 md:rounded-lg md:border md:border-[#e5e7eb] md:shadow-sm overflow-hidden bg-white dark:bg-slate-900">
          {/* Header — LedgerHeader */}
          <View class="flex-row items-center px-5 py-3 bg-gradient-to-r from-[#1a6b3c] to-[#27ae60] dark:from-[#145a30] dark:to-[#1e8c4c] rounded-t-lg">
            <View class="flex-row items-center gap-3 shrink-0">
              <View
                class="w-8 h-8 rounded-lg items-center justify-center transition-colors hover:bg-white/10 cursor-pointer"
                onPress={() => txSideOpen.set(true)}
              >
                <Avatar src={ICONS.table} class="w-4 h-4" imageClass="tint-white opacity-70" fallbackDelay={Infinity} accessibilityLabel="" />
              </View>
              <Text class="text-[15px] font-semibold text-white">Ledger</Text>
            </View>
            <View class="flex-1 flex-row justify-center">
              <View class="flex-row items-center p-1 bg-white/10 rounded-full">
                {(["view", "edit"] as const).map((m) => (
                  <View
                    class={`flex-row items-center px-3 py-1 rounded-full cursor-pointer transition-all ${
                      txMode() === m ? "bg-white/20 shadow-sm" : "hover:bg-white/10"
                    }`}
                    onPress={() => { txMode.set(m); if (m === "view") txPicked.set([]); }}
                  >
                    <Text class={`text-[13px] font-medium ${txMode() === m ? "text-white" : "text-white/50"}`}>
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
                class="w-8 h-8 items-center justify-center rounded-l-lg cursor-pointer transition-colors hover:bg-white/10"
                onPress={() => month().prev && goMonth(String(month().prev))}
              >
                <Avatar src={ICONS.chevronLeft} class="w-4 h-4" imageClass="tint-white opacity-70" fallbackDelay={Infinity} accessibilityLabel="" />
              </View>
              <Text class="px-2 text-xs font-medium text-white/70 truncate">{String(month().range || month().label || "")}</Text>
              <View
                class="w-8 h-8 items-center justify-center rounded-r-lg cursor-pointer transition-colors hover:bg-white/10"
                onPress={() => month().next && goMonth(String(month().next))}
              >
                <Avatar src={ICONS.chevronRight} class="w-4 h-4" imageClass="tint-white opacity-70" fallbackDelay={Infinity} accessibilityLabel="" />
              </View>
            </View>
            <View class="flex-1 min-w-0 flex-row items-center gap-1 overflow-hidden p-1">
              {tabs().map((k) => (
                <View
                  class={`shrink-0 flex-row items-center gap-1 px-3 py-1 rounded-md cursor-pointer transition-all ${
                    k === tabKey() ? "bg-white/20 shadow-sm border border-white/20" : "border border-transparent hover:bg-white/10"
                  }`}
                  onPress={() => goMonth(k)}
                >
                  <Text class={`text-[13px] font-medium whitespace-nowrap ${k === tabKey() ? "text-white" : "text-white/50"}`}>
                    {MONTH_KEYS[Number(k.slice(5, 7)) - 1]?.slice(0, 3)}
                  </Text>
                  <Text class="text-[10px] text-white/40">{k.slice(0, 4)}</Text>
                </View>
              ))}
            </View>
            <View
              class="shrink-0 flex-row items-center gap-1 rounded-lg bg-white px-3 py-1.5 cursor-pointer shadow-sm transition-colors hover:bg-[#f1f5f9]"
              onPress={() => p.sheet("journal")}
            >
              <Text class="text-[13px] font-semibold text-[#16a34a]">+ Add</Text>
            </View>
          </View>

          {/* Search + filters — LedgerSearchBar */}
          <View class="flex-row items-center gap-2 border-b border-[#e5e7eb] dark:border-slate-700 px-3 py-2">
            <View class="flex-1 min-w-0 flex-row items-center gap-2 rounded-lg border border-[#e2e8f0] bg-[#f9fafb] px-3 py-1.5 dark:border-white/10 dark:bg-white/5">
              <Avatar src={TX_ICONS.search} class="w-3.5 h-3.5" imageClass="tint-[#94a3b8] dark:tint-slate-500" fallbackDelay={Infinity} accessibilityLabel="" />
              <TextBox
                value={txSearch()}
                onChange={(v) => txSearch.set(v)}
                placeholder={`Search ${txRows().length} transactions…`}
              />
            </View>
            <View
              class={`shrink-0 flex-row items-center gap-1.5 rounded-lg border px-3 py-1.5 cursor-pointer transition-colors ${
                txFiltersOpen() || txHasFilters()
                  ? "border-[#2dd4bf] text-[#0f766e] bg-[#f0fdfa] dark:bg-teal-950/40 dark:border-teal-800"
                  : "border-[#e2e8f0] dark:border-white/10 hover:bg-[#f1f5f9] dark:hover:bg-slate-800"
              }`}
              onPress={() => txFiltersOpen.set(!txFiltersOpen())}
            >
              <Text class={`text-[13px] font-medium ${txFiltersOpen() || txHasFilters() ? "text-[#0f766e] dark:text-teal-300" : "text-[#475569] dark:text-slate-300"}`}>
                Filters
              </Text>
            </View>
          </View>
          <If when={chips().length > 0}>
            <View class="flex-row flex-wrap items-center gap-1.5 px-3 py-1.5 border-b border-[#f1f5f9] dark:border-slate-800">
              {chips().map((c) => (
                <View class="flex-row items-center gap-1 rounded-full bg-[#f1f5f9] px-2.5 py-0.5 dark:bg-slate-800 cursor-pointer hover:bg-[#e2e8f0] dark:hover:bg-slate-700" onPress={c.clear}>
                  <Text class="text-[11px] font-medium text-[#475569] dark:text-slate-300">{c.label}</Text>
                  <Avatar src={TX_ICONS.x} class="w-2.5 h-2.5" imageClass="tint-[#94a3b8]" fallbackDelay={Infinity} accessibilityLabel="" />
                </View>
              ))}
              <View class="cursor-pointer" onPress={txClearFilters}>
                <Text class="text-[11px] font-medium text-[#64748b] dark:text-slate-400 hover:underline">Clear all</Text>
              </View>
            </View>
          </If>
          <If when={txFiltersOpen()}>
            <View class="border-b border-[#e5e7eb] dark:border-slate-700 px-3 py-3 gap-3 bg-[#f8fafc] dark:bg-slate-950/50">
              <View class="gap-1">
                <Text class="text-[11px] font-semibold uppercase tracking-wider text-[#94a3b8] dark:text-slate-500">Type</Text>
                <View class="flex-row flex-wrap gap-1.5">
                  {TX_TYPE_OPTS.map(([k, label]) => (
                    <Chip label={label} on={txTypes().includes(k)} onPress={() => txToggleIn(txTypes, txTypes.set, k)} />
                  ))}
                </View>
              </View>
              <View class="gap-1">
                <Text class="text-[11px] font-semibold uppercase tracking-wider text-[#94a3b8] dark:text-slate-500">Status</Text>
                <View class="flex-row flex-wrap gap-1.5">
                  {TX_STATUS_OPTS.map(([k, label]) => (
                    <Chip label={label} on={txStatuses().includes(k)} onPress={() => txToggleIn(txStatuses, txStatuses.set, k)} />
                  ))}
                </View>
              </View>
              <View class="flex-row flex-wrap gap-3">
                <View class="gap-1">
                  <Text class="text-[11px] font-semibold uppercase tracking-wider text-[#94a3b8] dark:text-slate-500">Amount</Text>
                  <View class="flex-row items-center gap-1.5">
                    <TextBox value={txAmountMin()} onChange={(v) => txAmountMin.set(v)} placeholder="Min" />
                    <Text class="text-[#94a3b8]">–</Text>
                    <TextBox value={txAmountMax()} onChange={(v) => txAmountMax.set(v)} placeholder="Max" />
                  </View>
                </View>
                <View class="gap-1 flex-1 min-w-[160px]">
                  <Text class="text-[11px] font-semibold uppercase tracking-wider text-[#94a3b8] dark:text-slate-500">Party</Text>
                  <TextBox value={txPartyText()} onChange={(v) => txPartyText.set(v)} placeholder="Name contains…" />
                </View>
              </View>
            </View>
          </If>

          {/* List */}
          <Scroll class="flex-1 min-h-0">
            <If when={!txData() && !txData.error()}>
              <View class="items-center px-4 py-12">
                <Text class="text-sm text-slate-500 dark:text-slate-400">Loading transactions…</Text>
              </View>
            </If>
            <If when={Boolean(txData()) && rows().length === 0}>
              <View class="items-center gap-4 px-4 py-12">
                <Text class="text-sm text-slate-500 dark:text-slate-400">No transactions found matching your filters.</Text>
                <If when={txHasFilters()}>
                  <Btn label="Clear filters" onPress={txClearFilters} />
                </If>
              </View>
            </If>
            <If when={rows().length > 0}>
              {header}
              <View class="hidden md:block">
                {rows().map((r) => (
                  <TxRow
                    r={r}
                    balance={Boolean(txAccount()) && Boolean(txD()?.running_balance)}
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
                  <TxCard r={r} onOpen={() => drawerEntry.set(String(r.ticket))} />
                ))}
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
            <View class="flex-row items-center gap-3 border-t border-[#e5e7eb] dark:border-slate-700 bg-white dark:bg-slate-900 px-4 py-2.5 shadow">
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
            <Scroll class="flex-1"><TxSide /></Scroll>
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
                onPress={() => actBatch(txConfirm() === "void" ? "books_app_void_entry" : "books_app_discard_entry")}
              />
            </View>
          </View>
        </View>
      </If>
    </View>
  );
}
