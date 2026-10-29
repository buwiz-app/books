# AGENTS.md

Guidance for coding agents in this repo. Before you change a rule, a schema or a claim, read `docs/00-findings.md` (evidence) and `docs/07-decisions-and-blockers.md` (decisions, blockers, design corrections).

## What this repo is

Research, design and proof of concept for Buwiz Books, a Rox mini app (`dev.goldcoders.books`). Not production code. Nothing here deploys or files anything. The TypeScript Books repo at `/Volumes/goldcoders/OSS/buwiz-books` is a reference only.

## Invariants

These cannot be inferred from the file tree.

1. Money is integer centavos (`i64`; `i128` for intermediates). Rates are basis points (300 = 3.00%). JSON carries two-decimal text such as `"200000.00"`. No floats: no `f64`, no `parseFloat`, no JS `number` for money.
2. Rounding is half away from zero, once, at output. Banker's rounding is wrong here. Golden cases: ₱1.50 at 3% is ₱0.05; ₱1.49 is ₱0.04; −₱1.50 is −₱0.05.
3. Never put a raw TIN, a BIR ticket value (`tkt_...`), a key or a memo into a link, log, title, Rox prop, store file, fixture or doc. Use synthetic values only.
4. Books stores no TIN in any phase. The cloud link is the opaque tax-profile `id` only. Never join on a TIN.
5. Read the cloud tax-profile response as `id`, `display` and `permissions`. Refuse any response with TIN-shaped fields. Never rely on `filing.submit`. The live cloud code (9f5ee16) returns `tin`, `tin_masked` and no `display` (`docs/00-findings.md` F-29), so Books must refuse that shape until the cloud owner aligns it (`docs/07` B1).
6. Values Books gives to BIR use fill source `document` (label pending owner decision D3). Never `user`, never `from_card`. BIR keeps provenance only for the session.
7. Books never sends, submits, files, queues or pays. Never call `bir_form_send`. Never call a tool whose name contains submit, queue, pay, paid, file or send.
8. The chart of accounts is not tax meaning. Tax meaning lives on the line attribute (treatment, ATC, person scope, component, contribution side) and in versioned mapping rules. Never infer a form field from an account number or subtype.
9. Rules and rate rows are versioned and effective-dated. Effective windows apply to the filing period. A rule set is immutable once a proposal uses it; a change makes a new rule-set id. Recomputing makes a new proposal row. Nothing is overwritten.
10. Locks are per tax period. Closed entries are immutable. Reopening needs a reason and writes a `period_event`.
11. Every write takes a `client_request_id`. A repeat returns the stored result.
12. Only certificates with status `received` credit withholding. One certificate feeds one form.
13. Titles and cards carry no amounts, TINs or legal names by default.
14. Caution (INFERENCE): ₱3,000,000.00 is 300000000 centavos, a nine-digit run that looks like a TIN. The privacy scanner (see `docs/06-testing-plan.md`) must match by field context, not by digit count alone.
15. Slice B runs only on a scratch BIR profile. Set `BIR_DATABASE_PATH` to a scratch path and assert it before any call. Never open the live BIR database.
16. The first slice stops at a reviewable draft. No deploy, no filing, no real financial data.

## Writing rules for docs here

- Label facts: CONFIRMED, PARTIAL, MISSING, UNKNOWN or UNCONFIRMED. Mark reasoning INFERENCE and design choices PROPOSAL. Keep them separate.
- Cite a repo key, path and line, and the HEAD from the table below. Cite a report as `research/traces/<name>.md` with its section.
- `verify.md` governs conflicts between reports. `decisions.md` binds contracts with the Rox and cloud sessions.
- Plain sentences. Tables for lists of facts.

## Source repos (read-only from here)

Do not edit these. Re-check HEAD and state before you rely on a citation. HEADs are as checked on 2026-10-29.

| Key | Path | HEAD |
|---|---|---|
| BK-TS | `/Volumes/goldcoders/OSS/buwiz-books` | `1662ca3` (main) |
| ROX | `/Volumes/goldcoders/Projects/rox` | `8453d53` (perf-parity) |
| BIR (live; canonical for BIR changes) | `/Volumes/goldcoders/reverse-engineer-ebir-forms/bir` | `29ca915b` (rox-mini-app-round2) |
| FORMS (local, stale) | `/Volumes/goldcoders/OSS/buwiz-forms` | `301f9a0f` (main); pin `63267ca` |
| BOSS | `/Volumes/goldcoders/buwiz-oss` | `0f34106` (main; dirty) |
| SPIN | `/Volumes/goldcoders/buwiz-server` | `417ec3a` (main) |
| CLOUD | `/Volumes/goldcoders/buwiz-cloud` | `9f5ee16` (main, checked 2026-10-29 after 14:34 +0800, after `085c36d`); the repo moves, so re-check |

- Refresh: `git -C <path> rev-parse --short HEAD` and `git -C <path> status --short`.
- Read a pinned file with exact bytes and no fetch:
  `GIT_NO_LAZY_FETCH=1 git -C /Volumes/goldcoders/OSS/buwiz-forms cat-file blob 63267ca:crates/bir-core/data/agent-forms/2551q-v2018.json | shasum -a 256`
- Use `cat-file blob`, not a text dump. A trailing newline changes the digest.
- The live BIR tree is canonical for BIR changes. The Forms pin is for schema data only.

## Tooling

- Use `bun` for any TypeScript in this repo (`poc/`). Do not use npm, npx or node here.
- Rox's signing and conformance tools are Node. Run them from the Rox repo. Do not copy them here.

## Do not

- Import or edit BK-TS or any other source repo. Its defects are not ported.
- Commit, push or deploy unless the owner asks.
- Print a TIN, a ticket value, a key or a real amount into a log, a file or a reply.
- Use a cloud agent (`mcp`-scope) token from Books. Such a token can read full TINs through `GET /api/tax-profiles` and the MCP `list_tax_profiles` tool (`docs/00-findings.md` F-50).
