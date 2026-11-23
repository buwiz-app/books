// Buwiz Books, the window (app `books`). This file is the kit every page
// shares; the plugin serves kit, pages and window joined into one source
// (crates/books-mini-app/src/apps.rs). Imports only nsh and buwiz (protocol 1.0 §11.2);
// data comes from buwiz.call alone, and every tool called is in the app's
// permissions.tools. Money arrives as display text from Books: the app
// never does arithmetic on an amount (invariant 2).
import { signal, computed, resource, Text, View, Scroll, Input, Overlay } from "nsh";
import { Avatar, Combobox, DatePicker } from "nsh/base";
import { buwiz } from "buwiz";

type Any = any;
const PROPS = (buwiz.props || {}) as { ticket?: string; page?: string; account?: string; month?: string; filter?: string; form?: string; tax_option?: string; deduction?: string };

const MONTH_NAMES = [
  "January", "February", "March", "April", "May", "June",
  "July", "August", "September", "October", "November", "December",
];

/** "2026-10-29" → "October 29, 2026", for display only: the stored value
 *  stays ISO (the old web app's formatDisplay, without a locale API). */
const longDate = (iso: string) => {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(iso || "");
  if (!m) return iso;
  return `${MONTH_NAMES[Number(m[2]) - 1] || m[2]} ${String(Number(m[3]))}, ${m[1]}`;
};

/** Colors: a dark palette, with a light twin for each. */
const C = {
  bg: "bg-white dark:bg-[#0a0a0b]",
  rail: "bg-[#f1f5f9] dark:bg-[#141416]",
  panel: "bg-white dark:bg-[#141416]",
  raise: "bg-[#f1f5f9] dark:bg-[#1c1c1f]",
  hover: "bg-[#f8fafc] dark:bg-[#1c1c1f]",
  line: "border-[#e2e8f0] dark:border-[#2a2a2d]",
  ink: "text-[#0f172a] dark:text-[#fafafa]",
  mute: "text-[#475569] dark:text-[#a1a1aa]",
  dim: "text-[#94a3b8] dark:text-[#71717a]",
  green: "text-[#15803d] dark:text-[#22c55e]",
  greenBg: "bg-[#16a34a] dark:bg-[#22c55e]",
  red: "text-[#ef4444] dark:text-[#ff7d75]",
  redBg: "bg-[#ef4444] dark:bg-[#ff7d75]",
  amber: "text-[#f59e0b] dark:text-[#f3b64c]",
  blue: "text-[#2f6fd6] dark:text-[#7fb2ff]",
  blueBg: "bg-[#2f6fd6] dark:bg-[#7fb2ff]",
  violet: "text-[#6a46c9] dark:text-[#b79bff]",
  violetBg: "bg-[#6a46c9] dark:bg-[#b79bff]",
};

/** Old-app icons as data-URI Avatars: the path verbatim, drawn `white` and
 *  recoloured by `tint-*` (same pattern as entry_side.tsx's sideSvg).
 *  `xmlns` is left out on purpose: the privacy gate rejects URL literals. */
const svgIcon = (inner: string, width: number) =>
  `data:image/svg+xml,${encodeURIComponent(
    `<svg viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="${width}" stroke-linecap="round" stroke-linejoin="round">${inner}</svg>`,
  )}`;

const ICONS: Record<string, string> = {
  // ui/DayPicker.tsx: the calendar on the header (2px) and form (1.5px) triggers.
  calendar: svgIcon('<rect x="3" y="4" width="18" height="18" rx="2" ry="2"/><line x1="16" y1="2" x2="16" y2="6"/><line x1="8" y1="2" x2="8" y2="6"/><line x1="3" y1="10" x2="21" y2="10"/>', 2),
  calendarThin: svgIcon('<rect x="3" y="4" width="18" height="18" rx="2" ry="2"/><line x1="16" y1="2" x2="16" y2="6"/><line x1="8" y1="2" x2="8" y2="6"/><line x1="3" y1="10" x2="21" y2="10"/>', 1.5),
  // ui/DayPicker.tsx: the prev/next arrows and ui/Combobox.tsx's trigger chevron.
  chevronLeft: svgIcon('<polyline points="15 18 9 12 15 6"/>', 2.5),
  chevronRight: svgIcon('<polyline points="9 18 15 12 9 6"/>', 2.5),
  chevronDown: svgIcon('<path d="M6 9l6 6 6-6"/>', 2),
  // ui/Combobox.tsx: the selected tick and the create/suggest `+`.
  check: svgIcon('<path d="M20 6L9 17l-5-5"/>', 2.5),
  plus: svgIcon('<line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>', 2.5),
  // transactions/shared/constants.tsx: Award03, Consultant, Operations (accounts/icons.ts), LOCATION_ICON.
  category: svgIcon('<path d="M7.869 15.46 7 22l4.588-2.753c.15-.09.225-.135.305-.152a.5.5 0 0 1 .214 0c.08.017.155.062.305.152L17 22l-.868-6.543m.294-11.208c.154.373.45.67.824.825l1.309.542a1.525 1.525 0 0 1 .825 1.992l-.542 1.308a1.52 1.52 0 0 0 0 1.168l.542 1.307a1.525 1.525 0 0 1-.826 1.993l-1.308.542c-.373.154-.67.45-.825.824l-.542 1.309a1.524 1.524 0 0 1-1.992.825l-1.308-.542a1.53 1.53 0 0 0-1.166 0l-1.31.542a1.524 1.524 0 0 1-1.99-.824l-.542-1.31a1.52 1.52 0 0 0-.824-.825l-1.31-.542a1.524 1.524 0 0 1-.825-1.991l.542-1.308a1.53 1.53 0 0 0 0-1.167l-.542-1.31a1.525 1.525 0 0 1 .826-1.992l1.307-.542c.374-.154.67-.45.825-.823l.543-1.309a1.524 1.524 0 0 1 1.991-.825l1.308.542c.374.154.794.154 1.167-.001l1.31-.54a1.525 1.525 0 0 1 1.99.825l.543 1.31z"/>', 1.5),
  party: svgIcon('<path d="M16 3.468a4.5 4.5 0 0 1 0 8.064m2 5.234c1.512.684 2.873 1.799 4 3.234M2 20c1.946-2.477 4.59-4 7.5-4s5.554 1.523 7.5 4M14 7.5a4.5 4.5 0 1 1-9 0 4.5 4.5 0 0 1 9 0"/>', 1.5),
  department: svgIcon('<path d="M4 18v-.2c0-1.68 0-2.52.327-3.162a3 3 0 0 1 1.311-1.311C6.28 13 7.12 13 8.8 13h6.4c1.68 0 2.52 0 3.162.327a3 3 0 0 1 1.311 1.311C20 15.28 20 16.12 20 17.8v.2M4 18a2 2 0 1 0 0 4 2 2 0 0 0 0-4m16 0a2 2 0 1 0 0 4 2 2 0 0 0 0-4m-8 0a2 2 0 1 0 0 4 2 2 0 0 0 0-4m0 0V8M6 8h12c.932 0 1.398 0 1.765-.152a2 2 0 0 0 1.083-1.083C21 6.398 21 5.932 21 5s0-1.398-.152-1.765a2 2 0 0 0-1.083-1.083C19.398 2 18.932 2 18 2H6c-.932 0-1.398 0-1.765.152a2 2 0 0 0-1.083 1.083C3 3.602 3 4.068 3 5s0 1.398.152 1.765a2 2 0 0 0 1.083 1.083C4.602 8 5.068 8 6 8"/>', 1.5),
  location: svgIcon('<path d="M21 10c0 7-9 13-9 13s-9-6-9-13a9 9 0 0 1 18 0z"/><circle cx="12" cy="10" r="3"/>', 1.5),
  // PanelFab: the paperclip (Entry, from entry_side's SIDE_ICONS) and the
  // lucide panel-right (Accounts/Tags).
  paperclip: svgIcon('<path d="M21.44 11.05l-9.19 9.19a6 6 0 0 1-8.49-8.49l9.19-9.19a4 4 0 0 1 5.66 5.66l-9.2 9.19a2 2 0 0 1-2.83-2.83l8.49-8.48"/>', 2),
  panelRight: svgIcon('<rect x="3" y="3" width="18" height="18" rx="2"/><path d="M15 3v18"/>', 2),
  // BooksCombo's group tiles: arrow-right-left, diamond, trending-up/down, landmark.
  swap: svgIcon('<path d="m16 3 4 4-4 4"/><path d="M20 7H4"/><path d="m8 21-4-4 4-4"/><path d="M4 17h16"/>', 2),
  diamond: svgIcon('<path d="M2.7 10.3a2.41 2.41 0 0 0 0 3.41l7.59 7.59a2.41 2.41 0 0 0 3.41 0l7.59-7.59a2.41 2.41 0 0 0 0-3.41l-7.59-7.59a2.41 2.41 0 0 0-3.41 0Z"/>', 2),
  trendUp: svgIcon('<polyline points="22 7 13.5 15.5 8.5 10.5 2 17"/><polyline points="16 7 22 7 22 13"/>', 2),
  trendDown: svgIcon('<polyline points="22 17 13.5 8.5 8.5 13.5 2 7"/><polyline points="16 17 22 17 22 11"/>', 2),
  landmark: svgIcon('<line x1="3" x2="21" y1="22" y2="22"/><line x1="6" x2="6" y1="18" y2="11"/><line x1="10" x2="10" y1="18" y2="11"/><line x1="14" x2="14" y1="18" y2="11"/><line x1="18" x2="18" y1="18" y2="11"/><polygon points="12 2 20 7 4 7"/>', 2),
  // transactions/shared/constants.tsx TABS → accounts/icons.ts: the entry
  // type icons (Journal / CoinsHand / CoinsHand02 / ArrowSwitch).
  journal: svgIcon('<path d="m10.333 20-.083-.125c-.579-.868-.868-1.302-1.25-1.617a3.3 3.3 0 0 0-1.149-.614c-.473-.144-.995-.144-2.039-.144H4.667c-.934 0-1.4 0-1.757-.182a1.67 1.67 0 0 1-.728-.728C2 16.234 2 15.767 2 14.833V7.667c0-.934 0-1.4.182-1.757.16-.314.414-.569.728-.728C3.267 5 3.733 5 4.667 5H5c1.867 0 2.8 0 3.513.363.628.32 1.137.83 1.457 1.457.363.713.363 1.646.363 3.513m0 9.667v-9.667m0 9.667.084-.125c.579-.868.868-1.302 1.25-1.617a3.3 3.3 0 0 1 1.148-.614c.474-.144.996-.144 2.04-.144H16c.933 0 1.4 0 1.757-.182.313-.16.568-.414.728-.728.182-.356.182-.157.182-1.09m-8.334-5.167c0-1.867 0-2.8.364-3.513.32-.627.83-1.137 1.456-1.457C12.866 5 13.8 5 15.667 5H16c.85 0 1.314 0 1.659.137l.016.007.04.017.14.065c.134.062.372.179.478.274M15.097 12.548c.019-.172.028-.258.054-.339a.8.8 0 0 1 .098-.202c.046-.07.107-.132.23-.254l5.503-5.503a1.179 1.179 0 0 1 1.666 1.667l-5.502 5.502c-.123.123-.184.184-.255.23a.8.8 0 0 1-.202.098c-.08.026-.167.035-.339.055l-1.41.156z"/>', 1.5),
  payIn: svgIcon('<path d="M13.53 8.352a3.5 3.5 0 1 1 .941-4.704M6 20.088h2.61q.512 0 1.009.12l2.758.67c.598.147 1.222.16 1.826.043l3.05-.593a4.2 4.2 0 0 0 2.127-1.107l2.158-2.1a1.503 1.503 0 0 0 0-2.168 1.61 1.61 0 0 1-2.06-.143l-2.515 1.835c-.36.263-.799.405-1.25.405h-2.428 1.546c.871 0 1.577-.687 1.577-1.534v-.307c0-.703-.492-1.317-1.194-1.487l-2.385-.58A5 5 0 0 0 11.643 13c-.965 0-2.711.799-2.711.799L6 15.025M20 6.5a3.5 3.5 0 1 1-7 0 3.5 3.5 0 0 1 7 0M2 14.6v5.8c0 .56 0 .84.109 1.054a1 1 0 0 0 .437.437C2.76 22 3.04 22 3.6 22h.8c.56 0 .84 0 1.054-.109a1 1 0 0 0 .437-.437C6 21.24 6 20.96 6 20.4v-5.8c0-.56 0-.84-.109-1.054a1 1 0 0 0-.437-.437C5.24 13 4.96 13 4.4 13h-.8c-.56 0-.84 0-1.054.109a1 1 0 0 0-.437.437C2 13.76 2 14.04 2 14.6"/>', 1.5),
  payOut: svgIcon('<path d="M13.53 15.648a3.5 3.5 0 1 0 .941 4.704M6 3.912h2.61q.512 0 1.009-.12l2.758-.67a4.3 4.3 0 0 1 1.826-.043l3.05.593a4.2 4.2 0 0 1 2.127 1.107l2.158 2.1c.616.598.616 1.569 0 2.168a1.61 1.61 0 0 1-2.06.143l-2.515-1.835a2.1 2.1 0 0 0-1.25-.405h-2.428 1.546c.871 0 1.577.687 1.577 1.534v.307c0 .703-.492 1.317-1.194 1.487l-2.385.58a5 5 0 0 1-1.186.142c-.965 0-2.711-.799-2.711-.799L6 8.975M20 17.5a3.5 3.5 0 1 0-7 0 3.5 3.5 0 0 0 7 0M2 9.4V3.6c0-.56 0-.84.109-1.054a1 1 0 0 1 .437-.437C2.76 2 3.04 2 3.6 2h.8c.56 0 .84 0 1.054.109a1 1 0 0 1 .437.437C6 2.76 6 3.04 6 3.6v5.8c0 .56 0 .84-.109 1.054a1 1 0 0 1-.437.437C5.24 11 4.96 11 4.4 11h-.8c-.56 0-.84 0-1.054-.109a1 1 0 0 1-.437-.437C2 10.24 2 9.96 2 9.4"/>', 1.5),
  transfer: svgIcon('<path d="M20 17H4m0 0 4-4m-4 4 4 4M4 7h16m0 0-4-4m4 4-4 4"/>', 1.5),
  // TransferForm: the mobile down / desktop right arrows (stroke #94a3b8),
  // JournalForm's "Lines" list icon, and the totals bar's circle-x.
  arrowDown: svgIcon('<line x1="12" y1="5" x2="12" y2="19"/><polyline points="19 12 12 19 5 12"/>', 2),
  arrowRight: svgIcon('<line x1="5" y1="12" x2="19" y2="12"/><polyline points="12 5 19 12 12 19"/>', 2),
  list: svgIcon('<line x1="8" y1="6" x2="21" y2="6"/><line x1="8" y1="12" x2="21" y2="12"/><line x1="8" y1="18" x2="21" y2="18"/><line x1="3" y1="6" x2="3.01" y2="6"/><line x1="3" y1="12" x2="3.01" y2="12"/><line x1="3" y1="18" x2="3.01" y2="18"/>', 2),
  circleX: svgIcon('<circle cx="12" cy="12" r="10"/><line x1="15" y1="9" x2="9" y2="15"/><line x1="9" y1="9" x2="15" y2="15"/>', 2.5),
  // ReconSummary's card icons (dash for inflow/outflow, not-equal for Net)
  // and row labels (lucide landmark / book-open).
  dash: svgIcon('<path d="M5 12h14"/>', 2),
  notEqual: svgIcon('<path d="M5 9h14M5 15h14m0-10L5 19"/>', 1.5),
  bookOpen: svgIcon('<path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z"/><path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/>', 2),
  // Departments/locations pages: DeptIcon (building) and the accent band's
  // table glyph; locations reuse the location pin.
  building: svgIcon('<rect x="4" y="2" width="16" height="20" rx="2" ry="2"/><path d="M9 22v-4h6v4"/><path d="M8 6h.01M16 6h.01M12 6h.01M8 10h.01M16 10h.01M12 10h.01M8 14h.01M16 14h.01M12 14h.01"/>', 1.5),
  table: svgIcon('<rect x="3" y="3" width="18" height="18" rx="2"/><path d="M3 9h18"/><path d="M9 21V9"/>', 1.5),
};

// Weekday labels by day number (0 = Sunday): the owner's "M T W Th F S Su".
const WEEKDAYS = ["Su", "M", "T", "W", "Th", "F", "S"];

/** The entry type's old-app tab icon (TABS in transactions/shared/constants.tsx). */
const modeIcon = (mode: string) =>
  mode === "money_in" ? ICONS.payIn : mode === "money_out" ? ICONS.payOut : mode === "transfer" ? ICONS.transfer : ICONS.journal;

const BOOKS_TOKENS = {
  colors: {
    primary: {
      50: "#f0fdf4", 100: "#dcfce7", 200: "#bbf7d0", 300: "#86efac", 400: "#4ade80",
      500: "#22c55e", 600: "#16a34a", 700: "#15803d", 800: "#166534", 900: "#14532d",
      hover: "#16a34a", light: "#86efac", dark: "#15803d",
    },
    background: "#ffffff", surface: "#f8fafc", surfaceElevated: "#f1f5f9", border: "#e2e8f0", borderSubtle: "#cbd5e1",
    textPrimary: "#0f172a", textSecondary: "#475569", textMuted: "#94a3b8",
    accent: "var(--color-primary)", accentHover: "var(--color-primary-hover)",
    success: "#22c55e", warning: "#f59e0b", error: "#ef4444", info: "#3b82f6",
    asset: "#14b8a6", liability: "#fb923c", equity: "#a855f7", revenue: "#3b82f6", expense: "#f87171",
    debit: "#22c55e", credit: "#ef4444", income: "#22c55e",
    app: {
      backgroundStart: "#a5cbe7", backgroundEnd: "#9db9f0", card: "#f5f6f9",
      headerBlue: "#2490ce", headerTeal: "#2a9d8f", textNavy: "#32497f", textLight: "#6b7c93",
      buttonNeon: "#00ffbe", buttonNeonDark: "#00d9a0",
    },
    dark: {
      background: "#0a0a0b", surface: "#141416", surfaceElevated: "#1c1c1f",
      border: "#2a2a2d", borderSubtle: "#1f1f22", textPrimary: "#fafafa",
      textSecondary: "#a1a1aa", textMuted: "#71717a",
      app: { backgroundStart: "#0f172a", backgroundEnd: "#1e293b", card: "#1e293b", textNavy: "#e2e8f0", textLight: "#94a3b8" },
    },
  },
  spacing: { xs: "0.25rem", sm: "0.5rem", md: "1rem", lg: "1.5rem", xl: "2rem", "2xl": "3rem" },
  radius: { sm: "0.25rem", md: "0.5rem", lg: "0.75rem", xl: "1rem", "2xl": "1.5rem" },
  font: { sans: '"Inter", system-ui, -apple-system, sans-serif', mono: '"JetBrains Mono", "Fira Code", monospace' },
};

/** Pill tones (`st`). */
/** The accent: the page header, the main button and the picked state. A book picks one in
 *  Settings; the host tells an app light from dark and nothing else, so each solid is a tone
 *  that carries white text on both. Green is for money in and "posted", whatever the accent. */
const ACCENTS: Record<string, { name: string; solid: string; scale: string[] }> = {
  green: { name: "Green", solid: "#22c55e", scale: ["#f0fdf4", "#dcfce7", "#bbf7d0", "#86efac", "#4ade80", "#22c55e", "#16a34a", "#15803d", "#166534", "#14532d"] },
  teal: { name: "Teal", solid: "#0d9488", scale: ["#f0fdfa", "#ccfbf1", "#99f6e4", "#5eead4", "#2dd4bf", "#14b8a6", "#0d9488", "#0f766e", "#115e59", "#134e4a"] },
  blue: { name: "Blue", solid: "#2563eb", scale: ["#eff6ff", "#dbeafe", "#bfdbfe", "#93c5fd", "#60a5fa", "#3b82f6", "#2563eb", "#1d4ed8", "#1e40af", "#1e3a8a"] },
  violet: { name: "Violet", solid: "#7c3aed", scale: ["#f5f3ff", "#ede9fe", "#ddd6fe", "#c4b5fd", "#a78bfa", "#8b5cf6", "#7c3aed", "#6d28d9", "#5b21b6", "#4c1d95"] },
  orange: { name: "Orange", solid: "#ea580c", scale: ["#fff7ed", "#ffedd5", "#fed7aa", "#fdba74", "#fb923c", "#f97316", "#ea580c", "#c2410c", "#9a3412", "#7c2d12"] },
  rose: { name: "Rose", solid: "#e11d48", scale: ["#fff1f2", "#ffe4e6", "#fecdd3", "#fda4af", "#fb7185", "#f43f5e", "#e11d48", "#be123c", "#9f1239", "#881337"] },
};
const accent = signal("green");
const accShade = (step: number) => {
  const current = ACCENTS[accent()] || ACCENTS.green;
  return current.scale[step] || current.solid;
};
const acc = () => accShade(5);
/** Text on the accent. */
const ON_ACC = "text-white";
/** The border of a picked card or option. */
const picked = (on: boolean) => (on ? { borderColor: acc() } : undefined);

const PILL: Record<string, string> = {
  ok: "bg-[#dff3e8] dark:bg-[#123423] text-[#157a48] dark:text-[#3fcf89]",
  draft: "bg-[#f8ecd2] dark:bg-[#3a2e12] text-[#a2650f] dark:text-[#f3b64c]",
  rev: "bg-[#fbe3e0] dark:bg-[#3a1a17] text-[#c0392b] dark:text-[#ff7d75]",
  in: "bg-[#e1ebfb] dark:bg-[#15253f] text-[#2f6fd6] dark:text-[#7fb2ff]",
  bir: "bg-[#ebe4fb] dark:bg-[#2a2142] text-[#6a46c9] dark:text-[#b79bff]",
  mute: "bg-[#ececea] dark:bg-[#262626] text-[#5c5c5c] dark:text-[#a0a0a0]",
};
const TAG: Record<string, string> = {
  as: PILL.in,
  li: PILL.draft,
  eq: PILL.in,
  re: PILL.ok,
  ex: PILL.rev,
};
const BAR: Record<string, string> = { pos: C.greenBg, neg: C.redBg, blue: C.blueBg, "": C.greenBg };
const AMT: Record<string, string> = { pos: C.green, neg: C.ink, "": C.ink };

/** A row's key is its content: a row whose data changes is drawn again. */
const rowKey = (row: Any) => JSON.stringify(row);

const errText = (e: Any) => (e && e.message ? String(e.message) : String(e));

/** A client_request_id: unique per call, never a subject. */
let seq = 0;
const OPEN_TAG = String(Date.now()).slice(-8);
const rid = () => `bk-${OPEN_TAG}-${(seq += 1)}`;

/** Bumped after every write, so pages read again. */
const ver = signal(0);
const bump = () => ver.set(ver() + 1);

const toastText = signal("");
let toastTimer: Any = null;
function toast(text: string) {
  toastText.set(text);
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = setTimeout(() => toastText.set(""), 2600);
}

/** A Books write: a fresh client_request_id, then every page reads again. */
async function act(tool: string, args: Record<string, Any>): Promise<Any> {
  try {
    const r = await buwiz.call(tool, { client_request_id: rid(), ...args });
    bump();
    return r;
  } catch (e) {
    toast(errText(e));
    throw e;
  }
}

/** Typed money to two-place text with no float: "60,000" -> "60000.00". */
function moneyText(typed: string): string | null {
  const s = String(typed || "").replace(/[₱,\s]/g, "").replace(/^PHP/i, "");
  if (!/^\d+(\.\d{1,2})?$/.test(s)) return null;
  const [w, c = ""] = s.split(".");
  const whole = w.replace(/^0+(?=\d)/, "");
  const text = `${whole}.${(c + "00").slice(0, 2)}`;
  return text === "0.00" ? null : text;
}
/** "60000.00" -> "60,000.00", by string only. */
function grouped(text: string): string {
  const [w, c] = text.split(".");
  return `${w.replace(/\B(?=(\d{3})+(?!\d))/g, ",")}.${c}`;
}

function Btn(p: { label: string; onPress: () => void; tone?: "go" | "vi" | "plain"; disabled?: boolean; small?: boolean }) {
  const tone =
    p.tone === "go"
      ? `bg-[${acc()}] hover:bg-[${accShade(6)}] border-transparent`
      : p.tone === "vi"
        ? `${C.violetBg} border-[#6a46c9] dark:border-[#b79bff] hover:bg-[#5a35b3] dark:hover:bg-[#9d7ef2]`
        : `${C.raise} ${C.line} hover:bg-[#e2e8f0] dark:hover:bg-[#2a2a2d]`;
  const ink = p.tone === "go" ? "text-white" : p.tone === "vi" ? "text-white dark:text-[#1a1033]" : C.ink;
  return (
    <View
      class={`rounded-lg border ${p.small ? "px-2 py-0.5" : "px-3 py-1.5"} items-center justify-center transition-colors ${tone} ${p.disabled ? "opacity-40" : ""}`}
      activeStyle={{ opacity: 0.7 }}
      onPress={() => (p.disabled ? null : p.onPress())}
    >
      <Text class={`text-[12.5px] font-semibold ${ink} truncate`}>
        {p.label}
      </Text>
    </View>
  );
}

function Pill(p: { tone: string; label: string }) {
  return (
    <View class={`rounded-full px-2 py-0.5 self-start ${PILL[p.tone] || PILL.mute}`}>
      <Text class={`text-[11px] ${PILL[p.tone] || PILL.mute} truncate`}>
        {p.label}
      </Text>
    </View>
  );
}

/** `said`: for a chip whose label is someone's data. See `Pick`'s `optionSaid`. */
function Chip(p: { label: string; count?: Any; on: boolean; onPress: () => void; said?: string }) {
  return (
    <View
      accessibilityLabel={p.said}
      accessibilityValue={p.said ? { text: p.label } : undefined}
      class={`rounded-full px-2.5 py-1 border flex-row gap-1 transition-colors ${p.on ? "bg-[#1c1c1c] dark:bg-[#f0f0f0] border-transparent" : `${C.panel} ${C.line} hover:border-[#cbd5e1] dark:hover:border-[#3f3f46]`}`}
      onPress={p.onPress}
      activeStyle={{ opacity: 0.7 }}
    >
      <Text class={`text-[12px] ${p.on ? "text-white dark:text-[#111111] font-semibold" : C.mute}`}>{p.label}</Text>
      <If when={p.count !== undefined && p.count !== null}>
        <Text class={`text-[12px] opacity-70 ${p.on ? "text-white dark:text-[#111111]" : C.mute}`}>{String(p.count)}</Text>
      </If>
    </View>
  );
}

function Seg(p: { options: [string, string][]; value: string; onPick: (v: string) => void }) {
  return (
    <View class={`flex-row rounded-[9px] border p-0.5 gap-0.5 self-start ${C.panel} ${C.line}`}>
      <Each of={p.options}>
        {(o: Any) => (
          <View
            class={`rounded-[7px] px-2.5 py-1 transition-colors ${o()[0] === p.value ? C.raise : "hover:bg-[#e2e8f0] dark:hover:bg-[#26262a]"}`}
            onPress={() => p.onPick(o()[0])}
            activeStyle={{ opacity: 0.7 }}
          >
            <Text class={`text-[12.5px] ${o()[0] === p.value ? `font-semibold ${C.ink}` : C.mute}`}>{o()[1]}</Text>
          </View>
        )}
      </Each>
    </View>
  );
}

function Panel(p: { children?: Any; pad?: boolean; class?: string }) {
  return <View class={`rounded-xl border ${C.panel} ${C.line} ${p.pad ? "p-3.5" : ""} ${p.class || ""}`}>{p.children}</View>;
}

/** The owner-approved right-panel rule (2026-11-21): on lg+ the side panel
 *  is a quarter of the window, clamped to 340–520px; below lg it is a
 *  full-screen sheet (no floating drawer, no backdrop gap, no list strip)
 *  with the old drawer's ‹ Back row (CategoryManagerLayout.tsx:356). The
 *  sheet anchors to the page's relative root, so mount this where the
 *  column belongs and the overlay still covers the whole page. `body` is a
 *  function so each mode mounts its own copy; `card` wraps the sheet copy
 *  in the rounded panel chrome; `fill` drops the outer scroll so a body
 *  with its own Scroll (Entry's tabs) fills the height. */
function SidePanel(p: {
  open: () => boolean;
  onBack: () => void;
  body: () => Any;
  class?: string;
  card?: boolean;
  fill?: boolean;
}) {
  const sheetBody = () =>
    p.card ? (
      <View class={`rounded-[16px] border overflow-hidden flex-col bg-white dark:bg-slate-800 ${C.line}`}>{p.body()}</View>
    ) : (
      p.body()
    );
  return (
    <>
      <View class={`hidden lg:flex flex-col w-1/4 min-w-[340px] max-w-[520px] shrink-0 ${p.class || ""}`}>{p.body()}</View>
      <If when={p.open()}>
        <View class="fixed inset-0 lg:hidden flex-col bg-[#e8f0f2] dark:bg-slate-900">
          <View class="shrink-0 flex-row items-center px-2 py-1">
            <View
              accessibilityLabel="Back"
              class="h-11 flex-row items-center gap-1.5 rounded-lg px-2 transition-colors hover:text-slate-800 dark:hover:text-slate-200"
              onPress={p.onBack}
              activeStyle={{ opacity: 0.7 }}
            >
              <Text class="text-[16px] text-slate-600 dark:text-slate-400">‹</Text>
              <Text class="text-[14px] font-medium text-slate-600 dark:text-slate-400">Back</Text>
            </View>
          </View>
          <If when={Boolean(p.fill)}>
            <View class="flex-1 flex-col min-h-0 px-3 pb-3">{sheetBody()}</View>
          </If>
          <If when={!p.fill}>
            <Scroll class="flex-1">
              <View class="px-3 pb-3">{sheetBody()}</View>
            </Scroll>
          </If>
        </View>
      </If>
    </>
  );
}

/** The below-lg circular trigger for the hidden side panel (owner rule):
 *  a teal FAB at the bottom right, only while the sheet is closed. */
function PanelFab(p: { onPress: () => void; label: string; icon: string; badge?: () => number }) {
  const count = () => (p.badge ? p.badge() : 0);
  return (
    <View
      accessibilityLabel={p.label}
      class="fixed bottom-6 right-6 w-14 h-14 rounded-full items-center justify-center bg-teal-600 hover:bg-teal-700 dark:bg-teal-500 dark:hover:bg-teal-600 shadow-lg hover:shadow-xl transition-colors cursor-pointer lg:hidden focus-visible:ring-2 focus-visible:ring-teal-500/40 focus-visible:ring-offset-2"
      onPress={p.onPress}
      activeStyle={{ opacity: 0.85 }}
    >
      <Avatar src={ICONS[p.icon] || ""} class="w-6 h-6" imageClass="tint-white" fallbackDelay={Infinity} accessibilityLabel="" />
      <If when={count() > 0}>
        <View class="absolute -top-0.5 -right-0.5 min-w-5 h-5 px-1 rounded-full bg-white items-center justify-center">
          <Text class="text-[10px] font-bold text-teal-700">{`${count()}`}</Text>
        </View>
      </If>
    </View>
  );
}

/** Set by the Books window: back to Home. Cards leave it unset, so they show no arrow. */
let goBack: (() => void) | null = null;

/** The page header: a green band with the way back, the title, and buttons on the right. */
function Head(p: { title: string; sub?: string; children?: Any; home?: boolean }) {
  return (
    <View class="flex-row items-center gap-2.5 flex-wrap rounded-xl px-3.5 py-2.5" style={{ backgroundColor: acc() }}>
      <If when={Boolean(goBack) && !p.home}>
        <View
          accessibilityLabel="Back to Home"
          class="w-7 h-7 rounded-full items-center justify-center bg-white/20 hover:bg-white/30 transition-colors"
          onPress={() => (goBack ? goBack() : null)}
          activeStyle={{ opacity: 0.7 }}
        >
          <Text class="text-[15px] font-bold text-white">‹</Text>
        </View>
      </If>
      <Text class="text-[17px] font-bold text-white">{p.title}</Text>
      <If when={Boolean(p.sub)}>
        <Text class="text-[12.5px] text-white/80">{p.sub || ""}</Text>
      </If>
      <View class="flex-1" />
      {p.children}
    </View>
  );
}

/** One bar of a KPI panel: label, a percent track, the amount. */
function Bar(p: { label: string; pct: Any; shown: string; tone: string }) {
  return (
    <View class="flex-row items-center gap-2">
      <Text class={`text-[11.5px] w-[104px] ${C.mute} truncate`}>
        {p.label}
      </Text>
      <View class={`flex-1 h-[6px] rounded-[3px] overflow-hidden ${C.raise}`}>
        <View class={`h-full rounded-[3px] ${BAR[p.tone] || C.greenBg}`} style={{ width: `${String(p.pct || 0)}%` }} />
      </View>
      <Text class={`text-[11.5px] font-mono ${C.mute}`}>{p.shown}</Text>
    </View>
  );
}

function Dot(p: { color: string }) {
  return <View class="w-2 h-2 rounded-[2px]" style={{ backgroundColor: p.color || "#888888" }} />;
}

/** A label above a value or a control (the sheet's fields). */
function Field(p: { label: string; children?: Any; grow?: boolean; plain?: boolean }) {
  return (
    <View class={`gap-1 ${p.grow ? "flex-1" : ""}`}>
      <Text class={p.plain ? `text-[12px] font-medium ${C.mute}` : `text-[11px] uppercase tracking-wider ${C.dim}`}>{p.label}</Text>
      {p.children}
    </View>
  );
}

function TextBox(p: { value: string; onChange: (v: string) => void; placeholder?: string; big?: boolean; label?: string; entry?: boolean; transferAmount?: boolean }) {
  return (
    <Input
      accessibilityLabel={p.label}
      value={p.value}
      placeholder={p.placeholder || ""}
      onChange={(e: Any) => p.onChange(typeof e === "string" ? e : e.value)}
      class={p.transferAmount
        ? "w-28 h-8 text-right text-xl font-semibold text-white tabular-nums bg-transparent border-none outline-none placeholder:text-white/40 focus:ring-0"
        : `placeholder:text-[#8e8e93] transition-colors ${p.entry ? "rounded-md" : "rounded-lg"} border px-2.5 ${p.big ? "h-10 text-[18px] font-mono" : "h-8 text-[13px]"} focus:outline-none focus:ring-1 focus:ring-[#2a9d8f] focus:border-[#2a9d8f] ${p.entry ? "bg-white dark:bg-slate-700 border-[#e2e8f0] dark:border-slate-600 text-[#0f172a] dark:text-slate-100" : `${C.bg} ${C.line} ${C.ink}`}`}
    />
  );
}

/** A picker: the current choice; a press opens the list under it.
 *
 *  `optionSaid`: for a list of people, or of words a person typed. Each option then has these fixed words as its
 *  accessibility label and its text as its accessibility value. A button whose label differs from its text reaches
 *  the Bot's assistant unnamed (app/src/agent_hub/filter.rs), so the names stay out of what it reads, and it can't
 *  press those options; a screen reader still says the name. Lists of accounts and categories leave it out: the
 *  assistant picks those to fill a draft. */
function Pick(p: { value: string; options: Any[]; onPick: (o: Any) => void; placeholder?: string; label?: string; optionSaid?: string }) {
  const open = signal(false);
  const current = computed(() => (p.options || []).find((o: Any) => o.ticket === p.value));
  return (
    <View class="gap-1">
      <View
        accessibilityLabel={p.label}
        class={`flex-row items-center rounded-lg border px-2.5 h-8 transition-colors ${C.bg} ${open() ? "border-[#2a9d8f] dark:border-[#2dd4bf]" : `${C.line} hover:border-[#cbd5e1] dark:hover:border-[#3f3f46]`}`}
        onPress={() => open.set(!open())}
        activeStyle={{ opacity: 0.7 }}
      >
        <Text accessibilityElementsHidden={Boolean(p.label)} class={`flex-1 text-[13px] ${current() ? C.ink : C.dim} truncate`}>
          {current() ? current().label || current().name : p.placeholder || "Pick one"}
        </Text>
        <Avatar accessibilityElementsHidden={Boolean(p.label)} src={ICONS.chevronDown} class="w-5 h-5 shrink-0 opacity-40" imageClass={`tint-[#1e293b] dark:tint-slate-100`} fallbackDelay={Infinity} accessibilityLabel="" />
      </View>
      <If when={open()}>
        <View class={`rounded-lg border max-h-[180px] ${C.panel} ${C.line}`}>
          <If when={(p.options || []).length === 0}>
            <Text class={`px-2.5 py-1.5 text-[12.5px] ${C.dim}`}>No options</Text>
          </If>
          <Scroll class="max-h-[178px]">
            <Each of={p.options || []} key={rowKey}>
              {(o: Any) => (
                <View
                  accessibilityLabel={p.optionSaid}
                  accessibilityValue={p.optionSaid ? { text: o().label || o().name } : undefined}
                  class={`px-2.5 py-1.5 transition-colors ${o().ticket === p.value ? C.raise : "hover:bg-[#f8fafc] dark:hover:bg-[#1c1c1f]"}`}
                  onPress={() => {
                    p.onPick(o());
                    open.set(false);
                  }}
                >
                  <Text class={`text-[12.5px] ${C.ink} truncate`}>
                    {o().label || o().name}
                  </Text>
                </View>
              )}
            </Each>
          </Scroll>
        </View>
      </If>
    </View>
  );
}
function BooksCombo(p: {
  value: string;
  options: Any[] | (() => Any[]);
  onPick: (o: Any) => void;
  onQuery?: (query: string) => void;
  query?: string | (() => string);
  suggestions?: Any[] | (() => Any[]);
  onSuggestion?: (o: Any) => void;
  onCreate?: (query: string) => void;
  createLabel?: string;
  placeholder?: string;
  label?: string;
  group?: string;
  /** "chip": the reference's rounded, bordered field with a leading icon
   *  (tag for category, user for party, building for department, pin for
   *  location) and a trailing chevron, instead of a bare filter input. */
  trigger?: "chip";
  icon?: string;
}) {
  const CREATE_VALUE = "__books_create__";
  const open = signal(false);
  const optionRows = () => (typeof p.options === "function" ? p.options() : p.options) || [];
  const suggestionRows = () => (typeof p.suggestions === "function" ? p.suggestions() : p.suggestions) || [];
  const queryValue = () => (typeof p.query === "function" ? p.query() : p.query) || "";
  const createLabel = () => p.createLabel || "category";
  const items = computed(() => [
    ...optionRows().map((o: Any) => ({
      value: String(o.ticket),
      title: String(o.label || o.name || ""),
      group: String(o.group || p.group || "Accounts"),
    })),
    ...suggestionRows().map((o: Any, index: number) => ({
      value: `__books_suggested_${index}__`,
      title: `${String(o.name || o.label || "")} ${String((o.keywords || []).join(" "))}`,
      group: "Create New",
    })),
    ...(p.onCreate && queryValue().trim()
      ? [{ value: CREATE_VALUE, title: `${queryValue().trim()}`, group: "Create New" }]
      : []),
  ]);
  // "chip": the nsh Combobox borders its input alone (prefix, input and the
  // trigger button are siblings in one row), so the reference's one bordered
  // trigger is a View drawn under the Combobox: the input covers it invisibly,
  // so a press focuses it and toggles the list (the chip's own press is the
  // fallback), and typing filters as before. While the list is open or a
  // query is typed the input turns visible over the label, like the
  // reference, so what is typed is read in the field.
  const chip = p.trigger === "chip";
  const typing = () => open() || queryValue().trim().length > 0;
  const chipTitle = () =>
    items().find((i: Any) => i.value === p.value)?.title || p.placeholder || "Select...";
  // Non-chip: the nsh Combobox's own row can't carry classes, so the
  // reference's bordered trigger (Combobox.tsx) is this wrapper: the border,
  // radius, background and open-state teal border wrap prefix, input and the
  // ⌄ together, while the input inside is borderless and flex-1. The class
  // stays inline so open() re-evaluates it; a hoisted string freezes at mount.
  return (
    <View
      class={chip ? "relative w-full" : `flex-col justify-center w-full min-h-11 sm:min-h-0 px-3 py-2 rounded-lg transition-colors ${open() ? "border-2 border-[#2a9d8f] dark:border-[#2dd4bf]" : "border border-gray-200 hover:border-gray-300 dark:border-slate-600 dark:hover:border-slate-500"} bg-white dark:bg-slate-800`}
    >
      {chip && (
        <View
          class={`absolute inset-0 flex-row items-center gap-2 rounded-lg border bg-white px-3 py-2 transition-colors cursor-pointer dark:bg-slate-800 ${open() ? "border-[#2a9d8f] dark:border-[#2dd4bf]" : "border-[#e5e7eb] hover:border-[#d1d5db] dark:border-slate-600 dark:hover:border-[#64748b]"}`}
          onPress={() => open.set(!open())}
        >
          <If when={Boolean(p.icon && ICONS[p.icon || ""])}>
            <View class="w-5 h-5 items-center justify-center shrink-0">
              <Avatar src={ICONS[p.icon || ""]} class="w-4 h-4 opacity-50" imageClass="tint-[#1e293b] dark:tint-slate-100" fallbackDelay={Infinity} accessibilityLabel="" />
            </View>
          </If>
          <If when={!typing()}>
            <Text
              class={`flex-1 min-w-0 truncate text-sm ${p.value ? "text-[#1e293b] dark:text-slate-100" : "text-[#94a3b8] dark:text-slate-500"}`}
            >
              {chipTitle()}
            </Text>
          </If>
          <Avatar src={ICONS.chevronDown} class="w-5 h-5 shrink-0 opacity-40" imageClass="tint-[#1e293b] dark:tint-slate-100" fallbackDelay={Infinity} accessibilityLabel="" />
        </View>
      )}
      <Combobox
        items={items()}
        open={open()}
        onOpenChange={({ open: next }) => open.set(next)}
        value={p.value ? [p.value] : []}
        query={queryValue()}
        onChange={({ value }) => {
          const selected = value[0] || "";
          if (selected === CREATE_VALUE) {
            p.onCreate?.(queryValue().trim());
            return;
          }
          if (selected.startsWith("__books_suggested_")) {
            const index = Number(selected.slice("__books_suggested_".length).replace(/__/g, ""));
            p.onSuggestion?.(suggestionRows()[index] || {});
            return;
          }
          p.onPick(optionRows().find((o: Any) => String(o.ticket) === selected) || {});
        }}
        onQueryChange={({ query }) => p.onQuery?.(query)}
        accessibilityLabel={p.label || p.placeholder || "Account"}
        placeholder={p.placeholder || "Select..."}
        prefix={
          !chip && p.icon && ICONS[p.icon] ? (
            <Avatar src={ICONS[p.icon]} class="w-4 h-4 opacity-50" imageClass="tint-[#1e293b] dark:tint-slate-100" fallbackDelay={Infinity} accessibilityLabel="" />
          ) : undefined
        }
        inputClass={
          chip
            ? `w-full py-2 text-sm ${p.icon ? "pl-[38px] pr-3" : "px-3"} ${typing() ? "text-[#1e293b] dark:text-slate-100" : "opacity-0"}`
            : `flex-1 min-w-0 text-sm text-[#1e293b] dark:text-slate-100 focus:outline-none${p.icon ? " pl-2" : ""}`
        }
        popupClass={`rounded-xl border border-[#e2e8f0] bg-white shadow-xl overflow-hidden dark:border-slate-700 dark:bg-slate-800${chip ? " min-w-[240px]" : ""}`}
      popupStartingClass="opacity-0"
      listClass="max-h-[280px]"
      groupClass="px-4 py-1.5 text-[10px] font-semibold uppercase tracking-wider text-[#94a3b8] bg-[#f8fafc] border-t border-[#e2e8f0] dark:bg-slate-800/50 dark:text-slate-400 dark:border-slate-700"
      itemClass="px-4 min-h-10 text-[13px] text-[#0f172a] focus:bg-[#f8fafc] selected:bg-[#eaf6f6] dark:text-slate-100 dark:focus:bg-slate-700 dark:selected:bg-teal-900/30"
      searchAccessibilityLabel={`Find ${p.label || p.placeholder || "account"}`}
      searchPlaceholder="Type to filter..."
      empty={<Text class="px-4 py-2.5 text-[13px] text-[#94a3b8]">No matches found</Text>}
        triggerButton={
          chip ? <View class="w-0 h-0" /> : <Avatar src={ICONS.chevronDown} class="w-5 h-5 shrink-0 opacity-40" imageClass="tint-[#1e293b] dark:tint-slate-100" fallbackDelay={Infinity} accessibilityLabel="" />
        }
      renderItem={(item) => {
        const suggested = item.value.startsWith("__books_suggested_");
        const create = item.value === CREATE_VALUE;
        const suggestion = suggested
          ? suggestionRows()[Number(item.value.slice("__books_suggested_".length).replace(/__/g, ""))]
          : null;
        const pickedRow = item.value === p.value;
        const group = String(item.group || "").toLowerCase();
        const income = group.includes("revenue") || group.includes("income");
        const color = create || suggested
          ? "#2a9d8f"
          : group.includes("equity")
            ? "#8b5cf6"
            : income
              ? "#3b82f6"
              : group.includes("expense")
                ? "#f87171"
                : group.includes("liabilit")
                  ? "#fb923c"
                  : "#14b8a6";
        const icon = create || suggested
          ? ICONS.plus
          : group.includes("liabilit")
            ? ICONS.swap
            : group.includes("equity")
              ? ICONS.diamond
              : income
                ? ICONS.trendUp
                : group.includes("expense")
                  ? ICONS.trendDown
                  : ICONS.landmark;
        return (
          <View class="flex-row items-center gap-3">
            <If when={create || suggested}>
              <Avatar src={icon} class="w-5 h-5" imageClass={`tint-[${color}]`} fallbackDelay={Infinity} accessibilityLabel="" />
            </If>
            <If when={!create && !suggested}>
              <View class="w-5 h-5 rounded items-center justify-center" style={{ backgroundColor: `${color}22` }}>
                <Avatar src={icon} class="w-3.5 h-3.5" imageClass={`tint-[${color}]`} fallbackDelay={Infinity} accessibilityLabel="" />
              </View>
            </If>
            <View class="flex-1 gap-0.5">
              <Text class={`text-[13px] ${create || pickedRow ? "font-medium" : ""}`} style={create || pickedRow ? { color: "#2a9d8f" } : undefined}>
                {create ? `Create ${createLabel()} “${queryValue().trim()}”` : suggestion?.name || item.title}
              </Text>
              <If when={Boolean(suggestion?.description)}>
                <Text class="text-[11px] text-[#94a3b8] truncate">{suggestion?.description || ""}</Text>
              </If>
            </View>
            <If when={create}>
              <Text class="text-[10px] text-[#94a3b8]">Ctrl ⌘ ↵</Text>
            </If>
            <If when={pickedRow}>
              <Avatar src={ICONS.check} class="w-4 h-4 opacity-70" imageClass="tint-[#2a9d8f] dark:tint-[#2dd4bf]" fallbackDelay={Infinity} accessibilityLabel="" />
            </If>
          </View>
        );
      }}
    />
    </View>
  );
}

/** The date picker, 1:1 with the old app's ui/DayPicker.tsx. nsh Calendar's
 *  itemClass lands on every item (arrows, toggles, weekday labels, days), so
 *  the styling lives in renderItem per CalendarEntry.kind; `group` +
 *  `group-selected:`/`group-data-today:` carry the states (the item keeps its
 *  states as data attrs; the drawing is made once). The header's two toggles
 *  press into month/year views the old app lacks; the year toggle is hidden
 *  and the month toggle carries the "Month Year" title, pressed or not. */
function BooksDate(p: { value: string; onChange: (value: string) => void; today?: string; variant?: "header" | "form" }) {
  const open = signal(false);
  const header = p.variant === "header";
  return (
    <DatePicker
      value={p.value || null}
      open={open()}
      onOpenChange={({ open: next }) => open.set(next)}
      onChange={({ value }) => p.onChange(typeof value === "string" ? value : "")}
      accessibilityLabel="Date"
      dateFormat="%B %-d, %Y"
      presets={p.today ? [{ label: "Today", value: p.today }, { label: "Cancel", value: p.value || p.today }] : []}
      popupClass="rounded-xl border border-[#e2e8f0] bg-white shadow-xl dark:border-slate-700 dark:bg-slate-900"
      popupStyle={{ width: 264 }}
      contentClass="flex flex-col-reverse"
      presetsClass="flex-row items-center justify-between border-t border-[#e2e8f0] dark:border-slate-700 mt-3 mx-4 mb-4 pt-3"
      presetClass="text-xs font-medium transition-colors first:text-[#27ae60] first:dark:text-[#2ecc71] first:hover:underline last:px-4 last:py-1.5 last:text-[#64748b] last:dark:text-slate-400 last:hover:text-[#334155] last:dark:hover:text-slate-200"
      calendarClass="p-4"
      headerClass="mb-2"
      rowClass="gap-y-1"
      firstDayOfWeek={1}
      itemClass="group flex items-center justify-center data-outside:invisible"
      renderItem={(e: Any) => {
        const kind = String(e.kind || "");
        if (kind === "previous") {
          return (
            <View class="w-8 h-8 rounded-md items-center justify-center transition-colors group-hover:bg-[#f1f5f9] dark:group-hover:bg-slate-800">
              <Avatar src={ICONS.chevronLeft} class="w-3.5 h-3.5 group-disabled:opacity-30" imageClass="tint-[#64748b] dark:tint-slate-400" fallbackDelay={Infinity} accessibilityLabel="" />
            </View>
          );
        }
        if (kind === "next") {
          return (
            <View class="w-8 h-8 rounded-md items-center justify-center transition-colors group-hover:bg-[#f1f5f9] dark:group-hover:bg-slate-800">
              <Avatar src={ICONS.chevronRight} class="w-3.5 h-3.5 group-disabled:opacity-30" imageClass="tint-[#64748b] dark:tint-slate-400" fallbackDelay={Infinity} accessibilityLabel="" />
            </View>
          );
        }
        if (kind === "monthToggle" || kind === "month_toggle") {
          return (
            <Text class="flex-1 text-center text-sm font-semibold text-[#1e293b] dark:text-white">
              {`${MONTH_NAMES[Number(e.month || 1) - 1] || ""} ${e.year}`}
            </Text>
          );
        }
        if (kind === "yearToggle" || kind === "year_toggle") {
          return <View class="w-0 h-0 overflow-hidden" />;
        }
        if (kind === "weekday") {
          return (
            <Text class="w-full py-0.5 text-center text-[11px] font-medium text-[#94a3b8] dark:text-slate-500">
              {WEEKDAYS[Number(e.value || 0) % 7] || ""}
            </Text>
          );
        }
        if (kind === "month") {
          // The month view the month toggle opens: a superset of DayPicker.
          return (
            <View class="w-full py-1.5 rounded-md items-center justify-center transition-colors group-selected:bg-[#27ae60] group-hover:bg-[#f1f5f9] dark:group-hover:bg-slate-700 group-selected:group-hover:bg-[#27ae60]">
              <Text class="text-[13px] text-[#334155] dark:text-slate-300 group-selected:text-white group-selected:font-semibold">
                {(MONTH_NAMES[Number(e.month || e.value || 1) - 1] || "").slice(0, 3)}
              </Text>
            </View>
          );
        }
        if (kind === "year") {
          return (
            <View class="w-full py-1.5 rounded-md items-center justify-center transition-colors group-selected:bg-[#27ae60] group-hover:bg-[#f1f5f9] dark:group-hover:bg-slate-700 group-selected:group-hover:bg-[#27ae60]">
              <Text class="text-[13px] text-[#334155] dark:text-slate-300 group-selected:text-white group-selected:font-semibold">
                {e.value}
              </Text>
            </View>
          );
        }
        // Day: a fixed 32px square centred in its column, the states on the square.
        return (
          <View class="w-8 h-8 rounded-md items-center justify-center transition-colors group-selected:bg-[#27ae60] group-hover:bg-[#f1f5f9] dark:group-hover:bg-slate-700 group-selected:group-hover:bg-[#27ae60] group-data-today:ring-1 group-data-today:ring-inset group-data-today:ring-[#27ae60] dark:group-data-today:ring-[#2ecc71]">
            <Text class="text-[13px] text-[#334155] dark:text-slate-300 group-selected:text-white group-selected:font-semibold group-data-today:text-[#27ae60] group-data-today:font-medium dark:group-data-today:text-[#2ecc71] group-selected:group-data-today:text-white">
              {e.value}
            </Text>
          </View>
        );
      }}
      trigger={
        header ? (
          <View class="flex-row items-center justify-center gap-1.5 rounded-md bg-black/20 px-3 py-1.5 transition-colors hover:bg-black/30">
            <Text class="text-sm font-medium text-white/90">{longDate(p.value || p.today || "Select date")}</Text>
            <Avatar src={ICONS.calendar} class="w-3.5 h-3.5" imageClass="tint-white/90" fallbackDelay={Infinity} accessibilityLabel="" />
          </View>
        ) : (
          <View class="min-h-10 flex-row items-center justify-between rounded-lg border border-[#e2e8f0] bg-[#f9fafb] px-3 py-2.5 transition-colors hover:border-[#cbd5e1] dark:border-white/10 dark:bg-white/5 dark:hover:border-white/25">
            <Text class="text-[13px] font-medium text-[#1f2937] dark:text-white">{longDate(p.value || p.today || "Select date")}</Text>
            <Avatar src={ICONS.calendarThin} class="w-4 h-4" imageClass="tint-[#9ca3af] dark:tint-white/40" fallbackDelay={Infinity} accessibilityLabel="" />
          </View>
        )
      }
    />
  );
}

/** Loading, error and retry for a page's resource. */
function Loading(p: { res: Any }) {
  return (
    <View class="gap-2">
      <If when={Boolean(p.res.error())}>
        <View class="gap-2">
          <Text class={`text-[13px] ${C.red}`}>{errText(p.res.error())}</Text>
          <Btn label="Try again" onPress={bump} />
        </View>
      </If>
      <If when={!p.res() && !p.res.error()}>
        <Text class={`text-[13px] ${C.dim}`}>Loading…</Text>
      </If>
    </View>
  );
}
