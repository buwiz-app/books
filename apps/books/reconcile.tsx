// Reconcile: accounts down, months across (red means to do), then a
// statement beside Books with matches, suggestions and missing lines, the
// difference, and Finish. A statement with a snapshot shows its page on the
// left and its lines, each over its entry's journal lines, on the right.

const recAccount = signal("");
const recMonth = signal("");
const recQ = signal("");
const recStat = signal("all"); // "all" | "m" matched | "x" unsettled (suggested or missing)
const recFiltMenu = signal(false);
let recFiltAnchor: ViewRef | undefined;

/** "1,234.56" or "+42.18" -> centavos, string math only (integer centavos, no float). */
function centsOf(text: string): bigint {
  const t = String(text ?? "0");
  const neg = /^[-−–]/.test(t.trim()) || /^\(.*\)$/.test(t.trim());
  const cleaned = t.replace(/[^0-9.]/g, "");
  const [w = "", f = ""] = cleaned.split(".");
  const c = BigInt(w || "0") * 100n + BigInt((f + "00").slice(0, 2) || "0");
  return neg ? -c : c;
}

/** Centavos -> "1,234.56", `+` ahead when pos and the sum is in. */
function centsShown(c: bigint, pos = false): string {
  const neg = c < 0n;
  const a = neg ? -c : c;
  const w = (a / 100n).toString().replace(/\B(?=(\d{3})+(?!\d))/g, ",");
  const f = (a % 100n).toString().padStart(2, "0");
  return (neg ? "-" : pos ? "+" : "") + w + "." + f;
}
/** The account a statement document was just kept for: its note stays up until the screen is left. */
const keptFor = signal("");

/** Bytes as base64url, for a file part. */
function b64url(bytes: Any): string {
  const A = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
  let out = "";
  for (let i = 0; i < bytes.length; i += 3) {
    const n = (bytes[i] << 16) | ((bytes[i + 1] || 0) << 8) | (bytes[i + 2] || 0);
    const left = bytes.length - i;
    out += A[(n >> 18) & 63] + A[(n >> 12) & 63];
    if (left > 1) out += A[(n >> 6) & 63];
    if (left > 2) out += A[n & 63];
  }
  return out;
}

/** A picked file to Books in parts (one call's arguments are at most 256 KiB);
 * returns the upload id an import names. */
async function sendFile(file: Any): Promise<string> {
  const bytes = await file.read();
  const size = 96000;
  const parts = Math.max(1, Math.ceil(bytes.length / size));
  const upload = `up-${rid().slice(3)}`;
  for (let i = 0; i < parts; i += 1) {
    await buwiz.call("books_app_upload_part", { upload, part: i, parts, data_base64: b64url(bytes.slice(i * size, (i + 1) * size)) });
  }
  return upload;
}

/** The statement documents Reconcile keeps as they are, by extension. */
const STATEMENT_DOC: Record<string, string> = { pdf: "application/pdf", png: "image/png", jpg: "image/jpeg", jpeg: "image/jpeg" };

/** A statement's PDF or picture is kept in Books documents as a bank
 * statement. Books never reads the page: the assistant does, and its lines
 * come back as a statement read for the user to accept. */
async function keepStatementDocument(account: string, file: Any, mime: string) {
  const upload = await sendFile(file);
  await act("books_app_statement_import", { account, document_upload: upload, document_mime: mime });
  keptFor.set(account);
  toast("Kept in Books documents as a bank statement · ask the assistant to read it");
}

function importStatement(account: string) {
  if (!buwiz.pickFile) return toast("Picking files is not available in this Buwiz Bot.");
  buwiz
    .pickFile({ extensions: [".csv", ".xlsx", ".xls", ".pdf", ".png", ".jpg", ".jpeg"] })
    .then(async (files: Any[]) => {
      if (!files.length) return;
      const mime = STATEMENT_DOC[String(files[0].name || "").split(".").pop()?.toLowerCase() || ""];
      if (mime) return keepStatementDocument(account, files[0], mime);
      const upload = await sendFile(files[0]);
      const r: Any = await act("books_app_statement_import", { account, csv_upload: upload, file_name: String(files[0].name).slice(0, 120) });
      recAccount.set(account);
      recMonth.set(r.month);
      toast(`${String(r.lines)} lines imported`);
    })
    .catch((e: Any) => toast(errText(e)));
}

// ── List page: a 1:1 port of the old web app's /reconciliations ─────────────
// Header band (green gradient, title, year navigator, sort, Start, kebab),
// four filter tiles, then bank accounts grouped Assets / Liabilities with an
// expandable 12-month table: status dot, status pill, View/Continue/Start.

const REC_MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/** Old app cell states mapped to its labels: done→finalized, draft→in progress
 * (a started match), todo→not started (activity, nothing imported), none→no activity. */
const REC_STATE: Record<string, { label: string; pill: string; dot: string; fill: string }> = {
  done: { label: "finalized", pill: "bg-emerald-50 dark:bg-emerald-500/10 text-emerald-600 dark:text-emerald-400", dot: "bg-emerald-500", fill: "#10b981" },
  draft: { label: "in progress", pill: "bg-blue-50 dark:bg-blue-500/10 text-blue-600 dark:text-blue-400", dot: "bg-amber-500", fill: "#f59e0b" },
  todo: { label: "not started", pill: "bg-amber-50 dark:bg-amber-500/10 text-amber-600 dark:text-amber-400", dot: "bg-gray-300 dark:bg-white/20", fill: "#d1d5db" },
  none: { label: "no activity", pill: "bg-gray-100 dark:bg-white/5 text-gray-400 dark:text-white/30", dot: "", fill: "#d1d5db" },
};

const REC_TILES: { key: string; glyph: string; color: string; activeBg: string; label: (y: string) => string }[] = [
  { key: "none", glyph: "◷", color: "#94a3b8", activeBg: "bg-slate-100 dark:bg-slate-800/40", label: () => "No Activity" },
  { key: "todo", glyph: "👥", color: "#6366f1", activeBg: "bg-indigo-50 dark:bg-indigo-900/25", label: (y) => `Not Started for ${y}` },
  { key: "draft", glyph: "🗎", color: "#f59e0b", activeBg: "bg-blue-50 dark:bg-blue-900/25", label: (y) => `Drafts for ${y}` },
  { key: "done", glyph: "✓", color: "#10b981", activeBg: "bg-emerald-50 dark:bg-emerald-900/25", label: (y) => `Finalized for ${y}` },
];

const REC_SORTS: [string, string][] = [
  ["name", "Category Name"],
  ["number", "Category Number"],
  ["group", "Category Subtype"],
  ["last", "Last Reconciled"],
];

const recFilter = signal("");
const recSort = signal("name");
const recSortDesc = signal(false);
const recExpanded = signal<string[]>([]);
const recSortMenu = signal(false);
const recKebabMenu = signal(false);
const recStartMenu = signal(false);
/** The grid year; 0 until the default call names it. */
const recYear = signal(0);
let recSortAnchor: ViewRef | undefined;
let recKebabAnchor: ViewRef | undefined;
let recStartAnchor: ViewRef | undefined;

/** The latest month a row's cells are done in ("2026-02" sorts before "2026-10"). */
function recLastDone(r: Any): string {
  let last = "";
  for (const c of r.cells || []) if (c.state === "done" && String(c.month) > last) last = String(c.month);
  return last;
}

/** Old app's per-account stat: a row counts in a tile by the states its months hold. */
function recStats(rows: Any[]): Record<string, number> {
  const s: Record<string, number> = { none: 0, todo: 0, draft: 0, done: 0 };
  for (const r of rows) {
    const states = (r.cells || []).map((c: Any) => String(c.state));
    const allNone = states.every((st) => st === "none");
    if (allNone) s.none++;
    if (allNone || states.some((st) => st === "todo" || st === "none")) s.todo++;
    if (states.some((st) => st === "draft")) s.draft++;
    if (states.some((st) => st === "done")) s.done++;
  }
  return s;
}

/** Old app's tile filter: which rows a tile keeps. */
function recKeep(r: Any, f: string): boolean {
  const states = (r.cells || []).map((c: Any) => String(c.state));
  if (f === "none") return states.every((st) => st === "none");
  if (f === "todo") return states.some((st) => st === "todo" || st === "none");
  if (f === "draft") return states.some((st) => st === "draft");
  if (f === "done") return states.some((st) => st === "done");
  return true;
}

/** Old app's chevron, the row's chevron + account glyph is text (no SVG icons). */
function ReconcileGrid() {
  const data = resource(
    () => ({ v: ver(), y: recYear() }),
    async () => {
      const y = recYear();
      if (y) return buwiz.call("books_reconcile_grid", { year: y });
      // first load: the default call names the open year; the year call then
      // serves its twelve months like the old app's year matrix
      const d0: Any = await buwiz.call("books_reconcile_grid", {});
      return buwiz.call("books_reconcile_grid", { year: Number(d0?.year) || 0 });
    },
  );
  const accts = resource(
    () => ({ v: ver() }),
    () => buwiz.call("books_accounts_list", { group: "all" }),
  );
  const d = () => data() as Any;
  /** Books account ticket → its COA group ("Assets" | "Liabilities" | …). */
  const groupOf = (ticket: string): string => {
    const a = (accts()?.rows || []).find((x: Any) => x.ticket === ticket);
    return String(a?.group || "Other");
  };
  const year = () => String(d()?.year || "");
  const stats = () => recStats(d()?.rows || []);
  const rows = () => {
    let out = [...(d()?.rows || [])];
    if (recFilter()) out = out.filter((r) => recKeep(r, recFilter()));
    const key = recSort();
    const dir = recSortDesc() ? -1 : 1;
    const val = (r: Any) =>
      key === "name" ? String(r.name).toLowerCase() : key === "number" ? String(r.number) : key === "group" ? groupOf(String(r.ticket)) : recLastDone(r);
    out.sort((a, b) => (val(a) < val(b) ? -dir : val(a) > val(b) ? dir : 0));
    return out;
  };
  const groups = () => {
    const map = new Map<string, Any[]>();
    for (const r of rows()) {
      const g = groupOf(String(r.ticket));
      map.set(g, [...(map.get(g) || []), r]);
    }
    return [...map.entries()];
  };
  const openCell = (r: Any, c: Any) => {
    recAccount.set(String(r.ticket));
    recMonth.set(String(c.month));
  };
  const toggle = (t: string) => recExpanded.set(recExpanded().includes(t) ? recExpanded().filter((x) => x !== t) : [...recExpanded(), t]);
  const oldestOpen = (r: Any): Any => (r.cells || []).find((c: Any) => c.state === "todo" || c.state === "draft") || (r.cells || [])[0];
  const countOf = (r: Any, st: string) => (r.cells || []).filter((c: Any) => c.state === st).length;
  const hdrBtn = "flex-row items-center gap-1.5 px-3 py-2 rounded-lg bg-white/15";
  const hdrBtnHover = { backgroundColor: "rgba(255,255,255,0.25)" };

  return (
    <View class="gap-4">
      {/* ── Header band ── */}
      <View class="rounded-xl px-4 pt-5 pb-4 gap-4" style={{ backgroundImage: "linear-gradient(to right,#1a6b3c,#27ae60)" }}>
        <View class="flex-row items-center gap-3">
          <View class="w-8 h-8 rounded-lg bg-white/15 items-center justify-center">
            <Text class="text-[16px] text-white">✓</Text>
          </View>
          <Text class="text-[18px] font-semibold text-white">Reconciliations</Text>
        </View>
        <View class="flex-row items-center justify-between">
          {/* year navigator (Books' grid serves the open year; the chevrons stay dimmed) */}
          <View class="flex-row items-center bg-white/10 rounded-lg">
            <View
              class="w-8 h-8 items-center justify-center rounded-l-lg"
              hoverStyle={hdrBtnHover}
              activeStyle={{ opacity: 0.7 }}
              onPress={() => { const y = Number(year()); if (y) recYear.set(y - 1); }}
            >
              <Text class="text-[14px] font-bold text-white">‹</Text>
            </View>
            <View class="h-8 flex-row items-center gap-1.5 px-2">
              <Text class="text-[12px] text-white/70">▣</Text>
              <Text class="text-[12px] font-medium text-white/70">{year()}</Text>
            </View>
            <View
              class="w-8 h-8 items-center justify-center rounded-r-lg"
              hoverStyle={hdrBtnHover}
              activeStyle={{ opacity: 0.7 }}
              onPress={() => { const y = Number(year()); if (y) recYear.set(y + 1); }}
            >
              <Text class="text-[14px] font-bold text-white">›</Text>
            </View>
          </View>
          <View class="flex-row items-center gap-2">
            <View
              ref={recSortAnchor}
              class={hdrBtn}
              hoverStyle={hdrBtnHover}
              activeStyle={{ opacity: 0.7 }}
              onPress={() => recSortMenu.set(!recSortMenu())}
            >
              <Text class="text-[12px] font-medium text-white">⇅</Text>
            </View>
            <View
              ref={recStartAnchor}
              class={hdrBtn}
              hoverStyle={hdrBtnHover}
              activeStyle={{ opacity: 0.7 }}
              onPress={() => recStartMenu.set(!recStartMenu())}
            >
              <Text class="text-[12px] font-medium text-white">+ Start Reconciliation</Text>
            </View>
            <View
              ref={recKebabAnchor}
              class="w-8 h-8 rounded-lg bg-white/15 items-center justify-center"
              hoverStyle={hdrBtnHover}
              activeStyle={{ opacity: 0.7 }}
              onPress={() => recKebabMenu.set(!recKebabMenu())}
            >
              <Text class="text-[14px] font-bold text-white">⋮</Text>
            </View>
          </View>
        </View>
      </View>

      {/* ── Summary tiles ── */}
      <Loading res={data} />
      <If when={Boolean(d())}>
        <View class="gap-4">
          {/* Two-up below sm (like the reference's grid-cols-2), four-across from sm. */}
          <View class={`flex-row flex-wrap rounded-xl border overflow-hidden ${C.panel} ${C.line}`}>
            <Each of={REC_TILES}>
              {(t: Any, i: Any) => (
                <View
                  class={`w-1/2 sm:w-auto sm:flex-1 flex-row items-center gap-3 px-5 py-4 cursor-pointer transition-colors ${i() < 3 ? `border-r ${C.line}` : ""} ${i() < 2 ? `border-b ${C.line} sm:border-b-0` : ""} ${recFilter() === t().key ? t().activeBg : "hover:bg-gray-50 dark:hover:bg-white/5"}`}
                  activeStyle={{ opacity: 0.7 }}
                  onPress={() => recFilter.set(recFilter() === t().key ? "" : String(t().key))}
                >
                  <Text class={`text-[26px] ${recFilter() === t().key ? "" : "opacity-50"}`} style={{ color: t().color }}>
                    {t().glyph}
                  </Text>
                  <View>
                    <Text class="text-[24px] font-bold" style={{ color: t().color }}>
                      {String(stats()[String(t().key)] || 0)}
                      {t().key === "done" ? <Text class="text-[14px] font-normal text-gray-400 dark:text-white/30">{` / ${(d()?.rows || []).length}`}</Text> : null}
                    </Text>
                    <Text class={`text-[11px] ${recFilter() === t().key ? "font-medium" : ""}`} style={recFilter() === t().key ? { color: t().color } : undefined}>
                      {t().label(year())}
                    </Text>
                  </View>
                </View>
              )}
            </Each>
          </View>

          {/* ── Filtered-empty state ── */}
          <If when={Boolean(recFilter()) && rows().length === 0}>
            <View class="items-center py-16 gap-3">
              <Text class={`text-[15px] ${C.mute}`}>No Reconciliations</Text>
              <View class="flex-row items-center gap-1.5" activeStyle={{ opacity: 0.7 }} onPress={() => recFilter.set("")}>
                <Text class={`text-[13px] ${C.mute}`}>↻</Text>
                <Text class={`text-[13px] ${C.mute}`}>Reset Filters</Text>
              </View>
            </View>
          </If>

          {/* ── Account groups ── */}
          <Each of={groups()}>{(g: Any) => <ReconcileGroup name={g()[0]} entries={g()[1]} onCell={openCell} />}</Each>
        </View>
      </If>

      {/* ── Sort menu ── */}
      <If when={recSortMenu()}>
        <Overlay anchor={recSortAnchor} placement="bottom" align="end" offset={4} margin={8} onPressOutside={() => recSortMenu.set(false)} onCancel={() => recSortMenu.set(false)}>
          <View class={`w-52 rounded-xl border shadow-xl py-1 ${C.panel} ${C.line}`}>
            <Text class={`px-3 py-1.5 text-[10px] font-semibold uppercase tracking-wider ${C.dim}`}>Sort By</Text>
            <Each of={REC_SORTS}>
              {(o: Any) => (
                <View
                  class={`flex-row items-center gap-2.5 px-3 py-2 cursor-pointer transition-colors ${recSort() === o()[0] ? "bg-emerald-50 dark:bg-emerald-900/20" : "hover:bg-gray-50 dark:hover:bg-white/5"}`}
                  activeStyle={{ opacity: 0.7 }}
                  onPress={() => {
                    if (recSort() === o()[0]) recSortDesc.set(!recSortDesc());
                    else {
                      recSort.set(String(o()[0]));
                      recSortDesc.set(false);
                    }
                  }}
                >
                  <Text class={`text-[12px] ${recSort() === o()[0] ? "text-emerald-600 dark:text-emerald-400 font-medium" : "text-gray-700 dark:text-white/70"}`}>{o()[1]}</Text>
                  <View class="flex-1" />
                  <If when={recSort() === o()[0]}>
                    <Text class="text-[12px] text-emerald-600 dark:text-emerald-400">{recSortDesc() ? "↓" : "↑"}</Text>
                  </If>
                </View>
              )}
            </Each>
          </View>
        </Overlay>
      </If>

      {/* ── Start Reconciliation: pick an account (statement details follow on its page) ── */}
      <If when={recStartMenu()}>
        <Overlay anchor={recStartAnchor} placement="bottom" align="end" offset={4} margin={8} onPressOutside={() => recStartMenu.set(false)} onCancel={() => recStartMenu.set(false)}>
          <View class={`w-60 rounded-xl border shadow-xl py-1 ${C.panel} ${C.line}`}>
            <Text class={`px-3 py-1.5 text-[10px] font-semibold uppercase tracking-wider ${C.dim}`}>Start Reconciliation</Text>
            <Each of={d()?.rows || []}>
              {(r: Any) => (
                <View
                  class="flex-row items-center gap-2.5 px-3 py-2 cursor-pointer transition-colors hover:bg-gray-50 dark:hover:bg-white/5"
                  activeStyle={{ opacity: 0.7 }}
                  onPress={() => {
                    recStartMenu.set(false);
                    openCell(r(), oldestOpen(r()));
                  }}
                >
                  <Text class={`text-[12px] text-emerald-600`}>▦</Text>
                  <Text class={`text-[12px] text-gray-700 dark:text-white/70`}>{r().name}</Text>
                </View>
              )}
            </Each>
          </View>
        </Overlay>
      </If>

      {/* ── More actions ── */}
      <If when={recKebabMenu()}>
        <Overlay anchor={recKebabAnchor} placement="bottom" align="end" offset={4} margin={8} onPressOutside={() => recKebabMenu.set(false)} onCancel={() => recKebabMenu.set(false)}>
          <View class={`w-52 rounded-xl border shadow-xl py-1 ${C.panel} ${C.line}`}>
            <View class="flex-row items-center gap-2.5 px-3 py-2 cursor-pointer transition-colors hover:bg-gray-50 dark:hover:bg-white/5" activeStyle={{ opacity: 0.7 }}
              onPress={() => {
                recKebabMenu.set(false);
                toast("Open a month's reconciliation first, then print it.");
              }}
            >
              <Text class="text-[12px] text-gray-700 dark:text-white/70">⤓</Text>
              <Text class="text-[12px] text-gray-700 dark:text-white/70">Download PDF...</Text>
            </View>
          </View>
        </Overlay>
      </If>
    </View>
  );
}

/** One group card ("Assets" / "Liabilities") with its expandable account rows. */
function ReconcileGroup(p: { name: string; entries: Any[]; onCell: (r: Any, c: Any) => void }) {
  return (
    <View class={`rounded-xl border overflow-hidden ${C.panel} ${C.line}`}>
      <View class="flex-row items-center gap-2.5 px-4 py-2.5 border-b bg-[#f8fafc] dark:bg-white/[0.02] border-[#e2e8f0] dark:border-white/10">
        <View class="w-7 h-7 rounded-lg bg-emerald-500/10 dark:bg-emerald-500/15 items-center justify-center">
          <Text class="text-[13px] text-emerald-500">▦</Text>
        </View>
        <Text class="text-[13px] font-semibold text-gray-700 dark:text-white/80">{p.name}</Text>
        <View class="px-1.5 py-0.5 rounded-full bg-[#e2e8f0] dark:bg-white/10">
          <Text class="text-[10px] font-medium text-[#64748b] dark:text-white/50">{String(p.entries.length)}</Text>
        </View>
      </View>
      <Each of={p.entries}>{(r: Any) => <ReconcileAccount row={r()} onCell={p.onCell} />}</Each>
    </View>
  );
}

/** An account row + its 12 month rows when expanded. */
function ReconcileAccount(p: { row: Any; onCell: (r: Any, c: Any) => void }) {
  const r = () => p.row;
  const open = () => recExpanded().includes(String(r().ticket));
  const fin = () => (r().cells || []).filter((c: Any) => c.state === "done").length;
  const prog = () => (r().cells || []).filter((c: Any) => c.state === "draft").length;
  const active = () => (r().cells || []).filter((c: Any) => c.state !== "none").length;
  return (
    <View>
      <View
        class="flex-row items-center gap-3 px-4 py-3 border-b border-[#e2e8f0] dark:border-white/10 cursor-pointer transition-colors hover:bg-[#f8fafc] dark:hover:bg-white/[0.02]"
        activeStyle={{ opacity: 0.7 }}
        onPress={() => recExpanded.set(open() ? recExpanded().filter((x) => x !== String(r().ticket)) : [...recExpanded(), String(r().ticket)])}
      >
        <Text class={`text-[12px] font-bold text-gray-400 dark:text-white/30 ${open() ? "" : "-rotate-0"}`}>{open() ? "▾" : "▸"}</Text>
        <View class="flex-row items-center gap-2.5 flex-1 min-w-0">
          <View class="w-7 h-7 rounded-lg bg-emerald-500/10 dark:bg-emerald-500/15 items-center justify-center shrink-0">
            <Text class="text-[12px] text-emerald-500">▭</Text>
          </View>
          <Text class="text-[13px] font-semibold text-gray-800 dark:text-white truncate">
            {r().name}
          </Text>
        </View>
        <View class="flex-row items-center gap-2 shrink-0">
          <If when={fin() > 0}>
            <View class="flex-row items-center gap-1 px-2 py-0.5 rounded-full bg-emerald-50 dark:bg-emerald-500/10">
              <View class="w-1.5 h-1.5 rounded-full bg-emerald-500" />
              <Text class="text-[10px] font-medium text-emerald-600 dark:text-emerald-400">{`${fin()} Finalized`}</Text>
            </View>
          </If>
          <If when={prog() > 0}>
            <View class="flex-row items-center gap-1 px-2 py-0.5 rounded-full bg-blue-50 dark:bg-blue-500/10">
              <View class="w-1.5 h-1.5 rounded-full bg-blue-500" />
              <Text class="text-[10px] font-medium text-blue-600 dark:text-blue-400">{`${prog()} In Progress`}</Text>
            </View>
          </If>
          <Text class="text-[10px] text-gray-400 dark:text-white/30">{`${active()} of ${(r().cells || []).length} months`}</Text>
        </View>
      </View>
      <If when={open()}>
        <View class="bg-[#fafbfc] dark:bg-white/[0.01]">
          <Each of={r().cells} key={rowKey}>
            {(c: Any) => {
              const st = () => REC_STATE[String(c().state)] || REC_STATE.none;
              const empty = () => c().state === "none" || c().state === "todo";
              return (
                <View class="flex-row items-center gap-3 px-4 py-2.5 pl-14 border-b border-[#f0f3f6] dark:border-white/5 cursor-pointer transition-colors hover:bg-white dark:hover:bg-white/[0.03]">
                  <If when={empty()}>
                    <Text class="w-3 text-center text-[12px] text-gray-300 dark:text-white/20">◌</Text>
                  </If>
                  <If when={!empty()}>
                    <View class={`w-3 h-3 rounded-full shrink-0 ${st().dot}`} />
                  </If>
                  <View class="w-[100px] shrink-0">
                    <Text class="text-[13px] font-medium text-gray-700 dark:text-white/80">{`${REC_MONTHS[Number(String(c().month).slice(5, 7)) - 1] || ""} ${String(c().month).slice(0, 4)}`}</Text>
                  </View>
                  <View class="flex-1">
                    <View class={`self-start px-2 py-0.5 rounded-full ${st().pill}`}>
                      <Text class={`text-[11px] font-medium capitalize ${st().pill}`}>{st().label}</Text>
                    </View>
                  </View>
                  <View class="shrink-0">
                    <If when={!empty()}>
                      <View
                        class="flex-row items-center gap-1 px-2.5 py-1 rounded-lg bg-emerald-500 cursor-pointer transition-colors hover:bg-emerald-600"
                        activeStyle={{ opacity: 0.7 }}
                        onPress={() => p.onCell(r(), c())}
                      >
                        <Text class="text-[10px] font-medium text-white">{c().state === "done" ? "◉ View" : "✎ Continue"}</Text>
                      </View>
                    </If>
                    <If when={empty()}>
                      <View
                        class="flex-row items-center gap-1 px-2.5 py-1 rounded-lg bg-gray-100 dark:bg-white/5 cursor-pointer transition-colors hover:bg-gray-200 dark:hover:bg-white/10"
                        activeStyle={{ opacity: 0.7 }}
                        onPress={() => p.onCell(r(), c())}
                      >
                        <Text class="text-[10px] font-medium text-gray-500 dark:text-white/50">+ Start</Text>
                      </View>
                    </If>
                  </View>
                </View>
              );
            }}
          </Each>
        </View>
      </If>
    </View>
  );
}

const MARK: Record<string, string> = { m: "✓", s: "≈", x: "◷" };

/** One glass card of the green summary strip (old app's SummaryCard). */
function RecSumCard(p: { title: string; glyph: string; highlight?: string; rows: [string, string][] }) {
  return (
    <View class="flex-1 min-w-[140px] rounded-xl px-4 py-3 gap-1.5" style={{ backgroundColor: "rgba(255,255,255,0.10)", borderWidth: 1, borderColor: "rgba(255,255,255,0.10)" }}>
      <View class="flex-row items-center justify-between">
        <View class="flex-row items-center gap-2">
          <View class="w-5 h-5 rounded-full bg-white/20 items-center justify-center">
            <Text class="text-[11px] font-bold text-white">{p.glyph}</Text>
          </View>
          <Text class="text-[13px] font-semibold text-white">{p.title}</Text>
        </View>
        <Text class="text-[13px] font-bold text-white">{p.highlight || ""}</Text>
      </View>
      <Each of={[0, 1]}>
        {(i: Any) => (
          <View class="flex-row items-center justify-between">
            <View class="flex-row items-center gap-2">
              <Text class="text-[11px] text-white/60">{p.rows[i()][0] === "Bank" ? "▤" : "▦"}</Text>
              <Text class="text-[12px] font-medium text-white/60">{p.rows[i()][0]}</Text>
            </View>
            <Text class="text-[13px] font-medium text-white">{p.rows[i()][1]}</Text>
          </View>
        )}
      </Each>
    </View>
  );
}

/** The funnel's status menu: the same three states the counters toggle. */
function RecFilterMenu() {
  const item = (key: string, label: string) => (
    <View
      class={`flex-row items-center gap-2 px-3 py-2 ${recStat() === key ? C.hover : ""}`}
      hoverStyle={{ backgroundColor: "rgba(148,163,184,0.15)" }}
      activeStyle={{ opacity: 0.7 }}
      onPress={() => {
        recStat.set(key);
        recFiltMenu.set(false);
      }}
    >
      <Text class={`w-[16px] text-[12px] ${recStat() === key ? C.green : "text-transparent"}`}>✓</Text>
      <Text class={`text-[12.5px] ${C.ink}`}>{label}</Text>
    </View>
  );
  return (
    <Overlay anchor={recFiltAnchor} placement="bottom" align="end" offset={2} margin={8} open={recFiltMenu()} onPressOutside={() => recFiltMenu.set(false)} onCancel={() => recFiltMenu.set(false)}>
      <View class={`w-[170px] rounded-lg ${C.panel} border ${C.line} py-1`}>
        <Text class={`px-3 pt-1.5 pb-1 text-[10px] font-semibold uppercase tracking-wider ${C.dim}`}>Status</Text>
        {item("all", "All lines")}
        {item("m", "Matched")}
        {item("x", "Unsettled")}
      </View>
    </Overlay>
  );
}

function MatchView() {
  const data = resource(
    () => ({ v: ver(), a: recAccount(), m: recMonth() }),
    (k: Any) => buwiz.call("books_statement_get", { account: k.a, month: k.m }),
  );
  const d = () => data() as Any;
  const s = () => d()?.statement as Any;
  const call = (tool: string, extra: Record<string, Any> = {}) =>
    act(tool, { account: recAccount(), month: recMonth(), ...extra }).catch(() => null);
  const selectedLine = signal<number | null>(null);
  const snapPage = signal(0);
  const pick = (r: Any) => {
    selectedLine.set(r.no ?? null);
    if (r.page) snapPage.set(Number(r.page));
  };
  // the snapshot on the left and the lines on the right; with none, the full-width list
  const side = () => snapShown(s());
  // ── old-app detail page: light header, green summary strip, search bar, tinted rows ──
  const q = () => recQ().trim().toLowerCase();
  const rowsAll = () => (s()?.rows || []) as Any[];
  const matched = () => rowsAll().filter((r: Any) => r.match === "m").length;
  const unsettled = () => rowsAll().filter((r: Any) => r.match !== "m").length;
  const rowsView = () =>
    rowsAll().filter((r: Any) => {
      if (recStat() === "m" && r.match !== "m") return false;
      if (recStat() === "x" && r.match === "m") return false;
      if (q() && ![r.description, r.books?.title, r.books?.sub, r.why].some((t: Any) => String(t || "").toLowerCase().includes(q()))) return false;
      return true;
    });
  // summary strip sums, in centavos (Bank = the statement's own totals, Ledger = the matched entries)
  const sumOf = (f: (r: Any) => boolean, pick: (r: Any) => Any) =>
    rowsAll().filter(f).reduce((a: bigint, r: Any) => a + centsOf(String(pick(r) || "0")), 0n);
  const depBank = () => sumOf((r) => centsOf(r.amount_shown) > 0n, (r) => r.amount_shown);
  const wdBank = () => sumOf((r) => centsOf(r.amount_shown) < 0n, (r) => r.amount_shown);
  const depLeg = () => String(s()?.books_in_shown || "—");
  const wdLeg = () => String(s()?.books_out_shown || "—");
  const isCard = /card|credit/i.test(String(d()?.account?.name || ""));
  const hdrBtnCls = "flex-row items-center gap-1.5 px-3 py-1.5 rounded-lg";
  return (
    <View class="gap-0">
      {/* light header: account · period, status pill, actions — no Back (the window's × closes) */}
      <View class={`flex-row items-center gap-2 px-4 py-2 rounded-t-xl ${C.panel} border-b ${C.line}`}>
        <Text class={`text-[13px] font-semibold ${C.ink}`}>{d()?.account?.name || ""}</Text>
        <Text class={`text-[12px] ${C.mute}`}>{`· ${d()?.month?.label || ""}`}</Text>
        <If when={Boolean(s())}>
          <View class={`rounded-full px-2 py-0.5 ${s()?.status === "finished" ? "bg-emerald-50 dark:bg-emerald-500/10" : "bg-amber-50 dark:bg-amber-500/10"}`}>
            <Text class={`text-[11px] font-medium ${s()?.status === "finished" ? "text-emerald-600 dark:text-emerald-400" : "text-amber-600 dark:text-amber-400"}`}>{s()?.status === "finished" ? "Finalized" : "In Progress"}</Text>
          </View>
        </If>
        <View class="flex-1" />
        <If when={Boolean(s())}>
          <View class={`${hdrBtnCls} bg-gray-100 dark:bg-white/5`} hoverStyle={{ backgroundColor: "rgba(148,163,184,0.25)" }} activeStyle={{ opacity: 0.7 }} onPress={() => call("books_app_statement_accept")}>
            <Text class={`text-[12px] font-medium ${C.mute}`}>{`Accept ${String(s()?.suggested || 0)} suggestion${s()?.suggested === 1 ? "" : "s"}`}</Text>
          </View>
        </If>
        <If when={s()?.status !== "finished"}>
          <View class={`${hdrBtnCls} bg-gray-100 dark:bg-white/5`} hoverStyle={{ backgroundColor: "rgba(148,163,184,0.25)" }} activeStyle={{ opacity: 0.7 }} onPress={() => importStatement(recAccount())}>
            <Text class={`text-[12px] font-medium ${C.mute}`}>Import statement</Text>
          </View>
        </If>
      </View>
      <Loading res={data} />
      <If when={Boolean(d()?.draft)}>
        <DraftBanner account={recAccount()} month={recMonth()} draft={d()?.draft} />
      </If>
      <If when={Boolean(keptFor()) && keptFor() === recAccount() && !d()?.draft}>
        <Panel pad>
          <Text class={`text-[13px] font-semibold ${C.ink}`}>The statement is kept in Books documents.</Text>
          <Text class={`text-[12.5px] ${C.mute}`}>Books does not read it. Ask the assistant to read the statement for this account; its lines then show here for you to accept.</Text>
        </Panel>
      </If>
      <If when={Boolean(d()) && !s()}>
        <Panel pad>
          <Text class={`text-[13px] ${C.mute}`}>No statement for this month yet. Import the bank's CSV to match it, or its PDF or picture for the assistant to read.</Text>
        </Panel>
      </If>
      <If when={Boolean(s())}>
        <View class="gap-3.5">
          <If when={(s()?.flags || []).length > 0}>
            <FlagsNote flags={s()?.flags || []} />
          </If>
          {/* green summary strip: inflow / outflow / Net glass cards (old app's SummaryRow) */}
          <View class="flex-row gap-3 rounded-xl px-4 py-3" style={{ backgroundImage: "linear-gradient(to right,#1a6b3c,#27ae60)" }}>
            <RecSumCard title={isCard ? "Charges" : "Deposits"} glyph="↓" rows={[["Bank", centsShown(depBank(), true)], ["Ledger", depLeg()]]} />
            <RecSumCard title={isCard ? "Payments" : "Withdrawals"} glyph="↑" rows={[["Bank", centsShown(wdBank())], ["Ledger", wdLeg()]]} />
            <RecSumCard title="Net" glyph="≈" highlight={s()?.difference_shown} rows={[["Bank", String(s()?.closing_shown || "—")], ["Ledger", String(s()?.books_shown || "—")]]} />
          </View>
          <View class={side() ? "flex-row items-start gap-3.5" : "gap-3.5"}>
            <If when={side()}>
              <SnapshotPane
                rows={s()?.rows || []}
                evidence={Boolean(s()?.snapshot)}
                kind={String(s()?.snapshot_kind || "")}
                pictures={s()?.snapshot_pictures || []}
                doc={String(s()?.evidence || "")}
                stamp={String(s()?.snapshot_stamp || "")}
                selected={selectedLine()}
                onSelect={(no: number) => selectedLine.set(no)}
                page={snapPage()}
                onPage={(n: number) => snapPage.set(n)}
                account={recAccount()}
                month={recMonth()}
              />
            </If>
            <Panel class={side() ? "flex-1" : ""}>
              {/* search / filter bar inside the card (old app's ReconciliationSearchBar): magnifier, borderless input, counters, funnel, + */}
              <View class={`flex-row items-center gap-2 px-4 py-2.5 border-b ${C.line}`}>
                <Text class={`text-[13px] ${C.dim}`}>⌕</Text>
                <View class="flex-1">
                  <Input
                    accessibilityLabel="Filter transactions"
                    value={recQ()}
                    placeholder="Filter transactions…"
                    onChange={(e: Any) => recQ.set(typeof e === "string" ? e : e.value)}
                    class={`h-7 text-[13px] bg-transparent border-none outline-none placeholder:text-[#8e8e93] focus:ring-0 ${C.ink}`}
                  />
                </View>
                <View
                  class={`flex-row items-center gap-1.5 px-2 py-0.5 rounded-sm bg-[#e6f4ea] dark:bg-emerald-500/10 border cursor-pointer transition-colors ${recStat() === "m" ? "border-emerald-500" : "border-transparent hover:border-emerald-300 dark:hover:border-emerald-600"}`}
                  activeStyle={{ opacity: 0.7 }}
                  onPress={() => recStat.set(recStat() === "m" ? "all" : "m")}
                >
                  <Text class="text-[12px] font-semibold text-[#1e8e3e] dark:text-emerald-400">{String(matched())}</Text>
                  <Text class="text-[11px] font-bold text-[#1e8e3e] dark:text-emerald-400">✓</Text>
                </View>
                <View
                  class={`flex-row items-center gap-1.5 px-2 py-0.5 rounded-sm bg-[#fef7e0] dark:bg-amber-500/10 border cursor-pointer transition-colors ${recStat() === "x" ? "border-amber-500" : "border-transparent hover:border-amber-300 dark:hover:border-amber-600"}`}
                  activeStyle={{ opacity: 0.7 }}
                  onPress={() => recStat.set(recStat() === "x" ? "all" : "x")}
                >
                  <Text class="text-[12px] font-semibold text-[#f9ab00] dark:text-amber-400">{String(unsettled())}</Text>
                  <Text class="text-[11px] font-bold text-[#f9ab00] dark:text-amber-400">◷</Text>
                </View>
                <View
                  ref={recFiltAnchor}
                  class={`w-8 h-6 rounded-sm items-center justify-center ${recStat() !== "all" || recQ() ? "bg-[#ccece8] dark:bg-teal-900/40" : ""}`}
                  hoverStyle={{ backgroundColor: "rgba(148,163,184,0.2)" }}
                  activeStyle={{ opacity: 0.7 }}
                  onPress={() => recFiltMenu.set(!recFiltMenu())}
                >
                  <Text class={`text-[12px] ${recStat() !== "all" || recQ() ? "text-teal-600 dark:text-teal-400" : C.dim}`}>⏷</Text>
                </View>
                <View
                  class="w-8 h-6 rounded-sm items-center justify-center"
                  hoverStyle={{ backgroundColor: "rgba(148,163,184,0.2)" }}
                  activeStyle={{ opacity: 0.7 }}
                  onPress={() => importStatement(recAccount())}
                >
                  <Text class={`text-[14px] font-bold ${C.dim}`}>+</Text>
                </View>
              </View>
              <If when={!side()}>
                <View class={`flex-row border-b ${C.line}`}>
                  <Text class={`flex-1 px-3 py-2.5 text-[11px] uppercase tracking-wider ${C.dim}`}>Bank statement</Text>
                  <View class="w-[28px]" />
                  <Text class={`flex-1 px-3 py-2.5 text-[11px] uppercase tracking-wider ${C.dim}`}>Books</Text>
                </View>
              </If>
              <If when={side()}>
                <Text class={`px-3 py-2.5 text-[11px] uppercase tracking-wider border-b ${C.line} ${C.dim}`}>Statement lines and Books entries</Text>
              </If>
              <Each of={rowsView()} key={rowKey}>
                {(r: Any) => (
                  <View
                    class={`${side() ? "" : "flex-row items-center"} border-b cursor-pointer transition-colors ${
                      r().no === selectedLine()
                        ? "bg-[#eff6ff] dark:bg-blue-950/60 border-[#dbeafe] dark:border-blue-900/40"
                        : r().match === "m"
                          ? "bg-emerald-50/40 dark:bg-emerald-950/10 border-emerald-100/50 dark:border-emerald-900/20 hover:bg-emerald-50/70 dark:hover:bg-emerald-950/25"
                          : "bg-orange-50/40 dark:bg-orange-950/10 border-orange-100/50 dark:border-orange-900/20 hover:bg-orange-50/70 dark:hover:bg-orange-950/25"
                    }`}
                    onPress={() => pick(r())}
                  >
                    {/* left status bar: blue selected, emerald matched, orange unsettled */}
                    <View class={`w-[3px] self-stretch ${r().no === selectedLine() ? "bg-blue-500" : r().match === "m" ? "bg-emerald-400" : "bg-orange-400"}`} />
                    {/* date block: "Oct 15" over the year, like the old row */}
                    <View class="w-[58px] items-center py-2 shrink-0">
                      <Text class={`text-[12.5px] font-semibold ${C.ink}`}>{r().date_short}</Text>
                      <Text class={`text-[10px] ${C.dim}`}>{String(d()?.month?.key || "").slice(0, 4)}</Text>
                    </View>
                    <View class={`${side() ? "" : "flex-1"} flex-row items-center gap-2 px-2 py-2 min-h-[38px]`}>
                      <If when={side()}>
                        <Text class={`w-[18px] text-center font-bold ${r().match === "m" ? C.green : r().match === "s" ? C.amber : C.dim}`}>{MARK[r().match]}</Text>
                      </If>
                      <If when={Boolean(r().flag)}>
                        <Text class="text-[10px] text-rose-600 dark:text-rose-400">⚑</Text>
                      </If>
                      <Text class={`flex-1 text-[12.5px] ${C.ink} truncate`}>
                        {r().description}
                      </Text>
                      <If when={Boolean(r().low)}>
                        <Pill tone="draft" label={`read ${String(r().confidence)}%`} />
                      </If>
                      <Text class={`text-[12.5px] font-mono font-semibold ${String(r().amount_shown || "").startsWith("+") ? "text-[#16a34a] dark:text-emerald-400" : C.ink}`}>{r().amount_shown}</Text>
                    </View>
                    <If when={!side()}>
                      <Text class={`w-[28px] text-center font-bold ${r().match === "m" ? C.green : r().match === "s" ? C.amber : C.dim}`}>{MARK[r().match]}</Text>
                    </If>
                    <View class={`${side() ? "" : "flex-1"} px-3 py-2 gap-1.5 ${r().match === "x" ? PILL.rev : r().match === "s" ? PILL.draft : ""}`} onPress={() => {
                      // a press here does not reach the row, so it picks the line too
                      pick(r());
                      if (r().books?.ticket) drawerEntry.set(r().books.ticket);
                    }}>
                      <View class="flex-row items-center gap-2 min-h-[22px]">
                        <Text class={`w-[52px] text-[12px] font-mono ${C.ink}`}>{r().books?.date_short || "—"}</Text>
                        <View class="flex-1">
                          <Text class={`text-[12.5px] font-semibold ${C.ink}`}>{r().books?.title || ""}</Text>
                          <Text class={`text-[11px] ${C.dim}`}>{r().why || r().books?.sub || ""}</Text>
                          <If when={Boolean(r().hint?.category)}>
                            <Text class={`text-[11px] ${C.dim}`}>{`Usually ${r().hint?.category || ""}`}</Text>
                          </If>
                        </View>
                        <If when={r().match === "x"}>
                          <Btn label="+ Add" small onPress={() => call("books_app_statement_add_line", { line: r().ticket })} />
                        </If>
                        <If when={r().match === "s" && s()?.status !== "finished"}>
                          <Btn label="Match" small onPress={() => call("books_app_statement_accept", { line: r().no })} />
                        </If>
                        <If when={r().match !== "x"}>
                          <Text class={`text-[12.5px] font-mono ${C.ink}`}>{r().books?.amount_shown || ""}</Text>
                        </If>
                      </View>
                      <If when={(r().books?.lines || []).length > 0}>
                        <View class={`border-t ${C.line} pt-1.5 gap-0.5`}>
                          <View class="flex-row gap-2">
                            <Text class={`flex-1 text-[10px] uppercase tracking-wider ${C.dim}`}>Account</Text>
                            <Text class={`w-[84px] text-right text-[10px] uppercase tracking-wider ${C.dim}`}>Debit</Text>
                            <Text class={`w-[84px] text-right text-[10px] uppercase tracking-wider ${C.dim}`}>Credit</Text>
                          </View>
                          <Each of={r().books?.lines || []}>
                            {(l: Any) => (
                              <View class="flex-row gap-2">
                                <Text class={`flex-1 text-[11.5px] ${C.mute} truncate`}>
                                  {l().account || "—"}
                                </Text>
                                <Text class={`w-[84px] text-right text-[11.5px] font-mono ${C.ink}`}>{l().debit_shown}</Text>
                                <Text class={`w-[84px] text-right text-[11.5px] font-mono ${C.ink}`}>{l().credit_shown}</Text>
                              </View>
                            )}
                          </Each>
                        </View>
                      </If>
                    </View>
                  </View>
                )}
              </Each>
            </Panel>
          </View>
          <If when={!side()}>
            <PrintReconciliation account={recAccount()} month={recMonth()} />
          </If>
          <Panel pad class="flex-row items-center gap-4">
            <Text class={`text-[12.5px] ${C.mute}`}>{`Statement ending ${s()?.closing_shown || ""}`}</Text>
            <Text class={`text-[12.5px] ${C.mute}`}>{`Books ${s()?.books_shown || ""}`}</Text>
            <View class="flex-1" />
            <Text class={`text-[12.5px] font-mono font-bold ${s()?.difference === "0.00" ? C.green : C.ink}`}>{`Difference ${s()?.difference_shown || ""}`}</Text>
            <If when={s()?.status !== "finished"}>
              <Btn label={`Finish ${d()?.month?.short || ""}`} tone="go" disabled={!s()?.can_finish} onPress={() => call("books_app_statement_finish").then(() => recMonth.set(""))} />
            </If>
            <If when={s()?.status === "finished"}>
              <Pill tone="ok" label="Matched" />
            </If>
          </Panel>
          <If when={Boolean(s()?.reopenable)}>
            <ReopenFooter onReopen={(reason: string) => call("books_app_statement_finish", { reopen: true, reason })} />
          </If>
        </View>
      </If>
      <If when={recFiltMenu()}>
        <RecFilterMenu />
      </If>
    </View>
  );
}

function ReconcilePage(p: { go: (page: string, filter?: string) => void }) {
  return (
    <View>
      <If when={!recMonth()}>
        <ReconcileGrid />
      </If>
      <If when={Boolean(recMonth())}>
        <MatchView />
      </If>
    </View>
  );
}
