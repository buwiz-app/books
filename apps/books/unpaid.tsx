// Unpaid: invoices to collect and bills to pay. On a cash book they are
// reminders; "Mark paid" records the money on the day it moves (ADR 0008).

const unpaidSide = signal("collect");
const owedSheet = signal(false);

function UnpaidPage(p: { go: (page: string, filter?: string) => void }) {
  const data = resource(
    () => ({ v: ver(), s: unpaidSide() }),
    (k: Any) => buwiz.call("books_owed_list", { side: k.s }),
  );
  const opts = resource(
    () => ({ v: ver() }),
    () => buwiz.call("books_owed_options", {}),
  );
  const d = () => data() as Any;
  /** The document being marked paid, and from which account. */
  const settling = signal("");
  const fromAccount = signal("");
  const settle = (ticket: string) => {
    if (!fromAccount()) {
      toast("Pick the account the money moved through.");
      return;
    }
    act("books_app_owed_settle", { owed: ticket, account: fromAccount() })
      .then((r: Any) => {
        settling.set("");
        toast(`Paid · entry ${r.no} posted`);
      })
      .catch(() => null);
  };
  return (
    <View class="gap-3.5">
      <Head title="Unpaid" sub="invoices and bills waiting for money">
        <View class="flex-row gap-2">
          <Btn label={unpaidSide() === "collect" ? "+ New invoice" : "+ New bill"} tone="go" onPress={() => owedSheet.set(true)} />
        </View>
      </Head>
      <Seg
        options={[
          ["collect", `To collect · ${String(d()?.counts.collect ?? 0)}`],
          ["pay", `To pay · ${String(d()?.counts.pay ?? 0)}`],
        ]}
        value={unpaidSide()}
        onPick={(v: string) => unpaidSide.set(v)}
      />
      <Loading res={data} />
      <If when={Boolean(d())}>
        <Panel>
          <View class={`flex-row gap-3 px-3 py-2 border-b ${C.line}`}>
            <Text class={`flex-1 text-[10.5px] uppercase tracking-wider ${C.dim}`}>{unpaidSide() === "collect" ? "Customer" : "Vendor"}</Text>
            <Text class={`w-[150px] text-[10.5px] uppercase tracking-wider ${C.dim}`}>Document</Text>
            <Text class={`w-[64px] text-[10.5px] uppercase tracking-wider ${C.dim}`}>Due</Text>
            <Text class={`w-[110px] text-right text-[10.5px] uppercase tracking-wider ${C.dim}`}>Amount</Text>
            <View class="w-[100px]" />
          </View>
          <If when={(d()?.rows || []).length === 0}>
            <Text class={`text-[12.5px] px-3 py-4 ${C.dim}`}>Nothing waiting. Paid items are in the Ledger.</Text>
          </If>
          <Each of={d()?.rows || []} key={rowKey}>
            {(r: Any) => (
              <View class={`border-b ${C.line}`}>
                <View class="flex-row items-center gap-3 px-3 py-2.5">
                  <Text class={`flex-1 text-[13px] font-semibold ${C.ink}`} numberOfLines={1}>
                    {r().who}
                  </Text>
                  <Text class={`w-[150px] text-[12.5px] ${C.ink}`} numberOfLines={1}>
                    {r().doc}
                  </Text>
                  <Text class={`w-[64px] text-[12.5px] font-mono ${r().late ? C.red : C.ink}`}>{r().due_short}</Text>
                  <Text class={`w-[110px] text-right text-[12.5px] font-mono ${C.ink}`}>{r().amount_shown}</Text>
                  <View class="w-[100px] items-end">
                    <Btn label="Mark paid" small onPress={() => settling.set(settling() === r().ticket ? "" : r().ticket)} />
                  </View>
                </View>
                <If when={settling() === r().ticket}>
                  <View class={`flex-row items-center gap-2 px-3 pb-2.5`}>
                    <Text class={`text-[12px] ${C.dim}`}>{unpaidSide() === "collect" ? "Into" : "Paid from"}</Text>
                    <View class="w-[240px]">
                      <Pick value={fromAccount()} options={(opts() as Any)?.money || []} onPick={(a: Any) => fromAccount.set(a.ticket)} placeholder="Bank, cash or card" />
                    </View>
                    <Btn label="Record it today" tone="go" small onPress={() => settle(r().ticket)} />
                    <Btn label="Cancel document" small onPress={() => act("books_app_owed_cancel", { owed: r().ticket }).catch(() => null)} />
                  </View>
                </If>
              </View>
            )}
          </Each>
        </Panel>
        <Panel pad class="flex-row gap-3 items-center">
          <Pill tone="in" label={d()?.basis === "cash" ? "Cash basis" : "Accrual basis"} />
          <Text class={`flex-1 text-[12px] ${C.dim}`}>{`${d()?.note || ""} Total ${unpaidSide() === "collect" ? "to collect" : "to pay"}: ${d()?.total_shown || ""}.`}</Text>
        </Panel>
      </If>
      <If when={owedSheet()}>
        <OwedSheet side={unpaidSide()} opts={opts} close={() => owedSheet.set(false)} />
      </If>
    </View>
  );
}

function OwedSheet(p: { side: string; opts: Any; close: () => void }) {
  const o = () => p.opts() as Any;
  const amount = signal("");
  const who = signal("");
  const whoName = signal("");
  const category = signal("");
  const reference = signal("");
  const due = signal("");
  const err = signal("");
  const save = () => {
    const amt = moneyText(amount());
    if (!amt) return err.set("Type an amount, like 60,000.00.");
    if (!category()) return err.set("Pick a category.");
    const args: Record<string, Any> = { side: p.side, amount: amt, category: category() };
    if (who()) args.person = who();
    else if (whoName().trim()) args.person_name = whoName().trim();
    if (reference()) args.reference = reference();
    if (due()) args.due = due();
    act("books_app_owed_create", args)
      .then(() => {
        toast(p.side === "collect" ? "Invoice saved" : "Bill saved");
        p.close();
      })
      .catch((e: Any) => err.set(errText(e)));
  };
  const people = () => (p.side === "collect" ? o()?.customers : o()?.vendors) || [];
  return (
    <Panel pad class="gap-3">
      <Text class={`text-[14px] font-bold ${C.ink}`}>{p.side === "collect" ? "New invoice to collect" : "New bill to pay"}</Text>
      <View class="flex-row gap-3">
        <Field label="Amount" grow>
          <TextBox value={amount()} onChange={(v) => amount.set(v)} placeholder="₱ 0.00" />
        </Field>
        <Field label="Due" grow>
          <TextBox value={due()} onChange={(v) => due.set(v)} placeholder="YYYY-MM-DD" />
        </Field>
      </View>
      <View class="flex-row gap-3">
        <Field label={p.side === "collect" ? "Customer" : "Vendor"} grow>
          <Pick value={who()} options={people()} onPick={(x: Any) => who.set(x.ticket)} placeholder="Pick, or type a new name below" />
          <TextBox value={whoName()} onChange={(v) => whoName.set(v)} placeholder="New name" />
        </Field>
        <Field label="Category" grow>
          <Pick value={category()} options={(p.side === "collect" ? o()?.income : o()?.expense) || []} onPick={(x: Any) => category.set(x.ticket)} placeholder="Pick a category" />
        </Field>
      </View>
      <Field label="Document">
        <TextBox value={reference()} onChange={(v) => reference.set(v)} placeholder={p.side === "collect" ? "Invoice 0143" : "Bill PP-0381"} />
      </Field>
      <If when={Boolean(err())}>
        <Text class={`text-[12.5px] ${C.red}`}>{err()}</Text>
      </If>
      <View class="flex-row gap-2 justify-end">
        <Btn label="Cancel" onPress={p.close} />
        <Btn label="Save" tone="go" onPress={save} />
      </View>
    </Panel>
  );
}
