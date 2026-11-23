// Inbox: one queue of entries waiting to be booked. Bank lines to
// categorize, drafts the assistant or a routine saved (a receipt from
// email, a line said in chat) and the user's own drafts. A row opens to its
// source beside the suggested entry. Nothing posts until a person presses
// Accept here.

const INBOX_FILTERS: [string, string][] = [
  ["all", "All waiting"],
  ["review", "Needs review"],
  ["draft", "Drafts"],
];

const inboxFilter = signal("all");
const inboxMonth = signal("");
const inboxOpen = signal("");

/** Where a waiting entry came from, as the row's first pill. */
function inboxSource(r: Any): [string, string] {
  if (r.proposed_by === "routine") return ["bir", "Routine"];
  if (r.proposed_by) return ["in", "Assistant"];
  if (r.pill === "rev") return ["mute", "Bank"];
  return ["mute", "You"];
}

function InboxDetail(p: { ticket: string; row?: Any }) {
  const data = resource(
    () => ({ v: ver(), e: p.ticket }),
    (k: Any) => buwiz.call("books_entry_get", { entry: k.e }),
  );
  const e = () => data() as Any;
  const busy = signal(false);
  const run = (tool: string, extra: Record<string, Any> = {}) => {
    busy.set(true);
    act(tool, { entry: p.ticket, ...extra })
      .then((r: Any) => {
        if (tool === "books_app_post_entry") toast(`Entry ${r.no} posted`);
        if (tool === "books_app_discard_entry") toast("Draft discarded");
        if (tool !== "books_app_recategorize") inboxOpen.set("");
      })
      .catch(() => null)
      .finally(() => busy.set(false));
  };
  const has = (a: string) => (e()?.actions || []).includes(a);
  const src = () => (p.row ? inboxSource(p.row)[1] : "");
  return (
    <View>
      <Loading res={data} />
      <If when={Boolean(e())}>
        <View class="bg-slate-950 px-6 py-6 border-b border-slate-200 dark:border-slate-800">
          <View class="flex-row items-start justify-between gap-4">
            <View class="min-w-0 flex-1 gap-0.5">
              <Text class="text-[11px] font-semibold uppercase tracking-[0.16em] text-teal-300">{src()}</Text>
              <Text class="mt-2 text-[20px] font-semibold text-white truncate">
                {p.row ? String(p.row.title || "") : ""}
              </Text>
              <Text class="mt-1 text-[13px] text-slate-300">{`${e()?.date_label || ""} · ${e()?.big || ""}`}</Text>
            </View>
            <View class="rounded-full bg-white/10 px-2.5 py-1">
              <Text class="text-[12px] font-medium text-white">{p.row ? String(p.row.status_label || "") : ""}</Text>
            </View>
          </View>
        </View>
        <View class="flex-row gap-3 items-start px-3.5 py-3">
          <Panel pad class="w-[260px] gap-2">
            <Text class={`text-[10.5px] uppercase tracking-wider ${C.dim}`}>Source</Text>
            <If when={!e()?.extraction && (e()?.evidence || []).length === 0}>
              <Text class={`text-[12px] ${C.dim}`}>No document is attached to this entry.</Text>
            </If>
            <ExtractionPanel entry={e()} />
            <View class="gap-1.5">
              <Kv k="Date" v={e()?.date_label} />
              <Kv k={e()?.who_label || "Who"} v={e()?.payee || "—"} />
              <Kv k="Reference" v={e()?.reference || "—"} />
            </View>
          </Panel>
          <Panel pad class="flex-1 gap-2.5">
            <View class="flex-row items-center gap-2">
              <Text class={`flex-1 text-[10.5px] uppercase tracking-wider ${C.dim}`}>{`Suggested entry · ${e()?.no || ""}`}</Text>
              <Text class={`text-[15px] font-semibold font-mono ${AMT[e()?.tone] || C.ink}`}>{e()?.big || ""}</Text>
            </View>
            <View class={`rounded-[10px] border overflow-hidden ${C.line}`}>
              <View class="flex-row gap-1.5 px-2.5 py-1.5">
                <Text class={`flex-1 text-[10.5px] uppercase tracking-wider ${C.dim}`}>Account</Text>
                <Text class={`w-[96px] text-right text-[10.5px] uppercase tracking-wider ${C.dim}`}>Debit</Text>
                <Text class={`w-[96px] text-right text-[10.5px] uppercase tracking-wider ${C.dim}`}>Credit</Text>
              </View>
              <Each of={e()?.lines || []}>
                {(l: Any) => (
                  <View class={`flex-row gap-1.5 px-2.5 py-1.5 border-t ${C.line}`}>
                    <View class="flex-1 min-w-0">
                      <Text class={`text-[12px] ${C.ink} truncate`}>{l().account}</Text>
                      <If when={Boolean(lineAssignees(l()))}>
                        <Text class={`text-[10.5px] ${C.dim} truncate`}>{lineAssignees(l())}</Text>
                      </If>
                    </View>
                    <Text class={`w-[96px] text-right text-[12px] font-mono ${C.ink}`}>{l().debit_shown}</Text>
                    <Text class={`w-[96px] text-right text-[12px] font-mono ${C.ink}`}>{l().credit_shown}</Text>
                  </View>
                )}
              </Each>
            </View>
            <If when={Boolean(e()?.reviewable)}>
              <View class="gap-1.5">
                <Text class={`text-[12px] ${C.dim}`}>Pick a category</Text>
                <View class="flex-row gap-1.5 flex-wrap">
                  <Each of={e()?.suggest || []} key={rowKey}>
                    {(c: Any) => (
                      <View
                        class={`rounded-full border px-2.5 py-1 ${C.raise} ${C.line}`}
                          style={picked(Boolean(c().best))}
                        onPress={() => run("books_app_recategorize", { category: c().ticket })}
                      >
                        <Text class={`text-[12px] ${c().best ? C.green : C.ink}`}>{c().best ? `${c().name} · likely` : c().name}</Text>
                      </View>
                    )}
                  </Each>
                </View>
              </View>
            </If>
            <ReviewPanel entry={e()} busy={busy()} open={(t: string) => drawerEntry.set(t)} keepBoth={() => run("books_app_post_entry", { keep_both: true })} />
            <If when={Boolean(e()?.post_blocker)}>
              <Text class={`text-[12px] ${C.amber}`}>{String(e()?.post_blocker || "")}</Text>
            </If>
            <View class="flex-row gap-2 flex-wrap">
              <If when={has("post")}>
                <Btn label="Accept and post" tone="go" disabled={busy() || Boolean(e()?.post_blocker)} onPress={() => run("books_app_post_entry")} />
              </If>
              <Btn label="Open in full" disabled={busy()} onPress={() => drawerEntry.set(p.ticket)} />
              <View class="flex-1" />
              <If when={has("discard")}>
                <Btn label="Discard" disabled={busy()} onPress={() => run("books_app_discard_entry")} />
              </If>
            </View>
          </Panel>
        </View>
      </If>
    </View>
  );
}

function InboxPage(p: { go: (page: string, filter?: string) => void; sheet: (mode: string) => void }) {
  const query = signal("");
  const list = (filter: string) =>
    resource(
      () => ({ v: ver(), m: inboxMonth(), q: query().trim() }),
      (k: Any) =>
        buwiz.call("books_ledger_list", {
          ...(k.m ? { month: k.m } : {}),
          ...(k.q ? { search: k.q } : {}),
          filter,
        }),
    );
  const review = list("review");
  const drafts = list("draft");
  const rv = () => review() as Any;
  const dr = () => drafts() as Any;
  const rows = computed(() => {
    const a = inboxFilter() === "draft" ? [] : rv()?.rows || [];
    const b = inboxFilter() === "review" ? [] : dr()?.rows || [];
    const seen = new Set<string>();
    return [...a, ...b].filter((r: Any) => (seen.has(r.ticket) ? false : (seen.add(r.ticket), true)));
  });
  const waiting = () => (rv()?.counts?.review || 0) + (rv()?.counts?.draft || 0);
  const month = () => rv()?.month || {};
  // books_ledger_list already matched the search, on more fields than a row carries.
  const visibleRows = () => rows();
  const selectedTicket = () => {
    const cur = inboxOpen();
    const list = visibleRows();
    if (cur && list.some((r: Any) => r.ticket === cur)) return cur;
    return list[0]?.ticket || "";
  };
  return (
    <View class="flex-1 flex-col overflow-hidden">
      {/* Top Navigation + Month + New Entry */}
      <View class="shrink-0 flex-col sm:flex-row sm:items-center justify-between px-4 py-3 sm:px-6 sm:py-3.5 gap-2 sm:gap-3">
        <View class="flex-row items-center gap-3">
          <View
            class="group flex-row items-center gap-1.5 h-11 min-w-11 justify-center rounded-lg transition-colors sm:min-w-0 sm:justify-start"
            onPress={() => p.go("today")}
            activeStyle={{ opacity: 0.7 }}
          >
            <Text class={`text-[14px] font-medium transition-colors ${C.ink} group-hover:text-[#0f766e] dark:group-hover:text-[#2dd4bf]`}>‹ Back</Text>
          </View>
          <Text class={`text-[18px] font-semibold ${C.ink}`}>Inbox</Text>
          <Text class={`text-[12.5px] ${C.mute}`}>
            {waiting() === 1 ? "1 waiting to be booked" : `${String(waiting())} waiting to be booked`}
          </Text>
        </View>
        <View class="flex-row items-center gap-2">
          <Btn label="‹" small onPress={() => (month().prev ? inboxMonth.set(month().prev) : null)} />
          <Text class={`text-[12.5px] font-semibold ${C.ink}`}>{month().label || ""}</Text>
          <Btn label="›" small onPress={() => (month().next ? inboxMonth.set(month().next) : null)} />
          <View
            class="h-10 px-4 rounded-md bg-teal-700 hover:bg-teal-800 transition-colors items-center justify-center"
            onPress={() => p.sheet("money_out")}
            activeStyle={{ opacity: 0.85 }}
          >
            <Text class="text-[13px] font-semibold text-white">+ New entry</Text>
          </View>
        </View>
      </View>

      {/* Split Review Workspace — Queue on left, Source & Suggested Entry on right */}
      <View class="flex-1 min-h-0 flex-row gap-5 px-4 pb-4 sm:px-6 sm:pb-6 overflow-hidden">
        {/* Left Queue Panel */}
        <View class={`${inboxOpen() ? "hidden lg:flex" : "flex"} flex-1 lg:flex-none lg:w-[420px] shrink-0 min-h-0 rounded-[16px] border overflow-hidden flex-col bg-white dark:bg-slate-900 ${C.line}`}>
          <View class={`shrink-0 px-3.5 py-3 gap-2 border-b ${C.line}`}>
            <View class="h-10 flex-row items-center rounded-md border border-slate-200 bg-white pl-3 pr-2 transition-colors focus-within:border-teal-500 focus-within:ring-2 focus-within:ring-teal-500/15 dark:border-slate-700 dark:bg-slate-900">
              <Text class="text-[14px] text-slate-400 dark:text-slate-500">⌕</Text>
              <Input
                accessibilityLabel="Search inbox"
                value={query()}
                placeholder="Search inbox items..."
                onChange={(e: Any) => query.set(typeof e === "string" ? e : e.value)}
                class="flex-1 pl-2 bg-transparent border-none outline-none text-base sm:text-sm text-[#0f172a] dark:text-slate-100 placeholder:text-[#94a3b8]"
              />
            </View>
            <View class="flex-row gap-1.5 flex-wrap">
              <Each of={INBOX_FILTERS}>
                {(f: Any) => (
                  <Chip
                    label={f()[1]}
                    count={f()[0] === "all" ? waiting() : rv()?.counts?.[f()[0]] || 0}
                    on={inboxFilter() === f()[0]}
                    onPress={() => inboxFilter.set(f()[0])}
                  />
                )}
              </Each>
            </View>
          </View>
          <Loading res={review} />
          <If when={Boolean(rv())}>
            <Scroll class="flex-1 min-h-0">
              <If when={visibleRows().length === 0}>
                <View class="min-h-[300px] items-center justify-center p-8 gap-1">
                  <Text class={`text-[26px] ${query().trim() ? C.dim : "text-[#14b8a6]"}`}>{query().trim() ? "⌕" : "✓"}</Text>
                  <Text class={`text-[15px] font-semibold ${C.ink}`}>{query().trim() ? "No matching transactions" : "This queue is clear"}</Text>
                  <Text class={`text-[12.5px] text-center ${C.mute}`}>
                    {query().trim()
                      ? "Try a different transaction, vendor, or source name."
                      : "Bank lines, receipts a routine reads from your mail and drafts from chat show up here."}
                  </Text>
                </View>
              </If>
              <Each of={visibleRows()} key={(r: Any) => r.ticket}>
                {(r: Any) => (
                  <View
                    class={`relative px-5 py-4 border-b gap-1.5 transition-colors ${C.line} ${
                      selectedTicket() === r().ticket
                        ? "bg-teal-50/70 dark:bg-teal-950/30"
                        : "hover:bg-slate-50 dark:hover:bg-slate-800/50"
                    }`}
                    onPress={() => inboxOpen.set(r().ticket)}
                    activeStyle={{ opacity: 0.75 }}
                  >
                    <If when={selectedTicket() === r().ticket}>
                      <View class="absolute left-0 top-0 bottom-0 w-[3px] bg-[#0f766e]" />
                    </If>
                    <View class="flex-row items-center justify-between gap-2">
                      <View class="flex-row items-center gap-1.5">
                        <Pill tone={inboxSource(r())[0]} label={inboxSource(r())[1]} />
                        <Text class={`text-[11.5px] font-mono ${C.dim}`}>{r().date_short}</Text>
                      </View>
                      <Text class={`text-[13px] font-mono font-semibold ${AMT[r().tone] || C.ink}`}>
                        {r().amount_shown}
                      </Text>
                    </View>
                    <Text class={`text-[13.5px] font-semibold ${C.ink} truncate`}>
                      {r().title}
                    </Text>
                    <View class="flex-row items-center justify-between gap-2">
                      <View class="flex-row items-center gap-1.5 flex-1">
                        <Dot color={r().category_color} />
                        <Text class={`text-[12px] ${C.mute} truncate`}>
                          {r().category || "No category yet"}
                        </Text>
                      </View>
                      <Pill tone={r().pill} label={r().status_label} />
                    </View>
                    <ReviewBadges row={r()} />
                  </View>
                )}
              </Each>
            </Scroll>
          </If>
        </View>

        {/* Right Detail & Source Preview Pane */}
        <View class={`${inboxOpen() ? "flex" : "hidden lg:flex"} flex-1 min-h-0 rounded-[16px] border overflow-hidden flex-col bg-white dark:bg-slate-900 ${C.line}`}>
          <View class={`lg:hidden shrink-0 flex-row items-center border-b px-2 py-1 ${C.line}`}>
            <View class="h-9 flex-row items-center gap-1.5 rounded-lg px-2 transition-colors hover:bg-slate-100 dark:hover:bg-slate-800" onPress={() => inboxOpen.set("")} activeStyle={{ opacity: 0.7 }}>
              <Text class={`text-[13px] font-semibold ${C.mute}`}>‹ Back to queue</Text>
            </View>
          </View>
          <If when={Boolean(selectedTicket())}>
            <Scroll class="flex-1 min-h-0">
              <InboxDetail
                ticket={selectedTicket()}
                row={visibleRows().find((r: Any) => r.ticket === selectedTicket())}
              />
            </Scroll>
          </If>
          <If when={!selectedTicket()}>
            <View class="flex-1 items-center justify-center p-8 gap-1">
              <Text class="text-[26px] text-[#14b8a6]">✓</Text>
              <Text class={`text-[15px] font-semibold ${C.ink}`}>Inbox is clear</Text>
              <Text class={`text-[13px] text-center ${C.mute}`}>
                Filled by bank statement imports, routines and drafts from chat. Nothing posts until you accept it.
              </Text>
            </View>
          </If>
        </View>
      </View>
    </View>
  );
}
