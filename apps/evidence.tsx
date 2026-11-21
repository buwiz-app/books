// Books documents: the document library. Add a PDF or image (pick -> read -> the foundation's multipart
// books_app_evidence_attach), list generic metadata, preview it (static, through buwiz.preview), attach its scope
// ticket to the chat so the assistant can work from it, and accept or discard the assistant's proposed kind.
// Bytes only ever leave Books inside the preview; the list carries kind, type, pages and link counts.

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

function EvRow(p: { item: Any; proposals: Any[] }) {
  const e = p.item;
  const mine = () => p.proposals.filter((x: Any) => x.evidence === e.evidence);
  const pages = e.page_count ? ` · ${String(e.page_count)} ${e.page_count === 1 ? "page" : "pages"}` : "";
  return (
    <View class={`px-4 py-3 md:px-5 md:py-3.5 border-b transition-colors hover:bg-[#f8fafc] dark:hover:bg-white/[0.02] ${C.line}`}>
      <View class="flex-row items-center gap-3">
        <View class={`w-8 h-8 rounded items-center justify-center shrink-0 border bg-gray-50 border-gray-200 dark:bg-white/5 dark:border-white/10`}>
          <Text class="text-[9px] font-bold text-[#94a3b8] dark:text-white/40">{EV_MIME[String(e.mime)] || "FILE"}</Text>
        </View>
        <Text class={`flex-1 text-sm font-medium truncate ${C.ink}`}>{`${EV_KIND[String(e.kind)] || "Document"} · ${EV_MIME[String(e.mime)] || "File"}${pages}`}</Text>
        <Text class={`text-xs ${C.dim}`}>{`${String(e.linked_count || 0)} linked`}</Text>
        <Btn small label="Preview" onPress={() => evPreview(e.evidence)} />
        <Btn small label="Attach to chat" onPress={() => evAttach(e.evidence)} />
      </View>
      {mine().map((r: Any) => (
        <EvProposal row={r} />
      ))}
    </View>
  );
}

export default function Evidence() {
  const list = resource(
    () => ({ v: ver() }),
    () => buwiz.call("books_evidence_list", { limit: 200 }),
  );
  const review = resource(
    () => ({ v: ver() }),
    () => buwiz.call("books_app_evidence_review", {}),
  );
  const items = () => ((list() as Any)?.items || []) as Any[];
  const proposals = () => ((review() as Any)?.rows || []) as Any[];
  return (
    <View style={{ flex: 1 }} class={`${C.bg}`}>
      <Head title="Books documents" sub="Receipts, bills, statements and other papers kept with your books.">
        <View
          accessibilityLabel="Upload document"
          class={`h-11 px-5 rounded-lg bg-linear-to-r from-[#10b981] to-[#059669] flex-row items-center justify-center gap-2 shadow-sm hover:shadow-md transition-all ${evBusy() ? "opacity-60" : ""}`}
          onPress={() => {
            if (!evBusy()) evAdd();
          }}
          activeStyle={{ opacity: 0.85 }}
        >
          <Text class="text-[13px] font-medium text-white">{evBusy() ? "Adding…" : "↑ Upload document"}</Text>
        </View>
      </Head>
      <Scroll style={{ flex: 1 }}>
        <Panel>
          <Loading res={list} />
          <If when={Boolean(list()) && items().length === 0}>
            <View class="items-center justify-center px-4 py-16 gap-1">
              <View class="w-20 h-20 rounded-2xl bg-[#10b981]/10 items-center justify-center mb-4">
                <Text class="text-[30px] text-[#10b981]">▤</Text>
              </View>
              <Text class={`text-lg font-semibold ${C.ink}`}>No documents yet</Text>
              <Text class={`text-sm text-center text-[#94a3b8] dark:text-white/40`} style={{ maxWidth: 420 }}>
                Upload statements, invoices, receipts and other financial documents to keep everything organized. Then attach one to a chat so the assistant can read it.
              </Text>
              <View
                accessibilityLabel="Upload document"
                class="h-11 px-5 rounded-lg bg-linear-to-r from-[#10b981] to-[#059669] flex-row items-center justify-center gap-2 shadow-sm hover:shadow-md transition-all mt-4"
                onPress={() => {
                  if (!evBusy()) evAdd();
                }}
                activeStyle={{ opacity: 0.85 }}
              >
                <Text class="text-[13px] font-medium text-white">↑ Upload document</Text>
              </View>
            </View>
          </If>
          {items().map((e: Any) => (
            <EvRow item={e} proposals={proposals()} />
          ))}
        </Panel>
        <Text class={`px-4 py-3 text-[12px] ${C.dim}`}>A document attached in the chat alone does not reach Books: add it here first. Accepting a kind only labels the document; the page it opens is where you record anything.</Text>
      </Scroll>
    </View>
  );
}
