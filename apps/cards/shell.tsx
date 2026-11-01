// The shell every Books inline card shares (docs/design/inline-cards.md). A card sits inside the Bot's card frame,
// which draws the one header (the title, refresh, maximize); Books draws only the body, in the Bot's own card
// language (the Bot's app/src/cards/mod.rs: 12 padding, 8 gap, 13 text, 12 notes, 28-point pill buttons, primary
// first and right-aligned):
//   Meta   one line under the header: when, and a status pill. Never the title again.
//   body   the class's content. A list scrolls inside its own max height; the card never scrolls.
//   Foot   the actions, on a hairline. It never scrolls and is never cut off.
// The four classes: Glance (one figure, a few rows), List (rows that may scroll), Decision (what is decided,
// then its buttons), Notice (one line, one action). Loading, empty, error and settled stay in the card's one
// body. The card reports its natural height with buwiz.fit; each class's parts are bounded, so its height never
// passes the manifest's max_height. The kit (kit.tsx) comes first in the source.

const CP = (buwiz.props || {}) as Any;
const CARD = String(CP.card || "");

/** The Bot's card surface, hairline, button well and a skeleton bar, as solid colors (theme CARD, BORDER_S, HOVER). */
const K = {
  surface: "bg-white dark:bg-[#1f1f1f]",
  line: "border-[#e0e0e0] dark:border-[#3a3a3a]",
  well: "bg-[#ebebeb] dark:bg-[#333333]",
  skel: "bg-[#efefef] dark:bg-[#2c2c2c]",
};

/** A list's own max height, about seven rows: past it the list scrolls, the card and its footer do not. */
const LIST_MAX = "max-h-[240px]";

let fitSent = 0;
const fitCard = (e: Any) => {
  const h = Math.ceil(e.bounds.size.height);
  if (Math.abs(h - fitSent) >= 1 && typeof buwiz.fit === "function") buwiz.fit((fitSent = h));
};

/** The card's reads (`read`): the Bot hears `buwiz.ready()` once every read the card started has settled, data or
 *  error, and the manifest's `inline.wait_ready` holds the Bot's placeholder until then. A card that starts none says
 *  so at once. A Bot without `buwiz.ready` ignores it. */
let reading = 0;
let readStarted = false;
let readyTold = false;
function tellReady() {
  if (readyTold || reading > 0) return;
  readyTold = true;
  (buwiz as Any).ready?.();
}
/** nsh's `resource`, counted for `buwiz.ready`. A read that ends lets the others it set off start first. */
function read(source: Any, fetcher: Any) {
  return resource(source, (...a: Any[]) => {
    reading++;
    readStarted = true;
    return new Promise((ok) => ok(fetcher(...a))).finally(() => {
      reading--;
      setTimeout(tellReady, 0);
    });
  });
}
setTimeout(() => readStarted || tellReady(), 0);

/** Open the Books window at a page. */
function openBooks(page: string) {
  if (buwiz.open) buwiz.open({ app: "books", props: { page } }).catch((e: Any) => toast(errText(e)));
}

/** The card: its body, then its footer when it has one. The whole is measured for buwiz.fit. The Bot draws no card
 *  shorter than 96 points (mini_v1 HEIGHT), so the body fills that: a footer sits at the bottom, and a body
 *  with no footer (a settled line) is centered rather than leaving a gap under it. */
function Body(p: { children?: Any; foot?: Any; center?: boolean }) {
  return (
    <View class={`min-h-[96px] ${p.center ? "justify-center" : "justify-between"} ${K.surface}`} onLayout={fitCard}>
      <View class="px-3 pt-3 pb-3 gap-2">{p.children}</View>
      {p.foot || null}
    </View>
  );
}

/** The line under the Bot's header: when or what, and a status pill on the right. */
function Meta(p: { text: string; right?: Any }) {
  return (
    <View class="flex-row items-center gap-2 min-h-[20px]">
      <Text class={`flex-1 text-[12px] ${C.mute}`} numberOfLines={1}>
        {p.text}
      </Text>
      {p.right || null}
    </View>
  );
}

/** A Glance card's one figure. */
function Hero(p: { text: string; tone?: string }) {
  return <Text class={`text-[24px] leading-[30px] font-mono font-semibold ${p.tone === "pos" ? C.green : p.tone === "neg" ? C.red : C.ink}`}>{p.text}</Text>;
}

/** A row on a hairline: a label (and a line under it), then an amount (mono, and last period's), a name or
 *  other words (`text`), or something on the right. */
function Row(p: { label: string; sub?: string; value?: string; prev?: string; text?: string; tone?: string; right?: Any; strong?: boolean }) {
  const weight = p.strong ? "font-semibold" : "";
  return (
    <View class={`flex-row items-center gap-3 min-h-[32px] py-1 border-t ${K.line}`}>
      <View class="flex-1 gap-0.5">
        <Text class={`text-[13px] ${weight} ${C.ink}`} numberOfLines={1}>
          {p.label}
        </Text>
        <If when={Boolean(p.sub)}>
          <Text class={`text-[12px] ${C.mute}`} numberOfLines={1}>
            {p.sub || ""}
          </Text>
        </If>
      </View>
      <If when={Boolean(p.value) || p.prev !== undefined}>
        <Text class={`${p.prev !== undefined ? "w-[96px] text-right" : ""} text-[13px] font-mono ${weight} ${p.tone === "pos" ? C.green : p.tone === "neg" ? C.red : C.ink}`}>{p.value || ""}</Text>
      </If>
      <If when={p.prev !== undefined}>
        <Text class={`w-[96px] text-right text-[13px] font-mono ${C.mute}`}>{p.prev || ""}</Text>
      </If>
      <If when={Boolean(p.text)}>
        <Text class={`text-[13px] ${C.ink}`} numberOfLines={1}>
          {p.text || ""}
        </Text>
      </If>
      {p.right || null}
    </View>
  );
}

/** Rows that may outgrow the card: they scroll inside LIST_MAX, without bounce. */
function List(p: { children?: Any }) {
  return (
    <View class={LIST_MAX}>
      <Scroll class={LIST_MAX} bounces={false}>
        <View>{p.children}</View>
      </Scroll>
    </View>
  );
}

/** A plain sentence in the body. */
function Say(p: { text: string; dim?: boolean }) {
  return <Text class={`text-[13px] leading-[18px] ${p.dim ? C.mute : C.ink}`}>{p.text}</Text>;
}

/** The card's actions on a hairline, primary first and right-aligned; a status line, or the last write's error, on the left. */
function Foot(p: { status?: string; children?: Any }) {
  return (
    <View class={`flex-row items-center gap-2 px-3 py-2.5 border-t ${K.line}`}>
      <Text class={`flex-1 text-[12px] ${toastText() ? C.red : C.mute}`} numberOfLines={1}>
        {toastText() || p.status || ""}
      </Text>
      {p.children}
    </View>
  );
}

/** A card button, as the Bot draws them: a 28-point pill; the primary is ink on the surface, the rest sit in a well. */
function Act(p: { label: string; onPress: () => void; primary?: boolean; disabled?: boolean }) {
  return (
    <View
      class={`h-[28px] px-3 rounded-full items-center justify-center ${p.primary ? "bg-[#1c1c1c] dark:bg-[#f5f5f5]" : K.well} ${p.disabled ? "opacity-40" : ""}`}
      activeStyle={{ opacity: 0.7 }}
      onPress={() => (p.disabled ? null : p.onPress())}
    >
      <Text class={`text-[12.5px] font-medium ${p.primary ? "text-white dark:text-[#1f1f1f]" : C.ink}`} numberOfLines={1}>
        {p.label}
      </Text>
    </View>
  );
}

/** A checklist step: a box, ticked when done. */
function Check(p: { label: string; done: boolean }) {
  return (
    <View class={`flex-row items-center gap-2 min-h-[32px] border-t ${K.line}`}>
      <View class={`w-[18px] h-[18px] rounded-md items-center justify-center border ${p.done ? `${C.greenBg} border-transparent` : K.line}`}>
        <Text class="text-[11px] font-bold text-white dark:text-[#0d0d0d]">{p.done ? "✓" : ""}</Text>
      </View>
      <Text class={`flex-1 text-[13px] ${p.done ? C.mute : C.ink}`}>{p.label}</Text>
    </View>
  );
}

/** What a Decision card leaves once its action is done: one line, as the Bot's answered cards do. */
function Settled(p: { text: string; sub?: string }) {
  return (
    <Body center>
      <View class="flex-row items-center gap-2">
        <View class={`w-[18px] h-[18px] rounded-full items-center justify-center ${C.greenBg}`}>
          <Text class="text-[11px] font-bold text-white dark:text-[#0d0d0d]">✓</Text>
        </View>
        <Text class={`flex-1 text-[13px] font-medium ${C.ink}`} numberOfLines={1}>
          {p.text}
        </Text>
      </View>
      <If when={Boolean(p.sub)}>
        <Text class={`text-[12px] ${C.mute}`}>{p.sub || ""}</Text>
      </If>
    </Body>
  );
}

/** Nothing to show: a sentence, and where to go. */
function Empty(p: { text: string; sub?: string; open?: string; page?: string }) {
  return (
    <Body
      foot={
        p.open ? (
          <Foot>
            <Act label={p.open} onPress={() => openBooks(p.page || "today")} />
          </Foot>
        ) : null
      }
    >
      <Say text={p.text} />
      <If when={Boolean(p.sub)}>
        <Say text={p.sub || ""} dim />
      </If>
    </Body>
  );
}

/** Loading, or a failed read with Try again, in the card's one body. A read that found nothing is not loading. */
function Wait(p: { res: Any }) {
  return (
    <If when={p.res() === undefined}>
      <Body
        foot={
          <If when={Boolean(p.res.error())}>
            <Foot>
              <Act label="Try again" onPress={bump} />
            </Foot>
          </If>
        }
      >
        <If when={Boolean(p.res.error())}>
          <Text class={`text-[13px] ${C.red}`}>{p.res.error() ? errText(p.res.error()) : ""}</Text>
        </If>
        <If when={!p.res.error()}>
          <View class={`h-[12px] w-[120px] rounded ${K.skel}`} />
          <View class={`h-[22px] w-[220px] rounded ${K.skel}`} />
          <View class={`h-[12px] w-[180px] rounded ${K.skel}`} />
        </If>
      </Body>
    </If>
  );
}
