// Slice B, live: the demo book's Q3 2551Q goes from Books into a real eBIRForms draft on a scratch eBIRForms data
// folder, and a later Books change marks the hand-off out of date until the owner refreshes it.
// Run it through tools/bir/slice-b-live.sh. Direct use:
//   node slice-b-live.mjs <bot worktree> <run dir> <books binary> <ebirforms binary>
// The run dir must hold an `ebirforms` data folder seeded by eBIRForms' host-seed, and SLICE_B_TIN_ROOT name the
// synthetic nine-digit root it was seeded with, so the run can check it never leaks.
//
// Both plugins run under the Bot's own v1 broker modules in this process (no port). One throwaway key,
// books-test-01, signs both manifests, and a trust file of this run only trusts it. eBIRForms keeps its data in the
// run dir, never reads a keychain, and its Buwiz account URL is a closed loopback port. The script plays the owner
// (Books' app, through signed calls) and the assistant (the agent surface). The assistant passes Books' `form_open`
// and `form_fill` to eBIRForms as given, reads the boxes and their sources back, and saves a draft; nothing is
// queued or sent. It prints step results only: no TIN, ticket, key or signature.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { join, resolve, sep } from "node:path";
import { DatabaseSync } from "node:sqlite";
import { pathToFileURL } from "node:url";

const [WT, RUN, BOOKS_BIN, EBIR_BIN] = process.argv.slice(2).map((p) => p && resolve(p));
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
const EBIR = "app.buwiz.ebirforms";
const KEY_ID = "books-test-01";
const DEMO = join(import.meta.dirname, "..", "..", "fixtures", "demo", "buwiz-trading-2026.json");
const TODAY = "2026-10-29";

// the synthetic taxpayer's root, as nine digits and as a TIN is written; never printed
const ROOT = process.env.SLICE_B_TIN_ROOT || "";
assert.match(ROOT, /^\d{9}$/, "SLICE_B_TIN_ROOT names the seeded synthetic root");
const DASHED = `${ROOT.slice(0, 3)}-${ROOT.slice(3, 6)}-${ROOT.slice(6)}`;
const tinIn = (s) => s.includes(ROOT) || s.includes(DASHED);
const scrub = (s) => s.replaceAll(ROOT, "<tin>").replaceAll(DASHED, "<tin>");

const results = [];
const check = (name, ok, detail) => {
  results.push({ name, ok: !!ok });
  console.log(scrub(`${ok ? "PASS" : "FAIL"}  ${name}${detail === undefined ? "" : `  ${JSON.stringify(detail)}`}`));
};

// ── 0. the scratch eBIRForms data folder ───────────────────────────────
const within = (p) => resolve(p).startsWith(RUN + sep);
const EBIR_DATA = join(RUN, "ebirforms");
assert.ok(within(EBIR_DATA) && existsSync(join(EBIR_DATA, "host-test-scratch")), "the eBIRForms data folder is this run's scratch folder");
const ebirEnv = { EBIRFORMS_DATA_DIR: EBIR_DATA, BUWIZ_NO_KEYCHAIN: "1", EBIRFORMS_ACCOUNT_URL: "http://127.0.0.1:1" };
check("scratch eBIRForms data folder in the run dir", within(ebirEnv.EBIRFORMS_DATA_DIR));

// ── 1. one throwaway key and a trust file of this run ──────────────────
const publisher = generate();
const trustFile = join(RUN, "trust.json");
writeFileSync(trustFile, JSON.stringify({
  v: 1,
  about: "Per-run trust store for Books' live Slice B (a throwaway test key; never gc-2026-01).",
  publishers: { [KEY_ID]: { publisher: "app.buwiz", alg: "ed25519", public_key: publisher.pub, first_party: true } },
  approved_plugins: [{ id: BOOKS, publisher_key_id: KEY_ID }, { id: EBIR, publisher_key_id: KEY_ID }],
}));
const sources = (bin, manifest) => Object.fromEntries(manifest.apps.filter((a) => !a.native).map((a) => [a.id, execFileSync(bin, ["app-source", a.id])]));
const signed = (unsigned, appSources, file) => {
  writeFileSync(file, JSON.stringify(signManifest(unsigned, publisher.seed, appSources), null, 2));
  return file;
};
const booksUnsigned = JSON.parse(execFileSync(BOOKS_BIN, ["manifest", "--key-id", KEY_ID]).toString());
const booksEnv = { BOOKS_DATA_DIR: join(RUN, "books-data"), BOOKS_TODAY: TODAY };
const booksSpec = { command: BOOKS_BIN, args: ["mcp"], env: { ...booksEnv, BOOKS_V1_MANIFEST: signed(booksUnsigned, sources(BOOKS_BIN, booksUnsigned), join(RUN, "books.json")) } };
const ebirUnsigned = JSON.parse(execFileSync(EBIR_BIN, ["manifest", "--key-id", KEY_ID], { env: { PATH: process.env.PATH, ...ebirEnv } }).toString());
const ebirSpec = { command: EBIR_BIN, args: ["mcp"], env: { ...ebirEnv, EBIRFORMS_MANIFEST: signed(ebirUnsigned, sources(EBIR_BIN, ebirUnsigned), join(RUN, "ebirforms.json")) } };
check("both manifests signed with the run's test key", booksUnsigned.publisher.key_id === KEY_ID && ebirUnsigned.publisher.key_id === KEY_ID);

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
  for (const [id, spec] of [[BOOKS, booksSpec], [EBIR, ebirSpec]]) {
    const inspected = await plugins.inspect(spec);
    const installed = await plugins.install({ ...spec, manifest_sha256: inspected.manifest_sha256 });
    check(`${id} installed and running`, installed.ok && plugins.runtime(id).state === "running");
  }

  // a fake Bot: an app's session and its signed calls; the owner works through these
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
  // every result, by the plugin that sent it
  const seen = [];
  const keep = (plugin, v) => (seen.push({ plugin, text: typeof v === "string" ? v : JSON.stringify(v) }), v);
  const appCall = async (o, tool, args = {}) => {
    const env = { v: 1, typ: "buwiz-call", plugin: o.plugin, app: o.app, open_id: o.open_id, tool, args, ts: Date.now(), nonce: random128() };
    return keep(o.plugin, resultData((await sessions.call({ envelope: env, sig: sign(o.ks.seed, signedBytes(env)) })).result));
  };
  const appRefused = (o, tool, args) => appCall(o, tool, args).then(() => false, () => true);
  // the assistant: the Bot's own agent path (origin agent, app-only tools refused)
  const agent = async (tool, input) => {
    const r = await v1AgentCall(plugins, null, tool, input);
    keep(tool.startsWith("books_") ? BOOKS : EBIR, r.content);
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
  check("Books: the old hand-off cannot be confirmed", await appRefused(books, "books_app_handoff_confirm", { client_request_id: req(), handoff: made.ticket }));

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
  const logs = readdirSync(join(RUN, "logs")).map((f) => readFileSync(join(RUN, "logs", f), "utf8"));
  const secrets = readdirSync(join(RUN, "keys")).map((f) => readFileSync(join(RUN, "keys", f), "utf8").trim());
  const dbKey = existsSync(join(EBIR_DATA, "ebirforms.key")) ? readFileSync(join(EBIR_DATA, "ebirforms.key"), "utf8").trim() : "";
  check("no TIN, plugin key or eBIRForms database key in the broker's logs", logs.length > 0 && !logs.some((t) => tinIn(t) || [...secrets, dbKey].some((k) => k && t.includes(k))));

  status = results.every((r) => r.ok) ? 0 : 1;
} catch (e) {
  check("run", false, `${e?.code || ""} ${scrub(String(e?.message || e)).slice(0, 400)}`);
} finally {
  await plugins.stopAll().catch(() => {});
}
const failed = results.filter((r) => !r.ok).length;
console.log(`\n${results.length - failed} pass, ${failed} fail`);
process.exit(failed ? 1 : status);
