// The entry parts of the Books window: the drawer (the double entry behind
// an entry, Post or Void) and the new-entry page (Money in, Money out,
// Transfer, Journal). The Ledger itself is a native screen
// (crates/books-screens); `openLedger` opens it. The assistant drafts; only
// a click here posts or voids.

/** The open entry's ticket, for the drawer. */
const drawerEntry = signal("");

/** Open the ledger's native screen. Set only the props the caller has. */
// The line's party and tag assignments, joined for display: "Ana Cruz · North".
function lineAssignees(l: Any): string {
  return [l?.person?.name, l?.department?.name, l?.location?.name].filter(Boolean).join(" · ");
}

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
    <View class={`absolute top-0 right-0 bottom-0 w-[360px] max-w-full border-l ${C.panel} ${C.line}`}>
      <View class={`flex-row items-center gap-2 px-3.5 py-3 border-b ${C.line}`}>
        <Text class={`flex-1 text-[14px] font-bold ${C.ink} truncate`}>
          {e()?.title || "Entry"}
        </Text>
        <If when={Boolean(e())}>
          <Pill tone={e()?.pill} label={e()?.status_label} />
        </If>
        <Btn label="✕" small onPress={() => drawerEntry.set("")} />
      </View>
      <Scroll class="flex-1">
        <View class="px-3.5 pt-3.5 pb-24 lg:pb-3.5 gap-3">
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
                        <View class="flex-1 min-w-0">
                          <Text class={`text-[12px] ${C.ink} truncate`}>{l().account}</Text>
                          <If when={Boolean(lineAssignees(l()))}>
                            <Text class={`text-[10.5px] ${C.dim} truncate`}>{lineAssignees(l())}</Text>
                          </If>
                        </View>
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
      <Text class={`flex-1 text-[12.5px] ${C.ink} line-clamp-2`}>
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
                <Text class={`flex-1 text-[12px] ${C.ink} truncate`}>
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

/** Same labels as the old app's shared TYPE_LABELS (constants.tsx). */
const TYPE_LABEL: Record<string, string> = {
  journal: "Journal Entry",
  money_in: "Pay In",
  money_out: "Pay Out",
  transfer: "Transfer",
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
    deptQuery: signal(""),
    location: signal(""),
    locQuery: signal(""),
  });
  const allocationLine = () => ({
    category: signal(""),
    query: signal(""),
    amount: signal(""),
    memo: signal(""),
    person: signal(""),
    partyQuery: signal(""),
    department: signal(""),
    deptQuery: signal(""),
    location: signal(""),
    locQuery: signal(""),
  });
  const jl = signal<Any[]>([journalLine(), journalLine()]);
  const allocations = signal<Any[]>([allocationLine()]);
  // the draft this form already saved, so a second Save edits it instead of
  // writing a new one (PR37 finding 6)
  const savedEntry = signal("");
  const busy = signal(false);
  const err = signal("");
  const saveMenuOpen = signal(false);
  const typeMenuOpen = signal(false);
  let typeMenuAnchor: ViewRef | undefined;
  let saveMenuAnchor: ViewRef | undefined;
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
    // a mode with no natural kind (journal, transfer) asks every people kind
    // and merges the rows; omitting kind returns vendors only (PR37 finding 9)
    (key: Any) =>
      key.kind
        ? buwiz.call("books_people_list", { kind: key.kind })
        : Promise.all(
            ["vendors", "customers", "employees", "owners", "lenders", "government"].map((kind) =>
              buwiz.call("books_people_list", { kind }),
            ),
          ).then((rs: Any[]) => ({ rows: rs.flatMap((r: Any) => r?.rows || []) })),
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
    // `kind` is a catalog kind id picked at the source; `subtype` keeps the
    // old app's vocabulary for the cash/accrual filter (PR37 finding 8)
    newCategoryKind.set(suggestion?.kind || "unsure");
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
    // a catalog kind fixes the account type, so `type` goes only with
    // unsure/mixed and the server derives it otherwise (PR37 finding 3)
    const args: Record<string, Any> = { number, name, kind: newCategoryKind() };
    if (newCategoryKind() === "unsure" || newCategoryKind() === "mixed") args.type = newCategoryType();
    act("books_app_account_create", args)
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
  /** One-click party create: the combobox's "Create …" row calls the tool and
   *  selects the ticket it returns. The kind follows context: Money Out pays a
   *  vendor, Money In collects from a customer; a line with a category takes
   *  the party kind the old app's CategoryPartyCombobox derives from it
   *  (revenue-ish accounts collect, everything else pays), else vendor. */
  const partyKindFor = (accountTicket: string, fallback: string) => {
    const account =
      (o()?.all || []).find((a: Any) => String(a.ticket) === String(accountTicket)) ||
      (chart()?.rows || []).find((a: Any) => String(a.ticket) === String(accountTicket));
    const kind = `${account?.type || ""} ${account?.kind || ""} ${account?.group || ""}`.toLowerCase();
    if (/(revenue|income|receivable)/.test(kind)) return "customers";
    if (/(expense|payable|cost)/.test(kind)) return "vendors";
    return fallback;
  };
  const createParty = (kind: string, name: string, apply: (ticket: string) => void) => {
    const n = String(name || "").trim();
    if (!n) return;
    act("books_app_person_create", { kind, name: n })
      .then((r: Any) => { apply(r.person || ""); toast(`${n} added`); })
      .catch((e: Any) => err.set(errText(e)));
  };
  const createTag = (kind: string, name: string, apply: (ticket: string) => void) => {
    const n = String(name || "").trim();
    if (!n) return;
    act("books_app_tag_create", { kind, name: n })
      .then((r: Any) => { apply(r.tag || ""); toast(`${n} added`); })
      .catch((e: Any) => err.set(errText(e)));
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
  const args = (post: boolean): Record<string, Any> | null => {
    err.set("");
    const base: Record<string, Any> = { form: mode() };
    if (savedEntry()) base.entry = savedEntry();
    if (date() || o()?.today) base.date = date() || o()?.today;
    if (payee()) base.payee = payee();
    if (payeeTicket()) base.person = payeeTicket();
    // an edit of the saved draft says so when the person was taken off
    else if (savedEntry()) base.person = "";
    if (memo()) base.memo = memo();
    if (reference()) base.reference = reference();
    if (mode() === "journal") {
      // an unbalanced journal may wait as a draft; only a post refuses it
      if (post && (!journalTotals().balanced || journalTotals().debit === 0n)) {
        err.set("Debits and credits must be equal before posting.");
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
    const a = args(post);
    if (!a) return;
    // posting a remembered draft: `books_app_post_entry` posts the stored
    // draft and ignores every other argument, so the form's current values
    // first go to `books_entry_draft` as an edit; a refused edit keeps the
    // draft open and no post is sent
    const posting = post && Boolean(savedEntry());
    const ticket = savedEntry();
    busy.set(true);
    const write = posting
      ? act("books_entry_draft", a).then(() => act("books_app_post_entry", { entry: ticket }))
      : act(post ? "books_app_post_entry" : "books_entry_draft", a);
    write
      .then(async (r: Any) => {
        // a document staged in the side panel links to the entry it was kept
        // for; one that fails stays in the Documents library, unlinked, so
        // the next form never shows another entry's attachment (PR37 f1)
        let firstLinkError = "";
        let failed = 0;
        for (const d of entryStaged()) {
          try {
            await act("books_app_evidence_attach", { op: "link", evidence: d.evidence, target: { kind: "entry", ticket: r.entry } });
          } catch (e: Any) {
            if (!firstLinkError) firstLinkError = errText(e);
            failed += 1;
          }
        }
        entryStaged.set([]);
        if (failed === 0) toast(post ? `Entry ${r.no} posted` : `Draft ${r.no} saved`);
        else toast(`${post ? `Entry ${r.no} posted` : `Draft ${r.no} saved`}. ${failed} document${failed === 1 ? "" : "s"} couldn't be linked — still in Documents: ${firstLinkError}`);
        // the remembered draft ticket survives only a plain Save that keeps
        // the form open; Post, Save & Close and Save & New all clear it so a
        // reused page never edits or re-posts the old entry (PR37 finding 6)
        savedEntry.set(post || close || fresh ? "" : String(r.entry || ""));
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
      <View class="flex-1 flex-col overflow-hidden relative">
        <View class={`flex-row items-center gap-3 px-6 py-3 border-b bg-white dark:bg-slate-900 ${C.line}`}>
          <View
            class="w-9 h-9 shrink-0 items-center justify-center rounded-lg transition-colors hover:bg-[#f1f5f9] dark:hover:bg-slate-800"
            accessibilityLabel="Go back"
            activeStyle={{ opacity: 0.7 }}
            onPress={() => {
              // staged documents stay in Documents; they do not wait for the next entry
              entryStaged.set([]);
              p.close();
            }}
          >
            <Text class="text-[18px] font-medium text-[#64748b] dark:text-slate-400">←</Text>
          </View>
          <View
            ref={typeMenuAnchor}
            accessibilityLabel="Entry type"
            class={`h-9 shrink-0 flex-row items-center gap-2 rounded-lg border ${C.line} px-3 hover:bg-[#f1f5f9] dark:hover:bg-slate-800 transition-colors`}
            activeStyle={{ opacity: 0.7 }}
            onPress={() => { typeMenuOpen.set(!typeMenuOpen()); saveMenuOpen.set(false); }}
          >
            <Avatar src={modeIcon(mode())} class="w-4 h-4" imageClass="tint-[#2a9d8f] dark:tint-[#2dd4bf]" fallbackDelay={Infinity} accessibilityLabel="" />
            <Text class={`text-[13px] font-medium ${C.ink}`}>{modeName()}</Text>
            <Avatar src={ICONS.chevronDown} class="w-3.5 h-3.5 opacity-50" imageClass="tint-[#1e293b] dark:tint-slate-100" fallbackDelay={Infinity} accessibilityLabel="" />
          </View>
          <If when={Boolean(err())}>
            <Text class="flex-1 min-w-0 truncate text-[12px] font-medium text-[#ef4444]">{err()}</Text>
          </If>
          <If when={!Boolean(err())}>
            <View class="flex-1" />
          </If>
          <View ref={saveMenuAnchor} class="relative shrink-0 flex-row items-stretch">
            <View
              class={`items-center justify-center rounded-l-lg px-5 py-2.5 hover:bg-[#248f82] transition-colors ${busy() ? "opacity-50" : ""}`}
              style={{ backgroundColor: "#2a9d8f" }}
              activeStyle={{ opacity: 0.7 }}
              onPress={() => { if (!busy()) save(false, true); }}
            >
              <Text class="text-[13px] font-medium text-white">{busy() ? "Saving..." : "Save & Close"}</Text>
            </View>
            <View
              class="w-10 items-center justify-center rounded-r-lg border-l border-white/25 hover:bg-[#248f82] transition-colors"
              style={{ backgroundColor: "#2a9d8f" }}
              activeStyle={{ opacity: 0.7 }}
              accessibilityLabel="More save options"
              onPress={() => { saveMenuOpen.set(!saveMenuOpen()); typeMenuOpen.set(false); }}
            >
              <Avatar src={ICONS.chevronDown} class="w-3.5 h-3.5" imageClass="tint-white" fallbackDelay={Infinity} accessibilityLabel="" />
            </View>
          </View>
        </View>
        <View class="flex-1 flex-row gap-6 p-6 pb-20 lg:pb-4 overflow-hidden">
        <View class="flex-1 min-h-0 rounded-2xl overflow-hidden flex-col bg-white dark:bg-[#1e293b] shadow-[0_4px_24px_rgba(0,0,0,0.08)]">
            <View class={`shrink-0 px-6 pt-6 pb-12 ${accent() === "green" ? "bg-linear-to-r from-[#1a6b3c] to-[#27ae60] dark:from-[#145a30] dark:to-[#1e8c4c]" : ""}`} style={accent() === "green" ? undefined : { backgroundColor: acc() }}>
              <View class="flex-row flex-wrap items-start gap-5">
                <View class="flex-row items-center gap-5 flex-1 min-w-[280px]">
                  <View class="w-14 h-14 rounded-full border-2 border-white/30 items-center justify-center bg-white/20 shrink-0">
                    <Avatar src={modeIcon(mode())} class="w-6 h-6" imageClass="tint-white" fallbackDelay={Infinity} accessibilityLabel="" />
                  </View>
                  <View class="gap-1">
                    <If when={mode() === "money_in" || mode() === "money_out"}>
                      <View class="w-full min-w-0">
                        <BooksCombo
                          label={mode() === "money_in" ? "Customer" : "Vendor"}
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
                          onCreate={(query: string) => createParty(mode() === "money_in" ? "customers" : "vendors", query, (ticket) => { payeeTicket.set(ticket); payee.set(query); partyQuery.set(""); })}
                          createLabel={mode() === "money_in" ? "customer" : mode() === "money_out" ? "vendor" : "party"}
                          query={partyQuery()}
                          placeholder={payee() || "Select Party"}
                          group={mode() === "money_in" ? "Customers" : "Vendors"}
                        />
                      </View>
                    </If>
                    <If when={mode() === "journal" || mode() === "transfer"}>
                      <Text class={`text-[24px] font-bold ${ON_ACC}`}>{mode() === "transfer" ? "Transfer" : "Journal"}</Text>
                    </If>
                    <Input
                      accessibilityLabel="Reference"
                      value={reference()}
                      placeholder="Reference Number"
                      onChange={(e: Any) => reference.set(typeof e === "string" ? e : e.value)}
                      class="w-[200px] min-h-11 lg:min-h-0 rounded-md px-2 text-base sm:text-xs text-white placeholder:text-white/50 border-none outline-none bg-black/20 focus:bg-black/30 transition-colors"
                    />
                  </View>
                </View>
                <View class="flex-col items-end gap-1 ml-auto shrink-0">
                  <If when={mode() === "transfer"}>
                    <View class="flex-row items-center justify-end gap-1 border border-white/20 rounded-lg px-3 py-1.5 bg-white/10 focus-within:border-white/40 focus-within:ring-1 focus-within:ring-white/30 transition-colors">
                      <Text class="text-xl font-semibold text-white/60">₱</Text>
                      <TextBox label="Amount" value={amount()} onChange={(v) => { amount.set(v); err.set(""); }} placeholder="0.00" transferAmount />
                    </View>
                  </If>
                  <If when={mode() === "money_in" || mode() === "money_out"}>
                    <View class="flex-row items-center justify-end gap-1 border border-white/20 rounded-lg px-3 py-1.5 bg-white/10 focus-within:border-white/40 focus-within:ring-1 focus-within:ring-white/30 transition-colors">
                      <Text class="text-xl font-semibold text-white/60">₱</Text>
                      <TextBox label="Amount" value={amount()} onChange={(v) => { amount.set(v); err.set(""); }} placeholder="0.00" transferAmount />
                    </View>
                  </If>
                  <If when={mode() === "journal"}>
                    <Text class="text-[24px] font-bold text-white tabular-nums">{shownCents(journalTotals().debit)}</Text>
                  </If>
                  <View class="flex-row items-center justify-end gap-2 mt-1">
                    <BooksDate value={date() || o()?.today || ""} today={o()?.today} onChange={(v) => date.set(v)} variant="header" />
                  </View>
                  <View class="flex-row items-center gap-2 px-3 py-1.5 rounded-lg mt-4 bg-black/20 shadow-sm">
                    <Avatar src={modeIcon(mode())} class="w-3.5 h-3.5" imageClass="tint-white" fallbackDelay={Infinity} accessibilityLabel="" />
                    <Text class="text-[12px] font-semibold text-white">{TYPE_LABEL[mode()] || modeName()}</Text>
                  </View>
                </View>
              </View>
            </View>
            <View class="px-6 shrink-0" style={{ marginTop: -48 }}>
              <View class="flex-row items-center gap-1.5 mb-2 pl-1">
                <Text class="text-[14px] text-white">✎</Text>
                <Text class="text-[12px] font-semibold text-white">Memo</Text>
              </View>
              <View class="rounded-xl border p-4 min-h-16 bg-white dark:bg-slate-800 border-[#e2e8f0] dark:border-slate-700 shadow-sm">
                <View class="border-b border-transparent focus-within:border-[#2a9d8f] transition-colors">
                <Input
                  accessibilityLabel="Memo"
                  value={memo()}
                  placeholder="Add a memo..."
                  onChange={(e: Any) => memo.set(typeof e === "string" ? e : e.value)}
                  class="w-full bg-transparent border-none outline-none pb-1 text-base sm:text-sm text-[#1e293b] dark:text-slate-200 placeholder:text-[#cbd5e1] dark:placeholder:text-slate-500"
                />
                </View>
              </View>
            </View>
            <Scroll class="flex-1 min-h-0">
            <View class="px-6 pt-4 pb-24 lg:pb-4 gap-3.5 bg-[#f8f9fb] dark:bg-slate-800/70 flex-1 flex-col">
              <If when={mode() !== "journal"}>
                <View class="gap-3.5">
                  <If when={mode() === "transfer"}>
                    <View class="flex-col gap-2 md:flex-row md:items-start md:gap-5">
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
                          onCreate={(query: string) => createParty("vendors", query, (ticket) => { fromPerson.set(ticket); fromPartyQuery.set(""); })}
                          createLabel="vendor or customer"
                          placeholder="Party"
                          group="People"
                        />
                      </View>
                      <View class="flex-row items-center justify-center py-1 md:pt-8 md:py-0">
                        <Avatar src={ICONS.arrowDown} class="md:hidden w-5 h-5" imageClass="tint-[#94a3b8]" fallbackDelay={Infinity} accessibilityLabel="" />
                        <Avatar src={ICONS.arrowRight} class="hidden md:block w-5 h-5" imageClass="tint-[#94a3b8]" fallbackDelay={Infinity} accessibilityLabel="" />
                      </View>
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
                          onCreate={(query: string) => createParty("vendors", query, (ticket) => { toPerson.set(ticket); toPartyQuery.set(""); })}
                          createLabel="vendor or customer"
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
                        <View class="gap-2 rounded-xl border border-[#e2e8f0] dark:border-slate-600 bg-white dark:bg-slate-800 p-3 md:p-0 md:py-3 md:rounded-none md:border-0 md:border-b md:border-[#f0f2f5] md:dark:border-slate-800 md:bg-transparent md:dark:bg-transparent md:last:border-b-0">
                          {/* Desktop (md+): row 1 = # + description + amount */}
                          <View class="hidden md:flex flex-row items-center gap-2">
                            <Text class="w-5 shrink-0 text-[11px] font-semibold text-center text-[#94a3b8] dark:text-slate-500">{String(index() + 1)}</Text>
                            <View class="flex-[2] lg:flex-1 min-w-0">
                              <Input accessibilityLabel="Allocation description" value={line().memo()} onChange={(e: Any) => line().memo.set(typeof e === "string" ? e : e.value)} placeholder="Add description for this item..." class="w-full min-w-0 bg-white dark:bg-slate-800 border border-[#e2e8f0] dark:border-slate-600 rounded-md px-2 py-1.5 text-base sm:text-sm text-[#1e293b] dark:text-slate-200 placeholder:text-[#cbd5e1] dark:placeholder:text-slate-600 outline-none focus:border-[#2a9d8f] dark:focus:border-[#2a9d8f] transition-colors" />
                            </View>
                            <View class="flex-1 min-w-0 lg:flex-none lg:w-20">
                              <Input accessibilityLabel="Allocation amount" value={line().amount()} onChange={(e: Any) => { line().amount.set(typeof e === "string" ? e : e.value); err.set(""); }} placeholder="₱0.00" class="w-full text-right bg-white dark:bg-slate-800 border border-[#e2e8f0] dark:border-slate-600 rounded-md px-2 py-1.5 text-base sm:text-sm tabular-nums text-[#1e293b] dark:text-slate-200 placeholder:text-[#cbd5e1] dark:placeholder:text-slate-600 outline-none focus:border-[#2a9d8f] dark:focus:border-[#2a9d8f] transition-colors" />
                            </View>
                          </View>
                          {/* Desktop (md+): combos 2×2 at md–lg, one row at lg+, icons on the right */}
                          <View class="hidden md:flex flex-row items-center gap-2 pl-7">
                            <View class="grid grid-cols-2 lg:grid-cols-4 gap-2 flex-1 min-w-0">
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
                                trigger="chip"
                                icon="category"
                              />
                              <BooksCombo
                                label="Party"
                                value={line().person()}
                                options={() => partyOptions()}
                                onPick={(person: Any) => { line().person.set(person.ticket || ""); line().partyQuery.set(""); }}
                                query={() => line().partyQuery()}
                                onQuery={(query: string) => line().partyQuery.set(query)}
                                onCreate={(query: string) => createParty(partyKindFor(line().category(), mode() === "money_in" ? "customers" : "vendors"), query, (ticket) => { line().person.set(ticket); line().partyQuery.set(""); })}
                                createLabel={mode() === "money_in" ? "customer" : "vendor"}
                                placeholder="Party"
                                group={mode() === "money_in" ? "Customers" : "Vendors"}
                                trigger="chip"
                                icon="party"
                              />
                              <BooksCombo label="Department" value={line().department()} options={() => tagOptions(departments, "Departments")} onPick={(tag: Any) => { line().department.set(tag.ticket || ""); line().deptQuery.set(""); }} query={() => line().deptQuery()} onQuery={(query: string) => line().deptQuery.set(query)} onCreate={(query: string) => createTag("department", query, (ticket) => { line().department.set(ticket); line().deptQuery.set(""); })} createLabel="department" placeholder="Department" group="Departments" trigger="chip" icon="department" />
                              <BooksCombo label="Location" value={line().location()} options={() => tagOptions(locations, "Locations")} onPick={(tag: Any) => { line().location.set(tag.ticket || ""); line().locQuery.set(""); }} query={() => line().locQuery()} onQuery={(query: string) => line().locQuery.set(query)} onCreate={(query: string) => createTag("location", query, (ticket) => { line().location.set(ticket); line().locQuery.set(""); })} createLabel="location" placeholder="Location" group="Locations" trigger="chip" icon="location" />
                            </View>
                            <View class="flex-row items-center gap-0.5 shrink-0 ml-auto">
                              <If when={allocations().length > 1}>
                                <View
                                  accessibilityLabel="Delete allocation"
                                  class="w-11 h-11 lg:w-8 lg:h-8 rounded items-center justify-center hover:bg-[#fef2f2] dark:hover:bg-slate-700 transition-colors"
                                  activeStyle={{ opacity: 0.7 }}
                                  onPress={() => { allocations.set(allocations().filter((_: Any, i: number) => i !== index())); err.set(""); }}
                                >
                                  <Text class="text-[15px] text-[#94a3b8] hover:text-[#f87171]">×</Text>
                                </View>
                              </If>
                              <View
                                accessibilityLabel="Copy allocation"
                                class="w-11 h-11 lg:w-8 lg:h-8 rounded items-center justify-center hover:bg-[#f1f5f9] dark:hover:bg-slate-700 transition-colors"
                                activeStyle={{ opacity: 0.7 }}
                                onPress={() => {
                                  const copy = allocationLine();
                                  copy.category.set(line().category()); copy.query.set(line().query());
                                  copy.amount.set(line().amount());
                                  copy.person.set(line().person()); copy.partyQuery.set(line().partyQuery());
                                  copy.memo.set(line().memo());
                                  copy.department.set(line().department()); copy.deptQuery.set(line().deptQuery());
                                  copy.location.set(line().location()); copy.locQuery.set(line().locQuery());
                                  const next = [...allocations()];
                                  next.splice(index() + 1, 0, copy);
                                  allocations.set(next);
                                  err.set("");
                                }}
                              >
                                <Text class="text-[15px] text-[#94a3b8] hover:text-[#475569] dark:hover:text-slate-300">⧉</Text>
                              </View>
                              <View
                                accessibilityLabel="Add allocation below"
                                class="w-11 h-11 lg:w-8 lg:h-8 rounded items-center justify-center hover:bg-[#f0fdfa] dark:hover:bg-slate-700 transition-colors"
                                activeStyle={{ opacity: 0.7 }}
                                onPress={() => { const next = [...allocations()]; next.splice(index() + 1, 0, allocationLine()); allocations.set(next); err.set(""); }}
                              >
                                <Text class="text-[16px] font-semibold text-[#94a3b8] hover:text-[#2a9d8f]">+</Text>
                              </View>
                            </View>
                          </View>
                          {/* Mobile (below md): stacked card */}
                          <View class="md:hidden flex-col">
                            <View class="flex-row items-center gap-2 pb-2">
                              <View class="w-6 h-6 rounded-full bg-[#2a9d8f] items-center justify-center shrink-0">
                                <Text class="text-[11px] font-bold text-white">{String(index() + 1)}</Text>
                              </View>
                              <View class="flex-1 min-w-0">
                                <Input accessibilityLabel="Allocation description" value={line().memo()} onChange={(e: Any) => line().memo.set(typeof e === "string" ? e : e.value)} placeholder="Line description..." class="w-full bg-transparent border-none outline-none px-1 py-1 text-base sm:text-sm text-[#1e293b] dark:text-slate-200 placeholder:text-[#94a3b8] dark:placeholder:text-slate-500" />
                              </View>
                            </View>
                            <View class="gap-1 pb-2">
                              <Text class={`text-[10px] font-medium uppercase tracking-wider ${C.dim}`}>Amount</Text>
                              <Input accessibilityLabel="Allocation amount" value={line().amount()} onChange={(e: Any) => { line().amount.set(typeof e === "string" ? e : e.value); err.set(""); }} placeholder="₱0.00" class="w-full text-right bg-white dark:bg-slate-800 border border-[#e2e8f0] dark:border-slate-600 rounded-lg px-3 py-2 text-base sm:text-sm tabular-nums text-[#1e293b] dark:text-slate-200 placeholder:text-[#cbd5e1] dark:placeholder:text-slate-600 outline-none focus:border-[#2a9d8f] dark:focus:border-[#2a9d8f] focus:ring-1 focus:ring-[#2a9d8f] transition-colors" />
                            </View>
                            <View class="gap-2 pb-2">
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
                                trigger="chip"
                                icon="category"
                              />
                            </View>
                            <View class="gap-2 pb-2">
                              <BooksCombo
                                label="Party"
                                value={line().person()}
                                options={() => partyOptions()}
                                onPick={(person: Any) => { line().person.set(person.ticket || ""); line().partyQuery.set(""); }}
                                query={() => line().partyQuery()}
                                onQuery={(query: string) => line().partyQuery.set(query)}
                                onCreate={(query: string) => createParty(partyKindFor(line().category(), mode() === "money_in" ? "customers" : "vendors"), query, (ticket) => { line().person.set(ticket); line().partyQuery.set(""); })}
                                createLabel={mode() === "money_in" ? "customer" : "vendor"}
                                placeholder="Party"
                                group={mode() === "money_in" ? "Customers" : "Vendors"}
                                trigger="chip"
                                icon="party"
                              />
                            </View>
                            <View class="gap-2 pb-2">
                              <BooksCombo label="Department" value={line().department()} options={() => tagOptions(departments, "Departments")} onPick={(tag: Any) => { line().department.set(tag.ticket || ""); line().deptQuery.set(""); }} query={() => line().deptQuery()} onQuery={(query: string) => line().deptQuery.set(query)} onCreate={(query: string) => createTag("department", query, (ticket) => { line().department.set(ticket); line().deptQuery.set(""); })} createLabel="department" placeholder="Department" group="Departments" trigger="chip" icon="department" />
                            </View>
                            <View class="gap-2 pb-2">
                              <BooksCombo label="Location" value={line().location()} options={() => tagOptions(locations, "Locations")} onPick={(tag: Any) => { line().location.set(tag.ticket || ""); line().locQuery.set(""); }} query={() => line().locQuery()} onQuery={(query: string) => line().locQuery.set(query)} onCreate={(query: string) => createTag("location", query, (ticket) => { line().location.set(ticket); line().locQuery.set(""); })} createLabel="location" placeholder="Location" group="Locations" trigger="chip" icon="location" />
                            </View>
                            <View class="flex-row items-center gap-1 pb-1">
                              <If when={allocations().length > 1}>
                                <View
                                  accessibilityLabel="Delete allocation"
                                  class="w-8 h-8 min-h-11 min-w-11 rounded items-center justify-center hover:bg-[#fef2f2] dark:hover:bg-slate-700 transition-colors"
                                  activeStyle={{ opacity: 0.7 }}
                                  onPress={() => { allocations.set(allocations().filter((_: Any, i: number) => i !== index())); err.set(""); }}
                                >
                                  <Text class="text-[15px] text-[#94a3b8] hover:text-[#f87171]">×</Text>
                                </View>
                              </If>
                              <View
                                accessibilityLabel="Copy allocation"
                                class="w-8 h-8 min-h-11 min-w-11 rounded items-center justify-center hover:bg-[#f1f5f9] dark:hover:bg-slate-700 transition-colors"
                                activeStyle={{ opacity: 0.7 }}
                                onPress={() => {
                                  const copy = allocationLine();
                                  copy.category.set(line().category()); copy.query.set(line().query());
                                  copy.amount.set(line().amount());
                                  copy.person.set(line().person()); copy.partyQuery.set(line().partyQuery());
                                  copy.memo.set(line().memo());
                                  copy.department.set(line().department()); copy.deptQuery.set(line().deptQuery());
                                  copy.location.set(line().location()); copy.locQuery.set(line().locQuery());
                                  const next = [...allocations()];
                                  next.splice(index() + 1, 0, copy);
                                  allocations.set(next);
                                  err.set("");
                                }}
                              >
                                <Text class="text-[15px] text-[#94a3b8] hover:text-[#475569] dark:hover:text-slate-300">⧉</Text>
                              </View>
                              <View
                                accessibilityLabel="Add allocation below"
                                class="w-8 h-8 min-h-11 min-w-11 rounded items-center justify-center hover:bg-[#f0fdfa] dark:hover:bg-slate-700 transition-colors"
                                activeStyle={{ opacity: 0.7 }}
                                onPress={() => { const next = [...allocations()]; next.splice(index() + 1, 0, allocationLine()); allocations.set(next); err.set(""); }}
                              >
                                <Text class="text-[16px] font-semibold text-[#94a3b8] hover:text-[#2a9d8f]">+</Text>
                              </View>
                            </View>
                          </View>
                        </View>
                      )}
                    </Each>
                    <If when={allocationRemaining() === 0n && centsOf(amount()) > 0n}>
                      <Text class={`text-[12px] font-semibold ${C.green}`}>Allocations match the amount.</Text>
                    </If>
                  </If>
                </View>
              </If>
              <If when={mode() === "journal"}>
                <View class="gap-0">
                  <View class="flex md:hidden lg:flex flex-row items-center justify-between gap-2 pb-1 mb-2">
                    <View class="flex-row items-center gap-1.5 shrink-0"><Avatar src={ICONS.list} class="w-3.5 h-3.5" imageClass="tint-[#475569] dark:tint-slate-400" fallbackDelay={Infinity} accessibilityLabel="" /><Text class="text-[12px] font-semibold text-[#475569] dark:text-slate-400">Lines</Text></View>
                    <View class="hidden lg:flex lg:flex-row gap-8 shrink-0">
                      <Text class="w-20 text-right text-[12px] font-medium text-[#64748b] dark:text-slate-400">Debit</Text>
                      <Text class="w-20 text-right text-[12px] font-medium text-[#64748b] dark:text-slate-400">Credit</Text>
                    </View>
                  </View>
                  <View class="hidden md:flex lg:hidden flex-row items-center gap-2 mb-3">
                    <Text class="w-5 shrink-0" />
                    <View class="flex-[2] min-w-0">
                      <View class="flex-row items-center gap-1.5"><Avatar src={ICONS.list} class="w-3.5 h-3.5" imageClass="tint-[#475569] dark:tint-slate-400" fallbackDelay={Infinity} accessibilityLabel="" /><Text class="text-[12px] font-semibold text-[#475569] dark:text-slate-400">Lines</Text></View>
                    </View>
                    <Text class="flex-1 min-w-0 text-left text-[12px] font-medium text-[#64748b] dark:text-slate-400">Debit</Text>
                    <Text class="flex-1 min-w-0 text-left text-[12px] font-medium text-[#64748b] dark:text-slate-400">Credit</Text>
                  </View>
                  <Each of={jl()}>
                    {(l: Any, i: Any) => (
                      <View class="gap-2 rounded-xl border border-[#e2e8f0] dark:border-slate-700 bg-[#fafbfc] dark:bg-slate-800/60 shadow-[0_1px_3px_rgba(0,0,0,0.04)] mb-3 md:rounded-none md:border-0 md:border-b md:border-[#f0f2f5] md:dark:border-slate-700 md:bg-transparent md:dark:bg-transparent md:shadow-none md:px-0 md:py-3 md:mb-0">
                        {/* Tablet (md–lg): desc flex-[2] + flexible amounts */}
                        <View class="hidden md:flex lg:hidden md:flex-col gap-2">
                        <View class="flex-row gap-2 items-center">
                          <Text class="w-5 shrink-0 text-[11px] font-semibold text-center text-[#94a3b8] dark:text-slate-500">{String(i() + 1)}</Text>
                          <View class="flex-[2] min-w-0">
                            <Input accessibilityLabel="Line description" value={l().memo()} onChange={(e: Any) => l().memo.set(typeof e === "string" ? e : e.value)} placeholder="Add description for this item..." class="w-full min-w-0 bg-white dark:bg-slate-800 border border-[#e2e8f0] dark:border-slate-600 rounded-md px-2 py-1.5 text-base sm:text-sm text-[#1e293b] dark:text-slate-200 placeholder:text-[#cbd5e1] dark:placeholder:text-slate-600 outline-none focus:border-[#2a9d8f] dark:focus:border-[#2a9d8f] transition-colors" />
                          </View>
                          <View class="flex-1 min-w-0">
                            <Input accessibilityLabel="Debit" value={l().debit()} onChange={(e: Any) => { l().debit.set(typeof e === "string" ? e : e.value); err.set(""); }} placeholder="₱0.00" class="w-full text-right bg-white dark:bg-slate-800 border border-[#e2e8f0] dark:border-slate-600 rounded-md px-2 py-1.5 text-base sm:text-sm tabular-nums text-[#1e293b] dark:text-slate-200 placeholder:text-[#cbd5e1] dark:placeholder:text-slate-600 outline-none focus:border-[#2a9d8f] dark:focus:border-[#2a9d8f] transition-colors" />
                          </View>
                          <View class="flex-1 min-w-0">
                            <Input accessibilityLabel="Credit" value={l().credit()} onChange={(e: Any) => { l().credit.set(typeof e === "string" ? e : e.value); err.set(""); }} placeholder="₱0.00" class="w-full text-right bg-white dark:bg-slate-800 border border-[#e2e8f0] dark:border-slate-600 rounded-md px-2 py-1.5 text-base sm:text-sm tabular-nums text-[#1e293b] dark:text-slate-200 placeholder:text-[#cbd5e1] dark:placeholder:text-slate-600 outline-none focus:border-[#2a9d8f] dark:focus:border-[#2a9d8f] transition-colors" />
                          </View>
                        </View>
                        <View class="flex-row items-center gap-2 pl-7">
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
                              trigger="chip"
                              icon="category"
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
                              onCreate={(query: string) => createParty(partyKindFor(l().account(), "vendors"), query, (ticket) => { l().person.set(ticket); l().partyQuery.set(""); })}
                              createLabel="vendor or customer"
                              placeholder="Party"
                              group="People"
                              trigger="chip"
                              icon="party"
                            />
                          </View>
                        </View>
                        <View class="flex-row items-center gap-2 pl-7">
                          <View class="flex-1">
                            <BooksCombo label="Department" value={l().department()} options={() => tagOptions(departments, "Departments")} onPick={(tag: Any) => { l().department.set(tag.ticket || ""); l().deptQuery.set(""); }} query={() => l().deptQuery()} onQuery={(query: string) => l().deptQuery.set(query)} onCreate={(query: string) => createTag("department", query, (ticket) => { l().department.set(ticket); l().deptQuery.set(""); })} createLabel="department" placeholder="Department" group="Departments" trigger="chip" icon="department" />
                          </View>
                          <View class="flex-1">
                            <BooksCombo label="Location" value={l().location()} options={() => tagOptions(locations, "Locations")} onPick={(tag: Any) => { l().location.set(tag.ticket || ""); l().locQuery.set(""); }} query={() => l().locQuery()} onQuery={(query: string) => l().locQuery.set(query)} onCreate={(query: string) => createTag("location", query, (ticket) => { l().location.set(ticket); l().locQuery.set(""); })} createLabel="location" placeholder="Location" group="Locations" trigger="chip" icon="location" />
                          </View>
                          <View class="flex-row items-center gap-1 shrink-0">
                            <View
                              accessibilityLabel="Delete line"
                              class={`w-11 h-11 rounded items-center justify-center transition-colors ${jl().length <= 2 ? "opacity-0" : "hover:bg-[#fef2f2] dark:hover:bg-red-900/20"}`}
                              activeStyle={jl().length > 2 ? { opacity: 0.7 } : undefined}
                              onPress={() => { if (jl().length > 2) { jl.set(jl().filter((_: Any, j: number) => j !== i())); err.set(""); } }}
                            >
                              <Text class="text-[15px] text-[#94a3b8] hover:text-[#f87171]">🗑</Text>
                            </View>
                            <View
                              accessibilityLabel="Copy line"
                              class="w-11 h-11 rounded items-center justify-center hover:bg-[#f1f5f9] dark:hover:bg-slate-700 transition-colors"
                              activeStyle={{ opacity: 0.7 }}
                              onPress={() => {
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
                              }}
                            >
                              <Text class="text-[15px] text-[#94a3b8] hover:text-[#475569] dark:hover:text-slate-300">⧉</Text>
                            </View>
                            <View
                              accessibilityLabel="Add line below"
                              class="w-11 h-11 rounded items-center justify-center hover:bg-[#f0fdfa] dark:hover:bg-slate-700 transition-colors"
                              activeStyle={{ opacity: 0.7 }}
                              onPress={() => { const next = [...jl()]; next.splice(i() + 1, 0, journalLine()); jl.set(next); err.set(""); }}
                            >
                              <Text class="text-[16px] font-semibold text-[#94a3b8] hover:text-[#2a9d8f]">+</Text>
                            </View>
                          </View>
                        </View>
                        </View>
                        {/* Desktop (lg+): two compact rows */}
                        <View class="hidden lg:flex lg:flex-col gap-2">
                        <View class="flex-row gap-2 items-center">
                          <Text class="w-5 shrink-0 text-[11px] font-semibold text-center text-[#94a3b8] dark:text-slate-500">{String(i() + 1)}</Text>
                          <View class="flex-1 min-w-0">
                            <Input accessibilityLabel="Line description" value={l().memo()} onChange={(e: Any) => l().memo.set(typeof e === "string" ? e : e.value)} placeholder="Add description for this item..." class="w-full min-w-0 bg-white dark:bg-slate-800 border border-[#e2e8f0] dark:border-slate-600 rounded-md px-2 py-1.5 text-base sm:text-sm text-[#1e293b] dark:text-slate-200 placeholder:text-[#cbd5e1] dark:placeholder:text-slate-600 outline-none focus:border-[#2a9d8f] dark:focus:border-[#2a9d8f] transition-colors" />
                          </View>
                          <View class="w-20 shrink-0">
                            <Input accessibilityLabel="Debit" value={l().debit()} onChange={(e: Any) => { l().debit.set(typeof e === "string" ? e : e.value); err.set(""); }} placeholder="₱0.00" class="w-full text-right bg-white dark:bg-slate-800 border border-[#e2e8f0] dark:border-slate-600 rounded-md px-2 py-1.5 text-base sm:text-sm tabular-nums text-[#1e293b] dark:text-slate-200 placeholder:text-[#cbd5e1] dark:placeholder:text-slate-600 outline-none focus:border-[#2a9d8f] dark:focus:border-[#2a9d8f] transition-colors" />
                          </View>
                          <View class="w-20 shrink-0">
                            <Input accessibilityLabel="Credit" value={l().credit()} onChange={(e: Any) => { l().credit.set(typeof e === "string" ? e : e.value); err.set(""); }} placeholder="₱0.00" class="w-full text-right bg-white dark:bg-slate-800 border border-[#e2e8f0] dark:border-slate-600 rounded-md px-2 py-1.5 text-base sm:text-sm tabular-nums text-[#1e293b] dark:text-slate-200 placeholder:text-[#cbd5e1] dark:placeholder:text-slate-600 outline-none focus:border-[#2a9d8f] dark:focus:border-[#2a9d8f] transition-colors" />
                          </View>
                        </View>
                        <View class="flex-row items-center gap-2 pl-7">
                          <View class="grid grid-cols-4 gap-2 flex-1 min-w-0">
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
                              trigger="chip"
                              icon="category"
                            />
                            <BooksCombo
                              label="Party"
                              value={l().person()}
                              options={() => partyOptions()}
                              onPick={(person: Any) => { l().person.set(person.ticket || ""); l().partyQuery.set(""); }}
                              query={() => l().partyQuery()}
                              onQuery={(query: string) => l().partyQuery.set(query)}
                              onCreate={(query: string) => createParty(partyKindFor(l().account(), "vendors"), query, (ticket) => { l().person.set(ticket); l().partyQuery.set(""); })}
                              createLabel="vendor or customer"
                              placeholder="Party"
                              group="People"
                              trigger="chip"
                              icon="party"
                            />
                            <BooksCombo label="Department" value={l().department()} options={() => tagOptions(departments, "Departments")} onPick={(tag: Any) => { l().department.set(tag.ticket || ""); l().deptQuery.set(""); }} query={() => l().deptQuery()} onQuery={(query: string) => l().deptQuery.set(query)} onCreate={(query: string) => createTag("department", query, (ticket) => { l().department.set(ticket); l().deptQuery.set(""); })} createLabel="department" placeholder="Department" group="Departments" trigger="chip" icon="department" />
                            <BooksCombo label="Location" value={l().location()} options={() => tagOptions(locations, "Locations")} onPick={(tag: Any) => { l().location.set(tag.ticket || ""); l().locQuery.set(""); }} query={() => l().locQuery()} onQuery={(query: string) => l().locQuery.set(query)} onCreate={(query: string) => createTag("location", query, (ticket) => { l().location.set(ticket); l().locQuery.set(""); })} createLabel="location" placeholder="Location" group="Locations" trigger="chip" icon="location" />
                          </View>
                          <View class="flex-row items-center gap-1 shrink-0">
                            <View
                              accessibilityLabel="Delete line"
                              class={`w-11 h-11 lg:w-8 lg:h-8 rounded items-center justify-center transition-colors ${jl().length <= 2 ? "opacity-0" : "hover:bg-[#fef2f2] dark:hover:bg-red-900/20"}`}
                              activeStyle={jl().length > 2 ? { opacity: 0.7 } : undefined}
                              onPress={() => { if (jl().length > 2) { jl.set(jl().filter((_: Any, j: number) => j !== i())); err.set(""); } }}
                            >
                              <Text class="text-[15px] text-[#94a3b8] hover:text-[#f87171]">🗑</Text>
                            </View>
                            <View
                              accessibilityLabel="Copy line"
                              class="w-11 h-11 lg:w-8 lg:h-8 rounded items-center justify-center hover:bg-[#f1f5f9] dark:hover:bg-slate-700 transition-colors"
                              activeStyle={{ opacity: 0.7 }}
                              onPress={() => {
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
                              }}
                            >
                              <Text class="text-[15px] text-[#94a3b8] hover:text-[#475569] dark:hover:text-slate-300">⧉</Text>
                            </View>
                            <View
                              accessibilityLabel="Add line below"
                              class="w-11 h-11 lg:w-8 lg:h-8 rounded items-center justify-center hover:bg-[#f0fdfa] dark:hover:bg-slate-700 transition-colors"
                              activeStyle={{ opacity: 0.7 }}
                              onPress={() => { const next = [...jl()]; next.splice(i() + 1, 0, journalLine()); jl.set(next); err.set(""); }}
                            >
                              <Text class="text-[16px] font-semibold text-[#94a3b8] hover:text-[#2a9d8f]">+</Text>
                            </View>
                          </View>
                        </View>
                        </View>
                        {/* Mobile (below md): stacked card layout */}
                        <View class="md:hidden flex-col">
                          <View class="flex-row items-center gap-2 px-3 pt-3 pb-2">
                            <View class="w-6 h-6 rounded-full bg-[#2a9d8f] items-center justify-center shrink-0">
                              <Text class="text-[11px] font-bold text-white">{String(i() + 1)}</Text>
                            </View>
                            <View class="flex-1 min-w-0">
                              <Input
                                accessibilityLabel="Line description"
                                value={l().memo()}
                                placeholder="Line description..."
                                onChange={(e: Any) => l().memo.set(typeof e === "string" ? e : e.value)}
                                class="w-full bg-transparent border-none outline-none px-1 py-1 text-base sm:text-sm text-[#1e293b] dark:text-slate-200 placeholder:text-[#94a3b8] dark:placeholder:text-slate-500"
                              />
                            </View>
                          </View>
                          <View class="grid grid-cols-2 gap-2 px-3 pb-2">
                            <View>
                              <Text class="text-[10px] font-medium text-[#94a3b8] dark:text-slate-500 uppercase mb-1">Debit</Text>
                              <Input accessibilityLabel="Debit" value={l().debit()} onChange={(e: Any) => { l().debit.set(typeof e === "string" ? e : e.value); err.set(""); }} placeholder="₱0.00" class="w-full text-right bg-white dark:bg-slate-800 border border-[#e2e8f0] dark:border-slate-600 rounded-lg px-3 py-2 text-base sm:text-sm tabular-nums text-[#1e293b] dark:text-slate-200 placeholder:text-[#cbd5e1] dark:placeholder:text-slate-600 outline-none focus:border-[#2a9d8f] dark:focus:border-[#2a9d8f] focus:ring-1 focus:ring-[#2a9d8f] transition-colors" />
                            </View>
                            <View>
                              <Text class="text-[10px] font-medium text-[#94a3b8] dark:text-slate-500 uppercase mb-1">Credit</Text>
                              <Input accessibilityLabel="Credit" value={l().credit()} onChange={(e: Any) => { l().credit.set(typeof e === "string" ? e : e.value); err.set(""); }} placeholder="₱0.00" class="w-full text-right bg-white dark:bg-slate-800 border border-[#e2e8f0] dark:border-slate-600 rounded-lg px-3 py-2 text-base sm:text-sm tabular-nums text-[#1e293b] dark:text-slate-200 placeholder:text-[#cbd5e1] dark:placeholder:text-slate-600 outline-none focus:border-[#2a9d8f] dark:focus:border-[#2a9d8f] focus:ring-1 focus:ring-[#2a9d8f] transition-colors" />
                            </View>
                          </View>
                          <View class="px-3 pb-2">
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
                              trigger="chip"
                              icon="category"
                            />
                          </View>
                          <View class="px-3 pb-2">
                            <BooksCombo
                              label="Party"
                              value={l().person()}
                              options={() => partyOptions()}
                              onPick={(person: Any) => { l().person.set(person.ticket || ""); l().partyQuery.set(""); }}
                              query={() => l().partyQuery()}
                              onQuery={(query: string) => l().partyQuery.set(query)}
                              onCreate={(query: string) => createParty(partyKindFor(l().account(), "vendors"), query, (ticket) => { l().person.set(ticket); l().partyQuery.set(""); })}
                              createLabel="vendor or customer"
                              placeholder="Party"
                              group="People"
                              trigger="chip"
                              icon="party"
                            />
                          </View>
                          <View class="px-3 pb-2">
                            <BooksCombo label="Department" value={l().department()} options={() => tagOptions(departments, "Departments")} onPick={(tag: Any) => { l().department.set(tag.ticket || ""); l().deptQuery.set(""); }} query={() => l().deptQuery()} onQuery={(query: string) => l().deptQuery.set(query)} onCreate={(query: string) => createTag("department", query, (ticket) => { l().department.set(ticket); l().deptQuery.set(""); })} createLabel="department" placeholder="Department" group="Departments" trigger="chip" icon="department" />
                          </View>
                          <View class="flex-row items-center gap-2 px-3 pb-3">
                            <View class="flex-1 min-w-0">
                              <BooksCombo label="Location" value={l().location()} options={() => tagOptions(locations, "Locations")} onPick={(tag: Any) => { l().location.set(tag.ticket || ""); l().locQuery.set(""); }} query={() => l().locQuery()} onQuery={(query: string) => l().locQuery.set(query)} onCreate={(query: string) => createTag("location", query, (ticket) => { l().location.set(ticket); l().locQuery.set(""); })} createLabel="location" placeholder="Location" group="Locations" trigger="chip" icon="location" />
                            </View>
                            <View class="flex-row items-center gap-0.5 shrink-0">
                              <If when={jl().length > 2}>
                                <View
                                  accessibilityLabel="Delete line"
                                  class="w-11 h-11 rounded items-center justify-center text-[#94a3b8] hover:bg-[#fef2f2] dark:hover:bg-red-900/20 transition-colors"
                                  activeStyle={{ opacity: 0.7 }}
                                  onPress={() => { jl.set(jl().filter((_: Any, j: number) => j !== i())); err.set(""); }}
                                >
                                  <Text class="text-[15px] text-[#94a3b8] hover:text-[#f87171]">🗑</Text>
                                </View>
                              </If>
                              <View
                                accessibilityLabel="Copy line"
                                class="w-11 h-11 rounded items-center justify-center hover:bg-[#f1f5f9] dark:hover:bg-slate-700 transition-colors"
                                activeStyle={{ opacity: 0.7 }}
                                onPress={() => {
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
                                }}
                              >
                                <Text class="text-[15px] text-[#94a3b8] hover:text-[#475569] dark:hover:text-slate-300">⧉</Text>
                              </View>
                              <View
                                accessibilityLabel="Add line below"
                                class="w-11 h-11 rounded items-center justify-center hover:bg-[#f0fdfa] dark:hover:bg-slate-700 transition-colors"
                                activeStyle={{ opacity: 0.7 }}
                                onPress={() => { const next = [...jl()]; next.splice(i() + 1, 0, journalLine()); jl.set(next); err.set(""); }}
                              >
                                <Text class="text-[15px] font-semibold text-[#94a3b8] hover:text-[#2a9d8f]">+</Text>
                              </View>
                            </View>
                          </View>
                        </View>
                      </View>
                    )}
                  </Each>
                </View>
              </If>
            </View>
            </Scroll>
            <If when={mode() === "journal"}>
              <View class={`shrink-0 px-6 py-3 border-t flex-col md:flex-row md:items-center justify-between gap-1 md:gap-4 ${C.line}`}>
                <View class="flex-row items-center gap-1.5 min-w-0 shrink">
                  <Avatar src={journalTotals().balanced ? ICONS.check : ICONS.circleX} class="w-3.5 h-3.5" imageClass={journalTotals().balanced ? "tint-[#14b8a6]" : "tint-[#ef4444]"} fallbackDelay={Infinity} accessibilityLabel="" />
                  <Text class={`text-xs font-medium ${journalTotals().balanced ? "text-[#14b8a6]" : "text-[#ef4444]"}`}>
                    {shownCents(journalTotals().debit)}
                  </Text>
                  <If when={!journalTotals().balanced}>
                    <Text class={`text-[12px] ${C.dim}`}>{`Difference ${shownCents(journalTotals().difference)}`}</Text>
                  </If>
                </View>
                <View class="flex-row gap-6 justify-end">
                  <View class="flex-row gap-1">
                    <Text class="text-xs text-[#94a3b8] dark:text-slate-500">Debit</Text>
                    <Text class="text-xs font-semibold text-[#1e293b] dark:text-white tabular-nums">{shownCents(journalTotals().debit)}</Text>
                  </View>
                  <View class="flex-row gap-1">
                    <Text class="text-xs text-[#94a3b8] dark:text-slate-500">Credit</Text>
                    <Text class="text-xs font-semibold text-[#1e293b] dark:text-white tabular-nums">{shownCents(journalTotals().credit)}</Text>
                  </View>
                </View>
              </View>
            </If>
        </View>
        <SidePanel
          open={entrySideSheet}
          onBack={() => entrySideSheet.set(false)}
          body={() => <EntrySide />}
          fill
        />
        <If when={!entrySideSheet()}>
          <PanelFab
            label="Open attachments and chat"
            icon="paperclip"
            badge={() => entryStaged().length}
            onPress={() => entrySideSheet.set(true)}
          />
        </If>
        <If when={typeMenuOpen()}>
          <Overlay
            anchor={typeMenuAnchor}
            placement="bottom"
            align="start"
            offset={2}
            margin={8}
            onPressOutside={() => typeMenuOpen.set(false)}
            onCancel={() => typeMenuOpen.set(false)}
          >
          <View class={`w-48 rounded-lg border py-1 shadow-lg bg-white dark:bg-slate-800 ${C.line}`}>
            <Each of={MODES}>
              {(m: Any) => (
                <View
                  class="flex-row items-center gap-2 px-3 py-1.5 hover:bg-[#f1f5f9] dark:hover:bg-slate-800 transition-colors"
                  activeStyle={{ opacity: 0.7 }}
                  onPress={() => { typeMenuOpen.set(false); switchMode(m()[0]); }}
                >
                  <Avatar src={modeIcon(m()[0])} class="w-4 h-4" imageClass={mode() === m()[0] ? "tint-[#2a9d8f]" : "tint-[#64748b] dark:tint-slate-400"} fallbackDelay={Infinity} accessibilityLabel="" />
                  <Text class={`text-xs ${mode() === m()[0] ? "text-[#2a9d8f] dark:text-[#2a9d8f]" : "text-[#374151] dark:text-slate-200"}`}>{m()[1]}</Text>
                  <View class="flex-1" />
                  <If when={mode() === m()[0]}>
                    <Avatar src={ICONS.check} class="w-3.5 h-3.5" imageClass="tint-[#2a9d8f] dark:tint-[#2a9d8f]" fallbackDelay={Infinity} accessibilityLabel="" />
                  </If>
                </View>
              )}
            </Each>
          </View>
          </Overlay>
        </If>
        <If when={saveMenuOpen()}>
          <Overlay
            anchor={saveMenuAnchor}
            placement="bottom"
            align="end"
            offset={2}
            margin={8}
            onPressOutside={() => saveMenuOpen.set(false)}
            onCancel={() => saveMenuOpen.set(false)}
          >
          <View class={`w-40 rounded-lg border py-1 shadow-lg ${C.panel} ${C.line}`}>
            <View class="px-3 py-2 rounded-t-lg hover:bg-[#f9fafb] dark:hover:bg-slate-800 transition-colors" activeStyle={{ opacity: 0.7 }} onPress={() => { saveMenuOpen.set(false); if (!busy()) save(false, false); }}>
              <Text class="text-xs text-[#374151] dark:text-slate-200">Save</Text>
            </View>
            <View class="px-3 py-2 hover:bg-[#f9fafb] dark:hover:bg-slate-800 transition-colors" activeStyle={{ opacity: 0.7 }} onPress={() => { saveMenuOpen.set(false); if (!busy()) save(false, false, true); }}>
              <Text class="text-xs text-[#374151] dark:text-slate-200">Save & New</Text>
            </View>
            <View class={`px-3 py-2 border-t rounded-b-lg hover:bg-[#f9fafb] dark:hover:bg-slate-800 transition-colors ${C.line}`} activeStyle={{ opacity: 0.7 }} onPress={() => { saveMenuOpen.set(false); if (!busy()) save(true, true); }}>
              <Text class="text-xs text-[#374151] dark:text-slate-200">Post</Text>
            </View>
          </View>
          </Overlay>
        </If>
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
