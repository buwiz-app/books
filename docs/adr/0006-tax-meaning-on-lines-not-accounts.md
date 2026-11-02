# ADR 0006: Accounting meaning lives on journal lines, never on accounts

Status: Accepted. Narrowed by [ADR 0007](0007-books-supplies-measures-forms-owns-compliance.md): lines carry accounting and evidence facts only, never a tax choice.

## Context

One account can feed several forms in different ways. Salaries feed both the 1601-C employer withholding return
and the 1701-MS itemized deductions. Refunds and discounts reduce the 2551Q base, the graduated sales and the 8%
net sales. If an account carried meaning, one account would have to mean two things at once.

## Decision

- An account has a number, a type and a name. It carries no form meaning.
- A category is a user-facing grouping with default line attributes. It never fixes a form field.
- Each journal line carries its own accounting facts (`data/tax-attributes.v1.json`): income class, contra kind,
  person scope, payroll component, contribution side and certificate evidence. Defaults come from the category and
  are fixed at posting.
- Measures select lines by attribute. Versioned mapping rules (`data/rules/`) map measures to Forms input keys.
- Never infer a form field from an account number or subtype.

## Consequences

- Posting validates attributes, not only balance.
- Changing a category later never changes posted lines.
- Renaming or renumbering an account never changes a proposal.
