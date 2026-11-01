// M7 Slice B, live (docs/01 9.2, AGENTS.md 15): the demo book's Q3 2551Q goes from Books into a real BIR draft on a
// scratch BIR profile, and a later Books change marks the hand-off out of date until the owner refreshes it.
// Run it through tools/bir/slice-b-live.sh. Direct use:
//   node slice-b-live.mjs <buwiz worktree> <run dir> <books binary> <bir binary> <bir source tree>
//
// Both plugins run under the Bot's own v1 broker modules in this process (no port), the way BIR's broker harness runs
// BIR (crates/bir-shell/v1-tests/broker.test.mjs): one throwaway key, books-test-01, signs both manifests and a trust
// file of this run only trusts it. BIR gets HOME, BIR_DATABASE_PATH and a throwaway BIR_DB_KEY inside the run dir
// (so it never reaches the owner's database or the Keychain), EBIR_TEST_ENV, the debug build's auth bypass and
// dry-run filing. The script plays the owner (Books' app and BIR's sign-in app, through signed calls) and the
// assistant (the agent surface). The assistant uses BIR's form tools only to open, read, fill (source document) and
// save a draft. BIR's own made-up test taxpayer is read from BIR's harness at run time, so no TIN is in this repo.
// It prints step results only: no TIN, ticket, key or signature.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { randomBytes } from "node:crypto";
import { existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, resolve, sep } from "node:path";
import { DatabaseSync } from "node:sqlite";
import { pathToFileURL } from "node:url";

const [WT, RUN, BOOKS_BIN, BIR_BIN, BIR_SRC] = process.argv.slice(2).map((p) => p && resolve(p));
const load = (path) => import(pathToFileURL(join(WT, "server", "src", "v1", path)).href);
const { generate, random128, sign } = await load("ed.mjs");
const { signedBytes } = await load("jcs.mjs");
const { keystore } = await load("keystore.mjs");
const { permsDigest } = await load("manifest.mjs");
const { createPlugins, resultData } = await load("plugins.mjs");
const { createSessions } = await load("sessions.mjs");
const { signManifest } = await load("sign-manifest.mjs");
const { loadTrust } = await load("trust.mjs");
const { v1AgentCall } = await load("agent.mjs");

const BOOKS = "app.buwiz.books";
const BIR = "app.buwiz.ebirforms";
const KEY_ID = "books-test-01";
const DEMO = join(import.meta.dirname, "..", "..", "fixtures", "demo", "buwiz-trading-2026.json");
const TODAY = "2026-10-29";

// BIR's own made-up test taxpayer, read from its harness at run time (no TIN in this repo)
const harness = readFileSync(join(BIR_SRC, "crates", "bir-shell", "v1-tests", "broker.test.mjs"), "utf8");
const ROOT = harness.match(/const ROOT = "(\d+)"/)?.[1];
const NAME = harness.match(/The test taxpayer is made up \(TIN [^,]+, "([^"]+)"\)/)?.[1];
assert.ok(ROOT && NAME, "BIR's harness names its made-up test taxpayer");
const scrub = (s) => s.replaceAll(ROOT, "<tin>");

const results = [];
const check = (name, ok, detail) => {
  results.push({ name, ok: !!ok });
  console.log(scrub(`${ok ? "PASS" : "FAIL"}  ${name}${detail === undefined ? "" : `  ${JSON.stringify(detail)}`}`));
};

// ── 0. the scratch profile, asserted before BIR starts ─────────────────
const within = (p) => resolve(p).startsWith(RUN + sep);
const BIR_DB = join(RUN, "bir", "bir.db");
const HOME = join(RUN, "home");
assert.ok(within(BIR_DB) && within(HOME), "the BIR database and HOME are inside the run dir");
assert.ok(!existsSync(BIR_DB), "the scratch BIR database is new");
mkdirSync(join(RUN, "bir"), { recursive: true, mode: 0o700 });
mkdirSync(HOME, { recursive: true, mode: 0o700 });
const birEnv = {
  HOME,
  BIR_DATABASE_PATH: BIR_DB,
  BIR_DB_KEY: randomBytes(32).toString("hex"),
  BIR_MINI_APPS_DIR: join(BIR_SRC, "crates", "bir-shell", "mini-apps"),
  EBIR_TEST_ENV: "1",
  BIR_AUTH_BYPASS: "1",
  BIR_SFTP_DRY_RUN: "1",
};
check("scratch BIR profile: database, HOME and key in the run dir", within(birEnv.BIR_DATABASE_PATH) && within(birEnv.HOME));

// ── 1. one throwaway key and a trust file of this run ──────────────────
const publisher = generate();
const trustFile = join(RUN, "trust.json");
writeFileSync(trustFile, JSON.stringify({
  v: 1,
  about: "Per-run trust store for Books' live Slice B (a throwaway test key; never gc-2026-01).",
  publishers: { [KEY_ID]: { publisher: "app.buwiz", alg: "ed25519", public_key: publisher.pub } },
  approved_plugins: [{ id: BOOKS, publisher_key_id: KEY_ID }, { id: BIR, publisher_key_id: KEY_ID }],
}));
const signed = (unsigned, sources, file) => {
  // BIR's manifest names its release key; this run signs it with the test key instead
  unsigned.publisher.key_id = KEY_ID;
  writeFileSync(file, JSON.stringify(signManifest(unsigned, publisher.seed, sources), null, 2));
  return file;
};
const booksUnsigned = JSON.parse(execFileSync(BOOKS_BIN, ["manifest", "--key-id", KEY_ID]).toString());
const booksSources = Object.fromEntries(booksUnsigned.apps.map((a) => [a.id, execFileSync(BOOKS_BIN, ["app-source", a.id])]));
const booksEnv = { BOOKS_DATA_DIR: join(RUN, "books-data"), BOOKS_TODAY: TODAY };
const booksSpec = { command: BOOKS_BIN, args: ["mcp"], env: { ...booksEnv, BOOKS_V1_MANIFEST: signed(booksUnsigned, booksSources, join(RUN, "books.json")) } };
const birUnsigned = JSON.parse(execFileSync(BIR_BIN, ["print-v1-manifest"], { env: { PATH: process.env.PATH, ...birEnv } }).toString());
const birSpec = { command: BIR_BIN, args: ["mcp"], env: { ...birEnv, BIR_V1_MANIFEST: signed(birUnsigned, {}, join(RUN, "bir.json")) } };
check("both manifests signed with the run's test key", booksUnsigned.publisher.key_id === KEY_ID && birUnsigned.publisher.key_id === KEY_ID);

// the synthetic demo book, before Books starts
execFileSync(BOOKS_BIN, ["seed", DEMO], { env: { PATH: process.env.PATH, ...booksEnv } });

// ── 2. the broker, both plugins installed ──────────────────────────────
const plugins = createPlugins({
  db: new DatabaseSync(":memory:"),
  keys: keystore(`file:${join(RUN, "keys")}`),
  logDir: join(RUN, "logs"),
  trust: () => loadTrust(trustFile),
});
const sessions = createSessions(plugins);
let status = 1;
try {
  for (const [id, spec] of [[BOOKS, booksSpec], [BIR, birSpec]]) {
    const inspected = await plugins.inspect(spec);
    const installed = await plugins.install({ ...spec, manifest_sha256: inspected.manifest_sha256 });
    check(`${id} installed and running`, installed.ok && plugins.runtime(id).state === "running");
  }

  // a fake the Bot: an app's session and its signed calls; the owner works through these
  async function open(plugin, app) {
    const manifest = plugins.manifestOf(plugins.rowOf(plugin));
    const ks = generate();
    const req = {
      plugin, app, open_id: random128(), ks_pub: ks.pub,
      perms_sha256: permsDigest(manifest.apps.find((a) => a.id === app).permissions), not_after: Date.now() + 3600_000,
    };
    await sessions.open(req);
    return { ...req, ks };
  }
  // every result, by the plugin that sent it: Books' must never carry a TIN
  const seen = [];
  const keep = (plugin, v) => (seen.push({ plugin, text: typeof v === "string" ? v : JSON.stringify(v) }), v);
  const appCall = async (o, tool, args = {}) => {
    const env = { v: 1, typ: "buwiz-call", plugin: o.plugin, app: o.app, open_id: o.open_id, tool, args, ts: Date.now(), nonce: random128() };
    // BIR's own apps may see a TIN (the owner typed it); only Books' results and BIR's answers to the assistant are held to none
    return keep(o.plugin === BIR ? "bir-app" : o.plugin, resultData((await sessions.call({ envelope: env, sig: sign(o.ks.seed, signedBytes(env)) })).result));
  };
  const appRefused = (o, tool, args) => appCall(o, tool, args).then(() => false, () => true);
  // the assistant: the Bot's own agent path (origin agent, app-only tools refused)
  const agent = async (tool, input) => {
    const r = await v1AgentCall(plugins, null, tool, input);
    keep(tool.startsWith("books_") ? BOOKS : BIR, r.content);
    let data = null;
    try { data = JSON.parse(r.content); } catch { /* plain text */ }
    return { error: r.is_error, text: r.content, data };
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
  check("Books: the owner hands Q3 2551Q to a BIR draft", made.title === "BIR hand-off · 2551Q Q3 2026" && made.boxes === 2, { title: made.title, boxes: made.boxes });

  // ── 4. the assistant reads the hand-off ────────────────────────────────
  const got = await agent("books_handoff_get", { handoff: made.ticket });
  const fill = got.data?.fill_args || {};
  check("assistant: books_handoff_get gives fill_args, source document", !got.error && got.data.source === "document" && got.data.form === "2551Q", { year: got.data?.year, period: got.data?.bir_period, fill_args: fill });

  // ── 5. the owner's BIR: the made-up test taxpayer, by name as a ticket ──
  const register = await open(BIR, "register");
  await appCall(register, "bir_app_create_taxpayer", { args: [{ tin_root: ROOT, name: NAME, taxpayer_type: "Individual", head_office: { rdo_code: "040" } }] });
  const found = await sessions.resolve({ plugin: BIR, kind: "scope", name: "taxprofile", value: NAME.split(" ")[0], complete: true });
  const taxpayer = found.choices?.[0]?.value;
  check("BIR: the test taxpayer resolves to a ticket by name", typeof taxpayer === "string" && taxpayer.startsWith("tkt_"));

  // ── 6. the assistant fills BIR's draft once, source document, and saves it ──
  // BIR shows an amount with thousands separators ("290,000.00"); the read-back is to the centavo without them
  const plain = (v) => String(v ?? "").replaceAll(",", "");
  const valuesOf = (boxes, values) => Object.fromEntries(boxes.filter((b) => b.key in values).map((b) => [b.key, plain(b.value)]));
  const sameAs = (read, values) => Object.keys(values).every((k) => read[k] === values[k]);
  const fillDraft = async (label, values, previous) => {
    const opened = await agent("bir_form_open", { code: got.data.form, year: got.data.year, period: got.data.bir_period, tin: taxpayer });
    check(`${label}: bir_form_open 2551Q Q3 2026`, !opened.error, opened.error ? opened.text.slice(0, 200) : undefined);
    const boxes = (await agent("bir_form_fields", {})).data?.boxes || [];
    if (previous) {
      // the draft saved earlier opens with its values: it was kept in the scratch database
      const kept = valuesOf(boxes, previous);
      check(`${label}: the saved draft reopens with the earlier values`, sameAs(kept, previous), { kept });
    }
    const keys = boxes.map((b) => b.key);
    check(`${label}: every fill_args key is a box BIR lets the assistant fill`, Object.keys(values).every((k) => keys.includes(k)), { missing: Object.keys(values).filter((k) => !keys.includes(k)) });
    const filled = await agent("bir_form_fill", { fields: values, source: "document" });
    const d = filled.data || {};
    check(`${label}: bir_form_fill takes every box`, !filled.error && !d.refused && !(d.not_shown || []).length && !(d.kept_user_boxes || []).length,
      filled.error || d.refused || (d.not_shown || []).length ? filled.text.slice(0, 300) : undefined);
    const after = (await agent("bir_form_fields", {})).data?.boxes || [];
    const read = valuesOf(after, values);
    const sources = [...new Set(after.filter((b) => b.key in values).map((b) => b.source))];
    check(`${label}: read back, each box equals its fill_args amount, source document`, sameAs(read, values) && sources.length === 1 && sources[0] === "document", { read, sources });
    const saved = await agent("bir_form_save_draft", {});
    check(`${label}: bir_form_save_draft`, !saved.error, saved.error ? saved.text.slice(0, 200) : undefined);
    return read;
  };
  await fillDraft("BIR draft", fill);
  const drafts = await agent("bir_drafts_list", { tin: taxpayer });
  check("BIR: the 2551Q draft is listed", !drafts.error && /2551Q/.test(drafts.text));

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
  check("Books: the old hand-off cannot be confirmed", await appRefused(books, "books_app_handoff_confirm", { client_request_id: req(), handoff: made.ticket }));

  // ── 9. refresh: a new hand-off, filled into the same BIR draft ─────────
  const fresh = await appCall(books, "books_app_handoff_create", { client_request_id: req(), ...Q3 });
  const old = await agent("books_handoff_get", { handoff: made.ticket });
  const again = await agent("books_handoff_get", { handoff: fresh.ticket });
  const refill = again.data?.fill_args || {};
  check("Books: the refresh supersedes the old hand-off, with the late sale in", old.error && !again.error && refill.txtATCAmt1 !== fill.txtATCAmt1, { fill_args: refill });
  await fillDraft("BIR draft, refreshed", refill, fill);
  await appCall(books, "books_app_handoff_confirm", { client_request_id: req(), handoff: fresh.ticket });
  view = await appCall(books, "books_form_proposal", Q3);
  check("Books: the refreshed hand-off is In BIR draft", view.handoff?.status === "confirmed", { detail: view.handoff?.detail });

  // ── 10. privacy: no TIN in anything Books returned, nor in any log ─────
  const tinIn = (s) => s.includes(ROOT);
  check("no TIN in any Books result, to the owner's app or the assistant", !seen.some((r) => r.plugin === BOOKS && tinIn(r.text)));
  check("no TIN in BIR's answers to the assistant (BIR names taxpayers by ticket)", !seen.some((r) => r.plugin === BIR && tinIn(r.text)));
  const logs = readdirSync(join(RUN, "logs")).map((f) => readFileSync(join(RUN, "logs", f), "utf8"));
  const secrets = [birEnv.BIR_DB_KEY, ...readdirSync(join(RUN, "keys")).map((f) => readFileSync(join(RUN, "keys", f), "utf8").trim())];
  check("no TIN, plugin key or BIR database key in the broker's logs", logs.length > 0 && !logs.some((t) => tinIn(t) || secrets.some((k) => k && t.includes(k))));
  check("the scratch BIR database holds the draft", existsSync(BIR_DB));

  status = results.every((r) => r.ok) ? 0 : 1;
} catch (e) {
  check("run", false, `${e?.code || ""} ${String(e?.message || e).slice(0, 400)}`);
} finally {
  await plugins.stopAll().catch(() => {});
}
const failed = results.filter((r) => !r.ok).length;
console.log(`\n${results.length - failed} pass, ${failed} fail`);
process.exit(failed ? 1 : status);
