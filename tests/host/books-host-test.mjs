// Headless second-instance test of the Books plugin against a real Buwiz Bot broker (docs/06 section 13.2 steps 6-11).
// Run it through tools/bot/host-test.sh, which starts the throwaway broker. Direct use:
//   node books-host-test.mjs <buwiz worktree> <run dir> <books binary> <port>
// It reuses the Bot's own test helpers (openApp, envelope) from the worktree. Step 11 runs the Books window's served
// source on nsh-standin.mjs, types into a TextBox and changes pickers, with buwiz.call answered by this broker. It
// prints step results only: no host token, ticket values, keys or signatures.
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

const [WT, RUN, BIN, PORT] = process.argv.slice(2);
const { openApp, envelope } = await import(pathToFileURL(`${WT}/server/test/v1-helpers.mjs`).href);
const BASE = `http://127.0.0.1:${PORT}`;
const token = readFileSync(`${RUN}/data/host-token`, "utf8");
const signed = JSON.parse(readFileSync(`${RUN}/signed.json`, "utf8"));

async function post(path, body) {
  const r = await fetch(BASE + path, { method: "POST", headers: { "content-type": "application/json", "x-buwiz-host": token }, body: JSON.stringify(body) });
  const text = await r.text();
  let json;
  try { json = JSON.parse(text); } catch { json = { raw: text.slice(0, 200) }; }
  return { status: r.status, json };
}

const results = [];
const check = (name, ok, detail) => { results.push({ name, ok: !!ok }); console.log(`${ok ? "PASS" : "FAIL"}  ${name}${detail ? `  ${JSON.stringify(detail)}` : ""}`); };
const refusal = (r) => ({ status: r.status, code: r.json?.code ?? r.json?.error?.code });

const spec = { command: BIN, args: ["mcp"], env: { BOOKS_V1_MANIFEST: `${RUN}/signed.json`, BOOKS_DATA_DIR: `${RUN}/books-data`, BOOKS_TODAY: "2026-10-29" } };

// 0. a request without the host token is refused
const noAuth = await fetch(`${BASE}/v1/host/plugins/inspect`, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(spec) });
check("host route without X-Buwiz-Host is 401", noAuth.status === 401, { status: noAuth.status });

// 1. inspect: a temporary spawn, verified against the per-run trust file
const insp = await post("/v1/host/plugins/inspect", spec);
check("inspect verifies the signed manifest", insp.json.ok === true && /^[0-9a-f]{64}$/.test(insp.json.manifest_sha256),
  { status: insp.status, id: insp.json.manifest?.id, version: insp.json.manifest?.version, key_id: insp.json.manifest?.publisher?.key_id, warnings: insp.json.warnings, code: insp.json.code });
console.log("      review:", JSON.stringify(insp.json.review));

// 2. install what was reviewed
const inst = await post("/v1/host/plugins/install", { ...spec, manifest_sha256: insp.json.manifest_sha256 });
check("install the reviewed manifest", inst.json.ok === true, { status: inst.status, id: inst.json.id, code: inst.json.code, message: inst.json.message });

// 3. install with a different hash is refused
const wrong = await post("/v1/host/plugins/install", { ...spec, manifest_sha256: "0".repeat(64) });
check("install with a hash other than the reviewed one is refused", wrong.json.ok !== true, refusal(wrong));

// 4. resolve the /books command
const res = await post("/v1/host/resolve", { plugin: "app.buwiz.books", kind: "command", name: "books", value: "" });
const r = res.json;
check("/books resolves to open the Books window with a ticket and a neutral title", r.result === "open" && r.app === "books" && r.title === "Books" && /^tkt_/.test(String(r.ticket ?? r.props?.ticket ?? "")),
  { status: res.status, result: r.result, app: r.app, title: r.title, has_ticket: /^tkt_/.test(String(r.ticket ?? r.props?.ticket ?? "")) });

// 5. open the home app the way the Bot does (session key, open_id, permissions digest)
const sessions = { open: async (req) => { const o = await post("/v1/host/sessions/open", req); if (o.json.ok === false) throw Object.assign(new Error(o.json.message), { code: o.json.code }); return o.json; } };
const open = await openApp(sessions, signed, "home");
check("home session opens with a broker-verified certificate", !!open.res.cert && !!open.res.cert_sig, { permissions: open.res.permissions });

// 6. a signed call of books_status from home
const env1 = envelope(open, "books_status", {});
const call = await post("/v1/host/call", env1);
check("signed books_status call from home is answered", call.status === 200 && call.json.ok !== false,
  { status: call.status, result: call.json.result?.structuredContent ?? call.json.result ?? call.json });

// 7. the same envelope again is a replay
const replay = await post("/v1/host/call", env1);
check("the same envelope sent again is refused or replayed, not re-run", replay.status !== 200 || replay.json.replayed === true || JSON.stringify(replay.json) === JSON.stringify(call.json), refusal(replay));

// 8. a tool the home app's manifest does not grant is refused
const homeTools = signed.apps.find((a) => a.id === "home").permissions.tools;
const notGranted = homeTools.includes("books_resolve") ? null : await post("/v1/host/call", envelope(open, "books_resolve", {}));
check("a tool outside home's permissions.tools is refused", notGranted === null || notGranted.json.ok === false, notGranted ? refusal(notGranted) : "books_resolve is granted to home");

// 9. an envelope signed by another key is refused
const { generate } = await import(pathToFileURL(`${WT}/server/src/v1/ed.mjs`).href);
const forged = await post("/v1/host/call", envelope(open, "books_status", {}, {}, generate().seed));
check("an envelope signed by another key is refused", forged.json.ok === false, refusal(forged));

// 10. M1 through the broker: the Books window makes a book, drafts, posts, voids and lists
const win = await openApp(sessions, signed, "books");
check("the Books window session opens", !!win.res.cert && !!win.res.cert_sig);
let n = 0;
const call2 = async (tool, args) => {
  const c = await post("/v1/host/call", envelope(win, tool, args));
  return c.json.result?.structuredContent ?? c.json;
};
const rid = () => `host-${Date.now()}-${(n += 1)}`;
const made = await call2("books_app_book_create", { client_request_id: rid(), label: "Test Book" });
check("books_app_book_create makes a cash book", made.basis === "cash" && /^tkt_/.test(String(made.book)), { basis: made.basis, code: made.code });
const draft = await call2("books_entry_draft", { client_request_id: rid(), form: "money_out", date: "2026-10-18", amount: "1350.00", account: "11000", category: "67200", payee: "Synthetic Vendor", memo: "MEMO-SYNTH-host" });
check("books_entry_draft saves a draft", draft.status === "draft", { status: draft.status, code: draft.code });
const posted = await call2("books_app_post_entry", { client_request_id: rid(), entry: draft.entry });
check("books_app_post_entry posts it", posted.status === "posted", { status: posted.status, code: posted.code });
const voided = await call2("books_app_void_entry", { client_request_id: rid(), entry: draft.entry });
check("books_app_void_entry voids it and posts a reversal", voided.status === "voided" && /^tkt_/.test(String(voided.reversal)), { status: voided.status, code: voided.code });
const led = await openApp(sessions, signed, "ledger");
check("the ledger app's session opens", !!led.res.cert && !!led.res.cert_sig);
const call3 = async (tool, args) => {
  const c = await post("/v1/host/call", envelope(led, tool, args));
  return c.json.result?.structuredContent ?? c.json;
};
const list = await call3("books_ledger_list", { month: "2026-10" });
check("the Ledger lists the voided entry and its reversal", list.counts?.all === 2 && list.rows?.some((x) => x.status_label === "Voided") && list.rows?.some((x) => x.status_label === "Reversal"), { counts: list.counts });
const fromHome = await post("/v1/host/call", envelope(open, "books_app_post_entry", { client_request_id: rid(), entry: draft.entry }));
check("home cannot reach a tool outside its permissions", fromHome.json.ok === false || fromHome.json.result?.isError === true, refusal(fromHome));

// 11. the Books window's controls, driven through the broker as a user would: the app source the broker serves runs on
// the nsh stand-in (signals' set needs its signal as `this`, as in nsh-js), and its buwiz.call goes to this broker
const { createRequire } = await import("node:module");
const esbuild = createRequire(`${WT}/server/package.json`)("esbuild");
const { mountApp } = await import(pathToFileURL(`${import.meta.dirname}/nsh-standin.mjs`).href);
const ui = await openApp(sessions, signed, "books");
const bridge = {
  props: {},
  call: async (tool, args = {}) => {
    const c = await post("/v1/host/call", envelope(ui, tool, args));
    if (c.json.ok === false) throw Object.assign(new Error(c.json.message || c.json.code), { code: c.json.code });
    const res = c.json.result;
    if (res?.isError) throw Object.assign(new Error(res.content?.[0]?.text || "tool_error"), { code: "tool_error" });
    return res?.structuredContent ?? (res?.content || []).map((x) => x.text).join("\n");
  },
};
const src = await post("/v1/host/apps/source", { plugin: "app.buwiz.books", app: "books" });
check("the broker serves the Books window's source", src.json.ok === true && typeof src.json.source === "string", { status: src.status, code: src.json.code });
const step = async (name, body) => {
  try {
    const detail = await body();
    check(name, true, detail);
  } catch (e) {
    check(name, false, { error: `${e?.name || "Error"}: ${e?.message || e}` });
  }
};
let screen = null;
await step("the Books window mounts and shows its rail", async () => {
  screen = mountApp(src.json.source, { esbuild, bridge });
  await screen.until(() => screen.pressable("People"), "the rail's People row");
});
const NAME_BOX = "Name, never a TIN";
const TYPED = "Synthetic Vendor W4";
await step("typing into a TextBox (People › + New › Name) keeps the text, and Add saves it", async () => {
  screen.press("People");
  await screen.until(() => screen.pressable("+ New"), "People's + New");
  screen.press("+ New");
  await screen.until(() => screen.input(NAME_BOX), "the Name box");
  screen.type(NAME_BOX, TYPED);
  const shown = screen.input(NAME_BOX)?.props.value;
  if (shown !== TYPED) throw new Error(`the Name box shows ${JSON.stringify(shown)} after typing`);
  screen.press("Add");
  await screen.until(() => screen.hasText(TYPED), "the new person in the list");
  return { typed: true, saved: true };
});
const RAISED = "bg-[#f0f0ee]";
const chosen = (label) => String(screen.pressable(label)?.props.class || "").includes(RAISED);
await step("changing a picker (Certificates › + Add a 2307 › Form, Kind, Status) moves the choice", async () => {
  screen.press("Certificates");
  await screen.until(() => screen.pressable("+ Add a 2307"), "Certificates' + Add a 2307");
  screen.press("+ Add a 2307");
  await screen.until(() => screen.pressable("2306 (final)"), "the Form picker");
  const picks = [["2306 (final)", "2307"], ["Income tax", "Percentage tax"], ["Pending", "Received"]];
  for (const [to, from] of picks) {
    if (!chosen(from) || chosen(to)) throw new Error(`${from} is not the starting choice`);
    screen.press(to);
    if (!chosen(to) || chosen(from)) throw new Error(`pressing ${to} left ${from} chosen`);
  }
  return { picked: picks.map(([to]) => to) };
});
screen?.unmount();

const failed = results.filter((x) => !x.ok).length;
console.log(`\n${results.length - failed} pass, ${failed} fail`);
process.exit(failed ? 1 : 0);
