# ADR 0008: Cash basis by default; the accounting basis is a book setting

Status: Accepted (owner decision, binding, 2026-10-29). Replaces the open decision D4 and the `period_override` line attribute.
Date: 2026-10-29.

## Context

- D4 asked which date decides a sale's period: the entry, receipt or invoice date. That is the accounting basis by another name. Accrual counts a sale on its invoice date; cash counts it on the day the money arrives. The earlier recommendation ("entry date, with a `period_override`") left the basis implicit and added a per-line override knob.
- The TypeScript Books repo has the setting in name only. `src/db/validation/reports.ts:4` defines `accountingBasisEnum = ["accrual", "cash"]` as a per-report parameter defaulting to `"accrual"`, and the report handlers parse it and drop it (`src/routes/api/-reports.ts:59, 68`). Invoices post a receivable on send (`src/routes/api/-invoices.ts:661`). So that repo is always accrual, whatever the parameter says.
- The owner's decision (2026-10-29): "default should always cash as that is the simplest". Recorded in [`research/traces/decisions.md`](../../research/traces/decisions.md), addendum "Owner decisions D4 and D9".
- ADR 0007 already split D4. Which basis a tax return needs (the tax timing basis for 2551Q, for example) is Forms'. The basis the books are kept on is Books'.

## Decision

1. **Every business profile has an accounting basis: `cash` or `accrual`. The default is `cash`.** It is an accounting policy of the book, not a tax election, and it is not read from Forms.
2. **Cash basis:**
   - Income is posted on the day the money is received, and expenses on the day they are paid. An entry's date is the day money moved.
   - An unpaid invoice or bill is a document, not a journal entry.
   - The book therefore refuses lines on trade receivable and payable accounts (categories with `accrual_only: true` in [`data/categories.v1.json`](../../data/categories.v1.json): `accounts_receivable_gross`, `accounts_payable`).
   - The sales line of a cash sale carries the invoice it settles (`document_kind: sale_invoice`). A sales line tagged `payment_receipt` is refused, so it is never silently dropped from the sales measures (`EXC.BANK_DEPOSIT_NOT_SALES`).
   - Payroll liabilities, statutory withholdings and tax credits are allowed. Payroll is posted on the pay date.
3. **Accrual basis:** income is posted at invoice and expenses at bill, against receivables and payables. Collections and payments clear them.
4. **Measures read the entry date on either basis.** The basis decides which entries the book posts, never how a measure reads them. On a cash-basis book, `gross_sales` for a quarter is the gross amount received in that quarter. [`data/measures.v1.json`](../../data/measures.v1.json) says so at the top level.
5. **Every proposal records `accounting_basis`, and the inputs hash covers it.** A change of basis makes stored proposals stale. Forms and the user can see which basis a figure is on. Whether that basis suits a given return is Forms' call (ADR 0007).
6. **`period_override` is removed.** With the basis explicit, no line needs a recognition override. The attribute is gone from [`data/tax-attributes.v1.json`](../../data/tax-attributes.v1.json) and the journal-line model.

## Consequences

- The PoC enforces the cash rule when it loads an input (`poc/src/load.ts`, `checkCashBasis`). P1 enforces the same rule at post time in `books-core`.
- Fixtures state their basis explicitly. Cash: `c`, `c-ack`, `d`, `e` and `f` (Slice A). Accrual: `a`, `b` and `g`, which post 2307 certificates against receivables. The Slice A numbers are unchanged. Tests: `poc/test/accounting-basis.test.ts`.
- **Changing the basis (P1 rule, not in the PoC):** a change takes effect from the first day of a year that has no posted entries. A switch from accrual to cash also needs zero trade receivable and payable balances at the switch date. Without that, a prior-year invoice collected after the switch would be counted twice: once at invoice, again at receipt.
- **Certificates on a cash book (P4):**
  - A customer who withholds tax pays net. The withheld part is treated as received at payment. The receipt entry debits the bank for the net amount and the tax credit for the withheld amount, and credits sales for the gross.
  - Certificate status (received, pending) is evidence. P4 keeps it on the evidence record, so a certificate that arrives later does not need the posted line changed.
  - The accrual fixtures keep the certificate entries against receivables until P4.
- Reports (P1 and later) follow the book's basis. A report on the other basis is a later feature, not a toggle that silently does nothing, as it does in the TypeScript repo.

## Alternatives considered

| Option | Why not |
|---|---|
| Accrual by default, cash-basis figures derived from collections | More moving parts: allocating payments to invoices, partial payments, credit notes against part-paid invoices. The owner chose the simplest default. Accrual stays available. |
| Supply both bases as separate measures (sales by invoice date and sales by collection date) | Needs both invoice and collection entries on every book. That is accrual bookkeeping. One basis per book is simpler, and the proposal says which one. |
| Keep `period_override` | A per-line override hides the policy and invites inconsistent periods. The basis covers the real need. |
