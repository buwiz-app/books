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

function PeoplePage(p: { go: (page: string, filter?: string) => void }) {
  const list = resource(
    () => ({ v: ver(), k: peopleKind() }),
    (k: Any) => rox.call("books_people_list", { kind: k.k }),
  );
  const l = () => list() as Any;
  const current = computed(() => {
    const rows = l()?.rows || [];
    return rows.find((r: Any) => r.ticket === personPick()) || rows[0] || null;
  });
  const detail = resource(
    () => ({ v: ver(), t: current()?.ticket || "" }),
    (k: Any) => (k.t ? rox.call("books_person_get", { person: k.t }) : Promise.resolve(null)),
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
            <TextBox value={name()} onChange={name.set} placeholder="Name, never a TIN" />
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
