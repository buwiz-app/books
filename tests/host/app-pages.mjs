// Headless regression tests for the Books window's pages, driven on
// nsh-standin.mjs with a canned bridge. Covers the review findings on the
// old-UI PR: batch request ids, eligible-set bulk changes, detached signal
// setters on filter chips, the amount filter's sign bug, card-mode bulk
// selection, server-side search past the 200-row cap, the missing Back
// control, per-line assignments in the inbox, the hidden nav pages, and the
// new-entry page's draft/post flow (save twice edits, post drafts the new
// values first, a picked party goes as `person`, an unbalanced journal saves
// as draft and refuses to post).
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

// The window's join, read from BOOKS_FILES in crates/books-mini-app/build.rs
// (kit, the category catalog, then the page files with window.tsx last).
const FILES = [...readFileSync(join(ROOT, "crates/books-mini-app/build.rs"), "utf8")
    .matchAll(/"\.\.\/\.\.\/apps\/([^"]+\.(?:tsx|json))"/g)]
  .map((m) => m[1]);
if (FILES[0] !== "books/kit.tsx" || FILES[1] !== "books/suggested-categories.json" || FILES.length < 3) {
  throw new Error("build.rs BOOKS_FILES changed shape; kit and the category catalog must lead");
}
const PAGES = FILES.slice(2);
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

// Accounts and people the New entry form picks from (synthetic).
const ACCOUNTS = [
  { ticket: "tkt_cash1", number: "1010", name: "Cash on hand", label: "1010 Cash on hand", group: "Cash, bank & card", type: "asset", kind: "cash" },
  { ticket: "tkt_sup1", number: "5010", name: "Supplies", label: "5010 Supplies", group: "Expense categories", type: "expense", kind: "expense" },
];
const PEOPLE = [
  { ticket: "tkt_ven1", name: "Vera Vendor", kind: "vendors" },
  { ticket: "tkt_cus1", name: "Cory Customer", kind: "customers" },
];
const TAGS = [
  { ticket: "tkt_tag1", name: "North", kind: "department" },
  { ticket: "tkt_tag2", name: "Riverside", kind: "location" },
];
const OPTIONS = {
  book: { ticket: "tkt_book1", name: "Test book", accent: "green", basis: "cash" },
  today: "2026-10-29",
  money: [ACCOUNTS[0]],
  income: [],
  expense: [ACCOUNTS[1]],
  all: ACCOUNTS,
};

const hay = (r) =>
  [r.no, r.title, r.payee, r.category, r.sub, r.form, r.status, r.date, r.amount_shown]
    .map((x) => String(x || "").toLowerCase())
    .join(" ");
const byTicket = (t) => [...ROWS, ...DEEP_ROWS].find((r) => r.ticket === t);

// The tool's contract: `search` and every one of `terms` must match; a page
// of `rows`, `total`, `next_cursor`; a group names its rows by their place
// in `rows`. `LEDGER_PAGE` stands in for the tool's 200 so a test can page.
let LEDGER_PAGE = 200;
function ledger(args = {}) {
  const terms = [args.search, ...(args.terms || [])].map((t) => String(t || "").trim().toLowerCase()).filter(Boolean);
  let rows = terms.length || LEDGER_PAGE < 200 ? [...ROWS, ...DEEP_ROWS] : [...ROWS];
  if (args.filter === "draft") rows = rows.filter((r) => r.status === "draft" && !r.needs_review);
  if (args.filter === "review") rows = rows.filter((r) => r.needs_review);
  rows = rows.filter((r) => terms.every((t) => hay(r).includes(t)));
  const at = Number(args.cursor || 0);
  const page = rows.slice(at, at + LEDGER_PAGE);
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
    total: rows.length,
    next_cursor: at + page.length < rows.length ? String(at + page.length) : null,
    rows: page,
    groups: rows.length
      ? [{
          ticket: "tkt_cash1", number: "1010", name: "Cash on hand", type: "asset",
          count: rows.length, debit_shown: "440.00", credit_shown: "2,700.00", balance_shown: "−2,260.00",
          rows: page.map((_, i) => ({ row: i, debit_shown: "", credit_shown: "", balance_shown: null })),
        }]
      : [],
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
          return Promise.resolve({ rows: args.group === "all" ? ACCOUNTS : [] });
        case "books_entry_options":
          return Promise.resolve(OPTIONS);
        case "books_people_list":
          // one kind a call, vendors when none is named (records::people_list)
          return Promise.resolve({ rows: PEOPLE.filter((x) => x.kind === (args.kind || "vendors")) });
        case "books_tags_list":
          // one kind a call, `kind` beside `rows` (records::tags_list)
          return Promise.resolve({ kind: args.kind || "department", rows: TAGS.filter((x) => x.kind === (args.kind || "department")).map((x) => ({ ticket: x.ticket, name: x.name })) });
        case "books_entry_draft":
          return Promise.resolve({ entry: args.entry || "tkt_new1", no: "0009", status: "draft" });
        case "books_app_post_entry":
          return Promise.resolve({ entry: args.entry || "tkt_new1", no: "0009", status: "posted" });
        case "books_app_evidence_attach":
          return Promise.resolve({});
        case "books_app_person_create":
          return Promise.resolve({ person: "tkt_per9" });
        case "books_entry_get":
          return Promise.resolve(ENTRY);
        case "books_app_void_entry": {
          const r = byTicket(args.entry);
          if (r && r.locked) {
            return Promise.reject({ code: "statement_finished", message: "This entry is matched to the bank in a finished month." });
          }
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

// New-entry drivers: combos are nsh Combobox host nodes picked by their
// accessibility label; the save menu opens on its icon View's a11y label.
const combos = (screen, label) =>
  screen.nodes().filter((n) => n.host === "Combobox" && n.props.accessibilityLabel === label);
const pick = (screen, label, ticket, occurrence = 0) => {
  const n = combos(screen, label)[occurrence];
  if (!n) throw new Error(`no combo labelled ${JSON.stringify(label)}`);
  n.props.onChange({ value: [ticket] });
};
const inputsByLabel = (screen, label) =>
  screen.nodes().filter((n) => n.host === "Input" && n.props.accessibilityLabel === label);
const pressA11y = (screen, label) => {
  const n = screen.nodes().find((n) => typeof n.props.onPress === "function" && n.props.accessibilityLabel === label);
  if (!n) throw new Error(`nothing pressable labelled ${JSON.stringify(label)}`);
  n.props.onPress();
};
const openEntry = async (s, tile = "+ Money out") => {
  s.press(tile);
  await s.until(() => s.hasText("Submit"), "the new entry page");
};
const submitEntry = (s) => {
  s.press("Submit");
};
// a write call is recorded when it starts; further presses are ignored while
// busy, so wait for the button to leave "Saving..." before pressing again
const settle = async (s) => s.until(() => !s.hasText("Saving..."), "the write to settle");
const fillExpense = (s, amount = "100.00") => {
  s.type("0.00", amount);
  pick(s, "Money account", "tkt_cash1");
  pick(s, "Category", "tkt_sup1");
  s.type("₱0.00", amount);
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

test("transactions renders full-height ledger with accounts sidebar", async () => {
  const s = await fresh({ page: "transactions" });
  await s.until(() => s.hasText("Supplies"), "rows");
  ok(s.hasText("Ledger"), "missing Ledger header");
  ok(s.hasText("Cash on hand"), "missing accounts sidebar");
  ok(!s.hasText("Back to Dashboard"), "Back to Dashboard should be absent");
  s.unmount();
});

test("type chip filters the list", async () => {
  const s = await fresh({ page: "transactions" });
  await s.until(() => s.hasText("Supplies"), "rows");
  s.press("Filters");
  s.press("Pay In");
  // the row's own text: the Party picker lists the payees too
  ok(!s.hasText("Supplies · Bob"), "money-out row still shown after picking Pay In");
  ok(s.hasText("Deposit · Cory"), "money-in row missing after picking Pay In");
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
  ok(!s.hasText("Dana"), "capped-out row wrongly visible before search");
  typePrefix(s, "Search ", "Dana");
  await s.until(() => s.hasText("Dana"), "search to reach the older row");
  s.unmount();
});

test("inbox search reaches rows past the 200-row cap", async () => {
  const s = await fresh({ page: "inbox" });
  await s.until(() => s.hasText("waiting to be booked"), "the inbox");
  typePrefix(s, "Search inbox", "Eli");
  await s.until(() => s.hasText("Old draft · Eli"), "inbox search to reach the older row");
  s.unmount();
});

test("search keeps a row the ledger matched by amount", async () => {
  const s = await fresh({ page: "transactions" });
  await s.until(() => s.hasText("Supplies"), "rows");
  typePrefix(s, "Search ", "80,000");
  await s.until(() => s.hasText("Dana"), "an amount search to keep the matched row");
  s.unmount();
});

test("inbox search keeps a row the ledger matched by date", async () => {
  const s = await fresh({ page: "inbox" });
  await s.until(() => s.hasText("waiting to be booked"), "the inbox");
  typePrefix(s, "Search inbox", "2026-10-09");
  await s.until(() => s.hasText("Old draft · Eli"), "a date search to keep the matched row");
  s.unmount();
});

test("import journal entries opens the import page", async () => {
  const s = await fresh({ page: "transactions" });
  await s.until(() => s.hasText("Supplies"), "rows");
  pressA11y(s, "More add options");
  s.press("Import Journal Entries…");
  await s.until(() => !s.hasText("Ledger"), "the ledger to give way");
  ok(s.nodes().some((n) => n.host === "Text"), "Import Journal Entries opened a blank page");
  s.unmount();
});

test("search terms all go to the tool, whatever order they were typed in", async () => {
  for (const order of [["posted", "Dana"], ["Dana", "posted"]]) {
    const s = await fresh({ page: "transactions" });
    await s.until(() => s.hasText("Supplies"), "rows");
    for (const term of order) {
      typePrefix(s, "Search ", term);
      inputByPrefix(s, "Search ").props.onSubmit();
    }
    await s.until(() => s.hasText("Rent · Dana") && !s.hasText("Supplies · Bob"), `the ${order.join(" + ")} search`);
    const last = calls.filter((c) => c.tool === "books_ledger_list").pop();
    ok([...last.args.terms].sort().join() === "Dana,posted", `terms sent: ${JSON.stringify(last.args.terms)}`);
    s.unmount();
  }
});

test("matched, source and tag filters read the tool's own row fields", async () => {
  const supplies = ROWS.find((r) => r.title.startsWith("Supplies"));
  Object.assign(supplies, { matched: true, source: "bank", departments: ["North"], locations: ["Riverside"] });
  try {
    const s = await fresh({ page: "transactions" });
    await s.until(() => s.hasText("Supplies · Bob"), "rows");
    s.press("Filters");
    for (const [section, option] of [["Matched Status", "Matched"], ["Source", "Bank"], ["Department", "North"], ["Location", "Riverside"]]) {
      s.press(section);
      await s.until(() => s.hasText(option), `the ${section} options`);
      s.press(option);
      await s.until(() => !s.hasText("Deposit · Cory"), `${option} to narrow the list`);
      ok(s.hasText("Supplies · Bob"), `${option} dropped the row that has it`);
      s.press("Clear all");
      await s.until(() => s.hasText("Deposit · Cory"), "the filters to clear");
    }
    s.unmount();
  } finally {
    for (const k of ["matched", "source", "departments", "locations"]) delete supplies[k];
  }
});

test("the party picker lists every kind, and its search box hides no checked party", async () => {
  const s = await fresh({ page: "transactions" });
  await s.until(() => s.hasText("Supplies · Bob"), "rows");
  s.press("Filters");
  await s.until(() => s.hasText("Cory Customer") && s.hasText("Vera Vendor"), "vendors and customers in the picker");
  s.press("Bob"); // a payee on a row, not in People
  await s.until(() => !s.hasText("Deposit · Cory"), "the party filter to apply");
  typePrefix(s, "Search parties", "Cory");
  ok(s.hasText("Supplies · Bob"), "typing in the picker's search hid a checked party's rows");
  s.unmount();
});

test("a refused void keeps the rest of the selection", async () => {
  const cory = ROWS.find((r) => r.title.startsWith("Deposit"));
  cory.locked = true;
  try {
    const s = await fresh({ page: "transactions" });
    await s.until(() => s.hasText("Supplies"), "rows");
    s.press("Edit");
    s.pressContaining("Deposit", 0); // refused: matched in a finished month
    s.pressContaining("Supplies", 0);
    await s.until(() => s.hasText("2 selected"), "two rows selected");
    s.press("Void 2 posted");
    s.press("Void");
    await s.until(() => !s.hasText("Void transactions"), "the confirm to close");
    const voids = calls.filter((c) => c.tool === "books_app_void_entry").map((c) => c.args.entry);
    ok(voids.length >= 1, "no void was sent");
    const sent = new Set(voids);
    const refusedFirst = voids[0] === cory.ticket;
    // whatever was not voided is still picked
    ok(s.hasText(refusedFirst ? "2 selected" : "1 selected"), `selection after a refused void (sent ${[...sent].join()})`);
    s.unmount();
  } finally {
    delete cory.locked;
  }
});

test("discard refuses more drafts than one batch takes, before asking", async () => {
  const extra = Array.from({ length: 51 }, (_, i) => ({
    ticket: `tkt_bulk${i}`, no: String(100 + i), date: "2026-10-05", date_short: "Oct 5",
    status: "draft", pill: "in", status_label: "Draft", form: "money_out",
    category: "Supplies", category_color: "#2f6fd6", amount: "1.00", amount_shown: "−1.00",
    tone: "neg", title: `Bulk draft ${i}`, sub: "", payee: "", checks: [],
  }));
  ROWS.push(...extra);
  try {
    const s = await fresh({ page: "transactions" });
    await s.until(() => s.hasText("Bulk draft 0"), "rows");
    s.press("Edit");
    s.nodes().find((n) => typeof n.props.onPress === "function" && n.props.class && String(n.props.class).includes("md:w-auto")).props.onPress();
    await s.until(() => s.hasText("Discard 52 drafts"), "the group to be selected");
    s.press("Discard 52 drafts");
    ok(!s.hasText("Discard drafts"), "the confirm opened for more than a batch takes");
    ok(!calls.some((c) => c.tool === "books_app_batch"), "an over-limit batch was sent");
    s.unmount();
  } finally {
    ROWS.splice(ROWS.length - extra.length, extra.length);
  }
});

test("load more pages the grouped ledger", async () => {
  LEDGER_PAGE = 4;
  try {
    const s = await fresh({ page: "transactions" });
    await s.until(() => s.hasText("Supplies"), "rows");
    ok(!s.hasText("Dana"), "a later page's row shown before Load more");
    ok(s.hasText("Loaded 4 of 6"), "missing the loaded count");
    s.press("Load more");
    await s.until(() => s.hasText("Dana") && s.hasText("Loaded 6 of 6"), "the next page in the group");
    ok(!s.hasText("Load more"), "Load more offered past the last page");
    s.unmount();
  } finally {
    LEDGER_PAGE = 200;
  }
});

test("submit saves as draft with no dropdown and starts the next draft fresh", async () => {
  const s = await fresh({});
  await openEntry(s);
  ok(!s.hasText("Save & Close"), "old Save & Close label still rendered");
  ok(!s.hasText("Post"), "Post action still rendered on new entry");
  fillExpense(s);
  submitEntry(s);
  await s.until(() => calls.some((c) => c.tool === "books_entry_draft"), "the first save");
  await settle(s);
  fillExpense(s, "60.00");
  submitEntry(s);
  await s.until(() => calls.filter((c) => c.tool === "books_entry_draft").length === 2, "the second save");
  const drafts = calls.filter((c) => c.tool === "books_entry_draft");
  ok(!drafts[0].args.entry, `the first save already sent entry ${drafts[0].args.entry}`);
  ok(!drafts[1].args.entry, `Submit left a remembered draft: ${JSON.stringify(drafts[1].args)}`);
  ok(!calls.some((c) => c.tool === "books_app_post_entry"), "a post went out on Submit");
  const ids = drafts.map((c) => c.args.client_request_id);
  ok(ids.every(Boolean) && new Set(ids).size === ids.length, `request ids repeat: ${ids.join(", ")}`);
  s.unmount();
});

test("a picked payee goes to the draft as person", async () => {
  const s = await fresh({});
  await openEntry(s);
  fillExpense(s);
  pick(s, "Vendor", "tkt_ven1");
  submitEntry(s);
  await s.until(() => calls.some((c) => c.tool === "books_entry_draft"), "the draft save");
  const draft = calls.find((c) => c.tool === "books_entry_draft");
  ok(draft.args.person === "tkt_ven1", `the picked vendor did not reach the draft: ${JSON.stringify(draft.args)}`);
  s.unmount();
});

test("a vendor replaced by free text sends payee without person", async () => {
  const s = await fresh({});
  await openEntry(s);
  fillExpense(s);
  pick(s, "Vendor", "tkt_ven1");
  combos(s, "Vendor")[0].props.onQueryChange({ query: "Walk-in" });
  submitEntry(s);
  await s.until(() => calls.some((c) => c.tool === "books_entry_draft"), "the draft save");
  const draft = calls.find((c) => c.tool === "books_entry_draft");
  ok(!draft.args.person && draft.args.payee === "Walk-in",
    `replacing vendor with free text did not clear person: ${JSON.stringify(draft.args)}`);
  s.unmount();
});

test("an unbalanced journal saves as draft on Submit and never posts", async () => {
  const s = await fresh({});
  s.pressContaining("Any balanced set of lines");
  await s.until(() => s.hasText("Submit"), "the journal page");
  // each journal line renders tablet, desktop and mobile copies in that
  // order, all bound to the line's signals — the first copy of each line is
  // occurrence 0 and 3
  pick(s, "Account", "tkt_cash1", 0);
  pick(s, "Account", "tkt_sup1", 3);
  inputsByLabel(s, "Debit")[0].props.onChange({ value: "100.00" });
  inputsByLabel(s, "Credit")[3].props.onChange({ value: "50.00" });
  submitEntry(s);
  await s.until(() => calls.some((c) => c.tool === "books_entry_draft"), "the unbalanced draft save");
  await settle(s);
  ok(calls.find((c) => c.tool === "books_entry_draft").args.lines?.length === 2, "the draft went without its lines");
  ok(!calls.some((c) => c.tool === "books_app_post_entry"), "an unbalanced journal posted");
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
