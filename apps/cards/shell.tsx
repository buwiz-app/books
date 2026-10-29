// The shell every Books inline card shares (drawing: Books Inline Cards):
// the header (B for Books, BIR for a BIR card), rows, a three-number strip,
// a done line, buttons. A card reports its own height with rox.fit, so it
// grows to its content between 96 and 640 points. Books cards are green;
// BIR cards are violet. The kit (kit.tsx) comes first in the source.

const CP = (rox.props || {}) as Any;
const CARD = String(CP.card || "");

let fitSent = 0;
const fitCard = (e: Any) => {
  const h = Math.ceil(e.bounds.size.height);
  if (Math.abs(h - fitSent) >= 1 && typeof rox.fit === "function") rox.fit((fitSent = h));
};

/** Open the Books window at a page (the card's "Open …" link). */
function openBooks(page: string) {
  if (rox.open) rox.open({ app: "books", props: { page } }).catch((e: Any) => toast(errText(e)));
}

function Card(p: { title: string; right?: Any; bir?: boolean; children?: Any }) {
  return (
    <View class={`p-3 gap-2 ${C.panel}`} onLayout={fitCard}>
      <View class="flex-row items-center gap-2">
        <View class={`h-[20px] px-1.5 rounded-md items-center justify-center ${p.bir ? C.violetBg : C.greenBg}`}>
          <Text class="text-[10px] font-extrabold text-white dark:text-[#0d0d0d]">{p.bir ? "BIR" : "B"}</Text>
        </View>
        <Text class={`flex-1 text-[13px] font-bold ${C.ink}`} numberOfLines={1}>
          {p.title}
        </Text>
        {p.right}
      </View>
      {p.children}
      <If when={Boolean(toastText())}>
        <Text class={`text-[11.5px] ${C.mute}`}>{toastText()}</Text>
      </If>
    </View>
  );
}

function Big(p: { text: string }) {
  return <Text class={`text-[22px] font-mono font-bold ${C.ink}`}>{p.text}</Text>;
}

function Line(p: { label: string; sub?: string; value?: string; tone?: string; right?: Any }) {
  return (
    <View class={`flex-row items-center gap-2 py-1 border-t ${C.line}`}>
      <View class="flex-1">
        <Text class={`text-[12.5px] ${C.ink}`} numberOfLines={1}>
          {p.label}
        </Text>
        <If when={Boolean(p.sub)}>
          <Text class={`text-[11px] ${C.dim}`} numberOfLines={1}>
            {p.sub || ""}
          </Text>
        </If>
      </View>
      <If when={Boolean(p.value)}>
        <Text class={`text-[12.5px] font-mono ${p.tone === "pos" ? C.green : p.tone === "neg" ? C.red : C.ink}`}>{p.value || ""}</Text>
      </If>
      {p.right}
    </View>
  );
}

function Strip(p: { items: Any[] }) {
  return (
    <View class="flex-row gap-2">
      <Each of={p.items}>
        {(x: Any) => (
          <View class={`flex-1 rounded-lg px-2 py-1.5 ${C.raise}`}>
            <Text class={`text-[10.5px] uppercase tracking-wider ${C.dim}`}>{x()[0]}</Text>
            <Text class={`text-[14px] font-mono font-bold ${x()[2] === "pos" ? C.green : x()[2] === "warn" ? C.amber : x()[2] === "neg" ? C.red : C.ink}`}>{String(x()[1])}</Text>
          </View>
        )}
      </Each>
    </View>
  );
}

function DoneLine(p: { text: string; sub?: string }) {
  return (
    <View class="gap-0.5">
      <Text class={`text-[13px] font-semibold ${C.green}`}>{`✓ ${p.text}`}</Text>
      <If when={Boolean(p.sub)}>
        <Text class={`text-[11.5px] ${C.mute}`}>{p.sub || ""}</Text>
      </If>
    </View>
  );
}

function Note(p: { text: string }) {
  return <Text class={`text-[11.5px] ${C.mute}`}>{p.text}</Text>;
}

function Acts(p: { children?: Any }) {
  return <View class="flex-row items-center gap-2 flex-wrap">{p.children}</View>;
}

function LinkBtn(p: { label: string; onPress: () => void }) {
  return (
    <View onPress={p.onPress} activeStyle={{ opacity: 0.6 }}>
      <Text class={`text-[12.5px] font-semibold underline ${C.green}`}>{p.label}</Text>
    </View>
  );
}

/** Loading and errors inside a card, at its size. */
function CardWait(p: { res: Any; title: string }) {
  return (
    <If when={!p.res()}>
      <Card title={p.title}>
        <Text class={`text-[12.5px] ${p.res.error() ? C.red : C.dim}`}>{p.res.error() ? errText(p.res.error()) : "Loading…"}</Text>
      </Card>
    </If>
  );
}
