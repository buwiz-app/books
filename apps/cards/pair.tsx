// The BIR pair card, a Decision card: a form's Books totals for the BIR app. A return waits for the owner's OK
// first; then Ready, Hand to BIR draft (attached to the chat for the assistant to fill with source document), In
// BIR draft, or Out of date. Books never shows a tax due: BIR does. The 1701-MS tax option is read from the BIR
// draft (props), never set here.

function FormCard() {
  const form = ["2551Q", "1601C", "1701MS"].includes(String(CP.form)) ? String(CP.form) : "2551Q";
  const ctx: Record<string, string> = {};
  if (form === "1701MS" && CP.tax_option) ctx["/taxpayer/tax_option"] = String(CP.tax_option);
  if (form === "1701MS" && CP.tax_option === "graduated" && CP.deduction) ctx["/taxpayer/deduction"] = String(CP.deduction);
  const r = read(
    () => ({ v: ver() }),
    () => rox.call("books_form_proposal", CP.period ? { form, period: CP.period, form_context: ctx } : { form, form_context: ctx }),
  );
  const d = () => r() as Any;
  const h = () => d()?.handoff;
  const returns = () => (d()?.returns || []) as Any[];
  const ready = () => d()?.status === "ready" || d()?.status === "ready_held";
  const period = () => String(d()?.period_label || "").split(" · ")[0];
  const pill = () =>
    h()?.status === "confirmed"
      ? ["bir", "In BIR draft"]
      : h()?.status === "stale"
        ? ["rev", "Out of date"]
        : h()?.status === "handed"
          ? ["in", "Handed over"]
          : [ready() ? "ok" : returns().length ? "rev" : "draft", d()?.status_label || ""];
  const hand = () =>
    act("books_app_handoff_create", { form, period: d()?.period, form_context: ctx })
      .then((x: Any) => (rox.attach ? rox.attach("proposal", x.ticket) : null))
      .catch(() => null);
  const o = () => d()?.owners || {};
  const status = () =>
    returns().length ? "Confirm the return first" : d()?.status === "blocked" ? `Can be handed over once ${period()} is closed` : h()?.status === "handed" ? "Check the values in the BIR draft" : "";
  return (
    <View>
      <Wait res={r} />
      <If when={Boolean(d())}>
        <Body
          foot={
            <Foot status={status()}>
              <If when={!returns().length && ready() && (!h() || h()?.status === "stale")}>
                <Act label={h() ? "Refresh for BIR" : "Hand to BIR draft"} primary onPress={hand} />
              </If>
              <If when={!returns().length && h()?.status === "handed"}>
                <Act label="BIR shows these values" primary onPress={() => act("books_app_handoff_confirm", { handoff: h()?.ticket }).catch(() => null)} />
              </If>
              <Act label="Open Forms" onPress={() => openBooks("forms")} />
            </Foot>
          }
        >
          <Meta
            text={`${String(d()?.code || "")} · ${period()}`}
            right={
              <View class="flex-row gap-1.5">
                <Pill tone="bir" label="BIR" />
                <Pill tone={pill()[0]} label={pill()[1]} />
              </View>
            }
          />
          <If when={returns().length > 0}>
            <Say text="A return was posted in this period. Confirm it before Books hands over sales totals." />
            <List>
              <Each of={returns()} key={rowKey}>
                {(x: Any) => (
                  <Row
                    label={`${String(x().date)} · ${String(x().what || "Return")}`}
                    right={<Act label="It's correct" onPress={() => act("books_app_return_ok", { entry: x().ticket }).catch(() => null)} />}
                  />
                )}
              </Each>
            </List>
          </If>
          <If when={returns().length === 0}>
            <If when={form === "1701MS"}>
              <Say text={CP.tax_option ? `Tax option from BIR: ${CP.tax_option === "eight_percent" ? "8%" : "graduated"}. Books reads it and never sets it.` : "Ask the assistant to read the tax option from the BIR draft."} dim />
            </If>
            <List>
              <Each of={(d()?.rows || []).filter((x: Any) => x.mode === "fill" || x.preview_shown || x.value_shown)} key={rowKey}>
                {(x: Any) => (
                  <Row
                    label={x().measure}
                    sub={`${String(x().item).includes(":") ? String(x().item).split(":")[0] : `Item ${String(x().item).split(" ")[0]}`} · ${String(x().key)}`}
                    value={x().value_shown || x().preview_shown || ""}
                    right={!x().value_shown && !x().preview_shown ? <Pill tone="mute" label="Not sent" /> : null}
                  />
                )}
              </Each>
            </List>
            <Say text={`${String(o().books)} boxes from Books · ${String(o().bir)} BIR computes · ${String(o().profile)} from your profile · ${String(o().you)} you type`} dim />
            <If when={Boolean(h()?.detail)}>
              <Say text={h()?.detail || ""} dim />
            </If>
          </If>
        </Body>
      </If>
    </View>
  );
}

export default function Pair() {
  return (
    // the card lays out at its natural height and reports it; it never scrolls or bounces
    <Scroll class={`flex-1 ${K.surface}`} bounces={false} scrollable={false}>
      <If when={CARD === "form"}>
        <FormCard />
      </If>
    </Scroll>
  );
}
