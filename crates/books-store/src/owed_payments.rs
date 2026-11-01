//! Settling unpaid documents in parts (M10 parity, unpaid-1).
//!
//! Each settlement posts one money entry and keeps one `owed_payment` row
//! linked to it. What is paid to date counts only rows whose entry is still
//! posted, so a payment voided in the Ledger puts the balance back. A
//! document is `paid` once its balance reaches zero.

use books_core::catalog::Facts;
use books_core::date::Date;
use books_core::ledger::{Basis, KindRef};
use rusqlite::{Connection, params};

use crate::books::{self, Book};
use crate::entries::{self, DraftInput, Form, Gate, LineInput};
use crate::people::{self, sales_facts, trade_account};
use crate::{Result, error, now_iso};

/// One settlement of a document.
#[derive(Debug, Clone)]
pub struct Payment {
    pub id: i64,
    pub unpaid_id: i64,
    pub amount: i64,
    pub on: Date,
    pub money_account_id: i64,
    pub entry_id: i64,
    /// The payment's entry is no longer posted (voided in the Ledger).
    pub voided: bool,
}

/// What a settlement did: the entry it posted and the balance left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settled {
    pub entry_id: i64,
    pub balance: i64,
}

/// A document's payments, oldest first, voided ones included.
pub fn payments(conn: &Connection, unpaid_id: i64) -> Result<Vec<Payment>> {
    let mut stmt = conn.prepare(
        "SELECT p.payment_id, p.unpaid_id, p.amount, p.paid_on, p.money_account_id, p.entry_id, e.status
           FROM owed_payment p JOIN entry e ON e.entry_id = p.entry_id
          WHERE p.unpaid_id = ?1
          ORDER BY p.paid_on, p.payment_id",
    )?;
    let rows = stmt.query_map(params![unpaid_id], |r| {
        let on: String = r.get(3)?;
        Ok(Payment {
            id: r.get(0)?,
            unpaid_id: r.get(1)?,
            amount: r.get(2)?,
            on: Date::parse(&on).unwrap_or(Date {
                year: 1970,
                month: 1,
                day: 1,
            }),
            money_account_id: r.get(4)?,
            entry_id: r.get(5)?,
            voided: r.get::<_, String>(6)? != "posted",
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Paid to date: the payments whose entry is still posted.
pub fn paid(conn: &Connection, unpaid_id: i64) -> Result<i64> {
    let sum: i128 = payments(conn, unpaid_id)?
        .iter()
        .filter(|p| !p.voided)
        .map(|p| i128::from(p.amount))
        .sum();
    i64::try_from(sum).map_err(|_| error("store", "The payments are too large to add up."))
}

/// Settle `amount` of a document (the whole balance when `None`) on `on`
/// from `money_account`. Posts the money entry that day; the document is
/// paid once nothing is left.
pub fn settle(
    conn: &Connection,
    book: &Book,
    id: i64,
    money_account: i64,
    on: Date,
    amount: Option<i64>,
    gate: Gate<'_>,
) -> Result<Settled> {
    let u = people::unpaid(conn, id)?
        .filter(|u| u.book_id == book.id)
        .ok_or_else(|| error("not_found", "That document is not in this book."))?;
    if u.status == "cancelled" {
        return Err(error("cancelled", "That document is cancelled."));
    }
    let paid = paid(conn, id)?;
    let balance = u.amount - paid;
    if balance <= 0 {
        return Err(error(
            "already_settled",
            "That document is already settled.",
        ));
    }
    let amount = amount.unwrap_or(balance);
    if amount <= 0 {
        return Err(error("bad_amount", "The amount must be more than zero."));
    }
    if amount > balance {
        return Err(error(
            "over_balance",
            "The amount is more than what is still owed.",
        ));
    }
    if on < u.issued {
        return Err(error("invalid_args", "It cannot be paid before its date."));
    }
    gate(on)?;
    let money = books::account(conn, money_account)?
        .filter(|a| a.book_id == book.id && a.active)
        .ok_or_else(|| error("invalid_args", "That account is not in this book."))?;
    if !money.is_cash() && money.kind != KindRef::Kind("loan_payable".into()) {
        return Err(error(
            "invalid_args",
            format!("{} is not a bank, cash or card account.", money.name),
        ));
    }
    let category = books::account(conn, u.category_account_id)?
        .ok_or_else(|| error("store", "The document's category is missing."))?;
    // cash: the income or expense lands now; accrual: the trade balance clears
    let other = if book.basis == Basis::Accrual {
        trade_account(conn, book, &u.side)?
    } else {
        category.id
    };
    let facts = if other == category.id {
        sales_facts(&category.kind, &u.side)
    } else {
        Facts::default()
    };
    let money_line = |debit, credit| LineInput {
        account_id: money.id,
        debit,
        credit,
        facts: Facts::default(),
    };
    let other_line = |debit, credit, facts| LineInput {
        account_id: other,
        debit,
        credit,
        facts,
    };
    let (form, lines) = if u.side == "collect" {
        (
            Form::MoneyIn,
            vec![money_line(amount, 0), other_line(0, amount, facts)],
        )
    } else {
        (
            Form::MoneyOut,
            vec![other_line(amount, 0, facts), money_line(0, amount)],
        )
    };
    let payee = match u.person_id {
        Some(p) => people::person(conn, p)?.map(|p| p.name),
        None => None,
    };
    let draft = DraftInput {
        date: on,
        form,
        payee,
        memo: u.memo.clone(),
        reference: u.reference.clone(),
        needs_review: false,
        lines,
    };
    let entry = entries::save_draft(conn, book, None, &draft, "app")?;
    conn.execute(
        "UPDATE entry SET person_id = ?2 WHERE entry_id = ?1",
        params![entry, u.person_id],
    )?;
    entries::post(conn, book, entry, gate)?;
    conn.execute(
        "INSERT INTO owed_payment (book_id, unpaid_id, amount, paid_on, money_account_id, entry_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![book.id, id, amount, on.to_string(), money.id, entry, now_iso()],
    )?;
    let left = balance - amount;
    if left == 0 {
        conn.execute(
            "UPDATE unpaid SET status = 'paid', paid_entry_id = ?2, paid_on = ?3 WHERE unpaid_id = ?1",
            params![id, entry, on.to_string()],
        )?;
    }
    Ok(Settled {
        entry_id: entry,
        balance: left,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MIGRATIONS;

    /// A store at schema 18 with a bill marked paid the old way (one full
    /// entry, `status = 'paid'`) gains one payment row from 0019's backfill.
    #[test]
    fn migration_backfills_a_document_paid_before_payments() {
        let conn = Connection::open_in_memory().unwrap();
        for (n, sql) in MIGRATIONS.iter().filter(|m| m.0 < 19) {
            conn.execute_batch(sql)
                .unwrap_or_else(|e| panic!("migration {n}: {e}"));
        }
        let book_id = books::create(&conn, "Backfill Test", Basis::Cash, 1).unwrap();
        let book = books::get(&conn, book_id).unwrap().unwrap();
        let bank = books::account_by_number(&conn, book_id, "11000")
            .unwrap()
            .unwrap();
        let expense = books::account_by_number(&conn, book_id, "67200")
            .unwrap()
            .unwrap();
        conn.execute(
            "INSERT INTO unpaid (book_id, side, reference, issued, amount, category_account_id, status, created_at)
             VALUES (?1, 'pay', 'Bill SYN-0001', '2026-10-01', 250000, ?2, 'open', '2026-10-01T00:00:00Z')",
            params![book_id, expense.id],
        )
        .unwrap();
        let unpaid_id = conn.last_insert_rowid();
        let draft = DraftInput {
            date: Date::new(2026, 10, 5).unwrap(),
            form: Form::MoneyOut,
            payee: None,
            memo: None,
            reference: None,
            needs_review: false,
            lines: vec![
                LineInput {
                    account_id: expense.id,
                    debit: 250000,
                    credit: 0,
                    facts: sales_facts(&expense.kind, "pay"),
                },
                LineInput {
                    account_id: bank.id,
                    debit: 0,
                    credit: 250000,
                    facts: Facts::default(),
                },
            ],
        };
        let entry = entries::save_draft(&conn, &book, None, &draft, "app").unwrap();
        entries::post(&conn, &book, entry, &|_| Ok(())).unwrap();
        conn.execute(
            "UPDATE unpaid SET status = 'paid', paid_entry_id = ?2, paid_on = '2026-10-05' WHERE unpaid_id = ?1",
            params![unpaid_id, entry],
        )
        .unwrap();
        for (_, sql) in MIGRATIONS.iter().filter(|m| m.0 >= 19) {
            conn.execute_batch(sql).unwrap();
        }
        let list = payments(&conn, unpaid_id).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].amount, 250000);
        assert_eq!(list[0].money_account_id, bank.id);
        assert_eq!(list[0].entry_id, entry);
        assert_eq!(list[0].on, Date::new(2026, 10, 5).unwrap());
        assert!(!list[0].voided);
        assert_eq!(paid(&conn, unpaid_id).unwrap(), 250000);
        let u = people::unpaid(&conn, unpaid_id).unwrap().unwrap();
        assert_eq!(u.balance(), 0);
        // the backfill runs once per entry
        let again: i64 = conn
            .query_row("SELECT count(*) FROM owed_payment", [], |r| r.get(0))
            .unwrap();
        assert_eq!(again, 1);
    }
}
