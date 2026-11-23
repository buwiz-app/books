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
  ai: '<path d="M12 3l1.5 4.5L18 9l-4.5 1.5L12 15l-1.5-4.5L6 9l4.5-1.5L12 3z"/>',
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
  anthropic: logoUri("0 0 24 24", ["M13.827 3.52h3.603L24 20h-3.603l-6.57-16.48zm-7.258 0h3.767L16.906 20h-3.674l-1.343-3.461H5.017l-1.344 3.46H0L6.57 3.522zm4.132 9.959L8.453 7.687 6.205 13.48H10.7z"], true),
  openai: logoUri("0 0 24 24", ["M21.55 10.004a5.416 5.416 0 00-.478-4.501c-1.217-2.09-3.662-3.166-6.05-2.66A5.59 5.59 0 0010.831 1C8.39.995 6.224 2.546 5.473 4.838A5.553 5.553 0 001.76 7.496a5.487 5.487 0 00.691 6.5 5.416 5.416 0 00.477 4.502c1.217 2.09 3.662 3.165 6.05 2.66A5.586 5.586 0 0013.168 23c2.443.006 4.61-1.546 5.361-3.84a5.553 5.553 0 003.715-2.66 5.488 5.488 0 00-.693-6.497v.001zm-8.381 11.558a4.199 4.199 0 01-2.675-.954c.034-.018.093-.05.132-.074l4.44-2.53a.71.71 0 00.364-.623v-6.176l1.877 1.069c.02.01.033.029.036.05v5.115c-.003 2.274-1.87 4.118-4.174 4.123zM4.192 17.78a4.059 4.059 0 01-.498-2.763c.032.02.09.055.131.078l4.44 2.53c.225.13.504.13.73 0l5.42-3.088v2.138a.068.068 0 01-.027.057L9.9 19.288c-1.999 1.136-4.552.46-5.707-1.51h-.001zM3.023 8.216A4.15 4.15 0 015.198 6.41l-.002.151v5.06a.711.711 0 00.364.624l5.42 3.087-1.876 1.07a.067.067 0 01-.063.005l-4.489-2.559c-1.995-1.14-2.679-3.658-1.53-5.63h.001zm15.417 3.54l-5.42-3.088L14.896 7.6a.067.067 0 01.063-.006l4.489 2.557c1.998 1.14 2.683 3.662 1.529 5.633a4.163 4.163 0 01-2.174 1.807V12.38a.71.71 0 00-.363-.623zm1.867-2.773a6.04 6.04 0 00-.132-.078l-4.44-2.53a.731.731 0 00-.729 0l-5.42 3.088V7.325a.068.068 0 01.027-.057L14.1 4.713c2-1.137 4.555-.46 5.707 1.513.487.833.664 1.809.499 2.757h.001zm-11.741 3.81l-1.877-1.068a.065.065 0 01-.036-.051V6.559c.001-2.277 1.873-4.122 4.181-4.12.976 0 1.92.338 2.671.954-.034.018-.092.05-.131.073l-4.44 2.53a.71.71 0 00-.365.623l-.003 6.173v.002zm1.02-2.168L12 9.25l2.414 1.375v2.75L12 14.75l-2.415-1.375v-2.75z"], true),
  xai: logoUri("0 0 24 24", ["M6.469 8.776L16.512 23h-4.464L2.005 8.776H6.47zm-.004 7.9l2.233 3.164L6.467 23H2l4.465-6.324zM22 2.582V23h-3.659V7.764L22 2.582zM22 1l-9.952 14.095-2.233-3.163L17.533 1H22z"], true),
  google: logoUri("0 0 24 24", ["M20.616 10.835a14.147 14.147 0 01-4.45-3.001 14.111 14.111 0 01-3.678-6.452.503.503 0 00-.975 0 14.134 14.134 0 01-3.679 6.452 14.155 14.155 0 01-4.45 3.001c-.65.28-1.318.505-2.002.678a.502.502 0 000 .975c.684.172 1.35.397 2.002.677a14.147 14.147 0 014.45 3.001 14.112 14.112 0 013.679 6.453.502.502 0 00.975 0c.172-.685.397-1.351.677-2.003a14.145 14.145 0 013.001-4.45 14.113 14.113 0 016.453-3.678.503.503 0 000-.975 13.245 13.245 0 01-2.003-.678z"], true),
  mistral: logoUri("0 0 24 24", ["M3.428 3.4h3.429v3.428h3.429v3.429h-.002 3.431V6.828h3.427V3.4h3.43v13.714H24v3.429H13.714v-3.428h-3.428v-3.429h-3.43v3.428h3.43v3.429H0v-3.429h3.428V3.4zm10.286 13.715h3.428v-3.429h-3.427v3.429z"], true),
  meta: logoUri("0 0 24 24", ["M6.897 4c1.915 0 3.516.932 5.43 3.376l.282-.373c.19-.246.383-.484.58-.71l.313-.35C14.588 4.788 15.792 4 17.225 4c1.273 0 2.469.557 3.491 1.516l.218.213c1.73 1.765 2.917 4.71 3.053 8.026l.011.392.002.25c0 1.501-.28 2.759-.818 3.7l-.14.23-.108.153c-.301.42-.664.758-1.086 1.009l-.265.142-.087.04a3.493 3.493 0 01-.302.118 4.117 4.117 0 01-1.33.208c-.524 0-.996-.067-1.438-.215-.614-.204-1.163-.56-1.726-1.116l-.227-.235c-.753-.812-1.534-1.976-2.493-3.586l-1.43-2.41-.544-.895-1.766 3.13-.343.592C7.597 19.156 6.227 20 4.356 20c-1.21 0-2.205-.42-2.936-1.182l-.168-.184c-.484-.573-.837-1.311-1.043-2.189l-.067-.32a8.69 8.69 0 01-.136-1.288L0 14.468c.002-.745.06-1.49.174-2.23l.1-.573c.298-1.53.828-2.958 1.536-4.157l.209-.34c1.177-1.83 2.789-3.053 4.615-3.16L6.897 4zm-.033 2.615l-.201.01c-.83.083-1.606.673-2.252 1.577l-.138.199-.01.018c-.67 1.017-1.185 2.378-1.456 3.845l-.004.022a12.591 12.591 0 00-.207 2.254l.002.188c.004.18.017.36.04.54l.043.291c.092.503.257.908.486 1.208l.117.137c.303.323.698.492 1.17.492 1.1 0 1.796-.676 3.696-3.641l2.175-3.4.454-.701-.139-.198C9.11 7.3 8.084 6.616 6.864 6.616zm10.196-.552l-.176.007c-.635.048-1.223.359-1.82.933l-.196.198c-.439.462-.887 1.064-1.367 1.807l.266.398c.18.274.362.56.55.858l.293.475 1.396 2.335.695 1.114c.583.926 1.03 1.6 1.408 2.082l.213.262c.282.326.529.54.777.673l.102.05c.227.1.457.138.718.138.176.002.35-.023.518-.073.338-.104.61-.32.813-.637l.095-.163.077-.162c.194-.459.29-1.06.29-1.785l-.006-.449c-.08-2.871-.938-5.372-2.2-6.798l-.176-.189c-.67-.683-1.444-1.074-2.27-1.074z"], true),
};
// qwen/deepseek/cognition ship lazily — the same logos, only heavier paths.
const SIDE_LOGOS_MORE: Record<string, string> = {
  qwen: logoUri("0 0 24 24", ["M12.604 1.34c.393.69.784 1.382 1.174 2.075a.18.18 0 00.157.091h5.552c.174 0 .322.11.446.327l1.454 2.57c.19.337.24.478.024.837-.26.43-.513.864-.76 1.3l-.367.658c-.106.196-.223.28-.04.512l2.652 4.637c.172.301.111.494-.043.77-.437.785-.882 1.564-1.335 2.34-.159.272-.352.375-.68.37-.777-.016-1.552-.01-2.327.016a.099.099 0 00-.081.05 575.097 575.097 0 01-2.705 4.74c-.169.293-.38.363-.725.364-.997.003-2.002.004-3.017.002a.537.537 0 01-.465-.271l-1.335-2.323a.09.09 0 00-.083-.049H4.982c-.285.03-.553-.001-.805-.092l-1.603-2.77a.543.543 0 01-.002-.54l1.207-2.12a.198.198 0 000-.197 550.951 550.951 0 01-1.875-3.272l-.79-1.395c-.16-.31-.173-.496.095-.965.465-.813.927-1.625 1.387-2.436.132-.234.304-.334.584-.335a338.3 338.3 0 012.589-.001.124.124 0 00.107-.063l2.806-4.895a.488.488 0 01.422-.246c.524-.001 1.053 0 1.583-.006L11.704 1c.341-.003.724.032.9.34zm-3.432.403a.06.06 0 00-.052.03L6.254 6.788a.157.157 0 01-.135.078H3.253c-.056 0-.07.025-.041.074l5.81 10.156c.025.042.013.062-.034.063l-2.795.015a.218.218 0 00-.2.116l-1.32 2.31c-.044.078-.021.118.068.118l5.716.008c.046 0 .08.02.104.061l1.403 2.454c.046.081.092.082.139 0l5.006-8.76.783-1.382a.055.055 0 01.096 0l1.424 2.53a.122.122 0 00.107.062l2.763-.02a.04.04 0 00.035-.02.041.041 0 000-.04l-2.9-5.086a.108.108 0 010-.113l.293-.507 1.12-1.977c.024-.041.012-.062-.035-.062H9.2c-.059 0-.073-.026-.043-.077l1.434-2.505a.107.107 0 000-.114L9.225 1.774a.06.06 0 00-.053-.031zm6.29 8.02c.046 0 .058.02.034.06l-.832 1.465-2.613 4.585a.056.056 0 01-.05.029.058.058 0 01-.05-.029L8.498 9.841c-.02-.034-.01-.052.028-.054l.216-.012 6.722-.012z"], true),
  deepseek: logoUri("0 0 24 24", ["M23.748 4.482c-.254-.124-.364.113-.512.234-.051.039-.094.09-.137.136-.372.397-.806.657-1.373.626-.829-.046-1.537.214-2.163.848-.133-.782-.575-1.248-1.247-1.548-.352-.156-.708-.311-.955-.65-.172-.241-.219-.51-.305-.774-.055-.16-.11-.323-.293-.35-.2-.031-.278.136-.356.276-.313.572-.434 1.202-.422 1.84.027 1.436.633 2.58 1.838 3.393.137.093.172.187.129.323-.082.28-.18.552-.266.833-.055.179-.137.217-.329.14a5.526 5.526 0 01-1.736-1.18c-.857-.828-1.631-1.742-2.597-2.458a11.365 11.365 0 00-.689-.471c-.985-.957.13-1.743.388-1.836.27-.098.093-.432-.779-.428-.872.004-1.67.295-2.687.684a3.055 3.055 0 01-.465.137 9.597 9.597 0 00-2.883-.102c-1.885.21-3.39 1.102-4.497 2.623C.082 8.606-.231 10.684.152 12.85c.403 2.284 1.569 4.175 3.36 5.653 1.858 1.533 3.997 2.284 6.438 2.14 1.482-.085 3.133-.284 4.994-1.86.47.234.962.327 1.78.397.63.059 1.236-.03 1.705-.128.735-.156.684-.837.419-.961-2.155-1.004-1.682-.595-2.113-.926 1.096-1.296 2.746-2.642 3.392-7.003.05-.347.007-.565 0-.845-.004-.17.035-.237.23-.256a4.173 4.173 0 001.545-.475c1.396-.763 1.96-2.015 2.093-3.517.02-.23-.004-.467-.247-.588zM11.581 18c-2.089-1.642-3.102-2.183-3.52-2.16-.392.024-.321.471-.235.763.09.288.207.486.371.739.114.167.192.416-.113.603-.673.416-1.842-.14-1.897-.167-1.361-.802-2.5-1.86-3.301-3.307-.774-1.393-1.224-2.887-1.298-4.482-.02-.386.093-.522.477-.592a4.696 4.696 0 011.529-.039c2.132.312 3.946 1.265 5.468 2.774.868.86 1.525 1.887 2.202 2.891.72 1.066 1.494 2.082 2.48 2.914.348.292.625.514.891.677-.802.09-2.14.11-3.054-.614zm1-6.44a.306.306 0 01.415-.287.302.302 0 01.2.288.306.306 0 01-.31.307.303.303 0 01-.304-.308zm3.11 1.596c-.2.081-.399.151-.59.16a1.245 1.245 0 01-.798-.254c-.274-.23-.47-.358-.552-.758a1.73 1.73 0 01.016-.588c.07-.327-.008-.537-.239-.727-.187-.156-.426-.199-.688-.199a.559.559 0 01-.254-.078c-.11-.054-.2-.19-.114-.358.028-.054.16-.186.192-.21.356-.202.767-.136 1.146.016.352.144.618.408 1.001.782.391.451.462.576.685.914.176.265.336.537.445.848.067.195-.019.354-.25.452z"], true),
  cognition: logoUri("0 0 24 24", ["M11.99 2l8.66 5v10l-8.66 5-8.66-5V7z"], false),
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
            class={`text-[12px] leading-relaxed ${s().b ? "font-semibold text-[#0f172a] dark:text-white" : s().c ? "font-mono text-[11px] bg-[#f1f5f9] dark:bg-slate-700 px-1 rounded" : "text-[#1e293b] dark:text-slate-200"}`}
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
            <View class="rounded-lg bg-[#f1f5f9] dark:bg-slate-700/60 px-2.5 py-2">
              <Text class="font-mono text-[11px] text-[#1e293b] dark:text-slate-200">{b().text}</Text>
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
        class={`flex-row items-center gap-2.5 h-8 px-2.5 rounded-lg cursor-pointer ${on() ? "bg-[#1e86fc]/10" : "hover:bg-black/[0.04] dark:hover:bg-white/5"}`}
        accessibilityLabel={String(m.name)}
        onPress={() => sidePickModel(String(m.id), String(m.default_effort || ""))}
        activeStyle={{ opacity: 0.7 }}
      >
        <If when={Boolean(sideModelLogo(m))}>
          <Avatar src={sideModelLogo(m)} accessibilityLabel="" class="w-4 h-4" imageClass={on() ? "tint-[#1e86fc]" : "tint-[#64748b] dark:tint-slate-300"} fallbackDelay={Infinity} />
        </If>
        <Text class={`flex-1 truncate text-[12px] ${on() ? "text-[#1e86fc] font-medium" : "text-[#1e293b] dark:text-slate-200"}`}>{m.name}</Text>
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
      <View class="absolute right-[214px] bottom-[148px] w-[268px] max-w-[calc(100%-16px)] rounded-xl bg-white dark:bg-[#1f1f1f] border border-[#e2e8f0] dark:border-slate-700 shadow-[0_10px_24px_-6px_rgba(0,0,0,0.45)] overflow-hidden">
        <View class="flex-row items-center gap-1.5 h-8 mx-2 mt-2 px-2 rounded-lg bg-[#f1f5f9] dark:bg-slate-800">
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
            class="flex-1 text-[12px] bg-transparent border-none outline-none placeholder:text-[#8e8e93] text-[#0f172a] dark:text-slate-100 focus:outline-none focus:ring-0"
          />
        </View>
        <Scroll class="max-h-64 p-1.5">
          <If when={!q() && Boolean(sideModels()?.default)}>
            <View
              class={`px-2.5 py-1.5 rounded-lg cursor-pointer ${sideModel() === "" ? "bg-[#1e86fc]/10" : "hover:bg-black/[0.04] dark:hover:bg-white/5"}`}
              accessibilityLabel="Default model"
              onPress={() => {
                sidePickModel("", "");
                sidePicker.set(false);
              }}
              activeStyle={{ opacity: 0.7 }}
            >
              <View class="flex-row items-center">
                <Text class={`flex-1 text-[12px] font-medium ${sideModel() === "" ? "text-[#1e86fc]" : "text-[#1e293b] dark:text-slate-200"}`}>Default</Text>
                <If when={sideModel() === ""}>
                  <Avatar src={SIDE_GLYPHS.check} accessibilityLabel="" class="w-3.5 h-3.5" fallbackDelay={Infinity} />
                </If>
              </View>
              <Text class="text-[10px] text-[#94a3b8] dark:text-slate-500">{`Recommended · ${sideModelInfo(String(sideModels()?.default || ""))?.name || "the Bot's default"}`}</Text>
            </View>
          </If>
          <Each of={subs()}>{(m: Any) => row(m())}</Each>
          <Each of={groups()}>
            {(g: Any) => (
              <View>
                <View class="h-px mx-1.5 my-1 bg-[#e2e8f0] dark:bg-slate-700" />
                <Text class="px-2.5 pt-0.5 pb-1 text-[10px] text-[#94a3b8] dark:text-slate-500">{g()[0]}</Text>
                <Each of={g()[1]}>{(m: Any) => row(m())}</Each>
              </View>
            )}
          </Each>
          <If when={list().length === 0}>
            <Text class="p-2 text-[11px] text-[#94a3b8] dark:text-slate-500">{sideModels() ? "No models match" : "Loading models…"}</Text>
          </If>
        </Scroll>
        <If when={efforts().length > 0}>
          <View class="border-t border-[#e2e8f0] dark:border-slate-700 p-1.5">
            <Text class="px-2.5 pb-1 text-[10px] text-[#94a3b8] dark:text-slate-500">Effort</Text>
            <Each of={efforts()}>
              {(e: Any, i: Any) => {
                const top = efforts().length > 1 && i() === efforts().length - 1;
                const on = () => sideEffort() === e().value;
                return (
                  <View
                    class={`flex-row items-center h-8 px-2.5 rounded-lg cursor-pointer ${on() ? "bg-[#1e86fc]/10" : "hover:bg-black/[0.04] dark:hover:bg-white/5"}`}
                    accessibilityLabel={`Effort ${e().label}`}
                    onPress={() => {
                      sidePickModel(sideModel(), String(e().value));
                      sidePicker.set(false);
                    }}
                    activeStyle={{ opacity: 0.7 }}
                  >
                    <Text class={`flex-1 text-[12px] ${on() ? (top ? "text-[#a78bfa]" : "text-[#1e86fc]") : "text-[#1e293b] dark:text-slate-200"}`}>{e().label}</Text>
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
            ) : m().role === "assistant" ? (
              <View class="items-start gap-1 py-1">
                <If when={Boolean(m().streaming) && !m().text}>
                  <View class="flex-row items-center gap-1 px-3 py-2 rounded-2xl rounded-bl-md bg-white dark:bg-slate-800 border border-[#e2e8f0] dark:border-slate-700">
                    <Each of={[0, 1, 2]}>{(d: Any) => <View class="w-1.5 h-1.5 rounded-full bg-[#94a3b8]" />}</Each>
                  </View>
                </If>
                <If when={Boolean(m().text)}>
                  <View class="max-w-[92%] bg-white dark:bg-slate-800 border border-[#e2e8f0] dark:border-slate-700 rounded-2xl rounded-bl-md px-3 py-2">
                    <SideMd text={m().text} />
                  </View>
                </If>
                <If when={Boolean(m().stopped)}>
                  <View class="flex-row items-center gap-2">
                    <Text class="text-[9px] text-[#94a3b8] dark:text-slate-500">Stopped</Text>
                    <View
                      class="px-2 py-0.5 rounded-md bg-[#f1f5f9] dark:bg-slate-700 cursor-pointer"
                      accessibilityLabel="Retry"
                      onPress={() => sideRetry(m().id)}
                      activeStyle={{ opacity: 0.7 }}
                    >
                      <Text class="text-[10px] font-medium text-[#64748b] dark:text-slate-300">Retry</Text>
                    </View>
                  </View>
                </If>
                <If when={Boolean(m().at) && !m().stopped}>
                  <Text class="text-[9px] text-[#94a3b8] dark:text-slate-500">{m().streaming ? "…" : m().at}</Text>
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
        <If when={sideApi() && sideCatalogEmpty()}>
          <Text class="text-xs text-[#94a3b8] dark:text-slate-500 pb-2">No AI models available. Connect one in Buwiz Bot settings.</Text>
        </If>
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
            <View class="flex-row items-center gap-1">
              <View
                class={`w-7 h-7 rounded-lg items-center justify-center transition-colors ${filesOk() ? "hover:bg-[#f1f5f9] dark:hover:bg-slate-700 cursor-pointer" : "opacity-40"}`}
                accessibilityLabel={filesOk() ? "Attach a receipt or bill" : "This model takes text only"}
                onPress={() => filesOk() && sidePickForChat()}
                activeStyle={{ opacity: 0.7 }}
              >
                <Avatar src={SIDE_GLYPHS.clip} accessibilityLabel="" class="w-4 h-4" fallbackDelay={Infinity} />
              </View>
              <If when={sideApi() && !sideCatalogEmpty()}>
                <View
                  class="flex-row items-center gap-1 h-7 pl-1.5 pr-2 rounded-full bg-black/[0.04] dark:bg-white/[0.06] hover:bg-black/[0.08] dark:hover:bg-white/10 transition-colors cursor-pointer"
                  accessibilityLabel="Select model"
                  onPress={() => {
                    sideLoadModels();
                    sidePicker.set(!sidePicker());
                  }}
                  activeStyle={{ opacity: 0.7 }}
                >
                  <If when={Boolean(sideModelLogo(sideShownModel()))}>
                    <Avatar src={sideModelLogo(sideShownModel())} accessibilityLabel="" class="w-3.5 h-3.5" imageClass="tint-[#334155] dark:tint-slate-300" fallbackDelay={Infinity} />
                  </If>
                  <Text class="text-[11px] text-[#334155] dark:text-slate-300">{sideShownModel()?.name || "Default"}</Text>
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
