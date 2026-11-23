// Old books: bring in a web Buwiz Books export or CSV files, match each
// account once (Books suggests, the owner confirms), and opening
// totals with proof for a book that starts mid-year.

const csvChart = signal(null as Any);
const csvJournal = signal(null as Any);

// ImportPanel's upload glyph as an SVG (lucide upload, 1.75px); `xmlns` left
// out like kit.tsx's ICONS — the privacy gate rejects URL literals.
const UPLOAD_ICON = `data:image/svg+xml,${encodeURIComponent(
  '<svg viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/></svg>',
)}`;

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

/** One dashed pick zone, like the reference's drop zone (Linux has no drag-drop:
 *  the click is the mechanism, so the text names picking). */
function PickZone(p: { title: string; hint: string; file: Any; onPick: () => void; onClear: () => void }) {
  return (
    <View
      accessibilityLabel={p.title}
      class={`flex-1 min-w-0 border-2 border-dashed rounded-xl px-4 py-5 items-center gap-1 transition-colors ${C.line} hover:border-[#0d9488]/50 hover:bg-[#f8fafc] dark:hover:bg-[#134e4a]/10`}
      activeStyle={{ opacity: 0.7 }}
      onPress={p.onPick}
    >
      <Avatar src={UPLOAD_ICON} class="w-5 h-5" imageClass="tint-[#94a3b8] dark:tint-[#71717a]" fallbackDelay={Infinity} accessibilityLabel="" />
      <If when={Boolean(p.file)}>
        <View class="flex-row items-center gap-1.5 max-w-full rounded-full bg-[#f0fdfa] border border-[#99f6e4] px-2.5 py-1 dark:bg-[#134e4a]/30 dark:border-[#0f766e]">
          <Text class="text-[11.5px] font-medium text-[#0d9488] dark:text-[#2dd4bf] truncate">{String(p.file?.name || "")}</Text>
          <View class="w-4 h-4 items-center justify-center" onPress={p.onClear}>
            <Text class="text-[10px] text-[#0d9488]/60 hover:text-[#0d9488] dark:text-[#2dd4bf]">✕</Text>
          </View>
        </View>
      </If>
      <If when={!p.file}>
        <Text class={`text-[13px] font-medium ${C.ink} truncate`}>{p.title}</Text>
        <Text class={`text-[11.5px] ${C.dim} truncate`}>{p.hint}</Text>
      </If>
    </View>
  );
}

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
        <View class="flex-col lg:flex-row gap-3">
          {/* Web export — single file, reads it as soon as it's picked */}
          <Panel pad class="flex-1 gap-3">
            <View class="flex-row items-center gap-2">
              <Avatar src={UPLOAD_ICON} class="w-4 h-4" imageClass="tint-[#0d9488] dark:tint-[#2dd4bf]" fallbackDelay={Infinity} accessibilityLabel="" />
              <Text class={`text-[13.5px] font-bold ${C.ink}`}>Web Buwiz Books</Text>
            </View>
            <Text class={`text-[12.5px] ${C.mute}`}>The export file (.json) from Settings › Export in web Books. Books reads accounts, people, tags and posted transactions. Tax profiles, TINs and certificates stay out.</Text>
            <View class="flex-row">
              <PickZone
                title="Click to pick the export file"
                hint="Accepts .json"
                file={null}
                onPick={() => pickOne([".json"], (f: Any) => importWeb(f).catch(() => null))}
                onClear={() => null}
              />
            </View>
          </Panel>
          {/* CSV / Excel — a chart and a journal, then one click to bring both in */}
          <Panel pad class="flex-1 gap-3">
            <View class="flex-row items-center gap-2">
              <Avatar src={UPLOAD_ICON} class="w-4 h-4" imageClass="tint-[#0d9488] dark:tint-[#2dd4bf]" fallbackDelay={Infinity} accessibilityLabel="" />
              <Text class={`text-[13.5px] font-bold ${C.ink}`}>CSV or Excel files</Text>
            </View>
            <Text class={`text-[12.5px] ${C.mute}`}>A chart of accounts and a journal, from QuickBooks, Xero or a spreadsheet (.csv, .xlsx or .xls; a workbook reads its sheet named Chart or Journal, else its first).</Text>
            <View class="flex-row gap-2.5">
              <PickZone
                title="Chart of accounts"
                hint="csv · xlsx · xls"
                file={csvChart()}
                onPick={() => pickOne([".csv", ".xlsx", ".xls"], (f: Any) => csvChart.set(f))}
                onClear={() => csvChart.set(null)}
              />
              <PickZone
                title="Journal"
                hint="csv · xlsx · xls"
                file={csvJournal()}
                onPick={() => pickOne([".csv", ".xlsx", ".xls"], (f: Any) => csvJournal.set(f))}
                onClear={() => csvJournal.set(null)}
              />
            </View>
            <View class="flex-row justify-end">
              <View
                class={`h-[38px] px-5 rounded-lg items-center justify-center transition-colors ${csvChart() && csvJournal() ? "bg-[#0d9488] hover:bg-[#0f766e]" : "bg-[#0d9488] opacity-40"}`}
                activeStyle={{ opacity: 0.7 }}
                onPress={() => (csvChart() && csvJournal() ? importCsv().catch(() => null) : null)}
              >
                <Text class="text-[13px] font-medium text-white">Bring them in</Text>
              </View>
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
            <Text class={`flex-1 text-[10px] font-semibold uppercase tracking-wider ${C.dim}`}>Their account</Text>
            <Text class={`w-[44px] text-right text-[10px] font-semibold uppercase tracking-wider ${C.dim}`}>Sure</Text>
            <Text class={`w-[160px] md:w-[230px] text-[10px] font-semibold uppercase tracking-wider ${C.dim}`}>Kind in Books</Text>
            <Text class={`hidden md:block w-[170px] text-[10px] font-semibold uppercase tracking-wider ${C.dim}`}>Feeds</Text>
            <Text class={`w-[96px] text-[10px] font-semibold uppercase tracking-wider ${C.dim}`}>Status</Text>
          </View>
          <Each of={d()?.matches || []} key={rowKey}>
            {(m: Any) => (
              <View class={`flex-row items-start gap-3 px-3.5 py-2 border-b flex-wrap transition-colors ${C.line} hover:bg-[#f8fafc] dark:hover:bg-[#1c1c1f]`}>
                <View class="flex-1">
                  <Text class={`text-[13px] font-semibold ${C.ink} truncate`}>
                    {m().imported_name || m().name}
                  </Text>
                  <Text class={`text-[11.5px] ${C.dim} truncate`}>
                    {`${m().number} · ${m().type_label}${m().imported_name ? ` · now ${m().name}` : ""}`}
                  </Text>
                </View>
                <Text class={`w-[44px] text-right text-[12.5px] font-mono ${C.mute}`}>{m().confidence === null ? "—" : `${String(m().confidence)}%`}</Text>
                <View class="w-[160px] md:w-[230px]">
                  <Pick value={m().kind} options={(d()?.kinds || {})[m().side] || []} onPick={(k: Any) => act("books_app_account_match", { account: m().ticket, kind: k.ticket }).catch(() => null)} />
                </View>
                <Text class={`hidden md:block w-[170px] text-[12px] ${C.mute} line-clamp-2`}>
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
            <View
              class={`h-[38px] px-4 rounded-lg items-center justify-center transition-colors ${d()?.counts.to_confirm ? "bg-[#0d9488] hover:bg-[#0f766e]" : "bg-[#0d9488] opacity-40"}`}
              activeStyle={{ opacity: 0.7 }}
              onPress={() => (d()?.counts.to_confirm ? act("books_app_accounts_confirm", { min: 50 }).then((r: Any) => toast(`${String(r.confirmed)} confirmed`)).catch(() => null) : null)}
            >
              <Text class="text-[13px] font-medium text-white">Confirm all suggested</Text>
            </View>
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
              <View class={`flex-row items-center gap-3 py-1.5 border-b flex-wrap transition-colors ${C.line} hover:bg-[#f8fafc] dark:hover:bg-[#1c1c1f]`}>
                <Text class={`w-[200px] text-[13px] ${C.ink}`}>{o().label}</Text>
                <Text class={`w-[130px] text-[12.5px] ${C.mute}`}>{o().range_label}</Text>
                <Text class={`w-[120px] text-right text-[12.5px] font-mono ${C.ink}`}>{o().amount_shown}</Text>
                <Text class={`flex-1 text-[12px] ${C.dim} truncate`}>
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
            <View
              class="h-[38px] px-5 rounded-lg items-center justify-center transition-colors bg-[#0d9488] hover:bg-[#0f766e]"
              activeStyle={{ opacity: 0.7 }}
              onPress={addOpening}
            >
              <Text class="text-[13px] font-medium text-white">Save</Text>
            </View>
          </View>
          <If when={Boolean(err())}>
            <Text class={`text-[12.5px] ${C.red}`}>{err()}</Text>
          </If>
        </Panel>
      </If>
    </View>
  );
}
