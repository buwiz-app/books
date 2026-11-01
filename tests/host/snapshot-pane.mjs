// The reconcile snapshot pane, mounted headless on nsh-standin.mjs with a stand-in bridge (synthetic data only).
// Usage: bun tests/host/snapshot-pane.mjs <dir holding node_modules/esbuild> (e.g. a Bot worktree's server/).
// Checks: page tabs work after a row is selected; selecting a row switches the pane to its page; "See the page"
// and "Print reconciliation" stay reachable with a snapshot but no drawable rows; Print alone for a CSV statement.
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";

const REPO = new URL("../../", import.meta.url).pathname;
const esbuild = createRequire(`${process.argv[2] || process.cwd()}/`)("esbuild");
const { mountApp, textOf } = await import(new URL("./nsh-standin.mjs", import.meta.url).href);

// The window source exactly as apps.rs joins it: the include_str! files of BOOKS, in order.
const appsRs = readFileSync(`${REPO}crates/books-mini-app/src/apps.rs`, "utf8");
const books = appsRs.slice(appsRs.indexOf("const BOOKS"), appsRs.indexOf(");", appsRs.indexOf("const BOOKS")));
const files = [...books.matchAll(/include_str!\("\.\.\/\.\.\/\.\.\/(apps\/books\/[a-z_]+\.tsx)"\)/g)].map((m) => m[1]);
if (!files.includes("apps/books/statement_snapshot.tsx")) throw new Error("apps.rs does not join statement_snapshot.tsx");
const source = files.map((f) => readFileSync(`${REPO}${f}`, "utf8")).join("\n");

const box = (left, top) => ({ left, top, width: "40%", height: "4%" });
const line = (no, description, page, b) => ({ no, ticket: `stl-${no}`, date_short: "Oct 2", description, amount_shown: "1.00", match: "x", page, box: b });

function bridge(statement, previews) {
  const answers = {
    books_status: { status: "ready" },
    books_reconcile_grid: { months: [{ month: "2026-10", label: "Oct" }], rows: [{ ticket: "acct-synthetic", name: "Synthetic bank", cells: [{ state: "todo", month: "2026-10", label: "Oct" }] }] },
    books_statement_get: { month: { short: "Oct" }, account: { name: "Synthetic bank" }, statement },
  };
  return {
    props: { page: "reconcile" },
    call: async (tool) => answers[tool] ?? {},
    preview: async (req) => previews.push(req),
  };
}

async function open(statement) {
  const previews = [];
  const screen = mountApp(source, { esbuild, bridge: bridge(statement, previews) });
  await screen.until(() => screen.pressable("Oct ›"), "the grid's Oct cell");
  screen.press("Oct ›");
  await screen.until(() => screen.hasText("Bank statement"), "the match view");
  return { screen, previews };
}

const boxes = (screen) => screen.nodes().filter((n) => n.host === "View" && n.props.style && n.props.style.left);
const lefts = (screen) => boxes(screen).map((n) => n.props.style.left);
const rowOf = (screen, text) => screen.nodes().filter((n) => typeof n.props.onPress === "function" && String(n.props.class || "").includes("border-b") && textOf(n).includes(text)).pop();
function expect(ok, what) {
  if (!ok) throw new Error(what);
  console.log(`ok - ${what}`);
}

// Two pages with one drawable line each.
{
  const { screen, previews } = await open({ snapshot: "evd-synthetic", rows: [line(1, "Coffee shop", 1, box("10%", "20%")), line(2, "Hardware store", 2, box("30%", "50%"))] });
  expect(lefts(screen).join() === "10%", "the pane opens on page 1");
  boxes(screen)[0].props.onPress();
  screen.press("Page 2");
  expect(lefts(screen).join() === "30%", "a page tab works after a box is selected");
  rowOf(screen, "Coffee shop").props.onPress();
  expect(lefts(screen).join() === "10%", "selecting a row on page 1 switches the pane to page 1");
  screen.press("Page 2");
  expect(lefts(screen).join() === "30%", "a page tab works after a row is selected");
  rowOf(screen, "Hardware store").props.onPress();
  screen.press("Page 1");
  rowOf(screen, "Hardware store").props.onPress();
  expect(lefts(screen).join() === "30%", "selecting the same row again returns to its page");
  screen.press("See the page");
  expect(previews.at(-1)?.args?.kind === "page" && previews.at(-1)?.args?.page === 2 && previews.at(-1)?.args?.line === 2, "See the page previews the shown page and the selected line");
  screen.unmount();
}

// A snapshot ticket but no line with a page and a box.
{
  const { screen, previews } = await open({ snapshot: "evd-synthetic", rows: [line(1, "Coffee shop", 1, null)] });
  expect(boxes(screen).length === 0, "nothing is drawn without a box");
  expect(Boolean(screen.pressable("See the page")), "See the page stays reachable with a snapshot");
  screen.press("Print reconciliation");
  expect(previews.at(-1)?.args?.kind === "report", "Print reconciliation stays reachable with a snapshot");
  screen.unmount();
}

// A CSV statement: no snapshot, no pages.
{
  const { screen } = await open({ rows: [line(1, "Coffee shop", null, null)] });
  expect(!screen.pressable("See the page"), "no See the page without a snapshot");
  expect(screen.nodes().filter((n) => typeof n.props.onPress === "function" && textOf(n).trim() === "Print reconciliation").length === 1, "one Print reconciliation for a CSV statement");
  screen.unmount();
}
