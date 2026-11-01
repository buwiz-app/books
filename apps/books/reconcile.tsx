// Reconcile: accounts down, months across (red means to do), then a
// statement beside Books with matches, suggestions and missing lines, the
// difference, and Finish (drawing: Books Mini App › Reconcile).

const recAccount = signal("");
const recMonth = signal("");

const CELL: Record<string, string> = {
  done: PILL.ok,
  todo: PILL.rev,
  draft: PILL.draft,
  none: "",
};

/** Bytes as base64url, for a file part. */
function b64url(bytes: Any): string {
  const A = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
  let out = "";
  for (let i = 0; i < bytes.length; i += 3) {
    const n = (bytes[i] << 16) | ((bytes[i + 1] || 0) << 8) | (bytes[i + 2] || 0);
    const left = bytes.length - i;
    out += A[(n >> 18) & 63] + A[(n >> 12) & 63];
    if (left > 1) out += A[(n >> 6) & 63];
    if (left > 2) out += A[n & 63];
  }
  return out;
}

/** A picked file to Books in parts (one call's arguments are at most 256 KiB);
 * returns the upload id an import names. */
async function sendFile(file: Any): Promise<string> {
  const bytes = await file.read();
  const size = 96000;
  const parts = Math.max(1, Math.ceil(bytes.length / size));
  const upload = `up-${rid().slice(3)}`;
  for (let i = 0; i < parts; i += 1) {
    await buwiz.call("books_app_upload_part", { upload, part: i, parts, data_base64: b64url(bytes.slice(i * size, (i + 1) * size)) });
  }
  return upload;
}

function importStatement(account: string) {
  if (!buwiz.pickFile) return toast("Picking files is not available in this Buwiz Bot.");
  buwiz
    .pickFile({ extensions: [".csv", ".xlsx", ".xls"] })
    .then(async (files: Any[]) => {
      if (!files.length) return;
      const upload = await sendFile(files[0]);
      const r: Any = await act("books_app_statement_import", { account, csv_upload: upload, file_name: String(files[0].name).slice(0, 120) });
      recAccount.set(account);
      recMonth.set(r.month);
      toast(`${String(r.lines)} lines imported`);
    })
    .catch((e: Any) => toast(errText(e)));
}

function ReconcileGrid() {
  const data = resource(
    () => ({ v: ver() }),
    () => buwiz.call("books_reconcile_grid", {}),
  );
  const d = () => data() as Any;
  return (
    <View class="gap-3.5">
      <Head title="Reconcile" sub={d() ? String(d().year) : ""}>
        <Btn label="Import statement" tone="go" onPress={() => d()?.rows?.[0] && importStatement(d().rows[0].ticket)} />
      </Head>
      <Loading res={data} />
      <If when={Boolean(d())}>
        <View class="gap-3.5">
          <View class="flex-row gap-2.5">
            <Each of={d()?.kpis || []}>
              {(k: Any) => (
                <Panel class="flex-1 px-3.5 py-3 gap-1">
                  <Text class={`text-[11.5px] uppercase tracking-wider ${C.mute}`}>{k().label}</Text>
                  <Text class={`text-[18px] font-semibold ${C.ink}`}>{k().shown}</Text>
                </Panel>
              )}
            </Each>
          </View>
          <Panel class="p-1">
            <View class="flex-row gap-2 px-3 py-2">
              <Text class={`flex-1 text-[10.5px] uppercase tracking-wider ${C.dim}`}>Account</Text>
              <Each of={d()?.months || []}>{(m: Any) => <Text class={`w-[120px] text-[10.5px] uppercase tracking-wider ${C.dim}`}>{m().label}</Text>}</Each>
            </View>
            <Each of={d()?.rows || []} key={rowKey}>
              {(r: Any) => (
                <View class={`flex-row items-center gap-2 px-3 py-2.5 border-t ${C.line}`}>
                  <Text class={`flex-1 text-[13px] font-semibold ${C.ink}`} numberOfLines={1}>
                    {r().name}
                  </Text>
                  <Each of={r().cells}>
                    {(c: Any) => (
                      <View class="w-[120px]">
                        <If when={c().state !== "none"}>
                          <View
                            class={`rounded-lg px-2 py-1 self-start ${CELL[c().state]}`}
                            onPress={() => {
                              recAccount.set(r().ticket);
                              recMonth.set(c().month);
                            }}
                          >
                            <Text class={`text-[12px] ${CELL[c().state]}`}>{c().state === "done" ? c().label : `${c().label} ›`}</Text>
                          </View>
                        </If>
                        <If when={c().state === "none"}>
                          <Text class={`text-[12px] ${C.dim}`}>—</Text>
                        </If>
                      </View>
                    )}
                  </Each>
                </View>
              )}
            </Each>
          </Panel>
          <Text class={`text-[12px] ${C.dim}`}>Click a red or yellow month to match it.</Text>
        </View>
      </If>
    </View>
  );
}

const MARK: Record<string, string> = { m: "=", s: "≈", x: "?" };

function MatchView() {
  const data = resource(
    () => ({ v: ver(), a: recAccount(), m: recMonth() }),
    (k: Any) => buwiz.call("books_statement_get", { account: k.a, month: k.m }),
  );
  const d = () => data() as Any;
  const s = () => d()?.statement as Any;
  const call = (tool: string, extra: Record<string, Any> = {}) =>
    act(tool, { account: recAccount(), month: recMonth(), ...extra }).catch(() => null);
  const selectedLine = signal<number | null>(null);
  const snapPage = signal(0);
  return (
    <View class="gap-3.5">
      <Head title={`Match ${d()?.month?.short || ""} · ${d()?.account?.name || ""}`} sub="statement vs Books">
        <View class="flex-row gap-2">
          <Btn label="‹ Back" onPress={() => recMonth.set("")} />
          <If when={Boolean(s())}>
            <Btn label={`Accept ${String(s()?.suggested || 0)} suggestion${s()?.suggested === 1 ? "" : "s"}`} disabled={!s()?.suggested} onPress={() => call("books_app_statement_accept")} />
          </If>
          <If when={s()?.status !== "finished"}>
            <Btn label="Import statement" onPress={() => importStatement(recAccount())} />
          </If>
        </View>
      </Head>
      <Loading res={data} />
      <If when={Boolean(d()?.draft)}>
        <DraftBanner account={recAccount()} month={recMonth()} draft={d()?.draft} />
      </If>
      <If when={Boolean(d()) && !s()}>
        <Panel pad>
          <Text class={`text-[13px] ${C.mute}`}>No statement for this month yet. Import the bank's CSV to match it.</Text>
        </Panel>
      </If>
      <If when={Boolean(s())}>
        <View class="gap-3.5">
          <If when={(s()?.flags || []).length > 0}>
            <FlagsNote flags={s()?.flags || []} />
          </If>
          <Panel>
            <View class={`flex-row border-b ${C.line}`}>
              <Text class={`flex-1 px-3 py-2.5 text-[11px] uppercase tracking-wider ${C.dim}`}>Bank statement</Text>
              <View class="w-[28px]" />
              <Text class={`flex-1 px-3 py-2.5 text-[11px] uppercase tracking-wider ${C.dim}`}>Books</Text>
            </View>
            <Each of={s()?.rows || []} key={rowKey}>
              {(r: Any) => (
                <View
                  class={`flex-row items-center border-b ${C.line} ${r().no === selectedLine() ? C.panel : ""}`}
                  onPress={() => {
                    selectedLine.set(r().no ?? null);
                    if (r().page) snapPage.set(Number(r().page));
                  }}
                >
                  <View class="flex-1 flex-row items-center gap-2 px-3 py-2 min-h-[38px]">
                    <Text class={`w-[52px] text-[12px] font-mono ${C.ink}`}>{r().date_short}</Text>
                    <If when={Boolean(r().flag)}>
                      <View class="w-[7px] h-[7px] rounded-full bg-[#f3b64c]" />
                    </If>
                    <Text class={`flex-1 text-[12.5px] ${C.ink}`} numberOfLines={1}>
                      {r().description}
                    </Text>
                    <If when={Boolean(r().low)}>
                      <Pill tone="draft" label={`read ${String(r().confidence)}%`} />
                    </If>
                    <Text class={`text-[12.5px] font-mono ${C.ink}`}>{r().amount_shown}</Text>
                  </View>
                  <Text class={`w-[28px] text-center font-bold ${r().match === "m" ? C.green : r().match === "s" ? C.amber : C.red}`}>{MARK[r().match]}</Text>
                  <View
                    class={`flex-1 flex-row items-center gap-2 px-3 py-2 min-h-[38px] ${r().match === "x" ? PILL.rev : r().match === "s" ? PILL.draft : ""}`}
                    onPress={() => r().books?.ticket && drawerEntry.set(r().books.ticket)}
                  >
                    <Text class={`w-[52px] text-[12px] font-mono ${C.ink}`}>{r().books?.date_short || "—"}</Text>
                    <View class="flex-1">
                      <Text class={`text-[12.5px] font-semibold ${C.ink}`}>{r().books?.title || ""}</Text>
                      <Text class={`text-[11px] ${C.dim}`}>{r().why || r().books?.sub || ""}</Text>
                      <If when={Boolean(r().hint?.category)}>
                        <Text class={`text-[11px] ${C.dim}`}>{`Usually ${r().hint?.category || ""}`}</Text>
                      </If>
                    </View>
                    <If when={r().match === "x"}>
                      <Btn label="+ Add" small onPress={() => call("books_app_statement_add_line", { line: r().ticket })} />
                    </If>
                    <If when={r().match === "s" && s()?.status !== "finished"}>
                      <Btn label="Match" small onPress={() => call("books_app_statement_accept", { line: r().no })} />
                    </If>
                    <If when={r().match !== "x"}>
                      <Text class={`text-[12.5px] font-mono ${C.ink}`}>{r().books?.amount_shown || ""}</Text>
                    </If>
                  </View>
                </View>
              )}
            </Each>
          </Panel>
          <If when={snapShown(s())}>
            <SnapshotPane
              rows={s()?.rows || []}
              evidence={Boolean(s()?.snapshot)}
              selected={selectedLine()}
              onSelect={(no: number) => selectedLine.set(no)}
              page={snapPage()}
              onPage={(n: number) => snapPage.set(n)}
              account={recAccount()}
              month={recMonth()}
            />
          </If>
          <If when={!snapShown(s())}>
            <PrintReconciliation account={recAccount()} month={recMonth()} />
          </If>
          <Panel pad class="flex-row items-center gap-4">
            <Text class={`text-[12.5px] ${C.mute}`}>{`Statement ending ${s()?.closing_shown || ""}`}</Text>
            <Text class={`text-[12.5px] ${C.mute}`}>{`Books ${s()?.books_shown || ""}`}</Text>
            <View class="flex-1" />
            <Text class={`text-[12.5px] font-mono font-bold ${s()?.difference === "0.00" ? C.green : C.ink}`}>{`Difference ${s()?.difference_shown || ""}`}</Text>
            <If when={s()?.status !== "finished"}>
              <Btn label={`Finish ${d()?.month?.short || ""}`} tone="go" disabled={!s()?.can_finish} onPress={() => call("books_app_statement_finish").then(() => recMonth.set(""))} />
            </If>
            <If when={s()?.status === "finished"}>
              <Pill tone="ok" label="Matched" />
            </If>
          </Panel>
          <If when={Boolean(s()?.reopenable)}>
            <ReopenFooter onReopen={(reason: string) => call("books_app_statement_finish", { reopen: true, reason })} />
          </If>
        </View>
      </If>
    </View>
  );
}

function ReconcilePage(p: { go: (page: string, filter?: string) => void }) {
  return (
    <View>
      <If when={!recMonth()}>
        <ReconcileGrid />
      </If>
      <If when={Boolean(recMonth())}>
        <MatchView />
      </If>
    </View>
  );
}
