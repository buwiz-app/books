// The BIR pair card (480): a form's Books totals for the BIR app. A return
// waits for the owner's OK first; then Ready, Hand to BIR draft (attached
// to the chat for the assistant to fill with source document), In BIR
// draft, or Out of date. Books never shows a tax due: BIR does. The 1701-MS
// tax option is read from the BIR draft (props), never set here.

function FormCard() {
  const form = ["2551Q", "1601C", "1701MS"].includes(String(CP.form)) ? String(CP.form) : "2551Q";
  const ctx: Record<string, string> = {};
  if (form === "1701MS" && CP.tax_option) ctx["/taxpayer/tax_option"] = String(CP.tax_option);
  if (form === "1701MS" && CP.tax_option === "graduated" && CP.deduction) ctx["/taxpayer/deduction"] = String(CP.deduction);
  const r = resource(
    () => ({ v: ver() }),
    () => rox.call("books_form_proposal", CP.period ? { form, period: CP.period, form_context: ctx } : { form, form_context: ctx }),
  );
  const d = () => r() as Any;
  const h = () => d()?.handoff;
  const pill = () =>
    h()?.status === "confirmed" ? ["bir", "In BIR draft"] : h()?.status === "stale" ? ["rev", "Out of date"] : h()?.status === "handed" ? ["in", "Handed over"] : [d()?.status === "ready" || d()?.status === "ready_held" ? "ok" : d()?.returns?.length ? "rev" : "draft", d()?.status_label || ""];
  const hand = () =>
    act("books_app_handoff_create", { form, period: d()?.period, form_context: ctx })
      .then((x: Any) => (rox.attach ? rox.attach("proposal", x.ticket) : null))
      .catch(() => null);
  const o = () => d()?.owners || {};
  return (
    <View>
      <CardWait res={r} title="Books · For the BIR app" />
      <If when={Boolean(d())}>
        <Card title={`Books · ${String(d()?.code || "")} · ${String(d()?.period_label || "").split(" · ")[0]}`} right={<Pill tone={pill()[0]} label={pill()[1]} />}>
          <If when={(d()?.returns || []).length > 0}>
            <Note text="A return was posted in this period. Confirm it before Books hands over sales totals." />
            <Each of={d()?.returns || []} key={rowKey}>
              {(x: Any) => (
                <Line
                  label={`${String(x().date)} · ${String(x().what || "Return")}`}
                  right={<Btn label="It's correct" tone="go" small onPress={() => act("books_app_return_ok", { entry: x().ticket }).catch(() => null)} />}
                />
              )}
            </Each>
          </If>
          <If when={(d()?.returns || []).length === 0}>
            <If when={form === "1701MS"}>
              <Note text={CP.tax_option ? `From BIR: ${CP.tax_option === "eight_percent" ? "8%" : "Graduated"} (Books reads it, never sets it)` : "Ask the assistant to read the tax option from the BIR draft."} />
            </If>
            <Each of={(d()?.rows || []).filter((x: Any) => x.mode === "fill" || x.preview_shown || x.value_shown)} key={rowKey}>
              {(x: Any) => (
                <Line
                  label={x().measure}
                  sub={`${String(x().item).includes(":") ? String(x().item).split(":")[0] : `Item ${String(x().item).split(" ")[0]}`} · ${String(x().key)}`}
                  value={x().value_shown || x().preview_shown || ""}
                  right={!x().value_shown && !x().preview_shown ? <Pill tone="mute" label="Not sent" /> : null}
                />
              )}
            </Each>
            <View class={`rounded-lg px-2 py-1.5 ${PILL.bir}`}>
              <Text class={`text-[11.5px] ${C.violet}`}>{`${String(o().books)} boxes from Books · ${String(o().bir)} BIR computes · ${String(o().profile)} from your profile · ${String(o().you)} you type`}</Text>
            </View>
            <If when={Boolean(h())}>
              <Note text={h()?.detail || ""} />
            </If>
            <Acts>
              <If when={(d()?.status === "ready" || d()?.status === "ready_held") && (!h() || h()?.status === "stale")}>
                <Btn label={h() ? "Refresh for BIR" : "Hand to BIR draft"} tone="vi" onPress={hand} />
              </If>
              <If when={h()?.status === "handed"}>
                <Btn label="BIR shows these values" tone="go" onPress={() => act("books_app_handoff_confirm", { handoff: h()?.ticket }).catch(() => null)} />
              </If>
              <LinkBtn label="Where do these come from?" onPress={() => openBooks("forms")} />
            </Acts>
            <If when={d()?.status === "blocked"}>
              <Note text={`Can be handed over once ${String(d()?.period_label || "the period").split(" · ")[0]} is closed.`} />
            </If>
          </If>
        </Card>
      </If>
    </View>
  );
}

export default function Pair() {
  return (
    // a Scroll lays the card out at its natural height whatever the frame, so
    // it measures right for rox.fit; it never scrolls or bounces: it is a card
    <Scroll class={`flex-1 ${C.bg}`} bounces={false} scrollable={false}>
      <If when={CARD === "form"}>
        <FormCard />
      </If>
    </Scroll>
  );
}
