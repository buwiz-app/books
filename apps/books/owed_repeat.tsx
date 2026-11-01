// Monthly repeats on the Unpaid page (M10 parity, unpaid-3). A document
// marked "Repeat monthly" shows up here each month; the user adds the copy
// with one click. Nothing adds a copy by itself.

function RepeatsDue(p: { rows: Any[] }) {
  const n = () => (p.rows || []).length;
  const add = (owed: string) =>
    act("books_app_owed_repeat", { owed, action: "add" })
      .then(() => toast("This month's copy added"))
      .catch(() => null);
  const stop = (owed: string) =>
    act("books_app_owed_repeat", { owed, action: "stop" })
      .then(() => toast("Stopped repeating"))
      .catch(() => null);
  return (
    <If when={n() > 0}>
      <Panel>
        <Text class={`text-[12.5px] px-3 py-2.5 border-b ${C.line} ${C.mute}`}>
          {n() === 1 ? "1 repeating document is ready for this month" : `${String(n())} repeating documents are ready for this month`}
        </Text>
        <Each of={p.rows || []} key={(r: Any) => r.owed}>
          {(r: Any) => (
            <View class={`flex-row items-center gap-3 px-3 py-2 border-b ${C.line}`}>
              <Text class={`flex-1 text-[12.5px] ${C.ink}`} numberOfLines={1}>
                {r().who || ""}
              </Text>
              <Text class={`w-[150px] text-[12.5px] ${C.ink}`} numberOfLines={1}>
                {r().doc || ""}
              </Text>
              <Text class={`w-[90px] text-[12.5px] font-mono ${C.ink}`}>{r().issued}</Text>
              <Text class={`w-[110px] text-right text-[12.5px] font-mono ${C.ink}`}>{r().amount_shown}</Text>
              <Btn label="Add this month's copy" small onPress={() => add(r().owed)} />
              <Btn label="Stop repeating" small onPress={() => stop(r().owed)} />
            </View>
          )}
        </Each>
      </Panel>
    </If>
  );
}

function OwedRepeatToggle(p: { ticket: string; on: boolean }) {
  const flip = () =>
    act("books_app_owed_repeat", { owed: p.ticket, action: p.on ? "stop" : "start" })
      .then(() => toast(p.on ? "Stopped repeating" : "Repeats monthly"))
      .catch(() => null);
  return <Btn label={p.on ? "Stop repeating" : "Repeat monthly"} small onPress={() => flip()} />;
}
