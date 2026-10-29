// Quiet cards, the Notice class: one line and one action, from a routine. Counts and status only, no amounts,
// because nobody asked yet (owner decision; D22). Every number here comes from books_nudge, which carries none.

function TidyCard() {
  const r = resource(
    () => ({ v: ver() }),
    () => rox.call("books_nudge", { kind: "tidy", any_day: true }),
  );
  const d = () => r() as Any;
  const n = () => Number(d()?.counts.lines || 0);
  return (
    <View>
      <Wait res={r} />
      <If when={Boolean(d())}>
        <Body
          foot={
            <Foot status="The routine drafts only; you post">
              <Act label={n() > 0 ? `Review ${n()}` : "Open Ledger"} primary={n() > 0} onPress={() => openBooks("ledger")} />
            </Foot>
          }
        >
          <Meta text="This week" right={<Pill tone="mute" label="Mondays" />} />
          <Say text={`${n()} new bank line${n() === 1 ? "" : "s"} · ${String(d()?.counts.drafts || 0)} drafted · ${String(d()?.counts.posted || 0)} posted`} />
        </Body>
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
  const left = () => Number(d()?.left || 0);
  return (
    <View>
      <Wait res={r} />
      <If when={Boolean(d()?.closed)}>
        <Settled text={`${String(d()?.month_label || "")} is closed`} />
      </If>
      <If when={Boolean(d()) && !d()?.closed}>
        <Body
          foot={
            <Foot>
              <Act label="Start closing" primary onPress={() => openBooks("settings")} />
            </Foot>
          }
        >
          <Meta text={String(d()?.month_label || "")} right={<Pill tone="draft" label="Open" />} />
          <Say text={`${String(d()?.month_label || "")} is still open: ${left()} step${left() === 1 ? "" : "s"} left before you can close it.`} />
        </Body>
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
  const form = () => String(d()?.forms?.[0]?.title || "a hand-off").replace("BIR hand-off · ", "");
  return (
    <View>
      <Wait res={r} />
      <If when={Boolean(d()) && !d()?.count}>
        <Settled text="Every hand-off still matches the books" />
      </If>
      <If when={Boolean(d()?.count)}>
        <Body
          foot={
            <Foot>
              <Act label="Refresh for BIR" primary onPress={() => openBooks("forms")} />
            </Foot>
          }
        >
          <Meta text={form()} right={<Pill tone="rev" label="Out of date" />} />
          <Say text={`${String(d()?.forms?.[0]?.why || "A line changed")} after it was handed to BIR. ${String(d()?.count)} form${d()?.count === 1 ? "" : "s"} to refresh.`} />
        </Body>
      </If>
    </View>
  );
}

export default function Quiet() {
  return (
    // the card lays out at its natural height and reports it; it never scrolls or bounces
    <Scroll class={`flex-1 ${K.surface}`} bounces={false} scrollable={false}>
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
