# Buwiz Books: research, design and proof of concept

**Status: design, a proof of concept, and the mini app being built milestone by milestone (M1 done: an encrypted book, the ledger, Today, Ledger and Accounts). Not deployed. Synthetic data only.**

This repo holds the evidence and the design for Buwiz Books, a bookkeeping mini app for the Rox host (plugin id `dev.goldcoders.books`). It also holds a small proof of concept of the accounting measures and the bridge that maps them onto Forms input boxes. It does not file, send, submit or pay anything, and it contains no tax-compliance logic.

## The product in five lines

1. Books is a bookkeeping app that runs inside Rox on the user's Mac. Its ledger is encrypted on disk.
2. From posted entries, Books computes **accounting measures** (gross sales, returns, net sales, payroll components, certificates received, ...) per business profile per period. Every figure traces back to ledger lines. A thin bridge maps measures onto Buwiz Forms **input** boxes; the first is the 2551Q Schedule 1 row 1 amount.
3. For hand-off, the user attaches the proposal in chat. The assistant gives the values to the BIR mini app, which fills and saves a draft. Forms owns all compliance: elections, applicability, ATC, rates, tax due, thresholds, rounding, penalties and due dates.
4. Books never sends, submits, files, queues or pays. The user submits in BIR, outside this design.
5. Books stores no TIN. It links to a tax profile only by an opaque UUID. BIR keeps every TIN.

## Decisions

Owner decisions (binding; source: `research/traces/decisions.md`):

- Books is a separate service in its own repo. It is not a set of apps inside BIR.
- The ledger is local-first and encrypted. No cloud sync in the first version.
- Bookkeeping applies to every business. PH tax compliance is a separate, optional layer that a business links to.
- The TypeScript Books repo is a reference for domain logic only. Do not import it.
- No deploy, no change to real financial data, no filing. The first slice stops at a reviewable draft.
- (2026-10-29, scope correction) Books is bookkeeping and accounting and supplies measures; the bridge maps them onto Forms input boxes only and reads taxpayer choices from Forms; Buwiz Forms / the BIR mini app owns all compliance. See `docs/adr/0007-books-supplies-measures-forms-owns-compliance.md`.
- (2026-10-29, D4) Every book has an accounting basis, **cash by default** (income when received, expenses when paid), accrual available. Every proposal records the basis. See `docs/adr/0008-cash-basis-by-default.md`.
- (2026-10-29, D9) Books is signed at release with the existing Goldcoders publisher key `gc-2026-01`. Tests use a throwaway key under a test key id.

Design decisions (proposed; the full list and open items are in `docs/07-decisions-and-blockers.md`):

- Money is integer centavos. JSON carries two-decimal text. No floats. Measures are exact sums and are never rounded; Books holds no rates and no tax rounding (Forms').
- Line attributes are accounting and evidence facts. An account never implies a form field. Measures select lines; mapping rules map measures to Forms input keys only.
- Where a box depends on a taxpayer choice (1701-MS graduated → IV.5, 8% → IV.19), the bridge reads the choice from the Forms draft and never decides it.
- Mapping rule sets are versioned. A proposal records its rule set, the measures catalogue, the Forms context it read, the Forms schema hash and an inputs hash.
- One labelled exception: the 1601-C item 17 non-taxable 13th-month split (annual cap) stays in the bridge, open for the Forms owner.
- Hand-off is model-mediated. The BIR fill source is `document`, never `user` and never `from_card`.
- Tax-period locks, not one org-wide date. Closed entries are immutable.
- The cloud contract is read as `id`, `tin_masked`, `registered_name` and `permissions`. Books never relies on `filing.submit`. The cloud's full-TIN exposure (F-29, F-50) is fixed in buwiz-server PR #4, which was open and awaiting CI on 2026-10-29.

## Map

| Path | What it holds |
|---|---|
| `README.md`, `AGENTS.md`, `CLAUDE.md` | This file. Instructions for coding agents. |
| `docs/00-findings.md` | Evidence: what exists, what is partial, what is missing, and what could not be verified. Exact paths and HEADs. |
| `docs/01-architecture.md` | Components, boundaries, process model, identity, data model, tools and apps. |
| `docs/02-categories-and-attributes.md` | The category model, line attributes and measures. Chart of accounts is not form meaning. |
| `docs/03-crosswalk.md` | Verified field keys for 2551Q, 1601-C and 1701-MS, with who fills each box (Books measure, Forms, profile, user). |
| `docs/04-first-slice.md` | Slice A (Books alone, synthetic data: measures and 2551Q input boxes) and Slice B (hand-off to a scratch BIR profile, where Forms computes the tax), with acceptance criteria. |
| `docs/05-implementation-plan.md` | Phases P0 to P10 and their exit criteria. |
| `docs/06-testing-plan.md` | Golden, contract, privacy-scanner and Rox conformance tests. |
| `docs/07-decisions-and-blockers.md` | Owner decisions, blockers, discrepancies, design corrections, and the items handed to Forms (section 2.2). |
| `docs/plan/` | The build plan (`build-plan.html`, packed copy and published link inside), the live tracker page (`tracker.html`) and its starting data (`seed/`). Milestones M0 to M9 replace the P0 to P10 phases of `docs/05`. |
| `docs/08-mini-app-pages-and-feature-log.md` | The 12 mini-app pages, inline cards and guided chat as drawn for review, and the log of web features that do not come along (Rox has them, BIR owns them, later, dropped). Sources of the drawings: `research/drawings/`. |
| `docs/adr/` | Architecture decision records. ADR 0007 sets the Books / bridge / Forms boundary. |
| `data/` | The measures catalogue (`measures.v1.json`), the mapping rule sets (`rules/`, r2: measure to input key), the category and line-attribute catalogues, the coverage matrix (which measures feed which input boxes) and the crosswalk files. Each crosswalk file records its source commit and digest. No rates or compliance rules. |
| `research/traces/` | Raw investigation reports: `rox`, `bir-shell`, `books`, `forms`, `identity`, `sheets`, `bir`, `verify`, `decisions` (with the 2026-10-29 scope-correction addendum). |
| `research/handoff-to-forms/` | Compliance material handed to the Forms owner: the former rate table, the removed gates and eligibility notes, the r1 rule sets, and `compliance-notes.md` with evidence. Books code does not load it. |
| `research/inputs/` | The two research workbooks. |
| `research/workbook-form-links.md` | The workbook's category-to-form-line rows, kept as evidence. Not a Books rule. |
| `research/corrected/`, `research/spreadsheet-corrections.md` | Corrected workbook figures, labels and formulas. |
| `poc/` | Bun and TypeScript proof of concept: ledger → measures → mapping → fill args with lineage. |
| `fixtures/` | Synthetic fixtures for the PoC. Never real data. |
| `tools/` | Helper scripts used in the research. `tools/rox/conformance.sh` runs Rox's conformance tool against the plugin with a throwaway test key. |
| `Cargo.toml`, `crates/`, `apps/`, `tests/` | The Rust Rox plugin. `crates/books-core`: domain rules with no I/O (centavos, dates, the kind catalogue and chart preset, the ledger's post rules). `crates/books-store`: the SQLCipher store, its 0600 key file and migrations. `crates/books-mini-app`: the stdio MCP server, the protocol v1 reserved tools, the manifest generator and the `books_*` tools. `apps/books/`: the Books window (one app, a file per page); `apps/home.tsx`: the inline status card. `tests/host/` is the Node harness for the host test. |
| `docs/09-build-log.md` | The mini app as built, milestone by milestone: design choices made while building, gate results, what is blocked. |

## Run the gates

`tools/check-all.sh` runs every gate below in order and prints one line each (`ROX_DIR` names the Rox checkout or
worktree, `BOOKS_HOST_PORT` the host test's port; it exits non-zero on any failure). One by one:

```sh
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
(cd poc && bun test)
tools/privacy-scan.sh      # the scanner's self-test, the repo scan and what the assistant can read
tools/rox/conformance.sh   # needs a Rox checkout or worktree (ROX_DIR) with server/node_modules
tools/rox/host-test.sh     # installs Books in a throwaway Rox broker (port 8810+, file keystore) and calls it
```

The plugin keeps its store in `BOOKS_DATA_DIR` (default `~/Library/Application Support/dev.goldcoders.books`) with
the key file `BOOKS_DB_KEY_FILE` (default `<dir>/books.key`, mode 0600). Tests and the host test always point both
at a temp folder.

Buwiz sign-in is optional (Books works offline) and runs only when `BOOKS_CLOUD_URL` names the cloud (`https://…`):
device sign-in as `buwiz-books`, scope `books`; the rotating refresh token lives in the encrypted store; the book
links to a tax profile by its id only.

## Run the proof of concept

```sh
cd poc
bun test
```

The proof of concept in `poc/` is TypeScript and runs on Bun. Use `bun`, not npm, npx or node, there. The planned Books product is Rust (`docs/adr/0005-stack.md`), and the Bun rule from the TypeScript Books repo does not govern it (`docs/07` DD-24). `poc/package.json` declares no dependencies, so there is no install step. The PoC makes no calls to Rox, BIR or the cloud.

## Observed and recommended

- **Observed:** facts, each with a status label and a citation, in `docs/00-findings.md`.
- **Recommended (proposal):** the architecture, the first slice and the plan in `docs/01-architecture.md`, `docs/04-first-slice.md` and `docs/05-implementation-plan.md`.

## Status

- Research: nine investigation reports are in `research/traces/`. Read-only work dated 2026-10-29.
- Live checks: none. No live Rox, BIR or cloud call was made, and the live BIR database was not opened.
- Tests: the source repos' test suites were not run. Last PoC run (2026-10-29, after ADR 0008, `cd poc && bun test`): 177 pass, 0 fail, 2378 expect() calls, 11 files. The verbatim output is in `poc/README.md`.
- Git: the first commit holds this research, design and PoC. Pushing to `github.com/buwiz-corp/buwiz-books` is the owner's call.
- Owner decisions D4 (cash basis by default) and D9 (reuse `gc-2026-01`) are settled.
- Open questions that block parts of the build: the BIR source label (D3), the 1701-MS election locks in Forms (B7), the tax review of the open mappings (2551Q row 1 net or gross sales, former D20), the owner of the 1601-C item 17 cap (DD-29), and the cloud repo of record (D13). Cloud blockers B1 and B11 are closed: buwiz-server PR #4 merged on 2026-10-29. D5, D6, D7 and D14 are handed to Forms. See `docs/07-decisions-and-blockers.md`.
