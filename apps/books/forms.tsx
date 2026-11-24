// Tax hand-off: live Books totals for Buwiz Forms (COA nesting lives in Accounts).
const formNow = signal(["2551Q", "1601C", "1701MS"].includes(String(PROPS.form)) ? String(PROPS.form) : "2551Q");
const taxOption = signal(String(PROPS.tax_option || ""));
const deduction = signal(String(PROPS.deduction || ""));
const lineageKey = signal("");
const lineageItem = signal("");

function formContext(f: string): Record<string, string> {
  if (f !== "1701MS") return {};
  const c: Record<string, string> = {};
  if (taxOption()) c["/taxpayer/tax_option"] = taxOption();
  if (taxOption() === "graduated" && deduction()) c["/taxpayer/deduction"] = deduction();
  return c;
}

const STATUS_TONE: Record<string, string> = { ready: "ok", ready_held: "ok", blocked: "draft", in_bir: "bir", confirmed: "bir", handed: "in", stale: "rev" };

function handTo(form: string, period: string) {
  act("books_app_handoff_create", { form, period, form_context: formContext(form) })
    .then((r: Any) => {
      if (!buwiz.attach) return toast("Saved. This Buwiz Bot cannot attach it to a chat.");
      return buwiz.attach("proposal", r.ticket).then(() => toast("Attached to the chat")).catch((e: Any) => toast(errText(e)));
    })
    .catch(() => null);
}

function FormsPage(p: { go: (page: string, filter?: string) => void }) {
  const cards = resource(() => ({ v: ver(), t: taxOption(), d: deduction() }), () => buwiz.call("books_forms", { form_context: formContext("1701MS") }));
  const view = resource(() => ({ v: ver(), f: formNow(), t: taxOption(), d: deduction() }), (k: Any) => buwiz.call("books_form_proposal", { form: k.f, form_context: formContext(k.f) }));
  const lineage = resource(() => ({ k: lineageKey(), f: formNow(), v: ver() }), (k: Any) => (k.k ? buwiz.call("books_app_form_lineage", { form: k.f, key: k.k, form_context: formContext(k.f) }) : Promise.resolve(null)));

  const v = () => view() as Any;
  const locked = () => (v()?.blocking || []).some((i: Any) => i.code === "GATE.PERIOD_LOCK");
  const o = () => v()?.owners || {};

  const th = `text-[10.5px] uppercase tracking-wider ${C.dim}`;
  const rowCls = `flex-row items-center gap-3 px-3.5 py-2 border-b ${C.line}`;
  const hdrCls = `flex-row items-center justify-between px-3.5 pt-3 pb-2 border-b ${C.line}`;
  const numCls = `w-[120px] text-right text-[12.5px] font-mono ${C.ink}`;
  return (
    <View class="gap-3.5">
      <Head title="Tax hand-off" sub="Live Books totals for Buwiz Forms · COA nesting lives in Accounts">
        <View class="flex-row gap-2">
          <Btn label="Accounts" onPress={() => p.go("accounts")} />
          <Btn label="2307s" onPress={() => p.go("certificates")} />
          <Btn label="Payroll" onPress={() => p.go("payroll")} />
        </View>
      </Head>

      <View class="flex-row gap-2.5">
        <Each of={(cards() as Any)?.cards || []} key={rowKey}>
          {(c: Any) => (
            <View class={`flex-1 rounded-xl border p-3 gap-1 ${C.hoverBg} ${C.panel} ${formNow() === c().form ? "border-[#6a46c9] dark:border-[#b79bff]" : C.line}`} onPress={() => { lineageKey.set(""); lineageItem.set(""); formNow.set(c().form); }}>
              <View class="flex-row items-center justify-between">
                <Text class={`text-[12px] font-mono font-bold ${C.violet}`}>{c().code}</Text>
                <Pill tone={STATUS_TONE[c().status] || "mute"} label={c().status_label} />
              </View>
              <Text class={`text-[13.5px] font-bold ${C.ink}`}>{c().title}</Text>
              <Text class={`text-[12px] ${C.dim}`}>{c().period_label}</Text>
            </View>
          )}
        </Each>
      </View>

      <If when={formNow() === "1701MS"}>
        <View class={`flex-row items-center gap-2.5 rounded-xl border px-3 py-2 flex-wrap ${PILL.bir} border-[#6a46c9]/30`}>
          <Text class={`text-[12.5px] font-bold ${C.violet}`}>BIR draft:</Text>
          <Seg options={[["graduated", "Graduated"], ["eight_percent", "8%"]]} value={taxOption()} onPick={(x: string) => { lineageKey.set(""); lineageItem.set(""); taxOption.set(x); }} />
          <If when={taxOption() === "graduated"}><Seg options={[["osd", "OSD"], ["itemized", "Itemized"]]} value={deduction()} onPick={(x: string) => deduction.set(x)} /></If>
        </View>
      </If>

      <Loading res={view} />
      <If when={Boolean(v())}>
        <Panel>
          <View class={hdrCls}>
            <Text class={`text-[13.5px] font-bold ${C.ink}`}>{`${v()?.code || ""} · ${v()?.period_label || ""}`}</Text>
            <Text class={`text-[12px] ${C.dim}`}>click an amount for its lines</Text>
          </View>
          <View class={`flex-row gap-3 px-3.5 py-1.5 border-b ${C.line}`}>
            <Text class={`flex-1 ${th}`}>Books total</Text>
            <Text class={`w-[16px] ${th}`} />
            <Text class={`w-[260px] ${th}`}>BIR form line</Text>
            <Text class={`w-[120px] text-right ${th}`}>Amount</Text>
          </View>
          <Each of={v()?.rows || []} key={rowKey}>
            {(r: Any) => (
              <View class={`${rowCls} ${r().held ? C.subtle : ""}`}>
                <View class="flex-1 gap-0.5">
                  <Text class={`text-[13px] font-semibold ${r().held ? C.mute : C.ink}`}>{r().measure}</Text>
                  <If when={Boolean(r().sub)}><Text class={`text-[11.5px] ${C.mute} line-clamp-2`}>{r().sub || ""}</Text></If>
                  <If when={Boolean(r().flag) && !r().held}><Text class={`text-[11.5px] ${C.amber}`}>{r().flag || ""}</Text></If>
                  <If when={Boolean(r().held && r().why)}><Text class={`text-[11.5px] ${C.amber} line-clamp-2`}>{`Held: ${String(r().why || "")}`}</Text></If>
                  <If when={r().mode === "reference"}><Text class={`text-[11.5px] ${C.blue}`}>reference only</Text></If>
                </View>
                <Text class={`w-[16px] text-[13px] ${C.mute}`}>→</Text>
                <View class={`w-[260px] rounded-lg border px-2.5 py-1.5 ${r().held ? "border-dashed" : ""} ${C.line} ${C.raise}`}>
                  <Text class={`text-[12px] font-medium ${r().held ? C.mute : C.ink} line-clamp-2`}>{r().item}</Text>
                </View>
                <View class="w-[120px] items-end">
                  <If when={r().value_shown == null && Boolean(r().preview_shown)}>
                    <Text class={`text-[13px] font-mono ${C.mute}`}>{r().preview_shown || ""}</Text>
                    <Text class={`text-[10.5px] ${C.mute}`}>preview</Text>
                  </If>
                  <If when={r().value_shown == null && !r().preview_shown}><Pill tone="mute" label="Not sent" /></If>
                  <If when={r().value_shown != null}>
                    <View accessibilityLabel="Show where this comes from" accessibilityValue={{ text: r().value_shown || "" }} class={`rounded-md px-1.5 py-0.5 ${C.hoverBg} ${lineageKey() === r().key ? C.raise : ""}`} onPress={() => { const same = lineageKey() === r().key; lineageKey.set(same ? "" : r().key); lineageItem.set(same ? "" : r().item || r().measure || ""); }}>
                      <Text class={`text-[13px] font-mono font-semibold underline ${C.ink}`}>{r().value_shown || ""}</Text>
                    </View>
                  </If>
                </View>
              </View>
            )}
          </Each>
        </Panel>

        <If when={Boolean(lineageKey()) && Boolean(lineage())}>
          <Panel>
            <View class={hdrCls}>
              <Text class={`text-[13px] font-bold ${C.ink}`}>{`Lines behind ${lineageItem() || "this line"}`}</Text>
              <Btn label="Close" small onPress={() => { lineageKey.set(""); lineageItem.set(""); }} />
            </View>
            <Each of={(lineage() as Any)?.rows || []} key={rowKey}>
              {(l: Any) => (
                <View class={`flex-row gap-3 px-3.5 py-1.5 border-b ${C.line}`}>
                  <Text class={`w-[70px] text-[12.5px] font-mono ${C.mute}`}>{l().date}</Text>
                  <Text class={`flex-1 text-[12.5px] ${C.ink} truncate`}>{l().what}</Text>
                  <Text class={numCls}>{l().amount_shown}</Text>
                </View>
              )}
            </Each>
            <View class="flex-row gap-3 px-3.5 py-2">
              <Text class={`flex-1 text-[12.5px] font-semibold ${C.ink}`}>Total</Text>
              <Text class={`font-semibold ${numCls}`}>{(lineage() as Any)?.total_shown || ""}</Text>
            </View>
          </Panel>
        </If>

        <If when={(v()?.returns || []).length > 0}>
          <Panel pad class="gap-2">
            <Text class={`text-[13px] font-bold ${C.ink}`}>A return waits for your OK</Text>
            <Each of={v()?.returns || []} key={rowKey}>
              {(x: Any) => (
                <View class="flex-row items-center gap-3">
                  <Text class={`w-[70px] text-[12.5px] font-mono ${C.mute}`}>{x().date}</Text>
                  <Text class={`flex-1 text-[12.5px] ${C.ink}`}>{x().what || "Return"}</Text>
                  <Btn label="OK the return" tone="go" small onPress={() => act("books_app_return_ok", { entry: x().ticket }).catch(() => null)} />
                </View>
              )}
            </Each>
          </Panel>
        </If>

        <View class="flex-row gap-2">
          <View class={`flex-1 rounded-xl border p-2.5 ${PILL.bir} border-[#6a46c9]/30`}>
            <Text class={`text-[18px] font-bold ${C.violet}`}>{String(o().books ?? "")}</Text>
            <Text class={`text-[11.5px] ${C.violet}`}>lines Books fills</Text>
          </View>
          <Each of={[["bir", "Forms computes"], ["profile", "Tax Profile"], ["past", "past returns"], ["you", "typed in Forms"]]}>
            {(x: Any) => (
              <View class={`flex-1 rounded-xl border p-2.5 ${C.panel} ${C.line}`}>
                <Text class={`text-[18px] font-bold ${C.ink}`}>{String(o()[x()[0]] ?? "")}</Text>
                <Text class={`text-[11.5px] ${C.mute}`}>{x()[1]}</Text>
              </View>
            )}
          </Each>
        </View>

        <Panel pad class="flex-row items-center gap-3">
          <Text class={`flex-1 text-[12.5px] ${C.mute}`}>{v()?.bir_note || ""}</Text>
          <If when={!v()?.handoff || v()?.handoff?.status === "stale"}>
            <Btn label={v()?.handoff ? "Refresh" : "Hand to BIR draft"} tone="vi" disabled={!(v()?.status === "ready" || v()?.status === "ready_held")} onPress={() => handTo(formNow(), v()?.period)} />
          </If>
        </Panel>

        <If when={Boolean(v()?.handoff)}>
          <Panel pad class="flex-row items-center gap-3">
            <Pill tone={STATUS_TONE[v()?.handoff?.status] || "mute"} label={v()?.handoff?.label || ""} />
            <Text class={`flex-1 text-[12.5px] ${C.mute}`}>{v()?.handoff?.detail || ""}</Text>
            <If when={v()?.handoff?.status === "handed"}>
              <Btn label="BIR shows these values" tone="go" onPress={() => act("books_app_handoff_confirm", { handoff: v()?.handoff?.ticket }).then(() => toast("Recorded")).catch(() => null)} />
            </If>
          </Panel>
        </If>

        <CrossChecks />
        <If when={locked()}><Text class={`text-[12px] ${C.dim}`}>{`Close ${v()?.period_label || "the period"} to hand over.`}</Text></If>
      </If>
    </View>
  );
}

function CrossChecks() {
  const data = resource(() => ({ v: ver() }), () => buwiz.call("books_cross_checks", {}));
  const d = () => data() as Any;
  return (
    <If when={Boolean(d())}>
      <Panel>
        <View class={`flex-row items-center justify-between px-3.5 pt-3 pb-2 border-b ${C.line}`}>
          <Text class={`text-[13px] font-bold ${C.ink}`}>Cross-checks</Text>
          <Text class={`text-[12px] ${C.dim}`}>{d()?.differences ? `${String(d()?.differences)} to look at` : "all agree"}</Text>
        </View>
        <Each of={d()?.rows || []} key={rowKey}>
          {(r: Any) => (
            <View class={`flex-row items-center gap-3 px-3.5 py-1.5 border-b ${C.line}`}>
              <Text class={`flex-1 text-[12.5px] ${C.ink}`}>{r().check}</Text>
              <Text class={`w-[260px] text-right text-[12px] font-mono ${C.mute} truncate`}>{r().left_shown === r().right_shown ? r().left_shown : `${String(r().left_shown)} / ${String(r().right_shown)}`}</Text>
              <View class="w-[190px] items-end"><Pill tone={r().compared === false ? "mute" : r().ok ? "ok" : "rev"} label={r().note} /></View>
            </View>
          )}
        </Each>
      </Panel>
    </If>
  );
}
