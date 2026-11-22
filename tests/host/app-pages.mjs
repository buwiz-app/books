// Headless regression tests for the Books window's pages, driven on
// nsh-standin.mjs with a canned bridge. Covers the review findings on the
// old-UI PR: batch request ids, eligible-set bulk changes, detached signal
// setters on filter chips, the amount filter's sign bug, card-mode bulk
// selection, server-side search past the 200-row cap, the missing Back
// control, per-line assignments in the inbox, and the hidden nav pages.
//
// Run: node tests/host/app-pages.mjs
// esbuild resolves through $BUWIZ_SDK_DIR's conformance package first, else a
// bare "esbuild" import.
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { mountApp } from "./nsh-standin.mjs";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "../..");
const req = createRequire(
  process.env.BUWIZ_SDK_DIR ? join(process.env.BUWIZ_SDK_DIR, "packages/conformance/package.json") : import.meta.url,
);
const esbuild = req("esbuild");

// The window's join, kept in sync with BOOKS_FILES in
// crates/books-mini-app/build.rs (kit, the category catalog, then pages).
const PAGES = [
  "books/today.tsx",
  "books/ledger_review.tsx",
  "books/ledger_findings.tsx",
  "books/ledger_amend.tsx",
  "books/ledger_bulk.tsx",
  "books/entry.tsx",
  "books/entry_side.tsx",
  "books/inbox.tsx",
  "books/accounts.tsx",
  "books/category_detail.tsx",
  "books/unpaid.tsx",
  "books/owed_print.tsx",
  "books/owed_repeat.tsx",
  "books/people.tsx",
  "books/tags.tsx",
  "books/reports.tsx",
  "books/reconcile.tsx",
  "books/statement_read.tsx",
  "books/statement_snapshot.tsx",
  "books/reconcile_flags.tsx",
  "books/settings.tsx",
  "books/oldbooks.tsx",
  "books/forms.tsx",
  "books/payroll.tsx",
  "books/certificates.tsx",
  "books/activity.tsx",
  "books/transactions.tsx",
  "books/window.tsx",
];
const read = (f) => readFileSync(join(ROOT, "apps", f), "utf8");
const SOURCE =
  read("books/kit.tsx") +
  "\nconst SUGGESTED_CATEGORY_CATALOG = " +
  read("books/suggested-categories.json") +
  ";\n" +
  PAGES.map(read).join("\n");

// ---- canned data: synthetic rows only ----

const ROWS = [
  {
    ticket: "tkt_d1", no: "0001", date: "2026-10-28", date_short: "Oct 28",
    status: "draft", pill: "draft", status_label: "Draft", form: "money_in",
    category: "Sales", category_color: "#16a34a", amount: "400.00", amount_shown: "+400.00",
    tone: "pos", title: "Draft sale · Ana", sub: "", payee: "Ana", checks: [],
  },
  {
    ticket: "tkt_p1", no: "0002", date: "2026-10-26", date_short: "Oct 26",
    status: "posted", pill: "ok", status_label: "Posted", form: "money_out",
    category: "Supplies", category_color: "#2f6fd6", amount: "1,350.00", amount_shown: "−1,350.00",
    tone: "neg", title: "Supplies · Bob", sub: "", payee: "Bob", checks: [],
  },
  {
    ticket: "tkt_p2", no: "0003", date: "2026-10-25", date_short: "Oct 25",
    status: "posted", pill: "ok", status_label: "Posted", form: "money_in",
    category: "Sales", category_color: "#16a34a", amount: "40.00", amount_shown: "+40.00",
    tone: "pos", title: "Deposit · Cory", sub: "", payee: "Cory", checks: [],
  },
  {
    ticket: "tkt_r1", no: "0004", date: "2026-10-24", date_short: "Oct 24",
    status: "posted", pill: "in", status_label: "Reversal", form: "reversal",
    category: "Supplies", category_color: "#2f6fd6", amount: "1,350.00", amount_shown: "−1,350.00",
    tone: "neg", title: "Void of Supplies · Bob", sub: "", payee: "", checks: [],
  },
];
// Rows past the page cap: the server can still search them, the capped list
// never returns them.
const DEEP_ROWS = [
  {
    ticket: "tkt_o1", no: "0005", date: "2026-10-10", date_short: "Oct 10",
    status: "posted", pill: "ok", status_label: "Posted", form: "money_out",
    category: "Rent", category_color: "#6a46c9", amount: "80,000.00", amount_shown: "−80,000.00",
    tone: "neg", title: "Rent · Dana", sub: "", payee: "Dana", checks: [],
  },
  {
    ticket: "tkt_w1", no: "0006", date: "2026-10-09", date_short: "Oct 9",
    status: "draft", pill: "rev", status_label: "Needs review", form: "money_in",
    category: "", category_color: "", amount: "500.00", amount_shown: "+500.00",
    tone: "pos", title: "Old draft · Eli", sub: "", payee: "Eli", checks: [],
    needs_review: true, proposed_by: "agent",
  },
];

const ENTRY = {
  ticket: "tkt_d1", no: "0001", status: "draft", date_label: "October 28, 2026",
  big: "+400.00", tone: "pos", who_label: "Customer", payee: "Ana", reference: "",
  actions: ["discard", "edit", "post"], memo: "", evidence: [], review: [],
  lines: [
    {
      account: "Sales", debit_shown: "", credit_shown: "400.00",
      person: { ticket: "tkt_per1", name: "Ana Cruz" },
      department: { ticket: "tkt_tag1", name: "North" },
      location: { ticket: "tkt_tag2", name: "Riverside" },
    },
    { account: "Cash on hand", debit_shown: "400.00", credit_shown: "" },
  ],
};

const hay = (r) =>
  [r.no, r.title, r.payee, r.category, r.sub, r.form, r.status, r.date, r.amount_shown]
    .map((x) => String(x || "").toLowerCase())
    .join(" ");
const byTicket = (t) => [...ROWS, ...DEEP_ROWS].find((r) => r.ticket === t);

function ledger(args = {}) {
  const q = String(args.search || "").trim().toLowerCase();
  let rows = q ? [...ROWS, ...DEEP_ROWS] : [...ROWS];
  if (args.filter === "draft") rows = rows.filter((r) => r.status === "draft" && !r.needs_review);
  if (args.filter === "review") rows = rows.filter((r) => r.needs_review);
  if (q) rows = rows.filter((r) => hay(r).includes(q));
  return {
    book: { ticket: "tkt_book1" },
    month: {
      key: "2026-10", label: "October 2026", range: "Oct 1 – Oct 31",
      prev: "2026-09", next: "2026-11",
    },
    accounts: [],
    all_accounts: { ticket: null, number: "", name: "All accounts", balance_shown: "0.00", balance_label: "Cash" },
    account: { ticket: null, number: "", name: "All accounts", balance_shown: "0.00", balance_label: "Cash" },
    filter: args.filter || "all",
    counts: { all: rows.length, review: 1, draft: 1, posted: 2, checks: 0 },
    running_balance: false,
    rows: rows.slice(0, 200),
  };
}

function makeBridge(props, calls) {
  return {
    props,
    open: () => Promise.resolve({}),
    close: () => Promise.resolve({}),
    call(tool, args = {}) {
      calls.push({ tool, args });
      switch (tool) {
        case "books_status":
          return Promise.resolve({ status: "ready" });
        case "books_book_get":
          return Promise.resolve({ ticket: "tkt_book1", name: "Test book", accent: "green", basis: "cash" });
        case "books_today":
          return Promise.resolve({
            today_label: "October 29, 2026",
            cash: { label: "Cash on hand", shown: "0.00", rows: [] },
            month: { label: "Net income (MTD)", shown: "₱ 0.00", tone: "pos", rows: [] },
            unpaid: { label: "Unpaid", shown: "0.00", rows: [] },
            needs: [],
            needs_count: 0,
          });
        case "books_ledger_list":
          return Promise.resolve(ledger(args));
        case "books_accounts_list":
          return Promise.resolve({ rows: [] });
        case "books_entry_get":
          return Promise.resolve(ENTRY);
        case "books_app_void_entry": {
          const r = byTicket(args.entry);
          if (!r || r.status !== "posted" || r.form === "reversal") {
            return Promise.reject({ code: "wrong_status", message: "It cannot be voided." });
          }
          return Promise.resolve({ entry: {}, reversal: "tkt_rev" });
        }
        case "books_app_discard_entry": {
          const r = byTicket(args.entry);
          if (!r || r.status !== "draft") {
            return Promise.reject({ code: "wrong_status", message: "It cannot be discarded." });
          }
          return Promise.resolve({ entry: {} });
        }
        case "books_app_batch": {
          const bad = (args.entries || []).map(byTicket).find((r) => !r || r.status !== "draft");
          if (bad) return Promise.reject({ code: "wrong_status", message: "An entry cannot be discarded." });
          return Promise.resolve({ action: args.action, done: (args.entries || []).length, entries: args.entries });
        }
        default:
          return Promise.resolve({});
      }
    },
  };
}

// ---- tiny harness ----

const tests = [];
const test = (name, fn) => tests.push({ name, fn });
const calls = [];
const mount = (props = {}) => mountApp(SOURCE, { esbuild, bridge: makeBridge(props, calls) });
const ok = (cond, what) => { if (!cond) throw new Error(what); };
const inputByPrefix = (screen, prefix) =>
  screen.nodes().find((n) => n.host === "Input" && String(n.props.placeholder || "").startsWith(prefix));
const typePrefix = (screen, prefix, value) => {
  const n = inputByPrefix(screen, prefix);
  if (!n) throw new Error(`no input whose placeholder starts ${JSON.stringify(prefix)}`);
  n.props.onChange({ value });
};

const fresh = async (props) => {
  calls.length = 0;
  const s = mount(props);
  await s.until(() => s.hasText("Transactions") || s.hasText("Draft sale") || s.hasText("October"), "the page to render");
  return s;
};

// ---- tests ----

test("home lists payroll, certificates and tax hand-off", async () => {
  const s = await fresh({});
  await s.until(() => s.hasText("Transactions"), "nav tiles");
  ok(s.hasText("Pay runs"), "Home has no Pay runs tile");
  ok(s.hasText("Certificates"), "Home has no Certificates tile");
  ok(s.hasText("Tax hand-off"), "Home has no Tax hand-off tile");
  s.unmount();
});

test("transactions has a way back home", async () => {
  const s = await fresh({ page: "transactions" });
  await s.until(() => s.hasText("Supplies"), "rows");
  ok(s.pressable("‹ Back") || s.pressable("‹ Back to Dashboard") || s.pressable("‹ Backto Dashboard"),
    "no Back control on the Transactions header");
  s.press("‹ Back");
  await s.until(() => s.hasText("Reconcile"), "home tiles after Back");
  s.unmount();
});

test("type chip filters the list", async () => {
  const s = await fresh({ page: "transactions" });
  await s.until(() => s.hasText("Supplies"), "rows");
  s.press("Filters");
  s.press("Money in");
  ok(!s.hasText("Supplies · Bob"), "money-out row still shown after picking Money in");
  ok(s.hasText("Deposit · Cory"), "money-in row missing after picking Money in");
  s.unmount();
});

test("amount max compares digits, not the display sign", async () => {
  const s = await fresh({ page: "transactions" });
  await s.until(() => s.hasText("Deposit"), "rows");
  s.press("Filters");
  typePrefix(s, "Max", "50");
  await s.until(() => !s.hasText("Supplies · Bob"), "the over-max row to drop out");
  ok(s.hasText("Deposit · Cory"), "+40.00 wrongly excluded by max 50");
  s.unmount();
});

test("each bulk action gets a fresh request id", async () => {
  const s = await fresh({ page: "transactions" });
  await s.until(() => s.hasText("Supplies"), "rows");
  for (let round = 0; round < 2; round++) {
    s.press("Edit");
    s.pressContaining("Supplies", 0);
    await s.until(() => s.hasText("1 selected"), "the row to be selected");
    s.press("Void 1 posted");
    s.press("Void");
    await s.until(() => !s.hasText("Void transactions"), "the confirm to close");
  }
  const ids = calls.filter((c) => c.tool === "books_app_void_entry").map((c) => c.args.client_request_id);
  ok(ids.length === 2, `expected 2 void calls, got ${ids.length}`);
  ok(ids.every(Boolean), "a void call went without client_request_id");
  ok(new Set(ids).size === ids.length, `request ids repeat across batches: ${ids.join(", ")}`);
  s.unmount();
});

test("bulk void sends only eligible posted rows; discard uses one atomic batch", async () => {
  const s = await fresh({ page: "transactions" });
  await s.until(() => s.hasText("Supplies"), "rows");
  s.press("Edit");
  s.pressContaining("Draft sale", 0); // a draft
  s.pressContaining("Supplies", 0); // posted
  s.pressContaining("Void of", 0); // a posted reversal
  await s.until(() => s.hasText("3 selected"), "three rows selected");
  ok(s.hasText("Void 1 posted"), "reversal counted in the void set");
  s.press("Void 1 posted");
  s.press("Void");
  await s.until(() => !s.hasText("Void transactions"), "the confirm to close");
  const voids = calls.filter((c) => c.tool === "books_app_void_entry");
  ok(voids.length === 1 && voids[0].args.entry === "tkt_p1",
    `void sent ${JSON.stringify(voids.map((c) => c.args.entry))}, want ["tkt_p1"]`);
  s.press("Discard 1 drafts");
  s.press("Discard");
  await s.until(() => !s.hasText("Discard drafts"), "the confirm to close");
  const batches = calls.filter((c) => c.tool === "books_app_batch");
  ok(batches.length === 1 && batches[0].args.action === "discard" && batches[0].args.entries.join() === "tkt_d1",
    `discard did not go through books_app_batch: ${JSON.stringify(batches.map((c) => c.args))}`);
  ok(!calls.some((c) => c.tool === "books_app_discard_entry"), "per-entry discard used instead of batch");
  s.unmount();
});

test("card rows select in edit mode", async () => {
  const s = await fresh({ page: "transactions" });
  await s.until(() => s.hasText("Supplies"), "rows");
  s.press("Edit");
  s.pressContaining("Supplies", "last"); // the below-md card copy
  await s.until(() => s.hasText("1 selected"), "card press to select the row", 3000);
  s.unmount();
});

test("search reaches rows past the 200-row cap", async () => {
  const s = await fresh({ page: "transactions" });
  await s.until(() => s.hasText("Supplies"), "rows");
  ok(!s.hasText("Rent · Dana"), "capped-out row wrongly visible before search");
  typePrefix(s, "Search ", "Dana");
  await s.until(() => s.hasText("Rent · Dana"), "search to reach the older row");
  s.unmount();
});

test("inbox search reaches rows past the 200-row cap", async () => {
  const s = await fresh({ page: "inbox" });
  await s.until(() => s.hasText("waiting to be booked"), "the inbox");
  typePrefix(s, "Search inbox", "Eli");
  await s.until(() => s.hasText("Old draft · Eli"), "inbox search to reach the older row");
  s.unmount();
});

test("inbox detail shows per-line people and tags", async () => {
  const s = await fresh({ page: "inbox" });
  await s.until(() => s.hasText("Suggested entry"), "the detail pane");
  ok(s.hasText("Ana Cruz"), "line person not shown");
  ok(s.hasText("North"), "line department not shown");
  ok(s.hasText("Riverside"), "line location not shown");
  s.unmount();
});

// ---- run ----

let failed = 0;
for (const t of tests) {
  try {
    await t.fn();
    console.log(`PASS  ${t.name}`);
  } catch (e) {
    failed += 1;
    console.log(`FAIL  ${t.name}  ${e && e.message ? e.message : e}`);
  }
}
console.log(failed ? `${failed} test(s) failed` : "all tests passed");
process.exit(failed ? 1 : 0);
