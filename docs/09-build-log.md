# 09 Build log: the mini app as built, milestone by milestone

Status: started 2026-10-29 with M1. The plan is [docs/plan/build-plan.html](plan/build-plan.html); the live tracker is the
published tracker page (its data is mirrored in [docs/plan/seed/](plan/seed/)). This file records what was built, the
design choices made while building (each a PROPOSAL unless an owner decision covers it), the gate results, and what is
blocked. Labels follow [01, How to read this file](01-architecture.md#how-to-read-this-file).

## Owner decisions applied (given with the build goal, 2026-10-29)

| Decision | Applied as |
|---|---|
| After a void, the original shows **Voided** (Q-1 reading 1) | `entries::void` marks the original `voided` and posts a mirrored reversal dated like the original (DD-15). Balances include both (they net to zero); measures skip both. |
| First import: **web Buwiz Books, then CSV** | M5 order. |
| **The owner** confirms account matches | M5: a suggested kind waits (`kind_confirmed = 0`) until the owner confirms it in the app. |
| A mid-year start uses **opening totals with proof** | M5: `opening_total` rows with a proof note; lineage shows them as opening figures. |
| **The assistant passes the BIR tax option** as `form_context` | M6/M7: `books_form_proposal` takes `form_context`; Books never decides or defaults it. |
| Inline cards show **amounts only when the user asked** | M8: answer cards may show amounts; routine and quiet cards show counts and status. |
| **TMP means templates** | M8: the five starter templates of docs/08 §3. |

## M1 Book and Ledger

### What exists

| Part | Where | Notes |
|---|---|---|
| Domain rules, no I/O | `crates/books-core` | `money` (i64 centavos, two-place text, display text with a real minus sign), `date` (Manila days, period keys), `catalog` (the 62 kinds of `data/categories.v1.json` with their default line facts, the closed vocabularies of `data/tax-attributes.v1.json`, the chart preset), `ledger` (the post rules ported from `poc/src/ledger.ts` `postLedger` and `poc/src/load.ts` `normalizeLine` / `checkCashBasis`; mirror for reversals), `privacy` (TIN-shape refusal for typed text, Q-4 option a). |
| Encrypted store | `crates/books-store` | SQLCipher through `rusqlite` `bundled-sqlcipher-vendored-openssl` (as BIR). Key file `BOOKS_DB_KEY_FILE` (default `<BOOKS_DATA_DIR>/books.key`), 32 random bytes as hex, mode 0600, folder 0700; a key file others can read is refused. Refuses a wrong key, a build without SQLCipher (RK-7: plain SQLite would ignore `PRAGMA key`), a different SQLCipher major version, a plain SQLite file and a newer schema. Schema `migrations/0001_init.sql`. |
| Chart preset | `data/presets/ph-small-business.v1.json` | One account per kind (62), numbered as the catalogue says. PROPOSAL numbers for the eight kinds the catalogue leaves unnumbered (`proposed_numbers` in the file). Four practical accounts: Cash Box 11100 and Credit Cards 22000 (neutral kinds), Uncategorized Income 40999 and Uncategorized Expense 69999 (kind `unsure`: their lines will hold every box they could feed, M5). |
| Tools | `crates/books-mini-app/src/{tools,service,views,writes}.rs` | Below. |
| The Books window | `apps/books/{kit,today,ledger,accounts,window}.tsx` | One app, `books` (full screen), served as the five files joined (`apps.rs`). Today, Ledger (account list, month, filters, entry drawer, new-entry sheet with Money in, Money out, Transfer and Journal) and Accounts. First run makes the book. The other nine pages show "arrives in a later step". |
| Inline status card | `apps/home.tsx` | Unchanged from P0 (`books_status`, counts and status only). |

### Design choices (PROPOSAL)

- **One window, not twelve apps.** `rox.open` pushes onto the full-screen stack and never replaces (spec 7.5), so a rail
  that opened a separate app per page would grow the stack with every click, and Rox caps a plugin at 16 apps. The
  window keeps the drawing's rail and holds the twelve pages; each page is its own source file. Inline cards (M8) are
  separate apps. The cost: one app's `permissions.tools` grants every tool its pages call (at most 64).
- **The server shapes page data.** Every amount reaches the app as display text (`80,000.00`, `+80,000.00`,
  `−1,350.00`) and every bar as a whole percent, so the app never parses or adds money. Typed amounts are normalized to
  two-place text by string operations before they are sent.
- **Tool names.** Reads `books_status`, `books_book_get`, `books_today`, `books_ledger_list`, `books_entry_get`,
  `books_entry_options`, `books_accounts_list`; the one assistant write `books_entry_draft` (D16); app-only writes
  `books_app_book_create`, `books_app_account_create`, `books_app_post_entry`, `books_app_void_entry`,
  `books_app_discard_entry`. App-only tools carry `_meta["rox/audience"] = ["app"]` and Books refuses them from any
  other origin (DD-20). A routine's turn writes nothing yet (M8 adds declared writes).
- **What the assistant sees** (D11 option a): totals, statuses, categories and tickets. Payees, memos, references and
  lines are for the app (`tests/privacy_outputs.rs`).
- **Idempotency and audit.** Every write takes `client_request_id`. A repeat returns the stored result; the same id
  with another request is refused (`request_id_reused`). The write runs in one transaction that commits only on
  success: a refused save-and-post leaves no draft behind. Every write and every refusal leaves an audit row (tool,
  origin, subject, outcome code; no memo, TIN or amount).
- **Tickets** live in the store and survive a restart. Titles are neutral: `Books`, `Entry 0187`, `Account · Bank
  Accounts`. `rox.uninstall` revokes every ticket and keeps the book (D10).
- **The memo is the "What".** The Ledger row reads `memo · payee` and the reference under it, as the drawing does.
- **`BOOKS_TODAY`** pins today's date for the host test and the demo book.

### Gates

fmt clean; clippy clean; cargo test 53; PoC 177; conformance 53/53; host test 17/17; privacy scan 0 findings (commit
`7e5f183`).

### Checked in a test Rox (M1-09)

Rox `016f63c` (a detached worktree, its own target folder), plain-Node broker on 8810 with a per-run trust file
(`books-test-01`) and `ROX_KEYSTORE=file:`, the app on gpui-agent port 17441, workspace 4. Screenshots:
[docs/shots/m1/](shots/m1/).

- `/books` shows "Open your books · Buwiz Books" (M1-10) and opens the Books window full screen; first run shows
  "Make your book" with Cash (default) selected, and an empty name is refused in place.
- Today, Ledger (the drawer with "Behind the scenes", Discard and Post) and Accounts render as drawn.
- Post and Void were real presses in the window: the draft became Posted; the void marked it Voided ("Cancelled by
  entry 0006") and posted "Void of Office supplies" dated like the original.
- **Limit (UNCONFIRMED in Rox docs, CONFIRMED by test):** a mini app's own surface is not in the gpui-agent snapshot
  tree, so an agent can press into it by point (`invoke press --arg x= --arg y=`) but cannot type into its inputs.
  Data that needs typing went in through signed host calls as the window (the path `rox.call` takes); actions were
  presses. The rox session put the gap on the owner's list (nsh fork change).
- **Found and fixed by the check:** rows keyed by ticket kept their first props after a change (a posted row still read
  Draft); rows are now keyed by their content. Today's month counted a void and its reversal as money out and in; it
  now reads the month's P&L, as the drawing does.
- The test Rox must be launched outside the shell sandbox; inside it the app gets no window and its agent port backs up.

## M2 People, Tags, Unpaid · M3 Reports

### What exists

| Part | Where | Notes |
|---|---|---|
| Schema 0002 | `crates/books-store/migrations/0002_people_tags_unpaid.sql` | `person` (vendor, customer, employee, owner, lender, government; no TIN column), `tag` (department, location), entry links, `unpaid`. |
| People, tags, unpaid | `crates/books-store/src/people.rs` | A cash book's unpaid document is a reminder; marking it paid posts the money that day (Dr bank, Cr the category, `sale_invoice` on a sales line). An accrual book posts the receivable or payable when the document is saved, and marking it paid clears it; cancelling voids it. |
| Page data and writes | `crates/books-mini-app/src/records.rs` | People (one page, seven kinds; banks and cards are the cash and card accounts), Tags, Unpaid. The model sees no person names. |
| Reports | `crates/books-mini-app/src/reports.rs` | P&L with the previous period, balance sheet (with net income this year), cash flow, trial balance, aging (accrual books only), CSV. Every P&L and balance-sheet row carries its account ticket and month, so a click opens the Ledger there. |
| Pages | `apps/books/{unpaid,people,tags,reports}.tsx` | As drawn. Today's Unpaid panel and Quick actions now read real data. |
| Demo book | `tools/demo/gen_demo.py` → `fixtures/demo/buwiz-trading-2026.json`; `books-mini-app seed <file>` | Synthetic, 110 tool calls as the app. The generator checks its own golden numbers before writing. |
| Golden test | `tests/golden_reports.rs` | Seeds the demo through the tools and asserts the drawing's numbers. |

### Design choices (PROPOSAL)

- **Tool names avoid invariant 7's words.** "Unpaid" and "payroll" contain `paid`/`pay`, which no Books tool name may
  hold, so the Unpaid tools are `books_owed_list`, `books_owed_options`, `books_app_owed_create`,
  `books_app_owed_settle` (mark paid) and `books_app_owed_cancel`; payroll tools will be `books_wage_*`.
- **Lines on a "not sure yet" account need review.** The drawing's two bank lines (electricity, bank fee) are posted
  to Uncategorized Expense (kind `unsure`): they are real bank movements, so balances and the P&L include them, the
  P&L marks the row "2 to review", the Ledger shows them as Needs review, and (M5) they hold every box they could feed.
- **The demo book honours the M3 exit numbers, not every number in the drawing.** The drawing's pages disagree with
  each other (its P&L shows September sales of 300,000.00 while its Forms page builds Q3 net sales of 290,000.00 from
  July and August invoices). The demo keeps the Q3 lineage, so October's P&L, the trial balance, the balance sheet,
  cash now and unpaid match the drawing exactly, and the September column does not.
- **Export CSV copies to the clipboard** (`rox.copy`, permission `copy`). Protocol v1 has no save-file permission
  (permissions are `tools`, `open`, `attach`, `pick_file`, `storage`, `copy`, `preview`); `preview` (print to PDF) is
  the other route, left for later.
- A person or tag named on a write (`person_kind` with `payee`, `department`, `location`) is linked, and made when new.
  A reversal keeps its original's person and tags.

### Golden numbers (tests/golden_reports.rs, all passing)

| Check | Value |
|---|---|
| P&L October 2026, net income | 251,151.00 |
| Trial balance at Oct 31, each side | 1,224,548.40 |
| Balance sheet at Oct 31 | assets 429,250.00 = liabilities 24,548.40 + equity 404,701.60 (net income this year 480,361.00) |
| Cash flow October, cash at Oct 31 | 417,350.00 |
| Q3 2026 sales and returns | 300,000.00 and −10,000.00 (net 290,000.00) |
| Today | cash 417,350.00; October +251,151.00; to collect 105,000.00, to pay −5,800.00, next due Nov 05 |
| Mark paid (Invoice 0143) | posts +60,000.00 on Oct 29; October net becomes 311,151.00 |
| Accrual book | the invoice posts a receivable; aging puts it in 31–60 days; paying clears it with no second income |

### Gates

fmt clean; clippy clean; cargo test 85 (with the M6 port's 48); PoC 177; host test 17/17; privacy scan 0 findings.
**Conformance 61/65 on rox `016f63c`:** the four failures were BRG-03 `rate_limited`. The runner sent one signed call
per granted tool on one open, back to back, and the broker allows 20 new calls per second per open
(`sessions.mjs:240`); the window grants 24 tools. The rox session paced BRG-03 (`2023fa2`, PR #13, not merged yet);
with that runner Books conforms 65/65, and the runner prints the per-run trust store (M1-11).

Checked in the test Rox: [docs/shots/m2/](shots/m2/), [docs/shots/m3/](shots/m3/).

## M4 Reconcile and months

### What exists

| Part | Where | Notes |
|---|---|---|
| Schema 0003 | `crates/books-store/migrations/0003_months_statements.sql` | `period` and `period_event` (close, reopen, reason), `statement` and `statement_line`, `category_default`, and an entry's link to the entry that gave its "not sure yet" lines a category. |
| Statements, pure | `crates/books-core/src/statement.rs` | CSV with a Date, a Description and either a signed Amount or Debit and Credit columns, and an optional running Balance; dates `YYYY-MM-DD`, `MM/DD/YYYY`, `Oct 03, 2026`, `03 Oct 2026`; amounts with commas, parentheses or a peso sign, two decimals at most, no float. Any run of nine or more digits in a description is masked before it is stored. Matching: same amount and day is a match; same amount within five days is a suggestion the user accepts; anything else is missing. |
| Months, statements, defaults, settings | `crates/books-store/src/months.rs` | The month gate every post and void passes (`writes::gate`). Closing refuses a month with drafts dated in it; reopening needs a reason; both are events. Finish needs every line matched and a 0.00 difference. A missing line is added as a posted entry on its date, on the kind its words name (interest, bank charges) or else on the "not sure yet" default. |
| Pages | `apps/books/{reconcile,settings}.tsx` | Reconcile: the grid and the match view. Settings: Book, Months, Defaults, Link & data. The Ledger drawer offers category chips for a line that needs review. |
| Test | `tests/months_reconcile.rs` | On the golden book. |

### Design choices (PROPOSAL)

- **A void in a closed month is refused**, with a pointer to reopen. Under the owner's void rule the original becomes
  Voided, which would change a closed month's totals; a closed entry is immutable (invariant 10).
- **Giving a "not sure yet" line a category posts a reclassifying entry** on the same date (Dr the category, Cr the
  not-sure account). The posted original never changes; both entries link to each other, and the original no longer
  needs review.
- **One statement per account and month.** Importing again replaces an unfinished one; a finished month refuses.
- **The file arrives as bytes.** `rox.pickFile` gives a `Uint8Array`, which the app sends as base64url; the plugin
  decodes it and refuses anything that is not UTF-8 or larger than 2 MB. `pick_file` grants `.csv` only.
- **A TIN shape** in typed text is now nine or more digits in a row, or 3-3-3 groups (with an optional branch); a file
  name such as `bank-4021-2026-10.csv` is not one.

### Exit (tests/months_reconcile.rs, passing)

October's synthetic statement (`fixtures/demo/bank-4021-2026-10.csv`) shows ten matches, the bank fee a day off as a
suggestion, and the 42.18 interest missing; statement 412,392.18, Books 412,350.00, difference 42.18, Finish refused.
After accepting the suggestion and adding the interest (to Interest Income), the difference is 0.00 and October is
matched. Closing October is refused while a draft is dated in it; once closed, a post and a void dated in October are
refused; reopening without a reason is refused and with one is recorded. The two review lines take Utilities and Bank
Fees; October's net becomes 251,193.18 (the interest), and the trial balance still balances. A basis change in a year
with entries is refused.

### Checked in the test Rox

[docs/shots/m4/](shots/m4/): the grid (October Draft after the import), the match view (`=`, `≈`, `?`), Accept and
"+ Add" by presses, difference 0.00 and Finish, October ✓ Matched, September closed from Settings › Months, October's
close refused with its reason, and Defaults. The statement went in through a signed host call: `rox.pickFile` opens the
native file dialog, which the agent cannot drive.

### Gates

fmt clean; clippy clean; cargo test 92; PoC 177; conformance 77/77 (runner from rox `2023fa2`); host test 17/17;
privacy scan 0 findings.

## M6 (part): the bridge in Rust

Ported by a subagent in a separate worktree (branch `m6-port`, commit `bf33be7`) and verified here before it reached
main: `crates/books-core/src/{canonical,rules,input,posting,measures,bridge,proposal,engine_error}.rs`, `tests/vectors.rs`.
All 17 PoC vectors reproduce byte for byte: each case's proposal equals the expected proposal in canonical form, the
input digest matches, and the whole vector file re-emitted with the Rust proposal equals its bytes. A planted one-cent
change to a vector's input fails the test. Notes from the port: `certificates_2316_received` is `planned` in the
catalogue and fails closed, as in the PoC; Rust sums in i128 and refuses a measure past i64.

## M5 Old books in

### What exists

- `books-core::import` (built by a subagent on branch `m5-import`, `b59af50`, verified here before merging): the web
  Buwiz Books export (v1 to v4) and CSV chart and journal files (QuickBooks, Xero, spreadsheets) read into one plan;
  Map 1 suggests a kind and a confidence for each account. The web export's `ph*` tables, bills, banks and voided
  transactions are skipped, each with a reason; no TIN field is read, and nine-digit runs are masked.
- `books-store::imports` (schema 4): an import lands in one transaction or not at all, into a book with nothing posted.
  An imported account starts unconfirmed with its suggestion; the owner confirms one, all suggestions at once, or picks
  mixed or not sure yet. Opening totals with proof (`opening_total`); the owner's OK of a return event (`return_ack`);
  the import log (`import_batch`).
- Tools: `books_app_upload_part` (a picked file in parts: one Rox call takes at most 256 KiB), `books_app_import`,
  `books_import_get`, `books_app_account_match`, `books_app_accounts_confirm`, `books_app_opening_set`,
  `books_app_return_ok`. The statement import takes parts too (`csv_upload`).
- Pages: Old books (Settings › Link & data, and a banner on Accounts while matches wait): bring in, **Match your
  accounts** (their account, Sure, Kind in Books, Feeds, Status; Confirm all suggested), Opening totals.

### Design choices (PROPOSAL)

- **Holds (plan 2.2).** A line on an account that is not sure yet, imported and not confirmed, or mixed without its
  own facts goes to the bridge with neutral facts, and holds every box a kind on that side could feed, plus the
  measures derived from them (net sales from gross sales). The box shows "Held: N lines on X are not matched yet".
- **Receivable and payable accounts** of an old chart are left out of a cash book when no line uses them (listed in
  the import's skipped list); when lines use them the import is refused with the way out (make the book accrual first).
- **Opening totals** enter the bridge as one opening entry on the range's last day, balanced against owner's equity,
  so lineage shows "Opening figure". The months they cover count as closed (they are fixed by their proof); a month
  before the book's first entry that the owner closed counts as closed too.
- **Old drafts** come in as drafts to review; voided transactions of the web export stay out.

### Exit (tests/forms_bridge.rs, passing)

The three sample charts (`fixtures/import/`: web Books, QuickBooks CSV, spreadsheet CSV) each give 2551Q Q3 2026
`txtATCAmt1` = 290,000.00 once the owner confirms the matches, OKs the September return and closes Q3; before the
matches are confirmed that box is held. A file sent in three parts imports the same. A mid-year book with an opening
gross-sales total of 250,000.00 (Apr to Jun, with proof) gives 2551Q Q2 = 250,000.00; no proof, or a range that
overlaps the book's own months, is refused. The assistant sees opening totals without their proof text.

### Checked in the test Rox

[docs/shots/m5/](shots/m5/): the QuickBooks sample imported; the Accounts banner (9 to match); Match your accounts with
suggestions; Confirm all suggested (0 to confirm, 2 per line, 2 not matched); Q3 closed and the return waiting; after
"OK the return", 2551Q Ready with 290,000.00; its lineage (four lines, the return negative).

## M6 Forms, Certificates, Payroll

### What exists

- `books-mini-app::forms`: the book as the PoC's input (`buwiz-poc-fixture/3`), through the ported loader and engine,
  so the app runs the code the 17 vectors prove. Tools `books_forms` (three cards), `books_form_proposal` (one form,
  with the BIR choices the assistant read as `form_context`), `books_app_form_lineage` (app only).
- `books-store::certificates` (schema 5): a certificate is a posted line with the certificate facts as printed; its
  status changes later as a `certificate_status` row, which the bridge reads (a void's reversal reads its original's).
  Tools `books_certificates_list`, `books_app_certificate_add`, `books_app_certificate_set`.
- Pay runs: `books_wages_list`, `books_app_wage_run_save` (a draft entry whose lines carry the payroll facts; the
  Ledger's Post posts it). Tool names avoid the forbidden verbs (no "pay", "file").
- Pages: Forms (cards, Books total → BIR box → value, lineage on a click, the return to OK, who fills the boxes, the BIR
  note), Payroll (runs, new pay run), Certificates (All, Received, Pending; status changes; add one).

### Design choices (PROPOSAL)

- **The 1701-MS tax option** is shown as "Read from the BIR draft"; the assistant passes it (props `tax_option`,
  `deduction`, or `form_context` on the tool). Without it the card says "Needs the BIR option". Switching it on the page
  only previews; Books never stores or sets it. A box for the other option is not a row; a box another choice leaves
  out stays as "not sent: deduction is OSD".
- **Preview while a period is open:** summed boxes show their total so far, marked "so far · preview", never sent.
- **Card labels** come from the first blocking reason: Preview (period open), Needs your OK (a return), No pay run,
  Needs the BIR option; "Ready · N held" counts holds and boxes that fail closed (1601-C item 16).
- **Adding a certificate** asks how the sale was recorded: net of the tax (the withheld part is added to sales) or in
  full as money in (it comes back out of the bank). The ATC is copied from the paper. The drawing's "+ Add from a PDF"
  is "+ Add a 2307": Books does not read PDFs; the assistant can.
- **Item 17's cap** is shown from the rule file ("the one tax rule in the bridge", as the drawing labels it).
- The demo's September pay run now carries `document_kind: payroll_register` so the Payroll page lists it.

### Exit (tests/forms_bridge.rs, passing)

A September pay run (basic 40,000.00, de minimis 2,000.00, employee SSS 2,000.00, employer 1,500.00, tax withheld as
recorded 2,808.40) saves as a draft (net 37,191.60), posts, and gives 1601-C items 3 Yes, 14 42,000.00, 18 2,000.00,
19 2,000.00, 25 2,808.40. Of three Q3 certificates only the received 2307 counts (2551Q item 15 = 3,000.00); when the
pending one's paper arrives it is 5,000.00; the 2306 never counts. A made-up ATC is refused. 1701-MS without the tax
option is blocked on it; with 8% it shows IV.19 and not IV.5.

### Checked in the test Rox

[docs/shots/m6/](shots/m6/): on the demo book, Forms with the return waiting, then 2551Q Ready (290,000.00 and
3,000.00, "1 received · 1 pending and 1 final (2306) left out"); 1601-C Ready · 1 held; 1701-MS graduated with OSD
previewing 1,200,000.00, 10,000.00, 8,900.00 and 3,000.00; Payroll (September, 1601-C Sep ready) and its sheet;
Certificates with what each counts toward, and its sheet.

### Gates

fmt clean; clippy clean; cargo test 109; PoC 177; conformance 92/92 (one of three runs hit the broker's 20 calls a
second in BRG-03, which paces at exactly that rate; told the rox session); host test 17/17; privacy scan 0 findings.

## M7 Hand-off to the BIR app

### What exists

- The v1 hand-off, model-mediated (docs/01 9.2, DD-11). Books never calls BIR:
  1. **Hand to BIR draft** (`books_app_handoff_create`, app only) records the ready proposal's input boxes and their
     lineage (schema 6, `handoff`); the app attaches it to the chat with `rox.attach("proposal", ticket)` after Rox's
     consent dialog ("Attach BIR hand-off · 2551Q Q3 2026 to this chat?").
  2. The assistant reads it with `books_handoff_get`: form, year, BIR period, `fill_args` and `source: "document"`, no
     TIN, no Books id. The scope `proposal` (attachable) resolves a hand-off ticket to its token and lists the current
     ones; its hint tells the assistant to fill once with source `document`, read back, validate, save, and never send.
  3. **BIR shows these values** (`books_app_handoff_confirm`, app only) records `fill_args_sha256` and the time (B5).
- **Out of date** (M7-05): a hand-off is current while the proposal's inputs hash is unchanged. Otherwise the card says
  so and why ("September reopened", "Sep 20 · Isla Café: 1 line changed"); `books_handoff_get` and the confirm refuse
  it; **Refresh** makes a new one and the old one is superseded.
- **Cross-checks** (M7-06, `books_cross_checks`): the four quarters' net sales against the year's (2551Q against
  1701-MS IV.19), sales by account against gross sales by facts, each month's pay runs against 1601-C items 14, 19 and
  25, received income-tax 2307s against 1701-MS V.3, and filed returns the assistant reads from BIR (`filed`) against
  Books now. Display only.
- **The BIR tax option** (M7-01) is read by the assistant and passed as `form_context` (owner decision); no BIR
  read-only tool is needed.

### Blocked, and who must act

- **No scratch BIR profile exists** (B2, D12). Slice B ran against a test harness instead
  (`tests/handoff_slice_b.rs`: a fake BIR draft that takes a fill only with source `document` and only on 2551Q input
  keys, and has no send). The live step, Q3 2551Q in a scratch BIR draft, waits for a documented scratch install from
  the BIR and Rox owners. Asked in the tracker (decision "scratchbir"). The live BIR database was never opened.
- **D3, the source label**: `document` is used; the Forms owner's confirmation is asked in the tracker ("d3label").

### Exit (tests/handoff_slice_b.rs, passing)

The demo book's Q3 2551Q is refused while the September return waits; after the OK, the hand-off carries
`{"txtATCAmt1": "290000.00", "frm2551Qv2018:txt15": "0.00"}` for 2551Q 2026 period 3 with source `document`; the
harness refuses source `user`, takes the fill once, and reads back the same strings; the confirm records a 64-hex
digest and the card shows In BIR draft. Reopening September makes it out of date ("September reopened"); a late sale
of 5,000.00 on Sep 20 makes it "Sep 20 · Isla Café: 1 line changed"; the old ticket is refused, the refreshed one
carries 295000.00. On the demo the cross-checks agree (1,190,000.00; 1,200,000.00; September's pay run; 8,900.00), and
a filed 2551Q Q1 of 295,000.00 shows as one difference.

### Checked in the test Rox

[docs/shots/m7/](shots/m7/): 2551Q Ready with the cross-checks; Rox's consent dialog on Hand to BIR draft; Handed over
with "BIR shows these values"; In BIR draft, Handed over Oct 29, 2 boxes; then, after September was reopened and a late
sale posted through signed host calls, Out of date with what changed and Refresh.

### Gates

fmt clean; clippy clean; cargo test 111; PoC 177; conformance 97 pass, 1 skipped (rox `9c106cc`: INV-04's attachable
scope check skips while there are no hand-offs; BRG-03's pacer fixed in `3214010`); host test 17/17; privacy scan 0
findings.

## M8 Chat: cards, scopes, routines

### What exists

- **Inline cards** (drawing: Books Inline Cards), five card apps, each granted only the tools its source calls:
  `answer` (360: cash now, this month, a person, unpaid), `report` (480: the P&L, this month against last), `action`
  (360: post a draft, categorize bank lines, mark paid, match the bank, close a month, a 2307 received), `pair` (480: a
  form for the BIR app: the return to OK, Ready, Hand to BIR draft, In BIR draft, Out of date), `quiet` (360: weekly
  tidy-up, month end, BIR hand-off; `books_nudge` only, so no amount can reach it). Each card reports its height with
  `rox.fit`.
- **Offering a card**: `books_show` (its result carries `_meta["rox/open"]`; Rox shows it on a click), `/books <page or
  card>`, and the five **templates** as commands (`/books-receipt`, `/books-tidy`, `/books-close`, `/books-quarter`,
  `/books-results`): protocol v1 has no template field, so each is a command whose resolve opens its card (the rox
  session's answer; a real field is on the Rox owner's list).
- **Six `@books:` scopes** (person, account, period, entry, form, tag), with the M7 `proposal`: text gives up to 20
  choices whose values are tickets, a ticket gives a token with a neutral title; a period or form token works wherever
  a period or form goes. Each carries a hint: read, draft at most, never post.
- **Three routines**: weekly tidy-up (writes `books_entry_draft` only), month-end nudge and BIR pair check (write
  nothing). Their query is `books_nudge`: empty text means no turn; each item has an id Rox remembers, so each thing
  fires once (a line, a month as `close-2026-10`, a stale hand-off with its inputs hash). `every_s` is at most a day,
  so the tidy-up finds work on Mondays only. Books' own check (P6): a routine's turn may write only what it declares,
  and Books lets a routine declare drafts only.

### Design choices (PROPOSAL)

- Amounts on answer and action cards (the user asked); quiet cards show counts and status only.
- `/books choices` (complete: true) list the pages; the rox session cannot confirm the composer asks for them, so a
  plain `/books` still opens Today and `/books ledger` a page.
- The BIR pair check reads Books only: the BIR app has no read-only tool shaped `{items: [...]}` for a routine's
  `reads` (rox session), so reading the BIR draft waits on the BIR owner.
- A card lays out inside a `Scroll` with `bounces={false} scrollable={false}`: inside it the card measures its natural
  height for `rox.fit` (BIR's card kit does the same), and it never scrolls or bounces. The owner caught the first
  version, a plain `Scroll`, pulling and bouncing like a list.

### Found on the way

- The Books window had no `pick_file` grant since M4: the manifest looked for `rox.pickFile(` while the sources split
  `rox` and `.pickFile(` across lines. Fixed (the method name alone is the signal); the statement and old-books
  pickers work in Rox now. The same fix covers `attach` and the cards' `open`.
- An nsh paint issue: with `shrink-0` on a card's measured view, text in later rows painted blank after the frame grew.
  Avoided by the pattern above; reported to the rox session.
- Rox shows "A newer version of this app is available" on every open card after a plugin update. That banner is Rox's
  (stale open, spec 12.2), not Books'; it showed constantly here because the test plugin was reinstalled after each
  change. The owner wants plugin updates in the plugin modal only; passed to the rox session for the Rox owner.

### Exit (tests/chat_cards.rs, passing)

Every app-only tool, called by the assistant, is refused with "needs the user's click"; the assistant can draft. The
weekly tidy-up's turn can draft and cannot post or close; the other routines cannot write; a routine's query only
reads; the manifest declares writes for the tidy-up only. On a Thursday the tidy-up nudge is empty and on request it
finds the two lines; the month-end nudge fires in the first week (`close-2026-10`); no nudge carries an amount and the
quiet app reaches `books_nudge` alone. `books_show` offers the cash card on `_meta`; the six scopes give choices and
neutral tokens; a form token feeds `books_form_proposal`; `/books ledger` opens the page, `/books cash` the card, the
templates their cards; the answer card cannot post.

### Checked in the test Rox

[docs/shots/m8/](shots/m8/): cash now, the P&L, a draft to post, bank lines with the likely category, mark paid, match
and close, the tidy-up, a pending 2307 and the 2551Q pair card (Out of date, Refresh for BIR), and the month-end and
hand-off quiet cards, each opened with `/books …` or a template. The test Rox's assistant also read the attached M7
hand-off with `books_handoff_get` and said it could not reach a BIR app.

Not checked in the test Rox: the person card (it needs the assistant's `books_show` with a person ticket) and the
month card after its sign fix. The owner asked to stop driving the GUI (it took focus on their monitor); that waits
until they say so. Whether a scroll over a card reaches the chat was answered from nsh's source instead: the app
surface's wheel handler stops every wheel over it (`crates/gpui/nsh-platform-gpui/src/events.rs:301-326` calls
`cx.stop_propagation()` whatever the content), so no inline card passes a scroll to the chat. That is nsh's and Rox's
to change (stop only when a scroll view under the pointer can move); passed to the rox session.

### Gates

fmt clean; clippy clean; cargo test 115; PoC 177; conformance 179 pass, 7 skipped (rox `9c106cc`); host test 17/17;
privacy scan 0 findings.

## M9 Release work

### What exists

- `tools/check-all.sh` (M9-01): fmt, clippy, the Rust tests with the 17 vectors, the PoC tests, Rox conformance, the
  Rox host test and the privacy scan, one line each, non-zero on any failure (`SKIP_ROX=1` never counts as green).
- **Buwiz sign-in** (M9-02, `books-mini-app::cloud`, docs/01 5.7, P-C4, P-C8), optional (D19) and off unless
  `BOOKS_CLOUD_URL` names the cloud: device sign-in as `buwiz-books`, scope `books` (`POST /oauth/device/code`, then
  polls of `POST /oauth/token`); the device and user codes stay in memory; each new refresh token is saved in the
  encrypted store before its access token is used; a refused refresh ends the sign-in. Tools `books_app_signin`
  (start, poll, out) and `books_app_tax_list`, app only.
- **The link** (M9-03, `books-store::link`, `books_app_tax_link`): the book links to one approved tax profile by its
  opaque UUID only, and only to one the cloud lists for the user at that moment. The list is read as `id`, `tin_masked`,
  `registered_name` and `permissions`; any other field is ignored, and a response with a `tin` field or a TIN-shaped
  value is refused whole. Settings › Link & data has the sign-in, the code, the list and Link / Unlink.
- The tool names avoid the forbidden verbs even inside words ("profiles" holds "file"), as the manifest test checks.

### Exit (tests/cloud_link.rs, passing, against a fake cloud)

Without a cloud address sign-in is refused plainly and Books works. The device flow waits, then signs in; neither the
device code nor a token appears in a result or the replay table; the refresh token is in the store. The list keeps
four fields; the link takes a listed id and refuses an unlisted one. In a new process the refresh token rotates
(`REF-1` sent, `REF-2` saved) before the list is read with the new access token. A `tin` field or a TIN-shaped value
refuses the list; an expired access token refreshes once; a refused refresh signs out and forgets the token.

### Not done here, by design

- A live sign-in: it needs the owner's Buwiz account and the cloud's address (U-20); the agent has neither.
- M9-04, the trust entry for `dev.goldcoders.books` (Rox owner), and M9-05, signing with `gc-2026-01` and installing
  in the owner's Rox (the owner). The release manifest already carries `publisher.key_id` `gc-2026-01`.

### Gates

`tools/check-all.sh`: fmt clean; clippy clean; cargo test 117; PoC 177; conformance 182 pass, 7 skipped (rox
`9c106cc`); host test 17/17; privacy scan 0 findings.

## Not yet built

M9-04 and M9-05 are the Rox owner's and the owner's: see the tracker.
