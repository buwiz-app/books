// Reconcile: the statement snapshot beside the match list (M10 parity).
// Boxes come from Rust as percent text; the real page and the report show
// through buwiz.preview (macOS), which runs books_app_reconcile_document as the app.

async function showPreview(args: Record<string, Any>) {
  if (!buwiz.preview) return;
  try {
    await buwiz.preview({ tool: "books_app_reconcile_document", args });
  } catch (e: Any) {
    toast(String(e?.code || e?.message || "").includes("not_supported") ? "Preview needs Buwiz Bot on a Mac" : "Could not open the preview");
  }
}

function PrintReconciliation(p: { account: string; month: string }) {
  return (
    <If when={Boolean(buwiz.preview)}>
      <Btn label="Print reconciliation" onPress={() => showPreview({ account: p.account, month: p.month, kind: "report" })} />
    </If>
  );
}

const SNAP_EDGE: Record<string, string> = { m: "border-green-600", s: "border-amber-500", x: "border-red-600" };
const SNAP_FILL: Record<string, string> = { m: "bg-green-600/30", s: "bg-amber-500/30", x: "bg-red-600/30" };

function SnapshotPane(p: { rows: Any[]; evidence: boolean; selected: number | null; onSelect: (no: number) => void; account: string; month: string }) {
  const paged = () => (p.rows || []).filter((r: Any) => r.page && r.box);
  const pages = () => [...new Set(paged().map((r: Any) => Number(r.page)))].sort((a, b) => a - b);
  const picked = signal(0);
  const chosen = () => (p.rows || []).find((r: Any) => r.no === p.selected);
  const page = () => (chosen()?.page ? Number(chosen().page) : picked() || pages()[0] || 1);
  const onPage = () => paged().filter((r: Any) => Number(r.page) === page());
  return (
    <If when={paged().length > 0}>
      <View class="gap-2">
        <If when={pages().length > 1}>
          <Seg options={pages().map((n: number) => [String(n), `Page ${n}`] as [string, string])} value={String(page())} onPick={(v: string) => picked.set(Number(v))} />
        </If>
        <Panel class="relative w-[260px] h-[336px]">
          <Each of={onPage()} key={(r: Any) => String(r.no)}>
            {(r: Any) => (
              <View
                class={`absolute border-2 ${SNAP_EDGE[r().match] || SNAP_EDGE.x} ${r().no === p.selected ? SNAP_FILL[r().match] || SNAP_FILL.x : ""}`}
                style={{ left: r().box.left, top: r().box.top, width: r().box.width, height: r().box.height }}
                onPress={() => p.onSelect(r().no)}
              />
            )}
          </Each>
        </Panel>
        <View class="flex-row gap-2">
          <If when={Boolean(buwiz.preview) && p.evidence}>
            <Btn label="See the page" onPress={() => showPreview({ account: p.account, month: p.month, kind: "page", page: page(), line: p.selected ?? undefined })} />
          </If>
          <PrintReconciliation account={p.account} month={p.month} />
        </View>
        <If when={chosen()?.match === "x"}>
          <Btn label="Create from this line" onPress={() => act("books_app_statement_add_line", { account: p.account, month: p.month, line: chosen().ticket }).catch(() => null)} />
        </If>
      </View>
    </If>
  );
}
