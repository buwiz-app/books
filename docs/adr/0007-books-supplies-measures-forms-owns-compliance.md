# ADR 0007: Books supplies accounting measures; Buwiz Forms owns compliance

Status: Accepted (owner decision, binding, 2026-10-29).

## Context

Books is bookkeeping and accounting. Buwiz Forms (the BIR mini app) is the compliance side, and it already computes
the tax boxes: the crosswalk marks them `computed` or `locked_*`, and its fill tool refuses them
(`data/crosswalk/*.json`). A second implementation in Books could only disagree with Forms. Forms has no ledger;
only Books knows which transactions happened, with lineage to the line.

## Decision

| Side | Owns |
|---|---|
| **Books** | Transactions, payroll facts and evidence. **Accounting measures** per business profile per period, each with lineage to source lines (`data/measures.v1.json`). The book's accounting basis ([ADR 0008](0008-cash-basis-by-default.md)). |
| **The bridge** | Maps measures onto Forms **input** boxes only (`fill_class: input` with a fill policy), in versioned rule sets (`data/rules/*.rules.json`). Where a box depends on a taxpayer choice, it **reads** the choice from the Forms draft or profile (`form_context`) and never decides, infers or defaults it. It never targets computed, locked or not-fillable boxes. |
| **Buwiz Forms** | Elections (8% or graduated), whether a form applies for a period (including NIL returns), ATC selection, rates, tax due, thresholds, OSD or itemized and the OSD amount, graduated tax, tax rounding, penalties, due dates and overdue status. |

Rules that follow:

1. Books code loads no rate, threshold, election, applicability, ATC-choice, due-date or penalty data.
2. Line attributes are accounting and evidence facts. No `tax_type`, `atc` (as a choice), `treatment`, `regime`,
   `vat_status` or deductibility. The ATC printed on a received certificate stays, as evidence (`certificate_atc`).
3. Boxes Forms computes have policy `forms_computes`: Books neither computes nor compares them. ATC keys and inputs
   that need a tax judgement are `never_fill`.
4. **One labelled exception.** 1601-C item 17 asks for the non-taxable part of 13th month pay and other benefits
   (an annual cap per employee). It needs year-to-date payroll that only Books has, so the bridge keeps this split in
   the mapping layer (`1601C.II.17`, `input_level_tax_rule`), labelled "the one input-level tax rule in the bridge".
   Do not add another without an owner decision.
5. Where choosing the measure for a box is itself a tax question (for example 2551Q row 1, net or gross sales), the
   mapping carries `review.status: open` and the proposal warns `MAPPING_REVIEW_OPEN`.

## Consequences

- A proposal holds measures with lineage, fill arguments for input boxes, reference values (evidence, never sent),
  and the Forms context it read. It holds no tax due, rate, due date, overdue flag or penalty.
- Forms' computed values appear in the vectors only as `expected_forms_computed` documentation, never as a Books
  output.
- A change in the Forms context Books read (for example the tax option) makes a proposal stale.
