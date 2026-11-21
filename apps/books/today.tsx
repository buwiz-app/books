// Home (Overview): matches buwiz-books Overview dashboard geometry, cards,
// and color system while exposing every Books page and quick entry.

const ICON_TONE: Record<string, string> = {
  rev: PILL.rev,
  draft: PILL.draft,
  in: PILL.in,
  bir: PILL.bir,
};

function Kpi(p: { label: string; shown: string; tone?: string; rows: Any[] }) {
  const valTone =
    p.tone === "pos"
      ? "text-[#10b981] dark:text-[#34d399]"
      : p.tone === "neg"
        ? "text-[#ef4444] dark:text-[#f87171]"
        : C.ink;
  return (
    <View class={`flex-1 rounded-xl border p-5 gap-1.5 bg-white dark:bg-[#131826] transition-shadow hover:shadow-sm ${C.line}`}>
      <Text class="text-xs font-medium uppercase tracking-wide text-[#94a3b8] dark:text-white/40">{p.label}</Text>
      <Text class={`mt-2 text-2xl font-semibold tabular-nums ${valTone}`}>{p.shown}</Text>
      <View class="gap-1.5 mt-1.5">
        <Each of={p.rows || []}>{(r: Any) => <Bar label={r().label} pct={r().pct} shown={r().shown} tone={r().tone} />}</Each>
      </View>
    </View>
  );
}

function NeedRow(p: { n: Any; go: (page: string, filter?: string) => void }) {
  return (
    <View
      class={`flex-row items-center gap-3 px-5 py-3 border-t transition-colors hover:bg-slate-50 dark:hover:bg-slate-800/50 ${C.line}`}
      onPress={() => {
        if (p.n.filter === "review" || p.n.filter === "draft") {
          inboxFilter.set(p.n.filter);
          p.go("inbox");
        } else p.go(p.n.go, p.n.filter);
      }}
      activeStyle={{ opacity: 0.7 }}
    >
      <View class={`w-7 h-7 rounded-lg items-center justify-center ${ICON_TONE[p.n.tone] || PILL.mute}`}>
        <Text class={`text-[12px] font-bold ${ICON_TONE[p.n.tone] || PILL.mute}`}>{String(p.n.count)}</Text>
      </View>
      <View class="flex-1 gap-0.5">
        <Text class={`text-[13.5px] font-medium ${C.ink}`}>{p.n.title}</Text>
        <Text class={`text-[12px] ${C.dim} truncate`}>
          {p.n.detail}
        </Text>
      </View>
      <Text class={C.dim}>›</Text>
    </View>
  );
}

function Quick(p: { title: string; sub: string; onPress: () => void }) {
  return (
    <View
      class={`flex-1 rounded-xl border px-3.5 py-3 gap-0.5 bg-[#f8f9fb] transition-colors hover:bg-slate-100 dark:bg-slate-900 dark:hover:bg-slate-800 ${C.line}`}
      onPress={p.onPress}
      activeStyle={{ opacity: 0.75 }}
    >
      <Text class={`text-[13.5px] font-semibold ${C.ink}`}>{p.title}</Text>
      <Text class={`text-[11.5px] ${C.dim}`}>{p.sub}</Text>
    </View>
  );
}

/** Home's way to a page: a tile under its group. */
function NavTile(p: { title: string; sub: string; onPress: () => void }) {
  return (
    <View
      class={`w-[186px] rounded-xl border px-3.5 py-3 gap-0.5 bg-white transition-colors hover:bg-slate-50 dark:bg-[#131826] dark:hover:bg-slate-700/60 ${C.line}`}
      onPress={p.onPress}
      activeStyle={{ opacity: 0.75 }}
    >
      <Text class={`text-[13.5px] font-semibold ${C.ink}`}>{p.title}</Text>
      <Text class={`text-[11.5px] ${C.dim} truncate`}>
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
    <View class="gap-5">
      <View class="flex-row items-center justify-between gap-4">
        <View class="gap-0.5">
          <Text class={`text-2xl font-semibold ${C.ink}`}>Overview</Text>
          <Text class={`text-sm ${C.mute}`}>
            {t()?.today_label ? `Your business at a glance · ${t().today_label}` : "Your business at a glance"}
          </Text>
        </View>
        <View class="flex-row items-center gap-2">
          <Btn label="+ Money out" onPress={() => p.sheet("money_out")} />
          <Btn label="+ Money in" onPress={() => p.sheet("money_in")} />
          <View
            class="h-10 px-4 rounded-md bg-teal-700 transition-colors hover:bg-teal-800 items-center justify-center"
            onPress={() => p.sheet("journal")}
            activeStyle={{ opacity: 0.85 }}
          >
            <Text class="text-[13px] font-semibold text-white">+ New Entry</Text>
          </View>
        </View>
      </View>
      <Loading res={today} />
      <If when={Boolean(t())}>
        <View class="gap-5">
          <View class="flex-row gap-4">
            <Kpi label={t()?.cash.label || "Cash on hand"} shown={`₱ ${t()?.cash.shown || "0.00"}`} rows={t()?.cash.rows} />
            <Kpi label={t()?.month.label || "Net income (MTD)"} shown={t()?.month.shown || "₱ 0.00"} tone={t()?.month.tone} rows={t()?.month.rows} />
            <Kpi label={t()?.unpaid.label || "Unpaid"} shown={`₱ ${t()?.unpaid.shown || "0.00"}`} rows={t()?.unpaid.rows} />
          </View>
          <View class="flex-row gap-5 items-start">
            <View class={`flex-[1.25] rounded-xl border overflow-hidden bg-white dark:bg-[#131826] ${C.line}`}>
              <View class="flex-row items-center justify-between px-5 py-3.5">
                <Text class={`text-[14px] font-semibold ${C.ink}`}>Needs you</Text>
                <Text class={`text-[12px] ${C.dim}`}>
                  {t()?.needs_count === 1 ? "1 item" : `${String(t()?.needs_count || 0)} items`}
                </Text>
              </View>
              <If when={(t()?.needs || []).length === 0}>
                <Text class={`text-[13px] px-5 pb-4 pt-1 ${C.dim}`}>
                  Nothing waits for you. New drafts and bank lines show up here.
                </Text>
              </If>
              <Each of={t()?.needs || []} key={rowKey}>
                {(n: Any) => <NeedRow n={n()} go={p.go} />}
              </Each>
            </View>
            <View class={`flex-1 rounded-xl border overflow-hidden bg-white dark:bg-[#131826] ${C.line}`}>
              <View class="flex-row items-center justify-between px-5 py-3.5">
                <Text class={`text-[14px] font-semibold ${C.ink}`}>Quick entry</Text>
                <Text class={`text-[12px] ${C.dim}`}>also in chat as /books</Text>
              </View>
              <View class="px-5 pb-5 gap-2.5">
                <View class="flex-row gap-2.5">
                  <Quick title="Money in" sub="A customer paid you" onPress={() => p.sheet("money_in")} />
                  <Quick title="Money out" sub="You paid someone" onPress={() => p.sheet("money_out")} />
                </View>
                <View class="flex-row gap-2.5">
                  <Quick title="Transfer" sub="Between your accounts" onPress={() => p.sheet("transfer")} />
                  <Quick title="Journal" sub="Any balanced set of lines" onPress={() => p.sheet("journal")} />
                </View>
              </View>
            </View>
          </View>
        </View>
      </If>
      <Each of={NAV_GROUPS}>
        {(g: Any) => (
          <View class="gap-2">
            <Text class={`text-[11px] font-semibold uppercase tracking-wider ${C.dim}`}>{g()}</Text>
            <View class="flex-row gap-2.5 flex-wrap">
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
