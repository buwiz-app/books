// The Books window: Home, then one page at a time with a way back (no rail
// beside the page), the entry drawer and the full-page new entry. The Ledger row opens the ledger's native screen
// instead of a page here. First run makes the book (cash basis unless
// the owner picks accrual, ADR 0008).

const PAGES: { id: string; label: string; sub?: string; group?: string; hidden?: boolean }[] = [
  { id: "today", label: "Home" },
  { id: "inbox", label: "Inbox", sub: "Drafts waiting to be booked", group: "Money" },
  { id: "transactions", label: "Transactions", sub: "Every entry, by account", group: "Money" },
  { id: "unpaid", label: "Unpaid", sub: "Invoices and bills", group: "Money" },
  { id: "reconcile", label: "Reconcile", sub: "Match the bank statement", group: "Money" },
  { id: "reports", label: "Reports", sub: "P&L, balance sheet, cash flow", group: "Results" },
  { id: "people", label: "People", sub: "Customers, vendors, employees", group: "Records" },
  { id: "accounts", label: "Accounts", sub: "Chart of accounts", group: "Records" },
  { id: "tags", label: "Tags", sub: "Departments and locations", group: "Records" },
  { id: "settings", label: "Settings", sub: "Book, defaults, data", group: "Records" },
  // the full-page new entry, opened by the Add buttons
  { id: "new", label: "New entry", hidden: true },
  // reached from Settings and the Accounts page
  { id: "import", label: "Old books", hidden: true },
  // Also on Home: tax forms, payroll and certificates stay in navigation;
  // the chat cards and the eBIRForms hand-off open the same pages by link.
  { id: "forms", label: "Tax hand-off", sub: "Measures to eBIRForms", group: "Records" },
  { id: "payroll", label: "Pay runs", sub: "Payroll facts and payslips", group: "Records" },
  { id: "certificates", label: "Certificates", sub: "Withholding certificates", group: "Records" },
];

const page = signal(PAGES.some((x) => x.id === PROPS.page) && PROPS.page !== "ledger" && PROPS.page !== "new" ? String(PROPS.page) : "today");
const sheetMode = signal("");
// `books page:ledger` opens the ledger's native screen once, then shows Home.
if (PROPS.page === "ledger") {
  openLedger({ account: PROPS.account, month: PROPS.month, filter: PROPS.filter });
}

function go(id: string, filter?: string) {
  if (id === "ledger") {
    openLedger({ filter });
    return;
  }
  drawerEntry.set("");
  sheetMode.set("");
  page.set(id);
}
goBack = () => go("today");

/** Add an entry: the full page, in one of its four modes. */
function newEntry(mode: string) {
  drawerEntry.set("");
  sheetMode.set(mode);
  page.set("new");
}

/** First run: name the book and pick its basis. */
function Setup() {
  const label = signal("");
  const basis = signal("cash");
  const err = signal("");
  const busy = signal(false);
  const create = () => {
    if (!label().trim()) {
      err.set("Give the book a name, like your business's.");
      return;
    }
    busy.set(true);
    act("books_app_book_create", { label: label().trim(), basis: basis() })
      .catch((e: Any) => err.set(errText(e)))
      .finally(() => busy.set(false));
  };
  const Opt = (q: { id: string; title: string; sub: string }) => (
    <View
      class={`flex-1 rounded-[10px] border p-3 gap-1 transition-colors ${C.bg} ${C.line} hover:border-[#cbd5e1] dark:hover:border-[#3f3f46]`}
      style={picked(basis() === q.id)}
      onPress={() => basis.set(q.id)}
      activeStyle={{ opacity: 0.7 }}
    >
      <Text class={`text-[13.5px] font-bold ${C.ink}`}>{q.title}</Text>
      <Text class={`text-[12px] ${C.mute}`}>{q.sub}</Text>
    </View>
  );
  return (
    <View class={`flex-1 items-center justify-center ${C.bg}`}>
      <Panel pad class="w-[480px] gap-3.5">
        <Text class={`text-[18px] font-bold ${C.ink}`}>Make your book</Text>
        <Text class={`text-[13px] ${C.mute}`}>Books keeps it on this Mac, encrypted. It starts with the Philippines small-business chart of accounts.</Text>
        <Field label="Name shown in the Buwiz Bot">
          <TextBox value={label()} onChange={(v) => label.set(v)} placeholder="Buwiz Trading" />
        </Field>
        <Field label="Accounting basis">
          <View class="flex-row gap-2">
            <Opt id="cash" title="Cash (default)" sub="Money counts when it moves. Simplest." />
            <Opt id="accrual" title="Accrual" sub="Counts at invoice and bill. Adds receivables and payables." />
          </View>
        </Field>
        <Text class={`text-[12px] ${C.dim}`}>No TIN goes here. A tax profile links later, by its ID only.</Text>
        <If when={Boolean(err())}>
          <Text class={`text-[12.5px] ${C.red}`}>{err()}</Text>
        </If>
        <View class="flex-row justify-end">
          <Btn label="Make the book" tone="go" disabled={busy()} onPress={create} />
        </View>
      </Panel>
    </View>
  );
}

export default function Books() {
  const status = resource(
    () => ({ v: ver() }),
    () => buwiz.call("books_status", {}),
  );
  const book = resource(
    () => ({ v: ver(), ready: (status() as Any)?.status === "ready" }),
    (k: Any) =>
      k.ready
        ? buwiz.call("books_book_get", {}).then((b: Any) => {
            accent.set(ACCENTS[b?.accent] ? b.accent : "green");
            return b;
          })
        : Promise.resolve(null),
  );
  const ready = computed(() => (status() as Any)?.status === "ready");
  const b = () => book() as Any;
  return (
    <View style={{ flex: 1 }} class={`${C.bg}`}>
      <If when={!status() && !status.error()}>
        <View class="flex-1 items-center justify-center">
          <Text class={`text-[13px] ${C.dim}`}>Opening Books…</Text>
        </View>
      </If>
      <If when={Boolean(status.error())}>
        <View class="flex-1 items-center justify-center gap-2">
          <Text class={`text-[13px] ${C.red}`}>{errText(status.error())}</Text>
          <Btn label="Try again" onPress={bump} />
        </View>
      </If>
      <If when={Boolean(status()) && !ready()}>
        <Setup />
      </If>
      <If when={ready()}>
        <If when={page() === "new"}>
          <EntryPage mode={sheetMode} close={() => go("today")} setMode={(m: string) => sheetMode.set(m)} />
        </If>
        <If when={page() === "accounts"}>
          <AccountsPage go={go} />
        </If>
        <If when={page() === "category"}>
          <CategoryDetailPage go={go} />
        </If>
        <If when={page() === "inbox"}>
          <InboxPage go={go} sheet={newEntry} />
        </If>
        <If when={page() === "transactions"}>
          <TransactionsPage go={go} sheet={newEntry} />
        </If>
        <If when={page() === "tags"}>
          <TagsPage go={go} />
        </If>
        <If when={page() !== "new" && page() !== "accounts" && page() !== "category" && page() !== "inbox" && page() !== "tags" && page() !== "transactions"}>
          <Scroll class="flex-1">
            <View class="p-6 gap-5">
              <If when={page() === "today"}>
                <TodayPage go={go} sheet={newEntry} pages={PAGES.filter((x) => x.group)} />
              </If>
              <If when={page() === "unpaid"}>
                <UnpaidPage go={go} />
              </If>
              <If when={page() === "reconcile"}>
                <ReconcilePage go={go} />
              </If>
              <If when={page() === "settings"}>
                <SettingsPage go={go} />
              </If>
              <If when={page() === "reports"}>
                <ReportsPage go={go} />
              </If>
              <If when={page() === "people"}>
                <PeoplePage go={go} />
              </If>
              <If when={page() === "forms"}>
                <FormsPage go={go} />
              </If>
              <If when={page() === "payroll"}>
                <PayrollPage go={go} />
              </If>
              <If when={page() === "certificates"}>
                <CertificatesPage go={go} />
              </If>
              <If when={page() === "import"}>
                <ImportPage go={go} />
              </If>
            </View>
          </Scroll>
        </If>
        <If when={Boolean(drawerEntry())}>
          <EntryDrawer />
        </If>
        <If when={Boolean(accountDrawer()) && page() === "accounts"}>
          <AccountDrawer ticket={accountDrawer()} onClose={() => accountDrawer.set("")} go={go} />
        </If>
      </If>
      <If when={Boolean(toastText())}>
        <View class="absolute bottom-4 left-0 right-0 items-center">
          <View class="rounded-[10px] px-3.5 py-2 bg-[#1c1c1c] dark:bg-[#f0f0f0]">
            <Text class="text-[12.5px] font-semibold text-white dark:text-[#111111]">{toastText()}</Text>
          </View>
        </View>
      </If>
    </View>
  );
}
