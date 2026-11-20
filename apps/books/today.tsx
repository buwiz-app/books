// Home: cash now, the month so far, unpaid totals, what needs the user,
// and the way to every page (the window has no rail).

const ICON_TONE: Record<string, string> = {
  rev: PILL.rev,
  draft: PILL.draft,
  in: PILL.in,
  bir: PILL.bir,
};

function Kpi(p: { label: string; shown: string; tone?: string; rows: Any[] }) {
  return (
    <Panel class="flex-1 px-3.5 py-3 gap-1">
      <Text class={`text-[11.5px] uppercase tracking-wider ${C.mute}`}>{p.label}</Text>
      <Text class={`text-[22px] font-semibold font-mono ${p.tone === "pos" ? C.green : C.ink}`}>{p.shown}</Text>
      <View class="gap-1.5 mt-1">
        <Each of={p.rows || []}>{(r: Any) => <Bar label={r().label} pct={r().pct} shown={r().shown} tone={r().tone} />}</Each>
      </View>
    </Panel>
  );
}

function NeedRow(p: { n: Any; go: (page: string, filter?: string) => void }) {
  return (
    <View class={`flex-row items-center gap-2.5 px-3.5 py-2.5 border-t ${C.line}`} onPress={() => {
        if (p.n.filter === "review" || p.n.filter === "draft") {
          inboxFilter.set(p.n.filter);
          p.go("inbox");
        } else p.go(p.n.go, p.n.filter);
      }} activeStyle={{ opacity: 0.7 }}>
      <View class={`w-7 h-7 rounded-lg items-center justify-center ${ICON_TONE[p.n.tone] || PILL.mute}`}>
        <Text class={`text-[12px] font-bold ${ICON_TONE[p.n.tone] || PILL.mute}`}>{String(p.n.count)}</Text>
      </View>
      <View class="flex-1 gap-0.5">
        <Text class={`text-[13px] font-semibold ${C.ink}`}>{p.n.title}</Text>
        <Text class={`text-[12px] ${C.dim}`} numberOfLines={1}>
          {p.n.detail}
        </Text>
      </View>
      <Text class={C.dim}>›</Text>
    </View>
  );
}

function Quick(p: { title: string; sub: string; onPress: () => void }) {
  return (
    <View class={`flex-1 rounded-[10px] border p-2.5 gap-0.5 ${C.raise} ${C.line}`} onPress={p.onPress} activeStyle={{ opacity: 0.7 }}>
      <Text class={`text-[13px] font-bold ${C.ink}`}>{p.title}</Text>
      <Text class={`text-[11px] ${C.dim}`}>{p.sub}</Text>
    </View>
  );
}

/** Home's way to a page: a tile under its group. */
function NavTile(p: { title: string; sub: string; onPress: () => void }) {
  return (
    <View class={`w-[176px] rounded-[10px] border p-2.5 gap-0.5 ${C.panel} ${C.line}`} onPress={p.onPress} activeStyle={{ opacity: 0.7 }}>
      <Text class={`text-[13px] font-bold ${C.ink}`}>{p.title}</Text>
      <Text class={`text-[11px] ${C.dim}`} numberOfLines={1}>
        {p.sub}
      </Text>
    </View>
  );
}

const NAV_GROUPS = ["Money", "Results", "Records"];

function TodayPage(p: { go: (page: string, filter?: string) => void; sheet: (mode: string) => void; pages: Any[] }) {
  const today = resource(
    () => ({ v: ver() }),
    () => buwiz.call("books_today", {}),
  );
  const t = () => today() as Any;
  return (
    <View class="gap-3.5">
      <Head home title="Home" sub={t() ? t().today_label : ""}>
        <View class="flex-row gap-2">
          <Btn label="+ Money out" onPress={() => p.sheet("money_out")} />
          <Btn label="+ Money in" onPress={() => p.sheet("money_in")} />
          <Btn label="+ New entry" tone="go" onPress={() => p.sheet("journal")} />
        </View>
      </Head>
      <Loading res={today} />
      <If when={Boolean(t())}>
        <View class="gap-3.5">
          <View class="flex-row gap-2.5">
            <Kpi label={t()?.cash.label || ""} shown={`₱ ${t()?.cash.shown || ""}`} rows={t()?.cash.rows} />
            <Kpi label={t()?.month.label || ""} shown={t()?.month.shown || ""} tone={t()?.month.tone} rows={t()?.month.rows} />
            <Kpi label={t()?.unpaid.label || ""} shown={`₱ ${t()?.unpaid.shown || ""}`} rows={t()?.unpaid.rows} />
          </View>
          <View class="flex-row gap-3 items-start">
            <Panel class="flex-[1.25]">
              <View class="flex-row items-center justify-between px-3.5 pt-3 pb-1">
                <Text class={`text-[13px] font-bold ${C.ink}`}>Needs you</Text>
                <Text class={`text-[12px] ${C.dim}`}>{t()?.needs_count === 1 ? "1 thing" : `${String(t()?.needs_count || 0)} things`}</Text>
              </View>
              <If when={(t()?.needs || []).length === 0}>
                <Text class={`text-[12.5px] px-3.5 pb-3 pt-1 ${C.dim}`}>Nothing waits for you. New drafts and bank lines show up here.</Text>
              </If>
              <Each of={t()?.needs || []} key={rowKey}>
                {(n: Any) => <NeedRow n={n()} go={p.go} />}
              </Each>
            </Panel>
            <Panel class="flex-1">
              <View class="flex-row items-center justify-between px-3.5 pt-3 pb-1">
                <Text class={`text-[13px] font-bold ${C.ink}`}>Add an entry</Text>
                <Text class={`text-[12px] ${C.dim}`}>also in chat as /books</Text>
              </View>
              <View class="p-3.5 gap-2">
                <View class="flex-row gap-2">
                  <Quick title="Money in" sub="A customer paid you" onPress={() => p.sheet("money_in")} />
                  <Quick title="Money out" sub="You paid someone" onPress={() => p.sheet("money_out")} />
                </View>
                <View class="flex-row gap-2">
                  <Quick title="Transfer" sub="Between your accounts" onPress={() => p.sheet("transfer")} />
                  <Quick title="Journal" sub="Any balanced set of lines" onPress={() => p.sheet("journal")} />
                </View>
              </View>
            </Panel>
          </View>
        </View>
      </If>
      <Each of={NAV_GROUPS}>
        {(g: Any) => (
          <View class="gap-1.5">
            <Text class={`text-[10.5px] uppercase tracking-widest ${C.dim}`}>{g()}</Text>
            <View class="flex-row gap-2 flex-wrap">
              <Each of={(p.pages || []).filter((x: Any) => x.group === g())} key={(x: Any) => x.id}>
                {(x: Any) => <NavTile title={x().label} sub={x().sub || ""} onPress={() => p.go(x().id)} />}
              </Each>
              <If when={g() === "Records"}>
                <NavTile title="Documents" sub="Receipts, bills, statements" onPress={() => buwiz.open({ app: "evidence" }).catch(() => null)} />
              </If>
            </View>
          </View>
        )}
      </Each>
    </View>
  );
}
