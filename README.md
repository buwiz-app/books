# Buwiz Books: research, design and proof of concept

**Status: research and design only. Not production code. Not deployed.**

This repo holds the evidence and the design for Buwiz Books, a bookkeeping mini app for the Rox host (plugin id `dev.goldcoders.books`). It also holds a small proof of concept for the pure tax rules. It does not file, send, submit or pay anything.

## The product in five lines

1. Books is a bookkeeping app that runs inside Rox on the user's Mac. Its ledger is encrypted on disk.
2. From posted entries, Books computes tax proposals. The first is the 2551Q quarterly percentage tax. Every figure traces back to ledger lines.
3. For hand-off, the user attaches the proposal in chat. The assistant gives the values to the BIR mini app, which fills and saves a draft.
4. Books never sends, submits, files, queues or pays. The user submits in BIR, outside this design.
5. Books stores no TIN. It links to a tax profile only by an opaque UUID. BIR keeps every TIN.

## Decisions

Owner decisions (binding; source: `research/traces/decisions.md`):

- Books is a separate service in its own repo. It is not a set of apps inside BIR.
- The ledger is local-first and encrypted. No cloud sync in the first version.
- Bookkeeping applies to every business. PH tax compliance is a separate, optional layer that a business links to.
- The TypeScript Books repo is a reference for domain logic only. Do not import it.
- No deploy, no change to real financial data, no filing. The first slice stops at a reviewable draft.

Design decisions (proposed; the full list and open items are in `docs/07-decisions-and-blockers.md`):

- Money is integer centavos. Rates are basis points. JSON carries two-decimal text. No floats.
- Rounding is half away from zero, once, at output. ₱1.50 at 3% gives ₱0.05.
- Tax meaning is an explicit line attribute. An account never implies a form field.
- Rules and rate rows are versioned and effective-dated. A proposal records its rule set, the Forms schema hash and an inputs hash.
- Hand-off is model-mediated. The BIR fill source is `document`, never `user` and never `from_card`.
- Tax-period locks, not one org-wide date. Closed entries are immutable.
- The cloud contract is read as `id`, `display` and `permissions`. Books never relies on `filing.submit`. The live cloud code still returns `tin` and `tin_masked` (`docs/00-findings.md` F-29).

## Map

| Path | What it holds |
|---|---|
| `README.md`, `AGENTS.md`, `CLAUDE.md` | This file. Instructions for coding agents. |
| `docs/00-findings.md` | Evidence: what exists, what is partial, what is missing, and what could not be verified. Exact paths and HEADs. |
| `docs/01-architecture.md` | Components, boundaries, process model, identity, data model, tools and apps. |
| `docs/02-categories-and-attributes.md` | The category model and tax attributes. Chart of accounts is not tax meaning. |
| `docs/03-crosswalk.md` | Verified field keys for 2551Q, 1601-C and 1701-MS. |
| `docs/04-first-slice.md` | Slice A (Books alone, synthetic data) and Slice B (hand-off to a scratch BIR profile), with acceptance criteria. |
| `docs/05-implementation-plan.md` | Phases P0 to P10 and their exit criteria. |
| `docs/06-testing-plan.md` | Golden, contract, privacy-scanner and Rox conformance tests. |
| `docs/07-decisions-and-blockers.md` | Owner decisions, blockers, discrepancies and design corrections. |
| `docs/adr/` | Architecture decision records. |
| `data/` | Rate rows, rule sets, the category and tax-attribute catalogs, and the crosswalk files. Each crosswalk file records its source commit and digest. Source trail: `docs/00-findings.md` section 1 and `docs/03-crosswalk.md` section 1. |
| `research/traces/` | Raw investigation reports: `rox`, `bir-shell`, `books`, `forms`, `identity`, `sheets`, `bir`, `verify`, `decisions`. |
| `research/inputs/` | The two research workbooks. |
| `research/corrected/`, `research/spreadsheet-corrections.md` | Corrected workbook figures, labels and formulas. |
| `poc/` | Bun and TypeScript proof of concept for the pure rules. |
| `fixtures/` | Synthetic fixtures for the PoC. Never real data. |
| `tools/` | Helper scripts used in the research. |

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
- Tests: the source repos' test suites were not run. Last PoC run (2026-10-29, `cd poc && bun test`): 187 pass, 0 fail, 1341 expect() calls, 10 files. The verbatim output is in `poc/README.md`.
- Git: the first commit holds this research, design and PoC. Pushing to `github.com/buwiz-corp/buwiz-books` is the owner's call.
- Open questions that block parts of the build: the 8% election and 2551Q (D5), the 1% PT extension (D6), the BIR source label (D3), and the cloud repo of record (D13), and whether cloud agent tokens may read full TINs (Q-9, F-50). See `docs/07-decisions-and-blockers.md`.
