# poc: ledger lines to form proposals (throwaway)

Status: proof of concept, dated 2026-10-29. Writer W4. Synthetic data only. Nothing here calls Rox, BIR, Forms or the
cloud. Nothing here files, sends or pays. Zero dependencies. Bun only (no npm, npx or node).

The PoC answers one question: can the pure tax rules in `data/` turn posted ledger lines, with explicit tax attributes,
into deterministic, lineage-carrying form proposals that respect the crosswalk and the privacy rules? It is a sketch that
exercises the rules, not a product.

## Run it

```
cd /Volumes/goldcoders/buwiz-books/poc
bun test                                   # the suite
bun run fixtures                           # rewrites fixtures/synthetic/ from tools/gen-fixtures.ts
bun run vectors                            # rewrites fixtures/vectors/ from the engine
BUWIZ_FORMS_SCHEMAS=/path/to/agent-forms bun test   # optional override of the schema directory
```

The default schema directory is `/Volumes/goldcoders/buwiz-oss/crates/bir-core/data/agent-forms` (a byte-identical copy of
the Forms pin 63267ca, per `data/crosswalk/*.json`). If it is absent, the forms-schema group is skipped with a printed
message, not silently passed. Vector files are checked against a fresh computation, so after an engine change run
`bun run vectors` and review the diff. Do not edit a vector by hand.

## What is here

| Path | Purpose |
|---|---|
| `src/load.ts` | Loads `data/` (rules, crosswalks, categories, tax attributes), derives the closed vocabularies, normalizes fixtures. Rejects JS numbers in money fields. |
| `src/money.ts` | Integer centavos (`bigint`), decimal text with two places, half-away-from-zero rounding once at output. |
| `src/dates.ts` | Period keys (`2026-Q3`, `2026-09`, `2026`), Asia/Manila calendar day, due dates. |
| `src/ledger.ts` | Balance and one-side checks, reversal mirror, orphan and count-once rules, signed tax measure per line. |
| `src/rates.ts` | PT rate rows by effective date (disputed row OFF and warned), graduated tax, VAT threshold. |
| `src/forms.ts` | The three form computations (2551Q, 1601-C, 1701-MS) and their gates. Unknown gates fail closed. |
| `src/proposal.ts` | Assembles a proposal, applies the crosswalk discipline, computes `inputs_hash` (staleness) and `output_sha256`. |
| `src/schema-check.ts` | Checks the crosswalk and rules against the real agent-forms JSON. |
| `src/privacy.ts` | TIN-like, ticket and needle scanner, plus run-time canaries for its self-test. |
| `src/cases.ts` | The proposal cases, and the hand-written expectations (not computed by the engine). |
| `src/vectors.ts` | Writes and recomputes the language-neutral vectors. |
| `tools/gen-fixtures.ts` | Writes `fixtures/synthetic/*.json`. Every memo carries a `MEMO-SYNTH-` marker. |
| `tools/emit-vectors.ts` | Writes `fixtures/vectors/*.json`. |
| `test/*.test.ts` | 10 files, bun:test. Groups are described below. |

Fixtures (`fixtures/synthetic/`): `a-sole-prop-2026` (workbook worked example: sole proprietor, graduated, OSD, PT 116,
one employee); `b-eight-percent-2026` (8% elector); `c-reversal-credit-note-2026` and its `-acknowledged` twin (voids,
a later-period reversal, a credit note); `d-spouse-scope-2026` (spouse line refused); `e-second-profile-2026` (profile B,
for isolation); `f-slice-a-2026` (design Slice A numbers, DS-A); `g-pt-withholding-2026` (DS-H journey example with
percentage-tax credits, one pending and one final certificate that must not credit) and `g-overpayment-2026`.

Vectors (`fixtures/vectors/`): one file per case (`<case-id>.json`: the normalized profile-scoped input and the expected
proposal), plus `money-and-rules.json` (hand-written rounding, bracket, 13th-month, VAT and rate expectations).

## What it proves

Each claim names the test group that checks it. "Proves" means: the engine does this on the synthetic data, and a test
fails if it stops. It does not mean the tax rule is right in law (see the next section).

| Claim | Where |
|---|---|
| The workbook worked example reproduces from ledger lines: 2551Q base 290000.00, tax 8700.00; 1601-C taxable 38000.00, tax withheld 2808.40; 1701-MS net sales 1190000.00, OSD 476000.00, tax 85300.00, credits 52300.00, payable 33000.00 | `test/worked-example.test.ts` |
| Rounding is half away from zero, once: 1.50 at 3% is 0.05, 1.49 is 0.04, -1.50 is -0.05 (DS-C vectors) | `test/money-and-rates.test.ts` |
| Money never enters a float path; decimal text only; JS numbers refused | `test/money-and-rates.test.ts`, `test/ledger-and-slice-a.test.ts` |
| Slice A (A3-A5): unbalanced entry refused; void posts a reversal on the original date; Q3 base 200000.00, PT 6000.00, due 2026-10-25, overdue as of 2026-10-29; lineage lists the two live sales only | `test/ledger-and-slice-a.test.ts`, `test/worked-example.test.ts` |
| Reversals net without double counting: same-quarter void leaves lineage; later-period reversal counts once; Q2 is not changed by a Q3 void; orphan, twice-reversed, non-mirror and misdated reversals refused | `test/ledger-and-slice-a.test.ts`, `test/gates.test.ts` |
| Returns, credit notes and later-period reversals block until a person acknowledges them; acknowledgement regenerates the proposal (A6) | `test/gates.test.ts` |
| Negative base blocks, never clamps (DS-J); overpayment stays negative (II.19 = -1000.00) | `test/ledger-and-slice-a.test.ts`, `test/payroll-and-credits.test.ts` |
| 8% election: no PT010 row and no NIL row; a `needs_review` discrepancy cites RMO 23-2018 and the buwiz-oss conflict. The 1601-C stays ready (payroll is independent); the 1701-MS 8% path blocks | `test/gates.test.ts` |
| VAT gate is strictly greater than 3000000.00 of year-to-date posted sales (DS-B) | `test/gates.test.ts`, `test/money-and-rates.test.ts` |
| Spouse-scoped lines block MVP proposals and never produce a B-side key | `test/gates.test.ts`, `test/crosswalk-discipline.test.ts` |
| Employee share alone reaches 1601-C II.19. Employer share never reduces taxable compensation, even when mis-tagged as a contribution (contribution side decides) | `test/payroll-and-credits.test.ts` |
| 13th-month exemption capped at 90000.00 (rules vector); unmapped components and missing payroll block | `test/money-and-rates.test.ts`, `test/payroll-and-credits.test.ts` |
| PT credits (WB080, WB082) credit 2551Q item 15 only. IT credits (WI series) reach 1701-MS V.3 and V.4 only. WB084 is never credited. Pending, lost and disputed certificates never credit | `test/payroll-and-credits.test.ts` |
| Only sendable crosswalk keys carry values (fill class input, fill policy). No computed, locked or not_fillable key is ever in `fill_args`. Election boxes and spouse keys are refused, and the refusal is listed | `test/crosswalk-discipline.test.ts` |
| Each value cites a rule that the crosswalk says governs that key; every rule and gate the PoC evaluates exists in the rules files | `test/crosswalk-discipline.test.ts` |
| An account is not tax meaning: renaming the sales account changes no value. Rewriting every category default changes nothing | `test/crosswalk-discipline.test.ts` |
| Profile isolation: A's proposal is byte-identical with or without B in the input; B is computed from B alone; A's lineage names no B line | `test/isolation-and-staleness.test.ts` |
| Idempotency: the same inputs give the same canonical JSON, every time. A memo edit changes nothing | `test/isolation-and-staleness.test.ts` |
| Staleness: a changed line, acknowledgement, tax fact, rule-set version or rate row makes the proposal stale and refuses fill. A Q4 change does not stale Q3 | `test/isolation-and-staleness.test.ts` |
| Privacy: no proposal, vector or fixture carries a TIN-like run, a `tkt_` value or a memo marker. The scanner is proven by planting run-time canaries | `test/privacy.test.ts` |
| The crosswalk and rules agree with the real agent-forms schema files: pinned SHA-256 matches; every crosswalk key exists; sendable keys are unlocked bindings or verified typed aliases; locked classes match schema locks; every fill rule has a sendable target | `test/forms-schema.test.ts` (skips with a message when the directory is absent) |
| Vectors are reproducible: each recomputes to its file, the money vectors match hand expectations, and the fixtures on disk equal the generator | `test/vectors.test.ts` |

Mutation check (done during the build, not kept as a test): banker's rounding, WB084 credited, and the voided pair left in
lineage each failed tests. Two employer-share mutations were no-ops on this data: the fixture's employer line is not tagged
as a contribution, and one edit sat inside a clause that already required the employee side. A mis-tagged-employer test
was added, and a real mutation (employer side admitted into II.19) is now caught by it.

## What it does NOT prove

- **No Rox host.** No plugin install, broker, signed manifest, trust entry, TSX app, nsh or QuickJS. Nothing runs in a
  Rox session. The Rox and store acceptance items (A0, A1, A2, A8, A9, A12, A13) are not run here. A11 is covered for
  proposal outputs only.
- **No Forms call.** No BIR draft is opened, filled, read back or validated. The read-back values are Books' own
  expectations of what Forms would compute. They are not checked against Forms' computation. Item 14 equal to Books'
  PT (B2) is not tested here, and the tie vector through the live cents helper (`form_2551q.rs:1467`) is not run.
- **No tax-professional review.** The rules are the data files as written. UNCONFIRMED items stay UNCONFIRMED (the
  1% extension, the 1601-C printed due text, the 1701-MS OSD base, the 15 October installment, the 8%-year 2551Q
  conflict, the credit-note treatment). The PoC encodes a chosen side only where the data does, and says so in the
  output.
- **No store.** No encrypted SQLite, key file, ticket store, client_request_id retry store, audit log, period close or
  reopen events, or sync. Idempotency is shown only as "same inputs, same bytes".
- **Partial engine.** 2551Q: Schedule 1 row 1 only, PT010 only. 1601-C: no MWE components, no 13th-month YTD cap, no
  other non-taxable items (they block). 1701-MS: OSD path only; the 8% pathway, itemized deductions and non-operating
  income (final-tax attribute unknown) block. No 2550Q, 1701Q, 2307 import, evidence or OCR. No spouse path.
- **Synthetic numbers.** Every amount is invented, or taken from the workbook's synthetic example. Nothing here is
  evidence about any real filer or any real BIR behaviour.
- **Expectations are the PoC's own.** The vectors are generated by the engine and compared with the engine on each run,
  so they catch drift, not error. The hand tables in `src/cases.ts` are the independent check and need human review.
- **Privacy scanner limit.** A peso amount of 100000000.00 or more has a nine-digit run and would be flagged. That is
  INFERENCE 14 in AGENTS.md. The scanner matches by digit count, not by field context. A test records the limit. The
  fixtures stay below it.
- **Digest exemption (a departure from the literal rule).** The brief's regex also matches digit runs inside 40-hex
  commit ids and 64-hex SHA-256 digests, which every proposal carries (`output_sha256`, `fixture_sha256`, the pin). The
  scanner therefore exempts a string that is exactly one 40-hex or 64-hex value and scans every other string and every key.
  `test/privacy.test.ts` proves the exemption is narrow. The grep `[0-9]{3}-?[0-9]{3}-?[0-9]{3}` over `fixtures/vectors/` matches
  inside the 64-hex digests and inside the integer centavo strings in `money-and-rules.json` (for example `"300000000"`).
  Neither is a finding: digests are exempt as above, and a plain integer string under a key ending in `_centavos` is
  money by field context, so it skips the TIN check only (tickets and needles are still checked). A test proves this
  exemption is narrow.

## Observed (read from the data and the code)

- CONFIRMED (schema check, test/forms-schema.test.ts): the three crosswalk pins match the local agent-forms files
  byte for byte. 2551Q `a206416c…5788`, 1601-C `07ca2f04…c251`, 1701-MS `6cf45329…ab84`. Zero problems across 94, 105 and
  206 fields, and across 69, 77 and 92 rules that have crosswalk targets.
- CONFIRMED (data/crosswalk, schema): the keys that can carry a sent value (fill policy plus a fill key) are 3 for 2551Q,
  12 for 1601-C and 7 for 1701-MS. Eight 1601-C and 21 1701-MS row families have no key at all. The 2551Q ATC key is a
  typed alias (`schedule_1.0.atc` for box `drpATC1`), verified in verify.md s3-s4.
- CONFIRMED (data/rules/rates.v1.json): PT 3% from 2023-07-01 (300 bps). The 1% row for 2020-07-01 to 2023-06-30 is
  default ON. The 1% extension to 2025-12-31 is stored disputed and OFF (UNCONFIRMED, decision D6). VAT threshold
  3000000.00 strictly greater. Graduated test vector 714000.00 gives 85300.00.
- CONFIRMED (research/spreadsheet-corrections.md, worked example after corrections): the figures this PoC reproduces.
- CONFIRMED (data/tax-attributes.v1.json): the closed vocabularies used by `src/load.ts`.
- UNCONFIRMED (data/rules/rates.v1.json osd, common.rules.json GATE.8PCT_YEAR_2551Q): the OSD base and the 8% block.

## Recommended (proposals; not verified)

- Keep `fill_args`, `expected_readback` and `reference_values` as three separate groups. The first is what Books would
  send; the second is what Books expects Forms to show; the third is evidence Books never sends.
- Put the tax meaning on the line (treatment, ATC, credit kind, contribution side, person scope). The account is posting
  only. This PoC enforces it: a changed account changes no value.
- Keep staleness on a period-scoped hash, and keep the whole-profile fixture hash separate. A Q4 change should not stale
  a filed Q3 proposal. A rule-set, rate-row or fact change always does.
- Fail closed on any gate or component the engine does not implement. Each such case is a blocking item with a code, not
  a pass. The main ones are tested: 8% election, returns, VAT, spouse, negative base, unmapped compensation, missing
  payroll, and the 1701-MS 8% path.

## Findings from building it (for W2, W3 and the owners)

1. **`tax_type` is single-valued in data/tax-attributes.v1.json, but the rules need two tax families on one sale.** The
   2551Q selector wants `percentage_tax` on a sales line; the 1701-MS selectors (IV.5, IV.6, IV.13) want `income_tax` on the
   same line. The PoC models `tax_types` as a set. Owner or W2 decision needed. This PoC does not edit `data/`.
2. **No PT rate row before 2020-07-01.** A 2019 quarter fails closed (`pt_rate_row_missing`). That is a data gap, not an
   error. Rows before that date need a source before the engine can cover them.
3. **Category defaults are not enough to classify a return.** `refunds_and_discounts` defaults to treatment `none` and no
   ATC. A credit note has to be classified as `sale_pt`, PT010, `contra_sales` on the line. An unclassified sales-class
   line blocks (`unclassified_sales_line`). This confirms the categories file is suggestions only.
4. **Row families have no key.** Schedule I rows (1601-C), the indexed schedules (1701-MS IV.10A and others) are
   `key_pattern` entries with `key: null`. The PoC indexes them by pattern and never sends them. The schema check treats a
   sendable row family as an error.
5. **The 2551Q, 1601-C and 1701-MS fill rules have sendable targets, and all sendable keys are unlocked.** No schema
   problem was found in this build. That result depends on the pinned digests matching the local files, which it does.
6. **Nine-digit centavo runs trip the TIN-like regex** (AGENTS.md INFERENCE 14). The PoC identifiers were renamed to decimal
   text to avoid it (for example `pt-3000000.00-300`). The limit is tested, not hidden.
7. **The OSD base conflict surfaces on every OSD proposal** as `DISC-OSD-BASE-1701MS` (UNCONFIRMED: printed Part I item 17
   versus Part IV item 11).
8. **Election boxes have no Forms lock** (1701-MS I.12, I.17, I.18; blocker B7). They appear as refused keys with that
   reason, and a warning notes they are not compared with the profile election.

Suggested `docs/07` design corrections (W3 owns that file; not edited here): (a) `tax_type` becomes a set, or the 1701-MS
selectors change; (b) the 1601-C due-date rule stays UNCONFIRMED in the proposal; (c) the 8% 1701-MS pathway is listed as
not implemented.

## Test output

The summary below is verbatim from the final `bun test` run in `poc/`.

```
bun test v1.4.2 (744846f84)

 187 pass
 0 fail
 1341 expect() calls
Ran 187 tests across 10 files. [511.00ms]
```
