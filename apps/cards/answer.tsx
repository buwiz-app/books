// Answer cards: the user asked, Books shows (amounts, because they asked:
// owner decision). Cash now, this month, a person, unpaid.

function CashCard() {
  const r = resource(
    () => ({ v: ver() }),
    () => rox.call("books_today", {}),
  );
  const d = () => r() as Any;
  return (
    <View>
      <CardWait res={r} title="Books · Cash now" />
      <If when={Boolean(d())}>
        <Card title="Books · Cash now" right={<Pill tone="mute" label={String(d()?.today_label || "").split(", ")[1] || ""} />}>
          <Big text={`₱ ${String(d()?.cash.shown || "")}`} />
          <Each of={d()?.cash.rows || []} key={rowKey}>
            {(x: Any) => <Line label={x().label || x().name || ""} value={x().shown} tone={x().tone === "neg" ? "neg" : ""} />}
          </Each>
          <Acts>
            <LinkBtn label="Open Ledger" onPress={() => openBooks("ledger")} />
          </Acts>
        </Card>
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
  return (
    <View>
      <CardWait res={r} title="Books · This month" />
      <If when={Boolean(d())}>
        <Card title={`Books · ${String(d()?.month_name || "")}`} right={<Pill tone="in" label={d()?.book.basis_label || ""} />}>
          <Strip
            items={[
              ["In", d()?.month.rows[0].shown, "pos"],
              ["Out", String(d()?.month.rows[1].shown || "").replace(/^[+−-]\s?/, ""), ""],
              ["Net", String(d()?.month.shown || "").replace(/^\+\s?/, ""), d()?.month.tone === "neg" ? "neg" : ""],
            ]}
          />
          <Note text={`${String(d()?.month.rows[2].label || "")}: ${String(d()?.month.rows[2].shown || "")} net`} />
          <Acts>
            <LinkBtn label="Open the P&L" onPress={() => openBooks("reports")} />
          </Acts>
        </Card>
      </If>
    </View>
  );
}

function PersonCard() {
  const r = resource(
    () => ({ v: ver() }),
    () => rox.call("books_person_get", { person: CP.person }),
  );
  const d = () => r() as Any;
  return (
    <View>
      <CardWait res={r} title="Books · A person" />
      <If when={Boolean(d())}>
        <Card title={`Books · ${String(d()?.name || "")}`} right={<Pill tone="mute" label={d()?.sub || ""} />}>
          <Big text={`₱ ${String(d()?.kpi?.[0]?.shown || "0.00")}`} />
          <Note text={(d()?.kpi || []).slice(1).map((k: Any) => `${String(k.label)} ${String(k.shown)}`).join(" · ")} />
          <Each of={(d()?.recent || []).slice(0, 3)} key={rowKey}>
            {(x: Any) => (
              <Line
                label={`${String(x().date_short)} · ${String(x().title || "")}`}
                sub={x().sub || ""}
                value={x().status === "posted" ? x().amount_shown : ""}
                right={x().status !== "posted" ? <Pill tone={x().pill} label={x().status_label} /> : null}
              />
            )}
          </Each>
          <Acts>
            <LinkBtn label="Open in People" onPress={() => openBooks("people")} />
          </Acts>
        </Card>
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
  const title = side === "pay" ? "Books · To pay" : "Books · To collect";
  return (
    <View>
      <CardWait res={r} title={title} />
      <If when={Boolean(d())}>
        <Card title={title} right={<Pill tone="mute" label={String((d()?.rows || []).length)} />}>
          <Each of={d()?.rows || []} key={rowKey}>
            {(x: Any) => <Line label={x().who || x().name || ""} sub={`${String(x().doc || "")} · due ${String(x().due_short || "")}`} value={x().amount_shown} />}
          </Each>
          <Line label="Total" value={d()?.total_shown || ""} />
          <Acts>
            <LinkBtn label="Open Unpaid" onPress={() => openBooks("unpaid")} />
          </Acts>
        </Card>
      </If>
    </View>
  );
}

export default function Answer() {
  return (
    // a Scroll lays the card out at its natural height whatever the frame, so
    // it measures right for rox.fit; it never scrolls or bounces: it is a card
    <Scroll class={`flex-1 ${C.bg}`} bounces={false} scrollable={false}>
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
