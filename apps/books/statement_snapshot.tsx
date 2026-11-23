// Reconcile: the statement document viewer beside the match list, ported 1:1
// from the old web app's ReconciliationDocumentViewer.tsx.
//
// Each page that has a picture is drawn here: a PNG or JPEG of its own, or
// a page of the statement's PDF that Books drew to PNG when the document was
// kept. Its bytes come from books_app_statement_page in parts (one answer is
// at most 1 MiB) and join into one data: source. Boxes come from Rust as
// percent text and lie over the picture, never over an empty panel.
//
// Like the old viewer: a toolbar (zoom out / % / zoom in / Fit, page
// chevrons, and the optional Scan / Re-extract / Remove / Open actions), a
// dismissible error + warnings banner, a cyan "reconcile mode" strip, green
// matched / amber dashed unmatched boxes with a cyan clickable state under
// reconcileMode, a hover popover offering a quick match or a new entry, and
// an upload zone when no document is attached. Zoom scales the page and all
// boxes together inside the scroll area (the scroll is the pan). Ghost
// boxes — unmatched boxes covered more than 30% by a matched box — are
// hidden, the same filter the old app applies before drawing.
//
// New OPTIONAL props on SnapshotPane (old callers unchanged):
//   reconcileMode        boxes' click means "pick this line to reconcile"
//   onReconcileSelect    (no) clicked an unmatched box in reconcileMode
//   onQuickMatch         (no, ledgerId) the popover's Link Transaction
//   matchFor             (row) -> { id, label } | null: quick-match candidate
//   onCreateFromLine     (no) the popover's Create Transaction
//   matchingNo           the line that just matched (flashes once)
//   onScanFields/scanning, onReextract/extracting, onRemove/removing,
//   onOpen               toolbar actions, rendered only when provided
//   onUpload/uploading/uploadError/warnings/onGenerate/generating
//                        the empty-state upload zone

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

// Old app's box colors: #10b981 matched, #f59e0b unmatched, #06b6d4 the
// reconcile target. "s" (Books' suggested match) keeps its own amber-solid
// look between matched green and the unmatched dash.
const SNAP_EDGE: Record<string, string> = { m: "border-emerald-500", s: "border-amber-500", x: "border-amber-500" };
const SNAP_FILL: Record<string, string> = { m: "bg-emerald-500/25", s: "bg-amber-500/25", x: "bg-amber-500/25" };
const SNAP_SOFT: Record<string, string> = { m: "bg-emerald-500/10", s: "bg-amber-500/10", x: "bg-amber-500/10" };
const SNAP_DIM: Record<string, string> = { m: "bg-emerald-500/5", s: "bg-amber-500/5", x: "bg-amber-500/5" };
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
const SNAP_NOTE: Record<string, string> = {
  pdf: "This PDF's pages could not be drawn here. Open it with See the page.",
  large: "This page is too large to show here. Open it with See the page.",
  other: "This document cannot be shown here.",
};

// Old viewer's zoom: MIN 50%, MAX 400%, STEP 25%, Fit = 100%. Whole percents
// keep every zoom level an exact integer of SNAP_W, no floats needed.
const SNAP_ZOOM_MIN = 50;
const SNAP_ZOOM_MAX = 400;
const SNAP_ZOOM_STEP = 25;

/** "12.3%" -> 123: a box edge back in the statement's 0-1000 units, int only. */
function snapMil(v: Any): number {
  const s = String(v ?? "0").replace("%", "");
  const dot = s.indexOf(".");
  const w = dot < 0 ? s : s.slice(0, dot);
  const d = dot < 0 ? "0" : s.slice(dot + 1, dot + 2);
  return (parseInt(w, 10) || 0) * 10 + (parseInt(d, 10) || 0);
}

/** A row's box as { l, t, r, b } in 0-1000 units (integer math only). */
function snapBoxInt(box: Any): { l: number; t: number; r: number; b: number } {
  const l = snapMil(box?.left);
  const t = snapMil(box?.top);
  return { l, t, r: l + snapMil(box?.width), b: t + snapMil(box?.height) };
}

/** Ghost filter, the old app's: an unmatched ("x") box covered more than 30%
 * by any matched ("m") box on the same page is a duplicate and hidden. */
function snapGhostFree(rows: Any[]): Any[] {
  const matched = (rows || []).filter((r: Any) => r.match === "m");
  if (matched.length === 0) return rows || [];
  return (rows || []).filter((r: Any) => {
    if (r.match !== "x") return true;
    const b = snapBoxInt(r.box);
    const bArea = (b.r - b.l) * (b.b - b.t);
    return !matched.some((m: Any) => {
      if (Number(m.page) !== Number(r.page)) return false;
      const mb = snapBoxInt(m.box);
      const ox = Math.max(0, Math.min(b.r, mb.r) - Math.max(b.l, mb.l));
      const oy = Math.max(0, Math.min(b.b, mb.b) - Math.max(b.t, mb.t));
      return ox * oy * 10 > bArea * 3;
    });
  });
}

/** The last page images read, so paging back or a write elsewhere does not read them again. */
const snapKept: { key: string; image: Any }[] = [];

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

// The toolbar's small square button, the old viewer's w-7 h-7 rounded-md.
function SnapTool(p: { label: string; onPress: () => void; disabled?: boolean; acc?: string; tint?: string }) {
  return (
    <View
      class={`w-7 h-7 items-center justify-center rounded-md transition-colors ${p.disabled ? "opacity-30" : p.tint || "hover:bg-gray-100 dark:hover:bg-white/5"} cursor-pointer`}
      accessibilityLabel={p.acc || p.label}
      onPress={() => {
        if (!p.disabled) p.onPress();
      }}
    >
      <Text class="text-[13px] text-gray-500 dark:text-gray-400">{p.label}</Text>
    </View>
  );
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
  /* --- optional parity props (ReconciliationDocumentViewer 1:1) --- */
  reconcileMode?: boolean;
  onReconcileSelect?: (no: number) => void;
  onQuickMatch?: (no: number, ledgerId: string) => void;
  matchFor?: (row: Any) => Any;
  onCreateFromLine?: (no: number) => void;
  matchingNo?: number | null;
  onScanFields?: () => void;
  scanning?: boolean;
  onReextract?: () => void;
  extracting?: boolean;
  onRemove?: () => void;
  removing?: boolean;
  onOpen?: () => void;
  onUpload?: () => void;
  uploading?: boolean;
  uploadError?: string;
  warnings?: string[];
  onGenerate?: () => void;
  generating?: boolean;
}) {
  const paged = () => snapGhostFree(snapPaged(p.rows));
  const pictures = () => (p.pictures || []).map(Number);
  const pages = () => [...new Set([...(p.rows || []).filter((r: Any) => r.page).map((r: Any) => Number(r.page)), ...pictures()])].sort((a, b) => a - b);
  const chosen = () => (p.rows || []).find((r: Any) => r.no === p.selected);
  const page = () => (pages().includes(p.page) ? p.page : pages()[0] || 1);
  const pictured = () => pictures().includes(page());
  const key = () => `${p.account}|${p.month}|${p.doc}|${p.stamp}|${String(page())}`;
  const image = resource(
    () => ({ key: key(), a: p.account, m: p.month, n: page(), has: pictured() }),
    (k: Any) => (k.has ? snapImage(k.key, k.a, k.m, k.n).catch(() => null) : Promise.resolve(null)),
  );
  const img = () => {
    const i = image() as Any;
    return pictured() && i && i.key === key() ? i : null;
  };
  const onPage = () => (img() ? paged().filter((r: Any) => Number(r.page) === page()) : []);
  const baseH = () => (img()?.width > 0 && img()?.height > 0 ? Math.round((SNAP_W * img().height) / img().width) : SNAP_H);
  const note = () => {
    if (img()) return "";
    if (pictured()) return image.loading() ? "Loading the page…" : "The page could not be shown here. Open it with See the page.";
    if (p.kind === "image" || pictures().length > 0) return `No picture was kept for page ${String(page())}, so its lines are in the list only.`;
    return SNAP_NOTE[p.kind] || "";
  };

  // zoom + pan: the page view is drawn zoom%/100 bigger inside the scroll;
  // scrolling is the pan (the shell has no mouse-drag pan gesture).
  const zoom = signal(100);
  const scaleW = () => Math.round((SNAP_W * zoom()) / 100);
  const scaleH = () => Math.round((baseH() * zoom()) / 100);

  // flash the box that just matched, the old app's 1.2s highlight
  const flash = signal<number | null>(null);
  const checkFlash = () => {
    if (p.matchingNo != null && flash() !== p.matchingNo) {
      flash.set(p.matchingNo);
      setTimeout(() => flash.set(null), 1200);
    }
    return flash();
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

  const areasPage = () => (img() ? (p.areas || []).filter((a: Any) => a?.box && Number(a.page) === page()) : []);
  const hoverArea = signal("");
  const areaKey = (a: Any) => `${String(a?.kind)}|${String(a?.label)}|${String(a?.page)}|${String(a?.box?.top)}`;

  // banners + hover popover
  const bannerGone = signal(false);
  const bannerFor = signal("");
  const bannerText = () => {
    const w = (p.warnings || []).filter(Boolean);
    const text = [p.uploadError || "", ...w].filter(Boolean).join("\n");
    if (text !== bannerFor()) {
      bannerFor.set(text);
      bannerGone.set(false);
    }
    return text;
  };
  const popNo = signal<number | null>(null);
  const popRow = () => (p.rows || []).find((r: Any) => r.no === popNo());
  const popLeft = (r: Any) => `${(snapMil(r?.box?.left) + snapMil(r?.box?.width)) / 10}%`;
  const popTop = (r: Any) => `${snapMil(r?.box?.top) / 10}%`;
  const hoverBox = (no: number | null) => {
    p.onHover(no);
    if (no === null) {
      popNo.set(null);
      return;
    }
    const r = (p.rows || []).find((x: Any) => x.no === no);
    if (r?.match === "x" && !p.reconcileMode && (p.onQuickMatch || p.onCreateFromLine)) popNo.set(no);
    else popNo.set(null);
  };
  const boxCls = (r: Any) => {
    const match = String(r().match || "x");
    const lit = r().no === p.selected;
    const soft = r().no === p.hovered;
    if (r().no === checkFlash()) return "border-emerald-500 border-[3px] bg-emerald-500/25";
    if (p.reconcileMode && match !== "m") return `border-cyan-500 bg-cyan-500/10 ${soft || lit ? "border-2 bg-cyan-500/20" : "border"}`;
    return `border-2 ${SNAP_EDGE[match] || SNAP_EDGE.x} ${lit ? SNAP_FILL[match] || SNAP_FILL.x : soft ? SNAP_SOFT[match] || SNAP_SOFT.x : p.reconcileMode ? SNAP_DIM[match] || SNAP_DIM.x : match === "x" ? "border-dashed bg-amber-500/5" : ""}`;
  };

  const uploads = Boolean(p.onUpload);
  const empty = !img() && uploads && pictures().length === 0 && paged().length === 0;
  return (
    <View class="gap-2" style={{ width: SNAP_W }}>
      <Panel class="overflow-hidden">
        {/* toolbar: zoom −/%/+, Fit, page chevrons, optional actions */}
        <View class="flex-row items-center justify-between px-3 py-2 border-b border-gray-100 dark:border-white/5 bg-white/80 dark:bg-white/[0.02]">
          <View class="flex-row items-center gap-1">
            <SnapTool label="−" acc="Zoom out" disabled={zoom() <= SNAP_ZOOM_MIN} onPress={() => zoom.set(Math.max(SNAP_ZOOM_MIN, zoom() - SNAP_ZOOM_STEP))} />
            <Text class="text-[11px] font-semibold min-w-[3rem] text-center tabular-nums text-gray-400">{zoom()}%</Text>
            <SnapTool label="+" acc="Zoom in" disabled={zoom() >= SNAP_ZOOM_MAX} onPress={() => zoom.set(Math.min(SNAP_ZOOM_MAX, zoom() + SNAP_ZOOM_STEP))} />
            <View class="w-px h-4 mx-1 bg-gray-200 dark:bg-white/10" />
            <View class="px-2 py-1 rounded-md hover:bg-gray-100 dark:hover:bg-white/5 cursor-pointer" accessibilityLabel="Fit" onPress={() => zoom.set(100)}>
              <Text class="text-[11px] font-medium text-gray-400">Fit</Text>
            </View>
          </View>
          <View class="flex-row items-center gap-1">
            <If when={pages().length > 1}>
              <SnapTool label="‹" acc="Previous page" disabled={page() <= pages()[0]} onPress={() => p.onPage(page() - 1)} />
              <Text class="text-[11px] tabular-nums font-medium text-gray-400">{`${page()} / ${pages()[pages().length - 1] || 1}`}</Text>
              <SnapTool label="›" acc="Next page" disabled={page() >= (pages()[pages().length - 1] || 1)} onPress={() => p.onPage(page() + 1)} />
            </If>
            <If when={Boolean(p.onScanFields)}>
              <View class="px-2 py-1 rounded-md hover:bg-blue-50 dark:hover:bg-blue-900/10 cursor-pointer" accessibilityLabel={paged().length > 0 ? "Re-scan" : "Scan Fields"} onPress={() => !p.scanning && p.onScanFields?.()}>
                <Text class="text-[11px] font-medium text-blue-600 dark:text-blue-400">{p.scanning ? "Scanning…" : paged().length > 0 ? "Re-scan" : "Scan Fields"}</Text>
              </View>
            </If>
            <If when={Boolean(p.onReextract)}>
              <View class="px-2 py-1 rounded-md hover:bg-violet-50 dark:hover:bg-violet-900/10 cursor-pointer" accessibilityLabel="Re-extract" onPress={() => !p.extracting && p.onReextract?.()}>
                <Text class="text-[11px] font-medium text-violet-600 dark:text-violet-400">{p.extracting ? "Extracting…" : "↻ Re-extract"}</Text>
              </View>
            </If>
            <If when={Boolean(p.onRemove)}>
              <SnapTool label="🗑" acc="Remove statement" disabled={Boolean(p.removing)} tint="hover:bg-red-50 dark:hover:bg-red-900/10" onPress={() => p.onRemove?.()} />
            </If>
            <If when={Boolean(p.onOpen)}>
              <View class="px-1 cursor-pointer" accessibilityLabel="Open" onPress={() => p.onOpen?.()}>
                <Text class="text-[11px] font-medium text-emerald-600 dark:text-emerald-400">Open ↗</Text>
              </View>
            </If>
          </View>
        </View>
        {/* error + warnings banner, dismissible */}
        <If when={Boolean(bannerText()) && !bannerGone()}>
          <View class={`flex-row items-start gap-2 px-3 py-2 border-b ${p.uploadError ? "bg-red-50 dark:bg-red-950/40 border-red-200 dark:border-red-800/50" : "bg-amber-50 dark:bg-amber-950/40 border-amber-200 dark:border-amber-800/50"}`}>
            <View class="flex-1">
              <Each of={bannerText().split("\n")} key={(t: string) => t}>
                {(t: string) => <Text class={`text-[11px] ${p.uploadError ? "font-medium text-red-700 dark:text-red-300" : "text-amber-700 dark:text-amber-300"}`}>{t}</Text>}
              </Each>
            </View>
            <View class="px-1 cursor-pointer" accessibilityLabel="Dismiss" onPress={() => bannerGone.set(true)}>
              <Text class={`text-[11px] opacity-60 ${p.uploadError ? "text-red-700 dark:text-red-300" : "text-amber-700 dark:text-amber-300"}`}>✕</Text>
            </View>
          </View>
        </If>
        {/* reconcile-mode strip */}
        <If when={Boolean(p.reconcileMode)}>
          <View class="flex-row items-center gap-2 px-3 py-2 bg-cyan-50 dark:bg-cyan-950/50 border-b border-cyan-200 dark:border-cyan-800/50">
            <Text class="text-[11px] font-medium text-cyan-700 dark:text-cyan-300">🔗 Click an unmatched statement line to reconcile</Text>
          </View>
        </If>
        {/* upload zone when no document yet */}
        <If when={empty}>
          <View class="items-center justify-center px-6 py-10">
            <If when={Boolean(p.uploadError)}>
              <View class="mb-3 px-3 py-2 rounded-lg bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800/30">
                <Text class="text-[11px] font-medium text-red-600 dark:text-red-400">{p.uploadError}</Text>
              </View>
            </If>
            <If when={p.uploading}>
              <View class="w-16 h-16 rounded-2xl bg-emerald-50 dark:bg-emerald-900/20 items-center justify-center mb-4">
                <Text class="text-[20px]">…</Text>
              </View>
              <Text class="text-[13px] font-medium text-emerald-600 dark:text-emerald-400 mb-1">Processing statement…</Text>
              <Text class="text-[11px] text-gray-400 dark:text-white/30">AI is classifying, extracting, and matching transactions</Text>
            </If>
            <If when={!p.uploading}>
              <View class="w-16 h-16 rounded-2xl bg-gray-100 dark:bg-white/5 items-center justify-center mb-4">
                <Text class="text-[24px] text-gray-300 dark:text-white/20">⇪</Text>
              </View>
              <Text class="text-[13px] font-medium text-gray-500 dark:text-white/40 mb-1">Upload bank statement</Text>
              <Text class="text-[11px] text-gray-400 dark:text-white/30 mb-4 text-center">Pick a PDF or image. AI will extract and match transactions automatically.</Text>
              <View class="px-6 py-2 rounded-lg bg-emerald-500/10 hover:bg-emerald-500/20 transition-colors cursor-pointer" accessibilityLabel="Upload Statement" onPress={() => p.onUpload?.()}>
                <Text class="text-[12px] font-semibold text-emerald-700 dark:text-emerald-400">⇪ Upload Statement</Text>
              </View>
              <If when={Boolean(p.onGenerate)}>
                <View class="items-center mt-4 pt-4 border-t border-gray-200 dark:border-white/10">
                  <Text class="text-[11px] text-gray-500 dark:text-gray-400 mb-2">Missing the original PDF?</Text>
                  <View class="px-6 py-2 rounded-lg bg-blue-500/10 hover:bg-blue-500/20 transition-colors cursor-pointer" accessibilityLabel="Generate Statement PDF" onPress={() => !p.generating && p.onGenerate?.()}>
                    <Text class="text-[12px] font-semibold text-blue-700 dark:text-blue-400">{p.generating ? "Generating…" : "Generate Statement PDF"}</Text>
                  </View>
                </View>
              </If>
            </If>
          </View>
        </If>
        {/* the page image + boxes, scaled by zoom inside the scroll */}
        <If when={!empty && Boolean(note())}>
          <Text class={`text-[12px] px-3 py-2 ${C.dim}`}>{note()}</Text>
        </If>
        <If when={Boolean(img())}>
          <Scroll ref={scRef} class={`max-h-[${SNAP_SCROLL_H}px]`}>
            <View style={{ width: scaleW(), height: scaleH() }} onLayout={runWish}>
              <Avatar src={img()?.src || ""} accessibilityLabel="Statement page" style={{ width: scaleW(), height: scaleH() }} imageStyle={{ width: scaleW(), height: scaleH() }} />
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
                    class={`absolute ${boxCls(r)} cursor-pointer`}
                    style={{ left: r().box.left, top: r().box.top, width: r().box.width, height: r().box.height }}
                    accessibilityLabel={p.reconcileMode && r().match !== "m" ? "Pick this line to reconcile" : "Show this statement line"}
                    onLayout={(e: Any) => {
                      const y = Number(e?.bounds?.origin?.y) || 0;
                      boxTop.set(r().no, y);
                      if (r().no === wish()) runWish();
                    }}
                    onMouseEnter={() => hoverBox(r().no)}
                    onMouseLeave={() => hoverBox(null)}
                    onPress={() => {
                      if (p.reconcileMode && r().match !== "m") p.onReconcileSelect?.(r().no);
                      else p.onSelect(r().no);
                    }}
                  />
                )}
              </Each>
              {/* hover popover: quick match / create transaction */}
              <If when={Boolean(popRow())}>
                <View
                  class="absolute z-20 ml-2 w-[210px] rounded-lg border shadow-xl p-3 gap-2 bg-white dark:bg-[#1a1f2e]"
                  style={{ left: popLeft(popRow()), top: popTop(popRow()), borderColor: p.matchFor?.(popRow()) ? "#67e8f966" : "#fcd34d66" }}
                  onMouseEnter={() => popNo.set(popNo())}
                  onMouseLeave={() => {
                    popNo.set(null);
                    hoverBox(null);
                  }}
                >
                  <View class="flex-row items-center gap-1.5">
                    <View class={`w-5 h-5 rounded-full items-center justify-center ${p.matchFor?.(popRow()) ? "bg-cyan-500/15" : "bg-amber-500/15"}`}>
                      <Text class={`text-[10px] ${p.matchFor?.(popRow()) ? "text-cyan-500" : "text-amber-500"}`}>{p.matchFor?.(popRow()) ? "🔗" : "⚠"}</Text>
                    </View>
                    <Text class={`text-[11px] font-semibold ${p.matchFor?.(popRow()) ? "text-cyan-600 dark:text-cyan-400" : "text-amber-600 dark:text-amber-400"}`}>{p.matchFor?.(popRow()) ? "Match Available" : "Missing Transaction"}</Text>
                  </View>
                  <View class="gap-1">
                    <Text class="text-[11px] text-gray-700 dark:text-white/70 font-medium truncate">{popRow()?.description || popRow()?.title || ""}</Text>
                    <Text class="text-[12px] font-semibold tabular-nums text-gray-900 dark:text-white">{popRow()?.amount_shown || ""}</Text>
                  </View>
                  <If when={Boolean(p.matchFor?.(popRow()) && p.onQuickMatch)}>
                    <View
                      class="w-full items-center justify-center px-2 py-1.5 rounded-md bg-cyan-500/10 hover:bg-cyan-500/20 cursor-pointer"
                      accessibilityLabel="Link Transaction"
                      onPress={() => {
                        p.onQuickMatch?.(popNo()!, String(p.matchFor!(popRow()).id));
                        popNo.set(null);
                      }}
                    >
                      <Text class="text-[11px] font-semibold text-cyan-700 dark:text-cyan-400">Link Transaction</Text>
                    </View>
                  </If>
                  <If when={!p.matchFor?.(popRow()) && Boolean(p.onCreateFromLine)}>
                    <View
                      class="w-full items-center justify-center px-2 py-1.5 rounded-md bg-emerald-500/10 hover:bg-emerald-500/20 cursor-pointer"
                      accessibilityLabel="Create Transaction"
                      onPress={() => {
                        p.onCreateFromLine?.(popNo()!);
                        popNo.set(null);
                      }}
                    >
                      <Text class="text-[11px] font-semibold text-emerald-700 dark:text-emerald-400">+ Create Transaction</Text>
                    </View>
                  </If>
                </View>
              </If>
            </View>
          </Scroll>
        </If>
      </Panel>
      {/* above the page, so they show without scrolling */}
      <View class="flex-row gap-2 flex-wrap">
        <If when={Boolean(buwiz.preview) && p.evidence}>
          <Btn label="See the page" onPress={() => showPreview({ account: p.account, month: p.month, kind: "page", page: page(), line: p.selected ?? undefined })} />
        </If>
        <PrintReconciliation account={p.account} month={p.month} />
      </View>
      <If when={!p.reconcileMode && chosen()?.match === "x" && paged().length > 0}>
        <Btn label="Create from this line" onPress={() => act("books_app_statement_add_line", { account: p.account, month: p.month, line: chosen().ticket }).catch(() => null)} />
      </If>
    </View>
  );
}
