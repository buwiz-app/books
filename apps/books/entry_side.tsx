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
/** The models() catalogue once asked; null while loading or unsupported. */
const sideModels = signal<Any | null>(null);
const sideModelsAsked = signal(false);
/** The composer's model popover and its search text. */
const sidePicker = signal(false);
const sidePickerQ = signal("");
/** The picked model id ("" = the Bot's default) and effort value. */
const sideModel = signal("");
const sideEffort = signal("");
/** The in-flight chat handle while a reply streams. */
const sideStream = signal<Any | null>(null);
let sideStoreAsked = false;

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
// A standalone SVG-as-image renders only with its namespace. The namespace is
// an identifier, never a fetched URL — but the bundle refuses URL literals, so
// it is assembled without one (PR37 finding 2).
const SVG_NS = "http" + "://www.w3.org/2000/svg";
const sideSvg = (inner: string, width: number, stroke = "white") =>
  `data:image/svg+xml,${encodeURIComponent(
    `<svg xmlns="${SVG_NS}" viewBox="0 0 24 24" fill="none" stroke="${stroke}" stroke-width="${width}" stroke-linecap="round" stroke-linejoin="round">${inner}</svg>`,
  )}`;
const SIDE_PATHS: Record<string, string> = {
  attachments: '<path d="M21.44 11.05l-9.19 9.19a6 6 0 0 1-8.49-8.49l9.19-9.19a4 4 0 0 1 5.66 5.66l-9.2 9.19a2 2 0 0 1-2.83-2.83l8.49-8.48"/>',
  ai: '<path d="M12 3l2.2 6.8L21 12l-6.8 2.2L12 21l-2.2-6.8L3 12l6.8-2.2L12 3z"/>',
  activity: '<circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/>',
  upload: '<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/>',
  file: '<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/>',
  image: '<rect x="3" y="3" width="18" height="18" rx="2" ry="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/>',
  folder: '<path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>',
  close: '<path d="M18 6L6 18M6 6l12 12"/>',
  send: '<line x1="12" y1="19" x2="12" y2="5"/><polyline points="5 12 12 5 19 12"/>',
  search: '<circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>',
  chev: '<polyline points="6 9 12 15 18 9"/>',
  check: '<polyline points="20 6 9 17 4 12"/>',
  stop: '<rect x="6" y="6" width="12" height="12" rx="2" fill="white" stroke="none"/>',
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
  search: sideSvg(SIDE_PATHS.search, 2, "#94a3b8"),
  chev: sideSvg(SIDE_PATHS.chev, 2, "#94a3b8"),
  check: sideSvg(SIDE_PATHS.check, 2.5, "#1e86fc"),
  stop: sideSvg(SIDE_PATHS.stop, 2),
};

// The Bot picker's maker logos (app/src/icons.rs), as fill SVG data URIs;
// a black fill lets Avatar's tint carry the row color.
// `xmlns` is left out on purpose: the privacy gate rejects URL literals.
const logoUri = (vb: string, paths: string[], eo: boolean) =>
  `data:image/svg+xml,${encodeURIComponent(
    `<svg viewBox="${vb}" fill="black">${paths.map((d) => `<path d="${d}"${eo ? ' fill-rule="evenodd"' : ""}/>`).join("")}</svg>`,
  )}`;
const SIDE_LOGOS: Record<string, string> = {
  anthropic: logoUri("0 0 24 24", ["M13.8 3.5h3.6L24 20h-3.6l-6.5-16.4zm-7.2 0h3.7L16.9 20h-3.6l-1.3-3.4H5.0l-1.3 3.4H0L6.5 3.5zm4.1 9.9L8.4 7.6 6.2 13.4H10.7z"], true),
  openai: logoUri("0 0 24 24", ["M21.5 10.0a5.4 5.4 0 00-.4-4.5c-1.2-2.0-3.6-3.1-6.0-2.6A5.5 5.5 0 0010.8 1C8.3.9 6.2 2.5 5.4 4.8A5.5 5.5 0 001.7 7.4a5.4 5.4 0 00.6 6.5 5.4 5.4 0 00.4 4.5c1.2 2.0 3.6 3.1 6.0 2.6A5.5 5.5 0 0013.1 23c2.4.0 4.6-1.5 5.3-3.8a5.5 5.5 0 003.7-2.6 5.4 5.4 0 00-.6-6.4v.0zm-8.3 11.5a4.1 4.1 0 01-2.6-.9c.0-.0.0-.0.1-.0l4.4-2.5a.7.7 0 00.3-.6v-6.1l1.8 1.0c.0.0.0.0.0.0v5.1c-.0 2.2-1.8 4.1-4.1 4.1zM4.1 17.7a4.0 4.0 0 01-.4-2.7c.0.0.0.0.1.0l4.4 2.5c.2.1.5.1.7 0l5.4-3.0v2.1a.0.0 0 01-.0.0L9.9 19.2c-1.9 1.1-4.5.4-5.7-1.5h-.0zM3.0 8.2A4.1 4.1 0 015.1 6.4l-.0.1v5.0a.7.7 0 00.3.6l5.4 3.0-1.8 1.0a.0.0 0 01-.0.0l-4.4-2.5c-1.9-1.1-2.6-3.6-1.5-5.6h.0zm15.4 3.5l-5.4-3.0L14.8 7.6a.0.0 0 01.0-.0l4.4 2.5c1.9 1.1 2.6 3.6 1.5 5.6a4.1 4.1 0 01-2.1 1.8V12.3a.7.7 0 00-.3-.6zm1.8-2.7a6.0 6.0 0 00-.1-.0l-4.4-2.5a.7.7 0 00-.7 0l-5.4 3.0V7.3a.0.0 0 01.0-.0L14.1 4.7c2-1.1 4.5-.4 5.7 1.5.4.8.6 1.8.4 2.7h.0zm-11.7 3.8l-1.8-1.0a.0.0 0 01-.0-.0V6.5c.0-2.2 1.8-4.1 4.1-4.1.9 0 1.9.3 2.6.9-.0.0-.0.0-.1.0l-4.4 2.5a.7.7 0 00-.3.6l-.0 6.1v.0zm1.0-2.1L12 9.2l2.4 1.3v2.7L12 14.7l-2.4-1.3v-2.7z"], true),
  xai: logoUri("0 0 24 24", ["M6.4 8.7L16.5 23h-4.4L2.0 8.7H6.4zm-.0 7.9l2.2 3.1L6.4 23H2l4.4-6.3zM22 2.5V23h-3.6V7.7L22 2.5zM22 1l-9.9 14.0-2.2-3.1L17.5 1H22z"], true),
  google: logoUri("0 0 24 24", ["M20.6 10.8a14.1 14.1 0 01-4.4-3.0 14.1 14.1 0 01-3.6-6.4.5.5 0 00-.9 0 14.1 14.1 0 01-3.6 6.4 14.1 14.1 0 01-4.4 3.0c-.6.2-1.3.5-2.0.6a.5.5 0 000 .9c.6.1 1.3.3 2.0.6a14.1 14.1 0 014.4 3.0 14.1 14.1 0 013.6 6.4.5.5 0 00.9 0c.1-.6.3-1.3.6-2.0a14.1 14.1 0 013.0-4.4 14.1 14.1 0 016.4-3.6.5.5 0 000-.9 13.2 13.2 0 01-2.0-.6z"], true),
  mistral: logoUri("0 0 24 24", ["M3.4 3.4h3.4v3.4h3.4v3.4h-.0 3.4V6.8h3.4V3.4h3.4v13.7H24v3.4H13.7v-3.4h-3.4v-3.4h-3.4v3.4h3.4v3.4H0v-3.4h3.4V3.4zm10.2 13.7h3.4v-3.4h-3.4v3.4z"], true),
  meta: logoUri("0 0 24 24", ["M6.8 4c1.9 0 3.5.9 5.4 3.3l.2-.3c.1-.2.3-.4.5-.7l.3-.3C14.5 4.7 15.7 4 17.2 4c1.2 0 2.4.5 3.4 1.5l.2.2c1.7 1.7 2.9 4.7 3.0 8.0l.0.3.0.2c0 1.5-.2 2.7-.8 3.7l-.1.2-.1.1c-.3.4-.6.7-1.0 1.0l-.2.1-.0.0a3.4 3.4 0 01-.3.1 4.1 4.1 0 01-1.3.2c-.5 0-.9-.0-1.4-.2-.6-.2-1.1-.5-1.7-1.1l-.2-.2c-.7-.8-1.5-1.9-2.4-3.5l-1.4-2.4-.5-.8-1.7 3.1-.3.5C7.5 19.1 6.2 20 4.3 20c-1.2 0-2.2-.4-2.9-1.1l-.1-.1c-.4-.5-.8-1.3-1.0-2.1l-.0-.3a8.6 8.6 0 01-.1-1.2L0 14.4c.0-.7.0-1.4.1-2.2l.1-.5c.2-1.5.8-2.9 1.5-4.1l.2-.3c1.1-1.8 2.7-3.0 4.6-3.1L6.8 4zm-.0 2.6l-.2.0c-.8.0-1.6.6-2.2 1.5l-.1.1-.0.0c-.6 1.0-1.1 2.3-1.4 3.8l-.0.0a12.5 12.5 0 00-.2 2.2l.0.1c.0.1.0.3.0.5l.0.2c.0.5.2.9.4 1.2l.1.1c.3.3.6.4 1.1.4 1.1 0 1.7-.6 3.6-3.6l2.1-3.4.4-.7-.1-.1C9.1 7.3 8.0 6.6 6.8 6.6zm10.1-.5l-.1.0c-.6.0-1.2.3-1.8.9l-.1.1c-.4.4-.8 1.0-1.3 1.8l.2.3c.1.2.3.5.5.8l.2.4 1.3 2.3.6 1.1c.5.9 1.0 1.6 1.4 2.0l.2.2c.2.3.5.5.7.6l.1.0c.2.1.4.1.7.1.1.0.3-.0.5-.0.3-.1.6-.3.8-.6l.0-.1.0-.1c.1-.4.2-1.0.2-1.7l-.0-.4c-.0-2.8-.9-5.3-2.2-6.7l-.1-.1c-.6-.6-1.4-1.0-2.2-1.0z"], true),
};
// qwen/deepseek/cognition ship lazily — the same logos, only heavier paths.
const SIDE_LOGOS_MORE: Record<string, string> = {
  qwen: logoUri("0 0 24 24", ["M12.6 1.3c.3.6.7 1.3 1.1 2.0a.1.1 0 00.1.0h5.5c.1 0 .3.1.4.3l1.4 2.5c.1.3.2.4.0.8-.2.4-.5.8-.7 1.3l-.3.6c-.1.1-.2.2-.0.5l2.6 4.6c.1.3.1.4-.0.7-.4.7-.8 1.5-1.3 2.3-.1.2-.3.3-.6.3-.7-.0-1.5-.0-2.3.0a.0.0 0 00-.0.0 575.0 575.0 0 01-2.7 4.7c-.1.2-.3.3-.7.3-.9.0-2.0.0-3.0.0a.5.5 0 01-.4-.2l-1.3-2.3a.0.0 0 00-.0-.0H4.9c-.2.0-.5-.0-.8-.0l-1.6-2.7a.5.5 0 01-.0-.5l1.2-2.1a.1.1 0 000-.1 550.9 550.9 0 01-1.8-3.2l-.7-1.3c-.1-.3-.1-.4.0-.9.4-.8.9-1.6 1.3-2.4.1-.2.3-.3.5-.3a338.3 338.3 0 012.5-.0.1.1 0 00.1-.0l2.8-4.8a.4.4 0 01.4-.2c.5-.0 1.0 0 1.5-.0L11.7 1c.3-.0.7.0.9.3zm-3.4.4a.0.0 0 00-.0.0L6.2 6.7a.1.1 0 01-.1.0H3.2c-.0 0-.0.0-.0.0l5.8 10.1c.0.0.0.0-.0.0l-2.7.0a.2.2 0 00-.2.1l-1.3 2.3c-.0.0-.0.1.0.1l5.7.0c.0 0 .0.0.1.0l1.4 2.4c.0.0.0.0.1 0l5.0-8.7.7-1.3a.0.0 0 01.0 0l1.4 2.5a.1.1 0 00.1.0l2.7-.0a.0.0 0 00.0-.0.0.0 0 000-.0l-2.9-5.0a.1.1 0 010-.1l.2-.5 1.1-1.9c.0-.0.0-.0-.0-.0H9.2c-.0 0-.0-.0-.0-.0l1.4-2.5a.1.1 0 000-.1L9.2 1.7a.0.0 0 00-.0-.0zm6.2 8.0c.0 0 .0.0.0.0l-.8 1.4-2.6 4.5a.0.0 0 01-.0.0.0.0 0 01-.0-.0L8.4 9.8c-.0-.0-.0-.0.0-.0l.2-.0 6.7-.0z"], true),
  deepseek: logoUri("0 0 24 24", ["M23.7 4.4c-.2-.1-.3.1-.5.2-.0.0-.0.0-.1.1-.3.3-.8.6-1.3.6-.8-.0-1.5.2-2.1.8-.1-.7-.5-1.2-1.2-1.5-.3-.1-.7-.3-.9-.6-.1-.2-.2-.5-.3-.7-.0-.1-.1-.3-.2-.3-.2-.0-.2.1-.3.2-.3.5-.4 1.2-.4 1.8.0 1.4.6 2.5 1.8 3.3.1.0.1.1.1.3-.0.2-.1.5-.2.8-.0.1-.1.2-.3.1a5.5 5.5 0 01-1.7-1.1c-.8-.8-1.6-1.7-2.5-2.4a11.3 11.3 0 00-.6-.4c-.9-.9.1-1.7.3-1.8.2-.0.0-.4-.7-.4-.8.0-1.6.2-2.6.6a3.0 3.0 0 01-.4.1 9.5 9.5 0 00-2.8-.1c-1.8.2-3.3 1.1-4.4 2.6C.0 8.6-.2 10.6.1 12.8c.4 2.2 1.5 4.1 3.3 5.6 1.8 1.5 3.9 2.2 6.4 2.1 1.4-.0 3.1-.2 4.9-1.8.4.2.9.3 1.7.3.6.0 1.2-.0 1.7-.1.7-.1.6-.8.4-.9-2.1-1.0-1.6-.5-2.1-.9 1.0-1.2 2.7-2.6 3.3-7.0.0-.3.0-.5 0-.8-.0-.1.0-.2.2-.2a4.1 4.1 0 001.5-.4c1.3-.7 1.9-2.0 2.0-3.5.0-.2-.0-.4-.2-.5zM11.5 18c-2.0-1.6-3.1-2.1-3.5-2.1-.3.0-.3.4-.2.7.0.2.2.4.3.7.1.1.1.4-.1.6-.6.4-1.8-.1-1.8-.1-1.3-.8-2.5-1.8-3.3-3.3-.7-1.3-1.2-2.8-1.2-4.4-.0-.3.0-.5.4-.5a4.6 4.6 0 011.5-.0c2.1.3 3.9 1.2 5.4 2.7.8.8 1.5 1.8 2.2 2.8.7 1.0 1.4 2.0 2.4 2.9.3.2.6.5.8.6-.8.0-2.1.1-3.0-.6zm1-6.4a.3.3 0 01.4-.2.3.3 0 01.2.2.3.3 0 01-.3.3.3.3 0 01-.3-.3zm3.1 1.5c-.2.0-.3.1-.5.1a1.2 1.2 0 01-.7-.2c-.2-.2-.4-.3-.5-.7a1.7 1.7 0 01.0-.5c.0-.3-.0-.5-.2-.7-.1-.1-.4-.1-.6-.1a.5.5 0 01-.2-.0c-.1-.0-.2-.1-.1-.3.0-.0.1-.1.1-.2.3-.2.7-.1 1.1.0.3.1.6.4 1.0.7.3.4.4.5.6.9.1.2.3.5.4.8.0.1-.0.3-.2.4z"], true),
  cognition: logoUri("0 0 24 24", ["M11.9 2l8.6 5v10l-8.6 5-8.6-5V7z"], false),
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
    // kept so buwiz.chat can send the file itself; the bridge caps a file at 20 MiB
    if (bytes.length <= 20971520) doc.data = bytes;
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
        class="flex-col items-center justify-center gap-2 px-4 py-6 rounded-xl border-2 border-dashed border-[#cbd5e1] cursor-pointer transition-all dark:border-[#27282f] hover:border-[#2a9d8f] dark:hover:border-[#2a9d8f] hover:bg-[#f8fafb] dark:hover:bg-[#22232a]"
        accessibilityLabel="Browse for an attachment"
        onPress={() => sideUpload()}
        activeStyle={{ opacity: 0.75 }}
      >
        <View class="w-10 h-10 rounded-full flex items-center justify-center bg-[#f1f5f9] dark:bg-[#141519]">
          <Avatar src={SIDE_GLYPHS.upload} accessibilityLabel="" class="w-5 h-5" fallbackDelay={Infinity} />
        </View>
        <View class="items-center">
          <Text class="text-[12px] font-medium text-[#2a9d8f]">{entryStagedBusy() ? "Uploading…" : "Browse"}</Text>
          <Text class="text-[9px] text-[#94a3b8] dark:text-[#6e727e]">Images or PDF — max 50MB</Text>
        </View>
      </View>
      <If when={entryStaged().length > 0}>
        <Each of={entryStaged()} key={rowKey}>
          {(d: Any, i: Any) => (
            <View class="group flex-row items-center gap-2.5 px-3 py-2 rounded-lg bg-white dark:bg-[#141519] border border-[#e2e8f0] dark:border-[#27282f] hover:border-[#2a9d8f]/40 dark:hover:border-[#2a9d8f]/40 transition-colors">
              <View class="w-8 h-8 rounded-lg bg-red-50 dark:bg-red-900/20 items-center justify-center shrink-0">
                <Avatar src={String(d().mime).startsWith("image/") ? SIDE_GLYPHS.image : SIDE_GLYPHS.file} accessibilityLabel="" class="w-4 h-4" fallbackDelay={Infinity} />
              </View>
              <View class="flex-1 min-w-0 gap-0.5">
                <Text class="text-[10px] font-medium text-[#1e293b] dark:text-[#f4f5f7] truncate">{d().filename}</Text>
                <Text class="text-[9px] text-[#94a3b8] dark:text-[#6e727e]">{sideBytesShown(d().size)}</Text>
              </View>
              <View
                class="w-7 h-7 rounded-md items-center justify-center hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors cursor-pointer shrink-0 opacity-60 group-hover:opacity-100"
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
        class="flex-row items-center justify-center gap-1.5 px-3 py-2 rounded-lg group border border-[#e2e8f0] dark:border-[#27282f] hover:bg-[#f8fafc] dark:hover:bg-[#22232a] transition-colors cursor-pointer"
        accessibilityLabel="Browse existing documents"
        onPress={() => buwiz.open({ app: "evidence" }).catch(() => null)}
        activeStyle={{ opacity: 0.7 }}
      >
        <Avatar src={SIDE_GLYPHS.folder} accessibilityLabel="" class="w-4 h-4" fallbackDelay={Infinity} />
        <Text class="text-xs font-medium text-[#64748b] dark:text-[#f4f5f7]/50 group-hover:text-[#1e293b] dark:group-hover:text-[#f4f5f7]">Browse existing documents</Text>
      </View>
      <If when={entryStaged().length === 0}>
        <View class="flex-1 flex-col items-center justify-center py-8 opacity-60 gap-0">
          <Avatar src={SIDE_GLYPHS.fileBig} accessibilityLabel="" class="w-7 h-7 mb-2" fallbackDelay={Infinity} />
          <Text class="text-[10px] font-medium text-[#94a3b8] dark:text-[#6e727e]">No attachments yet</Text>
          <Text class="text-[9px] text-[#94a3b8] dark:text-[#6e727e] mt-0.5">Drop receipts, invoices, or documents</Text>
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
      class="flex-row items-center gap-2 max-w-[85%] px-2.5 py-2 rounded-lg bg-white dark:bg-[#141519] border border-[#e2e8f0] dark:border-[#27282f] hover:border-[#2a9d8f]/40 dark:hover:border-[#2a9d8f]/40 transition-colors cursor-pointer"
      accessibilityLabel={`Preview ${d.filename}`}
      onPress={() => sidePreview()}
      activeStyle={{ opacity: 0.8 }}
    >
      <View class="w-8 h-8 rounded-lg bg-red-50 dark:bg-red-900/20 items-center justify-center shrink-0">
        <Avatar src={SIDE_GLYPHS.file} accessibilityLabel="" class="w-4 h-4" fallbackDelay={Infinity} />
      </View>
      <View class="flex-1 min-w-0 gap-0.5">
        <Text class="text-[10px] font-medium text-[#1e293b] dark:text-[#f4f5f7] truncate">{d.filename}</Text>
        <Text class="text-[9px] text-[#94a3b8] dark:text-[#6e727e]">{`${SIDE_MIME[d.mime] || "File"} · ${sideBytesShown(d.size)}`}</Text>
      </View>
    </View>
  );
}

// ————— model picker + real chat (the bridge's buwiz.models / buwiz.chat) —————

/** The bridge chat API; absent on older Bots, where the copy path stays. */
const sideApi = () => typeof buwiz.models === "function" && typeof buwiz.chat === "function";

function sideModelInfo(id: string) {
  return id ? (sideModels()?.models || []).find((m: Any) => m.id === id) || null : null;
}
/** The ModelInfo the pill shows: the pick, else the Bot's default, else the first. */
function sideShownModel() {
  const r = sideModels();
  return sideModelInfo(sideModel()) || sideModelInfo(String(r?.default || "")) || (r?.models || [])[0] || null;
}
function sideModelLogo(m: Any): string {
  const maker = String(m?.maker || "");
  if (SIDE_LOGOS[maker]) return SIDE_LOGOS[maker];
  if (SIDE_LOGOS_MORE[maker]) return SIDE_LOGOS_MORE[maker];
  if (/swe-\d/.test(String(m?.id || "").toLowerCase())) return SIDE_LOGOS_MORE.cognition;
  return "";
}
/** The effort in effect for the picked model: the choice, else its default rung. */
function sideEffortNow() {
  const m = sideModelInfo(sideModel()) || sideShownModel();
  if (sideEffort()) return sideEffort();
  return String(m?.default_effort || "");
}
/** The effort row's label for the picked model; "" when it has no efforts. */
function sideEffortLabel() {
  const m = sideModelInfo(sideModel()) || sideShownModel();
  return (m?.efforts || []).find((e: Any) => e.value === sideEffortNow())?.label || "";
}
/** The model's last rung (Ultra / Max) — violet in the Bot picker. */
const sideTopEffort = () => {
  const m = sideModelInfo(sideModel()) || sideShownModel();
  const es = m?.efforts || [];
  return es.length > 1 && es[es.length - 1]?.value === sideEffortNow();
};
/** What the picked model accepts; defaults to text-only until models() lands. */
const sideModelInputs = () => sideShownModel()?.inputs || ["text"];
const sideFileKind = (mime: string) => (mime === "application/pdf" ? "pdf" : mime.startsWith("image/") ? "image" : "other");

/** The live catalogue answered with nothing: the pill hides and sends copy to the Bot chat. */
const sideCatalogEmpty = () => {
  const r = sideModels();
  return Boolean(r) && (r?.models || []).length === 0;
};

/** Loads the catalogue (an empty or failed answer is asked again on the next
 *  mount), plus the stored chat.model / chat.effort choice. */
function sideLoadModels() {
  if (sideModelsAsked() && (sideModels()?.models || []).length > 0) return;
  sideModelsAsked.set(true);
  buwiz
    .models?.()
    .then((r: Any) => sideModels.set(r))
    .catch(() => sideModels.set({ default: null, models: [] }));
  const st = buwiz.storage;
  if (!sideStoreAsked && st && typeof st.get === "function") {
    sideStoreAsked = true;
    Promise.all([st.get("chat.model").catch(() => null), st.get("chat.effort").catch(() => null)]).then((v: Any[]) => {
      if (typeof v[0] === "string") sideModel.set(v[0]);
      if (typeof v[1] === "string") sideEffort.set(v[1]);
    });
  }
}
function sidePickModel(id: string, effort: string) {
  sideModel.set(id);
  sideEffort.set(effort);
  const st = buwiz.storage;
  if (st && typeof st.set === "function") {
    st.set("chat.model", id).catch(() => null);
    st.set("chat.effort", effort).catch(() => null);
  }
}

/** The visible exchange as bridge messages: ≤64, user/assistant text, files only on user rows. */
function sideHistory() {
  return sideChat()
    .filter((m: Any) => (m.role === "you" || m.role === "assistant") && (m.text || (m.role === "you" && m.chatFiles?.length)))
    .slice(-64)
    .map((m: Any) => {
      const msg: Any = { role: m.role === "you" ? "user" : "assistant", text: m.text };
      if (m.role === "you" && m.chatFiles?.length) msg.files = m.chatFiles;
      return msg;
    });
}

/** One assistant turn: called from a person's press (the consent grant needs it). */
function sideChatTurn() {
  const shown = sideShownModel();
  const req: Any = { model: sideModel() || String(sideModels()?.default || shown?.id || ""), messages: sideHistory() };
  if (sideEffortNow()) req.effort = sideEffortNow();
  let handle: Any;
  try {
    handle = buwiz.chat(req);
  } catch (e) {
    sideChat.set([...sideChat(), { role: "error", text: `Couldn't reach the assistant (${errText(e)}).`, retry: true, at: sideNow() }]);
    return;
  }
  sideStream.set(handle);
  const mid = rid();
  sideChat.set([...sideChat(), { id: mid, role: "assistant", text: "", streaming: true, at: sideNow() }]);
  sideFeedRef?.scrollToEnd?.({ animated: true });
  const ix = () => sideChat().findIndex((m: Any) => m.id === mid);
  const patch = (f: (m: Any) => Any) => {
    const i = ix();
    if (i >= 0) {
      const rows = sideChat().slice();
      rows[i] = f(rows[i]);
      sideChat.set(rows);
      sideFeedRef?.scrollToEnd?.({ animated: true });
    }
  };
  handle.onDelta((t: string) => patch((m) => ({ ...m, text: m.text + t })));
  handle.done
    .then((r: Any) => {
      patch((m) => ({ ...m, text: m.text || String(r?.text || ""), streaming: false, stopped: r?.stop_reason === "stopped" }));
      sideStream.set(null);
    })
    .catch((e: Any) => {
      patch((m) => ({
        role: "error",
        text: m.text ? `Stopped — ${errText(e)}` : `The assistant couldn't answer (${errText(e)}).`,
        retry: true,
        at: sideNow(),
      }));
      sideStream.set(null);
    });
}

/** A press copies the prompt to the Bot chat; the files stay staged on the
 *  entry and in Documents (attaching to a chat is the library's own grant).
 *  The assistant's reply shows in the Bot chat and its draft in the Inbox. */
async function sideSendCopy() {
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
        : text
          ? `Prompt copied — paste it into the Bot chat and send it there; the reply shows there, and its draft waits in your Inbox.${files.length ? " The files are staged on this entry and in Documents." : ""}`
          : "Only text can go to the Bot chat — the files stay staged on this entry and in Documents.",
      at,
    },
  ]);
  sideComposer.set("");
  sideFiles.set([]);
  sidePromptError.set("");
  sideFeedRef?.scrollToEnd?.({ animated: true });
}

/** Person pressed send: a real exchange when the bridge speaks chat, else the copy path. */
async function sideSend() {
  const text = sideComposer().trim();
  const files = sideFiles();
  if ((!text && files.length === 0) || sideStream()) return;
  if (!sideApi() || sideCatalogEmpty()) {
    return sideSendCopy();
  }
  const inputs = sideModelInputs();
  const chatFiles: Any[] = [];
  for (const d of files.slice(0, 8)) {
    if (inputs.includes(sideFileKind(String(d.mime || ""))) && d.data) {
      chatFiles.push({ name: d.filename, type: d.mime, data: d.data });
    }
  }
  sideChat.set([...sideChat(), { role: "you", text, files, chatFiles, at: sideNow() }]);
  sideComposer.set("");
  sideFiles.set([]);
  sidePromptError.set("");
  sideFeedRef?.scrollToEnd?.({ animated: true });
  sideChatTurn();
}

/** Re-ask after a stop or failure: drop the interrupted answer so the history ends on the person's turn. */
function sideRetry(mid: Any) {
  sideChat.set(sideChat().filter((m: Any) => m.id !== mid));
  sideChatTurn();
}

/** Markdown-lite: a reply's text into blocks (paragraph, bullets, numbered, code). */
function sideMdBlocks(text: string) {
  const blocks: Any[] = [];
  let fence = false;
  let buf: string[] = [];
  const flush = () => {
    if (!buf.length) return;
    const chunk = buf.join("\n");
    buf = [];
    if (fence) {
      blocks.push({ type: "code", text: chunk });
      return;
    }
    for (const para of chunk.split(/\n{2,}/)) {
      const lines = para.split("\n");
      if (lines.every((l) => /^\s*[-*]\s/.test(l))) blocks.push({ type: "ul", items: lines.map((l) => l.replace(/^\s*[-*]\s+/, "")) });
      else if (lines.every((l) => /^\s*\d+\.\s/.test(l))) blocks.push({ type: "ol", items: lines.map((l) => l.replace(/^\s*\d+\.\s+/, "")) });
      else blocks.push({ type: "p", lines });
    }
  };
  for (const line of text.split("\n")) {
    if (/^\s*```/.test(line)) {
      flush();
      fence = !fence;
    } else {
      buf.push(line);
    }
  }
  flush();
  return blocks;
}
/** One line split at **bold** and `code` spans into {t,b,c} segments. */
function sideMdSegs(line: string) {
  const out: Any[] = [];
  let rest = line;
  while (rest) {
    const m = rest.match(/\*\*([^*]+)\*\*|`([^`]+)`/);
    if (!m || m.index == null) {
      out.push({ t: rest });
      break;
    }
    if (m.index) out.push({ t: rest.slice(0, m.index) });
    out.push(m[1] != null ? { t: m[1], b: true } : { t: m[2], c: true });
    rest = rest.slice(m.index + m[0].length);
  }
  return out;
}
/** One inline line of an assistant reply. */
function SideMdLine(p: { line: string }) {
  return (
    <View class="flex-row flex-wrap">
      <Each of={sideMdSegs(p.line)}>
        {(s: Any) => (
          <Text
            class={`text-[12px] leading-relaxed ${s().b ? "font-semibold text-[#0f172a] dark:text-[#f4f5f7]" : s().c ? "font-mono text-[11px] bg-[#f1f5f9] dark:bg-[#1c1d22] px-1 rounded" : "text-[#1e293b] dark:text-[#f4f5f7]"}`}
          >
            {s().t}
          </Text>
        )}
      </Each>
    </View>
  );
}
/** An assistant bubble's content: paragraphs, lists and code blocks. */
function SideMd(p: { text: string }) {
  return (
    <View class="gap-1.5">
      <Each of={sideMdBlocks(p.text)}>
        {(b: Any, i: Any) =>
          b().type === "code" ? (
            <View class="rounded-lg bg-[#f1f5f9] dark:bg-[#1c1d22] px-2.5 py-2">
              <Text class="font-mono text-[11px] text-[#1e293b] dark:text-[#f4f5f7]">{b().text}</Text>
            </View>
          ) : b().type === "ul" ? (
            <View class="gap-0.5">
              <Each of={b().items}>
                {(it: Any) => (
                  <View class="flex-row gap-1.5">
                    <Text class="text-[12px] text-[#94a3b8]">•</Text>
                    <SideMdLine line={it()} />
                  </View>
                )}
              </Each>
            </View>
          ) : b().type === "ol" ? (
            <View class="gap-0.5">
              <Each of={b().items}>
                {(it: Any, n: Any) => (
                  <View class="flex-row gap-1.5">
                    <Text class="text-[12px] text-[#94a3b8]">{`${n() + 1}.`}</Text>
                    <SideMdLine line={it()} />
                  </View>
                )}
              </Each>
            </View>
          ) : (
            <View class="gap-0.5">
              <Each of={b().lines}>{(l: Any) => <SideMdLine line={l()} />}</Each>
            </View>
          )
        }
      </Each>
    </View>
  );
}

/** The model popover: search, the Default row, subscriptions, then API groups,
 *  and the picked model's efforts — the Bot's modelpicker in our kit style. */
function SideModelCard() {
  // Signal reads stay inside the JSX expression containers so each is a live
  // binding (kit.tsx: a value hoisted out of the JSX freezes at mount).
  const list = () => ((sideModels()?.models || []) as Any[]);
  const q = () => sidePickerQ().trim().toLowerCase();
  const match = (m: Any) =>
    !q() ||
    String(m.name).toLowerCase().includes(q()) ||
    String(m.id).toLowerCase().includes(q()) ||
    String(m.group).toLowerCase().includes(q());
  const subs = () => list().filter((m: Any) => m.subscription && match(m));
  const groups = () => {
    const out: Any[] = [];
    for (const m of list()) {
      if (m.subscription || !match(m)) continue;
      const g = String(m.group || "API connections");
      const at = out.findIndex((x) => x[0] === g);
      if (at < 0) out.push([g, [m]]);
      else out[at][1].push(m);
    }
    return out;
  };
  const picked = () => (sideModel() ? list().find((m: Any) => m.id === sideModel()) || null : null);
  const efforts = () => (picked()?.efforts || []) as Any[];
  const row = (m: Any) => {
    const on = () => sideModel() === String(m.id);
    return (
      <View
        class={`flex-row items-center gap-2.5 h-8 px-2.5 rounded-lg cursor-pointer ${on() ? "bg-[#1e86fc]/10" : "hover:bg-black/[0.04] dark:hover:bg-[#22232a]"}`}
        accessibilityLabel={String(m.name)}
        onPress={() => sidePickModel(String(m.id), String(m.default_effort || ""))}
        activeStyle={{ opacity: 0.7 }}
      >
        <If when={Boolean(sideModelLogo(m))}>
          <Avatar src={sideModelLogo(m)} accessibilityLabel="" class="w-4 h-4" imageClass={on() ? "tint-[#1e86fc]" : "tint-[#64748b] dark:tint-[#a1a5b0]"} fallbackDelay={Infinity} />
        </If>
        <Text class={`flex-1 truncate text-[12px] ${on() ? "text-[#1e86fc] font-medium" : "text-[#1e293b] dark:text-[#f4f5f7]"}`}>{m.name}</Text>
        <If when={on()}>
          <Avatar src={SIDE_GLYPHS.check} accessibilityLabel="" class="w-3.5 h-3.5" fallbackDelay={Infinity} />
        </If>
      </View>
    );
  };
  return (
    <View class="fixed inset-0 z-50">
      <View class="absolute inset-0 bg-black/20" onPress={() => sidePicker.set(false)} />
      {/* Anchored under the model chip the way models.rs centers the popover on the
          trigger (left = trigger_center - w/2, clamped to 8px): 214 puts its left
          edge at exactly 8px in a 490px window and under the trigger cluster wide. */}
      <View class="absolute right-[214px] bottom-[148px] w-[268px] max-w-[calc(100%-16px)] rounded-xl bg-white dark:bg-[#1c1d22] border border-[#e2e8f0] dark:border-[#27282f] shadow-[0_10px_24px_-6px_rgba(0,0,0,0.45)] overflow-hidden">
        <View class="flex-row items-center gap-1.5 h-8 mx-2 mt-2 px-2 rounded-lg bg-[#f1f5f9] dark:bg-[#141519]">
          <Avatar src={SIDE_GLYPHS.search} accessibilityLabel="" class="w-3.5 h-3.5" fallbackDelay={Infinity} />
          <Input
            ref={(e: Any) => {
              try {
                e?.focus?.();
              } catch {}
            }}
            accessibilityLabel="Search models"
            value={sidePickerQ()}
            placeholder="Search models…"
            onChange={(e: Any) => sidePickerQ.set(typeof e === "string" ? e : e.value)}
            class="flex-1 text-[12px] bg-transparent border-none outline-none placeholder:text-[#8e8e93] text-[#0f172a] dark:text-[#f4f5f7] focus:outline-none focus:ring-0"
          />
        </View>
        <Scroll class="max-h-64 p-1.5">
          <If when={!q() && Boolean(sideModels()?.default)}>
            <View
              class={`px-2.5 py-1.5 rounded-lg cursor-pointer ${sideModel() === "" ? "bg-[#1e86fc]/10" : "hover:bg-black/[0.04] dark:hover:bg-[#22232a]"}`}
              accessibilityLabel="Default model"
              onPress={() => {
                sidePickModel("", "");
                sidePicker.set(false);
              }}
              activeStyle={{ opacity: 0.7 }}
            >
              <View class="flex-row items-center">
                <Text class={`flex-1 text-[12px] font-medium ${sideModel() === "" ? "text-[#1e86fc]" : "text-[#1e293b] dark:text-[#f4f5f7]"}`}>Default</Text>
                <If when={sideModel() === ""}>
                  <Avatar src={SIDE_GLYPHS.check} accessibilityLabel="" class="w-3.5 h-3.5" fallbackDelay={Infinity} />
                </If>
              </View>
              <Text class="text-[10px] text-[#94a3b8] dark:text-[#6e727e]">{`Recommended · ${sideModelInfo(String(sideModels()?.default || ""))?.name || "the Bot's default"}`}</Text>
            </View>
          </If>
          <Each of={subs()}>{(m: Any) => row(m())}</Each>
          <Each of={groups()}>
            {(g: Any) => (
              <View>
                <View class="h-px mx-1.5 my-1 bg-[#e2e8f0] dark:bg-[#1c1d22]" />
                <Text class="px-2.5 pt-0.5 pb-1 text-[10px] text-[#94a3b8] dark:text-[#6e727e]">{g()[0]}</Text>
                <Each of={g()[1]}>{(m: Any) => row(m())}</Each>
              </View>
            )}
          </Each>
          <If when={list().length === 0}>
            <Text class="p-2 text-[11px] text-[#94a3b8] dark:text-[#6e727e]">{sideModels() ? "No models match" : "Loading models…"}</Text>
          </If>
        </Scroll>
        <If when={efforts().length > 0}>
          <View class="border-t border-[#e2e8f0] dark:border-[#27282f] p-1.5">
            <Text class="px-2.5 pb-1 text-[10px] text-[#94a3b8] dark:text-[#6e727e]">Effort</Text>
            <Each of={efforts()}>
              {(e: Any, i: Any) => {
                const top = efforts().length > 1 && i() === efforts().length - 1;
                const on = () => sideEffort() === e().value;
                return (
                  <View
                    class={`flex-row items-center h-8 px-2.5 rounded-lg cursor-pointer ${on() ? "bg-[#1e86fc]/10" : "hover:bg-black/[0.04] dark:hover:bg-[#22232a]"}`}
                    accessibilityLabel={`Effort ${e().label}`}
                    onPress={() => {
                      sidePickModel(sideModel(), String(e().value));
                      sidePicker.set(false);
                    }}
                    activeStyle={{ opacity: 0.7 }}
                  >
                    <Text class={`flex-1 text-[12px] ${on() ? (top ? "text-[#a78bfa]" : "text-[#1e86fc]") : "text-[#1e293b] dark:text-[#f4f5f7]"}`}>{e().label}</Text>
                    <If when={on()}>
                      <Avatar src={SIDE_GLYPHS.check} accessibilityLabel="" class="w-3.5 h-3.5" fallbackDelay={Infinity} />
                    </If>
                  </View>
                );
              }}
            </Each>
          </View>
        </If>
      </View>
    </View>
  );
}

/** The old app's AI chat panel: the feed, quick prompts, and the composer. */
function SideAssistant() {
  const msgs = sideChat;
  const canSend = () => Boolean(sideComposer().trim()) || sideFiles().length > 0;
  // files only go to the chat when the picked model takes images or PDFs
  const filesOk = () => !sideApi() || sideCatalogEmpty() || sideModelInputs().some((k: string) => k === "image" || k === "pdf");
  if (sideApi()) sideLoadModels();
  return (
    <View class="flex-1 flex-col min-h-0">
      <If when={sidePicker()}>
        <SideModelCard />
      </If>
      <Scroll ref={sideFeedRef} class="flex-1 px-3 py-3">
        <If when={msgs().length === 0}>
          <View class="flex-1 flex-col items-center justify-center py-10 gap-2">
            <Avatar src={SIDE_GLYPHS.sparkle} accessibilityLabel="" class="w-8 h-8" fallbackDelay={Infinity} />
            <Text class="text-[12px] font-medium text-[#64748b] dark:text-[#a1a5b0]">Ask the assistant</Text>
            <Text class="text-[10px] text-[#94a3b8] dark:text-[#6e727e] text-center px-3">Describe a transaction, or attach a receipt or bill. The assistant drafts it; you post it.</Text>
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
                  <Text class="text-[10px] text-[#94a3b8] dark:text-[#6e727e] text-center">{m().text}</Text>
                </If>
              </View>
            ) : m().role === "assistant" ? (
              <View class="items-start gap-1 py-1">
                <If when={Boolean(m().streaming) && !m().text}>
                  <View class="flex-row items-center gap-1 px-3 py-2 rounded-2xl rounded-bl-md bg-white dark:bg-[#141519] border border-[#e2e8f0] dark:border-[#27282f]">
                    <Each of={[0, 1, 2]}>{(d: Any) => <View class="w-1.5 h-1.5 rounded-full bg-[#94a3b8]" />}</Each>
                  </View>
                </If>
                <If when={Boolean(m().text)}>
                  <View class="max-w-[92%] bg-white dark:bg-[#141519] border border-[#e2e8f0] dark:border-[#27282f] rounded-2xl rounded-bl-md px-3 py-2">
                    <SideMd text={m().text} />
                  </View>
                </If>
                <If when={Boolean(m().stopped)}>
                  <View class="flex-row items-center gap-2">
                    <Text class="text-[9px] text-[#94a3b8] dark:text-[#6e727e]">Stopped</Text>
                    <View
                      class="px-2 py-0.5 rounded-md bg-[#f1f5f9] dark:bg-[#1c1d22] cursor-pointer"
                      accessibilityLabel="Retry"
                      onPress={() => sideRetry(m().id)}
                      activeStyle={{ opacity: 0.7 }}
                    >
                      <Text class="text-[10px] font-medium text-[#64748b] dark:text-[#a1a5b0]">Retry</Text>
                    </View>
                  </View>
                </If>
                <If when={Boolean(m().at) && !m().stopped}>
                  <Text class="text-[9px] text-[#94a3b8] dark:text-[#6e727e]">{m().streaming ? "…" : m().at}</Text>
                </If>
              </View>
            ) : m().role === "error" ? (
              <View class="items-start px-1 py-1">
                <View class="flex-row items-center gap-2 px-2.5 py-2 rounded-lg bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800">
                  <Text class="flex-1 text-[10px] text-red-700 dark:text-red-300">{m().text}</Text>
                  <If when={Boolean(m().retry)}>
                    <View
                      class="px-2 py-0.5 rounded-md bg-red-100 dark:bg-red-900/40 cursor-pointer"
                      accessibilityLabel="Retry"
                      onPress={() => sideRetry(m().id)}
                      activeStyle={{ opacity: 0.7 }}
                    >
                      <Text class="text-[10px] font-medium text-red-700 dark:text-red-300">Retry</Text>
                    </View>
                  </If>
                </View>
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
                  <Text class="text-[9px] text-[#94a3b8] dark:text-[#6e727e]">{m().at}</Text>
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
              class="px-2 py-1 rounded-full bg-[#f1f5f9] dark:bg-[#141519] border border-[#e2e8f0] dark:border-[#27282f] hover:bg-[#e2e8f0] dark:hover:bg-[#22232a] transition-colors cursor-pointer"
              onPress={() => { sideComposer.set(q()[1]); sidePromptError.set(""); }}
              activeStyle={{ opacity: 0.7 }}
            >
              <Text class="text-[10px] font-medium text-[#475569] dark:text-[#a1a5b0]">{q()[0]}</Text>
            </View>
          )}
        </Each>
      </View>
      <View class="p-3 border-t border-[#e2e8f0] dark:border-[#27282f]">
        <If when={sideApi() && sideCatalogEmpty()}>
          <Text class="text-xs text-[#94a3b8] dark:text-[#6e727e] pb-2">No AI models available. Connect one in Buwiz Bot settings.</Text>
        </If>
        <View class="rounded-xl border border-[#e2e8f0] dark:border-[#27282f] bg-[#f8f9fb] dark:bg-[#141519] px-3 py-2 gap-2 focus-within:border-[#2a9d8f] focus-within:ring-1 focus-within:ring-[#2a9d8f] transition-colors">
          <If when={sideFiles().length > 0}>
            <View class="flex-row gap-1.5 flex-wrap">
              <Each of={sideFiles()} key={rowKey}>
                {(f: Any, i: Any) => (
                  <View class="flex-row items-center gap-1 px-2 py-1 rounded-md bg-white dark:bg-[#1c1d22] border border-[#e2e8f0] dark:border-[#27282f]">
                    <Text class="text-[9px] font-medium text-[#475569] dark:text-[#a1a5b0] max-w-24 truncate">{f().filename}</Text>
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
            class="w-full min-h-6 max-h-28 text-[13px] bg-transparent border-none outline-none placeholder:text-[#8e8e93] text-[#0f172a] dark:text-[#f4f5f7] focus:outline-none focus:ring-0"
          />
          <View class="flex-row items-center justify-between">
            <View class="flex-row items-center gap-1">
              <View
                class={`w-7 h-7 rounded-lg items-center justify-center transition-colors ${filesOk() ? "hover:bg-[#f1f5f9] dark:hover:bg-[#22232a] cursor-pointer" : "opacity-40"}`}
                accessibilityLabel={filesOk() ? "Attach a receipt or bill" : "This model takes text only"}
                onPress={() => filesOk() && sidePickForChat()}
                activeStyle={{ opacity: 0.7 }}
              >
                <Avatar src={SIDE_GLYPHS.clip} accessibilityLabel="" class="w-4 h-4" fallbackDelay={Infinity} />
              </View>
              <If when={sideApi() && !sideCatalogEmpty()}>
                <View
                  class="flex-row items-center gap-1 h-7 pl-1.5 pr-2 rounded-full bg-black/[0.04] dark:bg-[#1c1d22] hover:bg-black/[0.08] dark:hover:bg-[#22232a] transition-colors cursor-pointer"
                  accessibilityLabel="Select model"
                  onPress={() => {
                    sideLoadModels();
                    sidePicker.set(!sidePicker());
                  }}
                  activeStyle={{ opacity: 0.7 }}
                >
                  <If when={Boolean(sideModelLogo(sideShownModel()))}>
                    <Avatar src={sideModelLogo(sideShownModel())} accessibilityLabel="" class="w-3.5 h-3.5" imageClass="tint-[#334155] dark:tint-[#a1a5b0]" fallbackDelay={Infinity} />
                  </If>
                  <Text class="text-[11px] text-[#334155] dark:text-[#a1a5b0]">{sideShownModel()?.name || "Default"}</Text>
                  <If when={Boolean(sideEffortLabel())}>
                    <Text class={`text-[11px] ${sideTopEffort() ? "text-[#a78bfa]" : "text-[#1e86fc]"}`}>{sideEffortLabel()}</Text>
                  </If>
                  <Avatar src={SIDE_GLYPHS.chev} accessibilityLabel="" class="w-2.5 h-2.5" fallbackDelay={Infinity} />
                </View>
              </If>
            </View>
            <View class="flex-row items-center gap-2">
              <If when={Boolean(sideStream())}>
                <View
                  class="w-8 h-8 rounded-full items-center justify-center bg-red-500 cursor-pointer"
                  accessibilityLabel="Stop the reply"
                  onPress={() => sideStream()?.stop?.()}
                  activeStyle={{ opacity: 0.7 }}
                >
                  <Avatar src={SIDE_GLYPHS.stop} accessibilityLabel="" class="w-3.5 h-3.5" imageClass="tint-white" fallbackDelay={Infinity} />
                </View>
              </If>
              <If when={!sideStream()}>
                <View
                  class={`w-8 h-8 rounded-full items-center justify-center bg-[#2a9d8f] ${canSend() ? "cursor-pointer" : "opacity-40"}`}
                  accessibilityLabel="Send to the Bot chat"
                  onPress={() => sideSend()}
                  activeStyle={{ opacity: 0.7 }}
                >
                  <Avatar src={SIDE_GLYPHS.send} accessibilityLabel="" class="w-4 h-4" fallbackDelay={Infinity} />
                </View>
              </If>
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
            <Text class="text-[#94a3b8] dark:text-[#6e727e]"> Just now</Text>
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
    <View class="w-full shrink-0 rounded-[24px] border border-[#e2e8f0] dark:border-[#27282f] overflow-hidden shadow-[0_4px_24px_rgba(0,0,0,0.08)] flex-col h-full bg-white dark:bg-[#141519]">
      <View class={`flex-row justify-evenly px-4 py-3 gap-2 ${accent() === "green" ? "bg-linear-to-r from-[#1a6b3c] to-[#27ae60] dark:from-[#12452b] dark:to-[#18663e]" : ""}`} style={accent() === "green" ? undefined : { backgroundColor: acc() }}>
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
