// Old books: bring in a web Buwiz Books export or CSV files, match each
// account once (Map 1: Books suggests, the owner confirms), and opening
// totals with proof for a book that starts mid-year (drawing: Books Tax
// Bridge › Map 1, Four starting points).

const csvChart = signal(null as Any);
const csvJournal = signal(null as Any);

function pickOne(extensions: string[], then: (f: Any) => void) {
  if (!buwiz.pickFile) return toast("Picking files is not available in this Buwiz Bot.");
  buwiz
    .pickFile({ extensions })
    .then((files: Any[]) => (files.length ? then(files[0]) : null))
    .catch((e: Any) => toast(errText(e)));
}

async function importWeb(file: Any) {
  toast("Reading the export…");
  const upload = await sendFile(file);
  const r: Any = await act("books_app_import", { source: "web", web_upload: upload, file_name: String(file.name).slice(0, 120) });
  toast(`${String(r.posted)} entries in · match the accounts next`);
}

async function importCsv() {
  toast("Reading the CSV files…");
  const chart = await sendFile(csvChart());
  const journal = await sendFile(csvJournal());
  const r: Any = await act("books_app_import", { source: "csv", chart_upload: chart, journal_upload: journal, file_name: String(csvJournal().name).slice(0, 120) });
  csvChart.set(null);
  csvJournal.set(null);
  toast(`${String(r.posted)} entries in · match the accounts next`);
}

const MATCH_TONE: Record<string, [string, string]> = {
  confirmed: ["ok", "Confirmed"],
  suggested: ["draft", "Suggested"],
  mixed: ["in", "Per line"],
  unsure: ["rev", "Not matched"],
};

function ImportPage(p: { go: (page: string, filter?: string) => void }) {
  const data = resource(
    () => ({ v: ver() }),
    () => buwiz.call("books_import_get", {}),
  );
  const d = () => data() as Any;
  const measure = signal("gross_sales");
  const from = signal("");
  const to = signal("");
  const amount = signal("");
  const proof = signal("");
  const err = signal("");
  const addOpening = () => {
    err.set("");
    const a = moneyText(amount());
    if (!a) return err.set("Type the total, like 250,000.00.");
    act("books_app_opening_set", { measure: measure(), from: from().trim(), to: to().trim(), amount: a, proof: proof().trim() })
      .then(() => {
        amount.set("");
        proof.set("");
        toast("Opening total saved");
      })
      .catch((e: Any) => err.set(errText(e)));
  };
  const last = () => (d()?.batches || [])[0];
  return (
    <View class="gap-3.5">
      <Head title="Old books" sub="bring them in once, then match each account">
        <Btn label="Back to Settings" onPress={() => p.go("settings")} />
      </Head>
      <Loading res={data} />
      <If when={Boolean(d()?.can_import)}>
        <View class="flex-row gap-3">
          <Panel pad class="flex-1 gap-2">
            <Text class={`text-[13.5px] font-bold ${C.ink}`}>Web Buwiz Books</Text>
            <Text class={`text-[12.5px] ${C.mute}`}>The export file (.json) from Settings › Export in web Books. Books reads accounts, people, tags and posted transactions. Tax profiles, TINs and certificates stay out.</Text>
            <View class="flex-row">
              <Btn label="Pick the export…" tone="go" onPress={() => pickOne([".json"], (f: Any) => importWeb(f).catch(() => null))} />
            </View>
          </Panel>
          <Panel pad class="flex-1 gap-2">
            <Text class={`text-[13.5px] font-bold ${C.ink}`}>CSV or Excel files</Text>
            <Text class={`text-[12.5px] ${C.mute}`}>A chart of accounts and a journal, from QuickBooks, Xero or a spreadsheet (.csv, .xlsx or .xls; a workbook reads its sheet named Chart or Journal, else its first).</Text>
            <View class="flex-row gap-2 flex-wrap items-center">
              <Btn label={csvChart() ? `Chart: ${String(csvChart().name)}` : "Pick the chart…"} onPress={() => pickOne([".csv", ".xlsx", ".xls"], (f: Any) => csvChart.set(f))} />
              <Btn label={csvJournal() ? `Journal: ${String(csvJournal().name)}` : "Pick the journal…"} onPress={() => pickOne([".csv", ".xlsx", ".xls"], (f: Any) => csvJournal.set(f))} />
              <Btn label="Bring them in" tone="go" disabled={!csvChart() || !csvJournal()} onPress={() => importCsv().catch(() => null)} />
            </View>
          </Panel>
        </View>
      </If>
      <If when={(d()?.matches || []).length > 0}>
        <Panel>
          <View class={`px-3.5 pt-3 pb-2 gap-0.5 border-b ${C.line}`}>
            <Text class={`text-[14px] font-bold ${C.ink}`}>Match your accounts</Text>
            <Text class={`text-[12px] ${C.mute}`}>
              {`Imported from ${last()?.file_name || last()?.source || "old books"} · ${String(d()?.counts.accounts || 0)} accounts · ${String(d()?.counts.lines || 0)} lines`}
            </Text>
          </View>
          <View class={`flex-row gap-3 px-3.5 py-2 border-b ${C.line}`}>
            <Text class={`flex-1 text-[10.5px] uppercase tracking-wider ${C.dim}`}>Their account</Text>
            <Text class={`w-[44px] text-right text-[10.5px] uppercase tracking-wider ${C.dim}`}>Sure</Text>
            <Text class={`w-[230px] text-[10.5px] uppercase tracking-wider ${C.dim}`}>Kind in Books</Text>
            <Text class={`w-[170px] text-[10.5px] uppercase tracking-wider ${C.dim}`}>Feeds</Text>
            <Text class={`w-[96px] text-[10.5px] uppercase tracking-wider ${C.dim}`}>Status</Text>
          </View>
          <Each of={d()?.matches || []} key={rowKey}>
            {(m: Any) => (
              <View class={`flex-row items-start gap-3 px-3.5 py-2 border-b ${C.line}`}>
                <View class="flex-1">
                  <Text class={`text-[13px] font-semibold ${C.ink}`} numberOfLines={1}>
                    {m().imported_name || m().name}
                  </Text>
                  <Text class={`text-[11.5px] ${C.dim}`} numberOfLines={1}>
                    {`${m().number} · ${m().type_label}${m().imported_name ? ` · now ${m().name}` : ""}`}
                  </Text>
                </View>
                <Text class={`w-[44px] text-right text-[12.5px] font-mono ${C.mute}`}>{m().confidence === null ? "—" : `${String(m().confidence)}%`}</Text>
                <View class="w-[230px]">
                  <Pick value={m().kind} options={(d()?.kinds || {})[m().side] || []} onPick={(k: Any) => act("books_app_account_match", { account: m().ticket, kind: k.ticket }).catch(() => null)} />
                </View>
                <Text class={`w-[170px] text-[12px] ${C.mute}`} numberOfLines={2}>
                  {m().feeds}
                </Text>
                <View class="w-[96px] gap-1">
                  <Pill tone={(MATCH_TONE[m().status] || MATCH_TONE.suggested)[0]} label={(MATCH_TONE[m().status] || MATCH_TONE.suggested)[1]} />
                  <If when={m().status === "suggested"}>
                    <Btn label="Confirm" small onPress={() => act("books_app_account_match", { account: m().ticket, kind: m().kind }).catch(() => null)} />
                  </If>
                </View>
              </View>
            )}
          </Each>
          <View class="flex-row items-center gap-3 px-3.5 py-2.5">
            <Text class={`text-[12.5px] ${C.ink}`}>
              {`${String(d()?.counts.to_confirm || 0)} to confirm · ${String(d()?.counts.per_line || 0)} per line · ${String(d()?.counts.not_matched || 0)} not matched`}
            </Text>
            <Text class={`flex-1 text-[12px] ${d()?.held ? C.amber : C.green}`}>{d()?.held ? "Boxes that could read unmatched lines are held." : "Ready: every account has a kind."}</Text>
            <Btn label="Confirm all suggested" tone="go" disabled={!d()?.counts.to_confirm} onPress={() => act("books_app_accounts_confirm", { min: 50 }).then((r: Any) => toast(`${String(r.confirmed)} confirmed`)).catch(() => null)} />
          </View>
        </Panel>
      </If>
      <If when={Boolean(d())}>
        <Panel pad class="gap-3">
          <View class="gap-0.5">
            <Text class={`text-[14px] font-bold ${C.ink}`}>Opening totals</Text>
            <Text class={`text-[12px] ${C.mute}`}>A book that starts mid-year: January to the month before, one total each, with what proves it (filed returns, bank statements). Forms counts them; their lineage shows them as opening figures.</Text>
          </View>
          <Each of={d()?.openings || []} key={rowKey}>
            {(o: Any) => (
              <View class={`flex-row items-center gap-3 py-1.5 border-b ${C.line}`}>
                <Text class={`w-[200px] text-[13px] ${C.ink}`}>{o().label}</Text>
                <Text class={`w-[130px] text-[12.5px] ${C.mute}`}>{o().range_label}</Text>
                <Text class={`w-[120px] text-right text-[12.5px] font-mono ${C.ink}`}>{o().amount_shown}</Text>
                <Text class={`flex-1 text-[12px] ${C.dim}`} numberOfLines={1}>
                  {o().proof || ""}
                </Text>
                <Btn label="Remove" small onPress={() => act("books_app_opening_set", { measure: o().measure, from: o().from, delete: true }).catch(() => null)} />
              </View>
            )}
          </Each>
          <View class="flex-row gap-2 items-end flex-wrap">
            <View class="w-[200px]">
              <Field label="Total">
                <Pick value={measure()} options={d()?.measures || []} onPick={(m: Any) => measure.set(m.ticket)} />
              </Field>
            </View>
            <View class="w-[110px]">
              <Field label="From">
                <TextBox value={from()} onChange={(v) => from.set(v)} placeholder="2026-01-01" />
              </Field>
            </View>
            <View class="w-[110px]">
              <Field label="To">
                <TextBox value={to()} onChange={(v) => to.set(v)} placeholder="2026-06-30" />
              </Field>
            </View>
            <View class="w-[130px]">
              <Field label="Amount">
                <TextBox value={amount()} onChange={(v) => amount.set(v)} placeholder="0.00" />
              </Field>
            </View>
            <Field label="Proof" grow>
              <TextBox value={proof()} onChange={(v) => proof.set(v)} placeholder="Filed 2551Q for Q1 and Q2" />
            </Field>
            <Btn label="Save" tone="go" onPress={addOpening} />
          </View>
          <If when={Boolean(err())}>
            <Text class={`text-[12.5px] ${C.red}`}>{err()}</Text>
          </If>
        </Panel>
      </If>
    </View>
  );
}
