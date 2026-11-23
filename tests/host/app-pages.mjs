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
  { ticket: "tkt_ven1", name: "Vera Vendor" },
  { ticket: "tkt_cus1", name: "Cory Customer" },
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
    groups: rows.length
      ? [{
          ticket: "tkt_cash1", number: "1010", name: "Cash on hand", type: "asset",
          count: rows.length, debit_shown: "440.00", credit_shown: "2,700.00", balance_shown: "−2,260.00",
          rows: rows.slice(0, 200),
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
          return Promise.resolve({ rows: PEOPLE });
        case "books_tags_list":
          return Promise.resolve({ rows: [] });
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
  await s.until(() => s.hasText("Save & Close"), "the new entry page");
};
const menuSave = (s, item = "Save") => {
  pressA11y(s, "More save options");
  s.press(item);
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
  ok(!s.hasText("Bob"), "money-out row still shown after picking Pay In");
  ok(s.hasText("Cory"), "money-in row missing after picking Pay In");
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

test("a second save edits the draft it wrote", async () => {
  const s = await fresh({});
  await openEntry(s);
  fillExpense(s);
  menuSave(s);
  await s.until(() => calls.filter((c) => c.tool === "books_entry_draft").length === 1, "the first draft save");
  await settle(s);
  menuSave(s);
  await s.until(() => calls.filter((c) => c.tool === "books_entry_draft").length === 2, "the second draft save");
  const drafts = calls.filter((c) => c.tool === "books_entry_draft");
  ok(!drafts[0].args.entry, `the first save already sent entry ${drafts[0].args.entry}`);
  ok(drafts[1].args.entry === "tkt_new1", `the second save did not edit tkt_new1: ${JSON.stringify(drafts[1].args)}`);
  ok(!calls.some((c) => c.tool === "books_app_post_entry"), "a post went out on Save");
  s.unmount();
});

test("post after save drafts the form's new values first", async () => {
  const s = await fresh({});
  await openEntry(s);
  fillExpense(s, "100.00");
  menuSave(s);
  await s.until(() => calls.some((c) => c.tool === "books_entry_draft"), "the draft save");
  await settle(s);
  s.type("0.00", "250.00");
  s.type("₱0.00", "250.00");
  menuSave(s, "Post");
  await s.until(() => calls.some((c) => c.tool === "books_app_post_entry"), "the post");
  const edit = calls.filter((c) => c.tool === "books_entry_draft").at(-1);
  ok(edit.args.entry === "tkt_new1" && edit.args.amount === "250.00",
    `no draft edit carried the new amount before posting: ${JSON.stringify(edit.args)}`);
  const post = calls.find((c) => c.tool === "books_app_post_entry");
  ok(post.args.entry === "tkt_new1" && !("amount" in post.args) && !("allocations" in post.args),
    `post carried form values the server ignores: ${JSON.stringify(post.args)}`);
  const ids = calls.filter((c) => ["books_entry_draft", "books_app_post_entry"].includes(c.tool)).map((c) => c.args.client_request_id);
  ok(ids.every(Boolean) && new Set(ids).size === ids.length, `request ids repeat: ${ids.join(", ")}`);
  s.unmount();
});

test("save & new starts the next draft fresh", async () => {
  const s = await fresh({});
  await openEntry(s);
  fillExpense(s);
  menuSave(s, "Save & New");
  await s.until(() => calls.some((c) => c.tool === "books_entry_draft"), "the first save");
  await settle(s);
  fillExpense(s, "60.00");
  menuSave(s);
  await s.until(() => calls.filter((c) => c.tool === "books_entry_draft").length === 2, "the second save");
  const second = calls.filter((c) => c.tool === "books_entry_draft")[1];
  ok(!second.args.entry, `Save & New left a remembered draft: ${JSON.stringify(second.args)}`);
  s.unmount();
});

test("a picked payee goes to the draft as person", async () => {
  const s = await fresh({});
  await openEntry(s);
  fillExpense(s);
  pick(s, "Vendor", "tkt_ven1");
  menuSave(s);
  await s.until(() => calls.some((c) => c.tool === "books_entry_draft"), "the draft save");
  const draft = calls.find((c) => c.tool === "books_entry_draft");
  ok(draft.args.person === "tkt_ven1", `the picked vendor did not reach the draft: ${JSON.stringify(draft.args)}`);
  s.unmount();
});

test("a vendor replaced by free text leaves the saved draft", async () => {
  const s = await fresh({});
  await openEntry(s);
  fillExpense(s);
  pick(s, "Vendor", "tkt_ven1");
  menuSave(s);
  await s.until(() => calls.some((c) => c.tool === "books_entry_draft"), "the draft save");
  await settle(s);
  combos(s, "Vendor")[0].props.onQueryChange({ query: "Walk-in" });
  menuSave(s);
  await s.until(() => calls.filter((c) => c.tool === "books_entry_draft").length === 2, "the draft edit");
  const edit = calls.filter((c) => c.tool === "books_entry_draft")[1];
  ok(edit.args.entry === "tkt_new1" && edit.args.person === "" && edit.args.payee === "Walk-in",
    `the edit did not take the vendor off: ${JSON.stringify(edit.args)}`);
  s.unmount();
});

test("an unbalanced journal saves as draft and cannot post", async () => {
  const s = await fresh({});
  s.pressContaining("Any balanced set of lines");
  await s.until(() => s.hasText("Save & Close"), "the journal page");
  // each journal line renders tablet, desktop and mobile copies in that
  // order, all bound to the line's signals — the first copy of each line is
  // occurrence 0 and 3
  pick(s, "Account", "tkt_cash1", 0);
  pick(s, "Account", "tkt_sup1", 3);
  inputsByLabel(s, "Debit")[0].props.onChange({ value: "100.00" });
  inputsByLabel(s, "Credit")[3].props.onChange({ value: "50.00" });
  menuSave(s);
  await s.until(() => calls.some((c) => c.tool === "books_entry_draft"), "the unbalanced draft save");
  await settle(s);
  ok(calls.find((c) => c.tool === "books_entry_draft").args.lines?.length === 2, "the draft went without its lines");
  menuSave(s, "Post");
  await s.until(() => s.hasText("Debits and credits must be equal before posting."), "the balance refusal");
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
