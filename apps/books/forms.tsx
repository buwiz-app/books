// Forms: Books totals beside the BIR boxes they fill, for 2551Q, 1601-C and
// 1701-MS. A number opens the lines behind it. The 1701-MS tax option is
// read from the BIR draft (the assistant passes it); Books never sets it,
// and switching it here only previews which boxes change (drawing: Books
// Mini App › Forms; Books Tax Bridge › Map 3).

const formNow = signal(["2551Q", "1601C", "1701MS"].includes(String(PROPS.form)) ? String(PROPS.form) : "2551Q");
const taxOption = signal(String(PROPS.tax_option || ""));
const deduction = signal(String(PROPS.deduction || ""));
const lineageKey = signal("");

function formContext(form: string): Record<string, string> {
  if (form !== "1701MS") return {};
  const ctx: Record<string, string> = {};
  if (taxOption()) ctx["/taxpayer/tax_option"] = taxOption();
  if (taxOption() === "graduated" && deduction()) ctx["/taxpayer/deduction"] = deduction();
  return ctx;
}

const STATUS_TONE: Record<string, string> = { ready: "ok", ready_held: "ok", blocked: "draft", in_bir: "bir", confirmed: "bir", handed: "in", stale: "rev" };

/** "Hand to BIR draft": record the boxes, then attach them to the chat for
 * the assistant to fill into the BIR draft (source document). */
function handTo(form: string, period: string) {
  act("books_app_handoff_create", { form, period, form_context: formContext(form) })
    .then((r: Any) => {
      if (!rox.attach) return toast("Saved. This Rox cannot attach it to a chat.");
      const attached = rox.attach("proposal", r.ticket);
      return attached.then(() => toast("Attached to the chat · ask the assistant to fill the BIR draft")).catch((e: Any) => toast(errText(e)));
    })
    .catch(() => null);
}

function FormsPage(p: { go: (page: string, filter?: string) => void }) {
  const cards = resource(
    () => ({ v: ver(), t: taxOption(), d: deduction() }),
    () => rox.call("books_forms", { form_context: formContext("1701MS") }),
  );
  const view = resource(
    () => ({ v: ver(), f: formNow(), t: taxOption(), d: deduction() }),
    (k: Any) => rox.call("books_form_proposal", { form: k.f, form_context: formContext(k.f) }),
  );
  const lineage = resource(
    () => ({ k: lineageKey(), f: formNow(), v: ver() }),
    (k: Any) => (k.k ? rox.call("books_app_form_lineage", { form: k.f, key: k.k, form_context: formContext(k.f) }) : Promise.resolve(null)),
  );
  const v = () => view() as Any;
  const locked = () => (v()?.blocking || []).some((i: Any) => i.code === "GATE.PERIOD_LOCK");
  const o = () => v()?.owners || {};
  return (
    <View class="gap-3.5">
      <Head title="Forms" sub="Books totals for the BIR app">
        <View class="flex-row gap-2">
          <Btn label="2307s" onPress={() => p.go("certificates")} />
          <Btn label="Payroll" onPress={() => p.go("payroll")} />
        </View>
      </Head>
      <View class="flex-row gap-2.5">
        <Each of={(cards() as Any)?.cards || []} key={rowKey}>
          {(c: Any) => (
            <View
              class={`flex-1 rounded-xl border p-3 gap-1 ${C.panel} ${formNow() === c().form ? "border-[#6a46c9] dark:border-[#b79bff]" : C.line}`}
              onPress={() => {
                lineageKey.set("");
                formNow.set(c().form);
              }}
            >
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
          <Text class={`text-[12.5px] font-bold ${C.violet}`}>Read from the BIR draft:</Text>
          <Text class={`text-[12.5px] ${C.violet}`}>income tax option</Text>
          <Seg
            options={[
              ["graduated", "Graduated"],
              ["eight_percent", "8%"],
            ]}
            value={taxOption()}
            onPick={(x: string) => {
              lineageKey.set("");
              taxOption.set(x);
            }}
          />
          <If when={taxOption() === "graduated"}>
            <Text class={`text-[12.5px] ${C.violet}`}>deduction</Text>
            <Seg
              options={[
                ["osd", "OSD"],
                ["itemized", "Itemized"],
              ]}
              value={deduction()}
              onPick={(x: string) => deduction.set(x)}
            />
          </If>
          <Text class={`text-[12px] ${C.violet} opacity-80`}>{taxOption() ? "Books never sets this. Switch it to see which boxes change." : "Not read yet: ask the assistant to read it from the BIR draft, or pick one to preview."}</Text>
        </View>
      </If>
      <Loading res={view} />
      <If when={Boolean(v())}>
        <Panel>
          <View class={`flex-row items-center justify-between px-3.5 pt-3 pb-2 border-b ${C.line}`}>
            <Text class={`text-[13.5px] font-bold ${C.ink}`}>{`${v()?.code || ""} · ${v()?.period_label || ""}`}</Text>
            <Text class={`text-[12px] ${C.dim}`}>click a number to see where it comes from</Text>
          </View>
          <View class={`flex-row gap-3 px-3.5 py-1.5 border-b ${C.line}`}>
            <Text class={`flex-1 text-[10.5px] uppercase tracking-wider ${C.dim}`}>Books total</Text>
            <Text class={`w-[16px] text-[10.5px] ${C.dim}`} />
            <Text class={`w-[250px] text-[10.5px] uppercase tracking-wider ${C.dim}`}>BIR box</Text>
            <Text class={`w-[120px] text-right text-[10.5px] uppercase tracking-wider ${C.dim}`}>Value</Text>
          </View>
          <Each of={v()?.rows || []} key={rowKey}>
            {(r: Any) => (
              <View class={`flex-row items-center gap-3 px-3.5 py-2 border-b ${C.line} ${r().held ? "opacity-60" : ""}`}>
                <View class="flex-1 gap-0.5">
                  <Text class={`text-[13px] font-semibold ${C.ink}`}>{r().measure}</Text>
                  <If when={Boolean(r().sub)}>
                    <Text class={`text-[11.5px] ${C.mute}`} numberOfLines={2}>
                      {r().sub || ""}
                    </Text>
                  </If>
                  <If when={Boolean(r().flag) && !r().held}>
                    <Text class={`text-[11.5px] ${C.amber}`}>{r().flag || ""}</Text>
                  </If>
                  <If when={Boolean(r().held && r().why)}>
                    <Text class={`text-[11.5px] ${C.amber}`} numberOfLines={2}>
                      {`Held: ${String(r().why || "")}`}
                    </Text>
                  </If>
                  <If when={r().mode === "reference"}>
                    <Text class={`text-[11.5px] ${C.blue}`}>reference only</Text>
                  </If>
                </View>
                <Text class={`w-[16px] text-[13px] ${C.dim}`}>→</Text>
                <View class={`w-[250px] rounded-lg border px-2 py-1 ${r().held ? "border-dashed" : ""} ${C.line} ${C.raise}`}>
                  <Text class={`text-[11px] ${C.mute}`} numberOfLines={1}>
                    {r().item}
                  </Text>
                  <Text class={`text-[11px] font-mono ${C.violet}`} numberOfLines={1}>
                    {r().key}
                  </Text>
                </View>
                <View class="w-[120px] items-end">
                  <If when={(r().value_shown === null || r().value_shown === undefined) && Boolean(r().preview_shown)}>
                    <Text class={`text-[13px] font-mono ${C.dim}`}>{r().preview_shown || ""}</Text>
                    <Text class={`text-[10.5px] ${C.dim}`}>so far · preview</Text>
                  </If>
                  <If when={(r().value_shown === null || r().value_shown === undefined) && !r().preview_shown}>
                    <Pill tone="mute" label="Not sent" />
                  </If>
                  <If when={r().value_shown !== null && r().value_shown !== undefined}>
                    <View class={`rounded-md px-1.5 py-0.5 ${lineageKey() === r().key ? C.raise : ""}`} onPress={() => lineageKey.set(lineageKey() === r().key ? "" : r().key)}>
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
            <View class={`flex-row items-center justify-between px-3.5 pt-3 pb-2 border-b ${C.line}`}>
              <Text class={`text-[13px] font-bold ${C.ink}`}>{`Where ${lineageKey()} comes from`}</Text>
              <Btn label="Close" small onPress={() => lineageKey.set("")} />
            </View>
            <Each of={(lineage() as Any)?.rows || []} key={rowKey}>
              {(l: Any) => (
                <View class={`flex-row gap-3 px-3.5 py-1.5 border-b ${C.line}`}>
                  <Text class={`w-[70px] text-[12.5px] font-mono ${C.mute}`}>{l().date}</Text>
                  <Text class={`flex-1 text-[12.5px] ${C.ink}`} numberOfLines={1}>
                    {l().what}
                  </Text>
                  <Text class={`w-[120px] text-right text-[12.5px] font-mono ${C.ink}`}>{l().amount_shown}</Text>
                </View>
              )}
            </Each>
            <View class="flex-row gap-3 px-3.5 py-2">
              <Text class={`flex-1 text-[12.5px] font-semibold ${C.ink}`}>Total</Text>
              <Text class={`w-[120px] text-right text-[12.5px] font-mono font-semibold ${C.ink}`}>{(lineage() as Any)?.total_shown || ""}</Text>
            </View>
          </Panel>
        </If>
        <If when={(v()?.returns || []).length > 0}>
          <Panel pad class="gap-2">
            <Text class={`text-[13px] font-bold ${C.ink}`}>A return waits for your OK</Text>
            <Text class={`text-[12px] ${C.mute}`}>A credit note or a later reversal changes sales. OK it once you have checked it; the form counts it then.</Text>
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
            <Text class={`text-[11.5px] ${C.violet}`}>boxes Books fills</Text>
          </View>
          <Each
            of={[
              ["bir", "BIR computes"],
              ["profile", "from your BIR profile"],
              ["past", "from past returns"],
              ["you", "you type in BIR"],
            ]}
          >
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
            <Btn
              label={v()?.handoff ? "Refresh" : "Hand to BIR draft"}
              tone="vi"
              disabled={!(v()?.status === "ready" || v()?.status === "ready_held")}
              onPress={() => handTo(formNow(), v()?.period)}
            />
          </If>
        </Panel>
        <If when={Boolean(v()?.handoff)}>
          <Panel pad class="flex-row items-center gap-3">
            <Pill tone={STATUS_TONE[v()?.handoff?.status] || "mute"} label={v()?.handoff?.label || ""} />
            <Text class={`flex-1 text-[12.5px] ${C.mute}`}>{v()?.handoff?.detail || ""}</Text>
            <If when={v()?.handoff?.status === "handed"}>
              <Btn label="BIR shows these values" tone="go" onPress={() => act("books_app_handoff_confirm", { handoff: v()?.handoff?.ticket }).then(() => toast("Recorded: the BIR draft shows these values")).catch(() => null)} />
            </If>
          </Panel>
        </If>
        <CrossChecks />
        <If when={locked()}>
          <Text class={`text-[12px] ${C.dim}`}>{`${v()?.code || ""} can be handed over once ${v()?.period_label || "the period"} is closed. Until then it is a preview.`}</Text>
        </If>
      </If>
    </View>
  );
}

/** Cross-checks: two ways to the same total, ok or how far apart. */
function CrossChecks() {
  const data = resource(
    () => ({ v: ver() }),
    () => rox.call("books_cross_checks", {}),
  );
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
              <Text class={`w-[260px] text-right text-[12px] font-mono ${C.mute}`} numberOfLines={1}>
                {r().left_shown === r().right_shown ? r().left_shown : `${String(r().left_shown)} / ${String(r().right_shown)}`}
              </Text>
              <View class="w-[190px] items-end">
                <Pill tone={r().ok ? "ok" : "rev"} label={r().note} />
              </View>
            </View>
          )}
        </Each>
        <Text class={`text-[11.5px] px-3.5 py-2 ${C.dim}`}>Filed returns are compared when the assistant reads them from the BIR app.</Text>
      </Panel>
    </If>
  );
}
