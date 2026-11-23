// PR #37 review regressions, agent-d share: drive the app TSX headless through
// tests/host/nsh-standin.mjs the way the Bot mounts it (kit.tsx + the page
// files, joined as crates/books-mini-app/src/apps.rs does). The bridge is a
// recording stub; fixtures carry synthetic data only. Run:
//   node --test tests/app/agent-d.test.mjs   (needs the SDK's node_modules for esbuild)

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";
import { mountApp, textOf } from "../host/nsh-standin.mjs";

// esbuild resolves through $BUWIZ_SDK_DIR's conformance package, as
// tests/host/app-pages.mjs does; the home-relative path is a fallback.
const require = createRequire(
  process.env.BUWIZ_SDK_DIR
    ? join(process.env.BUWIZ_SDK_DIR, "packages/conformance/package.json")
    : "/home/ubuntu/repos/buwiz-sdk/packages/conformance/package.json",
);
const esbuild = require("esbuild");

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "../..");
const appSrc = (p) => readFileSync(join(ROOT, "apps", p), "utf8").replace(/^export default /m, "");
const KIT = appSrc("books/kit.tsx");
const bundle = (files, shim) => [KIT, ...files.map(appSrc), shim].join("\n");

const REC_FILES = [
  "books/reconcile_flags.tsx",
  "books/statement_read.tsx",
  "books/statement_snapshot.tsx",
  "books/reconcile.tsx",
  "books/reconcile_side.tsx",
];
const REC_SHIM = `
const drawerEntry = signal("");
function openLedger() {}
export default function App() {
  recAccount.set("acct-card-1");
  recMonth.set("2025-10");
  return <ReconcilePage go={() => {}} />;
}
`;

function mount(source, answer) {
  const calls = [];
  const bridge = {
    props: {},
    call: async (tool, args) => {
      calls.push([tool, args]);
      const r = await answer(tool, args);
      return r === undefined ? {} : r;
    },
  };
  const screen = mountApp(source, { esbuild, bridge });
  screen.calls = calls;
  return screen;
}

const texts = (screen) => screen.nodes().filter((n) => n.host === "Text").map(textOf).filter((t) => t !== "");
const hasExact = (screen, t) => texts(screen).includes(t);
const pressables = (screen, label) => {
  const n = screen.pressable(label);
  return n ? [n] : [];
};
const boxes = (screen) => screen.nodes().filter((n) => String(n.props.class || "").includes("absolute border-2"));
const rowWith = (screen, frag) => screen.nodes().find((n) => typeof n.props.onPress === "function" && textOf(n).includes(frag));

// A "finished" statement: one matched charge, one matched payment (a card:
// negative rows are charges, positive rows payments), synthetic amounts.
function statementFixture({ finished = false, card = true, snapshot = false, rows } = {}) {
  return {
    account: { ticket: "acct-card-1", name: card ? "Card" : "Bank", debit_normal: card ? false : true },
    month: { key: "2025-10", label: "October 2025", short: "Oct" },
    statement: {
      status: finished ? "finished" : "draft",
      file_name: "statement.csv",
      closing_shown: "60.00",
      books_shown: "60.00",
      books_in_shown: card ? "100.00" : "40.00",
      books_out_shown: card ? "40.00" : "100.00",
      difference: "0.00",
      difference_shown: "0.00",
      suggested: 0,
      missing: 0,
      can_finish: false,
      flags: [],
      hints_used: 0,
      ...(snapshot ? { snapshot: "doc-1", snapshot_kind: "image", snapshot_pictures: [1], evidence: "doc-1", snapshot_stamp: "st-1" } : {}),
      rows: rows || [
        { no: 1, date_short: "Oct 3", amount_shown: "-100.00", description: "Store purchase", match: "m", books: { date_short: "Oct 3", title: "Posted", sub: "Same amount and date", amount_shown: "-100.00", lines: [] } },
        { no: 2, date_short: "Oct 25", amount_shown: "+40.00", description: "Card payment", match: "m", books: { date_short: "Oct 25", title: "Posted", sub: "Same amount and date", amount_shown: "+40.00", lines: [] } },
      ],
    },
  };
}

const mountRec = (fixture, shimExtra = "") =>
  mount(bundle(REC_FILES, REC_SHIM + shimExtra), (tool) =>
    tool === "books_statement_get" ? fixture : tool === "books_reconcile_grid" ? { year: 2026, rows: [] } : tool === "books_accounts_list" ? { rows: [] } : {},
  );

test("F1: a card statement's Charges line pairs the bank's negative rows with the ledger's own sense", async () => {
  const screen = mountRec(statementFixture());
  await screen.until(() => screen.hasText("Charges"), "Charges card");
  const t = texts(screen);
  const i = t.indexOf("Charges");
  assert.deepEqual(t.slice(i, i + 5), ["Charges", "Bank", "+100.00", "Ledger", "100.00"]);
  const j = t.indexOf("Payments");
  assert.deepEqual(t.slice(j, j + 5), ["Payments", "Bank", "-40.00", "Ledger", "40.00"]);
});

test("F1: a bank statement keeps Deposits on positive rows and Withdrawals on negative ones", async () => {
  const screen = mountRec(statementFixture({ card: false }));
  await screen.until(() => screen.hasText("Deposits"), "Deposits card");
  const t = texts(screen);
  const i = t.indexOf("Deposits");
  assert.deepEqual(t.slice(i, i + 5), ["Deposits", "Bank", "+40.00", "Ledger", "40.00"]);
  const j = t.indexOf("Withdrawals");
  assert.deepEqual(t.slice(j, j + 5), ["Withdrawals", "Bank", "-100.00", "Ledger", "100.00"]);
});

test("F8: the statement page offers only the boxes of rows the filter leaves", async () => {
  const rows = [
    { no: 1, page: 1, box: { x: 0, y: 0, w: 10, h: 10 }, date_short: "Oct 3", amount_shown: "-100.00", description: "Store purchase", match: "m", books: { date_short: "Oct 3", title: "Posted", sub: "", amount_shown: "-100.00", lines: [] } },
    { no: 2, page: 1, box: { x: 0, y: 20, w: 10, h: 10 }, date_short: "Oct 25", amount_shown: "+40.00", description: "Card payment", match: "s", books: { date_short: "Oct 25", title: "Suggested match", sub: "", amount_shown: "+40.00", lines: [] } },
  ];
  const screen = mountRec(statementFixture({ snapshot: true, rows }), "recStat.set(\"m\");\n");
  await screen.until(() => boxes(screen).length > 0, "statement page boxes");
  assert.equal(boxes(screen).length, 1, "only the matched row's box is pressable");
});

test("F9: a finalized statement offers no writes (no Accept, no Import, no + Add)", async () => {
  const rows = [
    { no: 1, date_short: "Oct 3", amount_shown: "-100.00", description: "Store purchase", match: "x", books: { title: "Not in Books", sub: "Add it as money out" } },
  ];
  const fixture = statementFixture({ finished: true, rows });
  fixture.statement.suggested = 1;
  fixture.statement.missing = 1;
  const screen = mountRec(fixture);
  await screen.until(() => screen.hasText("Finalized"), "finalized pill");
  assert.equal(screen.pressable("Accept 1 suggestion"), null, "Accept is gone once finalized");
  assert.equal(pressables(screen, "+").length + pressables(screen, "+ Add").length, 0, "no import or add control");
});

test("F11: MatchView always has a route back to the months grid", async () => {
  const screen = mountRec(statementFixture({ finished: true }));
  await screen.until(() => screen.hasText("Finalized"), "finalized statement");
  const back = screen.pressable("‹ Back");
  assert.ok(back, "a Back control exists");
  back.props.onPress();
  await screen.until(() => screen.hasText("Reconciliations"), "the months grid");
});

// ── reports ──────────────────────────────────────────────────────────────

const REPORT_FIXTURE = {
  compare: true,
  columns: ["Oct 2025", "Sep 2025"],
  period: { key: "2025-10", label: "October 2025", prev: "2025-09", next: "2025-11" },
  sections: [
    { title: "Expenses", rows: [{ name: "Supplies", shown: "500.00", prev_shown: "0.00" }] },
  ],
  totals: [
    { label: "Total revenue", shown: "0.00", prev_shown: "0.00" },
    { label: "Total expenses", shown: "500.00", prev_shown: "0.00" },
  ],
  net: { label: "Net income", value: "-500.00", shown: "-500.00", prev_shown: "0.00" },
};

const mountReports = (go = "() => {}") =>
  mount(
    bundle(["books/reports.tsx"], `function openLedger() {}\nfunction LedgerBulk() { return null; }\nexport default function App() { return <ReportsPage go={${go}} />; }`),
    (tool) => (tool === "books_report" ? REPORT_FIXTURE : { rows: [] }),
  );

test("F2: a prior balance that is a real zero shows 0.00 and the absolute change, not an em dash", async () => {
  const screen = mountReports();
  await screen.until(() => screen.hasText("Supplies"), "P&L row");
  const row = rowWith(screen, "Supplies");
  const t = textOf(row);
  assert.ok(t.includes("0.00"), "the real zero prior is shown, got: " + t);
  assert.ok(t.includes("▲"), "the change badge appears without a percentage, got: " + t);
});

test("F3: the report tabs sit on their own wrapping row so narrow windows reach Aging and Month pack", async () => {
  const screen = mountReports();
  await screen.until(() => screen.hasText("Financials"), "the reports band");
  const tabs = screen.nodes().find((n) => String(n.props.class || "").includes("bg-white/10 rounded-full") && textOf(n).includes("Aging"));
  assert.ok(tabs, "the tab strip exists");
  const outer = screen.nodes().find((n) => {
    const c = String(n.props.class || "");
    return c.includes("flex-col") && textOf(n).includes("Financials") && textOf(n).includes("Aging");
  });
  assert.ok(outer, "title and tabs are in a column, so each row has the full width");
});

test("F5: the Financials band uses the book accent like every page header", async () => {
  const screen = mountReports();
  await screen.until(() => screen.hasText("Financials"), "the reports band");
  const band = screen.nodes().find((n) => textOf(n).includes("Financials") && (n.props.style || {}).backgroundImage);
  assert.equal(band, undefined, "no fixed gradient: the band follows the accent");
  const accent = screen.nodes().find((n) => textOf(n).includes("Financials") && (n.props.style || {}).backgroundColor === "#22c55e");
  assert.ok(accent, "the band is the book's accent (green is the default)");
});

test("Reports has a way back home", async () => {
  globalThis.__wentTo = "";
  const screen = mountReports("(page) => { globalThis.__wentTo = page; }");
  await screen.until(() => screen.hasText("Financials"), "the reports band");
  const back = pressables(screen, "‹ Back")[0];
  assert.ok(back, "no Back control on the Reports band");
  back.props.onPress();
  assert.equal(globalThis.__wentTo, "today", "Back goes Home");
});

// ── owed_repeat ──────────────────────────────────────────────────────────

test("F4: a repeating bill's reference and date stay visible below md", async () => {
  const rows = [
    { owed: "owed-a", who: "VendCo", doc: "Service A", issued: "2025-10-01", amount_shown: "500.00" },
    { owed: "owed-b", who: "VendCo", doc: "Service B", issued: "2025-10-01", amount_shown: "500.00" },
  ];
  const screen = mount(
    bundle(["books/owed_repeat.tsx"], `export default function App() { return <RepeatsDue rows={${JSON.stringify(rows)}} />; }`),
    () => ({}),
  );
  const visible = (frag) =>
    screen
      .nodes()
      .filter((n) => n.host === "Text")
      .some((n) => textOf(n).includes(frag) && !String(n.props.class || "").split(/\s+/).includes("hidden"));
  assert.ok(visible("Service A"), "Service A's reference is visible on a narrow screen");
  assert.ok(visible("Service B"), "Service B's reference is visible on a narrow screen");
});

// ── evidence ─────────────────────────────────────────────────────────────

const EVIDENCE_SHIM = `export default function App() { return <Evidence />; }`;
const DOC_A = { evidence: "doc-a", kind: "receipt", mime: "image/png", page_count: 1, source: "user", linked_count: 0 };
const DOC_B = { evidence: "doc-b", kind: "bill", mime: "application/pdf", page_count: 2, source: "user", linked_count: 0 };

test("F6: the library follows next_cursor so filters cover every document", async () => {
  let page = 0;
  const screen = mount(bundle(["evidence.tsx"], EVIDENCE_SHIM), (tool, args) => {
    if (tool === "books_app_evidence_review") return { rows: [] };
    if (tool === "books_evidence_list") {
      page += 1;
      return page === 1 ? { items: [DOC_A], next_cursor: "cur-a" } : { items: [DOC_B], next_cursor: null };
    }
    return {};
  });
  await screen.until(() => screen.hasText("Receipt"), "the first page's document");
  assert.equal(page, 2, "the second page was fetched");
  assert.ok(screen.calls.some(([, a]) => a && a.cursor === "cur-a"), "the page turn sent the cursor");
  await screen.until(() => screen.hasText("Bill"), "the second page's document");
});

test("F7: a filter that matches nothing does not show the first-upload prompt", async () => {
  const screen = mount(
    bundle(["evidence.tsx"], `export default function App() { return <Evidence />; }`),
    (tool) => (tool === "books_app_evidence_review" ? { rows: [] } : { items: [DOC_A], next_cursor: null }),
  );
  await screen.until(() => screen.hasText("Receipt"), "the library document");
  screen.type("Filter 1 document…", "zzz-nothing-matches");
  await screen.until(() => !screen.hasText("Receipt"), "the query to filter it out");
  assert.equal(screen.hasText("No documents yet"), false, "the library is not empty");
  assert.ok(screen.hasText("No documents match"), "a no-match note shows instead");
});
