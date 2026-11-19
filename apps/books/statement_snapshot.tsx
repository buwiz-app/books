// Reconcile: the statement snapshot beside the match list.
// A PNG or JPEG snapshot is drawn here: its bytes come from
// books_app_statement_page in parts (one answer is at most 1 MiB) and join
// into one data: source. Boxes come from Rust as percent text and lie over
// it. A PDF, and the report, show through buwiz.preview (macOS), which runs
// books_app_reconcile_document as the app.

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

/** The rows the pane can draw: those with a page and a box. */
function snapPaged(rows: Any[]): Any[] {
  return (rows || []).filter((r: Any) => r.page && r.box);
}

/** Whether MatchView shows the pane: a snapshot ticket or a row to draw. */
function snapShown(s: Any): boolean {
  return Boolean(s?.snapshot) || snapPaged(s?.rows || []).length > 0;
}

const SNAP_W = 380;
/** The pane's height with no image to measure: a letter page. */
const SNAP_H = 492;
const SNAP_NOTE: Record<string, string> = {
  pdf: "This statement is a PDF. Open it with See the page.",
  large: "This page is too large to show here. Open it with See the page.",
  other: "This document cannot be shown here.",
};

/** The last page image read, so a write elsewhere on the screen does not read it again. */
let snapKept: { key: string; image: Any } | null = null;

/** The statement's snapshot image as one data: source, with its size in pixels. */
async function snapImage(key: string, account: string, month: string): Promise<Any> {
  if (snapKept && snapKept.key === key) return snapKept.image;
  let data = "";
  let first: Any = null;
  let parts = 1;
  for (let i = 0; i < parts; i += 1) {
    const r: Any = await buwiz.call("books_app_statement_page", { account, month, part: i });
    if (!first) {
      first = r;
      parts = Math.max(1, Number(r.parts) || 1);
    }
    data += String(r.data_base64 || "");
  }
  const image = { src: `data:${String(first.mime)};base64,${data}`, width: Number(first.width) || 0, height: Number(first.height) || 0 };
  snapKept = { key, image };
  return image;
}

// One page signal: the tabs set it, and so does selecting a row (p.onPage).
function SnapshotPane(p: {
  rows: Any[];
  evidence: boolean;
  kind: string;
  doc: string;
  selected: number | null;
  onSelect: (no: number) => void;
  page: number;
  onPage: (page: number) => void;
  account: string;
  month: string;
}) {
  const paged = () => snapPaged(p.rows);
  const pages = () => [...new Set(paged().map((r: Any) => Number(r.page)))].sort((a, b) => a - b);
  const chosen = () => (p.rows || []).find((r: Any) => r.no === p.selected);
  const page = () => (pages().includes(p.page) ? p.page : pages()[0] || 1);
  const onPage = () => paged().filter((r: Any) => Number(r.page) === page());
  const image = resource(
    () => ({ k: p.kind, a: p.account, m: p.month, d: p.doc }),
    (k: Any) => (k.k === "image" ? snapImage(`${k.a}|${k.m}|${k.d}`, k.a, k.m).catch(() => null) : Promise.resolve(null)),
  );
  // an image is one page: its boxes are page 1's
  const img = () => (p.kind === "image" && page() === 1 ? (image() as Any) : null);
  const height = () => (img()?.width > 0 && img()?.height > 0 ? Math.round((SNAP_W * img().height) / img().width) : SNAP_H);
  const note = () => (p.kind === "image" ? (image.loading() ? "Loading the page…" : img() ? "" : page() === 1 ? "The page could not be shown here." : "Only the first page has a picture here.") : SNAP_NOTE[p.kind] || "");
  return (
    <View class="gap-2" style={{ width: SNAP_W }}>
      <If when={pages().length > 1}>
        <Seg options={pages().map((n: number) => [String(n), `Page ${n}`] as [string, string])} value={String(page())} onPick={(v: string) => p.onPage(Number(v))} />
      </If>
      {/* above the page, so they show without scrolling */}
      <View class="flex-row gap-2 flex-wrap">
        <If when={Boolean(buwiz.preview) && p.evidence}>
          <Btn label="See the page" onPress={() => showPreview({ account: p.account, month: p.month, kind: "page", page: page(), line: p.selected ?? undefined })} />
        </If>
        <PrintReconciliation account={p.account} month={p.month} />
      </View>
      <If when={Boolean(note())}>
        <Text class={`text-[12px] ${C.dim}`}>{note()}</Text>
      </If>
      <If when={Boolean(img()) || paged().length > 0}>
        <Panel class="relative overflow-hidden">
          <View style={{ width: SNAP_W, height: height() }}>
            <If when={Boolean(img())}>
              <Avatar src={img()?.src || ""} accessibilityLabel="Statement page" style={{ width: SNAP_W, height: height() }} imageStyle={{ width: SNAP_W, height: height() }} />
            </If>
            <Each of={onPage()} key={(r: Any) => String(r.no)}>
              {(r: Any) => (
                <View
                  class={`absolute border-2 ${SNAP_EDGE[r().match] || SNAP_EDGE.x} ${r().no === p.selected ? SNAP_FILL[r().match] || SNAP_FILL.x : ""}`}
                  style={{ left: r().box.left, top: r().box.top, width: r().box.width, height: r().box.height }}
                  accessibilityLabel="Show this statement line"
                  onPress={() => p.onSelect(r().no)}
                />
              )}
            </Each>
          </View>
        </Panel>
      </If>
      <If when={chosen()?.match === "x" && paged().length > 0}>
        <Btn label="Create from this line" onPress={() => act("books_app_statement_add_line", { account: p.account, month: p.month, line: chosen().ticket }).catch(() => null)} />
      </If>
    </View>
  );
}
