// A headless stand-in for nsh, for driving the Books window from the host test (tools/bot/host-test.sh).
//
// It runs an app's served TSX the way nsh does where the window's controls depend on it:
//   - JSX compiles to lazy calls (every attribute and child is a thunk, as nsh's Solid-style compiler makes them), so
//     components run once, `<If>` children are evaluated only when shown, and a page keeps its signals across reads;
//   - `signal().set` is a method that needs its signal as `this` and throws `TypeError: not a live signal` otherwise,
//     as nsh-js's `signal_set` does (crates/nsh-js/src/reactive.rs, `this_signal`); a detached `x.set` passed as a
//     prop and called as `p.onChange(v)` fails here as it does in the Bot;
//   - `<Input>`'s onChange gets `{ value }` (nsh docs/dev/elements.md).
// It draws nothing: the tree is host nodes ({host, props, kids}) to query and press. Imports allowed are those of
// spec 6.3 (`nsh`, `buwiz`); the caller supplies the bridge.

// ---- the reactive graph: signals, memos, effects, owners (Solid's algorithm, CLEAN / CHECK / DIRTY) ----

const CLEAN = 0;
const CHECK = 1;
const DIRTY = 2;
let Listener = null;
let Owner = null;
let flushing = false;
const queue = [];

function track(node) {
  if (Listener && !Listener.disposed) {
    Listener.sources.add(node);
    node.observers.add(Listener);
  }
}

function mark(c, state) {
  if (c.disposed || c.state >= state) return;
  const was = c.state;
  c.state = state;
  if (c.effect && was === CLEAN) queue.push(c);
  for (const o of c.observers) mark(o, CHECK);
}

function write(node, value) {
  if (Object.is(node.value, value)) return;
  node.value = value;
  for (const o of [...node.observers]) mark(o, DIRTY);
  flush();
}

function flush() {
  if (flushing) return;
  flushing = true;
  try {
    while (queue.length) update(queue.shift());
  } finally {
    flushing = false;
  }
}

function disposeOwned(c) {
  for (const k of c.owned.splice(0)) dispose(k);
  for (const f of c.cleanups.splice(0)) f();
}

function dispose(c) {
  if (c.disposed) return;
  c.disposed = true;
  for (const s of c.sources) s.observers.delete(c);
  c.sources.clear();
  disposeOwned(c);
}

function computation(fn, effect) {
  const c = { fn, effect, state: DIRTY, value: undefined, sources: new Set(), observers: new Set(), owned: [], cleanups: [], disposed: false };
  if (Owner) Owner.owned.push(c);
  return c;
}

function run(c) {
  for (const s of c.sources) s.observers.delete(c);
  c.sources.clear();
  disposeOwned(c);
  const [pl, po] = [Listener, Owner];
  Listener = c;
  Owner = c;
  let value;
  try {
    value = c.fn();
  } finally {
    Listener = pl;
    Owner = po;
  }
  c.state = CLEAN;
  if (!Object.is(value, c.value)) {
    c.value = value;
    for (const o of c.observers) if (o.state < DIRTY && !o.disposed) mark(o, DIRTY);
  }
}

function update(c) {
  if (c.disposed || c.state === CLEAN) return;
  if (c.state === CHECK) {
    for (const s of [...c.sources]) {
      if (s.fn) update(s);
      if (c.state === DIRTY) break;
    }
  }
  if (c.state === DIRTY) run(c);
  else c.state = CLEAN;
}

function memo(fn) {
  const c = computation(fn, false);
  return () => {
    update(c);
    track(c);
    return c.value;
  };
}

function effect(fn) {
  const c = computation(fn, true);
  update(c);
}

function untrack(fn) {
  const pl = Listener;
  Listener = null;
  try {
    return fn();
  } finally {
    Listener = pl;
  }
}

function root(fn) {
  const r = { owned: [], cleanups: [], disposed: false, sources: new Set(), observers: new Set() };
  const [pl, po] = [Listener, Owner];
  Listener = null;
  Owner = r;
  try {
    return [fn(), () => dispose(r)];
  } finally {
    Listener = pl;
    Owner = po;
  }
}

// ---- `nsh`: signal, computed, resource and the host views ----

const LIVE = new WeakMap();
const SignalProto = Object.create(Function.prototype);
function liveSignal(self) {
  const node = LIVE.get(self);
  if (!node) throw new TypeError("not a live signal");
  return node;
}
SignalProto.set = function set(value) {
  write(liveSignal(this), value);
};
SignalProto.update = function update_(f) {
  const node = liveSignal(this);
  write(node, f(node.value));
};

export function signal(value) {
  const node = { value, observers: new Set() };
  const read = function () {
    track(node);
    return node.value;
  };
  Object.setPrototypeOf(read, SignalProto);
  LIVE.set(read, node);
  return read;
}

export const computed = (fn) => memo(fn);

export function resource(source, fetcher) {
  if (fetcher === undefined) [source, fetcher] = [() => true, source];
  const value = signal(undefined);
  const error = signal(undefined);
  const loading = signal(false);
  let seq = 0;
  let alive = true;
  if (Owner) Owner.cleanups.push(() => (alive = false));
  const start = (key) => {
    const id = (seq += 1);
    loading.set(true);
    let p;
    try {
      p = Promise.resolve(fetcher(key));
    } catch (e) {
      p = Promise.reject(e);
    }
    p.then(
      (v) => {
        if (!alive || id !== seq) return;
        error.set(undefined);
        value.set(v);
        loading.set(false);
      },
      (e) => {
        if (!alive || id !== seq) return;
        error.set(e);
        loading.set(false);
      },
    );
  };
  effect(() => {
    const key = source();
    if (key === false || key === null || key === undefined) return;
    untrack(() => start(key));
  });
  const read = () => value();
  read.error = () => error();
  read.loading = () => loading();
  read.refetch = () => start(untrack(source));
  return read;
}

const HOST = new Map();
const host = (name) => {
  const f = () => {
    throw new Error(`${name} is drawn by nsh, not called`);
  };
  HOST.set(f, name);
  return f;
};
export const View = host("View");
export const Text = host("Text");
export const Scroll = host("Scroll");
export const Input = host("Input");
const Fragment = Symbol("Fragment");

// ---- what the compiled JSX calls ----

function makeProps(attrs, kids) {
  const props = {};
  for (const [name, thunk] of attrs) {
    if (name === "...") {
      const keys = Object.keys(untrack(thunk) || {});
      for (const k of keys) Object.defineProperty(props, k, { get: () => (thunk() || {})[k], enumerable: true, configurable: true });
    } else Object.defineProperty(props, name, { get: thunk, enumerable: true, configurable: true });
  }
  if (kids && kids.length) {
    Object.defineProperty(props, "children", { get: () => (kids.length === 1 ? kids[0]() : kids.map((k) => k())), enumerable: true, configurable: true });
  }
  return props;
}

function h(tag, attrs, kids) {
  if (tag === Fragment) return kids.map((k) => memo(k));
  const name = HOST.get(tag);
  if (name) return { host: name, props: makeProps(attrs, null), kids: kids.map((k) => memo(k)) };
  if (typeof tag !== "function") throw new Error(`unknown element ${String(tag)}`);
  const props = makeProps(attrs, kids);
  return untrack(() => tag(props));
}

/** nsh's compiler built-in `<If when>`: its children only while `when` holds. */
function If(p) {
  const shown = memo(() => Boolean(p.when));
  return memo(() => (shown() ? untrack(() => p.children) : null));
}

/** nsh's compiler built-in `<Each of key>`: a row per item, the child called with accessors; rows kept by key. */
function Each(p) {
  const list = memo(() => p.of || []);
  let rows = new Map();
  if (Owner) Owner.cleanups.push(() => rows.forEach((r) => r.dispose()));
  return memo(() => {
    const items = list();
    const keyOf = untrack(() => p.key);
    const fn = untrack(() => p.children);
    const next = new Map();
    const out = items.map((item, i) => {
      const k = keyOf ? keyOf(item) : item;
      let row = rows.get(k);
      if (row) rows.delete(k);
      else {
        const [node, dispose_] = root(() => fn(() => item, () => i));
        row = { node, dispose: dispose_ };
      }
      next.set(next.has(k) ? Symbol("dup") : k, row);
      return row.node;
    });
    rows.forEach((r) => r.dispose());
    rows = next;
    return out;
  });
}

// ---- JSX to lazy calls ----

/** esbuild's `jsx: "preserve"` output (TS stripped, JSX kept) to JS calling `__h` with thunks. */
export function lazyJsx(src) {
  let i = 0;
  const err = (m) => {
    throw new Error(`lazyJsx: ${m} at ${i}: ${JSON.stringify(src.slice(i, i + 40))}`);
  };
  const isId = (c) => /[A-Za-z0-9_$]/.test(c || "");
  const prevSignificant = (out) => {
    const m = /(\S)\s*$/.exec(out);
    if (!m) return "";
    const w = /([A-Za-z_$][\w$]*)\s*$/.exec(out);
    return w ? w[1] : m[1];
  };
  const regexMayStart = (out) => {
    const p = prevSignificant(out);
    return p === "" || /^[(,=:[!&|?{};+\-*%<>~^]$/.test(p) || ["return", "typeof", "case", "do", "else", "in", "of", "void", "yield", "await"].includes(p);
  };

  // JS until an unmatched `close` (or the end); returns the code with JSX turned into calls
  function code(close) {
    let out = "";
    let depth = 0;
    while (i < src.length) {
      const c = src[i];
      if (c === "/" && src[i + 1] === "/") {
        const e = src.indexOf("\n", i);
        i = e < 0 ? src.length : e;
        continue;
      }
      if (c === "/" && src[i + 1] === "*") {
        const e = src.indexOf("*/", i + 2);
        if (e < 0) err("unclosed comment");
        i = e + 2;
        out += " ";
        continue;
      }
      if (c === '"' || c === "'") {
        const s = i;
        i += 1;
        while (i < src.length && src[i] !== c) i += src[i] === "\\" ? 2 : 1;
        i += 1;
        out += src.slice(s, i);
        continue;
      }
      if (c === "`") {
        out += c;
        i += 1;
        while (i < src.length && src[i] !== "`") {
          if (src[i] === "\\") {
            out += src.slice(i, i + 2);
            i += 2;
          } else if (src[i] === "$" && src[i + 1] === "{") {
            i += 2;
            out += "${" + code("}") + "}";
            i += 1;
          } else {
            out += src[i];
            i += 1;
          }
        }
        out += "`";
        i += 1;
        continue;
      }
      if (c === "/" && regexMayStart(out)) {
        const s = i;
        i += 1;
        let cls = false;
        while (i < src.length && (src[i] !== "/" || cls)) {
          if (src[i] === "\\") i += 1;
          else if (src[i] === "[") cls = true;
          else if (src[i] === "]") cls = false;
          i += 1;
        }
        i += 1;
        while (isId(src[i])) i += 1;
        out += src.slice(s, i);
        continue;
      }
      if (c === "<" && /[A-Za-z_$>]/.test(src[i + 1] || "") && regexMayStart(out)) {
        out += element();
        continue;
      }
      if (c === "(" || c === "[" || c === "{") depth += 1;
      if (c === ")" || c === "]" || c === "}") {
        if (depth === 0 && c === close) return out;
        depth -= 1;
      }
      out += c;
      i += 1;
    }
    if (close) err(`missing ${close}`);
    return out;
  }

  const ws = () => {
    while (/\s/.test(src[i] || "")) i += 1;
  };
  const name = () => {
    const s = i;
    while (/[A-Za-z0-9_$.:-]/.test(src[i] || "")) i += 1;
    if (s === i) err("expected a name");
    return src.slice(s, i);
  };
  const text = (raw) => {
    const lines = raw.split("\n");
    const kept = lines
      .map((l, n) => {
        let t = l.replace(/\t/g, " ");
        if (n > 0) t = t.replace(/^\s+/, "");
        if (n < lines.length - 1) t = t.replace(/\s+$/, "");
        return t;
      })
      .filter((t) => t !== "");
    return kept.join(" ");
  };

  function element() {
    i += 1; // <
    let tag = "__Fragment";
    const attrs = [];
    const kids = [];
    if (src[i] === ">") {
      i += 1;
    } else {
      tag = name();
      for (;;) {
        ws();
        if (src.startsWith("/>", i)) {
          i += 2;
          return `__h(${tag}, [${attrs.join(", ")}], [])`;
        }
        if (src[i] === ">") {
          i += 1;
          break;
        }
        if (src[i] === "{") {
          i += 1;
          ws();
          if (!src.startsWith("...", i)) err("expected a spread");
          i += 3;
          attrs.push(`["...", () => (${code("}")})]`);
          i += 1;
          continue;
        }
        const n = name();
        ws();
        if (src[i] !== "=") {
          attrs.push(`[${JSON.stringify(n)}, () => true]`);
          continue;
        }
        i += 1;
        ws();
        if (src[i] === '"' || src[i] === "'") {
          const q = src[i];
          const e = src.indexOf(q, i + 1);
          attrs.push(`[${JSON.stringify(n)}, () => ${JSON.stringify(src.slice(i + 1, e))}]`);
          i = e + 1;
        } else if (src[i] === "{") {
          i += 1;
          attrs.push(`[${JSON.stringify(n)}, () => (${code("}")})]`);
          i += 1;
        } else if (src[i] === "<") {
          attrs.push(`[${JSON.stringify(n)}, () => (${element()})]`);
        } else err("expected an attribute value");
      }
    }
    // children
    for (;;) {
      if (src.startsWith("</", i)) {
        i += 2;
        ws();
        if (tag !== "__Fragment") {
          const closing = name();
          if (closing !== tag) err(`</${closing}> closes <${tag}>`);
          ws();
        }
        if (src[i] !== ">") err("expected >");
        i += 1;
        break;
      }
      if (src[i] === "<") {
        kids.push(`() => (${element()})`);
      } else if (src[i] === "{") {
        i += 1;
        const e = code("}");
        i += 1;
        if (e.trim()) kids.push(`() => (${e})`);
      } else {
        const s = i;
        while (i < src.length && src[i] !== "<" && src[i] !== "{") i += 1;
        if (i >= src.length) err(`unclosed <${tag}>`);
        const t = text(src.slice(s, i));
        if (t) kids.push(`() => ${JSON.stringify(t)}`);
      }
    }
    return `__h(${tag}, [${attrs.join(", ")}], [${kids.join(", ")}])`;
  }

  return code("");
}

// ---- compile, mount, query ----

/**
 * Compile an app's TSX with the Bot's esbuild and mount its default export.
 * `esbuild` is the module (the Bot worktree's); `bridge` is the `buwiz` object.
 */
export function mountApp(source, { esbuild, bridge }) {
  const stripped = esbuild.transformSync(source, { loader: "tsx", jsx: "preserve", format: "esm", target: "es2022" }).code;
  const lazy = lazyJsx(stripped);
  const cjs = esbuild.transformSync(lazy, { loader: "js", format: "cjs", target: "es2022" }).code;
  const nsh = { signal, computed, resource, View, Text, Scroll, Input };
  const modules = { nsh, buwiz: { buwiz: bridge, default: bridge } };
  const require = (id) => {
    if (!(id in modules)) throw new Error(`the app imports ${id}; apps may import only nsh and buwiz`);
    return modules[id];
  };
  const module = { exports: {} };
  new Function("require", "module", "exports", "buwiz", "__h", "__Fragment", "If", "Each", cjs)(require, module, module.exports, bridge, h, Fragment, If, Each);
  const App = module.exports.default;
  if (typeof App !== "function") throw new Error("the app has no default export");
  const [tree, unmount] = root(() => h(App, [], []));
  return new Screen(tree, unmount);
}

function resolve(v, out) {
  if (v === null || v === undefined || v === false || v === true) return out;
  if (Array.isArray(v)) v.forEach((x) => resolve(x, out));
  else if (typeof v === "function") resolve(v(), out);
  else if (typeof v === "object" && v.host) out.push(v);
  else out.push(String(v));
  return out;
}

const childrenOf = (node) => node.kids.flatMap((k) => resolve(k(), []));

export function textOf(node) {
  return childrenOf(node)
    .map((c) => (typeof c === "string" ? c : textOf(c)))
    .join("");
}

class Screen {
  constructor(tree, unmount) {
    this.tree = tree;
    this.unmount = unmount;
  }

  /** Every host node on screen now, in tree order. */
  nodes() {
    const out = [];
    const walk = (n) => {
      out.push(n);
      for (const c of childrenOf(n)) if (typeof c !== "string") walk(c);
    };
    for (const n of resolve(this.tree, [])) if (typeof n !== "string") walk(n);
    return out;
  }

  /** The innermost pressable node whose whole text is `label`. */
  pressable(label) {
    const hits = this.nodes().filter((n) => typeof n.props.onPress === "function" && textOf(n).trim() === label);
    return hits[hits.length - 1] || null;
  }

  input(placeholder) {
    return this.nodes().find((n) => n.host === "Input" && n.props.placeholder === placeholder) || null;
  }

  hasText(text) {
    return this.nodes().some((n) => n.host === "Text" && textOf(n).includes(text));
  }

  layout(height = 120) {
    for (const n of this.nodes()) {
      if (typeof n.props.onLayout === "function") n.props.onLayout({ bounds: { size: { height } } });
    }
  }

  press(label) {
    const n = this.pressable(label);
    if (!n) throw new Error(`nothing pressable reads ${JSON.stringify(label)}`);
    n.props.onPress();
  }

  /** Type into the Input with this placeholder: its onChange gets `{ value }`, as nsh sends it. */
  type(placeholder, value) {
    const n = this.input(placeholder);
    if (!n) throw new Error(`no input with placeholder ${JSON.stringify(placeholder)}`);
    n.props.onChange({ value });
  }

  async until(check, what, ms = 10000) {
    const end = Date.now() + ms;
    for (;;) {
      if (check()) return;
      if (Date.now() > end) throw new Error(`timed out waiting for ${what}`);
      await new Promise((r) => setTimeout(r, 25));
    }
  }
}
