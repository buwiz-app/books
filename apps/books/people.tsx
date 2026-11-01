// People: one page with a kind switch (banks and cards, vendors,
// customers, employees, owners, lenders, government). Each person shows
// what was paid or received and their last entries. No TIN: the BIR app
// keeps them.

const peopleKind = signal("vendors");
const personPick = signal("");

function Avatar(p: { initials: string; color: string; big?: boolean }) {
  return (
    <View class={`${p.big ? "w-10 h-10" : "w-8 h-8"} rounded-full items-center justify-center`} style={{ backgroundColor: p.color }}>
      <Text class={`${p.big ? "text-[14px]" : "text-[11px]"} font-bold text-white`}>{p.initials}</Text>
    </View>
  );
}

const stmtKind = signal("year");
const stmtPeriod = signal("");

const signed = (sign: string, shown: unknown) => {
  const t = String(shown ?? "");
  return t === "" || t === "0.00" ? t : `${sign}${t}`;
};

// One person's statement (books_report kind ps): open items with days
// late and the aging strip, then the period's money in and out.
function Statement(p: { person: string }) {
  const data = resource(
    () => ({ v: ver(), t: p.person, pk: stmtKind(), per: stmtPeriod() }),
    (k: Any) => {
      const args: Record<string, string> = { kind: "ps", person: k.t, period_kind: k.pk };
      if (k.per) args.period = k.per;
      return buwiz.call("books_report", args);
    },
  );
  const s = () => data() as Any;
  const employee = () => s()?.person?.kind === "employee";
  const copy = () => {
    const args: Record<string, string> = { kind: "ps", person: p.person, period_kind: stmtKind() };
    if (stmtPeriod()) args.period = stmtPeriod();
    buwiz.call("books_report_csv", args).then(
      (r: Any) => {
        if (buwiz.copy) {
          buwiz.copy(String(r.csv));
          toast(`${String(r.file_name)} copied`);
        } else toast("Copying is not available in this Buwiz Bot.");
      },
      (e: Any) => toast(errText(e)),
    );
  };
  const items = () => [...(s()?.open?.collect || []), ...(s()?.open?.pay || [])];
  const strip = () =>
    (s()?.buckets?.columns || []).map((label: string, i: number) => ({
      label,
      collect: signed("+", s()?.buckets?.collect?.[i]),
      pay: signed("−", s()?.buckets?.pay?.[i]),
    }));
  return (
    <View class={`gap-2 pt-2 border-t ${C.line}`}>
      <View class="flex-row items-center gap-2">
        <Text class={`flex-1 text-[13px] font-semibold ${C.ink}`}>Statement</Text>
        <Seg
          options={[
            ["year", "Year"],
            ["quarter", "Quarter"],
            ["month", "Month"],
          ]}
          value={stmtKind()}
          onPick={(v: string) => {
            stmtKind.set(v);
            stmtPeriod.set("");
          }}
        />
        <Btn label="‹" small onPress={() => stmtPeriod.set(s()?.period?.prev || "")} />
        <Text class={`text-[12.5px] ${C.ink}`}>{s()?.period?.label || ""}</Text>
        <Btn label="›" small onPress={() => stmtPeriod.set(s()?.period?.next || "")} />
        <Btn label="Copy statement" small onPress={copy} />
      </View>
      <Loading res={data} />
      <If when={Boolean(s()?.basis_note) && !employee()}>
        <Text class={`text-[11.5px] ${C.dim}`}>{s()?.basis_note || ""}</Text>
      </If>
      <If when={Boolean(s()) && !employee()}>
        <View class="gap-1">
          <Text class={`text-[12px] ${C.dim}`}>Open items</Text>
          <If when={items().length === 0}>
            <Text class={`text-[12px] ${C.dim}`}>Nothing open.</Text>
          </If>
          <Each of={items()} key={rowKey}>
            {(r: Any) => (
              <View class={`flex-row gap-3 py-1.5 border-t items-center ${C.line}`}>
                <View class={`rounded-full px-2 py-0.5 ${r().side === "collect" ? C.greenBg : C.redBg}`}>
                  <Text class="text-[11px] text-white">{r().side === "collect" ? "To collect" : "To pay"}</Text>
                </View>
                <Text class={`flex-1 text-[12.5px] ${C.ink}`}>Due {r().due_shown}</Text>
                <Text class={`w-[90px] text-[12.5px] ${C.dim}`}>{r().days_late > 0 ? `${String(r().days_late)} days late` : "Current"}</Text>
                <Text class={`w-[110px] text-right text-[12.5px] font-mono ${C.ink}`}>{r().amount_shown}</Text>
              </View>
            )}
          </Each>
          <View class="flex-row gap-2 pt-1">
            <Each of={strip()} key={rowKey}>
              {(c: Any) => (
                <View class={`flex-1 rounded-[8px] p-2 ${C.raise}`}>
                  <Text class={`text-[11px] ${C.dim}`}>{c().label}</Text>
                  <Text class={`text-[12px] font-mono ${C.green}`}>{c().collect}</Text>
                  <Text class={`text-[12px] font-mono ${C.ink}`}>{c().pay}</Text>
                </View>
              )}
            </Each>
          </View>
        </View>
      </If>
      <If when={Boolean(s())}>
        <View>
          <Text class={`text-[12px] ${C.dim}`}>Activity</Text>
          <If when={(s()?.activity?.rows || []).length === 0}>
            <Text class={`text-[12px] ${C.dim}`}>No entries in this period.</Text>
          </If>
          <Each of={s()?.activity?.rows || []} key={rowKey}>
            {(r: Any) => (
              <View class={`flex-row gap-3 py-1.5 border-t ${C.line}`} onPress={() => drawerEntry.set(r().entry)}>
                <Text class={`w-[52px] text-[12.5px] font-mono ${C.ink}`}>{r().date_shown}</Text>
                <Text class={`flex-1 text-[12.5px] ${C.ink}`} numberOfLines={1}>
                  {r().what}
                </Text>
                <Text class={`w-[100px] text-right text-[12.5px] font-mono ${C.green}`}>{r().in_shown}</Text>
                <Text class={`w-[100px] text-right text-[12.5px] font-mono ${C.ink}`}>{r().out_shown}</Text>
              </View>
            )}
          </Each>
          <View class={`flex-row gap-3 py-1.5 border-t ${C.line}`}>
            <Text class={`flex-1 text-[12.5px] font-semibold ${C.ink}`}>Total · net {s()?.activity?.net_shown || ""}</Text>
            <Text class={`w-[100px] text-right text-[12.5px] font-bold font-mono ${C.ink}`}>{s()?.activity?.total_in_shown || ""}</Text>
            <Text class={`w-[100px] text-right text-[12.5px] font-bold font-mono ${C.ink}`}>{s()?.activity?.total_out_shown || ""}</Text>
          </View>
        </View>
      </If>
    </View>
  );
}

function PeoplePage(p: { go: (page: string, filter?: string) => void }) {
  const list = resource(
    () => ({ v: ver(), k: peopleKind() }),
    (k: Any) => buwiz.call("books_people_list", { kind: k.k }),
  );
  const l = () => list() as Any;
  const current = computed(() => {
    const rows = l()?.rows || [];
    return rows.find((r: Any) => r.ticket === personPick()) || rows[0] || null;
  });
  const detail = resource(
    () => ({ v: ver(), t: current()?.ticket || "" }),
    (k: Any) => (k.t ? buwiz.call("books_person_get", { person: k.t }) : Promise.resolve(null)),
  );
  const d = () => detail() as Any;
  const adding = signal(false);
  const name = signal("");
  const add = () => {
    if (!name().trim()) return;
    act("books_app_person_create", { kind: peopleKind(), name: name().trim() })
      .then((r: Any) => {
        personPick.set(r.person);
        name.set("");
        adding.set(false);
      })
      .catch(() => null);
  };
  return (
    <View class="gap-3.5">
      <Head title="People">
        <If when={peopleKind() !== "banks"}>
          <Btn label="+ New" tone="go" onPress={() => adding.set(!adding())} />
        </If>
      </Head>
      <View class="flex-row gap-1.5 flex-wrap">
        <Each of={l()?.kinds || []} key={rowKey}>
          {(k: Any) => (
            <Chip
              label={k().label}
              count={k().count}
              on={peopleKind() === k().id}
              onPress={() => {
                peopleKind.set(k().id);
                personPick.set("");
              }}
            />
          )}
        </Each>
      </View>
      <If when={adding()}>
        <Panel pad class="flex-row gap-2 items-end">
          <Field label="Name" grow>
            <TextBox value={name()} onChange={(v) => name.set(v)} placeholder="Name, never a TIN" />
          </Field>
          <Btn label="Add" tone="go" onPress={add} />
        </Panel>
      </If>
      <Loading res={list} />
      <If when={Boolean(l())}>
        <View class="flex-row gap-3 items-start">
          <Panel class="w-[260px]">
            <If when={(l()?.rows || []).length === 0}>
              <Text class={`text-[12.5px] p-3 ${C.dim}`}>Nobody here yet.</Text>
            </If>
            <Each of={l()?.rows || []} key={rowKey}>
              {(r: Any) => (
                <View
                  class={`flex-row items-center gap-2.5 px-3 py-2 border-b ${C.line} ${current()?.ticket === r().ticket ? C.raise : ""}`}
                  onPress={() => personPick.set(r().ticket)}
                >
                  <Avatar initials={r().initials} color={r().color} />
                  <View class="flex-1 gap-0.5">
                    <Text class={`text-[13px] font-semibold ${C.ink}`} numberOfLines={1}>
                      {r().name}
                    </Text>
                    <Text class={`text-[11.5px] ${C.dim}`} numberOfLines={1}>
                      {r().sub}
                    </Text>
                  </View>
                </View>
              )}
            </Each>
          </Panel>
          <If when={Boolean(d())}>
            <Panel pad class="flex-1 gap-3">
              <View class="flex-row items-center gap-2.5">
                <Avatar initials={d()?.initials || ""} color={d()?.color || "#888888"} big />
                <View class="flex-1">
                  <Text class={`text-[16px] font-bold ${C.ink}`}>{d()?.name || ""}</Text>
                  <Text class={`text-[12px] ${C.dim}`}>{d()?.sub || ""}</Text>
                </View>
              </View>
              <View class="flex-row gap-2">
                <Each of={d()?.kpi || []}>
                  {(k: Any) => (
                    <View class={`flex-1 rounded-[10px] p-2.5 ${C.raise}`}>
                      <Text class={`text-[11px] ${C.dim}`}>{k().label}</Text>
                      <Text class={`text-[15px] font-bold font-mono ${C.ink}`}>{k().shown}</Text>
                    </View>
                  )}
                </Each>
              </View>
              <View class="gap-1.5">
                <Text class={`text-[12px] ${C.dim}`}>Last 10 months</Text>
                <View class="flex-row items-end gap-1 h-[56px]">
                  <Each of={d()?.spark || []}>
                    {(s: Any) => (
                      <View
                        class={`flex-1 rounded-t-[2px] ${d()?.spark_tone === "neg" ? C.redBg : C.greenBg}`}
                        style={{ height: `${String(s().pct)}%`, opacity: 0.85 }}
                      />
                    )}
                  </Each>
                </View>
              </View>
              <If when={(d()?.recent || []).length === 0}>
                <Text class={`text-[12px] ${C.dim}`}>No entries yet.</Text>
              </If>
              <View>
                <Each of={d()?.recent || []} key={rowKey}>
                  {(r: Any) => (
                    <View class={`flex-row gap-3 py-2 border-t ${C.line}`} onPress={() => drawerEntry.set(r().ticket)}>
                      <Text class={`w-[52px] text-[12.5px] font-mono ${C.ink}`}>{r().date_short}</Text>
                      <Text class={`flex-1 text-[12.5px] ${C.ink}`} numberOfLines={1}>
                        {r().title}
                      </Text>
                      <Text class={`text-[12.5px] font-mono ${AMT[r().tone] || C.ink}`}>{r().amount_shown}</Text>
                    </View>
                  )}
                </Each>
              </View>
              <If when={!d()?.bank && Boolean(current()?.ticket)}>
                <Statement person={current()?.ticket || ""} />
              </If>
              <If when={!d()?.bank}>
                <View class={`flex-row gap-2 items-center rounded-lg px-2.5 py-2 ${PILL.bir}`}>
                  <Text class={`text-[12px] ${PILL.bir}`}>{d()?.tin_note || ""}</Text>
                </View>
              </If>
            </Panel>
          </If>
        </View>
      </If>
    </View>
  );
}
