# ADR 0009: Fallback account-to-box mapping lives in Buwiz Forms

Status: Accepted (owner decision, 2026-11-24). Extends [ADR 0006](0006-tax-meaning-on-lines-not-accounts.md) and [ADR 0007](0007-books-supplies-measures-forms-owns-compliance.md).

## Context

Books maintains versioned rule sets (`data/rules/*.rules.json`) that map line-attribute accounting measures onto
Buwiz Forms input boxes for 2551Q, 1601-C, and 1701-MS. Buwiz Forms covers 43 BIR forms, many of which have input
boxes not yet covered by a Books rule set. Users also want to pick a Books nested chart-of-accounts parent or child
account directly in Buwiz Forms for those uncovered boxes.

Allowing account mapping on boxes that already have a Books measure rule would create two conflicting sources of
truth for the same box (an account's balance is not identical to a line-attribute measure — for example,
`gross_sales` selects lines by `income_class` and excludes payment receipts).

## Decision

1. **Measures remain authoritative where a Books rule exists.** For any form input box covered by a Books rule set,
   the Books measure and hand-off contract remain the single source of truth.
2. **Account-to-box mapping is allowed only for boxes with no Books rule.** When a Buwiz Forms input box has no
   Books measure rule, the person may map that box to a Books account (parent subtree or child account).
3. **The mapping lives in Buwiz Forms, not in Books.** Buwiz Forms stores the `(taxpayer, form, box) -> Books account`
   mapping in its own database. Books stores no form-box mapping on accounts and never infers form meaning from
   account numbers or subtypes ([ADR 0006](0006-tax-meaning-on-lines-not-accounts.md)).
4. **Books supplies only read-only balances and lineage.** Books exposes nested COA hierarchy (`parent_account_id`),
   exact integer-centavo period `subtree_balance` rollups (with contra accounts netted automatically by double-entry
   debit/credit polarity), and journal-line lineage.
5. **No cross-app writes.** If a needed sub-account does not exist in Books, Buwiz Forms directs the person to create
   it in Books' Accounts screen; Buwiz Forms does not create accounts in Books on the person's behalf.
6. **Fill provenance.** Values pulled from a Books account balance into a Buwiz Forms draft are tagged with fill
   source `document` and a `source_ref` pointing to the Books period and account, never `user`.

## Consequences

- Books stays country-neutral and compliance-free ([ADR 0007](0007-books-supplies-measures-forms-owns-compliance.md)).
- COA nesting stays in Books' Accounts screen alone (no duplicate COA view in Tax hand-off).
- Adding a Books measure rule for a form later supersedes any fallback account mapping for that box.
