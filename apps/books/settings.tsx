// Settings: Book (name, basis, fiscal year), Months (close and reopen with
// a reason), Defaults (the category mappings, shorter; accrual-only rows
// dim on a cash book), Link & data (drawing: Books Mini App › Settings).

const setTab = signal("book");
const SET_TABS: [string, string][] = [
  ["book", "Book"],
  ["months", "Months"],
  ["defaults", "Defaults"],
  ["link", "Link & data"],
];

function SettingsPage(p: { go: (page: string, filter?: string) => void }) {
  const data = resource(
    () => ({ v: ver() }),
    () => rox.call("books_settings_get", {}),
  );
  const d = () => data() as Any;
  const name = signal("");
  const reason = signal("");
  const reopening = signal("");
  const err = signal("");
  const save = (args: Record<string, Any>) => {
    err.set("");
    act("books_app_book_update", args)
      .then(() => toast("Saved"))
      .catch((e: Any) => err.set(errText(e)));
  };
  const close = (m: string) => act("books_app_month_close", { month: m }).catch((e: Any) => err.set(errText(e)));
  const reopen = (m: string) => {
    if (!reason().trim()) return err.set("Say why the month reopens.");
    act("books_app_month_reopen", { month: m, reason: reason().trim() })
      .then(() => {
        reopening.set("");
        reason.set("");
      })
      .catch((e: Any) => err.set(errText(e)));
  };
  const Opt = (q: { id: string; title: string; sub: string }) => (
    <View
      class={`flex-1 rounded-[10px] border p-3 gap-1 ${C.bg} ${d()?.book.basis === q.id ? "border-[#157a48] dark:border-[#3fcf89]" : C.line}`}
      onPress={() => (d()?.book.basis === q.id ? null : save({ basis: q.id }))}
    >
      <Text class={`text-[13.5px] font-bold ${C.ink}`}>{q.title}</Text>
      <Text class={`text-[12px] ${C.mute}`}>{q.sub}</Text>
    </View>
  );
  return (
    <View class="gap-3.5">
      <Head title="Settings" />
      <Loading res={data} />
      <If when={Boolean(d())}>
        <View class="flex-row gap-3 items-start">
          <Panel class="w-[180px] p-1.5 gap-0.5">
            <Each of={SET_TABS}>
              {(t: Any) => (
                <View class={`rounded-[7px] px-2.5 py-1.5 ${setTab() === t()[0] ? C.raise : ""}`} onPress={() => {
                    err.set("");
                    setTab.set(t()[0]);
                  }}>
                  <Text class={`text-[13px] ${setTab() === t()[0] ? `font-semibold ${C.ink}` : C.mute}`}>{t()[1]}</Text>
                </View>
              )}
            </Each>
          </Panel>
          <View class="flex-1 gap-3">
            <If when={Boolean(err())}>
              <Text class={`text-[12.5px] ${C.red}`}>{err()}</Text>
            </If>
            <If when={setTab() === "book"}>
              <Panel pad class="gap-3.5">
                <View class="flex-row gap-3">
                  <Field label="Name shown in Rox" grow>
                    <View class="flex-row gap-2">
                      <View class="flex-1">
                        <TextBox value={name()} onChange={name.set} placeholder={d()?.book.label || ""} />
                      </View>
                      <Btn label="Rename" disabled={!name().trim()} onPress={() => save({ label: name().trim() })} />
                    </View>
                  </Field>
                  <Field label="Fiscal year starts" grow>
                    <Text class={`text-[13px] py-1.5 ${C.ink}`}>{d()?.book.fiscal_label || ""}</Text>
                  </Field>
                </View>
                <Field label="Accounting basis">
                  <View class="flex-row gap-2">
                    <Opt id="cash" title="Cash (default)" sub="Money counts when it moves. Simplest." />
                    <Opt id="accrual" title="Accrual" sub="Counts at invoice and bill. Adds receivables, payables, aging." />
                  </View>
                </Field>
                <Text class={`text-[12px] ${C.dim}`}>The basis changes only from the start of a year with nothing posted yet. Every BIR hand-off records it.</Text>
              </Panel>
            </If>
            <If when={setTab() === "months"}>
              <View class="gap-3">
                <View class="flex-row gap-2 flex-wrap">
                  <Each of={d()?.months || []} key={rowKey}>
                    {(m: Any) => (
                      <View class={`rounded-[10px] p-2.5 gap-1.5 w-[150px] ${C.raise}`}>
                        <Text class={`text-[13px] font-bold ${C.ink}`}>{m().short}</Text>
                        <Pill tone={m().closed ? "ok" : "draft"} label={m().closed ? "Closed" : "Open"} />
                        <If when={!m().closed}>
                          <Btn label="Close" tone="go" small onPress={() => close(m().key)} />
                        </If>
                        <If when={m().closed}>
                          <Btn label="Reopen…" small onPress={() => reopening.set(m().key)} />
                        </If>
                      </View>
                    )}
                  </Each>
                </View>
                <If when={Boolean(reopening())}>
                  <Panel pad class="flex-row gap-2 items-end">
                    <Field label={`Why reopen ${reopening()}?`} grow>
                      <TextBox value={reason()} onChange={reason.set} placeholder="A late bank fee" />
                    </Field>
                    <Btn label="Reopen" tone="go" onPress={() => reopen(reopening())} />
                  </Panel>
                </If>
                <Text class={`text-[12px] ${C.dim}`}>A closed month cannot change. Reopening asks for a reason and marks BIR hand-offs from that month as out of date.</Text>
                <If when={(d()?.events || []).length > 0}>
                  <Panel>
                    <Each of={d()?.events || []} key={rowKey}>
                      {(e: Any) => (
                        <View class={`flex-row gap-3 px-3 py-2 border-b ${C.line}`}>
                          <Text class={`w-[130px] text-[12.5px] ${C.ink}`}>{e().month}</Text>
                          <Pill tone={e().event === "close" ? "ok" : "draft"} label={e().event === "close" ? "Closed" : "Reopened"} />
                          <Text class={`flex-1 text-[12.5px] ${C.mute}`} numberOfLines={1}>
                            {e().reason || ""}
                          </Text>
                        </View>
                      )}
                    </Each>
                  </Panel>
                </If>
              </View>
            </If>
            <If when={setTab() === "defaults"}>
              <View class="gap-2.5">
                <Each of={d()?.defaults || []} key={rowKey}>
                  {(g: Any) => (
                    <Panel>
                      <View class="flex-row items-center justify-between px-3.5 pt-3 pb-1">
                        <Text class={`text-[13px] font-bold ${C.ink}`}>{g().group}</Text>
                        <Text class={`text-[12px] ${C.dim}`}>used when nothing else says</Text>
                      </View>
                      <Each of={g().rows}>
                        {(r: Any) => (
                          <View class={`flex-row items-center gap-2 px-3 py-2 border-t ${C.line} ${r().off ? "opacity-50" : ""}`}>
                            <View class="w-[170px]">
                              <Text class={`text-[13px] ${C.ink}`}>{r().label}</Text>
                              <If when={r().off}>
                                <Text class={`text-[11px] ${C.dim}`}>accrual books only</Text>
                              </If>
                            </View>
                            <Text class={C.dim}>→</Text>
                            <View class="flex-1">
                              <Pick
                                value={r().account?.ticket || ""}
                                options={d()?.accounts || []}
                                onPick={(a: Any) => (r().off ? null : act("books_app_default_set", { slot: r().slot, account: a.ticket }).catch(() => null))}
                              />
                            </View>
                            <View class={`w-[72px] rounded-[5px] px-1.5 py-0.5 ${TAG[r().account?.tag] || PILL.mute}`}>
                              <Text class={`text-[10px] font-bold uppercase text-center ${TAG[r().account?.tag] || PILL.mute}`}>{r().account?.tag === "re" ? "Revenue" : r().account?.tag === "as" ? "Asset" : r().account?.tag === "li" ? "Liability" : r().account?.tag === "eq" ? "Equity" : "Expense"}</Text>
                            </View>
                          </View>
                        )}
                      </Each>
                    </Panel>
                  )}
                </Each>
              </View>
            </If>
            <If when={setTab() === "link"}>
              <Panel pad class="gap-3">
                <Kv k="Tax profile" v={d()?.link.tax_profile} />
                <Kv k="BIR app" v={d()?.link.bir} />
                <Kv k="Stored where" v={d()?.link.stored} />
                <Text class={`text-[12px] ${C.dim}`}>No TIN is stored here. A tax profile links by its ID only, through Buwiz sign-in.</Text>
              </Panel>
            </If>
          </View>
        </View>
      </If>
    </View>
  );
}
