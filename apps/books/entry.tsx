// The entry parts of the Books window: the drawer (the double entry behind
// an entry, Post or Void) and the new-entry sheet (Money in, Money out,
// Transfer, Journal). The Ledger itself is a native screen
// (crates/books-screens); `openLedger` opens it. The assistant drafts; only
// a click here posts or voids (DD-20).

/** The open entry's ticket, for the drawer. */
const drawerEntry = signal("");

/** Open the ledger's native screen. Set only the props the caller has. */
function openLedger(props: { account?: string; month?: string; filter?: string }) {
  const args: Record<string, Any> = {};
  if (props.account) args.account = props.account;
  if (props.month) args.month = props.month;
  if (props.filter) args.filter = props.filter;
  buwiz.open({ app: "ledger", props: args }).catch((e: Any) => toast(errText(e)));
}

/** The entry drawer, over the right side of the window. */
function EntryDrawer() {
  const data = resource(
    () => ({ v: ver(), e: drawerEntry() }),
    (k: Any) => (k.e ? buwiz.call("books_entry_get", { entry: k.e }) : Promise.resolve(null)),
  );
  const e = () => data() as Any;
  const busy = signal(false);
  const run = (tool: string, extra: Record<string, Any> = {}) => {
    busy.set(true);
    act(tool, { entry: drawerEntry(), ...extra })
      .then((r: Any) => {
        if (tool === "books_entry_draft") toast(`Draft ${r.no} saved`);
        if (tool === "books_app_post_entry") toast(`Entry ${r.no} posted`);
        if (tool === "books_app_void_entry") toast(`Entry ${r.no} voided; a reversal is posted`);
        if (tool === "books_app_discard_entry") drawerEntry.set("");
      })
      .catch(() => null)
      .finally(() => busy.set(false));
  };
  const has = (a: string) => (e()?.actions || []).includes(a);
  return (
    <View class={`absolute top-0 right-0 bottom-0 w-[340px] border-l ${C.panel} ${C.line}`}>
      <View class={`flex-row items-center gap-2 px-3.5 py-3 border-b ${C.line}`}>
        <Text class={`flex-1 text-[14px] font-bold ${C.ink}`} numberOfLines={1}>
          {e()?.title || "Entry"}
        </Text>
        <If when={Boolean(e())}>
          <Pill tone={e()?.pill} label={e()?.status_label} />
        </If>
        <Btn label="✕" small onPress={() => drawerEntry.set("")} />
      </View>
      <Scroll class="flex-1">
        <View class="p-3.5 gap-3">
          <Loading res={data} />
          <If when={Boolean(e())}>
            <View class="gap-3">
              <Text class={`text-[24px] font-semibold font-mono ${AMT[e()?.tone] || C.ink}`}>{e()?.big || ""}</Text>
              <View class="gap-1.5">
                <Kv k="Date" v={e()?.date_label} />
                <Kv k={e()?.who_label || "Who"} v={e()?.payee || "—"} />
                <Kv k="Category" v={e()?.category || "—"} />
                <Kv k="Reference" v={e()?.reference || "—"} />
                <Kv k="Entry" v={e()?.no} />
                <If when={Boolean(e()?.related)}>
                  <Kv k="Linked" v={e()?.related} />
                </If>
              </View>
              <ReviewPanel entry={e()} busy={busy()} open={(t: string) => drawerEntry.set(t)} keepBoth={() => run("books_app_post_entry", { keep_both: true })} />
              <If when={Boolean(e()?.reviewable)}>
                <View class="gap-1.5">
                  <Text class={`text-[12px] ${C.dim}`}>Pick a category</Text>
                  <View class="flex-row gap-1.5 flex-wrap">
                    <Each of={e()?.suggest || []} key={rowKey}>
                      {(c: Any) => (
                        <View
                          class={`rounded-full border px-2.5 py-1 ${C.raise} ${c().best ? "border-[#157a48] dark:border-[#3fcf89]" : C.line}`}
                          onPress={() =>
                            act("books_app_recategorize", { entry: drawerEntry(), category: c().ticket })
                              .then(() => toast(`Now ${c().name}`))
                              .catch(() => null)
                          }
                        >
                          <Text class={`text-[12px] ${c().best ? C.green : C.ink}`}>{c().best ? `${c().name} · likely` : c().name}</Text>
                        </View>
                      )}
                    </Each>
                  </View>
                </View>
              </If>
              <View class="gap-1.5">
                <Text class={`text-[12px] ${C.dim}`}>Behind the scenes</Text>
                <View class={`rounded-[10px] border overflow-hidden ${C.line}`}>
                  <View class="flex-row gap-1.5 px-2.5 py-1.5">
                    <Text class={`flex-1 text-[10.5px] uppercase tracking-wider ${C.dim}`}>Account</Text>
                    <Text class={`w-[84px] text-right text-[10.5px] uppercase tracking-wider ${C.dim}`}>Debit</Text>
                    <Text class={`w-[84px] text-right text-[10.5px] uppercase tracking-wider ${C.dim}`}>Credit</Text>
                  </View>
                  <Each of={e()?.lines || []}>
                    {(l: Any) => (
                      <View class={`flex-row gap-1.5 px-2.5 py-1.5 border-t ${C.line}`}>
                        <Text class={`flex-1 text-[12px] ${C.ink}`} numberOfLines={1}>
                          {l().account}
                        </Text>
                        <Text class={`w-[84px] text-right text-[12px] font-mono ${C.ink}`}>{l().debit_shown}</Text>
                        <Text class={`w-[84px] text-right text-[12px] font-mono ${C.ink}`}>{l().credit_shown}</Text>
                      </View>
                    )}
                  </Each>
                </View>
              </View>
              <If when={Boolean(e()?.memo)}>
                <View class="gap-1">
                  <Text class={`text-[12px] ${C.dim}`}>Memo</Text>
                  <Text class={`text-[12.5px] ${C.ink}`}>{e()?.memo || ""}</Text>
                </View>
              </If>
              <If when={Boolean(e()?.post_blocker)}>
                <Text class={`text-[12px] ${C.red}`}>{e()?.post_blocker || ""}</Text>
              </If>
            </View>
          </If>
        </View>
      </Scroll>
      <View class={`flex-row gap-2 flex-wrap px-3.5 py-3 border-t ${C.line}`}>
        <If when={has("void")}>
          <Btn label="Void (adds a reversal)" disabled={busy()} onPress={() => run("books_app_void_entry")} />
        </If>
        <If when={has("copy")}>
          <Btn
            label="Copy"
            disabled={busy()}
            onPress={() =>
              act("books_entry_draft", { copy_of: drawerEntry() })
                .then((r: Any) => {
                  drawerEntry.set(r.entry);
                  toast(`Draft ${r.no} saved`);
                })
                .catch(() => null)
            }
          />
        </If>
        <If when={has("discard")}>
          <Btn label="Discard" disabled={busy()} onPress={() => run("books_app_discard_entry")} />
        </If>
        <If when={has("post")}>
          <Btn label="Post" tone="go" disabled={busy() || Boolean(e()?.post_blocker)} onPress={() => run("books_app_post_entry")} />
        </If>
      </View>
    </View>
  );
}

function Kv(p: { k: string; v: Any }) {
  return (
    <View class="flex-row gap-2.5">
      <Text class={`w-[96px] text-[12.5px] ${C.dim}`}>{p.k}</Text>
      <Text class={`flex-1 text-[12.5px] ${C.ink}`} numberOfLines={2}>
        {String(p.v ?? "—")}
      </Text>
    </View>
  );
}

const MODES: [string, string][] = [
  ["money_in", "Money in"],
  ["money_out", "Money out"],
  ["transfer", "Transfer"],
  ["journal", "Journal"],
];

/** The new-entry sheet over the window. `mode` is the open mode, "" closed. */
function EntrySheet(p: { mode: Any; close: () => void; setMode: (m: string) => void }) {
  const opts = resource(
    () => ({ v: ver() }),
    () => buwiz.call("books_entry_options", {}),
  );
  const o = () => opts() as Any;
  const amount = signal("");
  const date = signal("");
  const money = signal("");
  const toAccount = signal("");
  const category = signal("");
  const payee = signal("");
  const memo = signal("");
  const reference = signal("");
  const jl = signal<Any[]>([
    { account: "", debit: "", credit: "" },
    { account: "", debit: "", credit: "" },
  ]);
  const busy = signal(false);
  const err = signal("");
  const mode = () => String(p.mode());
  const amt = computed(() => moneyText(amount()));
  const moneyName = computed(() => ((o()?.money || []).find((a: Any) => a.ticket === money()) || {}).label || "the bank");
  const catName = computed(() => (((mode() === "money_in" ? o()?.income : o()?.expense) || []).find((a: Any) => a.ticket === category()) || {}).label || "the category");
  const behind = computed(() => {
    const a = amt() ? grouped(amt() as string) : "0.00";
    if (mode() === "money_in") return `Dr ${moneyName()} ${a} · Cr ${catName()} ${a}`;
    if (mode() === "money_out") return `Dr ${catName()} ${a} · Cr ${moneyName()} ${a}`;
    if (mode() === "transfer") {
      const to = ((o()?.money || []).find((x: Any) => x.ticket === toAccount()) || {}).label || "the other account";
      return `Dr ${to} ${a} · Cr ${moneyName()} ${a}`;
    }
    return "";
  });
  const args = (): Record<string, Any> | null => {
    err.set("");
    const base: Record<string, Any> = { form: mode() };
    if (date()) base.date = date();
    if (payee()) base.payee = payee();
    if (memo()) base.memo = memo();
    if (reference()) base.reference = reference();
    if (mode() === "journal") {
      const lines = jl()
        .filter((l: Any) => l.account)
        .map((l: Any) => {
          const line: Record<string, string> = { account: l.account };
          const dr = moneyText(l.debit);
          const cr = moneyText(l.credit);
          if (dr) line.debit = dr;
          if (cr) line.credit = cr;
          return line;
        });
      if (lines.length < 2) {
        err.set("A journal needs at least two lines with an account.");
        return null;
      }
      base.lines = lines;
      return base;
    }
    if (!amt()) {
      err.set("Type an amount, like 1,350.00.");
      return null;
    }
    base.amount = amt();
    if (!money()) {
      err.set(mode() === "transfer" ? "Pick where the money leaves." : "Pick the bank, cash or card account.");
      return null;
    }
    base.account = money();
    if (mode() === "transfer") {
      if (!toAccount()) {
        err.set("Pick where the money goes.");
        return null;
      }
      base.to_account = toAccount();
    } else {
      if (!category()) {
        err.set("Pick a category.");
        return null;
      }
      base.category = category();
    }
    return base;
  };
  const save = (post: boolean) => {
    const a = args();
    if (!a) return;
    busy.set(true);
    act(post ? "books_app_post_entry" : "books_entry_draft", a)
      .then((r: Any) => {
        toast(post ? `Entry ${r.no} posted` : `Draft ${r.no} saved`);
        amount.set("");
        memo.set("");
        reference.set("");
        payee.set("");
        p.close();
      })
      .catch((e: Any) => err.set(errText(e)))
      .finally(() => busy.set(false));
  };
  const setLine = (i: number, key: string, v: string) => jl.set(jl().map((l: Any, j: number) => (j === i ? { ...l, [key]: v } : l)));
  return (
    <View class="absolute inset-0 bg-black/50 items-center justify-center">
      <View class={`w-[560px] max-h-[600px] rounded-[14px] border ${C.panel} ${C.line}`}>
        <View class={`flex-row items-center gap-2.5 px-3.5 py-3 border-b ${C.line}`}>
          <Text class={`flex-1 text-[14px] font-bold ${C.ink}`}>New entry</Text>
          <Seg options={MODES} value={mode()} onPick={p.setMode} />
          <Btn label="✕" small onPress={p.close} />
        </View>
        <Scroll class="max-h-[460px]">
          <View class="p-3.5 gap-3">
            <If when={mode() !== "journal"}>
              <View class="gap-3">
                <Field label="Amount">
                  <TextBox value={amount()} onChange={(v) => amount.set(v)} placeholder="₱ 0.00" big />
                </Field>
                <View class="flex-row gap-3">
                  <Field label="Date" grow>
                    <TextBox value={date()} onChange={(v) => date.set(v)} placeholder={o()?.today || "YYYY-MM-DD"} />
                  </Field>
                  <Field label={mode() === "money_in" ? "Into" : mode() === "transfer" ? "From" : "Paid from"} grow>
                    <Pick value={money()} options={o()?.money || []} onPick={(a: Any) => money.set(a.ticket)} placeholder="Bank, cash or card" />
                  </Field>
                </View>
                <If when={mode() === "transfer"}>
                  <Field label="To">
                    <Pick value={toAccount()} options={o()?.money || []} onPick={(a: Any) => toAccount.set(a.ticket)} placeholder="Bank, cash or card" />
                  </Field>
                </If>
                <If when={mode() !== "transfer"}>
                  <View class="flex-row gap-3">
                    <Field label={mode() === "money_in" ? "From" : "To"} grow>
                      <TextBox value={payee()} onChange={(v) => payee.set(v)} placeholder={mode() === "money_in" ? "Customer" : "Vendor"} />
                    </Field>
                    <Field label="Category" grow>
                      <Pick
                        value={category()}
                        options={(mode() === "money_in" ? o()?.income : o()?.expense) || []}
                        onPick={(a: Any) => category.set(a.ticket)}
                        placeholder="Pick a category"
                      />
                    </Field>
                  </View>
                </If>
                <View class="flex-row gap-3">
                  <Field label="What it was for" grow>
                    <TextBox value={memo()} onChange={(v) => memo.set(v)} placeholder="Office supplies" />
                  </Field>
                  <Field label="Reference" grow>
                    <TextBox value={reference()} onChange={(v) => reference.set(v)} placeholder="Invoice 0143" />
                  </Field>
                </View>
                <View class={`rounded-[10px] border border-dashed p-2.5 gap-1.5 ${C.line}`}>
                  <Text class={`text-[12px] font-bold ${C.mute}`}>Behind the scenes</Text>
                  <Text class={`text-[12px] font-mono ${C.ink}`}>{behind()}</Text>
                </View>
                <If when={mode() === "transfer"}>
                  <Text class={`text-[12px] ${C.dim}`}>Not income, not an expense. Reports leave it out of the P&L.</Text>
                </If>
              </View>
            </If>
            <If when={mode() === "journal"}>
              <View class="gap-2">
                <Text class={`text-[12px] ${C.dim}`}>For accountants: any balanced set of lines.</Text>
                <Each of={jl()}>
                  {(l: Any, i: Any) => (
                    <View class="flex-row gap-2 items-start">
                      <View class="flex-1">
                        <Pick value={l().account} options={o()?.all || []} onPick={(a: Any) => setLine(i(), "account", a.ticket)} placeholder="Account" />
                      </View>
                      <View class="w-[100px]">
                        <TextBox value={l().debit} onChange={(v: string) => setLine(i(), "debit", v)} placeholder="Debit" />
                      </View>
                      <View class="w-[100px]">
                        <TextBox value={l().credit} onChange={(v: string) => setLine(i(), "credit", v)} placeholder="Credit" />
                      </View>
                    </View>
                  )}
                </Each>
                <Btn label="+ Line" small onPress={() => jl.set([...jl(), { account: "", debit: "", credit: "" }])} />
                <Field label="What it was for">
                  <TextBox value={memo()} onChange={(v) => memo.set(v)} placeholder="Prepaid software" />
                </Field>
              </View>
            </If>
            <If when={Boolean(err())}>
              <Text class={`text-[12.5px] ${C.red}`}>{err()}</Text>
            </If>
          </View>
        </Scroll>
        <View class={`flex-row items-center gap-2 px-3.5 py-3 border-t ${C.line}`}>
          <Text class={`flex-1 text-[12px] ${C.dim}`}>{`${o()?.book?.basis_label || "Cash basis"} · counts on its date`}</Text>
          <Btn label="Cancel" onPress={p.close} />
          <Btn label="Save draft" disabled={busy()} onPress={() => save(false)} />
          <Btn label="Save & post" tone="go" disabled={busy()} onPress={() => save(true)} />
        </View>
      </View>
    </View>
  );
}
