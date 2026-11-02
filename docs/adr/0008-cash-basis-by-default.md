# ADR 0008: Cash basis by default; the accounting basis is a book setting

Status: Accepted (owner decision, binding, 2026-10-29).

## Decision

1. **Every business profile has an accounting basis, `cash` or `accrual`. The default is `cash`.** It is an
   accounting policy of the book, not a tax election, and it is not read from Forms.
2. **Cash basis:**
   - Income is posted when money is received and expenses when they are paid. An entry's date is the day money
     moved.
   - An unpaid invoice or bill is a document, not a journal entry.
   - The book refuses lines on trade receivable and payable accounts: categories with `accrual_only: true` in
     `data/categories.v1.json` (`accounts_receivable_gross`, `accounts_payable`).
   - A cash sale's sales line carries the invoice it settles (`document_kind: sale_invoice`). A sales line tagged
     `payment_receipt` is refused, so it is never silently dropped from the sales measures
     (`EXC.BANK_DEPOSIT_NOT_SALES`).
   - Payroll liabilities, statutory withholdings and tax credits are allowed. Payroll posts on the pay date.
3. **Accrual basis:** income posts at invoice and expenses at bill, against receivables and payables.
4. **Measures read the entry date on either basis.** The basis decides which entries the book posts, never how a
   measure reads them.
5. **Every proposal records `accounting_basis`, and the inputs hash covers it.** Changing the basis makes stored
   proposals stale. Whether that basis suits a given return is Forms' call ([ADR 0007](0007-books-supplies-measures-forms-owns-compliance.md)).
6. **No per-line recognition override.** With the basis explicit, no line needs one.

## Consequences

- A basis change takes effect from the first day of a year with no posted entries. A switch from accrual to cash
  also needs zero trade receivable and payable balances, so a prior-year invoice is never counted twice.
- A customer who withholds tax pays net: the receipt entry debits the bank for the net amount and the tax credit for
  the withheld amount, and credits sales for the gross. Certificate status is evidence on the evidence record.
- Reports follow the book's basis.
