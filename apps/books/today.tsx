// Today: cash now, the month so far, unpaid totals, then what needs the
// user and the quick actions (drawing: Books Mini App › Today).

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
    <View class={`flex-row items-center gap-2.5 px-3.5 py-2.5 border-t ${C.line}`} onPress={() => p.go(p.n.go, p.n.filter)} activeStyle={{ opacity: 0.7 }}>
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

function TodayPage(p: { go: (page: string, filter?: string) => void; sheet: (mode: string) => void }) {
  const today = resource(
    () => ({ v: ver() }),
    () => rox.call("books_today", {}),
  );
  const t = () => today() as Any;
  return (
    <View class="gap-3.5">
      <Head title="Today" sub={t() ? t().today_label : ""}>
        <View class="flex-row gap-2">
          <Btn label="+ Money out" onPress={() => p.sheet("money_out")} />
          <Btn label="+ Money in" tone="go" onPress={() => p.sheet("money_in")} />
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
                <Text class={`text-[13px] font-bold ${C.ink}`}>Quick</Text>
                <Text class={`text-[12px] ${C.dim}`}>also in chat as /books</Text>
              </View>
              <View class="p-3.5 gap-2">
                <View class="flex-row gap-2">
                  <Quick title="Money in" sub="A customer paid you" onPress={() => p.sheet("money_in")} />
                  <Quick title="Money out" sub="You paid someone" onPress={() => p.sheet("money_out")} />
                </View>
                <View class="flex-row gap-2">
                  <Quick title="Transfer" sub="Between your accounts" onPress={() => p.sheet("transfer")} />
                  <Quick title="Unpaid" sub="Invoices and bills waiting" onPress={() => p.go("unpaid")} />
                </View>
                <View class="flex-row gap-2">
                  <Quick title="Ledger" sub="Every entry this month" onPress={() => p.go("ledger")} />
                  <Quick title={`${t()?.month_name || "This month"}'s P&L`} sub={`Net ${t()?.month.shown || ""}`} onPress={() => p.go("reports")} />
                </View>
              </View>
            </Panel>
          </View>
        </View>
      </If>
    </View>
  );
}
