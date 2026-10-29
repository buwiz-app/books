import { describe, expect, test } from "bun:test";
import { readdirSync } from "node:fs";
import { postLedger } from "../src/ledger.ts";
import { InputError, normalizeFixture } from "../src/load.ts";
import { computeMeasure } from "../src/measures.ts";
import { SYNTHETIC_DIR } from "../src/paths.ts";
import { freshness } from "../src/proposal.ts";
import { clone, data, fixture, memoryInput, proposal, PROFILE_A, rawFixture } from "./support.ts";

type Obj = Record<string, unknown>;

const Q3 = { start: "2026-07-01", end: "2026-09-30" };
const Q4 = { start: "2026-10-01", end: "2026-12-31" };

const salesLine = (lineId: string, amount: string, documentKind = "sale_invoice"): Obj => ({
  line_id: lineId,
  account: "sales_revenue",
  debit: "0.00",
  credit: amount,
  measure_side: "credit",
  income_class: "business_sales",
  document_kind: documentKind,
});

function twoLine(entryId: string, date: string, first: Obj, second: Obj): Obj {
  return { entry_id: entryId, tax_profile_id: PROFILE_A, entry_date: date, lines: [first, second] };
}

/** Invoice on the books: receivable debit, sales credit (accrual only). */
const invoiced = (entryId: string, date: string, amount: string): Obj =>
  twoLine(entryId, date, { line_id: `${entryId}.1`, account: "accounts_receivable_gross", debit: amount, credit: "0.00" }, salesLine(`${entryId}.2`, amount));

/** Collection of that invoice: bank debit, receivable credit (accrual only). */
const collected = (entryId: string, date: string, amount: string): Obj =>
  twoLine(
    entryId,
    date,
    { line_id: `${entryId}.1`, account: "bank_accounts", debit: amount, credit: "0.00" },
    { line_id: `${entryId}.2`, account: "accounts_receivable_gross", debit: "0.00", credit: amount },
  );

/** Sale posted when the money arrives: bank debit, sales credit. */
const received = (entryId: string, date: string, amount: string, documentKind = "sale_invoice"): Obj =>
  twoLine(entryId, date, { line_id: `${entryId}.1`, account: "bank_accounts", debit: amount, credit: "0.00" }, salesLine(`${entryId}.2`, amount, documentKind));

/** Supplier bill on the books: expense debit, payable credit (accrual only). */
const billed = (entryId: string, date: string, amount: string): Obj =>
  twoLine(
    entryId,
    date,
    { line_id: `${entryId}.1`, account: "rent", debit: amount, credit: "0.00", measure_side: "debit", income_class: "operating_expense", document_kind: "expense_document" },
    { line_id: `${entryId}.2`, account: "accounts_payable", debit: "0.00", credit: amount },
  );

describe("the accounting basis is a book setting; cash is the default (ADR 0008)", () => {
  test("a profile that does not state a basis is a cash-basis book", () => {
    const raw = clone(rawFixture("f-slice-a-2026"));
    delete raw.profiles[0].accounting_basis;
    expect(normalizeFixture(raw, data.vocab).profiles[0].accounting_basis).toBe("cash");
  });

  test("only cash or accrual is accepted", () => {
    const raw = clone(rawFixture("f-slice-a-2026"));
    raw.profiles[0].accounting_basis = "modified_cash";
    expect(() => normalizeFixture(raw, data.vocab)).toThrow(InputError);
  });

  test("every synthetic fixture states its basis, so the default never decides a fixture silently", () => {
    for (const name of readdirSync(SYNTHETIC_DIR).filter((n) => n.endsWith(".json"))) {
      for (const profile of rawFixture(name.replace(/\.json$/, "")).profiles) {
        expect(["cash", "accrual"], name).toContain(profile.accounting_basis);
      }
    }
  });
});

describe("a cash-basis book posts income when money is received and expenses when they are paid", () => {
  test("it refuses a trade receivable line: an unpaid invoice is not posted", () => {
    expect(() => memoryInput({ entries: [invoiced("I-1", "2026-09-25", "50000.00")] })).toThrow(/accrual account/);
  });

  test("it refuses a trade payable line: an unpaid bill is not posted", () => {
    expect(() => memoryInput({ entries: [billed("B-1", "2026-09-25", "8000.00")] })).toThrow(/accrual account/);
  });

  test("it refuses a sales line tagged payment_receipt instead of dropping it from the sales measures", () => {
    expect(() => memoryInput({ entries: [received("R-1", "2026-10-05", "50000.00", "payment_receipt")] })).toThrow(/sale_invoice/);
  });

  test("an accrual book accepts the same receivable and payable lines", () => {
    const input = memoryInput({ basis: "accrual", entries: [invoiced("I-1", "2026-09-25", "50000.00"), billed("B-1", "2026-09-25", "8000.00")] });
    expect(input.profiles[0].accounting_basis).toBe("accrual");
  });

  test("worked example: invoiced 2026-09-25, paid 2026-10-05; accrual counts it in Q3, cash in Q4", () => {
    const accrual = postLedger(
      memoryInput({ basis: "accrual", entries: [invoiced("I-1", "2026-09-25", "50000.00"), collected("I-2", "2026-10-05", "50000.00")] }).entries,
    );
    const cash = postLedger(memoryInput({ entries: [received("R-1", "2026-10-05", "50000.00")] }).entries);

    expect(computeMeasure("gross_sales", accrual, Q3).value).toBe(5_000_000n);
    expect(computeMeasure("gross_sales", accrual, Q4).value).toBe(0n);
    expect(computeMeasure("gross_sales", cash, Q3).value).toBe(0n);
    expect(computeMeasure("gross_sales", cash, Q4).value).toBe(5_000_000n);
    expect(computeMeasure("gross_sales", cash, Q4).lines.map((l) => l.line.line_id)).toEqual(["R-1.2"]);
  });
});

describe("the proposal carries the basis, and the basis is covered by staleness", () => {
  test("the Slice A proposal says cash", () => {
    const p = proposal(fixture("f-slice-a-2026"), "2551Q", "2026-Q3");
    expect(p.accounting_basis).toBe("cash");
    expect(p.status).toBe("ready");
  });

  test("an accrual fixture's proposal says accrual", () => {
    expect(proposal(fixture("g-pt-withholding-2026"), "2551Q", "2026-Q3").accounting_basis).toBe("accrual");
  });

  test("changing the book's basis makes a stored proposal stale", () => {
    const input = fixture("f-slice-a-2026");
    const stored = proposal(input, "2551Q", "2026-Q3");
    expect(freshness(stored, data, input)).toBe("fresh");
    const switched = clone(input);
    switched.profiles[0].accounting_basis = "accrual";
    expect(freshness(stored, data, switched)).toBe("stale");
  });
});
