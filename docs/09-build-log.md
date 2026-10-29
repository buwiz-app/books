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

## Not yet built

M5, the rest of M6 (Forms, Certificates, Payroll pages), M7 to M9: see the tracker.
