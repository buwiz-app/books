// Reconciliation detail: the centre's dual lists (statement lines beside the
// journal's entries), their search/filter bar, the pending-edits bar and the
// 4-tab right sidebar (Summary, Flag, Comments, Activity) — the old web app's
// reconciliations_.$reconciliationId.tsx centre and ReconciliationSidebar.

/** The old app's 12-colour initials palette (LedgerTransactionsList / transactions). */
const REC_PALETTE = [
  "#ef4444",
  "#f97316",
  "#f59e0b",
  "#84cc16",
  "#22c55e",
  "#14b8a6",
  "#06b6d4",
  "#3b82f6",
  "#6366f1",
  "#a855f7",
  "#ec4899",
  "#f43f5e",
];

/** Stable initials + colour from a party/description, like the old avatar hash. */
function recAvatar(name: string): { initials: string; color: string } {
  const n = String(name || "?").trim() || "?";
  const initials =
    n
      .split(/\s+/)
      .slice(0, 2)
      .map((w) => (w[0] || "").toUpperCase())
      .join("") || "?";
  let hash = 0;
  for (let i = 0; i < n.length; i++) hash = n.charCodeAt(i) + ((hash << 5) - hash);
  return { initials, color: REC_PALETTE[Math.abs(hash) % REC_PALETTE.length] };
}

function RecAvatar(p: Any) {
  const a = () => recAvatar(p.name);
  return (
    <View class="w-7 h-7 rounded-full items-center justify-center shrink-0" style={{ backgroundColor: a().color }}>
      <Text class="text-[10px] font-bold text-white">{a().initials}</Text>
    </View>
  );
}

/** Row tint: the old app's emerald matched / orange unsettled hover families. */
function recRowTint(match: string, hovered: boolean, selected: boolean): string {
  if (selected) return "bg-[#eff6ff] dark:bg-blue-950/60 border-[#dbeafe] dark:border-blue-900/40";
  if (match === "m")
    return hovered
      ? "bg-emerald-50/80 dark:bg-emerald-950/30 border-emerald-100 dark:border-emerald-900/40"
      : "bg-emerald-50/40 dark:bg-emerald-950/10 border-emerald-100/50 dark:border-emerald-900/20 hover:bg-emerald-50/70 dark:hover:bg-emerald-950/25";
  return hovered
    ? "bg-orange-50/80 dark:bg-orange-950/30 border-orange-100 dark:border-orange-900/40"
    : "bg-orange-50/40 dark:bg-orange-950/10 border-orange-100/50 dark:border-orange-900/20 hover:bg-orange-50/70 dark:hover:bg-orange-950/25";
}

function recBarTint(match: string, hovered: boolean, selected: boolean): string {
  if (selected) return "bg-blue-500";
  if (hovered) return match === "m" ? "bg-emerald-400" : "bg-orange-400";
  return "bg-transparent";
}

const R_ICN = "items-center justify-center";
const AS = { opacity: 0.7 };
const HS2 = { backgroundColor: "rgba(148,163,184,0.2)" };
const HS3 = { backgroundColor: "rgba(148,163,184,0.25)" };
const BL = `border-b ${C.line}`;
const FR = "flex-row items-center gap-2";
const rowPill = (bg: string) => `w-6 h-6 rounded-full ${bg} ${R_ICN}`;
const rStop = (e: Any, fn?: () => void) => {
  e?.stopPropagation?.();
  fn?.();
};
const unmatchBtn = (on: () => void) => (
  <View
    class={`w-8 h-8 rounded-full bg-orange-500/15 ${R_ICN} cursor-pointer hover:bg-orange-500/25`}
    onPress={(e: Any) => rStop(e, on)}
  >
    <Text class="text-[13px] font-bold text-orange-500">⤫</Text>
  </View>
);
const viewBtn = (on?: () => void) => (
  <View class="cursor-pointer" onPress={(e: Any) => rStop(e, on)}>
    <Text class="text-[13px] text-gray-400 hover:text-gray-600 dark:hover:text-gray-300">›</Text>
  </View>
);

/** One row of either centre list — statement line or journal entry (l=true). */
function RecRow(p: Any) {
  const m = () => String(p.r.match || (p.r.matched ? "m" : "x"));
  const hi = () => p.hovered || Boolean(p.linked);
  const flags = () => (p.r.flags || []) as Any[];
  const av = () => String(p.r.avatar || p.r.party || p.r.books?.title || p.r.name || p.r.description || "");
  const ttl = () => p.r.title || p.r.description || p.r.name || "";
  const why = () => p.r.why || (p.r.hint?.category ? `Usually ${p.r.hint.category}` : "");
  const conf = () => p.r.conf ?? p.r.confidence;
  const amt = () => (p.l ? `${p.r.credit ? "+" : ""}${p.r.amount}` : String(p.r.amount_shown ?? p.r.amount ?? ""));
  const crd = () => Boolean(p.r.credit) || String(p.r.amount_shown || "").startsWith("+");
  return (
    <View
      class={`group flex-row items-stretch border-b cursor-pointer transition-colors ${recRowTint(m(), hi(), Boolean(p.selected))} ${p.recon && p.l ? "bg-cyan-50/50 dark:bg-cyan-950/30" : ""}`}
      onLayout={p.onLayout}
      onMouseEnter={p.onHover}
      onMouseLeave={p.onLeave}
      onPress={p.onPick}
    >
      <View class={`w-[4px] self-stretch ${p.recon && p.l ? "bg-cyan-500" : recBarTint(m(), hi(), Boolean(p.selected))}`} />
      <View class="w-[58px] items-center py-2.5 shrink-0">
        <Text class={`text-[12.5px] font-semibold leading-tight ${C.ink}`}>{p.r.date_short}</Text>
        <Text class={`${RT10D}`}>{p.r.year || ""}</Text>
      </View>
      <View class="flex-1 flex-row items-center gap-2.5 px-2 py-2.5 min-w-0">
        <RecAvatar name={av()} />
        <View class={RF1M}>
          <View class="flex-row items-center gap-1.5">
            <Text class={`flex-1 text-[13px] font-medium ${C.ink} truncate`}>{ttl()}</Text>
            <If when={flags().length > 0}>
              <View class="flex-row items-center gap-0.5 px-1.5 rounded-sm bg-rose-500/10">
                <Text class={RT10R}>⚑</Text>
                <If when={flags().length > 1}>
                  <Text class="text-[10px] font-semibold text-rose-600 dark:text-rose-400">{String(flags().length)}</Text>
                </If>
              </View>
            </If>
          </View>
          <View class="flex-row items-center gap-2 mt-0.5">
            <If when={Boolean(why())}>
              <Text class={`text-[10px] ${C.dim} truncate`}>{why()}</Text>
            </If>
            <If when={Boolean(conf())}>
              <Text class="text-[10px] text-[#94a3b8] dark:text-slate-500">{`${conf()}% conf`}</Text>
            </If>
            <If when={Boolean(p.r.num)}>
              <Text class={`text-[10px] font-mono ${C.dim} truncate`}>{`#${p.r.num}`}</Text>
            </If>
            <If when={Boolean(p.r.source)}>
              <View class="px-1.5 rounded-sm bg-gray-100 dark:bg-white/5">
                <Text class={`${RT10D}`}>{p.r.source}</Text>
              </View>
            </If>
          </View>
        </View>
      </View>
      <View class="w-[88px] items-end justify-center px-2 py-2.5">
        <Text class={`text-[13px] font-semibold font-mono whitespace-nowrap ${crd() ? "text-[#16a34a] dark:text-emerald-400" : C.ink}`}>
          {crd() && !String(amt()).startsWith("+") ? `+${amt()}` : String(amt() || "—")}
        </Text>
      </View>
      <View class={`w-[40px] ${R_ICN}`}>
        <If when={m() === "m" && !p.editMode}>
          <View class={rowPill("bg-emerald-500/15")}>
            <Text class="text-[11px] font-bold text-emerald-600 dark:text-emerald-400">✓</Text>
          </View>
        </If>
        <If when={m() === "m" && p.editMode && !p.finalized}>{unmatchBtn(p.onUnmatch)}</If>
        <If when={m() === "s" && !p.editMode}>
          <View class={rowPill("bg-amber-500/10")}>
            <Text class={`text-[11px] font-bold ${C.amber}`}>≈</Text>
          </View>
        </If>
        <If when={m() === "x" && !p.editMode}>
          <View class={rowPill("bg-amber-500/10")}>
            <Text class="text-[11px] font-bold text-amber-600 dark:text-amber-400">◷</Text>
          </View>
        </If>
        <If when={m() !== "m" && p.editMode && !p.l && !p.finalized}>
          <View class={`w-5 h-5 rounded-full border border-dashed ${R_ICN} ${p.recon ? "border-emerald-500 bg-emerald-500/10" : "border-emerald-400/50 dark:border-emerald-400/30"}`}>
            <Text class={`text-[10px] font-bold ${p.recon ? "text-emerald-600 dark:text-emerald-400" : "text-emerald-500/50"}`}>+</Text>
          </View>
        </If>
        <If when={m() !== "m" && p.editMode && p.l && !p.finalized}>
          <View
            class={`w-8 h-8 rounded-full ${R_ICN} cursor-pointer transition-colors ${p.recon ? "bg-emerald-500" : "bg-emerald-500/10 hover:bg-emerald-500/25"}`}
            onPress={(e: Any) => rStop(e, p.onRecon)}
          >
            <Text class={`text-[12px] font-bold ${p.recon ? "text-white" : "text-emerald-600 dark:text-emerald-400"}`}>⇄</Text>
          </View>
        </If>
      </View>
      <View class={`w-[36px] ${R_ICN} opacity-0 group-hover:opacity-100 transition-opacity`}>
        <If when={!p.l && m() === "x" && !p.finalized && !p.editMode}>
          <View class={`w-5 h-5 rounded-full bg-emerald-500 ${R_ICN} cursor-pointer`} onPress={(e: Any) => rStop(e, p.onQuickAdd)}>
            <Text class="text-[10px] font-bold text-white">+</Text>
          </View>
        </If>
        <If when={!p.l && m() === "s" && !p.finalized && !p.editMode}>
          <View class={`w-5 h-5 rounded-full bg-emerald-500/15 ${R_ICN} cursor-pointer`} onPress={(e: Any) => rStop(e, p.onQuickMatch)}>
            <Text class="text-[10px] font-bold text-emerald-600 dark:text-emerald-400">✓</Text>
          </View>
        </If>
        <If when={p.l || m() === "m" || p.finalized || p.editMode}>{viewBtn(p.onView)}</If>
      </View>
    </View>
  );
}

/** Sticky "N changes pending" bar above the lists, while edit-mode ops queue. */
function RecEditBar(p: Any) {
  return (
    <View class={`${FR} px-4 py-2 ${BL} bg-[#eff6ff] dark:bg-blue-950/40`}>
      <Text class="text-[12px] font-semibold text-blue-700 dark:text-blue-300">{`${p.count} change${p.count === 1 ? "" : "s"} pending`}</Text>
      <View class="flex-1" />
      <View class="px-3 py-1 rounded-md bg-gray-100 dark:bg-white/10 cursor-pointer" activeStyle={AS} onPress={p.onCancel}>
        <Text class={`text-[11.5px] font-medium ${C.mute}`}>Cancel</Text>
      </View>
      <View
        class="px-3 py-1 rounded-md bg-blue-600 cursor-pointer"
        activeStyle={{ opacity: p.applying ? 1 : 0.7 }}
        onPress={() => {
          if (!p.applying) p.onApply();
        }}
      >
        <Text class="text-[11.5px] font-semibold text-white">{p.applying ? "Applying…" : "Apply"}</Text>
      </View>
    </View>
  );
}

/** The cyan banner while a ledger entry waits for its statement line. */
function RecReconBanner(p: Any) {
  return (
    <View class="flex-row items-center gap-2 px-4 py-2 border-b border-cyan-200 dark:border-cyan-800/50 bg-cyan-50 dark:bg-cyan-950/40">
      <Text class="text-[12px] font-semibold text-cyan-700 dark:text-cyan-300">⇄</Text>
      <Text class={`flex-1 text-[12px] font-medium text-cyan-800 dark:text-cyan-200 truncate`}>{`Pick the statement line for ${p.name}`}</Text>
      <View class="px-2.5 py-1 rounded-md bg-cyan-100 dark:bg-cyan-900/40 cursor-pointer" activeStyle={AS} onPress={p.onCancel}>
        <Text class="text-[11.5px] font-semibold text-cyan-700 dark:text-cyan-300">Cancel</Text>
      </View>
    </View>
  );
}

/** "N matched" / "N unsettled" counter pills that also set the status filter. */
function RecCounter(p: Any) {
  const bg = p.tone === "m" ? "bg-[#e6f4ea] dark:bg-emerald-500/10" : "bg-[#fef7e0] dark:bg-amber-500/10";
  const ink = p.tone === "m" ? "text-[#1e8e3e] dark:text-emerald-400" : "text-[#f9ab00] dark:text-amber-400";
  const edge = p.tone === "m" ? "border-emerald-500" : "border-amber-500";
  const hov = p.tone === "m" ? "hover:border-emerald-300 dark:hover:border-emerald-600" : "hover:border-amber-300 dark:hover:border-amber-600";
  return (
    <View
      class={`flex-row items-center gap-1.5 px-2 py-0.5 rounded-sm ${bg} border cursor-pointer transition-colors ${p.active ? edge : `border-transparent ${hov}`}`}
      activeStyle={AS}
      onPress={p.onPress}
    >
      <Text class={`text-[12px] font-semibold ${ink}`}>{String(p.n)}</Text>
      <Text class={`text-[11px] font-bold ${ink}`}>{p.icon}</Text>
    </View>
  );
}

const SIDE_TABS: [string, string, string][] = [
  ["summary", "◈", "Summary"],
  ["flag", "⚑", "Flag"],
  ["comments", "✎", "Comments"],
  ["activity", "≡", "Activity"],
];

const sideEmpty = (glyph: string, title: string, sub: string, bg = "bg-gray-100 dark:bg-white/10") => (
  <View class="items-center py-10 px-4">
    <View class={`w-12 h-12 rounded-full ${bg} ${R_ICN} mb-3`}>
      <Text class={`text-[15px] ${C.dim}`}>{glyph}</Text>
    </View>
    <Text class={`text-[13px] font-semibold ${C.ink}`}>{title}</Text>
    <Text class={`text-[11.5px] ${C.dim} text-center`}>{sub}</Text>
  </View>
);

/** The 4-tab right sidebar (Summary / Flag / Comments / Activity). */
function RecSide(p: Any) {
  const tab = signal("summary");
  const comment = signal("");
  const badge = () => (p.flags || []).filter((f: Any) => !f.resolved).length + p.unmatched;
  const comments = () => (p.s?.comments || []) as Any[];
  const activity = () => (p.s?.activity || []) as Any[];
  const sh = (k: string) => String(p.s?.[k] ?? "—");
  const sendComment = () => {
    const text = comment().trim();
    if (!text) return;
    act("books_app_statement_action", { account: recAccount(), month: recMonth(), action: "comment", body: text })
      .then(() => {
        comment.set("");
        ver.set(ver() + 1);
      })
      .catch(() => toast("Comments are not available yet"));
  };
  const tabBtn = (key: string, glyph: string, label: string) => (
    <View
      class={`flex-1 items-center py-2 border-b-2 cursor-pointer ${tab() === key ? "border-blue-500" : "border-transparent"}`}
      activeStyle={AS}
      onPress={() => tab.set(key)}
    >
      <View class="flex-row items-center gap-1">
        <Text class={`text-[11px] ${tab() === key ? "text-blue-600 dark:text-blue-400" : C.dim}`}>{glyph}</Text>
        <Text class={`text-[11px] font-medium ${tab() === key ? "text-blue-600 dark:text-blue-400" : C.dim}`}>{label}</Text>
        <If when={key === "flag" && badge() > 0}>
          <View class="px-1 rounded-full bg-rose-500">
            <Text class="text-[9px] font-bold text-white">{String(badge())}</Text>
          </View>
        </If>
      </View>
    </View>
  );
  const fact = (label: string, value: string, mono = false, tone = "") => (
    <View class="flex-row items-center gap-2 px-3.5 py-1.5">
      <Text class={`flex-1 text-[12px] ${C.dim}`}>{label}</Text>
      <Text class={`text-[12px] ${mono ? "font-mono" : ""} font-semibold ${tone || C.ink}`}>{value}</Text>
    </View>
  );
  return (
    <View class={`flex-col flex-1 min-h-0`}>
      <View class={`flex-row ${BL}`}>
        <Each of={SIDE_TABS}>{(t: Any) => tabBtn(t()[0], t()[1], t()[2])}</Each>
      </View>
      <Scroll class="flex-1">
        <If when={tab() === "summary"}>
          <View class={RY1}>
            <View class="px-3.5 pt-2.5 pb-1">
              <View class={`self-start rounded-full px-2 py-0.5 ${p.s?.status === "finished" ? "bg-emerald-50 dark:bg-emerald-500/10" : "bg-amber-50 dark:bg-amber-500/10"}`}>
                <Text class={`text-[11px] font-medium ${p.s?.status === "finished" ? "text-emerald-600 dark:text-emerald-400" : "text-amber-600 dark:text-amber-400"}`}>
                  {p.s?.status === "finished" ? "Finalized" : "In Progress"}
                </Text>
              </View>
            </View>
            {fact("Account", String(p.d?.account?.name || ""))}
            {fact("Period", String(p.d?.month?.label || ""))}
            {fact("Statement", sh("closing_shown"), true)}
            {fact("Books", sh(`books_shown`), true)}
            {fact("Difference", sh("difference_shown"), true, p.s?.difference === "0.00" ? C.green : C.ink)}
            {fact(p.isCard ? "Charges" : "Deposits", sh(`books_in_shown`), true)}
            {fact(p.isCard ? "Payments" : "Withdrawals", sh(`books_out_shown`), true)}
            {fact("Matched lines", String(p.s?.matched ?? "—"))}
            {fact("Unsettled lines", String(p.unmatched))}
            <If when={Number(p.s?.suggested || 0) > 0}>
              {fact("Suggestions open", String(p.s?.suggested))}
            </If>
          </View>
        </If>
        <If when={tab() === "flag"}>
          <View class={RY1}>
            <If when={badge() === 0}>{sideEmpty("✓", "Nothing flagged", "Every statement line is settled.", "bg-emerald-500/10")}</If>
            <Each of={p.flags || []}>
              {(f: Any) => (
                <View
                  class={`${FR} px-3.5 py-2 ${BL} ${f().line ? "cursor-pointer" : ""}`}
                  onPress={() => {
                    if (f().line) p.onFlagLine?.(Number(f().line));
                  }}
                >
                  <Text class="text-[11px] text-rose-500">⚑</Text>
                  <Text class={`flex-1 text-[12px] ${f().resolved ? `${C.dim} line-through` : C.ink}`}>{f().line ? `Line ${String(f().line)}: ${f().note}` : f().note}</Text>
                </View>
              )}
            </Each>
            <If when={p.unmatched > 0}>
              <View class="flex-row items-center gap-2 px-3.5 py-2">
                <Text class={`text-[11px] ${C.amber}`}>◷</Text>
                <Text class={`flex-1 text-[12px] ${C.mute}`}>{`${p.unmatched} statement line${p.unmatched === 1 ? "" : "s"} still unsettled`}</Text>
              </View>
            </If>
          </View>
        </If>
        <If when={tab() === "comments"}>
          <View class={RY1}>
            <If when={comments().length === 0}>{sideEmpty("✎", "No comments yet", "Notes about this reconciliation land here.")}</If>
            <Each of={comments()}>
              {(cm: Any) => (
                <View class={`px-3.5 py-2 ${BL}`}>
                  <View class={`${FR}`}>
                    <RecAvatar name={String(cm().by || "You")} />
                    <View class={RF1M}>
                      <Text class={`text-[12px] font-semibold ${C.ink}`}>{cm().by || "You"}</Text>
                      <Text class={`${RT10D}`}>{cm().at}</Text>
                    </View>
                  </View>
                  <Text class={`text-[12px] ${C.mute} mt-1`}>{cm().body || cm().text}</Text>
                </View>
              )}
            </Each>
          </View>
        </If>
        <If when={tab() === "activity"}>
          <View class={RY1}>
            <If when={activity().length === 0}>{sideEmpty("≡", "No activity yet", "Imports, matches and finishes land here.")}</If>
            <Each of={activity()}>
              {(a: Any) => (
                <View class={`flex-row gap-2.5 px-3.5 py-2 ${BL}`}>
                  <View class="w-1.5 h-1.5 rounded-full bg-teal-500 mt-1.5 shrink-0" />
                  <View class={RF1M}>
                    <Text class={`text-[12px] ${C.ink}`}>{a().event || a().text}</Text>
                    <If when={Boolean(a().detail)}>
                      <Text class={`text-[11px] ${C.dim}`}>{a().detail}</Text>
                    </If>
                    <Text class={`${RT10D}`}>{a().at}</Text>
                  </View>
                </View>
              )}
            </Each>
          </View>
        </If>
      </Scroll>
      <If when={tab() === "comments"}>
        <View class={`${FR} px-3 py-2 border-t ${C.line}`}>
          <View class="flex-1">
            <Input
              value={comment()}
              placeholder="Write a comment…"
              onChange={(e: Any) => comment.set(typeof e === "string" ? e : e.value)}
              class={`h-8 text-[12px] bg-transparent border-none outline-none placeholder:text-[#8e8e93] focus:ring-0 ${C.ink}`}
            />
          </View>
          <View class="px-2.5 py-1.5 rounded-md bg-blue-600 cursor-pointer" activeStyle={AS} onPress={sendComment}>
            <Text class="text-[11px] font-semibold text-white">Post</Text>
          </View>
        </View>
      </If>
    </View>
  );
}

/** The advanced-filter popover behind the funnel (status, type, amount, party). */
function RecFilterPop(p: Any) {
  const item = (v: string, label: string, cur: string, onPick: (v: string) => void) => (
    <View
      class={`flex-row items-center gap-2 px-3 py-1.5 rounded-md cursor-pointer ${cur === v ? "bg-teal-50 dark:bg-teal-900/30" : ""}`}
      hoverStyle={{ backgroundColor: "rgba(148,163,184,0.15)" }}
      activeStyle={AS}
      onPress={() => onPick(v)}
    >
      <Text class={`w-3 text-[11px] font-bold ${cur === v ? "text-teal-600 dark:text-teal-400" : "text-transparent"}`}>✓</Text>
      <Text class={`text-[12px] ${cur === v ? `font-semibold ${C.ink}` : C.mute}`}>{label}</Text>
    </View>
  );
  const head = (t: string) => <Text class={`px-3 pt-2.5 pb-1 text-[10px] font-semibold uppercase tracking-wider ${C.dim}`}>{t}</Text>;
  const num = (label: string, v: string, onV: (v: string) => void) => (
    <View class="flex-1">
      <Input
        value={v}
        placeholder={label}
        onChange={(e: Any) => onV(typeof e === "string" ? e : e.value)}
        class={`h-7 px-2 text-[12px] rounded-md border ${C.line} bg-transparent outline-none focus:ring-0 ${C.ink}`}
      />
    </View>
  );
  return (
    <View class={`w-[220px] rounded-lg ${C.panel} border ${C.line} py-1`}>
      {head("Status")}
      {item("all", "All lines", p.stat, p.onStat)}
      {item("m", "Matched", p.stat, p.onStat)}
      {item("x", "Unsettled", p.stat, p.onStat)}
      {item("f", "Flagged", p.stat, p.onStat)}
      {head("Type")}
      {item("all", "All", p.type, p.onType)}
      {item("in", "Money in", p.type, p.onType)}
      {item("out", "Money out", p.type, p.onType)}
      {head("Amount")}
      <View class="flex-row gap-2 px-3 py-1">
        {num("Min", p.min, p.onMin)}
        {num("Max", p.max, p.onMax)}
      </View>
      {head("Party")}
      <View class="px-3 py-1">
        {num("Name…", p.party, p.onParty)}
      </View>
      <View class={`px-3 pt-2 pb-1 mt-1 border-t ${C.line}`}>
        <View
          class="items-center py-1.5 rounded-md bg-gray-100 dark:bg-white/10 cursor-pointer"
          activeStyle={AS}
          onPress={p.onClear}
        >
          <Text class={`text-[11.5px] font-semibold ${C.mute}`}>Clear all</Text>
        </View>
      </View>
    </View>
  );
}
