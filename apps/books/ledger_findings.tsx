// Ledger review findings (M10 parity, ledger-2): the words for each check id that books_ledger_list puts on a row
// (`checks`) and the 'Checks' chip label. Words only: a finding never carries an amount, a payee or a memo.
// The ledger page and entry drawer read these; the Checks filter itself is `filter: "checks"` on the ledger list.

const FINDING_WORDS: Record<string, string> = {
  uncategorized: "Not categorized",
  stale_draft: "Old draft",
  unmatched_month: "Month not matched",
  weekend_date: "Weekend date",
  round_amount: "Round amount",
  missing_person: "No person",
};

const CHECKS_CHIP: [string, string] = ["checks", "Checks"];

function findingWord(id: string): string {
  return FINDING_WORDS[id] || id;
}

function findingWords(row: Any): string {
  return ((row?.checks || []) as string[]).map(findingWord).join(" · ");
}

function FindingPills(p: { row: Any }) {
  return (
    <View class="flex-row gap-1">
      <Each of={(p.row?.checks || []) as string[]} key={rowKey}>
        {(id: Any) => <Pill tone="rev" label={findingWord(String(id()))} />}
      </Each>
    </View>
  );
}
