// Statement read (M10 parity reconcile-1): the lines the assistant read from
// a statement the user attached, shown beside the month before they count.
// The user leaves lines out, gives the closing if the page had none, and
// uses them or puts them aside. Books never reads the page itself.

export function DraftBanner(p: { account: string; month: string; draft: Any }) {
  const excluded = signal<number[]>([]);
  const closing = signal("");
  const dr = () => p.draft as Any;
  const rows = () => (dr()?.rows || []) as Any[];
  const need = () => Number(dr()?.low_confidence || 0) + Number(dr()?.outside_month || 0);
  const isOut = (no: number) => excluded().includes(no);
  const toggle = (no: number) => excluded.set(isOut(no) ? excluded().filter((n) => n !== no) : [...excluded(), no]);
  const closingText = () => moneyText(closing());
  const blocked = () => rows().some((r: Any) => r.outside_month && !isOut(r.no)) || (!dr()?.has_closing && !closingText());
  const use = () =>
    act("books_app_statement_accept", {
      account: p.account,
      month: p.month,
      draft: dr()?.ticket,
      exclude: excluded(),
      ...(closingText() ? { closing: closingText() } : {}),
    }).catch(() => null);
  const later = () => act("books_app_statement_accept", { account: p.account, month: p.month, draft: dr()?.ticket, dismiss: true }).catch(() => null);
  return (
    <Panel pad class="gap-2.5">
      <Text class={`text-[13px] font-semibold ${C.ink}`}>{`The assistant read ${String(dr()?.lines || 0)} lines from a statement · ${String(need())} need a look`}</Text>
      <Each of={rows()} key={(r: Any) => String(r.no)}>
        {(r: Any) => (
          <View class={`flex-row items-center gap-2 border-b ${C.line} py-1.5`}>
            <Btn label={isOut(r().no) ? "Left out" : "Use"} small onPress={() => toggle(r().no)} />
            <Text class={`w-[52px] text-[12px] font-mono ${C.ink}`}>{r().date_short}</Text>
            <Text class={`flex-1 text-[12.5px] ${isOut(r().no) ? C.dim : C.ink}`} numberOfLines={1}>
              {r().description}
            </Text>
            <If when={Boolean(r().low)}>
              <Pill tone="draft" label={`read ${String(r().confidence)}%`} />
            </If>
            <If when={Boolean(r().outside_month)}>
              <Pill tone="rev" label="Other month" />
            </If>
            <Text class={`text-[12.5px] font-mono ${C.ink}`}>{r().amount_shown}</Text>
          </View>
        )}
      </Each>
      <If when={!dr()?.has_closing}>
        <View class="flex-row items-center gap-2">
          <Text class={`text-[12.5px] ${C.mute}`}>Ending balance on the statement</Text>
          <TextBox value={closing()} onChange={(v) => closing.set(v)} placeholder="0.00" />
        </View>
      </If>
      <View class="flex-row gap-2">
        <Btn label="Use these lines" tone="go" disabled={blocked()} onPress={() => use()} />
        <Btn label="Not now" onPress={() => later()} />
      </View>
    </Panel>
  );
}
