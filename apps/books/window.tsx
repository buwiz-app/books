// The Books window: the rail of twelve pages, the page, the entry drawer
// and the new-entry sheet. First run makes the book (cash basis unless
// the owner picks accrual, ADR 0008).

const PAGES: { id: string; label: string; group?: string; bir?: boolean }[] = [
  { id: "today", label: "Today" },
  { id: "ledger", label: "Ledger", group: "Money" },
  { id: "unpaid", label: "Unpaid", group: "Money" },
  { id: "reconcile", label: "Reconcile", group: "Money" },
  { id: "reports", label: "Reports", group: "Results" },
  { id: "forms", label: "Forms", group: "Results", bir: true },
  { id: "people", label: "People", group: "Records" },
  { id: "accounts", label: "Accounts", group: "Records" },
  { id: "tags", label: "Tags", group: "Records" },
  { id: "payroll", label: "Payroll", group: "Records" },
  { id: "certificates", label: "Certificates", group: "Records" },
  { id: "settings", label: "Settings", group: "end" },
];
const BUILT = ["today", "ledger", "accounts"];

const page = signal(PAGES.some((x) => x.id === PROPS.page) ? String(PROPS.page) : "today");
const sheetMode = signal("");

function go(id: string, filter?: string) {
  if (filter) ledgerFilter.set(filter);
  drawerEntry.set("");
  page.set(id);
}

function Rail(p: { book: Any; counts: Any }) {
  let last = "";
  const rows: Any[] = [];
  for (const x of PAGES) {
    if (x.group === "end") rows.push({ spacer: true });
    else if (x.group && x.group !== last) {
      rows.push({ head: x.group });
      last = x.group;
    }
    rows.push({ page: x });
  }
  const count = (id: string) => (id === "ledger" ? (p.counts?.review || 0) + (p.counts?.drafts || 0) : 0);
  return (
    <View class={`w-[178px] border-r px-2 py-2.5 gap-px ${C.rail} ${C.line}`}>
      <View class={`flex-row items-center gap-2 px-1.5 pt-1 pb-2.5 mb-1.5 border-b ${C.line}`}>
        <View class="w-[26px] h-[26px] rounded-lg items-center justify-center bg-[#157a48]">
          <Text class="text-[13px] font-extrabold text-white">B</Text>
        </View>
        <View class="flex-1">
          <Text class={`text-[12.5px] font-bold ${C.ink}`} numberOfLines={1}>
            {p.book?.label || "Books"}
          </Text>
          <Text class={`text-[11px] ${C.dim}`}>{p.book ? `${p.book.basis_label} · ${String(p.book.year)}` : ""}</Text>
        </View>
      </View>
      <Each of={rows}>
        {(r: Any) => (
          <View class={r().spacer ? "flex-1" : ""}>
            <If when={Boolean(r().head)}>
              <Text class={`text-[10.5px] uppercase tracking-widest px-2 pt-2.5 pb-1 ${C.dim}`}>{r().head || ""}</Text>
            </If>
            <If when={Boolean(r().page)}>
              <View
                class={`flex-row items-center gap-2 px-2 py-1.5 rounded-lg ${page() === r().page?.id ? C.raise : ""}`}
                onPress={() => go(r().page.id)}
                activeStyle={{ opacity: 0.7 }}
              >
                <Text class={`flex-1 text-[13px] ${page() === r().page?.id ? `font-semibold ${C.ink}` : C.mute}`}>{r().page?.label || ""}</Text>
                <If when={Boolean(r().page?.bir)}>
                  <View class={`rounded-[5px] px-1.5 ${PILL.bir}`}>
                    <Text class={`text-[10px] font-bold ${PILL.bir}`}>BIR</Text>
                  </View>
                </If>
                <If when={count(r().page?.id) > 0}>
                  <View class={`rounded-full px-1.5 ${PILL.draft}`}>
                    <Text class={`text-[11px] ${PILL.draft}`}>{String(count(r().page?.id))}</Text>
                  </View>
                </If>
              </View>
            </If>
          </View>
        )}
      </Each>
    </View>
  );
}

function Later(p: { id: string }) {
  const label = (PAGES.find((x) => x.id === p.id) || { label: "" }).label;
  return (
    <View class="gap-3.5">
      <Head title={label} />
      <Panel pad>
        <Text class={`text-[13px] ${C.mute}`}>{`${label} arrives in a later step of the build. Today, Ledger and Accounts work now.`}</Text>
      </Panel>
    </View>
  );
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
      class={`flex-1 rounded-[10px] border p-3 gap-1 ${C.bg} ${basis() === q.id ? "border-[#157a48] dark:border-[#3fcf89]" : C.line}`}
      onPress={() => basis.set(q.id)}
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
        <Field label="Name shown in Rox">
          <TextBox value={label()} onChange={label.set} placeholder="Buwiz Trading" />
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
    () => rox.call("books_status", {}),
  );
  const book = resource(
    () => ({ v: ver(), ready: (status() as Any)?.status === "ready" }),
    (k: Any) => (k.ready ? rox.call("books_book_get", {}) : Promise.resolve(null)),
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
        <View class="flex-1 flex-row">
          <Rail book={b()} counts={{ review: b()?.to_review || 0, drafts: b()?.drafts || 0 }} />
          <Scroll class="flex-1">
            <View class="p-4 gap-3.5">
              <If when={page() === "today"}>
                <TodayPage go={go} sheet={(m: string) => sheetMode.set(m)} />
              </If>
              <If when={page() === "ledger"}>
                <LedgerPage go={go} sheet={(m: string) => sheetMode.set(m)} />
              </If>
              <If when={page() === "accounts"}>
                <AccountsPage go={go} />
              </If>
              <If when={!BUILT.includes(page())}>
                <Later id={page()} />
              </If>
            </View>
          </Scroll>
        </View>
        <If when={Boolean(drawerEntry())}>
          <EntryDrawer />
        </If>
        <If when={Boolean(sheetMode())}>
          <EntrySheet mode={sheetMode} close={() => sheetMode.set("")} setMode={(m: string) => sheetMode.set(m)} />
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
