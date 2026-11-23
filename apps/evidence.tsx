// Books documents: the document library, laid out like the old app's /documents (src/routes/documents.tsx):
// a filter box, a grid/list toggle, a Filters popover, grid cards and a list table. Add a PDF or image
// (pick -> read -> the foundation's multipart books_app_evidence_attach), preview it (static, through
// buwiz.preview), attach its scope ticket to the chat so the assistant can work from it, and accept or
// discard the assistant's proposed kind. Bytes only ever leave Books inside the preview; the list carries
// kind, type, pages and link counts.

const EV_KIND: Record<string, string> = {
  statement: "Bank statement",
  bill: "Bill",
  invoice: "Invoice",
  receipt: "Receipt",
  payslip: "Payslip",
  contract: "Contract",
  tax_form: "Tax form",
  other: "Other document",
};
const EV_MIME: Record<string, string> = { "application/pdf": "PDF", "image/png": "PNG", "image/jpeg": "JPEG", "image/webp": "WebP" };
const EV_EXT: Record<string, string> = { pdf: "application/pdf", png: "image/png", jpg: "image/jpeg", jpeg: "image/jpeg", webp: "image/webp" };
const EV_ROUTE: Record<string, [string, string]> = {
  ledger_draft: ["ledger", "Open the Ledger"],
  unpaid_review: ["unpaid", "Open Unpaid"],
  statement_import: ["reconcile", "Import the statement"],
  certificate_review: ["certificates", "Open Certificates"],
};

function evB64(bytes: Any): string {
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

const evBusy = signal(false);

async function evAdd() {
  if (!buwiz.pickFile) return toast("This Buwiz Bot cannot pick files.");
  const files = (await buwiz.pickFile({ extensions: [".pdf", ".png", ".jpg", ".jpeg", ".webp"] }).catch(() => [])) as Any[];
  const file = (files || [])[0];
  if (!file) return;
  const ext = String(file.name || "").split(".").pop()?.toLowerCase() || "";
  const mime = EV_EXT[ext] || String(file.type || "");
  if (!EV_MIME[mime]) return toast("Books keeps PDF, PNG, JPEG and WebP documents.");
  evBusy.set(true);
  try {
    const bytes = await file.read();
    const size = 96000;
    const parts = Math.max(1, Math.ceil(bytes.length / size));
    const upload = `ev-${rid().slice(3)}`;
    for (let i = 0; i < parts; i += 1) {
      await buwiz.call("books_app_evidence_attach", { client_request_id: rid(), op: "part", upload, part: i, parts, data_base64: evB64(bytes.slice(i * size, (i + 1) * size)) });
    }
    await act("books_app_evidence_attach", { op: "commit", upload, mime, kind: "other", source: "user" });
    toast("Added to Books documents.");
  } catch (e) {
    toast(errText(e));
  } finally {
    evBusy.set(false);
  }
}

function evPreview(evidence: string) {
  if (!buwiz.preview) return toast("Previews need a newer Buwiz Bot.");
  buwiz.preview({ tool: "books_app_evidence_preview", args: { evidence }, title: "Evidence" }).catch((e: Any) => toast(errText(e)));
}

function evAttach(evidence: string) {
  if (!buwiz.attach) return toast("This Buwiz Bot cannot attach a document to a chat.");
  buwiz.attach("evidence", evidence)
    .then(() => toast("Attached to the chat · ask the assistant what it is"))
    .catch((e: Any) => toast(errText(e)));
}

function EvProposal(p: { row: Any }) {
  const r = p.row;
  const route = EV_ROUTE[String(r.route || "")];
  return (
    <View class={`flex-row items-center gap-2 py-1`}>
      <Text class={`text-[12px] ${C.mute}`}>{`Assistant: ${EV_KIND[String(r.kind)] || "Document"} · ${String(r.confidence)} of 100`}</Text>
      <If when={r.status === "draft"}>
        <Btn small tone="go" label="Accept" onPress={() => act("books_app_evidence_kind", { proposal: r.proposal, decision: "accept" }).catch(() => null)} />
        <Btn small label="Discard" onPress={() => act("books_app_evidence_kind", { proposal: r.proposal, decision: "discard" }).catch(() => null)} />
      </If>
      <If when={r.status !== "draft"}>
        <Pill tone="mute" label={r.status === "accepted" ? "Accepted" : "Discarded"} />
      </If>
      <If when={r.status === "accepted" && Boolean(route)}>
        <Btn small label={route ? route[1] : ""} onPress={() => buwiz.open({ app: "books", props: { page: route[0] } }).catch(() => null)} />
      </If>
    </View>
  );
}

/* A document's one-line name: Books keeps no filename, so the kind label and the file type stand in. */
function evName(e: Any): string {
  const pages = e.page_count ? ` · ${String(e.page_count)} ${e.page_count === 1 ? "page" : "pages"}` : "";
  return `${EV_KIND[String(e.kind)] || "Document"} · ${EV_MIME[String(e.mime)] || "File"}${pages}`;
}

/* The file-type tile the old app draws from an SVG: a red tile for PDF, teal for an image (text glyphs only). */
function EvTypeTile(p: { mime: string; big?: boolean }) {
  const isPdf = p.mime === "application/pdf";
  const label = EV_MIME[p.mime] || "FILE";
  const size = p.big ? "w-12 h-12 text-[13px]" : "w-8 h-8 text-[9px]";
  const tone = isPdf ? "bg-red-50 text-red-600 dark:bg-red-500/10 dark:text-red-400" : "bg-teal-50 text-teal-600 dark:bg-teal-500/10 dark:text-teal-400";
  return (
    <View class={`${size} rounded-md items-center justify-center shrink-0 ${tone}`}>
      <Text class={`${p.big ? "text-[13px]" : "text-[9px]"} font-bold ${isPdf ? "text-red-600 dark:text-red-400" : "text-teal-600 dark:text-teal-400"}`}>{label}</Text>
    </View>
  );
}

function EvUploadBtn() {
  /* The label collapses to the icon below `sm` — a two-word button cannot share 375px
     with the title without wrapping (documents.tsx:398-423). */
  return (
    <View
      accessibilityLabel="Upload Document"
      class={`items-center justify-center flex-row gap-2 h-11 w-11 sm:w-auto sm:px-5 rounded-lg bg-linear-to-r from-[#10b981] to-[#059669] shadow-sm hover:shadow-md transition-all ${evBusy() ? "opacity-60" : ""}`}
      onPress={() => {
        if (!evBusy()) evAdd();
      }}
      activeStyle={{ opacity: 0.85 }}
    >
      <Text class="text-[15px] font-semibold text-white">↑</Text>
      <Text class="hidden sm:inline text-sm font-medium text-white">{evBusy() ? "Adding…" : "Upload Document"}</Text>
    </View>
  );
}

export default function Evidence() {
  const list = resource(
    () => ({ v: ver() }),
    // every page: the filters and the count cover the whole library, not
    // only the first 200 documents
    async () => {
      const all: Any[] = [];
      let cursor = "";
      for (;;) {
        const page: Any = await buwiz.call("books_evidence_list", { limit: 200, ...(cursor ? { cursor } : {}) });
        all.push(...((page?.items as Any[]) || []));
        cursor = String(page?.next_cursor || "");
        if (!cursor) break;
      }
      return { items: all };
    },
  );
  const review = resource(
    () => ({ v: ver() }),
    () => buwiz.call("books_app_evidence_review", {}),
  );
  const items = () => ((list() as Any)?.items || []) as Any[];
  const proposals = () => ((review() as Any)?.rows || []) as Any[];
  const mine = (e: Any) => proposals().filter((x: Any) => x.evidence === e.evidence);

  const evView = signal("grid");
  const evQuery = signal("");
  const evFilterOpen = signal(false);
  const evKind = signal("");
  const evFile = signal("");
  const evLinked = signal("");
  const evActionsFor = signal("");
  const evSheetOpen = signal(false);
  const evSelKind = signal(false);
  const evSelFile = signal(false);
  const evSelLinked = signal(false);
  let evFilterAnchor: ViewRef | undefined;
  const evActionAnchors: Record<string, ViewRef> = {};

  const evActiveCount = () => (evKind() ? 1 : 0) + (evFile() ? 1 : 0) + (evLinked() ? 1 : 0);
  const evFiltered = () => {
    const q = evQuery().trim().toLowerCase();
    return items().filter((e: Any) => {
      if (evKind() && String(e.kind) !== evKind()) return false;
      if (evFile() === "pdf" && e.mime !== "application/pdf") return false;
      if (evFile() === "image" && e.mime === "application/pdf") return false;
      if (evLinked() === "linked" && !(e.linked_count > 0)) return false;
      if (evLinked() === "unlinked" && e.linked_count > 0) return false;
      if (q && !evName(e).toLowerCase().includes(q)) return false;
      return true;
    });
  };

  const evFileTypes: [string, string][] = [["", "All Files"], ["pdf", "PDF"], ["image", "Image"]];
  const evLinkTypes: [string, string][] = [["", "All"], ["linked", "Linked"], ["unlinked", "Unlinked"]];
  const evKinds: [string, string][] = [["", "All Types"]].concat(Object.keys(EV_KIND).map((k) => [k, EV_KIND[k]] as [string, string]));

  /* The old app's <select>: a collapsed h-11 bordered row (⌄) that opens its option list in place
     (documents.tsx:485-510). */
  const evSelect = (label: string, options: [string, string][], value: () => string, open: () => boolean, onOpen: (v: boolean) => void, onPick: (v: string) => void) => {
    const current = () => ((options.find((o) => o[0] === value()) || options[0]) as [string, string])[1];
    return (
      <View class="flex-col gap-1.5 mb-3">
        <Text class="text-[11px] md:text-xs font-semibold uppercase tracking-wider text-[#94a3b8] dark:text-white/40">{label}</Text>
        <View class="flex-col rounded-lg border border-[#e2e8f0] dark:border-white/10 bg-white dark:bg-[#0f172a]">
          <View
            class="h-11 md:h-auto md:py-2 flex-row items-center px-3 transition-colors"
            activeStyle={{ opacity: 0.7 }}
            onPress={() => onOpen(!open())}
          >
            <Text class="flex-1 text-base md:text-sm text-[#1e293b] dark:text-white truncate">{current()}</Text>
            <Text class="text-[13px] text-[#94a3b8] dark:text-white/30">⌄</Text>
          </View>
          <If when={open()}>
            <View class="border-t border-[#e2e8f0] dark:border-white/10">
              <Each of={options}>
                {(o: Any) => (
                  <View
                    class={`flex-row items-center px-3 min-h-11 md:min-h-0 md:py-2 transition-colors ${value() === o()[0] ? "bg-[#10b981]/10" : "hover:bg-[#f8fafc] dark:hover:bg-white/5"}`}
                    activeStyle={{ opacity: 0.7 }}
                    onPress={() => {
                      onPick(o()[0]);
                      onOpen(false);
                    }}
                  >
                    <Text class={`flex-1 text-sm ${value() === o()[0] ? "font-medium text-[#1e293b] dark:text-white" : "text-[#64748b] dark:text-white/50"}`}>{o()[1]}</Text>
                    <If when={value() === o()[0]}>
                      <Text class="text-sm text-[#10b981]">✓</Text>
                    </If>
                  </View>
                )}
              </Each>
            </View>
          </If>
        </View>
      </View>
    );
  };

  const evFilterFields = () => (
    <>
      {evSelect("Document Type", evKinds, evKind, evSelKind, (v) => evSelKind.set(v), (v) => evKind.set(v))}
      {evSelect("File Type", evFileTypes, evFile, evSelFile, (v) => evSelFile.set(v), (v) => evFile.set(v))}
      {evSelect("Linked", evLinkTypes, evLinked, evSelLinked, (v) => evSelLinked.set(v), (v) => evLinked.set(v))}
      <If when={evActiveCount() > 0}>
        <View
          class="items-center py-1.5"
          activeStyle={{ opacity: 0.7 }}
          onPress={() => {
            evKind.set("");
            evFile.set("");
            evLinked.set("");
          }}
        >
          <Text class="text-xs text-[#10b981] font-medium hover:underline">Clear Filters</Text>
        </View>
      </If>
    </>
  );

  const evToggleBtn = (mode: string, glyph: string, label: string) => (
    <View
      accessibilityLabel={label}
      class={`h-11 w-11 items-center justify-center transition-colors ${evView() === mode ? "bg-[#10b981]/10" : ""}`}
      activeStyle={{ opacity: 0.7 }}
      onPress={() => evView.set(mode)}
    >
      <Text class={`text-[15px] transition-colors ${evView() === mode ? "text-[#10b981]" : "text-[#94a3b8] dark:text-white/30 hover:text-[#64748b] dark:hover:text-white/50"}`}>{glyph}</Text>
    </View>
  );

  return (
    <View style={{ flex: 1 }} class={`bg-[#fafbfc] dark:bg-[#0c1322]`}>
      {/* ‹ Back — Books' way home. */}
      <View
        class="group flex-row items-center gap-1.5 h-11 min-w-11 justify-center rounded-lg transition-colors sm:min-w-0 sm:justify-start"
        onPress={() => buwiz.open({ app: "books", props: { page: "today" } }).catch(() => null)}
        activeStyle={{ opacity: 0.7 }}
      >
        <Text class={`text-[14px] font-medium transition-colors ${C.ink} group-hover:text-[#0f766e] dark:group-hover:text-[#2dd4bf]`}>‹ Back</Text>
      </View>

      {/* Header: folder icon + title + teal Upload Document button (documents.tsx:379-424). */}
      <View class={`flex-row items-center justify-between gap-3 px-4 py-4 md:px-8 md:py-5 border-b border-[#e2e8f0] dark:border-white/10`}>
        <View class="flex-row min-w-0 items-center gap-3">
          <Text class="text-[22px] text-[#10b981]">▤</Text>
          <Text class="truncate text-lg sm:text-xl font-semibold text-[#1e293b] dark:text-white">Documents</Text>
        </View>
        <EvUploadBtn />
      </View>

      {/* Toolbar: the filter box, the grid/list toggle and the Filters button (documents.tsx:426-597).
          Below md the old app stacks the search over a Filters button + toggle row (FilterBar). */}
      <View class={`flex-col gap-2 px-4 py-3 md:px-8 md:flex-row md:items-center md:gap-3 border-b border-[#e2e8f0] dark:border-white/10`}>
        <View class="relative min-w-0 w-full md:flex-1 flex-row items-center">
          <Text class="absolute left-3 text-[15px] text-[#94a3b8] dark:text-white/30">⌕</Text>
          <Input
            accessibilityLabel="Filter documents"
            value={evQuery()}
            placeholder={`Filter ${evFiltered().length} document${evFiltered().length !== 1 ? "s" : ""}…`}
            onChange={(e: Any) => evQuery.set(typeof e === "string" ? e : e.value)}
            class={`flex-1 h-11 pl-10 pr-4 rounded-lg border border-[#e2e8f0] dark:border-white/10 bg-white dark:bg-[#111827] text-base md:text-sm text-[#1e293b] dark:text-white placeholder:text-[#94a3b8] dark:placeholder:text-white/30 focus:outline-none focus:ring-2 focus:ring-[#10b981]/20 focus:border-[#10b981] transition-all`}
          />
        </View>
        <View class="flex-row items-center gap-2 md:gap-3">
          <View class={`flex-row shrink-0 items-center rounded-lg border border-[#e2e8f0] dark:border-white/10 overflow-hidden`}>
            {evToggleBtn("grid", "▦", "Grid view")}
            {evToggleBtn("list", "☰", "List view")}
          </View>
          {/* Desktop: the bordered funnel icon with its active badge (documents.tsx:443-471). */}
          <View
            ref={(r: Any) => { evFilterAnchor = r; }}
            accessibilityLabel="Filter"
            class={`hidden md:flex relative h-11 w-11 items-center justify-center rounded-lg border transition-colors ${
              evActiveCount() > 0
                ? "border-[#10b981]/40 bg-[#10b981]/5"
                : "border-[#e2e8f0] dark:border-white/10 hover:border-[#cbd5e1] dark:hover:border-white/20"
            }`}
            activeStyle={{ opacity: 0.7 }}
            onPress={() => {
              evFilterOpen.set(!evFilterOpen());
              evSheetOpen.set(false);
            }}
          >
            <Text class={`text-[15px] transition-colors ${evActiveCount() > 0 ? "text-[#10b981]" : "text-[#94a3b8] dark:text-white/30 hover:text-[#64748b] dark:hover:text-white/50"}`}>▽</Text>
            <If when={evActiveCount() > 0}>
              <View class="absolute -top-1 -right-1 w-4 h-4 rounded-full bg-[#10b981] items-center justify-center">
                <Text class="text-[9px] font-bold text-white">{`${evActiveCount()}`}</Text>
              </View>
            </If>
          </View>
          {/* Mobile: FilterBar's secondary "Filters (N)" that opens the bottom sheet
              (Actions.tsx:125-148). */}
          <View
            accessibilityLabel="Filters"
            class="md:hidden flex-row items-center gap-2 h-11 px-4 rounded-lg border border-[#e2e8f0] dark:border-white/10 bg-white dark:bg-[#111827]"
            activeStyle={{ opacity: 0.7 }}
            onPress={() => {
              evSheetOpen.set(true);
              evFilterOpen.set(false);
            }}
          >
            <Text class="text-[14px] text-[#64748b] dark:text-white/50 transition-colors hover:text-[#475569] dark:hover:text-white/70">☰</Text>
            <Text class="text-sm font-medium text-[#1e293b] dark:text-white transition-colors hover:text-[#475569] dark:hover:text-white/70">{`Filters${evActiveCount() > 0 ? ` (${evActiveCount()})` : ""}`}</Text>
          </View>
        </View>
      </View>

      {/* Content. */}
      <Scroll style={{ flex: 1 }} class="px-4 py-4 md:px-8 md:py-6">
        <Loading res={list} />
        {/* the first-upload prompt only when the library is empty; a query or
            filter that matches nothing says so instead */}
        <If when={Boolean(list()) && items().length === 0}>
          <View class="items-center justify-center px-4 py-16 md:py-24 gap-1">
            <View class="w-20 h-20 rounded-2xl bg-[#10b981]/10 items-center justify-center mb-4">
              <Text class="text-[30px] text-[#10b981]">▤</Text>
            </View>
            <Text class="text-lg font-semibold text-[#1e293b] dark:text-white">No documents yet</Text>
            <Text class="text-sm text-center text-[#94a3b8] dark:text-white/40 mb-4" style={{ maxWidth: 420 }}>
              Upload statements, invoices, receipts, and other financial documents to keep everything organized.
            </Text>
            <EvUploadBtn />
          </View>
        </If>
        <If when={Boolean(list()) && items().length > 0 && evFiltered().length === 0}>
          <View class="items-center justify-center px-4 py-16 md:py-24 gap-1">
            <Text class="text-lg font-semibold text-[#1e293b] dark:text-white">No documents match</Text>
            <Text class="text-sm text-center text-[#94a3b8] dark:text-white/40" style={{ maxWidth: 420 }}>
              Try different words, or change the filters.
            </Text>
          </View>
        </If>

        {/* Grid view: the old app's card (documents.tsx:688-730) — nsh has no grid, so the
            2/3/4/5-column layout is flex-wrap with percentage widths and the same gaps. */}
        <If when={Boolean(list()) && evFiltered().length > 0 && evView() === "grid"}>
          <View class="flex-row flex-wrap -m-1.5 md:-m-2.5">
            <Each of={evFiltered()} key={(e: Any) => String(e.evidence)}>
              {(e: Any) => (
                <View class="w-1/2 md:w-1/3 lg:w-1/4 xl:w-1/5 p-1.5 md:p-2.5">
                  <View class={`rounded-xl border border-[#e2e8f0] dark:border-white/10 bg-white dark:bg-[#111827] overflow-hidden hover:shadow-lg hover:border-[#a7f3d0] dark:hover:border-[#10b981]/30 transition-all cursor-pointer`}>
                    <View class="w-full h-[120px] bg-[#f1f5f9] dark:bg-[#0f172a] items-center justify-center overflow-hidden">
                      <View class="items-center gap-2">
                        <EvTypeTile mime={String(e().mime)} big />
                        <Text class="text-xs text-[#94a3b8] dark:text-white/30">{e().mime === "application/pdf" ? "PDF Document" : "Image"}</Text>
                      </View>
                    </View>
                    <View class="flex-row items-start gap-2 px-3 py-3">
                      <EvTypeTile mime={String(e().mime)} />
                      <View class="flex-1 min-w-0">
                        <Text class="text-sm font-medium text-[#1e293b] dark:text-white truncate leading-tight">{evName(e())}</Text>
                        <Text class="text-xs text-[#94a3b8] dark:text-white/40 mt-0.5">{`${String(e().linked_count || 0)} linked`}</Text>
                      </View>
                    </View>
                    <View class="flex-row items-center gap-2 px-3 pb-3">
                      <Btn small label="Preview" onPress={() => evPreview(e().evidence)} />
                      <Btn small label="Attach to chat" onPress={() => evAttach(e().evidence)} />
                    </View>
                    {mine(e()).map((r: Any) => (
                      <View class="px-3 pb-2">
                        <EvProposal row={r} />
                      </View>
                    ))}
                  </View>
                </View>
              )}
            </Each>
          </View>
        </If>

        {/* List view: the old app's table (documents.tsx:624-890). "Uploaded On" is dropped —
            books_evidence_list carries no date; "Uploaded By" is the source (user/assistant). */}
        <If when={Boolean(list()) && evFiltered().length > 0 && evView() === "list"}>
          <View class={`rounded-xl border border-[#e2e8f0] dark:border-white/10 overflow-hidden bg-white dark:bg-[#111827]`}>
            <View class={`hidden md:flex flex-row items-center px-5 py-3 border-b border-[#e2e8f0] dark:border-white/10 bg-[#f8fafc] dark:bg-[#0f172a]`}>
              <Text class="flex-1 text-[11px] font-semibold text-[#94a3b8] dark:text-white/40 uppercase tracking-wider">Document Name</Text>
              <Text class="w-[140px] text-[11px] font-semibold text-[#94a3b8] dark:text-white/40 uppercase tracking-wider">Document Type</Text>
              <Text class="w-[140px] text-[11px] font-semibold text-[#94a3b8] dark:text-white/40 uppercase tracking-wider">Uploaded By</Text>
              <Text class="w-[120px] text-[11px] font-semibold text-[#94a3b8] dark:text-white/40 uppercase tracking-wider">Linked</Text>
              <Text class="w-[80px]" />
            </View>
            <Each of={evFiltered()} key={(e: Any) => String(e.evidence)}>
              {(e: Any) => (
                <View>
                  <View class={`flex-row items-center gap-3 px-4 py-3 md:px-5 md:py-3.5 border-b border-[#e2e8f0] dark:border-white/10 hover:bg-[#f8fafc] dark:hover:bg-white/[0.02] transition-colors cursor-pointer`}>
                    <View class="flex-row min-w-0 flex-1 items-center gap-3">
                      <View class="h-10 w-10 shrink-0 items-center justify-center">
                        <EvTypeTile mime={String(e().mime)} />
                      </View>
                      <View class="min-w-0 flex-col">
                        <Text class="truncate text-sm font-medium text-[#1e293b] dark:text-white">{evName(e())}</Text>
                        <Text class="mt-0.5 truncate text-xs text-[#64748b] dark:text-white/50 md:hidden">{`${e().source === "assistant" ? "Assistant" : "User"} · ${String(e().linked_count || 0)} linked`}</Text>
                        <View class="mt-1.5 md:hidden">
                          <Text class="inline-flex px-2 py-0.5 rounded text-xs font-medium bg-gray-100 text-gray-800 dark:bg-white/10 dark:text-gray-300">{EV_KIND[String(e().kind)] || "Document"}</Text>
                        </View>
                      </View>
                    </View>
                    <View class="hidden md:flex w-[140px]">
                      <Text class="px-2 py-0.5 rounded text-xs font-medium bg-gray-100 text-gray-800 dark:bg-white/10 dark:text-gray-300">{EV_KIND[String(e().kind)] || "Document"}</Text>
                    </View>
                    <View class="hidden md:flex flex-row w-[140px] items-center gap-2">
                      <View class="w-6 h-6 rounded-full bg-[#10b981]/20 items-center justify-center">
                        <Text class="text-[10px] font-bold text-[#10b981]">{e().source === "assistant" ? "A" : "U"}</Text>
                      </View>
                      <Text class="text-sm text-[#64748b] dark:text-white/50 truncate">{e().source === "assistant" ? "Assistant" : "User"}</Text>
                    </View>
                    <Text class="hidden md:block w-[120px] text-sm text-[#64748b] dark:text-white/50">{`${String(e().linked_count || 0)} linked`}</Text>
                    <View
                      ref={(r: Any) => { evActionAnchors[String(e().evidence)] = r; }}
                      accessibilityLabel="Document actions"
                      class="w-[80px] flex-row shrink-0 items-center justify-end"
                      activeStyle={{ opacity: 0.7 }}
                      onPress={() => evActionsFor.set(String(e().evidence))}
                    >
                      <Text class="h-8 w-8 text-center text-[18px] text-[#94a3b8] dark:text-white/30 transition-colors hover:text-[#64748b] dark:hover:text-white/50">⋯</Text>
                    </View>
                  </View>
                  {mine(e()).map((r: Any) => (
                    <View class={`px-4 pb-2 border-b border-[#e2e8f0] dark:border-white/10`}>
                      <EvProposal row={r} />
                    </View>
                  ))}
                </View>
              )}
            </Each>
          </View>
        </If>
        <Text class={`px-4 py-3 text-[12px] ${C.dim}`}>A document attached in the chat alone does not reach Books: add it here first. Accepting a kind only labels the document; the page it opens is where you record anything.</Text>
      </Scroll>

      {/* Filters popover (documents.tsx:473-523): only filters the evidence data answers —
          kind, file type and linked. The old app's document-type list maps to Books' kinds. */}
      <If when={evFilterOpen()}>
        <Overlay
          anchor={evFilterAnchor}
          placement="bottom"
          align="end"
          offset={2}
          margin={8}
          onPressOutside={() => evFilterOpen.set(false)}
          onCancel={() => evFilterOpen.set(false)}
        >
          <View class={`hidden md:flex md:flex-col w-64 rounded-xl border border-[#e2e8f0] dark:border-white/10 bg-white dark:bg-[#1e293b] shadow-xl py-3 px-4`}>
            {evFilterFields()}
          </View>
        </Overlay>
      </If>

      {/* Mobile Filters sheet (FilterBar's Modal mobile="sheet", Actions.tsx:150-163): a dim backdrop
          and a bottom panel with the same selects and a "Show results" footer. */}
      <If when={evSheetOpen()}>
        <View class="fixed inset-0 md:hidden" onPress={() => evSheetOpen.set(false)}>
          <View class="absolute inset-0 bg-black/40" />
          <View
            class={`absolute bottom-0 left-0 right-0 rounded-t-2xl bg-white dark:bg-[#1e293b] pt-4 pb-4`}
            onPress={() => null}
          >
            <View class="flex-row items-center justify-between px-4 mb-3">
              <Text class="text-base font-semibold text-[#1e293b] dark:text-white">Filters</Text>
            </View>
            <Scroll style={{ maxHeight: 420 }}>
              <View class="px-4">
                {evFilterFields()}
              </View>
            </Scroll>
            <View class="px-4 pt-2">
              <View
                class="h-11 w-full rounded-lg bg-[#10b981] items-center justify-center"
                activeStyle={{ opacity: 0.85 }}
                onPress={() => evSheetOpen.set(false)}
              >
                <Text class="text-sm font-medium text-white">Show results</Text>
              </View>
            </View>
          </View>
        </View>
      </If>

      {/* Row actions overflow menu (the old app's RowActionsMenu): Books' own actions. */}
      <If when={Boolean(evActionsFor())}>
        <Overlay
          anchor={evActionAnchors[evActionsFor()]}
          placement="bottom"
          align="end"
          offset={2}
          margin={8}
          onPressOutside={() => evActionsFor.set("")}
          onCancel={() => evActionsFor.set("")}
        >
          <View class={`w-44 rounded-xl border border-[#e2e8f0] dark:border-white/10 bg-white dark:bg-[#1e293b] shadow-xl py-1`}>
            <View class="px-3 py-2 hover:bg-[#f8fafc] dark:hover:bg-white/5 transition-colors" activeStyle={{ opacity: 0.7 }} onPress={() => { const id = evActionsFor(); evActionsFor.set(""); evPreview(id); }}>
              <Text class="text-sm text-[#64748b] dark:text-white/50">Preview</Text>
            </View>
            <View class="px-3 py-2 hover:bg-[#f8fafc] dark:hover:bg-white/5 transition-colors" activeStyle={{ opacity: 0.7 }} onPress={() => { const id = evActionsFor(); evActionsFor.set(""); evAttach(id); }}>
              <Text class="text-sm text-[#64748b] dark:text-white/50">Attach to chat</Text>
            </View>
          </View>
        </Overlay>
      </If>
    </View>
  );
}
