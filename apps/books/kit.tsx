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
          <View class={`rounded-[7px] px-2.5 py-1 transition-colors ${o()[0] === p.value ? C.raise : "hover:bg-[#e2e8f0] dark:hover:bg-[#26262a]"}`} onPress={() => p.onPick(o()[0])}>
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
        ? "w-28 h-8 text-right text-xl font-semibold text-white tabular-nums bg-transparent border-none outline-none placeholder-white/40 focus:ring-0"
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
      <View accessibilityLabel={p.label} class={`flex-row items-center rounded-lg border px-2.5 h-8 transition-colors ${C.bg} ${C.line} hover:border-[#cbd5e1] dark:hover:border-[#3f3f46]`} onPress={() => open.set(!open())}>
        <Text accessibilityElementsHidden={Boolean(p.label)} class={`flex-1 text-[13px] ${current() ? C.ink : C.dim} truncate`}>
          {current() ? current().label || current().name : p.placeholder || "Pick one"}
        </Text>
        <Text accessibilityElementsHidden={Boolean(p.label)} class={C.dim}>
          ⌄
        </Text>
      </View>
      <If when={open()}>
        <View class={`rounded-lg border max-h-[180px] ${C.panel} ${C.line}`}>
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
  return (
    <View class={chip ? "relative w-full" : ""}>
      {chip && (
        <View
          class={`absolute inset-0 flex-row items-center gap-2 rounded-lg border bg-white px-3 py-2 transition-colors cursor-pointer dark:bg-slate-800 ${open() ? "border-[#2a9d8f] dark:border-[#2dd4bf]" : "border-[#e5e7eb] hover:border-[#d1d5db] dark:border-slate-600 dark:hover:border-[#64748b]"}`}
          onPress={() => open.set(!open())}
        >
          <If when={Boolean(p.icon)}>
            <View class="w-5 h-5 items-center justify-center shrink-0">
              <Text class="text-[14px] text-[#94a3b8] dark:text-slate-500">{p.icon || ""}</Text>
            </View>
          </If>
          <If when={!typing()}>
            <Text
              class={`flex-1 min-w-0 truncate text-sm ${p.value ? "text-[#1e293b] dark:text-slate-100" : "text-[#94a3b8] dark:text-slate-500"}`}
            >
              {chipTitle()}
            </Text>
          </If>
          <Text class="shrink-0 text-[15px] text-[#1e293b] opacity-40 dark:text-slate-100">⌄</Text>
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
          !chip && p.icon ? (
            <Text class="text-[14px] text-[#94a3b8] dark:text-slate-500">{p.icon}</Text>
          ) : undefined
        }
        inputClass={
          chip
            ? `w-full py-2 text-sm ${p.icon ? "pl-[38px] pr-3" : "px-3"} ${typing() ? "text-[#1e293b] dark:text-slate-100" : "opacity-0"}`
            : "min-h-10 rounded-lg border border-[#e2e8f0] bg-white px-3 py-2 text-[13px] text-[#0f172a] transition-colors focus:outline-none focus:ring-1 focus:ring-[#2a9d8f] focus:border-[#2a9d8f] dark:border-slate-600 dark:bg-slate-800 dark:text-slate-100"
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
          chip ? <View class="w-0 h-0" /> : <Text class="text-[16px] text-[#94a3b8] opacity-60">⌄</Text>
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
          ? "+"
          : group.includes("liabilit")
            ? "⇄"
            : group.includes("equity")
              ? "◆"
              : income
                ? "↗"
                : group.includes("expense")
                  ? "↘"
                  : "▤";
        return (
          <View class="flex-row items-center gap-3">
            <If when={create || suggested}>
              <Text class="w-5 text-[14px] font-semibold" style={{ color }}>{icon}</Text>
            </If>
            <If when={!create && !suggested}>
              <View class="w-5 h-5 rounded items-center justify-center" style={{ backgroundColor: `${color}22` }}>
                <Text class="text-[11px] font-semibold" style={{ color }}>{icon}</Text>
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
              <Text class="text-[16px]" style={{ color: "#2a9d8f" }}>✓</Text>
            </If>
          </View>
        );
      }}
    />
    </View>
  );
}

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
      presets={p.today ? [{ label: "Today", value: p.today }] : []}
      popupClass="rounded-xl border border-[#e2e8f0] bg-white shadow-xl dark:border-slate-700 dark:bg-slate-900"
      popupStyle={{ width: 280 }}
      calendarClass="p-5"
      headerClass="mb-3"
      itemClass="rounded-md py-1.5 text-[13px] text-[#334155] dark:text-slate-300 selected:bg-[#27ae60] selected:text-white selected:font-semibold data-today:ring-1 data-today:ring-[#27ae60] data-today:text-[#27ae60] dark:data-today:text-[#2ecc71] data-today:font-medium data-outside:text-[#cbd5e1] dark:data-outside:text-slate-600"
      trigger={
        header ? (
          <View class="flex-row items-center justify-center gap-1.5 rounded-md bg-black/20 px-3 py-1.5 hover:bg-black/30">
            <Text class="text-sm font-medium text-white/90">{longDate(p.value || p.today || "Select date")}</Text>
            <Text class="text-[14px] text-white/90">▦</Text>
          </View>
        ) : (
          <View class="min-h-10 flex-row items-center justify-between rounded-lg border border-[#e2e8f0] bg-[#f9fafb] px-3 py-2.5 transition-colors hover:border-[#cbd5e1] dark:border-white/10 dark:bg-white/5 dark:hover:border-white/25">
            <Text class="text-[13px] font-medium text-[#1f2937] dark:text-white">{longDate(p.value || p.today || "Select date")}</Text>
            <Text class="text-[16px] text-[#9ca3af] dark:text-white/40">▦</Text>
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
