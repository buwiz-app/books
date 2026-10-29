// Buwiz Books, the window (app `books`). This file is the kit every page
// shares; the plugin serves kit, pages and window joined into one source
// (crates/books-mini-app/src/apps.rs). Imports only nsh and rox (spec 6.3);
// data comes from rox.call alone, and every tool called is in the app's
// permissions.tools. Money arrives as display text from Books: the app
// never does arithmetic on an amount (invariant 2).
import { signal, computed, resource, Text, View, Scroll, Input } from "nsh";
import { rox } from "rox";

type Any = any;
const PROPS = (rox.props || {}) as { ticket?: string; page?: string; account?: string; month?: string; filter?: string };

/** Colors: the drawing's dark palette, with a light twin for each. */
const C = {
  bg: "bg-[#f6f6f4] dark:bg-[#151515]",
  rail: "bg-[#efefec] dark:bg-[#0f0f0f]",
  panel: "bg-white dark:bg-[#1d1d1d]",
  raise: "bg-[#f0f0ee] dark:bg-[#262626]",
  hover: "bg-[#e9e9e6] dark:bg-[#2d2d2d]",
  line: "border-black/10 dark:border-[#2b2b2b]",
  ink: "text-[#1c1c1c] dark:text-[#f0f0f0]",
  mute: "text-[#5c5c5c] dark:text-[#a0a0a0]",
  dim: "text-[#8a8a8a] dark:text-[#6e6e6e]",
  green: "text-[#157a48] dark:text-[#3fcf89]",
  greenBg: "bg-[#157a48] dark:bg-[#3fcf89]",
  red: "text-[#c0392b] dark:text-[#ff7d75]",
  redBg: "bg-[#c0392b] dark:bg-[#ff7d75]",
  amber: "text-[#a2650f] dark:text-[#f3b64c]",
  blue: "text-[#2f6fd6] dark:text-[#7fb2ff]",
  blueBg: "bg-[#2f6fd6] dark:bg-[#7fb2ff]",
  violet: "text-[#6a46c9] dark:text-[#b79bff]",
  violetBg: "bg-[#6a46c9] dark:bg-[#b79bff]",
};

/** Pill tones (`st` in the drawing). */
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

/** A client_request_id: unique per call, never a subject (DD-17). */
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
    const r = await rox.call(tool, { client_request_id: rid(), ...args });
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
      ? `${C.greenBg} border-[#157a48] dark:border-[#3fcf89]`
      : p.tone === "vi"
        ? `${C.violetBg} border-[#6a46c9] dark:border-[#b79bff]`
        : `${C.raise} ${C.line}`;
  const ink = p.tone === "go" ? "text-white dark:text-[#06281a]" : p.tone === "vi" ? "text-white dark:text-[#1a1033]" : C.ink;
  return (
    <View
      class={`rounded-lg border ${p.small ? "px-2 py-0.5" : "px-3 py-1.5"} items-center justify-center ${tone} ${p.disabled ? "opacity-40" : ""}`}
      activeStyle={{ opacity: 0.7 }}
      onPress={() => (p.disabled ? null : p.onPress())}
    >
      <Text class={`text-[12.5px] font-semibold ${ink}`} numberOfLines={1}>
        {p.label}
      </Text>
    </View>
  );
}

function Pill(p: { tone: string; label: string }) {
  return (
    <View class={`rounded-full px-2 py-0.5 self-start ${PILL[p.tone] || PILL.mute}`}>
      <Text class={`text-[11px] ${PILL[p.tone] || PILL.mute}`} numberOfLines={1}>
        {p.label}
      </Text>
    </View>
  );
}

function Chip(p: { label: string; count?: Any; on: boolean; onPress: () => void }) {
  return (
    <View
      class={`rounded-full px-2.5 py-1 border flex-row gap-1 ${p.on ? "bg-[#1c1c1c] dark:bg-[#f0f0f0] border-transparent" : `${C.panel} ${C.line}`}`}
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
          <View class={`rounded-[7px] px-2.5 py-1 ${o()[0] === p.value ? C.raise : ""}`} onPress={() => p.onPick(o()[0])}>
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

/** The page header: title, sub, buttons on the right. */
function Head(p: { title: string; sub?: string; children?: Any }) {
  return (
    <View class="flex-row items-center gap-2.5 flex-wrap">
      <Text class={`text-[18px] font-bold ${C.ink}`}>{p.title}</Text>
      <If when={Boolean(p.sub)}>
        <Text class={`text-[13px] ${C.mute}`}>{p.sub || ""}</Text>
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
      <Text class={`text-[11.5px] w-[104px] ${C.mute}`} numberOfLines={1}>
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
function Field(p: { label: string; children?: Any; grow?: boolean }) {
  return (
    <View class={`gap-1 ${p.grow ? "flex-1" : ""}`}>
      <Text class={`text-[11px] uppercase tracking-wider ${C.dim}`}>{p.label}</Text>
      {p.children}
    </View>
  );
}

function TextBox(p: { value: string; onChange: (v: string) => void; placeholder?: string; big?: boolean }) {
  return (
    <Input
      value={p.value}
      placeholder={p.placeholder || ""}
      onChange={(e: Any) => p.onChange(typeof e === "string" ? e : e.value)}
      class={`placeholder:text-[#8e8e93] rounded-lg border px-2.5 ${p.big ? "h-10 text-[18px] font-mono" : "h-8 text-[13px]"} ${C.bg} ${C.line} ${C.ink}`}
    />
  );
}

/** A picker: the current choice; a press opens the list under it. */
function Pick(p: { value: string; options: Any[]; onPick: (o: Any) => void; placeholder?: string }) {
  const open = signal(false);
  const current = computed(() => (p.options || []).find((o: Any) => o.ticket === p.value));
  return (
    <View class="gap-1">
      <View class={`flex-row items-center rounded-lg border px-2.5 h-8 ${C.bg} ${C.line}`} onPress={() => open.set(!open())}>
        <Text class={`flex-1 text-[13px] ${current() ? C.ink : C.dim}`} numberOfLines={1}>
          {current() ? current().label || current().name : p.placeholder || "Pick one"}
        </Text>
        <Text class={C.dim}>⌄</Text>
      </View>
      <If when={open()}>
        <View class={`rounded-lg border max-h-[180px] ${C.panel} ${C.line}`}>
          <Scroll class="max-h-[178px]">
            <Each of={p.options || []} key={rowKey}>
              {(o: Any) => (
                <View
                  class={`px-2.5 py-1.5 ${o().ticket === p.value ? C.raise : ""}`}
                  onPress={() => {
                    p.onPick(o());
                    open.set(false);
                  }}
                >
                  <Text class={`text-[12.5px] ${C.ink}`} numberOfLines={1}>
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
