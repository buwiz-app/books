// Review notes on an entry (M10 parity, ledger-1): who proposed it, the
// fields the assistant or a routine set with the confidence the assistant
// reported (Books computes none), and likely duplicates the user decides on.

const FIELD_NAMES: Record<string, string> = {
  date: "Date",
  amount: "Amount",
  account: "Account",
  to_account: "To account",
  category: "Category",
  payee: "Payee",
  memo: "Memo",
  reference: "Reference",
  lines: "Lines",
};

/** A whole-percent confidence as a word: High 90+, Check 70-89, Low below 70. */
function confidenceWord(c: Any): string {
  if (c === null || c === undefined) return "";
  const n = c as number;
  return n >= 90 ? "High" : n >= 70 ? "Check" : "Low";
}

function ReviewBadges(p: { row: Any }) {
  const r = p.row || {};
  return (
    <View class="flex-row gap-1">
      <If when={Boolean(r.proposed_by)}>
        <Pill tone="in" label="Assistant" />
      </If>
      <If when={Boolean(r.low_confidence)}>
        <Pill tone="rev" label="Check fields" />
      </If>
      <If when={Boolean(r.likely_duplicate)}>
        <Pill tone="draft" label="Possible duplicate" />
      </If>
    </View>
  );
}

function ReviewPanel(p: { entry: Any; open: (ticket: string) => void; keepBoth: () => void; busy?: boolean }) {
  const rv = () => (p.entry && p.entry.review) || {};
  const changed = () => (rv().fields || []).filter((f: Any) => f.changed_by && f.changed_by !== "app");
  const dups = () => rv().likely_duplicates || [];
  const flagged = () => dups().length > 0 && p.entry && p.entry.status === "draft";
  return (
    <If when={changed().length > 0 || dups().length > 0}>
      <View class="gap-1.5">
        <If when={changed().length > 0}>
          <Text class={`text-[12px] ${C.dim}`}>{rv().proposed_by === "routine" ? "Set by a routine" : "Set by the assistant"}</Text>
          <Each of={changed()} key={rowKey}>
            {(f: Any) => <Kv k={FIELD_NAMES[f().field] || f().field} v={confidenceWord(f().confidence) || "—"} />}
          </Each>
        </If>
        <If when={dups().length > 0}>
          <Text class={`text-[12px] ${C.dim}`}>Possible duplicates</Text>
          <Each of={dups()} key={rowKey}>
            {(d: Any) => (
              <View class={`flex-row gap-2 rounded-[8px] border px-2.5 py-1.5 ${C.line}`} onPress={() => p.open(d().ticket)}>
                <Text class={`flex-1 text-[12px] ${C.ink}`}>{`Entry ${d().no} · ${d().date}`}</Text>
                <Text class={`text-[12px] ${C.dim}`}>{d().amount_shown || d().status}</Text>
              </View>
            )}
          </Each>
          <If when={flagged()}>
            <Btn label="Keep both and post" tone="go" disabled={Boolean(p.busy)} onPress={() => p.keepBoth()} />
          </If>
        </If>
      </View>
    </If>
  );
}
