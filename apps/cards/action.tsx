// Action cards, the Decision class: what is decided, then its buttons in the footer, and the click is the
// user's. The assistant may draft; posting, categorizing, marking paid, matching and closing happen here,
// behind a press (DD-20). Once done, a card settles to one line.

/** The entry a card is about: its prop, else the newest of a ledger filter. */
function firstEntry(filter: string) {
  return CP.entry
    ? Promise.resolve(CP.entry)
    : buwiz.call("books_ledger_list", { filter }).then((r: Any) => (r.rows || [])[0]?.ticket || null);
}

const plain = (amount: Any) => String(amount || "").replace(/^[+−]\s?/, "");

function DraftCard() {
  const done = signal(false);
  const r = read(
    () => ({ v: ver() }),
    () => firstEntry("draft").then((t: Any) => (t ? buwiz.call("books_entry_get", { entry: t }) : { none: true })),
  );
  const d = () => r() as Any;
  return (
    <View>
      <Wait res={r} />
      <If when={Boolean(d()?.none) && !done()}>
        <Empty text="No draft is waiting." sub="Attach a receipt and say “Record this”: the assistant drafts it, you post it here." open="Open Ledger" page="ledger" />
      </If>
      <If when={Boolean(d()) && !d()?.none && !done()}>
        <Body
          foot={
            <Foot>
              <Act label="Post" primary onPress={() => act("books_app_post_entry", { entry: d()?.ticket }).then(() => done.set(true)).catch(() => null)} />
              <Act label="Edit in Books" onPress={() => openBooks("ledger")} />
              <Act label="Discard" onPress={() => act("books_app_discard_entry", { entry: d()?.ticket }).catch(() => null)} />
            </Foot>
          }
        >
          <Meta text={`${String(d()?.date_short || "")} · ${String(d()?.title || "Draft")}`} right={<Pill tone={d()?.pill} label={d()?.status_label || ""} />} />
          <Hero text={`₱ ${plain(d()?.amount_shown)}`} />
          <List>
            <Each of={d()?.lines || []} key={rowKey}>
              {(l: Any) => <Row label={l().account || ""} value={l().debit_shown || l().credit_shown || ""} />}
            </Each>
          </List>
        </Body>
      </If>
      <If when={done()}>
        <Settled text={`Posted ${plain(d()?.amount_shown)} · ${String(d()?.title || "")}`} sub={`${String(d()?.date_short || "")} · in the Ledger and the P&L`} />
      </If>
    </View>
  );
}

function BankLinesCard() {
  const pick = signal("");
  const r = read(
    () => ({ v: ver() }),
    () =>
      buwiz.call("books_ledger_list", { filter: "review" }).then(async (l: Any) => {
        const rows = (l.rows || []).filter((x: Any) => x.status === "posted");
        const first = rows[0] ? await buwiz.call("books_entry_get", { entry: rows[0].ticket }) : null;
        return { count: rows.length, first };
      }),
  );
  const d = () => r() as Any;
  const options = () => (d()?.first?.suggest || []) as Any[];
  const chosen = () => options().find((s: Any) => s.ticket === pick()) || options().find((s: Any) => s.best) || null;
  return (
    <View>
      <Wait res={r} />
      <If when={Boolean(d()) && !d()?.first}>
        <Settled text="Nothing left to categorize" sub="New bank statement lines show up here." />
      </If>
      <If when={Boolean(d()?.first)}>
        <Body
          foot={
            <Foot>
              <Act
                label={chosen() ? `Post as ${String(chosen().name)}` : "Pick a category"}
                primary
                disabled={!chosen()}
                onPress={() =>
                  act("books_app_recategorize", { entry: d()?.first.ticket, category: chosen().ticket })
                    .then(() => pick.set(""))
                    .catch(() => null)
                }
              />
              <Act label="Other…" onPress={() => openBooks("ledger")} />
            </Foot>
          }
        >
          <Meta text={`${String(d()?.count)} bank line${d()?.count === 1 ? "" : "s"} to categorize`} />
          <Row label={`${String(d()?.first.date_short)} · ${String(d()?.first.title || "")}`} sub="From the bank statement" value={d()?.first.amount_shown} />
          <View class="flex-row gap-1.5 flex-wrap">
            <Each of={options()} key={rowKey}>
              {(s: Any) => <Chip label={`${String(s().name)}${s().best ? " · likely" : ""}`} on={chosen()?.ticket === s().ticket} onPress={() => pick.set(s().ticket)} />}
            </Each>
          </View>
        </Body>
      </If>
    </View>
  );
}

function OwedCard() {
  const paid = signal("");
  const into = signal("");
  const pay = CP.side === "pay";
  const r = read(
    () => ({ v: ver() }),
    () =>
      Promise.all([buwiz.call("books_owed_list", { side: pay ? "pay" : "collect" }), buwiz.call("books_owed_options", {})]).then(([l, o]: Any) => ({
        row: (l.rows || []).find((x: Any) => !CP.owed || x.ticket === CP.owed) || null,
        money: o.money || [],
      })),
  );
  const d = () => r() as Any;
  // the account the money moved through is the owner's pick, as on the Unpaid page: never a silent default
  const account = () => (d()?.money || []).find((m: Any) => m.ticket === into()) || null;
  return (
    <View>
      <Wait res={r} />
      <If when={Boolean(d()) && !d()?.row && !paid()}>
        <Empty text={pay ? "No bill is waiting to be paid." : "Nobody owes you right now."} open="Open Unpaid" page="unpaid" />
      </If>
      <If when={Boolean(d()?.row) && !paid()}>
        <Body
          foot={
            <Foot status={(d()?.money || []).length ? (account() ? "" : pay ? "Pick where the money came from" : "Pick where the money went") : "Add a bank or cash account first"}>
              <Act
                label="Mark paid today"
                primary
                disabled={!account()}
                onPress={() =>
                  act("books_app_owed_settle", { owed: d()?.row.ticket, account: account().ticket })
                    .then(() => paid.set(String(d()?.row.amount_shown)))
                    .catch(() => null)
                }
              />
              <Act label="Different date" onPress={() => openBooks("unpaid")} />
            </Foot>
          }
        >
          <Meta text={`${String(d()?.row.doc || "")} · due ${String(d()?.row.due_short || "")}`} right={<Pill tone="mute" label={pay ? "To pay" : "To collect"} />} />
          <Hero text={`₱ ${String(d()?.row.amount_shown || "")}`} />
          <Row label={pay ? "Vendor" : "Customer"} text={d()?.row.who || ""} />
          <Text class={`text-[12px] ${C.mute}`}>{pay ? "Paid from" : "Paid into"}</Text>
          <View class="flex-row gap-1.5 flex-wrap">
            <Each of={(d()?.money || []).slice(0, 6)} key={rowKey}>
              {(m: Any) => <Chip label={m().name || m().label} on={into() === m().ticket} onPress={() => into.set(m().ticket)} />}
            </Each>
          </View>
        </Body>
      </If>
      <If when={Boolean(paid())}>
        <Settled text={`Money ${pay ? "out" : "in"} ${paid()} recorded`} sub="Today · on a cash basis it counts on the day the money moves" />
      </If>
    </View>
  );
}

function ReconcileCard() {
  const r = read(
    () => ({ v: ver() }),
    () =>
      buwiz.call("books_reconcile_grid", {}).then(async (g: Any) => {
        const months = g.months || [];
        for (const row of g.rows || [])
          for (let i = (row.cells || []).length - 1; i >= 0; i -= 1) {
            const c = row.cells[i];
            if (c.state === "draft" && (!CP.month || c.month === CP.month)) {
              const s = await buwiz.call("books_statement_get", { account: row.ticket, month: c.month });
              return { account: row.ticket, name: row.name, month: c.month, label: (months.find((m: Any) => m.key === c.month) || {}).label, s: s.statement };
            }
          }
        return { none: true };
      }),
  );
  const d = () => r() as Any;
  const count = (k: string) => (d()?.s?.rows || []).filter((x: Any) => x.match === k).length;
  const finished = () => d()?.s?.status === "finished";
  return (
    <View>
      <Wait res={r} />
      <If when={Boolean(d()?.none)}>
        <Empty text="No bank statement is waiting." sub="Import one on the Reconcile page." open="Open Reconcile" page="reconcile" />
      </If>
      <If when={Boolean(d()?.s) && finished()}>
        <Settled text={`${String(d()?.label || "")} matches the bank`} sub={`Difference 0.00 · ending balance ${String(d()?.s.closing_shown || "")}`} />
      </If>
      <If when={Boolean(d()?.s) && !finished()}>
        <Body
          foot={
            <Foot>
              <If when={Boolean(d()?.s.can_finish)}>
                <Act label="Finish" primary onPress={() => act("books_app_statement_finish", { account: d()?.account, month: d()?.month }).catch(() => null)} />
              </If>
              <If when={!d()?.s.can_finish && count("s") > 0}>
                <Act label={`Accept ${count("s")} suggested`} primary onPress={() => act("books_app_statement_accept", { account: d()?.account, month: d()?.month }).catch(() => null)} />
              </If>
              <If when={count("x") > 0}>
                <Act label="Add the missing lines" onPress={() => openBooks("reconcile")} />
              </If>
            </Foot>
          }
        >
          <Meta text={`${String(d()?.name || "")} · ${String(d()?.label || "")}`} />
          <Say text={`${count("m")} matched · ${count("s")} suggested · ${count("x")} missing`} />
          <List>
            <Each of={(d()?.s.rows || []).filter((x: Any) => x.match === "x")} key={rowKey}>
              {(x: Any) => <Row label={`${String(x().date_short)} · ${String(x().description)}`} sub="On the statement, not in Books" value={x().amount_shown} />}
            </Each>
          </List>
          <Row label="Difference" value={d()?.s.difference_shown || ""} strong />
        </Body>
      </If>
    </View>
  );
}

function CloseCard() {
  const r = read(
    () => ({ v: ver() }),
    () => buwiz.call("books_nudge", CP.month ? { kind: "month_end", month: CP.month } : { kind: "month_end" }),
  );
  const d = () => r() as Any;
  const name = () => String(d()?.month_label || "").split(" ")[0];
  const left = () => Number(d()?.left || 0);
  return (
    <View>
      <Wait res={r} />
      <If when={Boolean(d()?.closed)}>
        <Settled text={`${name()} is closed`} sub="Nothing in it can change now. Reopening asks for a reason." />
      </If>
      <If when={Boolean(d()) && !d()?.closed}>
        <Body
          foot={
            <Foot status={left() > 0 ? `${left()} step${left() === 1 ? "" : "s"} left` : "Ready to close"}>
              <Act label={`Close ${name()}`} primary disabled={left() > 0} onPress={() => act("books_app_month_close", { month: d()?.month }).catch(() => null)} />
            </Foot>
          }
        >
          <Meta text={String(d()?.month_label || "")} right={<Pill tone="mute" label={`${String((d()?.checks || []).length - left())} of ${String((d()?.checks || []).length)} done`} />} />
          <View>
            <Each of={d()?.checks || []} key={rowKey}>
              {(c: Any) => <Check label={c().label} done={Boolean(c().done)} />}
            </Each>
          </View>
        </Body>
      </If>
    </View>
  );
}

function CertCard() {
  const r = read(
    () => ({ v: ver() }),
    () => buwiz.call("books_certificates_list", { filter: "all" }).then((l: Any) => (l.rows || []).find((x: Any) => (CP.certificate ? x.ticket === CP.certificate : x.status === "pending")) || null),
  );
  const d = () => r() as Any;
  return (
    <View>
      <Wait res={r} />
      <If when={r() === null}>
        <Empty text="No 2307 is waiting." sub="Add one on the Certificates page when a customer sends it." open="Open Certificates" page="certificates" />
      </If>
      <If when={Boolean(d()) && d()?.status === "received"}>
        <Settled text={`2307 from ${String(d()?.from || "")} received`} sub={`${String(d()?.quarter || "")} · ${String(d()?.amount_shown || "")} withheld · counts to ${String(d()?.counts || "")}`} />
      </If>
      <If when={Boolean(d()) && d()?.status !== "received"}>
        <Body
          foot={
            <Foot>
              <Act label="Mark received" primary onPress={() => act("books_app_certificate_set", { certificate: d()?.ticket, status: "received" }).catch(() => null)} />
              <Act label="Not ours" onPress={() => act("books_app_certificate_set", { certificate: d()?.ticket, status: "disputed" }).catch(() => null)} />
            </Foot>
          }
        >
          <Meta text={`${String(d()?.from || "")} · ${String(d()?.quarter || "")}`} right={<Pill tone="draft" label="Pending" />} />
          <Hero text={`₱ ${String(d()?.amount_shown || "")}`} />
          <View>
            <Row label="ATC on the paper" text={`${String(d()?.atc || "—")} · ${String(d()?.kind || "").toLowerCase()}`} />
            <Row label="Counts to" text="Not counted until received" />
          </View>
        </Body>
      </If>
    </View>
  );
}

export default function Action() {
  return (
    // the card lays out at its natural height and reports it; it never scrolls or bounces
    <Scroll class={`flex-1 ${K.surface}`} bounces={false} scrollable={false}>
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
