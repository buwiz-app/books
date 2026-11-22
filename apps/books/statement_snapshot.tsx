// Reconcile: the statement snapshot beside the match list.
// Each page that has a picture is drawn here: a PNG or JPEG of its own, or
// a page of the statement's PDF that Books drew to PNG when the document was
// kept. Its bytes come from books_app_statement_page in parts (one answer is
// at most 1 MiB) and join into one data: source. Boxes come from Rust as
// percent text and lie over the picture, never over an empty panel: a page
// without a picture shows a note and "See the page". The preview and the
// report show through buwiz.preview (macOS), which runs
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
const SNAP_SOFT: Record<string, string> = { m: "bg-green-600/15", s: "bg-amber-500/15", x: "bg-red-600/15" };
/** The pane's page area scrolls its picture when it is taller than this. */
const SNAP_SCROLL_H = 600;

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
/** Why a page has no picture to draw, by what the statement's document is. */
const SNAP_NOTE: Record<string, string> = {
  pdf: "This PDF's pages could not be drawn here. Open it with See the page.",
  large: "This page is too large to show here. Open it with See the page.",
  other: "This document cannot be shown here.",
};

/** The last page images read, so paging back or a write elsewhere does not read them again. */
const snapKept: { key: string; image: Any }[] = [];

/** One page's picture as one data: source, with its size in pixels and the key it was read for. */
async function snapImage(key: string, account: string, month: string, page: number): Promise<Any> {
  const kept = snapKept.find((k) => k.key === key);
  if (kept) return kept.image;
  let data = "";
  let first: Any = null;
  let parts = 1;
  for (let i = 0; i < parts; i += 1) {
    const r: Any = await buwiz.call("books_app_statement_page", { account, month, page, part: i });
    if (!first) {
      first = r;
      parts = Math.max(1, Number(r.parts) || 1);
    }
    data += String(r.data_base64 || "");
  }
  const image = { key, src: `data:${String(first.mime)};base64,${data}`, width: Number(first.width) || 0, height: Number(first.height) || 0 };
  snapKept.push({ key, image });
  if (snapKept.length > 3) snapKept.shift();
  return image;
}

// One page signal: the tabs set it, and so does selecting a row (p.onPage).
function SnapshotPane(p: {
  rows: Any[];
  evidence: boolean;
  kind: string;
  /** The pages that have a picture the window can draw. */
  pictures: number[];
  doc: string;
  /** Which pictures the pages have: it changes when any page's picture does. */
  stamp: string;
  selected: number | null;
  onSelect: (no: number) => void;
  /** The line the pointer is on, either side (its box draws lit). */
  hovered: number | null;
  /** A box's pointer entering (its line no) or leaving (null). */
  onHover: (no: number | null) => void;
  /** Whole-field boxes (title, date) from the statement, labelled on hover. */
  areas: Any[];
  /** Receives { scrollToLine, cancel } for the right side's row hover. */
  bind: (api: Any) => void;
  page: number;
  onPage: (page: number) => void;
  account: string;
  month: string;
}) {
  const paged = () => snapPaged(p.rows);
  const pictures = () => (p.pictures || []).map(Number);
  // every page a line is on, boxed or not, so picking a line never opens another page
  const pages = () => [...new Set([...(p.rows || []).filter((r: Any) => r.page).map((r: Any) => Number(r.page)), ...pictures()])].sort((a, b) => a - b);
  const chosen = () => (p.rows || []).find((r: Any) => r.no === p.selected);
  const page = () => (pages().includes(p.page) ? p.page : pages()[0] || 1);
  const pictured = () => pictures().includes(page());
  const key = () => `${p.account}|${p.month}|${p.doc}|${p.stamp}|${String(page())}`;
  const image = resource(
    () => ({ key: key(), a: p.account, m: p.month, n: page(), has: pictured() }),
    (k: Any) => (k.has ? snapImage(k.key, k.a, k.m, k.n).catch(() => null) : Promise.resolve(null)),
  );
  // only the picture read for this very page: never another page's while this one loads
  const img = () => {
    const i = image() as Any;
    return pictured() && i && i.key === key() ? i : null;
  };
  // boxes lie over a picture only; a page without one shows the note instead
  const onPage = () => (img() ? paged().filter((r: Any) => Number(r.page) === page()) : []);
  const height = () => (img()?.width > 0 && img()?.height > 0 ? Math.round((SNAP_W * img().height) / img().width) : SNAP_H);
  const note = () => {
    if (img()) return "";
    if (pictured()) return image.loading() ? "Loading the page…" : "The page could not be shown here. Open it with See the page.";
    if (p.kind === "image" || pictures().length > 0) return `No picture was kept for page ${String(page())}, so its lines are in the list only.`;
    return SNAP_NOTE[p.kind] || "";
  };
  // hover → scroll: a row hover on the right asks the pane to show its box
  let scRef: Any;
  const wish = signal<number | null>(null);
  const boxTop = new Map<number, number>();
  const runWish = () => {
    const no = wish();
    if (no === null || !img()) return;
    const y = boxTop.get(no);
    if (y === undefined) return;
    scRef?.scrollTo?.({ y: Math.max(0, y - 24), animated: true });
    wish.set(null);
  };
  const scrollToLine = (no: number) => {
    const r = (p.rows || []).find((x: Any) => x.no === no);
    if (!r?.page || !r?.box) return;
    if (Number(r.page) !== page()) p.onPage(Number(r.page));
    wish.set(no);
    runWish();
  };
  p.bind({ scrollToLine, cancel: () => wish.set(null) });
  // area boxes (title, date) are quieter than line boxes and name on hover
  const areasPage = () => (img() ? (p.areas || []).filter((a: Any) => a?.box && Number(a.page) === page()) : []);
  const hoverArea = signal("");
  const areaKey = (a: Any) => `${String(a?.kind)}|${String(a?.label)}|${String(a?.page)}|${String(a?.box?.top)}`;
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
      <If when={Boolean(img())}>
        <Panel class="relative overflow-hidden">
          <Scroll ref={scRef} class={`max-h-[${SNAP_SCROLL_H}px]`}>
            <View style={{ width: SNAP_W, height: height() }} onLayout={runWish}>
              <Avatar src={img()?.src || ""} accessibilityLabel="Statement page" style={{ width: SNAP_W, height: height() }} imageStyle={{ width: SNAP_W, height: height() }} />
              <Each of={areasPage()} key={(a: Any) => areaKey(a)}>
                {(a: Any) => (
                  <View
                    class={`absolute border ${hoverArea() === areaKey(a()) ? "border-sky-500 dark:border-sky-400 bg-sky-500/10" : "border-dashed border-slate-400/70 dark:border-slate-500/60"}`}
                    style={{ left: a().box.left, top: a().box.top, width: a().box.width, height: a().box.height }}
                    accessibilityLabel={String(a().label || a().kind || "Statement field")}
                    onMouseEnter={() => hoverArea.set(areaKey(a()))}
                    onMouseLeave={() => hoverArea.set("")}
                  >
                    <If when={hoverArea() === areaKey(a())}>
                      <View class="absolute -top-4 left-0 rounded-sm px-1 py-px bg-slate-700/90 dark:bg-slate-200/90">
                        <Text class="text-[9px] font-medium text-white dark:text-slate-800">{a().label || a().kind}</Text>
                      </View>
                    </If>
                  </View>
                )}
              </Each>
              <Each of={onPage()} key={(r: Any) => String(r.no)}>
                {(r: Any) => (
                  <View
                    class={`absolute border-2 ${SNAP_EDGE[r().match] || SNAP_EDGE.x} ${
                      r().no === p.selected
                        ? SNAP_FILL[r().match] || SNAP_FILL.x
                        : r().no === p.hovered
                          ? SNAP_SOFT[r().match] || SNAP_SOFT.x
                          : r().match === "x"
                            ? "border-dashed"
                            : ""
                    }`}
                    style={{ left: r().box.left, top: r().box.top, width: r().box.width, height: r().box.height }}
                    accessibilityLabel="Show this statement line"
                    onLayout={(e: Any) => {
                      const y = Number(e?.bounds?.origin?.y) || 0;
                      boxTop.set(r().no, y);
                      if (r().no === wish()) runWish();
                    }}
                    onMouseEnter={() => p.onHover(r().no)}
                    onMouseLeave={() => p.onHover(null)}
                    onPress={() => p.onSelect(r().no)}
                  />
                )}
              </Each>
            </View>
          </Scroll>
        </Panel>
      </If>
      <If when={chosen()?.match === "x" && paged().length > 0}>
        <Btn label="Create from this line" onPress={() => act("books_app_statement_add_line", { account: p.account, month: p.month, line: chosen().ticket }).catch(() => null)} />
      </If>
    </View>
  );
}
