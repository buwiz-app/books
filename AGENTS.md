# AGENTS.md

Buwiz Books is a Buwiz Bot mini app: plugin id `app.buwiz.books`, publisher `app.buwiz`, Buwiz mini app protocol
1.0 ([buwiz-sdk `docs/protocol-1.0.md`](https://github.com/buwiz-app/buwiz-sdk/blob/main/docs/protocol-1.0.md)).
README.md covers the build, gates and releases.

## Invariants

These cannot be inferred from the code.

1. **Books contains no compliance logic** ([ADR 0007](docs/adr/0007-books-supplies-measures-forms-owns-compliance.md)).
   This means no rates, elections, thresholds, applicability or NIL decisions, ATC selection, tax due, OSD or graduated
   tax, tax rounding, due dates, overdue status or penalties: those belong to Buwiz Forms. Books records transactions,
   payroll facts and evidence, and produces accounting measures with lineage. The bridge maps measures onto Forms
   **input** boxes only. It reads a taxpayer choice from the Forms context, and never decides, infers or defaults it.
   The one labelled exception is the 1601-C item 17 non-taxable 13th-month split (`1601C.II.17`). Do not add another
   without an owner decision.
2. **Money is integer centavos.** Use `i64`, with `i128` for intermediates. JSON carries two-decimal text such as
   `"200000.00"`. No floats anywhere in a money path (the crates deny float lints; no `parseFloat` or JS `number` in
   the apps). Measures are exact sums and are never rounded.
3. **Privacy.**
   - Never put a raw TIN, a ticket value (`tkt_...`), a key, a token or a memo into a link, log, title, app prop,
     store file, fixture, test or doc. Use synthetic values only.
   - Books stores no TIN. The cloud link is the opaque tax-profile `id`. Read the tax-profile response as `id`,
     `tin_masked`, `registered_name` and `permissions`, and refuse a response that carries a `tin` or other
     TIN-shaped field.
   - Titles and cards carry no amounts, TINs or legal names by default. The Bot's approval card is the exception: it
     is the person's to read, stays out of logs and the model, and has to show the real amount and accounts.
   - ₱3,000,000.00 is `300000000` centavos, a nine-digit run that looks like a TIN. The privacy scan matches by field
     context, not by digit count alone.
4. **Nothing posts or closes unless a person decided.** `Reach::App` tools that write (post, void, close and reopen
   a month, confirm, hand off, import, sign in, and the other changes a person makes) run from the person's own click
   in the Books app: `origin: app` with no `press`. The assistant saves drafts: directly, or by pressing a button on a
   Books screen, which reaches Books as `origin: agent` with the Bot's `press` record, on a tool whose
   `_meta["buwiz/audience"]` names `agent`. Only the draft tools do, and a draft counts in no total until a person
   posts it. The one other way is the owner's D1: the assistant may press **Post** (`books_app_post_entry`, or
   `books_app_batch` with `action: post`) and **Close month** (`books_app_month_close`) only after a person allows
   that one press. The Bot holds the call unsent, shows the person what it will do from `books_app_press_card`, and
   only their own press of Allow lets it go. It arrives as `origin: agent` with a `press` record of `by: ai_approved`
   whose `args_hash` is this call's arguments (`press.rs`), and the audit row names the AI and the person. A tool opts
   in through `press::APPROVED` and nowhere else. D1 names no other tool, so void, reopen, return confirm, hand-off and
   discard stay the person's own click, and a held press is never a filing (invariant 5). While the assistant drives a
   screen its reads still run for that screen, as `origin: app` with the `press` record, and their answers stay in the
   app. So `write: false` must be strictly true: a tool that writes a file, reaches the cloud or changes any state is
   a write. The one thing a read may add is a ticket for something that already exists: a handle, not a record.
   Resolves and routines only read; a routine writes only the drafts it declares.
5. **Books never sends, submits, files, queues or pays.** Never call an eBIRForms queue or send tool, or any tool
   whose name contains submit, queue, pay, paid, file or send. Values Books gives to BIR use fill source `document`.
6. **The chart of accounts is not form meaning** ([ADR 0006](docs/adr/0006-tax-meaning-on-lines-not-accounts.md)).
   Accounting facts live on line attributes, and measures select lines by attribute. Never infer a form field from an
   account number or subtype, and never put a compliance choice on a line or profile.
7. **Rule sets and the measures catalogue in `data/` are versioned and immutable** once a proposal uses them. A
   change makes a new rule-set id. A change in the Forms context the bridge read makes a proposal stale.
8. **Every business profile has an accounting basis**, cash (the default) or accrual
   ([ADR 0008](docs/adr/0008-cash-basis-by-default.md)). A cash book refuses trade receivable and payable lines.
9. **Locks are per tax period.** Closed entries are immutable. Reopening needs a reason and writes a `period_event`.
10. **Every write takes a `client_request_id`**, and a repeat returns the stored result.
11. **Certificates:** only status `received` counts in a certificate measure. Percentage-tax and income-tax
    certificates are separate measures, and a 2306 never counts as a 2307.
12. **Cloud:** sign in only as the `buwiz-books` client (scope `books`). Never use an agent (`mcp`-scope) token.

## Tests and tools

- `tools/check-all.sh` must pass before a commit. It runs fmt, clippy with `-D warnings`, `cargo test`, the Bot
  gates and the privacy scan; README.md says what each needs.
- **Test keys only.** Conformance and the host test sign with a throwaway key (`books-test-01`), never `gc-2026-01`.
  The publisher key is the owner's, in the login Keychain, and only `tools/release/publish.sh` uses it.
- Throwaway Bot brokers use port 8810 or above and `BUWIZ_KEYSTORE=file:`, never the login Keychain. Port 8787 is the
  owner's broker. Never touch the owner's Bot data folder. Use a worktree of the Bot repo, and never switch branches
  in the Bot checkout itself.
- The live Slice B test (`tools/bir/slice-b-live.sh`) runs only on a scratch eBIRForms data folder.
- The hand-off contract with eBIRForms is `tests/fixtures/handoff/`, the same files as eBIRForms'
  `tests/fixtures/books-handoff/`, pinned by one hash in both repos (`tests/handoff_contract.rs`).
- One protocol version, 1.0, with no shims for older names (2.0 and 2.1 were its pre-release numbering).
- The Bot's signing and conformance tools are Node; run them from a Bot checkout, never copy them here.

## Do not

- Commit, push, merge or publish unless the owner asks.
- Print a TIN, a ticket value, a key or a real amount into a log, a file or a reply.
