// The new-entry page's right column: the tab rail (Attachments, Assistant,
// Activity) and the three panels, a port of the old web app's sidebar on
// /transactions/new (AttachmentsPanel, AIChatPanel, Activity Log). Uploads go
// through the host file picker and books_app_evidence_attach. The chat feed
// shows what the person sends from this tab — the assistant's reply lives in
// the Bot chat; the assistant only drafts, nothing posts itself.

/** A document this page staged for the entry being drafted. */
const entryStaged = signal<Any[]>([]);
const entryStagedBusy = signal(false);
const entrySide = signal("attachments");
const entrySideSheet = signal(false);
const sideComposer = signal("");
const sidePromptError = signal("");
/** What the person sent from the chat tab; role "note" rows are delivery notes. */
const sideChat = signal<Any[]>([]);
/** The feed's scroll view; scrollToEnd keeps the newest message in view. */
let sideFeedRef: Any;
/** Files picked into the composer, not yet sent. */
const sideFiles = signal<Any[]>([]);

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
const SIDE_MIME: Record<string, string> = { "application/pdf": "PDF", "image/png": "PNG", "image/jpeg": "JPEG" };
const SIDE_EXT: Record<string, string> = { pdf: "application/pdf", png: "image/png", jpg: "image/jpeg", jpeg: "image/jpeg" };
// the manifest's pick_file grant caps a file at 50 MiB; the upload path
// enforces the same so the limit is told before a pick, not after one
const SIDE_MAX_BYTES = 52428800;
// images at most this large get an in-feed thumbnail; bigger ones a file card
const SIDE_THUMB_BYTES = 2097152;

// The rail's tab icons and the panel glyphs: the old app's stroke SVGs
// (transactions_.new.tsx, AttachmentsPanel.tsx), as data URIs through Avatar —
// the SDK-1 sandbox has no Image import.
const sideSvg = (inner: string, width: number, stroke = "white") =>
  `data:image/svg+xml,${encodeURIComponent(
    `<svg viewBox="0 0 24 24" fill="none" stroke="${stroke}" stroke-width="${width}" stroke-linecap="round" stroke-linejoin="round">${inner}</svg>`,
  )}`;
const SIDE_PATHS: Record<string, string> = {
  attachments: '<path d="M21.44 11.05l-9.19 9.19a6 6 0 0 1-8.49-8.49l9.19-9.19a4 4 0 0 1 5.66 5.66l-9.2 9.19a2 2 0 0 1-2.83-2.83l8.49-8.48"/>',
  ai: '<path d="M12 3l1.5 4.5L18 9l-4.5 1.5L12 15l-1.5-4.5L6 9l4.5-1.5L12 3z"/>',
  activity: '<circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/>',
  upload: '<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/>',
  file: '<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/>',
  image: '<rect x="3" y="3" width="18" height="18" rx="2" ry="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/>',
  folder: '<path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>',
  close: '<path d="M18 6L6 18M6 6l12 12"/>',
  send: '<line x1="12" y1="19" x2="12" y2="5"/><polyline points="5 12 12 5 19 12"/>',
};
const SIDE_ICONS: Record<string, string> = {
  attachments: sideSvg(SIDE_PATHS.attachments, 2),
  ai: sideSvg(SIDE_PATHS.ai, 2),
  activity: sideSvg(SIDE_PATHS.activity, 1.5),
};
const SIDE_GLYPHS: Record<string, string> = {
  upload: sideSvg(SIDE_PATHS.upload, 1.5, "#94a3b8"),
  file: sideSvg(SIDE_PATHS.file, 1.5, "#ef4444"),
  image: sideSvg(SIDE_PATHS.image, 1.5, "#ef4444"),
  folder: sideSvg(SIDE_PATHS.folder, 2, "#64748b"),
  fileBig: sideSvg(SIDE_PATHS.file, 1.5, "#94a3b8"),
  close: sideSvg(SIDE_PATHS.close, 2.5, "#94a3b8"),
  clip: sideSvg(SIDE_PATHS.attachments, 2, "#64748b"),
  sparkle: sideSvg(SIDE_PATHS.ai, 2, "#94a3b8"),
  clock: sideSvg(SIDE_PATHS.activity, 1.5, "#94a3b8"),
  send: sideSvg(SIDE_PATHS.send, 2),
};

function sideB64(bytes: Any, urlSafe = true): string {
  const A = urlSafe ? "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_" : "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  let out = "";
  for (let i = 0; i < bytes.length; i += 3) {
    const n = (bytes[i] << 16) | ((bytes[i + 1] || 0) << 8) | (bytes[i + 2] || 0);
    const left = bytes.length - i;
    out += A[(n >> 18) & 63] + A[(n >> 12) & 63];
    if (left > 1) out += A[(n >> 6) & 63];
    else if (!urlSafe) out += "=";
    if (left > 2) out += A[n & 63];
    else if (!urlSafe) out += "=";
  }
  return out;
}

function sideBytesShown(size: Any): string {
  const n = Number(size || 0);
  if (n >= 1048576) return `${Math.round(n / 104857.6) / 10} MB`;
  if (n >= 1024) return `${Math.round(n / 1024)} KB`;
  return `${n} B`;
}

function sideNow(): string {
  try {
    const d = new Date();
    return `${d.getHours()}:${`0${d.getMinutes()}`.slice(-2)}`;
  } catch {
    return "";
  }
}

/** Pick a PDF or image and add it to Books documents; returns the doc, or null. */
async function sidePickUpload(): Promise<Any | null> {
  if (!buwiz.pickFile) {
    toast("This Buwiz Bot cannot pick files.");
    return null;
  }
  let files: Any[] = [];
  try {
    files = (await buwiz.pickFile({ extensions: [".pdf", ".png", ".jpg", ".jpeg"] })) as Any[];
  } catch (e) {
    toast(errText(e));
    return null;
  }
  const file = (files || [])[0];
  if (!file) {
    toast("No file was picked — this build may not have a file dialog.");
    return null;
  }
  const ext = String(file.name || "").split(".").pop()?.toLowerCase() || "";
  const mime = SIDE_EXT[ext] || String(file.type || "");
  if (!SIDE_MIME[mime]) {
    toast("Books keeps PDF, PNG and JPEG documents.");
    return null;
  }
  if (Number(file.size || 0) > SIDE_MAX_BYTES) {
    toast("A file is at most 50 MB.");
    return null;
  }
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
    const doc: Any = { evidence: r?.evidence || "", filename: String(file.name || "document"), mime, size: bytes.length };
    if (mime.startsWith("image/") && bytes.length <= SIDE_THUMB_BYTES) doc.thumb = `data:${mime};base64,${sideB64(bytes, false)}`;
    return doc;
  } catch (e) {
    toast(errText(e));
    return null;
  } finally {
    entryStagedBusy.set(false);
  }
}

/** Drop-zone upload: straight into this entry's staged documents. */
async function sideUpload() {
  const doc = await sidePickUpload();
  if (doc) {
    entryStaged.set([...entryStaged(), doc]);
    toast("Added to Books documents.");
  }
}

/** Composer paperclip: stage the file on the message and count it on the entry too. */
async function sidePickForChat() {
  const doc = await sidePickUpload();
  if (doc) {
    sideFiles.set([...sideFiles(), doc]);
    entryStaged.set([...entryStaged(), doc]);
  }
}

function sidePreview() {
  // naming the preview tool here would push the window past the 64-tool grant
  // cap, so a press opens the document library where the file previews
  buwiz.open({ app: "evidence" }).catch((e: Any) => toast(errText(e)));
}

/** The old app's attachments panel: drop zone, staged documents, library browse. */
function SideAttachments() {
  return (
    <View class="p-3 gap-3">
      <View
        class="flex-col items-center justify-center gap-2 px-4 py-6 rounded-xl border-2 border-dashed border-[#cbd5e1] cursor-pointer transition-all dark:border-slate-600 hover:border-[#2a9d8f] dark:hover:border-[#2a9d8f] hover:bg-[#f8fafb] dark:hover:bg-slate-800/50"
        accessibilityLabel="Browse for an attachment"
        onPress={() => sideUpload()}
        activeStyle={{ opacity: 0.75 }}
      >
        <View class="w-10 h-10 rounded-full flex items-center justify-center bg-[#f1f5f9] dark:bg-slate-800">
          <Avatar src={SIDE_GLYPHS.upload} accessibilityLabel="" class="w-5 h-5" fallbackDelay={Infinity} />
        </View>
        <View class="items-center">
          <Text class="text-[12px] font-medium text-[#2a9d8f]">{entryStagedBusy() ? "Uploading…" : "Browse"}</Text>
          <Text class="text-[9px] text-[#94a3b8] dark:text-slate-500">Images or PDF — max 50MB</Text>
        </View>
      </View>
      <If when={entryStaged().length > 0}>
        <Each of={entryStaged()} key={rowKey}>
          {(d: Any, i: Any) => (
            <View class="group flex-row items-center gap-2.5 px-3 py-2 rounded-lg bg-white dark:bg-slate-800 border border-[#e2e8f0] dark:border-slate-700 hover:border-[#2a9d8f]/40 dark:hover:border-[#2a9d8f]/40 transition-colors">
              <View class="w-8 h-8 rounded-lg bg-red-50 dark:bg-red-900/20 items-center justify-center shrink-0">
                <Avatar src={String(d().mime).startsWith("image/") ? SIDE_GLYPHS.image : SIDE_GLYPHS.file} accessibilityLabel="" class="w-4 h-4" fallbackDelay={Infinity} />
              </View>
              <View class="flex-1 min-w-0 gap-0.5">
                <Text class="text-[10px] font-medium text-[#1e293b] dark:text-white truncate">{d().filename}</Text>
                <Text class="text-[9px] text-[#94a3b8] dark:text-slate-500">{sideBytesShown(d().size)}</Text>
              </View>
              <View
                class="w-7 h-7 rounded-md items-center justify-center hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors cursor-pointer shrink-0 opacity-0 group-hover:opacity-100"
                accessibilityLabel="Remove attachment"
                onPress={() => entryStaged.set(entryStaged().filter((_: Any, j: number) => j !== i()))}
                activeStyle={{ opacity: 0.7 }}
              >
                <Avatar src={SIDE_GLYPHS.close} accessibilityLabel="" class="w-3 h-3" fallbackDelay={Infinity} />
              </View>
            </View>
          )}
        </Each>
      </If>
      <View
        class="flex-row items-center justify-center gap-1.5 px-3 py-2 rounded-lg group border border-[#e2e8f0] dark:border-white/10 hover:bg-[#f8fafc] dark:hover:bg-white/5 transition-colors cursor-pointer"
        accessibilityLabel="Browse existing documents"
        onPress={() => buwiz.open({ app: "evidence" }).catch(() => null)}
        activeStyle={{ opacity: 0.7 }}
      >
        <Avatar src={SIDE_GLYPHS.folder} accessibilityLabel="" class="w-4 h-4" fallbackDelay={Infinity} />
        <Text class="text-xs font-medium text-[#64748b] dark:text-white/50 group-hover:text-[#1e293b] dark:group-hover:text-white">Browse existing documents</Text>
      </View>
      <If when={entryStaged().length === 0}>
        <View class="flex-1 flex-col items-center justify-center py-8 opacity-60 gap-0">
          <Avatar src={SIDE_GLYPHS.fileBig} accessibilityLabel="" class="w-7 h-7 mb-2" fallbackDelay={Infinity} />
          <Text class="text-[10px] font-medium text-[#94a3b8] dark:text-slate-500">No attachments yet</Text>
          <Text class="text-[9px] text-[#94a3b8] dark:text-slate-500 mt-0.5">Drop receipts, invoices, or documents</Text>
        </View>
      </If>
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

/** A file on a sent message: an image thumbnail or the old panel's file card. */
function SideChatFile(p: { d: Any }) {
  const d = p.d;
  if (d.thumb) {
    return (
      <View class="rounded-lg overflow-hidden cursor-pointer" accessibilityLabel={`Preview ${d.filename}`} onPress={() => sidePreview()} activeStyle={{ opacity: 0.8 }}>
        <Avatar src={d.thumb} accessibilityLabel={d.filename} class="w-44 h-28" imageClass="object-cover" fallbackDelay={Infinity} />
      </View>
    );
  }
  return (
    <View
      class="flex-row items-center gap-2 max-w-[85%] px-2.5 py-2 rounded-lg bg-white dark:bg-slate-800 border border-[#e2e8f0] dark:border-slate-700 hover:border-[#2a9d8f]/40 dark:hover:border-[#2a9d8f]/40 transition-colors cursor-pointer"
      accessibilityLabel={`Preview ${d.filename}`}
      onPress={() => sidePreview()}
      activeStyle={{ opacity: 0.8 }}
    >
      <View class="w-8 h-8 rounded-lg bg-red-50 dark:bg-red-900/20 items-center justify-center shrink-0">
        <Avatar src={SIDE_GLYPHS.file} accessibilityLabel="" class="w-4 h-4" fallbackDelay={Infinity} />
      </View>
      <View class="flex-1 min-w-0 gap-0.5">
        <Text class="text-[10px] font-medium text-[#1e293b] dark:text-white truncate">{d.filename}</Text>
        <Text class="text-[9px] text-[#94a3b8] dark:text-slate-500">{`${SIDE_MIME[d.mime] || "File"} · ${sideBytesShown(d.size)}`}</Text>
      </View>
    </View>
  );
}

/** A press copies the prompt to the Bot chat; the files stay staged on the
 *  entry and in Documents (attaching to a chat is the library's own grant).
 *  The assistant's reply shows in the Bot chat and its draft in the Inbox. */
async function sideSend() {
  const text = sideComposer().trim();
  const files = sideFiles();
  if (!text && files.length === 0) return;
  const problems: string[] = [];
  if (text) {
    if (!buwiz.copy) problems.push("this Bot has no chat to copy into");
    else {
      try {
        await buwiz.copy(text);
      } catch (e) {
        problems.push(`the prompt (${errText(e)})`);
      }
    }
  }
  const at = sideNow();
  sideChat.set([...sideChat(), { role: "you", text, files, at }]);
  // only the document library may attach evidence to a chat (manifest_shape),
  // so the files stay staged on this entry and in Documents instead
  sideChat.set([
    ...sideChat(),
    {
      role: "note",
      error: problems.length > 0,
      text: problems.length
        ? `Couldn't send ${problems.join(", ")} — paste it into the Bot chat yourself.`
        : `Sent to the Bot chat — the reply shows there, and its draft waits in your Inbox.${files.length ? " The files are staged on this entry and in Documents." : ""}`,
      at,
    },
  ]);
  sideComposer.set("");
  sideFiles.set([]);
  sidePromptError.set("");
  sideFeedRef?.scrollToEnd?.({ animated: true });
}

/** The old app's AI chat panel: the feed, quick prompts, and the composer. */
function SideAssistant() {
  const msgs = sideChat;
  const canSend = () => Boolean(sideComposer().trim()) || sideFiles().length > 0;
  return (
    <View class="flex-1 flex-col min-h-0">
      <Scroll ref={sideFeedRef} class="flex-1 px-3 py-3">
        <If when={msgs().length === 0}>
          <View class="flex-1 flex-col items-center justify-center py-10 gap-2">
            <Avatar src={SIDE_GLYPHS.sparkle} accessibilityLabel="" class="w-8 h-8" fallbackDelay={Infinity} />
            <Text class="text-[12px] font-medium text-[#64748b] dark:text-slate-400">Ask the assistant</Text>
            <Text class="text-[10px] text-[#94a3b8] dark:text-slate-500 text-center px-3">Describe a transaction, or attach a receipt or bill. The assistant drafts it; you post it.</Text>
          </View>
        </If>
        <Each of={msgs()} key={rowKey}>
          {(m: Any) =>
            m().role === "note" ? (
              <View class="items-center px-2 py-1">
                <If when={Boolean(m().error)}>
                  <View class="px-2.5 py-2 rounded-lg bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800">
                    <Text class="text-[10px] text-red-700 dark:text-red-300 text-center">{m().text}</Text>
                  </View>
                </If>
                <If when={!m().error}>
                  <Text class="text-[10px] text-[#94a3b8] dark:text-slate-500 text-center">{m().text}</Text>
                </If>
              </View>
            ) : (
              <View class="items-end gap-1 py-1">
                <Each of={m().files || []} key={rowKey}>{(f: Any) => <SideChatFile d={f()} />}</Each>
                <If when={Boolean(m().text)}>
                  <View class="max-w-[85%] bg-[#2a9d8f] rounded-2xl rounded-br-md px-3 py-2">
                    <Text class="text-[12px] leading-relaxed text-white">{m().text}</Text>
                  </View>
                </If>
                <If when={Boolean(m().at)}>
                  <Text class="text-[9px] text-[#94a3b8] dark:text-slate-500">{m().at}</Text>
                </If>
              </View>
            )
          }
        </Each>
      </Scroll>
      <View class="px-3 pb-2 flex-row gap-1.5 flex-wrap">
        <Each of={SIDE_ASK}>
          {(q: Any) => (
            <View
              class="px-2 py-1 rounded-full bg-[#f1f5f9] dark:bg-slate-800 border border-[#e2e8f0] dark:border-slate-700 hover:bg-[#e2e8f0] dark:hover:bg-slate-700 transition-colors cursor-pointer"
              onPress={() => { sideComposer.set(q()[1]); sidePromptError.set(""); }}
              activeStyle={{ opacity: 0.7 }}
            >
              <Text class="text-[10px] font-medium text-[#475569] dark:text-slate-400">{q()[0]}</Text>
            </View>
          )}
        </Each>
      </View>
      <View class="p-3 border-t border-[#e2e8f0] dark:border-slate-700">
        <View class="rounded-xl border border-[#e2e8f0] dark:border-slate-600 bg-[#f8f9fb] dark:bg-slate-800 px-3 py-2 gap-2 focus-within:border-[#2a9d8f] focus-within:ring-1 focus-within:ring-[#2a9d8f] transition-colors">
          <If when={sideFiles().length > 0}>
            <View class="flex-row gap-1.5 flex-wrap">
              <Each of={sideFiles()} key={rowKey}>
                {(f: Any, i: Any) => (
                  <View class="flex-row items-center gap-1 px-2 py-1 rounded-md bg-white dark:bg-slate-700 border border-[#e2e8f0] dark:border-slate-600">
                    <Text class="text-[9px] font-medium text-[#475569] dark:text-slate-300 max-w-24 truncate">{f().filename}</Text>
                    <View
                      class="w-3.5 h-3.5 rounded items-center justify-center cursor-pointer"
                      accessibilityLabel="Remove file"
                      onPress={() => sideFiles.set(sideFiles().filter((_: Any, j: number) => j !== i()))}
                      activeStyle={{ opacity: 0.7 }}
                    >
                      <Avatar src={SIDE_GLYPHS.close} accessibilityLabel="" class="w-2.5 h-2.5" fallbackDelay={Infinity} />
                    </View>
                  </View>
                )}
              </Each>
            </View>
          </If>
          <Input
            accessibilityLabel="Assistant prompt"
            value={sideComposer()}
            placeholder='"Paid ₱500 for office supplies"'
            multiline={true}
            submitBehavior="submit"
            onSubmit={() => sideSend()}
            onChange={(e: Any) => { sideComposer.set(typeof e === "string" ? e : e.value); sidePromptError.set(""); }}
            class="w-full min-h-6 max-h-28 text-[13px] bg-transparent border-none outline-none placeholder:text-[#8e8e93] text-[#0f172a] dark:text-slate-100 focus:outline-none focus:ring-0"
          />
          <View class="flex-row items-center justify-between">
            <View
              class="w-7 h-7 rounded-lg items-center justify-center hover:bg-[#f1f5f9] dark:hover:bg-slate-700 transition-colors cursor-pointer"
              accessibilityLabel="Attach a receipt or bill"
              onPress={() => sidePickForChat()}
              activeStyle={{ opacity: 0.7 }}
            >
              <Avatar src={SIDE_GLYPHS.clip} accessibilityLabel="" class="w-4 h-4" fallbackDelay={Infinity} />
            </View>
            <View class="flex-row items-center gap-2">
              <Text class="text-[9px] text-[#94a3b8] dark:text-slate-500">↵ send · ⇧↵ line</Text>
              <View
                class={`w-8 h-8 rounded-full items-center justify-center bg-[#2a9d8f] ${canSend() ? "cursor-pointer" : "opacity-40"}`}
                accessibilityLabel="Send to the Bot chat"
                onPress={() => sideSend()}
                activeStyle={{ opacity: 0.7 }}
              >
                <Avatar src={SIDE_GLYPHS.send} accessibilityLabel="" class="w-4 h-4" fallbackDelay={Infinity} />
              </View>
            </View>
          </View>
        </View>
      </View>
    </View>
  );
}

/** The old app's activity tab: the clock icon, the log, one live row per change. */
function SideActivity() {
  return (
    <View class="p-3 gap-3">
      <View class="items-center">
        <Avatar src={SIDE_GLYPHS.clock} accessibilityLabel="" class="w-5 h-5" fallbackDelay={Infinity} />
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
    <View class="w-full shrink-0 rounded-[24px] border border-[#e2e8f0] dark:border-slate-600 overflow-hidden shadow-[0_4px_24px_rgba(0,0,0,0.08)] flex-col h-full bg-white dark:bg-slate-800">
      <View class={`flex-row justify-evenly px-4 py-3 gap-2 ${accent() === "green" ? "bg-linear-to-r from-[#1a6b3c] to-[#27ae60] dark:from-[#145a30] dark:to-[#1e8c4c]" : ""}`} style={accent() === "green" ? undefined : { backgroundColor: acc() }}>
        <Each of={["attachments", "ai", "activity"]}>
          {(t: Any) => (
            <View
              accessibilityLabel={t() === "ai" ? "Assistant" : t() === "attachments" ? "Attachments" : "Activity log"}
              class={`group relative w-[88px] h-11 rounded-full items-center justify-center transition-colors ${side() === t() ? "bg-white/20" : "hover:bg-white/10"}`}
              onPress={() => side.set(t())}
              activeStyle={{ opacity: 0.7 }}
            >
              <Avatar
                src={SIDE_ICONS[t()]}
                accessibilityLabel=""
                class="w-4 h-4"
                fallbackDelay={Infinity}
                imageClass={side() === t() ? "tint-white" : "tint-white/50 group-hover:tint-white"}
              />
              <If when={t() === "attachments" && entryStaged().length > 0}>
                <View class="absolute -top-0.5 -right-0.5 w-4 h-4 rounded-full bg-white items-center justify-center">
                  <Text class="text-[9px] font-bold text-[#1a6b3c]">{`${entryStaged().length}`}</Text>
                </View>
              </If>
            </View>
          )}
        </Each>
      </View>
      <If when={side() === "ai"}>
        <SideAssistant />
      </If>
      <If when={side() !== "ai"}>
        <Scroll class="flex-1">
          <If when={side() === "attachments"}>
            <SideAttachments />
          </If>
          <If when={side() === "activity"}>
            <SideActivity />
          </If>
        </Scroll>
      </If>
    </View>
  );
}
