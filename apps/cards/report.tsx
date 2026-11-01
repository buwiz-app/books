// Report card, a List card: the P&L, this period against the last, as the user asked ("How did we do?"; the
// template /books-results). Two number columns: this period, and the last one dimmed.

function PlCard() {
  const r = read(
    () => ({ v: ver() }),
    () => buwiz.call("books_report", CP.period ? { kind: "pl", period: CP.period } : { kind: "pl" }),
  );
  const d = () => r() as Any;
  const rows = () => (d()?.sections || []).flatMap((s: Any) => s.rows || []);
  const cols = () => [String(d()?.columns?.[0] || ""), String(d()?.columns?.[1] || "")];
  return (
    <View>
      <Wait res={r} />
      <If when={Boolean(d())}>
        <Body
          foot={
            <Foot>
              <Act label="Open Reports" onPress={() => openBooks("reports")} />
            </Foot>
          }
        >
          <Meta text={`${cols()[0]} against ${cols()[1]}`} right={<Pill tone="mute" label={d()?.basis_label || ""} />} />
          <View class="flex-row items-center gap-3">
            <Text class={`flex-1 text-[11px] uppercase tracking-wider ${C.mute}`}>Account</Text>
            <Text class={`w-[96px] text-right text-[11px] uppercase tracking-wider ${C.mute}`}>{cols()[0]}</Text>
            <Text class={`w-[96px] text-right text-[11px] uppercase tracking-wider ${C.mute}`}>{cols()[1]}</Text>
          </View>
          <List>
            <Each of={rows()} key={rowKey}>
              {(x: Any) => <Row label={x().name || ""} sub={x().review || ""} value={String(x().shown)} prev={String(x().prev_shown)} />}
            </Each>
          </List>
          <Row label={d()?.net.label || "Net income"} value={String(d()?.net.shown || "")} prev={String(d()?.net.prev_shown || "")} strong />
        </Body>
      </If>
    </View>
  );
}

export default function Report() {
  return (
    // the card lays out at its natural height and reports it; it never scrolls or bounces
    <Scroll class={`flex-1 ${K.surface}`} bounces={false} scrollable={false}>
      <If when={CARD === "pl"}>
        <PlCard />
      </If>
    </Scroll>
  );
}
