// Category detail page: the Books port of the old web app's
// /accounts/category/<id> (accounts_/category/$categoryId.tsx). Reached from
// the Category Manager's rightmost row button; ‹ Back returns to the manager.
// Shows only what Books tools return: the account's kind siblings stand in for
// subcategories, the month's ledger rows for transactions, and no
// comments/people/sharing (the old app's tabs need data Books doesn't store).

/** Ticket of the account the Category page shows. */
const catDetailTicket = signal("");

function CategoryDetailPage(p: { go: (page: string, filter?: string) => void }) {
  const list = resource(
    () => ({ v: ver() }),
    () => buwiz.call("books_accounts_list", { group: "all" }),
  );
  const rows = () => ((list() as Any)?.rows || []) as Any[];
  const acct = () => rows().find((a: Any) => String(a.ticket) === catDetailTicket()) || null;
  // Siblings: the same-kind group this account belongs to (the tree's children).
  const siblings = () => {
    const a = acct();
    if (!a || a.kind === "unsure" || a.kind === "mixed") return [] as Any[];
    return rows().filter((x: Any) => x.kind === a.kind && String(x.ticket) !== String(a.ticket));
  };
  // Empty = the book's current month; the response's month.{key,label,prev,next}
  // drives the ‹ › nav so the seeded BOOKS_TODAY stays the anchor.
  const month = signal("");
  const monthLabel = () =>
    String((ledger() as Any)?.month?.label || "…");
  const shiftMonth = (d: number) => {
    const m = (ledger() as Any)?.month;
    const next = d < 0 ? m?.prev : m?.next;
    if (next) month.set(String(next));
  };
  const ledger = resource(
    () => ({ v: ver(), a: catDetailTicket(), m: month() }),
    (k: Any) => {
      if (!k.a) return Promise.resolve(null);
      const args: Any = { account: k.a, filter: "posted" };
      if (k.m) args.month = k.m;
      return buwiz.call("books_ledger_list", args);
    },
  );
  const tx = () => ((ledger() as Any)?.rows || []) as Any[];
  // The account's own month movement comes back computed; summing the page
  // would count whole entries and stop at the row cap.
  const monthShown = () => String((ledger() as Any)?.account?.month_shown || "");
  const totalCount = () => Number((ledger() as Any)?.counts?.posted ?? tx().length);
  const countLabel = () =>
    totalCount() > tx().length
      ? `first ${tx().length} of ${totalCount()} transactions`
      : `${tx().length} transaction${tx().length === 1 ? "" : "s"}`;

  const a = acct;
  const t = () => (a() ? acctType(String(a()?.type)) : ACCT_TYPES[0]);
  const icon = () => (a() ? ACCT_ICON_PATHS[ACCT_KIND_ICON[String(a()?.kind || "unsure")]] || ACCT_TYPE_ICONS[String(a()?.type)] || ACCT_TYPE_ICONS.asset : ACCT_TYPE_ICONS.asset);
  const tint = () => `tint-[${t()[2]}] dark:tint-[${t()[4]}]`;
  const detailRow = (label: string, body: Any) => (
    <View class="gap-1">
      <Text class="text-[12px] font-medium text-[#64748b] dark:text-slate-400">{label}</Text>
      {body}
    </View>
  );

  return (
    <View class="flex-1 flex-col overflow-hidden">
      {/* Top nav — ‹ Back to Categories */}
      <View class="shrink-0 flex-row items-center px-3 sm:px-6 py-2 sm:py-3">
        <View class="flex-row items-center gap-1.5 py-1.5 pr-2" onPress={() => p.go("accounts")} activeStyle={{ opacity: 0.7 }}>
          <Text class={`text-[14px] font-medium transition-colors ${C.ink} hover:text-[#6366f1]`}>‹</Text>
          <Text class={`text-[14px] font-medium transition-colors ${C.ink} hover:text-[#6366f1]`}>Back to Categories</Text>
        </View>
      </View>
      {/* Keep the flex skeleton unconditional — an <If>-wrapped root breaks the
          flex height chain and the page collapses to content size. */}
      <View class="flex-1 min-h-0 flex-row gap-6 px-3 pb-3 sm:px-6 sm:pb-6 overflow-hidden">
          {/* Main column */}
          <View class={`flex-1 min-h-0 rounded-[16px] border overflow-hidden flex-col bg-white dark:bg-slate-800 ${C.line}`}>
            <If when={!a()}>
              <View class="flex-1 items-center justify-center gap-2">
                <Loading res={list} />
                <If when={Boolean((list() as Any)?.rows)}>
                  <Text class={`text-[14px] ${C.mute}`}>Category not found.</Text>
                </If>
              </View>
            </If>
            <If when={Boolean(a())}>
              <View class="flex-1 min-h-0 flex-col">
            {/* Header: type icon + name + number, like the detail card header */}
            <View class="shrink-0 bg-[#2a9d8f] px-5 py-4 flex-row items-center gap-3">
              <View class="w-10 h-10 rounded-lg bg-white/15 items-center justify-center">
                <Avatar src={icon()} class="w-6 h-6" imageClass="tint-white" fallbackDelay={Infinity} accessibilityLabel="" />
              </View>
              <View class="flex-1 min-w-0">
                <Text class="text-[16px] font-bold text-white truncate">{a()?.name || ""}</Text>
                <Text class="text-[12px] text-white/80 font-mono">{a()?.number || ""}</Text>
              </View>
              <Text class="text-[14px] font-semibold text-white font-mono">{a()?.balance_shown || ""}</Text>
            </View>
            {/* Transactions: month nav + rows (ledger_list) */}
            <View class={`shrink-0 flex-row items-center justify-between px-5 py-3 border-b ${C.line}`}>
                <View class="flex-row items-center gap-2">
                  <Btn label="‹" small onPress={() => shiftMonth(-1)} />
                  <Text class={`text-[14px] font-bold ${C.ink}`}>{monthLabel()}</Text>
                  <Btn label="›" small onPress={() => shiftMonth(1)} />
                </View>
                <View class="items-end">
                  <Text class={`text-[13px] font-semibold ${C.ink}`}>{monthShown()}</Text>
                  <Text class={`text-[11px] ${C.dim}`}>{tx().length > 0 ? countLabel() : ""}</Text>
                </View>
            </View>
            <Scroll class="flex-1 min-h-0">
              <Loading res={ledger} />
              <Each of={tx()} key={rowKey}>
                {(r: Any) => (
                  <View
                    class={`flex-row items-center gap-3 px-5 py-2.5 border-b border-[#f0f1f3] dark:border-slate-700 transition-colors hover:bg-[#f8f9fb] dark:hover:bg-slate-800`}
                    onPress={() => openLedger({ account: catDetailTicket(), month: month() || String((ledger() as Any)?.month?.key || ""), entry: r().ticket })}
                    activeStyle={{ opacity: 0.75 }}
                  >
                    <Text class={`w-[48px] text-[12px] ${C.dim}`}>{r().date_short || ""}</Text>
                    <View class="flex-1 min-w-0">
                      <Text class={`text-[13px] truncate ${C.ink}`}>{r().title || r().category || ""}</Text>
                    </View>
                    <Text class={`text-[13px] font-mono ${(r().account_tone || r().tone) === "pos" ? "text-emerald-600 dark:text-emerald-400" : (r().account_tone || r().tone) === "neg" ? C.ink : C.mute}`}>
                      {r().account_amount_shown || r().amount_shown || ""}
                    </Text>
                  </View>
                )}
              </Each>
              <If when={Boolean((ledger() as Any)) && tx().length === 0}>
                <View class="items-center py-12">
                  <Text class={`text-[13px] ${C.mute}`}>No transactions found for this category during the selected period.</Text>
                </View>
              </If>
            </Scroll>
              </View>
            </If>
          </View>

          {/* Right sidebar — the reference's Details tab */}
          <View class={`hidden lg:flex w-[340px] min-w-[340px] max-w-[520px] rounded-[16px] border overflow-hidden flex-col bg-white dark:bg-slate-800 ${C.line}`}>
            <View class="shrink-0 bg-gradient-to-r from-[#4338ca] to-[#6366f1] px-5 py-3 flex-row justify-center">
              <View class="h-11 px-6 items-center justify-center rounded-full bg-white/20 flex-row gap-2">
                <Avatar src={ACCT_ICONS.doc} class="w-4 h-4" imageClass="tint-white" fallbackDelay={Infinity} accessibilityLabel="" />
                <Text class="text-[13px] font-semibold text-white">Details</Text>
              </View>
            </View>
            <Scroll class="flex-1 min-h-0">
              <View class="p-5 items-center gap-4">
                <View class="w-16 h-16 rounded-full items-center justify-center" style={{ backgroundColor: t()[3] }}>
                  <Avatar src={icon()} class="w-8 h-8" imageClass={tint()} fallbackDelay={Infinity} accessibilityLabel="" />
                </View>
                <Text class={`text-[15px] font-semibold text-center ${C.ink}`}>{a()?.name || ""}</Text>
                <View class="w-full gap-4">
                  {detailRow(
                    "Type",
                    <Text class={`text-[13px] capitalize ${C.ink}`}>{a() ? String(a()?.type).replace("_", " ") : "—"}</Text>,
                  )}
                  {detailRow(
                    "Account Number",
                    <View class={`rounded px-2 py-1 self-start bg-slate-50 dark:bg-slate-800`}>
                      <Text class={`text-[13px] font-mono ${C.ink}`}>{a()?.number || "—"}</Text>
                    </View>,
                  )}
                  {detailRow(
                    "Sub-type",
                    <Text class={`text-[13px] ${C.ink}`}>{a() ? acctKindLabel(String(a()?.kind || "unsure")) : "—"}</Text>,
                  )}
                  {detailRow(
                    "Status",
                    <View class="flex-row items-center gap-2">
                      <View class={`w-2 h-2 rounded-full ${a() && !a()?.off ? "bg-emerald-500" : "bg-slate-300"}`} />
                      <Text class={`text-[13px] ${C.ink}`}>{a() && !a()?.off ? "Active" : "Inactive"}</Text>
                    </View>,
                  )}
                  {detailRow(
                    "Current Balance",
                    <Text class={`text-[15px] font-semibold font-mono ${C.ink}`}>{a()?.balance_shown || "—"}</Text>,
                  )}
                </View>
                {/* Sub-categories: same-kind siblings */}
                <If when={siblings().length > 0}>
                  <View class="w-full gap-1 pt-2">
                    <Text class="text-[12px] font-medium text-[#64748b] dark:text-slate-400">Sub-categories</Text>
                    <Each of={siblings()} key={rowKey}>
                      {(s: Any) => (
                        <View
                          class={`w-full flex-row items-center gap-2 py-2 px-3 rounded-lg transition-colors hover:bg-[#f1f5f9] dark:hover:bg-slate-800`}
                          onPress={() => catDetailTicket.set(String(s().ticket))}
                          activeStyle={{ opacity: 0.75 }}
                        >
                          <View class="w-1.5 h-1.5 rounded-full bg-[#cbd5e1] dark:bg-slate-600 shrink-0" />
                          <Text class={`flex-1 text-[13px] truncate ${C.ink}`}>{s().name}</Text>
                          <Text class={`text-[12px] font-mono ${C.dim}`}>{s().balance_shown || ""}</Text>
                          <Text class="text-[13px] text-[#cbd5e1]">›</Text>
                        </View>
                      )}
                    </Each>
                  </View>
                </If>
              </View>
            </Scroll>
          </View>
        </View>
    </View>
  );
}
