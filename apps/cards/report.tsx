// Report card (480): the P&L, this month against last, as the user asked
// ("How did we do?"; the template /books-results).

function PlCard() {
  const r = resource(
    () => ({ v: ver() }),
    () => rox.call("books_report", CP.period ? { kind: "pl", period: CP.period } : { kind: "pl" }),
  );
  const d = () => r() as Any;
  const rows = () => (d()?.sections || []).flatMap((s: Any) => s.rows || []);
  return (
    <View>
      <CardWait res={r} title="Books · P&L" />
      <If when={Boolean(d())}>
        <Card title="Books · P&L" right={<Pill tone="mute" label={`${String(d()?.columns?.[0] || "")} vs ${String(d()?.columns?.[1] || "")}`} />}>
          <Each of={rows()} key={rowKey}>
            {(x: Any) => <Line label={x().name || ""} sub={x().review || ""} value={`${String(x().shown)} · ${String(x().prev_shown)}`} />}
          </Each>
          <View class={`flex-row items-center gap-2 py-1 border-t ${C.line}`}>
            <Text class={`flex-1 text-[12.5px] font-bold ${C.ink}`}>{d()?.net.label || "Net income"}</Text>
            <Text class={`text-[12.5px] font-mono font-bold ${C.ink}`}>{`${String(d()?.net.shown || "")} · ${String(d()?.net.prev_shown || "")}`}</Text>
          </View>
          <Note text={`${String(d()?.basis_label || "")} · ${String(d()?.columns?.[0] || "")} first, then ${String(d()?.columns?.[1] || "")}`} />
          <Acts>
            <LinkBtn label="Open Reports" onPress={() => openBooks("reports")} />
          </Acts>
        </Card>
      </If>
    </View>
  );
}

export default function Report() {
  return (
    // a Scroll lays the card out at its natural height whatever the frame, so
    // it measures right for rox.fit; it never scrolls or bounces: it is a card
    <Scroll class={`flex-1 ${C.bg}`} bounces={false} scrollable={false}>
      <If when={CARD === "pl"}>
        <PlCard />
      </If>
    </Scroll>
  );
}
