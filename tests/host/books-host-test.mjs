// Headless second-instance test of the Books plugin against a real Rox v1 broker (docs/06 section 13.2 steps 6-11).
// Run it through tools/rox/host-test.sh, which starts the throwaway broker. Direct use:
//   node books-host-test.mjs <rox worktree> <run dir> <books binary> <port>
// It reuses Rox's own test helpers (openApp, envelope) from the worktree. It prints step results only: no host
// token, ticket values, keys or signatures.
import { readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

const [WT, RUN, BIN, PORT] = process.argv.slice(2);
const { openApp, envelope } = await import(pathToFileURL(`${WT}/server/test/v1-helpers.mjs`).href);
const BASE = `http://127.0.0.1:${PORT}`;
const token = readFileSync(`${RUN}/data/host-token`, "utf8");
const signed = JSON.parse(readFileSync(`${RUN}/signed.json`, "utf8"));

async function post(path, body) {
  const r = await fetch(BASE + path, { method: "POST", headers: { "content-type": "application/json", "x-rox-host": token }, body: JSON.stringify(body) });
  const text = await r.text();
  let json;
  try { json = JSON.parse(text); } catch { json = { raw: text.slice(0, 200) }; }
  return { status: r.status, json };
}

const results = [];
const check = (name, ok, detail) => { results.push({ name, ok: !!ok }); console.log(`${ok ? "PASS" : "FAIL"}  ${name}${detail ? `  ${JSON.stringify(detail)}` : ""}`); };
const refusal = (r) => ({ status: r.status, code: r.json?.code ?? r.json?.error?.code });

const spec = { command: BIN, args: ["mcp"], env: { BOOKS_V1_MANIFEST: `${RUN}/signed.json` } };

// 0. a request without the host token is refused
const noAuth = await fetch(`${BASE}/v1/host/plugins/inspect`, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(spec) });
check("host route without X-Rox-Host is 401", noAuth.status === 401, { status: noAuth.status });

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
const res = await post("/v1/host/resolve", { plugin: "dev.goldcoders.books", kind: "command", name: "books", value: "" });
const r = res.json;
check("/books resolves to open home with a ticket and a neutral title", r.result === "open" && r.app === "home" && /^tkt_/.test(String(r.ticket ?? r.props?.ticket ?? "")),
  { status: res.status, result: r.result, app: r.app, title: r.title, has_ticket: /^tkt_/.test(String(r.ticket ?? r.props?.ticket ?? "")) });

// 5. open the home app the way Rox does (session key, open_id, permissions digest)
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

const failed = results.filter((x) => !x.ok).length;
console.log(`\n${results.length - failed} pass, ${failed} fail`);
process.exit(failed ? 1 : 0);
