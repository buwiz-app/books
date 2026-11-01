# AGENTS.md

Guidance for coding agents in this repo. Before you change a rule, a schema or a claim, read `docs/00-findings.md` (evidence) and `docs/07-decisions-and-blockers.md` (decisions, blockers, design corrections).

## What this repo is

Buwiz Books, a Buwiz Bot mini app: plugin id `app.buwiz.books`, publisher `app.buwiz` (key `gc-2026-01`), protocol 1.0 (`buwiz-sdk` `docs/protocol-1.0.md`; 2.0 and 2.1 were its pre-release numbering, which a 1.0 Bot refuses). The Rust plugin is `crates/`, `apps/` and `tests/`; the research and design it was built from are `docs/`, `research/`, `data/` and `poc/`. Nothing here deploys or files anything. The TypeScript Books repo at `/Volumes/goldcoders/OSS/buwiz-books` is a reference only.

## Invariants

These cannot be inferred from the file tree.

1. **Books contains no compliance logic** (owner correction 2026-10-29; `docs/adr/0007-books-supplies-measures-forms-owns-compliance.md`). No rates, elections, thresholds (VAT, 8% eligibility, mixed income), applicability or NIL decisions, ATC selection, tax due, OSD or graduated tax, tax rounding, due dates, overdue status or penalties. Those belong to Buwiz Forms / the BIR mini app. Books records transactions, payroll facts and evidence and produces accounting measures with lineage. The bridge maps measures onto Forms **input** boxes only (`fill_class: input` with a fill policy). Where a box depends on a taxpayer choice, the bridge reads the choice from the Forms draft or profile (`form_context`) and never decides, infers or defaults it. The one labelled exception is the 1601-C item 17 non-taxable 13th-month split (`1601C.II.17`, "the one input-level tax rule in the bridge"); do not add another without an owner decision. Compliance reference material lives in `research/handoff-to-forms/`; Books code must not load it.
2. Money is integer centavos (`i64`; `i128` for intermediates). JSON carries two-decimal text such as `"200000.00"`. No floats: no `f64`, no `parseFloat`, no JS `number` for money. Measures are exact sums and are never rounded.
3. Never put a raw TIN, a BIR ticket value (`tkt_...`), a key or a memo into a link, log, title, app prop, store file, fixture or doc. Use synthetic values only.
4. Books stores no TIN in any phase. The cloud link is the opaque tax-profile `id` only. Never join on a TIN.
5. Read the cloud tax-profile response as `id`, `tin_masked`, `registered_name` and `permissions`. Ignore other fields, but refuse any response with a `tin` field or other TIN-shaped fields. Never rely on `filing.submit`. Sign in only as the `buwiz-books` client, whose `books` scope gets `profile.view` and no `tin` since buwiz-server PR #4 merged (`docs/07` B1, closed).
6. Values Books gives to BIR use fill source `document` (label pending owner decision D3). Never `user`, never `from_card`. BIR keeps provenance only for the session.
7. Books never sends, submits, files, queues or pays. Never call `bir_form_send`. Never call a tool whose name contains submit, queue, pay, paid, file or send.
8. The chart of accounts is not form meaning. Accounting facts live on the line attribute (income class, contra kind, person scope, payroll component, contribution side, certificate evidence); measures select lines by attribute; versioned mapping rules map measures to input keys. Never infer a form field from an account number or subtype. A line or profile never carries a compliance choice (tax type, ATC selection, treatment, regime, VAT status).
9. Mapping rule sets and the measures catalogue are versioned. A rule set is immutable once a proposal uses it; a change makes a new rule-set id. Recomputing makes a new proposal row. Nothing is overwritten. A change in the Forms context the bridge read makes the proposal stale.
10. Locks are per tax period. Closed entries are immutable. Reopening needs a reason and writes a `period_event`.
    Every business profile has an accounting basis, cash (the default) or accrual (`docs/adr/0008-cash-basis-by-default.md`). A cash-basis book posts income when money is received and expenses when they are paid, and refuses trade receivable and payable lines (`accrual_only` categories). Measures read the entry date on either basis; every proposal records the basis. Never add a per-line recognition override.
11. Every write takes a `client_request_id`. A repeat returns the stored result.
12. Only certificates with status `received` count in a certificate measure. One certificate feeds one form. Percentage-tax and income-tax certificates are separate measures; a 2306 (final withholding) never counts as a 2307.
13. Titles and cards carry no amounts, TINs or legal names by default.
14. Caution (INFERENCE): ₱3,000,000.00 is 300000000 centavos, a nine-digit run that looks like a TIN. The privacy scanner (see `docs/06-testing-plan.md`) must match by field context, not by digit count alone.
15. Slice B runs only on a scratch BIR profile. Set `BIR_DATABASE_PATH` to a scratch path and assert it before any call. Never open the live BIR database.
16. The first slice stops at a reviewable draft. No deploy, no filing, no real financial data.

## Writing rules for docs here

- Label facts: CONFIRMED, PARTIAL, MISSING, UNKNOWN or UNCONFIRMED. Mark reasoning INFERENCE and design choices PROPOSAL. Keep them separate.
- Cite a repo key, path and line, and the HEAD from the table below. Cite a report as `research/traces/<name>.md` with its section.
- `verify.md` governs conflicts between reports. `decisions.md` binds contracts with the Bot and cloud sessions.
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
- The Bot's signing and conformance tools are Node. Run them from a Bot checkout. Do not copy them here.
- Rust (the plugin, `crates/`): `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test` must pass clean. `tools/bot/conformance.sh` signs with a throwaway test key (`books-test-01`, never `gc-2026-01`) and runs the Bot's `buwiz-plugin-test`; it needs a Bot checkout at protocol 1.0 (`BUWIZ_BOT_DIR`) with its `server/node_modules` present. `tools/bot/host-test.sh` installs Books in a throwaway Bot broker from a worktree of that checkout at `BUWIZ_BOT_COMMIT`. Keep it on port 8810 or above (8787 is the owner's broker, 8797-8803 the rox session's tests), with `BUWIZ_KEYSTORE=file:` and never the login Keychain (without it the broker falls back to the Keychain). Never switch branches in the Bot checkout itself. One protocol version: 1.0, with no shims for older names. No floats: the crate denies float lints.

## Do not

- Import or edit BK-TS or any other source repo. Its defects are not ported.
- Commit, push or deploy unless the owner asks.
- Print a TIN, a ticket value, a key or a real amount into a log, a file or a reply.
- Use a cloud agent (`mcp`-scope) token from Books. Books has its own `buwiz-books` client and `books` scope; agent tokens belong to AI agents, not to Books (`docs/00-findings.md` F-50).
