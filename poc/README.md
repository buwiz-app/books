# poc: ledger lines to accounting measures to Forms input boxes (throwaway)

Status: proof of concept, dated 2026-10-29, refactored the same day for [ADR 0007](../docs/adr/0007-books-supplies-measures-forms-owns-compliance.md).
Synthetic data only. Nothing here calls Rox, BIR, Forms or the cloud. Nothing here files, sends or pays. Zero dependencies.
Bun only (no npm, npx or node).

The PoC answers one question: can posted ledger lines, carrying accounting attributes only, become deterministic,
lineage-carrying **accounting measures**, and can a thin **bridge** map those measures onto Forms **input** boxes while
reading (never deciding) the taxpayer choices that select between boxes? It holds no compliance logic: no rate,
election, threshold, applicability or NIL test, ATC choice, tax due, OSD, graduated tax, tax rounding, due date,
overdue status or penalty. Those are Buwiz Forms'. The one labelled exception is the 1601-C item 17 non-taxable
13th-month split, "the one input-level tax rule in the bridge".

## Run it

```
cd /Volumes/goldcoders/buwiz-books/poc
bun test                                   # the suite
bun run fixtures                           # rewrites fixtures/synthetic/ from tools/gen-fixtures.ts
bun run vectors                            # rewrites fixtures/vectors/ from the engine (and removes vectors no case writes)
BUWIZ_FORMS_SCHEMAS=/path/to/agent-forms bun test   # optional override of the schema directory
```

The default schema directory is `/Volumes/goldcoders/buwiz-oss/crates/bir-core/data/agent-forms` (a byte-identical copy of
the Forms pin 63267ca, per `data/crosswalk/*.json`). If it is absent, the forms-schema group is skipped with a printed
message, not silently passed. Vector files are checked against a fresh computation, so after an engine change run
`bun run vectors` and review the diff. Do not edit a vector by hand.

## The chain

```
ledger (accounting attributes) ─> measures (data/measures.v1.json, with lineage)
   ─> mapping (data/rules/*.rules.json r2: measure -> input key; optional form_context read from Forms)
   ─> proposal: measures + fill_args (input keys only) + reference_values (evidence, never sent) + lineage
   ─> Forms computes the tax, totals, rate, due date (not here)
```

## What is here

| Path | Purpose |
|---|---|
| `src/load.ts` | Loads `data/` (measures catalogue, mapping rule sets, crosswalks, categories, line attributes), derives the closed vocabularies, normalizes fixtures. Rejects JS numbers in money fields, and rejects compliance attributes (`atc`, `treatment`, `tax_types`, `regime`) on lines and tax facts or elections on profiles. Loads no rate file. |
| `src/money.ts` | Integer centavos (`bigint`) and decimal text with two places. No rounding helper: measures are exact. |
| `src/dates.ts` | Period keys (`2026-Q3`, `2026-09`, `2026`), Asia/Manila calendar day. No due dates. |
| `src/ledger.ts` | Balance and one-side checks, reversal mirror, orphan rules, signed amount per line (`measure_side`). |
| `src/measures.ts` | The accounting measures: sales family (gross, returns, allowances, discounts, net), cost of sales, operating expenses by category, other income, payroll components, employee and employer contributions (separate), tax withheld as recorded, certificates received (PT and IT, separate). Each with lineage, netting of same-window voids, and the lines it excludes by rule. |
| `src/bridge.ts` | The mapping: reads the Forms context, selects mappings, enforces the period lock, return-event acknowledgement, spouse refusal and payroll completeness, aggregates measures into fill values, and fails closed on mappings that are open for tax review. Holds the 1601-C item 17 cap split (labelled). |
| `src/proposal.ts` | Assembles a proposal, applies the crosswalk discipline (input keys with a fill policy only), computes `inputs_hash` (staleness, including the Forms context) and `output_sha256`. |
| `src/schema-check.ts` | Checks the crosswalk, the mappings and the Forms-context paths against the real agent-forms JSON. |
| `src/privacy.ts` | TIN-like, ticket and needle scanner, plus run-time canaries for its self-test. |
| `src/cases.ts` | The proposal cases with their Forms context, `expected_forms_computed` documentation, and hand-written money and item-17 expectations. |
| `src/vectors.ts` | Writes and recomputes the language-neutral vectors. |
| `tools/gen-fixtures.ts` | Writes `fixtures/synthetic/*.json`. Every memo carries a `MEMO-SYNTH-` marker. |
| `tools/emit-vectors.ts` | Writes `fixtures/vectors/*.json`. |
| `test/*.test.ts` | 10 files, bun:test. Groups are described below. |

Fixtures (`fixtures/synthetic/`): `a-sole-prop-2026` (workbook worked example: sales, a return, expenses, income-tax
certificates, one employee); `b-eight-percent-2026` (the same ledger under another profile, used with an 8% Forms
context); `c-reversal-credit-note-2026` and its `-acknowledged` twin (voids, a later-period reversal, a credit note);
`d-spouse-scope-2026` (spouse line refused); `e-second-profile-2026` (profile B, for isolation); `f-slice-a-2026` (design
Slice A numbers, DS-A); `g-pt-withholding-2026` (percentage-tax 2307s: one received, one pending, and a WB084 2306) and
`g-overpayment-2026`. Profiles carry period statuses only: no tax facts, elections or prior filings.

Vectors (`fixtures/vectors/`): one file per case (`<case-id>.json`: the normalized profile-scoped input, the Forms
context, the expected proposal, and for some cases `expected_forms_computed` with `asserted_against_books: false`), plus
`money-and-mapping.json` (hand-written money text round trips and item-17 split expectations).

## What it proves

| Claim | Where |
|---|---|
| Worked-example measures: Q3 2026 gross sales 300000.00, returns 10000.00 (separate measure), net sales 290000.00, so the 2551Q `txtATCAmt1` input is 290000.00 (the mapping uses net sales; open for tax review) | `test/worked-example.test.ts` |
| September 2026 payroll: gross compensation 42000.00, de minimis 2000.00, employee contributions 2000.00 to `frm1601c:txtTax19`; the employer share is a separate measure and never reaches item 19, even with a misleading scheme tag | `test/worked-example.test.ts`, `test/payroll-and-credits.test.ts` |
| 1701-MS: net sales 1190000.00. Forms context graduated: IV.5 (`/taxpayer/sales`) 1200000.00 and IV.6 10000.00. Forms context eight_percent: IV.19 (`/taxpayer/eight_sales`) 1190000.00. Both variants tested. IV.8 only when the Forms draft says itemized | `test/worked-example.test.ts`, `test/returns-and-context.test.ts` |
| Only input keys are proposed: every fill key is `fill_class: input` with a fill policy; no computed, locked, not_fillable, ATC, rate, tax, total or election key is ever proposed; forged fills are refused | `test/crosswalk-discipline.test.ts`, `test/worked-example.test.ts` |
| Every fill-policy key has exactly one mapping; every crosswalk rule id resolves to a mapping, a `not_mapped` pointer (computed boxes point to Forms) or a common rule; every mapping's measures exist and are implemented | `test/crosswalk-discipline.test.ts` |
| The bridge reads the Forms context and never decides it: a missing, unsupported or undeclared value blocks; the 8% context changes only the 1701-MS mapping | `test/returns-and-context.test.ts` |
| Reversals and returns: same-quarter void netted out of lineage; later-period reversal counted once in its own period; Q2 unchanged; return events held until acknowledged; returns, allowances and discounts are separate measures summed once into IV.6 | `test/returns-and-context.test.ts`, `test/ledger-and-slice-a.test.ts` |
| PT and IT certificates never cross; a WB084 2306 and pending, lost or disputed 2307s never count; certificates are windowed by their own period end | `test/payroll-and-credits.test.ts` |
| 1601-C item 17, the one input-level tax rule in the bridge: 95000.00 paid gives 90000.00; earlier payments in the year use up the cap; the cap is per employee; a line with no employee ref blocks | `test/payroll-and-credits.test.ts`, `test/money-and-dates.test.ts` |
| No compliance code: no PoC source holds a rate, threshold, graduated table, due date or penalty helper; `data/rules/` holds no rate file and no compliance gate; lines and profiles carrying compliance attributes are refused | `test/returns-and-context.test.ts`, `test/ledger-and-slice-a.test.ts` |
| Money is integer centavos and decimal text; JS numbers refused; measures never rounded | `test/money-and-dates.test.ts`, `test/ledger-and-slice-a.test.ts` |
| Profile isolation: A's proposal is byte-identical with or without B in the input; B is computed from B alone | `test/isolation-and-staleness.test.ts` |
| Idempotency: the same inputs give the same canonical JSON; a memo edit changes nothing | `test/isolation-and-staleness.test.ts` |
| Staleness: a changed line, a withdrawn acknowledgement, the Forms context, a mapping change, a rule-set version or the measures catalogue makes the proposal stale and refuses fill; a Q4 change does not stale Q3 | `test/isolation-and-staleness.test.ts` |
| Privacy: no proposal, vector or fixture carries a TIN-like run, a `tkt_` value or a memo marker; the scanner is proven with run-time canaries | `test/privacy.test.ts` |
| The crosswalk, the mappings and the Forms-context paths agree with the real agent-forms schema files (pinned SHA-256; every key exists; sendable keys are unlocked bindings or verified aliases; each Forms-context path and value is set by a schema option) | `test/forms-schema.test.ts` (skips with a message when the directory is absent) |
| Vectors are reproducible; `expected_forms_computed` is flagged as not asserted and never overlaps Books output keys; fixtures on disk equal the generator | `test/vectors.test.ts` |

## What it does NOT prove

- **No Rox host, no Forms call.** No BIR draft is opened or filled. Whether Forms computes the documented
  `expected_forms_computed` values is not tested here: those values (for example the 2551Q tax 8700.00) are Forms'
  output and are recorded only as documentation.
- **No tax-professional review.** Open mapping questions stay open: net or gross sales for 2551Q row 1 (former D20),
  final-taxed income in 1701-MS IV.13/IV.20 (fails closed), the MWE split for 1601-C item 16 (fails closed), who applies
  the item 17 cap (DD-29).
- **No store.** No encrypted SQLite, key file, ticket store, retry store, audit log, period close events or sync.
  Idempotency is shown only as "same inputs, same bytes".
- **Partial engine.** 2551Q: row 1 and item 15 only. 1601-C: no per-employee MWE flag (item 16 fails closed when
  holiday/overtime lines exist). 1701-MS: IV.10A (itemized deductions) is not mapped (deductibility is a tax
  judgement); V.3/V.4 are reference values only. No 2316 measure yet. No spouse path.
- **Synthetic numbers.** Every amount is invented or taken from the workbook's synthetic example.
- **Expectations are the PoC's own.** Vectors catch drift, not error. The hand tables in `src/cases.ts` are the
  independent check.
- **Privacy scanner limit.** A peso amount of 100000000.00 or more has a nine-digit run and would be flagged (AGENTS.md
  invariant 14). The fixtures stay below it. A whole 40-hex or 64-hex digest is exempt, and a plain integer string under
  a key ending in `_centavos` skips the TIN check only; `test/privacy.test.ts` proves both exemptions are narrow.

## Findings from the refactor

1. **1701-MS IV.5 takes gross sales, not net.** The form prints IV.5 (sales), IV.6 (less returns, allowances and
   discounts) and IV.7 (net, computed). So under the graduated option the bridge sends gross 1200000.00 to IV.5 and
   10000.00 to IV.6; Forms computes IV.7 = 1190000.00. Under 8%, IV.19 takes net sales 1190000.00 directly.
2. **ATC selection left Books.** `schedule_1.0.atc` (2551Q) and `frm1601c:txtATC` are now `never_fill`, source `forms`.
   The bridge fills the row-1 amount only; Forms must have the ATC on row 1.
3. **Three 1601-C inputs left Books.** Items 20 (other non-taxable compensation) and 23 (not subject to withholding)
   need a tax classification or a threshold test; 1701-MS IV.10A needs a deductibility judgement. They are `never_fill`.
4. **The item 17 cap needs a per-employee reference.** The cap is per employee per year, so 13th-month lines carry the
   employee's `party_ref`; without it the bridge blocks.
5. **WB084 is evidence of a different document.** The fixture now records it as a `certificate_2306`, so the exclusion is a
   document-kind fact, not an ATC rule in Books.

## Test output

The summary below is verbatim from the final `bun test` run in `poc/` (2026-10-29, after ADR 0008: cash basis by default).

```
bun test v1.4.2 (744846f84)

 177 pass
 0 fail
 2378 expect() calls
Ran 177 tests across 11 files.
```
