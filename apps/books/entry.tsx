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
  ["money_in", "Pay In"],
  ["money_out", "Pay Out"],
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
  const chart = resource(
    () => ({ v: ver() }),
    () => buwiz.call("books_accounts_list", { group: "all" }),
  );
  const groupedOptions = (items: Any[]) => {
    const groups = new Map((chart()?.rows || []).map((account: Any) => [String(account.ticket), account.group]));
    return (items || []).map((item: Any) => ({
      ...item,
      group: String(groups.get(String(item.ticket)) || item.group || "Accounts"),
    }));
  };
  const amount = signal("");
  const date = signal("");
  const money = signal("");
  const toAccount = signal("");
  const category = signal("");
  const categoryQuery = signal("");
  const payee = signal("");
  const payeeTicket = signal("");
  const partyQuery = signal("");
  const memo = signal("");
  const reference = signal("");
  const categoryCreateOpen = signal(false);
  const categoryCreating = signal(false);
  const categoryCreateError = signal("");
  const newCategoryName = signal("");
  const newCategoryNumber = signal("");
  const newCategoryType = signal("expense");
  const newCategoryKind = signal("unsure");
  const journalLine = () => ({
    account: signal(""),
    query: signal(""),
    debit: signal(""),
    credit: signal(""),
  });
  const jl = signal<Any[]>([journalLine(), journalLine()]);
  const busy = signal(false);
  const err = signal("");
  const mode = () => String(p.mode());
  const switchMode = (next: string) => {
    if (next !== mode()) {
      category.set("");
      categoryQuery.set("");
      payee.set("");
      payeeTicket.set("");
      partyQuery.set("");
    }
    p.setMode(next);
  };
  const people = resource(
    () => ({ v: ver(), kind: mode() === "money_in" ? "customers" : "vendors" }),
    (key: Any) => buwiz.call("books_people_list", { kind: key.kind }),
  );
  const partyOptions = () =>
    (people()?.rows || []).map((person: Any) => ({
      ticket: person.ticket,
      name: person.name,
      label: person.name,
      group: mode() === "money_in" ? "Customers" : "Vendors",
    }));
  const categorySuggestions = () => {
    const query = categoryQuery().trim().toLowerCase();
    if (!query || (mode() !== "money_in" && mode() !== "money_out")) return [];
    const types =
      mode() === "money_in"
        ? ["revenue", "other_income", "equity"]
        : ["expense", "cost_of_revenue", "other_expense", "asset", "equity"];
    const existing = new Set(
      [...(o()?.income || []), ...(o()?.expense || [])].map((account: Any) => String(account.label || account.name || "").toLowerCase()),
    );
    return SUGGESTED_CATEGORY_CATALOG.filter((suggestion: Any) => {
      const text = `${suggestion.name} ${suggestion.description} ${(suggestion.keywords || []).join(" ")}`.toLowerCase();
      const cashAccrualOnly =
        o()?.book?.basis === "cash" && ["account_receivable", "account_payable"].includes(String(suggestion.subtype));
      return types.includes(suggestion.accountType) && !cashAccrualOnly && text.includes(query) && !existing.has(String(suggestion.name).toLowerCase());
    }).slice(0, 8);
  };
  const startCategoryCreate = (name: string, suggestion?: Any) => {
    newCategoryName.set(name || suggestion?.name || "");
    newCategoryNumber.set("");
    newCategoryType.set(suggestion?.accountType || (mode() === "money_in" ? "revenue" : "expense"));
    newCategoryKind.set(suggestion?.subtype || "unsure");
    categoryCreateError.set("");
    categoryCreateOpen.set(true);
  };
  const createCategory = () => {
    const name = newCategoryName().trim();
    const number = newCategoryNumber().trim();
    if (!name) return categoryCreateError.set("Enter a category name.");
    if (!number) return categoryCreateError.set("Enter an account number.");
    categoryCreating.set(true);
    categoryCreateError.set("");
    act("books_app_account_create", {
      number,
      name,
      kind: newCategoryKind(),
      type: newCategoryType(),
    })
      .then((result: Any) => {
        category.set(result.account || "");
        categoryQuery.set("");
        categoryCreateOpen.set(false);
        toast(`${name} added`);
      })
      .catch((e: Any) => categoryCreateError.set(errText(e)))
      .finally(() => categoryCreating.set(false));
  };
  const amt = computed(() => moneyText(amount()));
  const journalTotals = computed(() => {
    let debit = 0n;
    let credit = 0n;
    for (const line of jl()) {
      const dr = moneyText(line.debit());
      const cr = moneyText(line.credit());
      if (dr) {
        const [whole, cents] = dr.split(".");
        debit += BigInt(whole) * 100n + BigInt(cents);
      }
      if (cr) {
        const [whole, cents] = cr.split(".");
        credit += BigInt(whole) * 100n + BigInt(cents);
      }
    }
    return { debit, credit, difference: debit - credit, balanced: debit > 0n && debit === credit };
  });
  const shownCents = (value: bigint) => {
    const negative = value < 0n;
    const absolute = negative ? -value : value;
    const whole = String(absolute / 100n).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
    const cents = String(absolute % 100n).padStart(2, "0");
    return `${negative ? "−" : ""}₱${whole}.${cents}`;
  };
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
    if (date() || o()?.today) base.date = date() || o()?.today;
    if (payee()) base.payee = payee();
    if (memo()) base.memo = memo();
    if (reference()) base.reference = reference();
    if (mode() === "journal") {
      if (!journalTotals().balanced) {
        err.set("Debits and credits must be equal before saving.");
        return null;
      }
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
      if (money() === toAccount()) {
        err.set("Choose two different accounts for a transfer.");
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
    <View class="flex-1 flex-row bg-[#f0f2f5] dark:bg-slate-950">
      <View class="w-16 shrink-0 bg-[#f7f8fa] dark:bg-slate-900 border-r border-[#e2e8f0] dark:border-slate-700 flex-col items-center pt-4 gap-1">
          <Each of={MODES}>
            {(m: Any) => (
              <View
                class={`w-14 flex-col items-center gap-1 py-2.5 rounded-lg border transition-all ${mode() === m()[0] ? `bg-white dark:bg-slate-800 shadow-sm ${C.line}` : "border-transparent"}`}
                style={mode() === m()[0] ? { color: "#2a9d8f" } : undefined}
                onPress={() => switchMode(m()[0])}
                activeStyle={{ opacity: 0.7 }}
              >
                <Text class={`text-[16px] ${mode() === m()[0] ? "text-[#2a9d8f] dark:text-[#2a9d8f]" : "text-[#64748b] dark:text-slate-500"}`}>
                  {m()[0] === "money_in" ? "↓" : m()[0] === "money_out" ? "↑" : m()[0] === "transfer" ? "⇄" : "≡"}
                </Text>
                <Text class={`text-[10px] font-medium ${mode() === m()[0] ? "text-[#2a9d8f] dark:text-[#2a9d8f]" : "text-[#64748b] dark:text-slate-500"}`}>{m()[1]}</Text>
              </View>
            )}
          </Each>
      </View>
      <View class="flex-1 flex-col overflow-hidden">
        <View class={`flex-row items-center gap-3 px-6 py-3 border-b bg-white dark:bg-slate-900 ${C.line}`}>
          <View accessibilityLabel="Go back" class="w-9 h-9 rounded-lg items-center justify-center" onPress={p.close} activeStyle={{ opacity: 0.7 }}>
            <Text class={`text-[18px] ${C.mute}`}>‹</Text>
          </View>
          <View class="flex-1" />
          <If when={Boolean(err())}>
            <Text class={`text-[12px] font-medium ${C.red}`} numberOfLines={1}>{err()}</Text>
          </If>
          <Btn label="Save" disabled={busy()} onPress={() => save(false)} />
          <Btn label="Save & post" tone="go" disabled={busy()} onPress={() => save(true)} />
        </View>
        <View class="flex-1 flex-row gap-5 p-5 overflow-hidden">
        <Scroll class="flex-1">
          <View class={`rounded-2xl border overflow-hidden ${C.panel} ${C.line}`}>
            <View class={`px-4 pt-3.5 pb-4 gap-3 ${accent() === "green" ? "bg-linear-to-r from-[#1a6b3c] to-[#27ae60] dark:from-[#145a30] dark:to-[#1e8c4c]" : ""}`} style={accent() === "green" ? undefined : { backgroundColor: acc() }}>
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
                  <BooksDate value={date() || o()?.today || ""} today={o()?.today} onChange={(v) => date.set(v)} />
                </View>
                <View class="flex-1 gap-1">
                  <Text class="text-[11px] uppercase tracking-wider text-white/80">Reference number</Text>
                  <TextBox label="Reference" value={reference()} onChange={(v) => reference.set(v)} placeholder="Invoice 0143" />
                </View>
                <If when={mode() !== "transfer"}>
                  <View class="flex-1 gap-1">
                    <Text class="text-[11px] uppercase tracking-wider text-white/80">{mode() === "money_in" ? "Received from" : mode() === "money_out" ? "Paid to" : "Party"}</Text>
                    <BooksCombo
                      label={mode() === "money_in" ? "Customer" : "Vendor"}
                      value={payeeTicket()}
                      options={partyOptions()}
                      onPick={(person: Any) => {
                        payeeTicket.set(person.ticket || "");
                        payee.set(person.name || person.label || "");
                        partyQuery.set("");
                      }}
                      onQuery={(query: string) => {
                        partyQuery.set(query);
                        if (query) {
                          payeeTicket.set("");
                          payee.set(query);
                        }
                      }}
                      query={partyQuery()}
                      placeholder={payee() || (mode() === "money_in" ? "Customer" : "Vendor")}
                      group={mode() === "money_in" ? "Customers" : "Vendors"}
                    />
                  </View>
                </If>
              </View>
            </View>
            <View class="px-6 relative z-10 -mt-12">
              <Text class="text-[12px] font-semibold text-white mb-2 pl-1">Memo</Text>
              <View class={`rounded-xl shadow-sm border p-4 min-h-16 ${C.panel} ${C.line}`}>
                <TextBox label="Memo" value={memo()} onChange={(v) => memo.set(v)} placeholder="Add a memo..." />
              </View>
            </View>
            <View class="px-6 py-4 pb-12 gap-3.5 bg-[#f8f9fb] dark:bg-slate-950/40">
              <If when={mode() !== "journal"}>
                <View class="gap-3.5">
                  <View class="flex-row gap-3">
                    <Field label={mode() === "money_in" ? "Into" : mode() === "transfer" ? "From" : "Paid from"} grow>
                      <BooksCombo label="Account" value={money()} options={groupedOptions(o()?.money || [])} onPick={(a: Any) => money.set(a.ticket)} placeholder="Bank, cash or card" group="Cash, bank & card" />
                    </Field>
                    <If when={mode() === "transfer"}>
                      <Field label="To" grow>
                        <BooksCombo label="To account" value={toAccount()} options={groupedOptions(o()?.money || [])} onPick={(a: Any) => toAccount.set(a.ticket)} placeholder="Bank, cash or card" group="Cash, bank & card" />
                      </Field>
                    </If>
                    <If when={mode() !== "transfer"}>
                      <Field label="Category" grow>
                        <BooksCombo
                          label="Category"
                          value={category()}
                          options={groupedOptions((mode() === "money_in" ? o()?.income : o()?.expense) || [])}
                          onPick={(a: Any) => {
                            category.set(a.ticket);
                            categoryQuery.set("");
                          }}
                          query={categoryQuery()}
                          onQuery={(query: string) => categoryQuery.set(query)}
                          suggestions={categorySuggestions()}
                          onSuggestion={(suggestion: Any) => startCategoryCreate(suggestion.name, suggestion)}
                          onCreate={(query: string) => startCategoryCreate(query)}
                          placeholder="Pick a category"
                          group={mode() === "money_in" ? "Income categories" : "Expense categories"}
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
                          <BooksCombo
                            value={l().account()}
                            options={groupedOptions(o()?.all || [])}
                            onPick={(a: Any) => {
                              l().account.set(a.ticket);
                              l().query.set("");
                            }}
                            query={l().query()}
                            onQuery={(query: string) => l().query.set(query)}
                            placeholder="Account"
                            group="Accounts"
                          />
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
                    <Text class={`flex-1 text-[12px] ${C.dim}`}>Debits must equal credits.</Text>
                  </View>
                  <View class={`flex-row items-center justify-between rounded-md border px-3 py-2 ${journalTotals().balanced ? "border-[#86efac] bg-[#f0fdf4] dark:border-[#166534] dark:bg-[#14532d]" : "border-[#e2e8f0] bg-[#f8fafc] dark:border-[#2a2a2d] dark:bg-[#141416]"}`}>
                    <Text class={`text-[12px] font-semibold ${journalTotals().balanced ? "text-[#15803d] dark:text-[#86efac]" : C.mute}`}>
                      {journalTotals().balanced ? "Balanced" : "Difference"}
                    </Text>
                    <View class="flex-row gap-4">
                      <Text class={`text-[12px] ${C.mute}`}>{`Debit ${shownCents(journalTotals().debit)}`}</Text>
                      <Text class={`text-[12px] ${C.mute}`}>{`Credit ${shownCents(journalTotals().credit)}`}</Text>
                      <Text class={`text-[12px] font-semibold ${journalTotals().balanced ? "text-[#15803d] dark:text-[#86efac]" : "text-[#ef4444]"}`}>{shownCents(journalTotals().difference)}</Text>
                    </View>
                  </View>
                </View>
              </If>
              <If when={Boolean(err())}>
                <Text class={`text-[12.5px] ${C.red}`}>{err()}</Text>
              </If>
            </View>
          </View>
        </Scroll>
        <View class={`w-[360px] shrink-0 rounded-2xl border overflow-hidden shadow-[0_4px_24px_rgba(0,0,0,0.08)] flex-col h-full ${C.panel} ${C.line}`}>
          <View class={`flex-row justify-evenly px-4 py-3 gap-2 ${accent() === "green" ? "bg-linear-to-r from-[#1a6b3c] to-[#27ae60] dark:from-[#145a30] dark:to-[#1e8c4c]" : ""}`} style={accent() === "green" ? undefined : { backgroundColor: acc() }}>
            <Each of={[["attachments", "⌁"], ["ai", "✧"], ["activity", "◷"]]}>
              {(t: Any) => (
                <View
                  accessibilityLabel={t()[0] === "ai" ? "Assistant" : t()[0] === "attachments" ? "Attachments" : "Activity log"}
                  class={`w-[88px] h-11 rounded-full items-center justify-center ${side() === t()[0] ? "bg-white/20" : ""}`}
                  onPress={() => side.set(t()[0])}
                >
                  <Text class={`text-[16px] ${side() === t()[0] ? "text-white" : "text-white/50"}`}>{t()[1]}</Text>
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
          <If when={side() === "attachments"}>
            <View class="p-3 gap-2.5">
              <Text class={`text-[13px] font-semibold ${C.ink}`}>Attachments</Text>
              <View class={`rounded-xl border border-dashed p-4 gap-2 items-center ${C.line} ${C.raise}`}>
                <Text class={`text-[22px] ${C.dim}`}>⌁</Text>
                <Text class={`text-[12.5px] text-center ${C.mute}`}>Add a receipt or document to Books before asking the assistant to read it.</Text>
                <Btn label="Browse documents" onPress={() => buwiz.open({ app: "evidence" }).catch(() => null)} />
              </View>
              <Text class={`text-[11.5px] ${C.dim}`}>Documents are stored in Books and can be attached to a chat. This entry draft does not yet link a document directly.</Text>
            </View>
          </If>
          <If when={side() === "activity"}>
            <View class="p-3 gap-2.5">
              <Text class={`text-[14px] font-semibold ${C.ink}`}>Activity Log</Text>
              <View class="flex-row items-start gap-2">
                <View class="w-6 h-6 rounded-full items-center justify-center" style={{ backgroundColor: "#2a9d8f" }}>
                  <Text class="text-[10px] font-bold text-white">U</Text>
                </View>
                <View class="gap-0.5">
                  <Text class={`text-[12px] ${C.ink}`}>You · Just now</Text>
                  <Text class={`text-[11px] ${C.mute}`}>Creating transaction.</Text>
                </View>
              </View>
            </View>
          </If>
        </View>
      </View>
      </View>
      <If when={categoryCreateOpen()}>
        <Overlay
          anchor={{ x: 24, y: 120 }}
          placement="bottom"
          align="start"
          offset={4}
          margin={8}
          onCancel={() => categoryCreateOpen.set(false)}
          onPressOutside={() => categoryCreateOpen.set(false)}
        >
          <View class={`w-[360px] gap-3 p-4 rounded-xl border shadow-lg ${C.panel} ${C.line}`}>
            <Text class={`text-[14px] font-bold ${C.ink}`}>Create category</Text>
            <Field label="Category name">
              <TextBox value={newCategoryName()} onChange={(value: string) => newCategoryName.set(value)} placeholder="Category name" />
            </Field>
            <Field label="Account number">
              <TextBox value={newCategoryNumber()} onChange={(value: string) => newCategoryNumber.set(value.replace(/[^0-9]/g, ""))} placeholder="Enter an unused number" />
            </Field>
            <Text class={`text-[12px] ${C.dim}`}>{`Account type: ${newCategoryType()} · kind: ${newCategoryKind()}`}</Text>
            <If when={Boolean(categoryCreateError())}>
              <Text class={`text-[12px] ${C.red}`}>{categoryCreateError()}</Text>
            </If>
            <View class="flex-row justify-end gap-2">
              <Btn label="Cancel" disabled={categoryCreating()} onPress={() => categoryCreateOpen.set(false)} />
              <Btn label={categoryCreating() ? "Adding…" : "Add category"} tone="go" disabled={categoryCreating()} onPress={createCategory} />
            </View>
          </View>
        </Overlay>
      </If>
    </View>
  );
}
