// Tags: departments and locations, one page with two segments. Tags ride
// on each entry; reports can filter by them.

const tagKind = signal("department");

function TagsPage(p: { go: (page: string, filter?: string) => void }) {
  const data = resource(
    () => ({ v: ver(), k: tagKind() }),
    (k: Any) => rox.call("books_tags_list", { kind: k.k }),
  );
  const d = () => data() as Any;
  const adding = signal(false);
  const name = signal("");
  const add = () => {
    if (!name().trim()) return;
    act("books_app_tag_create", { kind: tagKind(), name: name().trim() })
      .then(() => {
        name.set("");
        adding.set(false);
      })
      .catch(() => null);
  };
  return (
    <View class="gap-3.5">
      <Head title="Tags" sub="departments and locations">
        <Btn label={tagKind() === "department" ? "+ New department" : "+ New location"} tone="go" onPress={() => adding.set(!adding())} />
      </Head>
      <Seg
        options={[
          ["department", "Departments"],
          ["location", "Locations"],
        ]}
        value={tagKind()}
        onPick={(v: string) => tagKind.set(v)}
      />
      <If when={adding()}>
        <Panel pad class="flex-row gap-2 items-end">
          <Field label="Name" grow>
            <TextBox value={name()} onChange={name.set} placeholder={tagKind() === "department" ? "Operations" : "Makati office"} />
          </Field>
          <Btn label="Add" tone="go" onPress={add} />
        </Panel>
      </If>
      <Loading res={data} />
      <If when={Boolean(d())}>
        <Panel>
          <View class={`flex-row gap-3 px-3 py-2 border-b ${C.line}`}>
            <Text class={`flex-1 text-[10.5px] uppercase tracking-wider ${C.dim}`}>Name</Text>
            <Text class={`w-[140px] text-right text-[10.5px] uppercase tracking-wider ${C.dim}`}>{`Spent in ${d()?.month.name || ""}`}</Text>
            <Text class={`w-[100px] text-right text-[10.5px] uppercase tracking-wider ${C.dim}`}>Transactions</Text>
            <View class="w-[14px]" />
          </View>
          <If when={(d()?.rows || []).length === 0}>
            <Text class={`text-[12.5px] px-3 py-4 ${C.dim}`}>{`No ${tagKind()}s yet.`}</Text>
          </If>
          <Each of={d()?.rows || []} key={rowKey}>
            {(r: Any) => (
              <View class={`flex-row items-center gap-3 px-3 py-2.5 border-b ${C.line}`} onPress={() => p.go("ledger")}>
                <Text class={`flex-1 text-[13px] font-semibold ${C.ink}`}>{r().name}</Text>
                <Text class={`w-[140px] text-right text-[12.5px] font-mono ${C.ink}`}>{r().spent_shown}</Text>
                <Text class={`w-[100px] text-right text-[12.5px] font-mono ${C.ink}`}>{String(r().count)}</Text>
                <Text class={`w-[14px] text-right ${C.dim}`}>›</Text>
              </View>
            )}
          </Each>
        </Panel>
        <Text class={`text-[12px] ${C.dim}`}>Tags ride on each transaction. Reports can filter by them.</Text>
      </If>
    </View>
  );
}
