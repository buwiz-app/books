// Answer cards: the user asked, Books shows (amounts, because they asked: owner decision). Cash now, this month
// and a person are Glance cards (one figure, a few rows); unpaid is a List card.

function CashCard() {
  const r = resource(
    () => ({ v: ver() }),
    () => rox.call("books_today", {}),
  );
  const d = () => r() as Any;
  return (
    <View>
      <Wait res={r} />
      <If when={Boolean(d())}>
        <Body
          foot={
            <Foot>
              <Act label="Open Ledger" onPress={() => openBooks("ledger")} />
            </Foot>
          }
        >
          <Meta text={`As of ${String(d()?.today_label || "")}`} />
          <Hero text={`₱ ${String(d()?.cash.shown || "")}`} />
          <View>
            <Each of={d()?.cash.rows || []} key={rowKey}>
              {(x: Any) => <Row label={x().label || x().name || ""} value={x().shown} tone={x().tone === "neg" ? "neg" : ""} />}
            </Each>
          </View>
        </Body>
      </If>
    </View>
  );
}

function MonthCard() {
  const r = resource(
    () => ({ v: ver() }),
    () => rox.call("books_today", {}),
  );
  const d = () => r() as Any;
  const rows = () => d()?.month.rows || [];
  return (
    <View>
      <Wait res={r} />
      <If when={Boolean(d())}>
        <Body
          foot={
            <Foot>
              <Act label="Open the P&L" onPress={() => openBooks("reports")} />
            </Foot>
          }
        >
          <Meta text={`${String(d()?.month_name || "")} so far · net`} right={<Pill tone="mute" label={d()?.book.basis_label || ""} />} />
          <Hero text={String(d()?.month.shown || "")} tone={d()?.month.tone === "neg" ? "neg" : "pos"} />
          <View>
            <Row label="Money in" value={rows()[0]?.shown || ""} tone="pos" />
            <Row label="Money out" value={rows()[1]?.shown || ""} />
            <Row label={rows()[2]?.label || "Last month"} value={rows()[2]?.shown || ""} />
          </View>
        </Body>
      </If>
    </View>
  );
}

/** One person: what you paid them or they paid you, and the latest entries. */
function PersonView(p: { person: () => string; back: (() => void) | null }) {
  const r = resource(
    () => ({ v: ver(), p: p.person() }),
    () => rox.call("books_person_get", { person: p.person() }),
  );
  const d = () => r() as Any;
  return (
    <View>
      <Wait res={r} />
      <If when={Boolean(d())}>
        <Body
          foot={
            <Foot>
              <If when={Boolean(p.back)}>
                <Act label="All people" onPress={() => (p.back ? p.back() : null)} />
              </If>
              <Act label="Open in People" onPress={() => openBooks("people")} />
            </Foot>
          }
        >
          <Meta text={`${String(d()?.name || "")} · ${String(d()?.sub || "")}`} />
          <Hero text={`₱ ${String(d()?.kpi?.[0]?.shown || "0.00")}`} />
          <Say text={(d()?.kpi || []).map((k: Any) => `${String(k.label)} ${String(k.shown)}`).join(" · ")} dim />
          <List>
            <Each of={(d()?.recent || []).slice(0, 12)} key={rowKey}>
              {(x: Any) => (
                <Row
                  label={`${String(x().date_short)} · ${String(x().title || "")}`}
                  sub={x().sub || ""}
                  value={x().status === "posted" ? x().amount_shown : ""}
                  right={x().status !== "posted" ? <Pill tone={x().pill} label={x().status_label} /> : null}
                />
              )}
            </Each>
          </List>
        </Body>
      </If>
    </View>
  );
}

/** No person named: pick one, by kind. */
function PersonPicker(p: { onPick: (ticket: string) => void }) {
  const kind = signal("");
  const r = resource(
    () => ({ v: ver(), k: kind() }),
    () => rox.call("books_people_list", kind() ? { kind: kind() } : {}),
  );
  const d = () => r() as Any;
  return (
    <View>
      <Wait res={r} />
      <If when={Boolean(d())}>
        <Body
          foot={
            <Foot status="Pick a person to see them here">
              <Act label="Open People" onPress={() => openBooks("people")} />
            </Foot>
          }
        >
          <View class="flex-row gap-1.5 flex-wrap">
            <Each of={(d()?.kinds || []).filter((k: Any) => k.count > 0)} key={rowKey}>
              {(k: Any) => <Chip label={k().label} count={k().count} on={(kind() || d()?.kind) === k().id} onPress={() => kind.set(k().id)} />}
            </Each>
          </View>
          <List>
            <Each of={d()?.rows || []} key={rowKey}>
              {(x: Any) => (
                <View onPress={() => p.onPick(x().ticket)} activeStyle={{ opacity: 0.6 }}>
                  <Row label={x().name || ""} sub={x().sub || ""} right={<Text class={`text-[13px] ${C.mute}`}>›</Text>} />
                </View>
              )}
            </Each>
          </List>
        </Body>
      </If>
    </View>
  );
}

function PersonCard() {
  const picked = signal(String(CP.person || ""));
  return (
    <View>
      <If when={!picked()}>
        <PersonPicker onPick={(t) => picked.set(t)} />
      </If>
      <If when={Boolean(picked())}>
        <PersonView person={() => picked()} back={CP.person ? null : () => picked.set("")} />
      </If>
    </View>
  );
}

function UnpaidCard() {
  const side = CP.side === "pay" ? "pay" : "collect";
  const r = resource(
    () => ({ v: ver() }),
    () => rox.call("books_owed_list", { side }),
  );
  const d = () => r() as Any;
  const n = () => (d()?.rows || []).length;
  return (
    <View>
      <Wait res={r} />
      <If when={Boolean(d()) && n() === 0}>
        <Empty text={side === "pay" ? "No bill is waiting to be paid." : "Nobody owes you right now."} open="Open Unpaid" page="unpaid" />
      </If>
      <If when={n() > 0}>
        <Body
          foot={
            <Foot>
              <Act label="Open Unpaid" onPress={() => openBooks("unpaid")} />
            </Foot>
          }
        >
          <Meta text={`${n()} ${side === "pay" ? "bill" : "invoice"}${n() === 1 ? "" : "s"} ${side === "pay" ? "to pay" : "to collect"}`} />
          <List>
            <Each of={d()?.rows || []} key={rowKey}>
              {(x: Any) => <Row label={x().who || x().name || ""} sub={`${String(x().doc || "")} · due ${String(x().due_short || "")}`} value={x().amount_shown} />}
            </Each>
          </List>
          <Row label="Total" value={d()?.total_shown || ""} strong />
        </Body>
      </If>
    </View>
  );
}

export default function Answer() {
  return (
    // the card lays out at its natural height and reports it; it never scrolls or bounces
    <Scroll class={`flex-1 ${K.surface}`} bounces={false} scrollable={false}>
      <If when={CARD === "cash"}>
        <CashCard />
      </If>
      <If when={CARD === "month"}>
        <MonthCard />
      </If>
      <If when={CARD === "person"}>
        <PersonCard />
      </If>
      <If when={CARD === "unpaid"}>
        <UnpaidCard />
      </If>
    </Scroll>
  );
}
