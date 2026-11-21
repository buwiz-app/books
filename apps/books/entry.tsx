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
  ["journal", "Journal"],
  ["money_in", "Pay In"],
  ["money_out", "Pay Out"],
  ["transfer", "Transfer"],
];

const MODE_SUB: Record<string, string> = {
  money_in: "A customer paid you",
  money_out: "You paid someone",
  transfer: "Between your accounts",
  journal: "Any balanced set of lines",
};

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
  const fromPerson = signal("");
  const toPerson = signal("");
  const fromPartyQuery = signal("");
  const toPartyQuery = signal("");
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
  const categoryCreateApply = signal<Any>(null);
  const journalLine = () => ({
    account: signal(""),
    query: signal(""),
    debit: signal(""),
    credit: signal(""),
    memo: signal(""),
    person: signal(""),
    partyQuery: signal(""),
    department: signal(""),
    location: signal(""),
  });
  const allocationLine = () => ({
    category: signal(""),
    query: signal(""),
    amount: signal(""),
    memo: signal(""),
    person: signal(""),
    partyQuery: signal(""),
    department: signal(""),
    location: signal(""),
  });
  const jl = signal<Any[]>([journalLine(), journalLine()]);
  const allocations = signal<Any[]>([allocationLine()]);
  const busy = signal(false);
  const err = signal("");
  const saveMenuOpen = signal(false);
  const mode = () => String(p.mode());
  const switchMode = (next: string) => {
    if (next !== mode()) {
      err.set("");
      category.set("");
      categoryQuery.set("");
      payee.set("");
      payeeTicket.set("");
      partyQuery.set("");
    }
    p.setMode(next);
  };
  const people = resource(
    () => ({
      v: ver(),
      kind: mode() === "money_in" ? "customers" : mode() === "money_out" ? "vendors" : "",
    }),
    (key: Any) => buwiz.call("books_people_list", key.kind ? { kind: key.kind } : {}),
  );
  const departments = resource(
    () => ({ v: ver() }),
    () => buwiz.call("books_tags_list", { kind: "department" }),
  );
  const locations = resource(
    () => ({ v: ver() }),
    () => buwiz.call("books_tags_list", { kind: "location" }),
  );
  const tagOptions = (res: Any, group: string) =>
    (res()?.rows || []).map((tag: Any) => ({
      ticket: tag.ticket,
      name: tag.name,
      label: tag.name,
      group,
    }));
  const partyOptions = () =>
    (people()?.rows || []).map((person: Any) => ({
      ticket: person.ticket,
      name: person.name,
      label: person.name,
      group: mode() === "money_in" ? "Customers" : mode() === "money_out" ? "Vendors" : "People",
    }));
  const suggestionsFor = (rawQuery: string) => {
    const query = rawQuery.trim().toLowerCase();
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
  const categorySuggestions = () => suggestionsFor(categoryQuery());
  const startCategoryCreate = (name: string, suggestion?: Any, apply?: (ticket: string) => void) => {
    newCategoryName.set(name || suggestion?.name || "");
    newCategoryNumber.set("");
    newCategoryType.set(suggestion?.accountType || (mode() === "money_in" ? "revenue" : "expense"));
    newCategoryKind.set(suggestion?.subtype || "unsure");
    categoryCreateApply.set(apply || null);
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
        const ticket = result.account || "";
        if (categoryCreateApply()) categoryCreateApply()(ticket);
        else category.set(ticket);
        categoryQuery.set("");
        categoryCreateApply.set(null);
        categoryCreateOpen.set(false);
        toast(`${name} added`);
      })
      .catch((e: Any) => categoryCreateError.set(errText(e)))
      .finally(() => categoryCreating.set(false));
  };
  const amt = computed(() => moneyText(amount()));
  const centsOf = (typed: string): bigint => {
    const text = moneyText(typed);
    if (!text) return 0n;
    const [whole, cents] = text.split(".");
    return BigInt(whole) * 100n + BigInt(cents);
  };
  const allocationTotal = computed(() =>
    allocations().reduce((sum: bigint, line: Any) => sum + centsOf(line.amount()), 0n),
  );
  const allocationRemaining = computed(() => centsOf(amount()) - allocationTotal());
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
    return { debit, credit, difference: debit - credit, balanced: debit === credit };
  });
  const shownCents = (value: bigint) => {
    const negative = value < 0n;
    const absolute = negative ? -value : value;
    const whole = String(absolute / 100n).replace(/\B(?=(\d{3})+(?!\d))/g, ",");
    const cents = String(absolute % 100n).padStart(2, "0");
    return `${negative ? "−" : ""}₱${whole}.${cents}`;
  };
  const args = (): Record<string, Any> | null => {
    err.set("");
    const base: Record<string, Any> = { form: mode() };
    if (date() || o()?.today) base.date = date() || o()?.today;
    if (payee()) base.payee = payee();
    if (memo()) base.memo = memo();
    if (reference()) base.reference = reference();
    if (mode() === "journal") {
      if (!journalTotals().balanced || journalTotals().debit === 0n) {
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
          if (l.memo()) line.memo = l.memo();
          if (l.person()) line.person = l.person();
          if (l.department()) line.department = l.department();
          if (l.location()) line.location = l.location();
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
      if (fromPerson()) base.from_person = fromPerson();
      if (toPerson()) base.to_person = toPerson();
    } else {
      const rows = allocations()
        .filter((line: Any) => centsOf(line.amount()) > 0n || line.category())
        .map((line: Any) => {
          const row: Record<string, string> = { category: line.category() };
          const value = moneyText(line.amount());
          if (value) row.amount = value;
          if (line.memo()) row.memo = line.memo();
          if (line.person()) row.person = line.person();
          if (line.department()) row.department = line.department();
          if (line.location()) row.location = line.location();
          return row;
        });
      if (!rows.length) {
        err.set("Add at least one category allocation.");
        return null;
      }
      if (rows.some((row) => !row.category || !row.amount)) {
        err.set("Choose a category and enter an amount for each allocation.");
        return null;
      }
      if (allocationRemaining() !== 0n) {
        err.set("Allocation amounts must add up exactly to the transaction amount.");
        return null;
      }
      base.allocations = rows;
    }
    return base;
  };
  const save = (post: boolean, close = true, fresh = false) => {
    const a = args();
    if (!a) return;
    busy.set(true);
    act(post ? "books_app_post_entry" : "books_entry_draft", a)
      .then(async (r: Any) => {
        // a document staged in the side panel links to the entry it was kept
        // for; one that fails stays staged so nothing vanishes
        let firstLinkError = "";
        const failed: Any[] = [];
        for (const d of entryStaged()) {
          try {
            await act("books_app_evidence_attach", { op: "link", evidence: d.evidence, target: { kind: "entry", ticket: r.entry } });
          } catch (e: Any) {
            if (!firstLinkError) firstLinkError = errText(e);
            failed.push(d);
          }
        }
        entryStaged.set(failed);
        if (failed.length === 0) toast(post ? `Entry ${r.no} posted` : `Draft ${r.no} saved`);
        else toast(`${post ? `Entry ${r.no} posted` : `Draft ${r.no} saved`}. ${failed.length} document${failed.length === 1 ? "" : "s"} couldn't be linked: ${firstLinkError}`);
        if (close || fresh) {
          amount.set("");
          date.set("");
          memo.set("");
          reference.set("");
          payee.set("");
          payeeTicket.set("");
          partyQuery.set("");
          money.set("");
          toAccount.set("");
          fromPerson.set("");
          toPerson.set("");
          fromPartyQuery.set("");
          toPartyQuery.set("");
          category.set("");
          categoryQuery.set("");
          allocations.set([allocationLine()]);
          jl.set([journalLine(), journalLine()]);
        }
        if (close) p.close();
      })
      .catch((e: Any) => err.set(errText(e)))
      .finally(() => busy.set(false));
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
          <Text class={`text-[16px] font-semibold ${C.ink}`}>{`New ${modeName()}`}</Text>
          <View class="flex-1" />
        </View>
        <View class="flex-1 flex-row gap-6 p-6 pb-4 overflow-hidden">
        <View class={`flex-1 min-h-0 rounded-[24px] border overflow-hidden flex-col bg-white dark:bg-slate-800 ${C.line}`}>
            <View class={`shrink-0 px-4 pt-3.5 pb-4 gap-3 ${accent() === "green" ? "bg-linear-to-r from-[#1a6b3c] to-[#27ae60] dark:from-[#145a30] dark:to-[#1e8c4c]" : ""}`} style={accent() === "green" ? undefined : { backgroundColor: acc() }}>
              <View class="flex-row items-center gap-3">
                <View class="w-12 h-12 rounded-full items-center justify-center bg-white/15">
                  <Text class="text-[20px] text-white">{mode() === "journal" ? "≡" : mode() === "transfer" ? "⇄" : mode() === "money_in" ? "↓" : "↑"}</Text>
                </View>
                <View class="flex-1 gap-0.5">
                  <Text class={`text-[24px] font-bold ${ON_ACC}`}>{modeName()}</Text>
                  <Text class="text-[12px] text-white/80">{MODE_SUB[mode()] || ""}</Text>
                </View>
                <If when={mode() !== "journal"}>
                  <If when={mode() === "transfer"}>
                    <View class="w-[200px] flex-row items-center justify-end gap-1 border border-white/20 rounded-lg px-3 py-1.5 bg-white/10 focus-within:border-white/40 focus-within:ring-1 focus-within:ring-white/30 transition-colors">
                      <Text class="text-xl font-semibold text-white/60">₱</Text>
                      <TextBox label="Amount" value={amount()} onChange={(v) => { amount.set(v); err.set(""); }} placeholder="0.00" transferAmount />
                    </View>
                  </If>
                  <If when={mode() !== "transfer"}>
                    <View class="w-[200px]">
                      <TextBox label="Amount" value={amount()} onChange={(v) => { amount.set(v); err.set(""); }} placeholder="₱ 0.00" big entry />
                    </View>
                  </If>
                </If>
                <If when={mode() === "journal"}>
                  <Text class="text-[20px] font-semibold text-white tabular-nums">{shownCents(journalTotals().debit)}</Text>
                </If>
              </View>
              <View class="flex-row gap-3">
                <View class="w-[150px] gap-1">
                  <Text class="text-[13px] font-medium text-white/85">Date</Text>
                  <BooksDate value={date() || o()?.today || ""} today={o()?.today} onChange={(v) => date.set(v)} />
                </View>
                <View class="flex-1 gap-1">
                  <Text class="text-[13px] font-medium text-white/85">Reference number</Text>
                  <TextBox label="Reference" value={reference()} onChange={(v) => reference.set(v)} placeholder="Invoice 0143" entry />
                </View>
                <If when={mode() !== "transfer"}>
                  <View class="flex-1 gap-1">
                    <Text class="text-[13px] font-medium text-white/85">{mode() === "money_in" ? "Received from" : mode() === "money_out" ? "Paid to" : "Party"}</Text>
                    <BooksCombo
                      label={mode() === "money_in" ? "Customer" : mode() === "money_out" ? "Vendor" : "Party"}
                      value={payeeTicket()}
                      options={() => partyOptions()}
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
                      group={mode() === "money_in" ? "Customers" : mode() === "money_out" ? "Vendors" : "People"}
                    />
                  </View>
                </If>
              </View>
            </View>
            <Scroll class="flex-1 min-h-0">
              <View class="px-6 pt-4">
              <Text class={`text-[13px] font-semibold ${C.ink} mb-2 pl-1`}>Memo</Text>
              <View class="rounded-xl border p-3 min-h-16 bg-white dark:bg-slate-800 border-[#e2e8f0] dark:border-slate-600">
                <TextBox label="Memo" value={memo()} onChange={(v) => memo.set(v)} placeholder="Add a memo..." entry />
              </View>
            </View>
            <View class="px-6 py-4 gap-3.5 bg-[#f8f9fb] dark:bg-slate-800/70 flex-1 flex-col">
              <If when={mode() !== "journal"}>
                <View class="gap-3.5">
                  <If when={mode() === "transfer"}>
                    <View class="flex-row items-start gap-5">
                      <View class="flex-1 gap-2">
                        <Field label="Transfer From" plain>
                          <BooksCombo label="Transfer From" value={money()} options={() => groupedOptions(o()?.money || [])} onPick={(a: Any) => money.set(a.ticket)} placeholder="Select account" group="Cash, bank & card" />
                        </Field>
                        <BooksCombo
                          label="Transfer From party"
                          value={fromPerson()}
                          options={() => partyOptions()}
                          onPick={(person: Any) => { fromPerson.set(person.ticket || ""); fromPartyQuery.set(""); }}
                          query={() => fromPartyQuery()}
                          onQuery={(query: string) => { fromPartyQuery.set(query); if (query) fromPerson.set(""); }}
                          placeholder="Party"
                          group="People"
                        />
                      </View>
                      <Text class="pt-6 text-[24px] text-[#94a3b8]">→</Text>
                      <View class="flex-1 gap-2">
                        <Field label="Transfer To" plain>
                          <BooksCombo label="Transfer To" value={toAccount()} options={() => groupedOptions(o()?.money || [])} onPick={(a: Any) => toAccount.set(a.ticket)} placeholder="Select account" group="Cash, bank & card" />
                        </Field>
                        <BooksCombo
                          label="Transfer To party"
                          value={toPerson()}
                          options={() => partyOptions()}
                          onPick={(person: Any) => { toPerson.set(person.ticket || ""); toPartyQuery.set(""); }}
                          query={() => toPartyQuery()}
                          onQuery={(query: string) => { toPartyQuery.set(query); if (query) toPerson.set(""); }}
                          placeholder="Party"
                          group="People"
                        />
                      </View>
                    </View>
                  </If>
                  <If when={mode() === "money_in" || mode() === "money_out"}>
                    <Field label={mode() === "money_in" ? "Into" : "Paid from"} plain>
                      <BooksCombo label="Money account" value={money()} options={() => groupedOptions(o()?.money || [])} onPick={(a: Any) => money.set(a.ticket)} placeholder="Bank, cash or card" group="Cash, bank & card" />
                    </Field>
                    <View class="flex-row items-center justify-between">
                      <Text class={`text-[15px] font-semibold ${C.ink}`}>{mode() === "money_in" ? "Allocate this payment" : "Allocate this expense"}</Text>
                      <Text class={`text-[12px] ${C.mute}`}>{`Remaining ${shownCents(allocationRemaining())}`}</Text>
                    </View>
                    <Each of={allocations()}>
                      {(line: Any, index: Any) => (
                        <View class="rounded-xl border border-[#e2e8f0] dark:border-slate-600 bg-white dark:bg-slate-800 p-3 gap-2.5">
                          <View class="flex-row items-center gap-2">
                            <Text class="w-5 text-[11px] font-semibold text-[#94a3b8]">{String(index() + 1)}</Text>
                            <View class="flex-1">
                              <BooksCombo
                                label="Category"
                                value={line().category()}
                                options={() => groupedOptions((mode() === "money_in" ? o()?.income : o()?.expense) || [])}
                                onPick={(account: Any) => { line().category.set(account.ticket || ""); line().query.set(""); err.set(""); }}
                                query={() => line().query()}
                                onQuery={(query: string) => line().query.set(query)}
                                suggestions={() => suggestionsFor(line().query())}
                                onSuggestion={(suggestion: Any) => startCategoryCreate(suggestion.name, suggestion, (ticket) => { line().category.set(ticket); line().query.set(""); err.set(""); })}
                                onCreate={(query: string) => startCategoryCreate(query, undefined, (ticket) => { line().category.set(ticket); line().query.set(""); err.set(""); })}
                                placeholder="Category"
                                group={mode() === "money_in" ? "Income categories" : "Expense categories"}
                              />
                            </View>
                            <View class="w-[120px]">
                              <TextBox label="Allocation amount" value={line().amount()} onChange={(value: string) => { line().amount.set(value); err.set(""); }} placeholder="₱ 0.00" entry />
                            </View>
                            <If when={allocations().length > 1}>
                              <Btn label="×" small onPress={() => { allocations.set(allocations().filter((_: Any, i: number) => i !== index())); err.set(""); }} />
                            </If>
                          </View>
                          <View class="flex-row gap-2">
                            <View class="flex-1">
                              <BooksCombo
                                label="Party"
                                value={line().person()}
                                options={() => partyOptions()}
                                onPick={(person: Any) => { line().person.set(person.ticket || ""); line().partyQuery.set(""); }}
                                query={() => line().partyQuery()}
                                onQuery={(query: string) => line().partyQuery.set(query)}
                                placeholder="Party"
                                group={mode() === "money_in" ? "Customers" : "Vendors"}
                              />
                            </View>
                            <View class="flex-1">
                              <BooksCombo label="Department" value={line().department()} options={() => tagOptions(departments, "Departments")} onPick={(tag: Any) => line().department.set(tag.ticket || "")} placeholder="Department" group="Departments" />
                            </View>
                            <View class="flex-1">
                              <BooksCombo label="Location" value={line().location()} options={() => tagOptions(locations, "Locations")} onPick={(tag: Any) => line().location.set(tag.ticket || "")} placeholder="Location" group="Locations" />
                            </View>
                          </View>
                          <TextBox label="Allocation memo" value={line().memo()} onChange={(value: string) => line().memo.set(value)} placeholder="Description for this category" entry />
                        </View>
                      )}
                    </Each>
                    <Btn label="+ Add allocation" small onPress={() => allocations.set([...allocations(), allocationLine()])} />
                    <If when={allocationRemaining() === 0n && centsOf(amount()) > 0n}>
                      <Text class={`text-[12px] font-semibold ${C.green}`}>Allocations match the amount.</Text>
                    </If>
                  </If>
                </View>
              </If>
              <If when={mode() === "journal"}>
                <View class="gap-1">
                  <View class="flex-row gap-2 pb-1">
                    <Text class={`w-5 text-[10.5px] uppercase tracking-wider ${C.dim}`}>#</Text>
                    <Text class={`flex-1 text-[11px] font-semibold ${C.mute}`}>Line description</Text>
                    <Text class={`w-[100px] text-[11px] font-semibold text-right ${C.mute}`}>Debit</Text>
                    <Text class={`w-[100px] text-[11px] font-semibold text-right ${C.mute}`}>Credit</Text>
                    <View class="w-[104px]" />
                  </View>
                  <Each of={jl()}>
                    {(l: Any, i: Any) => (
                      <View class="border-b border-[#e2e8f0] dark:border-slate-700 py-3 gap-2">
                        <View class="flex-row gap-2 items-center">
                          <Text class={`w-5 text-[11px] font-semibold text-center ${C.dim}`}>{String(i() + 1)}</Text>
                          <View class="flex-1">
                            <TextBox label="Line description" value={l().memo()} onChange={(v: string) => l().memo.set(v)} placeholder="Add description for this item..." entry />
                          </View>
                          <View class="w-[100px]">
                            <TextBox label="Debit" value={l().debit()} onChange={(v: string) => { l().debit.set(v); err.set(""); }} placeholder="₱ 0.00" entry />
                          </View>
                          <View class="w-[100px]">
                            <TextBox label="Credit" value={l().credit()} onChange={(v: string) => { l().credit.set(v); err.set(""); }} placeholder="₱ 0.00" entry />
                          </View>
                          <View class="flex-row items-center gap-1">
                            <If when={jl().length > 2}>
                              <Btn label="×" small onPress={() => { jl.set(jl().filter((_: Any, j: number) => j !== i())); err.set(""); }} />
                            </If>
                            <Btn label="⧉" small onPress={() => {
                              const copy = journalLine();
                              copy.account.set(l().account());
                              copy.query.set(l().query());
                              copy.debit.set(l().debit());
                              copy.credit.set(l().credit());
                              copy.memo.set(l().memo());
                              copy.person.set(l().person());
                              copy.department.set(l().department());
                              copy.location.set(l().location());
                              const next = [...jl()];
                              next.splice(i() + 1, 0, copy);
                              jl.set(next);
                              err.set("");
                            }} />
                            <Btn label="+" small onPress={() => { const next = [...jl()]; next.splice(i() + 1, 0, journalLine()); jl.set(next); }} />
                          </View>
                        </View>
                        <View class="flex-row gap-2 pl-7">
                          <View class="flex-1">
                            <BooksCombo
                              label="Account"
                              value={l().account()}
                              options={() => groupedOptions(o()?.all || [])}
                              onPick={(account: Any) => { l().account.set(account.ticket || ""); l().query.set(""); err.set(""); }}
                              query={() => l().query()}
                              onQuery={(query: string) => l().query.set(query)}
                              onSuggestion={(suggestion: Any) => startCategoryCreate(suggestion.name, suggestion, (ticket) => { l().account.set(ticket); l().query.set(""); err.set(""); })}
                              onCreate={(query: string) => startCategoryCreate(query, undefined, (ticket) => { l().account.set(ticket); l().query.set(""); err.set(""); })}
                              placeholder="Category"
                              group="Accounts"
                            />
                          </View>
                          <View class="flex-1">
                            <BooksCombo
                              label="Party"
                              value={l().person()}
                              options={() => partyOptions()}
                              onPick={(person: Any) => { l().person.set(person.ticket || ""); l().partyQuery.set(""); }}
                              query={() => l().partyQuery()}
                              onQuery={(query: string) => l().partyQuery.set(query)}
                              placeholder="Party"
                              group="People"
                            />
                          </View>
                          <View class="flex-1">
                            <BooksCombo label="Department" value={l().department()} options={() => tagOptions(departments, "Departments")} onPick={(tag: Any) => l().department.set(tag.ticket || "")} placeholder="Department" group="Departments" />
                          </View>
                          <View class="flex-1">
                            <BooksCombo label="Location" value={l().location()} options={() => tagOptions(locations, "Locations")} onPick={(tag: Any) => l().location.set(tag.ticket || "")} placeholder="Location" group="Locations" />
                          </View>
                        </View>
                      </View>
                    )}
                  </Each>
                  <View class="flex-row items-center gap-2 pt-2">
                    <Btn label="+ Add line" small onPress={() => jl.set([...jl(), journalLine()])} />
                    <Text class={`flex-1 text-[12px] ${C.dim}`}>Debits must equal credits.</Text>
                  </View>
                </View>
              </If>
            </View>
            </Scroll>
            <If when={mode() === "journal"}>
              <View class="shrink-0 flex-row items-center justify-between border-t border-[#e2e8f0] dark:border-slate-600 px-6 py-2.5 bg-[#f8f9fb] dark:bg-slate-800/90">
                <View class="flex-row items-center gap-2">
                  <Text class={`text-[16px] font-bold ${journalTotals().balanced ? "text-[#16a34a] dark:text-[#4ade80]" : "text-[#ef4444]"}`}>{journalTotals().balanced ? "✓" : "!"}</Text>
                  <Text class={`text-[12px] font-semibold tabular-nums ${journalTotals().balanced ? "text-[#16a34a] dark:text-[#4ade80]" : "text-[#ef4444]"}`}>
                    {journalTotals().balanced ? shownCents(journalTotals().debit) : `Difference ${shownCents(journalTotals().difference)}`}
                  </Text>
                </View>
                <View class="flex-row gap-6">
                  <Text class={`text-[12px] ${C.mute}`}>{`Debit ${shownCents(journalTotals().debit)}`}</Text>
                  <Text class={`text-[12px] ${C.mute}`}>{`Credit ${shownCents(journalTotals().credit)}`}</Text>
                </View>
              </View>
            </If>
            <View class={`shrink-0 flex-row items-center justify-between border-t px-6 py-3 bg-white dark:bg-slate-800 ${C.line}`}>
              <View class="flex-1">
                <If when={Boolean(err())}>
                  <Text class={`text-[12px] ${C.red}`} numberOfLines={1}>{err()}</Text>
                </If>
                <If when={!err() && mode() === "money_in" || !err() && mode() === "money_out"}>
                  <Text class={`text-[12px] ${allocationRemaining() === 0n ? "text-[#14b8a6]" : C.mute}`}>{`Remaining ${shownCents(allocationRemaining())}`}</Text>
                </If>
              </View>
              <View class="relative flex-row items-stretch">
                <View
                  class="items-center justify-center rounded-l-lg bg-[#2a9d8f] px-5 py-2.5"
                  activeStyle={{ opacity: 0.7 }}
                  onPress={() => { if (!busy()) save(false, true); }}
                >
                  <Text class="text-[13px] font-medium text-white">{busy() ? "Saving..." : "Save & Close"}</Text>
                </View>
                <View
                  class="w-10 items-center justify-center rounded-r-lg border-l border-white/25 bg-[#2a9d8f]"
                  activeStyle={{ opacity: 0.7 }}
                  accessibilityLabel="More save options"
                  onPress={() => saveMenuOpen.set(!saveMenuOpen())}
                >
                  <Text class="text-[12px] text-white">⌄</Text>
                </View>
                <If when={saveMenuOpen()}>
                  <View class={`absolute right-0 bottom-full z-50 mb-1 w-[160px] rounded-lg border py-1 shadow-lg ${C.panel} ${C.line}`}>
                    <View class="px-3 py-2" activeStyle={{ opacity: 0.7 }} onPress={() => { saveMenuOpen.set(false); if (!busy()) save(false, false); }}>
                      <Text class={`text-[12px] ${C.ink}`}>Save</Text>
                    </View>
                    <View class="px-3 py-2" activeStyle={{ opacity: 0.7 }} onPress={() => { saveMenuOpen.set(false); if (!busy()) save(false, false, true); }}>
                      <Text class={`text-[12px] ${C.ink}`}>Save & New</Text>
                    </View>
                    <View class={`px-3 py-2 border-t ${C.line}`} activeStyle={{ opacity: 0.7 }} onPress={() => { saveMenuOpen.set(false); if (!busy()) save(true, true); }}>
                      <Text class={`text-[12px] ${C.ink}`}>Post</Text>
                    </View>
                  </View>
                </If>
              </View>
            </View>
        </View>
        <EntrySide />
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
