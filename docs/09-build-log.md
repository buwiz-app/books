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

Recorded in the tracker and in the M1 commit message.

## Not yet built

M2 to M9: see the tracker.
