// Action cards: one click to finish, and the click is the user's. The
// assistant may draft; posting, categorizing, marking paid, matching and
// closing happen here, behind a press (DD-20).

/** The entry a card is about: its prop, else the newest of a ledger filter. */
function firstEntry(filter: string) {
  return CP.entry
    ? Promise.resolve(CP.entry)
    : rox.call("books_ledger_list", { filter }).then((r: Any) => (r.rows || [])[0]?.ticket || null);
}

function DraftCard() {
  const done = signal(false);
  const r = resource(
    () => ({ v: ver() }),
    () => firstEntry("draft").then((t: Any) => (t ? rox.call("books_entry_get", { entry: t }) : { none: true })),
  );
  const d = () => r() as Any;
  return (
    <View>
      <CardWait res={r} title="Books · Draft to post" />
      <If when={Boolean(d()?.none)}>
        <Card title="Books · Draft from a receipt">
          <Note text="Attach a receipt photo or PDF and say “Record this”. The assistant drafts it; you post it here." />
        </Card>
      </If>
      <If when={Boolean(d()) && !d()?.none}>
        <Card title={`Books · ${String(d()?.title || "Draft")}`} right={<Pill tone={done() ? "ok" : d()?.pill} label={done() ? "Posted" : d()?.status_label || ""} />}>
          <If when={!done()}>
            <Line label="Amount" value={String(d()?.amount_shown || "").replace(/^[+−]/, "")} />
            <Line label="Date" value={d()?.date_short || ""} />
            <Each of={d()?.lines || []} key={rowKey}>
              {(l: Any) => <Line label={l().account || ""} value={l().debit_shown || l().credit_shown || ""} />}
            </Each>
            <Acts>
              <Btn label="Post" tone="go" onPress={() => act("books_app_post_entry", { entry: d()?.ticket }).then(() => done.set(true)).catch(() => null)} />
              <Btn label="Edit in Books" onPress={() => openBooks("ledger")} />
              <LinkBtn label="Discard" onPress={() => act("books_app_discard_entry", { entry: d()?.ticket }).catch(() => null)} />
            </Acts>
          </If>
          <If when={done()}>
            <DoneLine text={`Posted ${String(d()?.amount_shown || "").replace(/^[+−]/, "")}`} sub={`${String(d()?.date_short || "")} · shows in the Ledger and the P&L`} />
          </If>
        </Card>
      </If>
    </View>
  );
}

function BankLinesCard() {
  const r = resource(
    () => ({ v: ver() }),
    () =>
      rox.call("books_ledger_list", { filter: "review" }).then(async (l: Any) => {
        const rows = (l.rows || []).filter((x: Any) => x.status === "posted");
        const first = rows[0] ? await rox.call("books_entry_get", { entry: rows[0].ticket }) : null;
        return { count: rows.length, first };
      }),
  );
  const d = () => r() as Any;
  return (
    <View>
      <CardWait res={r} title="Books · Bank lines" />
      <If when={Boolean(d()) && !d()?.first}>
        <Card title="Books · Bank lines" right={<Pill tone="ok" label="All done" />}>
          <DoneLine text="Nothing left to categorize" sub="New statement lines show up here." />
        </Card>
      </If>
      <If when={Boolean(d()?.first)}>
        <Card title="Books · Bank lines to categorize" right={<Pill tone="mute" label={`${String(d()?.count)} left`} />}>
          <Line label={`${String(d()?.first.date_short)} · ${String(d()?.first.title || "")}`} sub="From the bank statement" value={d()?.first.amount_shown} />
          <View class="flex-row gap-1.5 flex-wrap">
            <Each of={d()?.first.suggest || []} key={rowKey}>
              {(s: Any) => (
                <Chip
                  label={`${String(s().name)}${s().best ? " · likely" : ""}`}
                  on={Boolean(s().best)}
                  onPress={() => act("books_app_recategorize", { entry: d()?.first.ticket, category: s().ticket }).catch(() => null)}
                />
              )}
            </Each>
            <Chip label="Other…" on={false} onPress={() => openBooks("ledger")} />
          </View>
          <Note text="Picking a category posts it." />
        </Card>
      </If>
    </View>
  );
}

function OwedCard() {
  const paid = signal("");
  const r = resource(
    () => ({ v: ver() }),
    () =>
      Promise.all([rox.call("books_owed_list", { side: CP.side === "pay" ? "pay" : "collect" }), rox.call("books_owed_options", {})]).then(([l, o]: Any) => ({
        row: (l.rows || []).find((x: Any) => !CP.owed || x.ticket === CP.owed) || null,
        money: (o.money || [])[0] || null,
      })),
  );
  const d = () => r() as Any;
  return (
    <View>
      <CardWait res={r} title="Books · Mark paid" />
      <If when={Boolean(d()) && !d()?.row && !paid()}>
        <Card title="Books · Unpaid">
          <DoneLine text="Nothing waiting" />
        </Card>
      </If>
      <If when={Boolean(d()?.row) && !paid()}>
        <Card title={CP.side === "pay" ? "Books · Unpaid bill" : "Books · Unpaid invoice"} right={<Pill tone="mute" label={`Due ${String(d()?.row.due_short || "")}`} />}>
          <Line label={CP.side === "pay" ? "Vendor" : "Customer"} value={d()?.row.who || ""} />
          <Line label="Document" value={d()?.row.doc || ""} />
          <Line label="Amount" value={d()?.row.amount_shown || ""} />
          <Line label={CP.side === "pay" ? "From" : "Into"} value={d()?.money?.label || d()?.money?.name || "—"} />
          <Acts>
            <Btn
              label="Mark paid today"
              tone="go"
              disabled={!d()?.money}
              onPress={() =>
                act("books_app_owed_settle", { owed: d()?.row.ticket, account: d()?.money.ticket })
                  .then(() => paid.set(String(d()?.row.amount_shown)))
                  .catch(() => null)
              }
            />
            <Btn label="Different date" onPress={() => openBooks("unpaid")} />
          </Acts>
        </Card>
      </If>
      <If when={Boolean(paid())}>
        <Card title="Books · Paid" right={<Pill tone="ok" label="Paid" />}>
          <DoneLine text={`Money ${CP.side === "pay" ? "out" : "in"} ${paid()} recorded`} sub="Today · cash basis: it counts on the day the money moves" />
        </Card>
      </If>
    </View>
  );
}

function ReconcileCard() {
  const r = resource(
    () => ({ v: ver() }),
    () =>
      rox.call("books_reconcile_grid", {}).then(async (g: Any) => {
        const months = g.months || [];
        for (const row of g.rows || [])
          for (let i = (row.cells || []).length - 1; i >= 0; i -= 1) {
            const c = row.cells[i];
            if (c.state === "draft" && (!CP.month || c.month === CP.month)) {
              const s = await rox.call("books_statement_get", { account: row.ticket, month: c.month });
              return { account: row.ticket, name: row.name, month: c.month, label: (months.find((m: Any) => m.key === c.month) || {}).label, s: s.statement };
            }
          }
        return { none: true };
      }),
  );
  const d = () => r() as Any;
  const counts = () => {
    const rows = d()?.s?.rows || [];
    return [rows.filter((x: Any) => x.match === "m").length, rows.filter((x: Any) => x.match === "s").length, rows.filter((x: Any) => x.match === "x").length];
  };
  return (
    <View>
      <CardWait res={r} title="Books · Match the bank" />
      <If when={Boolean(d()?.none)}>
        <Card title="Books · Match the bank">
          <Note text="No statement waits. Import one on the Reconcile page." />
          <Acts>
            <LinkBtn label="Open Reconcile" onPress={() => openBooks("reconcile")} />
          </Acts>
        </Card>
      </If>
      <If when={Boolean(d()?.s)}>
        <Card title={`Books · Reconcile ${String(d()?.name || "")}`} right={<Pill tone={d()?.s.status === "finished" ? "ok" : "mute"} label={d()?.s.status === "finished" ? "Matched" : String(d()?.label || "")} />}>
          <If when={d()?.s.status !== "finished"}>
            <Strip
              items={[
                ["Matched", counts()[0], ""],
                ["Suggested", counts()[1], "warn"],
                ["Missing", counts()[2], "neg"],
              ]}
            />
            <Each of={(d()?.s.rows || []).filter((x: Any) => x.match === "x")} key={rowKey}>
              {(x: Any) => <Line label={`${String(x().date_short)} · ${String(x().description)}`} sub="On the statement, not in Books" value={x().amount_shown} />}
            </Each>
            <Note text={`Difference ${String(d()?.s.difference_shown || "")}`} />
            <Acts>
              <If when={counts()[1] > 0}>
                <Btn label="Accept the suggestions" tone="go" onPress={() => act("books_app_statement_accept", { account: d()?.account, month: d()?.month }).catch(() => null)} />
              </If>
              <If when={Boolean(d()?.s.can_finish)}>
                <Btn label="Finish" tone="go" onPress={() => act("books_app_statement_finish", { account: d()?.account, month: d()?.month }).catch(() => null)} />
              </If>
              <If when={counts()[2] > 0}>
                <Btn label="Add the missing lines" onPress={() => openBooks("reconcile")} />
              </If>
            </Acts>
          </If>
          <If when={d()?.s.status === "finished"}>
            <DoneLine text={`${String(d()?.label || "")} matches the bank`} sub={`Difference 0.00 · ending balance ${String(d()?.s.closing_shown || "")}`} />
          </If>
        </Card>
      </If>
    </View>
  );
}

function CloseCard() {
  const r = resource(
    () => ({ v: ver() }),
    () => rox.call("books_nudge", CP.month ? { kind: "month_end", month: CP.month } : { kind: "month_end" }),
  );
  const d = () => r() as Any;
  const name = () => String(d()?.month_label || "").split(" ")[0];
  return (
    <View>
      <CardWait res={r} title="Books · Close the month" />
      <If when={Boolean(d()) && !d()?.closed}>
        <Card title={`Books · Close ${name()}`} right={<Pill tone="mute" label={`${String(3 - (d()?.left || 0))} of 3`} />}>
          <Each of={d()?.checks || []} key={rowKey}>
            {(c: Any) => (
              <View class="flex-row items-center gap-2 py-1">
                <View class={`w-[18px] h-[18px] rounded-md items-center justify-center border ${c().done ? `${C.greenBg} border-transparent` : C.line}`}>
                  <Text class="text-[11px] font-bold text-white dark:text-[#0d0d0d]">{c().done ? "✓" : ""}</Text>
                </View>
                <Text class={`text-[12.5px] ${C.ink}`}>{c().label}</Text>
              </View>
            )}
          </Each>
          <Acts>
            <Btn label={`Close ${name()}`} tone="go" disabled={(d()?.left || 0) > 0} onPress={() => act("books_app_month_close", { month: d()?.month }).catch(() => null)} />
            <Text class={`text-[11.5px] ${C.mute}`}>{(d()?.left || 0) > 0 ? "Finish each one first" : "Ready"}</Text>
          </Acts>
        </Card>
      </If>
      <If when={Boolean(d()?.closed)}>
        <Card title={`Books · ${name()}`} right={<Pill tone="ok" label="Closed" />}>
          <DoneLine text={`${name()} is closed`} sub="Nothing in it can change now. Reopening asks for a reason." />
        </Card>
      </If>
    </View>
  );
}

function CertCard() {
  const r = resource(
    () => ({ v: ver() }),
    () => rox.call("books_certificates_list", { filter: "all" }).then((l: Any) => (l.rows || []).find((x: Any) => (CP.certificate ? x.ticket === CP.certificate : x.status === "pending")) || null),
  );
  const d = () => r() as Any;
  return (
    <View>
      <CardWait res={r} title="Books · 2307 received" />
      <If when={r() === null}>
        <Card title="Books · 2307s">
          <DoneLine text="No 2307 is waiting" sub="Add one on the Certificates page." />
        </Card>
      </If>
      <If when={Boolean(d())}>
        <Card title="Books · 2307 received" right={<Pill tone={d()?.status === "received" ? "ok" : "draft"} label={d()?.status === "received" ? "Received" : "Pending"} />}>
          <Line label={`${String(d()?.from || "")} · ${String(d()?.quarter || "")}`} sub={`ATC on the paper: ${String(d()?.atc || "—")} (${String(d()?.kind || "").toLowerCase()})`} />
          <Line label="Withheld" value={d()?.amount_shown || ""} />
          <Line label="Counts to" value={d()?.status === "received" ? d()?.counts : "Not counted until received"} />
          <If when={d()?.status !== "received"}>
            <Acts>
              <Btn label="Mark received" tone="go" onPress={() => act("books_app_certificate_set", { certificate: d()?.ticket, status: "received" }).catch(() => null)} />
              <Btn label="Not ours" onPress={() => act("books_app_certificate_set", { certificate: d()?.ticket, status: "disputed" }).catch(() => null)} />
            </Acts>
          </If>
          <If when={d()?.status === "received"}>
            <Note text="The payor's TIN stays in the BIR app." />
          </If>
        </Card>
      </If>
    </View>
  );
}

export default function Action() {
  return (
    // a Scroll lays the card out at its natural height whatever the frame, so
    // it measures right for rox.fit; it never scrolls or bounces: it is a card
    <Scroll class={`flex-1 ${C.bg}`} bounces={false} scrollable={false}>
      <If when={CARD === "draft"}>
        <DraftCard />
      </If>
      <If when={CARD === "bank_lines"}>
        <BankLinesCard />
      </If>
      <If when={CARD === "owed"}>
        <OwedCard />
      </If>
      <If when={CARD === "reconcile"}>
        <ReconcileCard />
      </If>
      <If when={CARD === "close"}>
        <CloseCard />
      </If>
      <If when={CARD === "cert"}>
        <CertCard />
      </If>
    </Scroll>
  );
}
