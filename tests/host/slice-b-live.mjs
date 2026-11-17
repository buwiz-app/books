// Slice B, live: the demo book's Q3 2551Q goes from Books into a real eBIRForms draft on a scratch eBIRForms data
// folder, and a later Books change marks the hand-off out of date until the owner refreshes it.
// Run it through tools/bir/slice-b-live.sh, which starts the broker and makes the manifests and the key. Direct use:
//   node slice-b-live.mjs <run dir> <books binary> <ebirforms binary> <port> <model port> <broker pid> <sdk conformance dir>
// The run dir must hold an `ebirforms` data folder seeded by eBIRForms' host-seed, the two signed manifests
// (books.json, ebirforms.json), the broker's `data` and `keys` folders, its broker.log and the test key (test.key).
// SLICE_B_TIN_ROOT names the synthetic nine-digit root it was seeded with, so the run can check it never leaks. The
// plugins' environment (BOOKS_DATA_DIR, BOOKS_TODAY, EBIRFORMS_DATA_DIR, BUWIZ_NO_KEYCHAIN, EBIRFORMS_ACCOUNT_URL) is
// set once, by the shell script, and read here; a missing one stops the run.
//
// Both plugins run under the Rust buwiz-broker, over its loopback HTTP routes. One throwaway key, books-test-01,
// signed both manifests, and a trust file of this run only trusts it. eBIRForms keeps its data in the run dir, never
// reads a keychain, and its Buwiz account URL is a closed loopback port. The script plays the owner (Books' app,
// through signed calls) and the assistant: a chat whose model is a scripted stand-in on loopback, so each assistant
// call is the broker's own agent path (origin agent, app-only tools refused). The assistant passes Books' `form_open`
// and `form_fill` to eBIRForms as given, reads the boxes and their sources back, and saves a draft; nothing is
// queued or sent. It uses Node built-ins and fetch, plus the SDK's ed25519, JCS and permissions-digest modules
// (<sdk conformance dir>/src/v1/). It prints step results only: no TIN, ticket, key or signature.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, resolve, sep } from "node:path";
import { pathToFileURL } from "node:url";
import { hasToolResults, scriptedModel } from "./scripted-model.mjs";

const [RUN, BOOKS_BIN, EBIR_BIN, PORT, MODEL_PORT, BROKER_PID, SDK_CONF] = process.argv.slice(2);
const sdk = (file) => import(pathToFileURL(`${SDK_CONF}/src/v1/${file}`).href);
const { generate, random128, sign } = await sdk("ed.mjs");
const { signedBytes } = await sdk("jcs.mjs");
const { permsDigest } = await sdk("manifest.mjs");

const BOOKS = "app.buwiz.books";
const EBIR = "app.buwiz.ebirforms";
const KEY_ID = "books-test-01";
const DEMO = join(import.meta.dirname, "..", "..", "fixtures", "demo", "buwiz-trading-2026.json");

// the synthetic taxpayer's root, as nine digits and as a TIN is written; never printed
const ROOT = process.env.SLICE_B_TIN_ROOT || "";
assert.match(ROOT, /^\d{9}$/, "SLICE_B_TIN_ROOT names the seeded synthetic root");
const DASHED = `${ROOT.slice(0, 3)}-${ROOT.slice(3, 6)}-${ROOT.slice(6)}`;
const tinIn = (s) => s.includes(ROOT) || s.includes(DASHED);
// everything the script prints goes through this: the synthetic root, and any ticket value
const scrub = (s) => String(s).replaceAll(ROOT, "<tin>").replaceAll(DASHED, "<tin>").replace(/tkt_[A-Za-z0-9_-]+/g, "<ticket>");

const results = [];
const check = (name, ok, detail) => {
  results.push({ name, ok: !!ok });
  console.log(scrub(`${ok ? "PASS" : "FAIL"}  ${name}${detail === undefined ? "" : `  ${JSON.stringify(detail)}`}`));
};

// ── 0. the scratch eBIRForms data folder ───────────────────────────────
const within = (p) => resolve(p).startsWith(resolve(RUN) + sep);
const EBIR_DATA = join(RUN, "ebirforms");
assert.ok(within(EBIR_DATA) && existsSync(join(EBIR_DATA, "host-test-scratch")), "the eBIRForms data folder is this run's scratch folder");
const need = (name) => {
  const v = process.env[name];
  if (!v) throw new Error(`${name} is not set: run this through tools/bir/slice-b-live.sh`);
  return v;
};
const ebirEnv = Object.fromEntries(["EBIRFORMS_DATA_DIR", "BUWIZ_NO_KEYCHAIN", "EBIRFORMS_ACCOUNT_URL"].map((k) => [k, need(k)]));
const booksEnv = Object.fromEntries(["BOOKS_DATA_DIR", "BOOKS_TODAY"].map((k) => [k, need(k)]));
assert.equal(resolve(ebirEnv.EBIRFORMS_DATA_DIR), resolve(EBIR_DATA), "the eBIRForms data folder is the run's");
check("scratch eBIRForms data folder in the run dir", within(ebirEnv.EBIRFORMS_DATA_DIR));

// ── 1. the manifests, signed by the shell script with the run's test key ──
const booksSigned = JSON.parse(readFileSync(join(RUN, "books.json"), "utf8"));
const ebirSigned = JSON.parse(readFileSync(join(RUN, "ebirforms.json"), "utf8"));
const manifests = { [BOOKS]: booksSigned, [EBIR]: ebirSigned };
const booksSpec = { command: BOOKS_BIN, args: ["mcp"], env: { ...booksEnv, BOOKS_V1_MANIFEST: join(RUN, "books.json") } };
const ebirSpec = { command: EBIR_BIN, args: ["mcp"], env: { ...ebirEnv, EBIRFORMS_MANIFEST: join(RUN, "ebirforms.json") } };
check("both manifests signed with the run's test key", booksSigned.publisher?.key_id === KEY_ID && ebirSigned.publisher?.key_id === KEY_ID);

// the synthetic demo book, before Books starts
execFileSync(BOOKS_BIN, ["seed", DEMO], { env: { PATH: process.env.PATH, ...booksEnv } });

// ── 2. the broker, both plugins installed ──────────────────────────────
const BASE = `http://127.0.0.1:${PORT}`;
const alive = () => { try { process.kill(Number(BROKER_PID), 0); return true; } catch { return false; } };
let ready = false;
for (let i = 0; i < 150 && !ready; i++) {
  if (!alive()) {
    const tail = readFileSync(join(RUN, "broker.log"), "utf8").split("\n").filter(Boolean).slice(-5).map((l) => l.slice(0, 200));
    console.error(scrub(`the broker exited before it answered; its log ends:\n${tail.join("\n")}`));
    throw new Error("broker exited before startup");
  }
  if (existsSync(join(RUN, "data", "host-token"))) {
    try { ready = (await fetch(`${BASE}/health`, { signal: AbortSignal.timeout(1000) })).ok; } catch { /* not yet */ }
  }
  if (!ready) await new Promise((r) => setTimeout(r, 200));
}
if (!ready) throw new Error("broker startup timed out");
const token = readFileSync(join(RUN, "data", "host-token"), "utf8").trim();
const request = async (method, path, body) => {
  const response = await fetch(BASE + path, {
    method, signal: AbortSignal.timeout(60_000),
    headers: { "content-type": "application/json", "x-buwiz-host": token },
    ...(body === undefined ? {} : { body: JSON.stringify(body) }),
  });
  const text = await response.text();
  let json;
  try { json = JSON.parse(text); } catch { json = { raw: text.slice(0, 200) }; }
  return { status: response.status, json };
};
const post = (path, body) => request("POST", path, body);

const model = await scriptedModel(MODEL_PORT);
let status = 1;
try {
  for (const [id, spec] of [[BOOKS, booksSpec], [EBIR, ebirSpec]]) {
    const inspected = await post("/v1/host/plugins/inspect", spec);
    const installed = inspected.json.ok === true ? await post("/v1/host/plugins/install", { ...spec, manifest_sha256: inspected.json.manifest_sha256 }) : inspected;
    // the broker starts the plugin at install: wait for it to say so
    let running = false;
    for (let i = 0; i < 50 && !running && installed.json.ok === true; i++) {
      running = ((await request("GET", "/v1/host/plugins")).json.plugins ?? []).some((p) => p.id === id && p.state === "running");
      if (!running) await new Promise((r) => setTimeout(r, 100));
    }
    check(`${id} installed and running`, installed.json.ok === true && running);
  }

  // a fake Bot: an app's session and its signed calls; the owner works through these
  async function open(plugin, app) {
    const ks = generate();
    const manifest = manifests[plugin];
    const req = {
      plugin, app, open_id: random128(), ks_pub: ks.pub,
      perms_sha256: permsDigest(manifest.apps.find((a) => a.id === app).permissions), not_after: Date.now() + 3600_000,
    };
    const opened = await post("/v1/host/sessions/open", req);
    if (opened.json.ok !== true) throw Object.assign(new Error("session open failed"), { code: opened.json.code });
    return { ...req, ks };
  }
  // every result, by the plugin that sent it
  const seen = [];
  const keep = (plugin, v) => (seen.push({ plugin, text: typeof v === "string" ? v : JSON.stringify(v) }), v);
  // a tool result as data: its structured content, else its first text read as JSON
  const dataOf = (r) => {
    if (r?.structuredContent && typeof r.structuredContent === "object") return r.structuredContent;
    const t = (r?.content || []).find((c) => c.type === "text")?.text;
    try { return JSON.parse(t); } catch { return null; }
  };
  // every result is recorded whole (all its content blocks and its structured content) before anything throws, so
  // the privacy scan sees refusals and errors too
  const appCall = async (o, tool, args = {}) => {
    const env = { v: 1, typ: "buwiz-call", plugin: o.plugin, app: o.app, open_id: o.open_id, tool, args, actor: "person", ts: Date.now(), nonce: random128() };
    const c = await post("/v1/host/call", { envelope: env, sig: sign(o.ks.seed, signedBytes(env)) });
    keep(o.plugin, c.json.result ?? c.json);
    if (c.json.ok === false) throw Object.assign(new Error(c.json.message || c.json.code), { code: c.json.code });
    if (c.json.result?.isError) throw Object.assign(new Error(dataOf(c.json.result)?.code || "tool_error"), { code: dataOf(c.json.result)?.code || "tool_error" });
    return dataOf(c.json.result);
  };
  // refused for this reason: the call fails with this code, and not for any other
  const appRefused = (o, tool, args, code) => appCall(o, tool, args).then(() => false, (e) => e?.code === code);
  // the assistant: a chat in the broker, whose scripted model makes this one tool call; the broker's own agent path
  // (origin agent, app-only tools refused) answers it, and the answer is read from the model's next request. One
  // thread for the whole run.
  for (const plugin of [BOOKS, EBIR]) {
    const allowed = await post("/v1/host/tools/set", { bot: "chats", key: `v1:${plugin}`, state: "on" });
    if (allowed.status !== 200 || allowed.json.ok !== true) throw new Error("could not switch the plugin on for chats");
  }
  const made_thread = await post("/thread/new", {});
  const thread = made_thread.json.id;
  if (thread === undefined || thread === null || thread === "") throw new Error(`POST /thread/new gave no thread id (status ${made_thread.status})`);
  let chats = 0;
  const agent = async (tool, input) => {
    const name = `slice-b-${String(++chats).padStart(4, "0")}`;
    model.script(name, [{ type: "tool_use", name: tool, input }]);
    const started = await post("/chat", { prompt: `[[script:${name}]] go`, thread });
    if (!Number.isInteger(started.json.id)) throw new Error(`the chat for ${tool} gave no job id`);
    let job = {};
    for (let i = 0; i < 600 && !job.done; i++) {
      job = (await request("GET", `/status?id=${started.json.id}`)).json;
      if (!job.done) await new Promise((r) => setTimeout(r, 100));
    }
    const rounds = model.requests.filter((r) => r.script === name).map((r) => r.body);
    // the answer to the call this chat made: found by the id the model gave that call, never by type alone
    const [useId] = model.idsFor(name);
    const answer = (rounds.find((b) => hasToolResults(b))?.messages?.at(-1)?.content ?? []).find((b) => b.type === "tool_result" && b.tool_use_id === useId);
    if (job.done !== true || !answer) throw new Error(`the chat for ${tool} did not answer`);
    const text = typeof answer.content === "string" ? answer.content : (answer.content ?? []).map((c) => c.text ?? "").join("\n");
    keep(tool.startsWith("books_") ? BOOKS : EBIR, answer);
    let data = null;
    try { data = JSON.parse(text); } catch { /* plain text */ }
    return { error: answer.is_error === true, text, data };
  };
  let rid = 0;
  const req = () => `slice-b-live-${String(++rid).padStart(4, "0")}`;

  const books = await open(BOOKS, "books");
  const Q3 = { form: "2551Q", period: "2026-Q3" };

  // ── 3. Books: Q3 closed, the September return confirmed, Hand to BIR draft ──
  for (const month of ["2026-07", "2026-08", "2026-09"]) await appCall(books, "books_app_month_close", { client_request_id: req(), month });
  const before = await appCall(books, "books_form_proposal", Q3);
  for (const r of before.returns || []) await appCall(books, "books_app_return_ok", { client_request_id: req(), entry: r.ticket });
  const made = await appCall(books, "books_app_handoff_create", { client_request_id: req(), ...Q3 });
  check("Books: the owner hands Q3 2551Q over", made.title === "BIR hand-off · 2551Q Q3 2026" && made.boxes === 2, { title: made.title, boxes: made.boxes });

  // ── 4. the assistant reads the hand-off ────────────────────────────────
  const got = await agent("books_handoff_get", { handoff: made.ticket });
  const handoff = got.data || {};
  check("assistant: books_handoff_get gives form_open and form_fill", !got.error && handoff.form_open?.code === "2551Q" && handoff.form_fill?.source === "document" && handoff.form_fill?.source_ref?.plugin === BOOKS,
    { form_open: handoff.form_open, fields: handoff.form_fill?.fields });

  // ── 5. the taxpayer, as the assistant finds it: a ticket by name ───────
  const listed = await agent("ebirforms_profiles_list", {});
  const taxpayer = listed.data?.[0]?.tin_root;
  check("eBIRForms: the synthetic taxpayer is listed as a ticket", !listed.error && typeof taxpayer === "string" && taxpayer.startsWith("tkt_"));

  // ── 6. the assistant fills the draft once, as given, and saves it ──────
  // eBIRForms shows amounts with thousands separators (some forms in whole pesos): compare as amounts, by the
  // rule of books-core money::parse_shown (and eBIRForms' shown_amount): groups of three, two places or none
  const amount = (v) => {
    const m = /^(-?)(\d{1,3}(?:,\d{3})+|\d+)(?:\.(\d{2}))?$/.exec(String(v ?? "").trim());
    return m ? `${m[1]}${m[2].replaceAll(",", "")}.${m[3] ?? "00"}` : null;
  };
  const fillDraft = async (label, h, previous) => {
    const opened = await agent("ebirforms_form_open", { ...h.form_open, tin: taxpayer });
    check(`${label}: ebirforms_form_open with form_open`, !opened.error, opened.error ? opened.text.slice(0, 200) : undefined);
    if (previous) {
      // the draft saved earlier reopens with its values and their sources
      const kept = (await agent("ebirforms_form_fields", { keys: Object.keys(previous.form_fill.fields) })).data || {};
      const same = Object.entries(previous.form_fill.fields).every(([k, v]) => amount(kept.values?.[k]) === v
        && JSON.stringify(kept.sources?.[k]) === JSON.stringify({ source: "document", source_ref: previous.form_fill.source_ref }));
      check(`${label}: the saved draft reopens with the earlier values and sources`, same);
    }
    const filled = await agent("ebirforms_form_fill", h.form_fill);
    const d = filled.data || {};
    const keys = Object.keys(h.form_fill.fields);
    check(`${label}: ebirforms_form_fill takes every box`, !filled.error && Object.keys(d.rejected || {}).length === 0 && !d.not_kept && keys.every((k) => (d.filled || []).includes(k)),
      filled.error || d.not_kept || Object.keys(d.rejected || {}).length ? filled.text.slice(0, 300) : undefined);
    const after = (await agent("ebirforms_form_fields", { keys })).data || {};
    const read = Object.fromEntries(keys.map((k) => [k, amount(after.values?.[k])]));
    const sourced = keys.every((k) => JSON.stringify(after.sources?.[k]) === JSON.stringify({ source: "document", source_ref: h.form_fill.source_ref }));
    check(`${label}: read back, each box equals its amount, sourced to this hand-off`, keys.every((k) => read[k] === h.form_fill.fields[k]) && sourced, { read });
    const saved = await agent("ebirforms_form_save_draft", {});
    check(`${label}: ebirforms_form_save_draft keeps the sources`, !saved.error && saved.data?.sources_kept === true, saved.error ? saved.text.slice(0, 200) : undefined);
    await agent("ebirforms_form_dismiss", {});
  };
  await fillDraft("eBIRForms draft", handoff);
  const drafts = await agent("ebirforms_drafts_list", { tin: taxpayer });
  check("eBIRForms: the 2551Q draft is listed", !drafts.error && /2551Q/.test(drafts.text));

  // ── 7. the owner's click: BIR shows these values ───────────────────────
  const confirmed = await appCall(books, "books_app_handoff_confirm", { client_request_id: req(), handoff: made.ticket });
  let view = await appCall(books, "books_form_proposal", Q3);
  check("Books: confirmed, In BIR draft", confirmed.fill_args_sha256?.length === 64 && view.handoff?.status === "confirmed" && view.handoff?.label === "In BIR draft", { status: view.handoff?.status, detail: view.handoff?.detail });

  // ── 8. a later change: September reopens for a late sale ───────────────
  await appCall(books, "books_app_month_reopen", { client_request_id: req(), month: "2026-09", reason: "A late sale" });
  view = await appCall(books, "books_form_proposal", Q3);
  const stale = await agent("books_handoff_get", { handoff: made.ticket });
  check("Books: reopening September marks the hand-off out of date", view.handoff?.status === "stale" && stale.error, { detail: view.handoff?.detail });
  await appCall(books, "books_app_post_entry", { client_request_id: req(), form: "money_in", date: "2026-09-20", amount: "5000.00", account: "11010", category: "41000", payee: "Isla Café" });
  await appCall(books, "books_app_month_close", { client_request_id: req(), month: "2026-09" });
  view = await appCall(books, "books_form_proposal", Q3);
  check("Books: the late sale keeps it out of date, and says what changed", view.handoff?.status === "stale", { detail: view.handoff?.detail });
  check("Books: the old hand-off cannot be confirmed", await appRefused(books, "books_app_handoff_confirm", { client_request_id: req(), handoff: made.ticket }, "out_of_date"));

  // ── 9. refresh: a new hand-off, filled into the same draft ─────────────
  const fresh = await appCall(books, "books_app_handoff_create", { client_request_id: req(), ...Q3 });
  const old = await agent("books_handoff_get", { handoff: made.ticket });
  const again = await agent("books_handoff_get", { handoff: fresh.ticket });
  const refreshed = again.data || {};
  check("Books: the refresh supersedes the old hand-off, with the late sale in", old.error && !again.error
    && refreshed.form_fill?.fields?.txtATCAmt1 !== handoff.form_fill.fields.txtATCAmt1
    && refreshed.form_fill?.source_ref?.handoff !== handoff.form_fill.source_ref.handoff, { fields: refreshed.form_fill?.fields });
  await fillDraft("eBIRForms draft, refreshed", refreshed, handoff);
  await appCall(books, "books_app_handoff_confirm", { client_request_id: req(), handoff: fresh.ticket });
  view = await appCall(books, "books_form_proposal", Q3);
  check("Books: the refreshed hand-off is In BIR draft", view.handoff?.status === "confirmed", { detail: view.handoff?.detail });

  // ── 10. privacy: no TIN anywhere it must not be, and no key in a log ──
  check("no TIN in any Books result, to the owner's app or the assistant", !seen.some((r) => r.plugin === BOOKS && tinIn(r.text)));
  check("no eBIRForms ticket in any Books result", !seen.some((r) => r.plugin === BOOKS && r.text.includes(taxpayer)));
  check("no TIN in eBIRForms' answers to the assistant (it names taxpayers by ticket)", !seen.some((r) => r.plugin === EBIR && tinIn(r.text)));
  // every file under the run's folder, as bytes (the broker's data folder and databases, the logs, the plugins'
  // folders): none may hold a TIN or a secret. A secret's own file is not a leak of it. eBIRForms' data folder holds
  // the seeded taxpayer by design, so it is not scanned for the TIN root, only its logs are.
  const readText = (f) => readFileSync(f, "utf8").trim();
  const keyFiles = readdirSync(join(RUN, "keys")).map((f) => join(RUN, "keys", f));
  const secretOf = new Map(); // value → the file that holds it by design
  for (const f of [...keyFiles, join(EBIR_DATA, "ebirforms.key"), join(RUN, "test.key")]) {
    if (existsSync(f)) secretOf.set(readText(f), f);
  }
  for (const v of [...secretOf.keys()]) if (v.length < 16) secretOf.delete(v);
  const allFiles = readdirSync(RUN, { recursive: true }).map((f) => join(RUN, String(f))).filter((f) => { try { return statSync(f).isFile(); } catch { return false; } });
  const ebirLogs = allFiles.filter((f) => f.startsWith(join(EBIR_DATA, "logs") + sep));
  const leaks = [];
  for (const f of allFiles) {
    const bytes = readFileSync(f);
    const inEbirData = f.startsWith(EBIR_DATA + sep) && !ebirLogs.includes(f);
    if (!inEbirData && (bytes.includes(ROOT) || bytes.includes(DASHED))) leaks.push(f.slice(RUN.length + 1));
    for (const [secret, home] of secretOf) if (f !== home && bytes.includes(secret)) leaks.push(f.slice(RUN.length + 1));
  }
  check("no TIN, plugin key, eBIRForms database key or publisher test key in any file of the run (the broker's data folder included)", ebirLogs.length > 0 && allFiles.length > 0 && leaks.length === 0, leaks.length ? { files: leaks } : { scanned: allFiles.length });

  status = results.every((r) => r.ok) ? 0 : 1;
} catch (e) {
  check("run", false, `${e?.code || ""} ${scrub(String(e?.message || e)).slice(0, 400)}`);
} finally {
  await model.close().catch(() => {});
}
const failed = results.filter((r) => !r.ok).length;
console.log(`\n${results.length - failed} pass, ${failed} fail`);
process.exit(failed ? 1 : status);
