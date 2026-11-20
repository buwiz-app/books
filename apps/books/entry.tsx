// The entry parts of the Books window: the drawer (the double entry behind
// an entry, Post or Void) and the new-entry page (Money in, Money out,
// Transfer, Journal). The Ledger itself is a native screen
// (crates/books-screens); `openLedger` opens it. The assistant drafts; only
// a click here posts or voids.

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
                <If when={Boolean(e()?.pressed)}>
                  <Kv k="Record" v={e()?.pressed} />
                </If>
                <If when={Boolean(e()?.related)}>
                  <Kv k="Linked" v={e()?.related} />
                </If>
              </View>
              <ReviewPanel entry={e()} busy={busy()} open={(t: string) => drawerEntry.set(t)} keepBoth={() => run("books_app_post_entry", { keep_both: true })} />
              <ExtractionPanel entry={e()} />
              <If when={Boolean(e()?.reviewable)}>
                <View class="gap-1.5">
                  <Text class={`text-[12px] ${C.dim}`}>Pick a category</Text>
                  <View class="flex-row gap-1.5 flex-wrap">
                    <Each of={e()?.suggest || []} key={rowKey}>
                      {(c: Any) => (
                        <View
                          class={`rounded-full border px-2.5 py-1 ${C.raise} ${C.line}`}
                          style={picked(Boolean(c().best))}
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

/**
 * What the assistant read off a document: the suggested value
 * beside the one the entry carries now, how sure the reading was, and the
 * evidence it came from. Read-only; `books_entry_get` sends `extraction` and
 * `evidence` to the app only.
 */
function ExtractionPanel(p: { entry: Any }) {
  const x = () => (p.entry && p.entry.extraction) || null;
  const fields = () => (x() && x().fields) || [];
  const docs = () => (p.entry && p.entry.evidence) || [];
  return (
    <If when={Boolean(x()) || docs().length > 0}>
      <View class="gap-1.5">
        <If when={Boolean(x())}>
          <View class="flex-row items-center gap-2">
            <Text class={`flex-1 text-[12px] ${C.dim}`}>{`Read off a document by ${x() ? x().proposed_by : ""}`}</Text>
            <Pill tone={x() && x().needs_review ? "rev" : "in"} label={(x() && x().confidence_label) || "Unsure"} />
          </View>
          <Each of={fields()} key={rowKey}>
            {(f: Any) => (
              <View class={`gap-1 rounded-[8px] border px-2.5 py-1.5 ${C.line}`}>
                <View class="flex-row items-center gap-2">
                  <Text class={`flex-1 text-[12px] ${C.ink}`}>{FIELD_NAMES[f().field] || f().field}</Text>
                  <Text class={`text-[11.5px] ${C.dim}`}>{f().confidence_label || "—"}</Text>
                  <If when={Boolean(f().corrected)}>
                    <Pill tone="draft" label="You changed it" />
                  </If>
                </View>
                <Kv k="Suggested" v={f().suggested} />
                <Kv k="Now" v={f().current} />
              </View>
            )}
          </Each>
        </If>
        <If when={docs().length > 0}>
          <Text class={`text-[12px] ${C.dim}`}>Documents</Text>
          <Each of={docs()} key={rowKey}>
            {(d: Any) => (
              <View class="flex-row gap-2">
                <Text class={`flex-1 text-[12px] ${C.ink}`} numberOfLines={1}>
                  {d().label}
                </Text>
                <Text class={`text-[11.5px] ${C.dim}`}>{d().source === "assistant" ? "Assistant" : "You"}</Text>
              </View>
            )}
          </Each>
        </If>
      </View>
    </If>
  );
}

const MODES: [string, string][] = [
  ["money_in", "Money in"],
  ["money_out", "Money out"],
  ["transfer", "Transfer"],
  ["journal", "Journal"],
];

const MODE_SUB: Record<string, string> = {
  money_in: "A customer paid you",
  money_out: "You paid someone",
  transfer: "Between your accounts",
  journal: "Any balanced set of lines",
};

/** What to ask the assistant in the chat: it saves a draft, which lands in the Inbox. */
const ASK: [string, string][] = [
  ["Pay expense", "Draft a money-out entry in Books: I paid [who] [amount] for [what] from [bank or cash]."],
  ["Receive payment", "Draft a money-in entry in Books: [who] paid me [amount] for [what] into [bank or cash]."],
  ["Transfer", "Draft a transfer in Books: moved [amount] from [account] to [account]."],
  ["Journal", "Draft a journal entry in Books: debit [account] [amount], credit [account] [amount], for [what]."],
  ["Rent", "Draft a money-out entry in Books: paid this month's rent of [amount] from [bank]."],
  ["Utility", "Draft a money-out entry in Books: paid the [electric or water] bill of [amount] from [bank]."],
];

/** The new-entry page: the type rail, the entry, and the assistant beside it. `mode` is the open mode. */
function EntryPage(p: { mode: Any; close: () => void; setMode: (m: string) => void }) {
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
  const journalLine = () => ({
    account: signal(""),
    debit: signal(""),
    credit: signal(""),
  });
  const jl = signal<Any[]>([journalLine(), journalLine()]);
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
        .filter((l: Any) => l.account())
        .map((l: Any) => {
          const line: Record<string, string> = { account: l.account() };
          const dr = moneyText(l.debit());
          const cr = moneyText(l.credit());
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
  const side = signal("ai");
  const ask = (text: string) => {
    if (!buwiz.copy) return toast("This Buwiz Bot cannot copy. Type it in the chat.");
    buwiz.copy(text);
    toast("Copied. Paste it in the chat; the draft lands in your Inbox.");
  };
  const modeName = () => (MODES.find((m) => m[0] === mode()) || ["", "New entry"])[1];
  return (
    <View class={`flex-1 ${C.bg}`}>
      <View class="flex-row items-center gap-2.5 px-4 py-3">
        <View accessibilityLabel="Back to Home" class={`w-8 h-8 rounded-full border items-center justify-center ${C.panel} ${C.line}`} onPress={p.close} activeStyle={{ opacity: 0.7 }}>
          <Text class={`text-[15px] font-bold ${C.ink}`}>‹</Text>
        </View>
        <Text class={`flex-1 text-[14px] font-bold ${C.ink}`}>New entry</Text>
        <Text class={`text-[12px] ${C.dim}`}>{`${o()?.book?.basis_label || "Cash basis"} · counts on its date`}</Text>
        <Btn label="Save draft" disabled={busy()} onPress={() => save(false)} />
        <Btn label="Save & post" tone="go" disabled={busy()} onPress={() => save(true)} />
      </View>
      <View class="flex-1 flex-row gap-3 px-4 pb-4">
        <View class="w-[104px] gap-1.5">
          <Each of={MODES}>
            {(m: Any) => (
              <View
                class={`rounded-[10px] border px-2.5 py-2 ${mode() === m()[0] ? "border-transparent" : `${C.panel} ${C.line}`}`}
                style={mode() === m()[0] ? { backgroundColor: acc() } : undefined}
                onPress={() => p.setMode(m()[0])}
                activeStyle={{ opacity: 0.7 }}
              >
                <Text class={`text-[12.5px] font-semibold ${mode() === m()[0] ? ON_ACC : C.ink}`}>{m()[1]}</Text>
              </View>
            )}
          </Each>
        </View>
        <Scroll class="flex-1">
          <View class={`rounded-xl border overflow-hidden ${C.panel} ${C.line}`}>
            <View class="px-4 pt-3.5 pb-4 gap-3" style={{ backgroundColor: acc() }}>
              <View class="flex-row items-center gap-3">
                <View class="flex-1 gap-0.5">
                  <Text class={`text-[18px] font-bold ${ON_ACC}`}>{modeName()}</Text>
                  <Text class="text-[12px] text-white/80">{MODE_SUB[mode()] || ""}</Text>
                </View>
                <If when={mode() !== "journal"}>
                  <View class="w-[200px]">
                    <TextBox label="Amount" value={amount()} onChange={(v) => amount.set(v)} placeholder="₱ 0.00" big />
                  </View>
                </If>
              </View>
              <View class="flex-row gap-3">
                <View class="w-[150px] gap-1">
                  <Text class="text-[11px] uppercase tracking-wider text-white/80">Date</Text>
                  <TextBox label="Date" value={date()} onChange={(v) => date.set(v)} placeholder={o()?.today || "YYYY-MM-DD"} />
                </View>
                <View class="flex-1 gap-1">
                  <Text class="text-[11px] uppercase tracking-wider text-white/80">Reference number</Text>
                  <TextBox label="Reference" value={reference()} onChange={(v) => reference.set(v)} placeholder="Invoice 0143" />
                </View>
                <If when={mode() !== "transfer"}>
                  <View class="flex-1 gap-1">
                    <Text class="text-[11px] uppercase tracking-wider text-white/80">{mode() === "money_in" ? "Received from" : mode() === "money_out" ? "Paid to" : "Party"}</Text>
                    <TextBox label="Party" value={payee()} onChange={(v) => payee.set(v)} placeholder={mode() === "money_in" ? "Customer" : mode() === "money_out" ? "Vendor" : "Optional"} />
                  </View>
                </If>
              </View>
            </View>
            <View class="p-4 gap-3.5">
              <Field label="Memo">
                <TextBox label="Memo" value={memo()} onChange={(v) => memo.set(v)} placeholder="What it was for" />
              </Field>
              <If when={mode() !== "journal"}>
                <View class="gap-3.5">
                  <View class="flex-row gap-3">
                    <Field label={mode() === "money_in" ? "Into" : mode() === "transfer" ? "From" : "Paid from"} grow>
                      <Pick label="Account" value={money()} options={o()?.money || []} onPick={(a: Any) => money.set(a.ticket)} placeholder="Bank, cash or card" />
                    </Field>
                    <If when={mode() === "transfer"}>
                      <Field label="To" grow>
                        <Pick label="To account" value={toAccount()} options={o()?.money || []} onPick={(a: Any) => toAccount.set(a.ticket)} placeholder="Bank, cash or card" />
                      </Field>
                    </If>
                    <If when={mode() !== "transfer"}>
                      <Field label="Category" grow>
                        <Pick
                          label="Category"
                          value={category()}
                          options={(mode() === "money_in" ? o()?.income : o()?.expense) || []}
                          onPick={(a: Any) => category.set(a.ticket)}
                          placeholder="Pick a category"
                        />
                      </Field>
                    </If>
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
                  <View class="flex-row gap-2">
                    <Text class={`w-5 text-[10.5px] uppercase tracking-wider ${C.dim}`}>#</Text>
                    <Text class={`flex-1 text-[10.5px] uppercase tracking-wider ${C.dim}`}>Account</Text>
                    <Text class={`w-[120px] text-[10.5px] uppercase tracking-wider ${C.dim}`}>Debit</Text>
                    <Text class={`w-[120px] text-[10.5px] uppercase tracking-wider ${C.dim}`}>Credit</Text>
                    <View class="w-7" />
                  </View>
                  <Each of={jl()}>
                    {(l: Any, i: Any) => (
                      <View class="flex-row gap-2 items-start">
                        <Text class={`w-5 pt-2 text-[12px] font-mono ${C.dim}`}>{String(i() + 1)}</Text>
                        <View class="flex-1">
                          <Pick value={l().account()} options={o()?.all || []} onPick={(a: Any) => l().account.set(a.ticket)} placeholder="Account" />
                        </View>
                        <View class="w-[120px]">
                          <TextBox value={l().debit()} onChange={(v: string) => l().debit.set(v)} placeholder="Debit" />
                        </View>
                        <View class="w-[120px]">
                          <TextBox value={l().credit()} onChange={(v: string) => l().credit.set(v)} placeholder="Credit" />
                        </View>
                        <View class="w-7 pt-1">
                          <If when={jl().length > 2}>
                            <Btn label="✕" small onPress={() => jl.set(jl().filter((_: Any, j: number) => j !== i()))} />
                          </If>
                        </View>
                      </View>
                    )}
                  </Each>
                  <View class="flex-row items-center gap-2">
                    <Btn label="+ Line" small onPress={() => jl.set([...jl(), journalLine()])} />
                    <Text class={`flex-1 text-[12px] ${C.dim}`}>Debits must equal credits. Books checks the lines when you save.</Text>
                  </View>
                </View>
              </If>
              <If when={Boolean(err())}>
                <Text class={`text-[12.5px] ${C.red}`}>{err()}</Text>
              </If>
            </View>
          </View>
        </Scroll>
        <View class={`w-[260px] rounded-xl border overflow-hidden ${C.panel} ${C.line}`}>
          <View class="flex-row gap-1 px-2 py-2" style={{ backgroundColor: acc() }}>
            <Each of={[["ai", "Assistant"], ["docs", "Attachments"]]}>
              {(t: Any) => (
                <View class={`rounded-[7px] px-2.5 py-1 ${side() === t()[0] ? "bg-white/25" : ""}`} onPress={() => side.set(t()[0])}>
                  <Text class={`text-[12.5px] font-semibold ${ON_ACC}`}>{t()[1]}</Text>
                </View>
              )}
            </Each>
          </View>
          <If when={side() === "ai"}>
            <View class="p-3 gap-2.5">
              <Text class={`text-[12.5px] ${C.mute}`}>Tell the assistant in the chat what happened. It saves a draft, which waits in your Inbox for you to post.</Text>
              <View class="flex-row gap-1.5 flex-wrap">
                <Each of={ASK}>
                  {(q: Any) => (
                    <View class={`rounded-full border px-2.5 py-1 ${C.raise} ${C.line}`} onPress={() => ask(q()[1])} activeStyle={{ opacity: 0.7 }}>
                      <Text class={`text-[12px] ${C.ink}`}>{q()[0]}</Text>
                    </View>
                  )}
                </Each>
              </View>
              <Text class={`text-[11.5px] ${C.dim}`}>A press copies a sentence to fill in and send. The assistant never posts.</Text>
            </View>
          </If>
          <If when={side() === "docs"}>
            <View class="p-3 gap-2.5">
              <Text class={`text-[12.5px] ${C.mute}`}>Save the draft first, then attach its receipt or bill from Documents.</Text>
              <Btn label="Open Documents" onPress={() => buwiz.open({ app: "evidence" }).catch(() => null)} />
            </View>
          </If>
        </View>
      </View>
    </View>
  );
}
