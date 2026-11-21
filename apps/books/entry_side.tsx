// The new-entry page's right column: the tab rail (Attachments, Assistant,
// Activity) and the three panels, a port of the old web app's sidebar on
// /transactions/new (AttachmentsPanel, AIChatPanel, Activity Log). Uploads go
// through the host file picker and books_app_evidence_attach, and a prompt is
// copied to the Bot chat — the assistant only drafts, nothing posts itself.

/** A document this page staged for the entry being drafted. */
const entryStaged = signal<Any[]>([]);
const entryStagedBusy = signal(false);
const entrySide = signal("attachments");
const sideComposer = signal("");
const sidePromptError = signal("");

const SIDE_KIND: Record<string, string> = {
  statement: "Bank statement",
  bill: "Bill",
  invoice: "Invoice",
  receipt: "Receipt",
  payslip: "Payslip",
  contract: "Contract",
  tax_form: "Tax form",
  other: "Other document",
};
const SIDE_MIME: Record<string, string> = { "application/pdf": "PDF", "image/png": "PNG", "image/jpeg": "JPEG", "image/webp": "WebP" };
const SIDE_EXT: Record<string, string> = { pdf: "application/pdf", png: "image/png", jpg: "image/jpeg", jpeg: "image/jpeg", webp: "image/webp" };

function sideB64(bytes: Any): string {
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

function sideBytesShown(size: Any): string {
  const n = Number(size || 0);
  if (n >= 1048576) return `${Math.round(n / 104857.6) / 10} MB`;
  if (n >= 1024) return `${Math.round(n / 1024)} KB`;
  return `${n} B`;
}

/** Pick a PDF or image and add it to Books documents, then stage it for this entry. */
async function sideUpload() {
  if (!buwiz.pickFile) return toast("This Buwiz Bot cannot pick files.");
  const files = (await buwiz.pickFile({ extensions: [".pdf", ".png", ".jpg", ".jpeg"] }).catch(() => [])) as Any[];
  const file = (files || [])[0];
  if (!file) return;
  const ext = String(file.name || "").split(".").pop()?.toLowerCase() || "";
  const mime = SIDE_EXT[ext] || String(file.type || "");
  if (!SIDE_MIME[mime]) return toast("Books keeps PDF, PNG, JPEG and WebP documents.");
  entryStagedBusy.set(true);
  try {
    const bytes = await file.read();
    const size = 96000;
    const parts = Math.max(1, Math.ceil(bytes.length / size));
    const upload = `ev-${rid().slice(3)}`;
    for (let i = 0; i < parts; i += 1) {
      await buwiz.call("books_app_evidence_attach", { client_request_id: rid(), op: "part", upload, part: i, parts, data_base64: sideB64(bytes.slice(i * size, (i + 1) * size)) });
    }
    const r = await act("books_app_evidence_attach", { op: "commit", upload, mime, kind: "other", source: "user" });
    entryStaged.set([...entryStaged(), { evidence: r?.evidence || "", filename: String(file.name || "document"), mime, size: bytes.length }]);
    toast("Added to Books documents.");
  } catch (e) {
    toast(errText(e));
  } finally {
    entryStagedBusy.set(false);
  }
}

/** The old app's attachments panel: drop zone, staged documents, library browse. */
function SideAttachments() {
  return (
    <View class="p-3 gap-2.5">
      <Text class={`text-[13px] font-semibold ${C.ink}`}>Attachments</Text>
      <View
        class="flex-col items-center justify-center gap-2 px-4 py-6 rounded-xl border-2 border-dashed border-[#cbd5e1] cursor-pointer transition-all dark:border-slate-600 hover:border-[#2a9d8f] hover:bg-[#f8fafb] dark:hover:bg-slate-800/50"
        accessibilityLabel="Browse or drag and drop an attachment"
        onPress={() => sideUpload()}
        activeStyle={{ opacity: 0.75 }}
      >
        <View class="w-10 h-10 rounded-full flex items-center justify-center bg-[#f1f5f9] text-[#94a3b8] dark:bg-slate-800 dark:text-slate-500">
          <Text class="text-[20px]">⇧</Text>
        </View>
        <View class="items-center">
          <Text class="text-[12px] font-medium text-[#2a9d8f]">{entryStagedBusy() ? "Uploading…" : "Browse"}<Text class="font-normal text-[#94a3b8] dark:text-slate-500"> or Drag & Drop</Text></Text>
          <Text class="text-[9px] text-[#94a3b8] dark:text-slate-500">Images or PDF — max 20MB</Text>
        </View>
      </View>
      <If when={entryStaged().length > 0}>
        <Each of={entryStaged()} key={rowKey}>
          {(d: Any, i: Any) => (
            <View class={`flex-row items-center gap-2 rounded-xl border px-2.5 py-2 ${C.line}`}>
              <View class="w-8 h-8 rounded-lg items-center justify-center bg-[#f1f5f9] dark:bg-slate-800">
                <Text class="text-[14px] text-[#94a3b8]">{String(d().mime).startsWith("image/") ? "▦" : "≣"}</Text>
              </View>
              <View class="flex-1 gap-0.5">
                <Text class={`text-[12px] font-medium ${C.ink}`} numberOfLines={1}>{d().filename}</Text>
                <Text class={`text-[10px] ${C.dim}`}>{`${SIDE_MIME[String(d().mime)] || "File"} · ${sideBytesShown(d().size)}`}</Text>
              </View>
              <Btn label="×" small accessibilityLabel="Remove attachment" onPress={() => entryStaged.set(entryStaged().filter((_: Any, j: number) => j !== i()))} />
            </View>
          )}
        </Each>
      </If>
      <Btn label="Browse existing documents" onPress={() => buwiz.open({ app: "evidence" }).catch(() => null)} />
      <If when={entryStaged().length === 0}>
        <View class="flex-col items-center justify-center gap-1.5 py-8">
          <Text class="text-[20px] text-[#94a3b8]">▤</Text>
          <Text class="text-[11px] font-medium text-[#94a3b8] dark:text-slate-500">No attachments yet</Text>
          <Text class="text-[9px] text-[#94a3b8] dark:text-slate-500">Drop receipts, invoices, or documents</Text>
        </View>
      </If>
      <Text class={`text-[11.5px] ${C.dim}`}>A press adds the document to Books documents, and it links to the entry when you save. Previewing and attaching an existing document lives in Books documents.</Text>
    </View>
  );
}

/** What to ask the assistant in the chat: it saves a draft, which lands in the Inbox. */
const SIDE_ASK: [string, string][] = [
  ["💸 Pay expense", "Draft a money-out entry in Books: I paid [who] [amount] for [what] from [bank or cash]."],
  ["💰 Receive payment", "Draft a money-in entry in Books: [who] paid me [amount] for [what] into [bank or cash]."],
  ["🔄 Transfer", "Draft a transfer in Books: moved [amount] from [account] to [account]."],
  ["📝 Journal", "Draft a journal entry in Books: debit [account] [amount], credit [account] [amount], for [what]."],
  ["🏠 Rent", "Draft a money-out entry in Books: paid this month's rent of [amount] from [bank]."],
  ["⚡ Utility", "Draft a money-out entry in Books: paid the [electric or water] bill of [amount] from [bank]."],
];

function sideAsk(text: string) {
  if (!buwiz.copy) return toast("This Buwiz Bot cannot copy. Type it in the chat.");
  buwiz.copy(text);
  toast("Copied. Paste it in the chat; the draft lands in your Inbox.");
}

/** The old app's AI chat panel: quick prompts, the prompt box, and Apply. */
function SideAssistant() {
  const send = () => {
    const text = sideComposer().trim();
    if (!text) return;
    sidePromptError.set("");
    sideAsk(text);
    sideComposer.set("");
  };
  return (
    <View class="p-3 gap-3">
      <View class="flex-row gap-1.5 flex-wrap">
        <Each of={SIDE_ASK}>
          {(q: Any) => (
            <View
              class="px-2 py-1 rounded-full text-[10px] font-medium bg-[#f1f5f9] dark:bg-slate-800 border border-[#e2e8f0] dark:border-slate-700 hover:bg-[#e2e8f0] dark:hover:bg-slate-700 transition-colors cursor-pointer"
              onPress={() => { sideComposer.set(q()[1]); sidePromptError.set(""); }}
              activeStyle={{ opacity: 0.7 }}
            >
              <Text class="text-[10px] font-medium text-[#475569] dark:text-slate-400">{q()[0]}</Text>
            </View>
          )}
        </Each>
      </View>
      <View class="relative">
        <View class="rounded-lg border border-[#e2e8f0] dark:border-slate-600 bg-[#f8f9fb] dark:bg-slate-800 px-3 py-2 focus-within:border-[#2a9d8f] focus-within:ring-1 focus-within:ring-[#2a9d8f] transition-colors">
          <TextBox label="Assistant prompt" value={sideComposer()} onChange={(v: string) => { sideComposer.set(v); sidePromptError.set(""); }} placeholder='"Paid ₱500 for office supplies"' entry />
        </View>
        <If when={Boolean(sideComposer())}>
          <View
            class="absolute right-2 top-2 w-4 h-4 rounded items-center justify-center cursor-pointer"
            onPress={() => { sideComposer.set(""); sidePromptError.set(""); }}
            activeStyle={{ opacity: 0.7 }}
            accessibilityLabel="Clear prompt"
          >
            <Text class="text-[10px] text-[#94a3b8]">×</Text>
          </View>
        </If>
      </View>
      <If when={Boolean(sidePromptError())}>
        <View class="px-2.5 py-2 rounded-lg bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800">
          <Text class="text-[10px] text-red-700 dark:text-red-300">{sidePromptError()}</Text>
        </View>
      </If>
      <View class="flex-row items-center justify-between mt-1.5">
        <Text class="text-[9px] text-[#94a3b8] dark:text-slate-500">A press copies the prompt to the Bot chat</Text>
        <View
          class={`flex-row items-center gap-1 px-3 py-1.5 rounded-md bg-[#2a9d8f] ${sideComposer().trim() ? "" : "opacity-40"}`}
          onPress={send}
          activeStyle={{ opacity: 0.7 }}
          accessibilityLabel="Apply prompt"
        >
          <Text class="text-[10px] font-medium text-white">✈ Apply</Text>
        </View>
      </View>
      <Text class={`text-[11.5px] ${C.dim}`}>Tell the assistant in the chat what happened. It saves a draft, which waits in your Inbox for you to post — the assistant never posts.</Text>
    </View>
  );
}

/** The old app's activity tab: the clock icon, the log, one live row per change. */
function SideActivity() {
  return (
    <View class="p-3 gap-3">
      <View class="items-center">
        <Text class="text-[18px] text-[#94a3b8]">◷</Text>
      </View>
      <Text class={`text-[14px] font-semibold ${C.ink} text-center`}>Activity Log</Text>
      <View class="flex-row items-start gap-2">
        <View class="w-6 h-6 rounded-full items-center justify-center shrink-0" style={{ backgroundColor: "#2a9d8f" }}>
          <Text class="text-[10px] font-bold text-white">U</Text>
        </View>
        <View class="gap-0.5">
          <Text class={`text-[12px] ${C.ink}`}>
            <Text class="font-medium">User</Text>
            <Text class="text-[#94a3b8] dark:text-slate-500"> Just now</Text>
          </Text>
          <Text class={`text-[11px] ${C.mute}`}>Creating transaction.</Text>
        </View>
      </View>
    </View>
  );
}

/** The new-entry right column itself: the rail and the open panel. */
function EntrySide() {
  const side = entrySide;
  return (
    <View class="w-[360px] shrink-0 rounded-[24px] border border-[#e2e8f0] dark:border-slate-600 overflow-hidden shadow-[0_4px_24px_rgba(0,0,0,0.08)] flex-col h-full bg-white dark:bg-slate-800">
      <View class={`flex-row justify-evenly px-4 py-3 gap-2 ${accent() === "green" ? "bg-linear-to-r from-[#1a6b3c] to-[#27ae60] dark:from-[#145a30] dark:to-[#1e8c4c]" : ""}`} style={accent() === "green" ? undefined : { backgroundColor: acc() }}>
        <Each of={[["attachments", "⌁"], ["ai", "✧"], ["activity", "◷"]]}>
          {(t: Any) => (
            <View
              accessibilityLabel={t()[0] === "ai" ? "Assistant" : t()[0] === "attachments" ? "Attachments" : "Activity log"}
              class={`w-[88px] h-11 rounded-full items-center justify-center ${side() === t()[0] ? "bg-white/20" : ""}`}
              onPress={() => side.set(t()[0])}
              activeStyle={{ opacity: 0.7 }}
            >
              <Text class={`text-[16px] ${side() === t()[0] ? "text-white" : "text-white/50"}`}>{t()[1]}</Text>
            </View>
          )}
        </Each>
      </View>
      <Scroll class="flex-1">
        <If when={side() === "attachments"}>
          <SideAttachments />
        </If>
        <If when={side() === "ai"}>
          <SideAssistant />
        </If>
        <If when={side() === "activity"}>
          <SideActivity />
        </If>
      </Scroll>
    </View>
  );
}
