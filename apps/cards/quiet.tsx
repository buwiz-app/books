// Quiet cards: they appear on their own, from a routine. Counts and status
// only, no amounts, because nobody asked yet (owner decision; D22). Every
// number here comes from books_nudge, which carries no amount.

function TidyCard() {
  const r = resource(
    () => ({ v: ver() }),
    () => rox.call("books_nudge", { kind: "tidy", any_day: true }),
  );
  const d = () => r() as Any;
  return (
    <View>
      <CardWait res={r} title="Books · Weekly tidy-up" />
      <If when={Boolean(d())}>
        <Card title="Books · Weekly tidy-up" right={<Pill tone="mute" label="Mondays" />}>
          <Strip
            items={[
              ["New bank lines", d()?.counts.lines, ""],
              ["Drafted", d()?.counts.drafts, ""],
              ["Posted", d()?.counts.posted, ""],
            ]}
          />
          <Note text="The routine drafts only. Open the Ledger to post." />
          <Acts>
            <Btn label={`Review ${String(d()?.counts.lines || 0)}`} onPress={() => openBooks("ledger")} />
          </Acts>
        </Card>
      </If>
    </View>
  );
}

function MonthEndCard() {
  const r = resource(
    () => ({ v: ver() }),
    () => rox.call("books_nudge", { kind: "month_end" }),
  );
  const d = () => r() as Any;
  return (
    <View>
      <CardWait res={r} title="Books · Month end" />
      <If when={Boolean(d())}>
        <Card title="Books · Month end" right={<Pill tone={d()?.closed ? "ok" : "mute"} label={d()?.closed ? "Closed" : String(d()?.month_label || "")} />}>
          <Note text={d()?.closed ? `${String(d()?.month_label)} is closed.` : `${String(d()?.month_label || "")} is still open. ${String(d()?.left)} step${d()?.left === 1 ? "" : "s"} left before closing.`} />
          <If when={!d()?.closed}>
            <Acts>
              <Btn label="Start closing" onPress={() => openBooks("settings")} />
            </Acts>
          </If>
        </Card>
      </If>
    </View>
  );
}

function HandoffCard() {
  const r = resource(
    () => ({ v: ver() }),
    () => rox.call("books_nudge", { kind: "bir_pair" }),
  );
  const d = () => r() as Any;
  return (
    <View>
      <CardWait res={r} title="Books · BIR hand-off" />
      <If when={Boolean(d())}>
        <Card title="Books · BIR hand-off" right={<Pill tone={d()?.count ? "rev" : "ok"} label={d()?.count ? "Out of date" : "Up to date"} />}>
          <Note text={d()?.count ? `${String(d()?.forms?.[0]?.why || "A line changed")} after ${String(d()?.forms?.[0]?.title || "a hand-off").replace("BIR hand-off · ", "")} was handed over. ${String(d()?.count)} form${d()?.count === 1 ? "" : "s"} to refresh.` : "Every hand-off still matches the books."} />
          <If when={Boolean(d()?.count)}>
            <Acts>
              <Btn label="Refresh for BIR" tone="vi" onPress={() => openBooks("forms")} />
            </Acts>
          </If>
        </Card>
      </If>
    </View>
  );
}

export default function Quiet() {
  return (
    // a Scroll lays the card out at its natural height whatever the frame, so
    // it measures right for rox.fit; it never scrolls or bounces: it is a card
    <Scroll class={`flex-1 ${C.bg}`} bounces={false} scrollable={false}>
      <If when={CARD === "tidy"}>
        <TidyCard />
      </If>
      <If when={CARD === "month_end"}>
        <MonthEndCard />
      </If>
      <If when={CARD === "handoff"}>
        <HandoffCard />
      </If>
    </Scroll>
  );
}
