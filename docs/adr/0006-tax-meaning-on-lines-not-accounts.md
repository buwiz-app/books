# ADR 0006: Tax meaning lives on journal lines and mapping rules, never on accounts

Status: Proposed. Owner review requested with the category model (see [docs/02](../02-categories-and-attributes.md)).
Date: 2026-10-29. Writer: W3.

## Context

- One account can feed several forms with different legal treatment:
  - `salaries` (61100) feeds 1601-C II.14 (employer withholding return) and 1701-MS IV.10A (owner's itemized deduction) `[sheets §8 item 1]`;
  - `refunds_and_discounts` (49000) reduces the PT base (2551Q), graduated sales (1701-MS IV.6) and 8% net sales (IV.19) `[sheets §8 item 1]`.
- BK-TS accounts have no tax, ATC, currency or normal-balance attribute `[books §1 item 3; §2]`.
- BK-TS journal lines have no tax fields `[books §2]`.
- BK-TS maps accounts to categories through three mapping types only: bank, bill, invoice `[books §2 "Category mappings"]`. PH accounts resolve by number, not by mapping `[books §3c]`.
- Under the BK-TS design, a payee's ATC is a default on the party, not on the line `[books §2]`.
- The research workbook's mapping tab holds these links as free-text filters `[sheets §8 item 1]`.

## Decision

- An account has a number, a type and a name. It carries no tax meaning.
- A category is a user-facing grouping. It carries default attributes. It does not fix a form field.
- Each journal line carries its own tax attributes: `treatment`, `atc`, `credit_kind`, `person_scope`, `comp_component`, `contribution_side`, `tax_period_override`. Defaults come from the category and can be changed at posting.
- Mapping rules select lines by attribute and produce form fields. Rules are versioned and bound to a pinned Forms schema hash (DD-8).
- A per-line override carries a reason. It applies before rules. It becomes `needs_review` if a later rule conflicts.

## Alternatives considered

| Option | Why not |
|---|---|
| Put tax meaning on the account | One account would then serve two forms with different treatments. The `salaries` example shows the failure. |
| Put tax meaning only on the category | A category can be changed later, so past lines would silently change meaning. Line attributes are fixed at posting. |
| Put tax meaning only in mapping rules, with no line attributes | The rules cannot tell a sale from a refund that uses the same account. Lines need the attribute. |

## Consequences

- Every posting carries more fields. The write validation has to check attributes, not only balance.
- The journal line schema is larger than BK-TS's. That is intended.
- Defaults keep the user's work small. A category fills the attribute, and the user can override it.
- The rule engine reads lines, not accounts. Tests cover attribute-driven selection (06 section 3 and 4).
- Changing a category later does not change posted lines.

## Evidence

- `[sheets §8 item 1]`
- `[books §1 item 3]`; `[books §2]` (no tax fields on accounts or journal lines); `[books §3c]`
- `[canonical design, journey §7.1, four-layer model]`
- `[01 section 7.1-7.2]`

## Revisit when

- The category model (docs/02) changes the default-attribute rules.
- A form needs a meaning that no attribute can carry.
