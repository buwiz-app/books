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

function InboxDetail(p: { ticket: string }) {
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
  return (
    <View class={`px-3.5 py-3 gap-3 border-b ${C.raise} ${C.line}`}>
      <Loading res={data} />
      <If when={Boolean(e())}>
        <View class="flex-row gap-3 items-start">
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
                    <Text class={`flex-1 text-[12px] ${C.ink}`} numberOfLines={1}>
                      {l().account}
                    </Text>
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
  const list = (filter: string) =>
    resource(
      () => ({ v: ver(), m: inboxMonth() }),
      (k: Any) => buwiz.call("books_ledger_list", k.m ? { filter, month: k.m } : { filter }),
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
  return (
    <View class="gap-3.5">
      <Head title="Inbox" sub={waiting() === 1 ? "1 waiting to be booked" : `${String(waiting())} waiting to be booked`}>
        <View class="flex-row items-center gap-2">
          <Btn label="‹" small onPress={() => (month().prev ? inboxMonth.set(month().prev) : null)} />
          <Text class="text-[12.5px] font-semibold text-white">{month().label || ""}</Text>
          <Btn label="›" small onPress={() => (month().next ? inboxMonth.set(month().next) : null)} />
          <Btn label="+ New entry" tone="go" onPress={() => p.sheet("money_out")} />
        </View>
      </Head>
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
      <Loading res={review} />
      <If when={Boolean(rv())}>
        <Panel class="overflow-hidden">
          <If when={rows().length === 0}>
            <Text class={`text-[12.5px] px-3.5 py-3 ${C.dim}`}>Nothing waits in this month. Bank lines, receipts a routine reads from your mail and drafts from chat show up here.</Text>
          </If>
          <Each of={rows()} key={(r: Any) => r.ticket}>
            {(r: Any) => (
              <View>
                <View
                  class={`flex-row items-center gap-3 px-3.5 py-2.5 border-b ${C.line} ${inboxOpen() === r().ticket ? C.raise : ""}`}
                  onPress={() => inboxOpen.set(inboxOpen() === r().ticket ? "" : r().ticket)}
                  activeStyle={{ opacity: 0.7 }}
                >
                  <View class="w-[78px]">
                    <Pill tone={inboxSource(r())[0]} label={inboxSource(r())[1]} />
                  </View>
                  <Text class={`w-[52px] text-[12.5px] font-mono ${C.ink}`}>{r().date_short}</Text>
                  <View class="flex-1 gap-0.5">
                    <Text class={`text-[13px] font-semibold ${C.ink}`} numberOfLines={1}>
                      {r().title}
                    </Text>
                    <ReviewBadges row={r()} />
                  </View>
                  <View class="w-[170px] flex-row items-center gap-1.5">
                    <Dot color={r().category_color} />
                    <Text class={`text-[12.5px] ${C.ink}`} numberOfLines={1}>
                      {r().category || "No category yet"}
                    </Text>
                  </View>
                  <View class="w-[96px]">
                    <Pill tone={r().pill} label={r().status_label} />
                  </View>
                  <Text class={`w-[110px] text-right text-[13px] font-mono ${AMT[r().tone] || C.ink}`}>{r().amount_shown}</Text>
                </View>
                <If when={inboxOpen() === r().ticket}>
                  <InboxDetail ticket={r().ticket} />
                </If>
              </View>
            )}
          </Each>
        </Panel>
      </If>
      <Text class={`text-[12px] ${C.dim}`}>Filled by bank statement imports, routines and drafts from chat. Nothing posts until you accept it.</Text>
    </View>
  );
}
