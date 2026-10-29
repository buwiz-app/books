# ADR 0007: Books supplies accounting measures; Buwiz Forms owns compliance

Status: Accepted (owner correction, binding, 2026-10-29). Supersedes the tax-computation parts of ADR 0004 and ADR 0006 and of `docs/07` section 1.3 (listed below).
Date: 2026-10-29. Writer: ADR-0007 refactor.

## Context

- The owner set the boundary on 2026-10-29: Books is bookkeeping and accounting; Buwiz Forms / the BIR mini app is the compliance side. The correction is recorded in [`research/traces/decisions.md`](../../research/traces/decisions.md), "Addendum 2026-10-29: owner correction of the Books scope".
- The phase-2 design had drifted past that boundary. Books held a percentage-tax rate table with the disputed 1% window (`rates.v1.json`), the VAT threshold, the 8% parameters, the 2023 graduated table, the OSD fraction, a tax-rounding rule and due dates. Its rules gated output on the 8% election, the VAT threshold and overdue status, and its PoC computed the 2551Q tax, the 1601-C totals and the 1701-MS tax and payable, then compared them with Forms ("cross_check").
- Forms already computes those boxes. The crosswalk marks them `computed` or `locked_*`, and the fill tool refuses them (`data/crosswalk/*.json`; `[verify §3, §4]`). A second implementation in Books could only disagree with Forms, and the open decisions show where: D5 (2551Q in an 8% year; RMO 23-2018 versus buwiz-oss NIL PT010), D6 (an unsourced 1% extension), D7 (tax rounding; Forms' f64 cents helper), D14 (overdue and penalties).
- Forms has no ledger. Only Books knows which transactions happened, with lineage to the line.

## Decision

| Side | Owns |
|---|---|
| **Books** | Transactions, payroll facts and evidence. **Accounting measures** per business profile per period, each with lineage (source line ids). Catalogue: [`data/measures.v1.json`](../../data/measures.v1.json). Examples: gross sales; sales returns, allowances and discounts (separate measures); net sales; cost of sales; other non-operating income; operating expenses by category; gross compensation and its components; EMPLOYEE mandatory contributions, kept separate from EMPLOYER shares; tax withheld from employees as recorded by payroll (a fact); certificates received (2307/2316) with status. The revenue-recognition date policy (the accounting part of D4). |
| **The bridge** | Maps measures onto Forms **input** boxes only (`fill_class: input` with a fill policy), with lineage, in versioned rule sets ([`data/rules/*.rules.json`](../../data/rules/), r2). Where a form has alternative boxes for a taxpayer choice, it **reads** the choice from the Forms draft or profile (`form_context`) and never decides it: 1701-MS `/taxpayer/tax_option` graduated → `frm1701MS:txtSalesRevenueNo5a` (`/taxpayer/sales`) and IV.6; eight_percent → `frm1701MS:txtSalesRevenuesFeesANo19a` (`/taxpayer/eight_sales`). It never targets computed, locked or not_fillable boxes. |
| **Buwiz Forms / BIR mini app** | Elections (8% versus graduated, chosen yearly in the tax profile); whether a form is required or applicable for a period (including 2551Q in an 8% year and NIL returns); ATC selection; rates (including the 1%/3% percentage-tax history); tax due; thresholds (VAT ₱3M, 8% eligibility, mixed income); OSD versus itemized and the OSD amount; graduated tax; rounding of tax amounts; penalties; due dates and overdue status. |

Rules that follow:

1. Books code loads no rate, threshold, election, applicability, ATC-choice, due-date or penalty data. `rates.v1.json` and the removed gates moved to [`research/handoff-to-forms/`](../../research/handoff-to-forms/) as reference for the Forms owner.
2. Line attributes are accounting and evidence facts. `tax_type`, `atc` (as a choice), `treatment`, `regime`, `vat_status`, `election_event` and `deductibility` are removed ([`data/tax-attributes.v1.json`](../../data/tax-attributes.v1.json) `forms_supplied`). The ATC printed on a received certificate stays, as evidence (`certificate_atc`).
3. Crosswalk policy `cross_check` becomes `forms_computes`: Books neither computes nor compares those boxes. ATC keys and inputs that need a tax classification, threshold or deductibility judgement become `never_fill`.
4. **One labelled exception.** 1601-C item 17 asks for the NON-TAXABLE part of 13th month pay and other benefits (annual ₱90,000 cap per employee). It needs year-to-date payroll that only Books has. The bridge keeps this split in the mapping layer (`1601C.II.17`, `input_level_tax_rule`), labelled "the one input-level tax rule in the bridge". It is open for the Forms owner: either Forms applies the cap or the bridge keeps it.
5. Which measure feeds a box is a mapping rule. Where it is a tax question (2551Q row 1: net or gross sales, former D20; non-operating income and final tax, `EXC.FINAL_TAX_INTEREST`), the mapping carries `review.status: open` and the proposal warns `MAPPING_REVIEW_OPEN`.

## Superseded

| Where | What is superseded |
|---|---|
| [ADR 0004](0004-forms-bridge.md) | Read-back of computed boxes (for example "item 14 equals Books' PT"). Books has no expected tax value to compare. The read-back compares only the input keys Books sent. The hand-off mechanism itself stands. |
| [ADR 0006](0006-tax-meaning-on-lines-not-accounts.md) | The line attributes `treatment`, `atc`, `tax_type`; "mapping rules produce form fields" for computed fields. Lines carry accounting facts; measures select lines; mappings feed input boxes only. "Never on accounts" stands. |
| [docs/07 §1.3](../07-decisions-and-blockers.md#13-design-decisions-w3-write-up-2026-10-29) | DD-5 (rates in basis points), DD-6 (PT rounding), DD-8 (rate row ids in proposals), DD-9 (PT rate rows), DD-10 (VAT gate), DD-14 (8%-year block). |
| [docs/07 §2](../07-decisions-and-blockers.md#2-open-owner-decisions) | D5, D6, D7, D14 move to "Handed to Forms". D4 splits (recognition date stays). D20 becomes a mapping question for tax review. |

## Consequences

- A proposal holds measures (with lineage), fill arguments for input boxes, reference values (evidence, never sent), and the Forms context it read. It holds no tax due, rate, due date, overdue flag or penalty.
- The 2551Q proposal is produced whatever the election. Whether it is needed is Forms' call. The worked example gives net sales 290,000.00 for `txtATCAmt1`. The 3% tax of 8,700.00 is Forms' output and appears only as `expected_forms_computed` documentation in the vectors.
- A change in the Forms context (for example the tax option) makes a 1701-MS proposal stale.
- Tests no longer cover rates, rounding ties, thresholds or due dates in Books. They now cover: input-only proposals, both Forms-context variants, measure lineage and the absence of compliance code.
- The Forms owner receives a hand-off list, with its evidence: [`research/handoff-to-forms/compliance-notes.md`](../../research/handoff-to-forms/compliance-notes.md).

## Evidence

- Owner correction: `research/traces/decisions.md`, addendum dated 2026-10-29.
- Crosswalk classes and locks: `data/crosswalk/*.json`; `[verify §3, §4]`.
- Conflicts that showed the duplication: `[verify §2 items 63, 73, 84]`; `[bir §3.2 8-6]`; buwiz-oss `form_2551q.rs:2202-2228`; Forms cents helper `form_2551q.rs:1467` (live tree).
- 1701-MS election boxes are not locked against the profile (a Forms bug): `[verify §1 item 3, §2 item 10]`.
