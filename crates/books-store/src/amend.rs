//! Amend by reversal: a posted entry is never edited. An amend voids it
//! (its mirrored reversal posts, [`entries::void`]) and saves a draft
//! replacement with the same lines, linked both ways, for the user to
//! correct and post. The reason is stored in `amendment` only.

use books_core::date::Date;
use books_core::ledger::Status;
use rusqlite::{Connection, params};

use crate::books::Book;
use crate::entries::{self, DraftInput, Form, Gate, LineInput};
use crate::{Result, error, now_iso, people};

/// The reason an amend gives: 3 to 200 characters, no TIN.
pub fn check_reason(reason: &str) -> Result<String> {
    let reason = entries::check_text("reason", Some(reason), 200)?.unwrap_or_default();
    if reason.chars().count() < 3 {
        return Err(error(
            "invalid_args",
            "An amend needs a reason of at least 3 characters.",
        ));
    }
    Ok(reason)
}

/// Amend posted entry `id`: void it with a reversal dated `date` and save
/// a replacement draft dated `date`. Returns (reversal, replacement).
/// Runs inside the caller's transaction; both months must be open.
pub fn amend(
    conn: &Connection,
    book: &Book,
    id: i64,
    date: Date,
    reason: &str,
    gate: Gate<'_>,
) -> Result<(i64, i64)> {
    let reason = check_reason(reason)?;
    let original = entries::get(conn, id)?
        .filter(|e| e.book_id == book.id)
        .ok_or_else(|| error("not_found", "That entry is not in this book."))?;
    if original.amended_by.is_some() {
        return Err(error("already_amended", "That entry is already amended."));
    }
    if original.status == Status::Posted && original.form != Form::Reversal {
        // the original's month is checked too: a closed month never changes
        gate(original.date)?;
    }
    let reversal = entries::void(conn, book, id, date, gate)?;
    let lines: Vec<LineInput> = entries::lines(conn, id)?
        .into_iter()
        .map(|l| LineInput {
            account_id: l.account_id,
            debit: l.debit,
            credit: l.credit,
            facts: l.own,
        })
        .collect();
    let input = DraftInput {
        date,
        form: original.form,
        payee: original.payee.clone(),
        memo: original.memo.clone(),
        reference: original.reference.clone(),
        needs_review: false,
        lines,
    };
    let replacement = entries::save_draft(conn, book, None, &input, "app")?;
    let (person, department, location) = people::links(conn, id)?;
    people::set_entry_links(conn, book, replacement, person, department, location)?;
    conn.execute(
        "UPDATE entry SET amends_entry_id = ?2 WHERE entry_id = ?1",
        params![replacement, id],
    )?;
    conn.execute(
        "UPDATE entry SET amended_by_entry_id = ?2 WHERE entry_id = ?1",
        params![id, replacement],
    )?;
    conn.execute(
        "INSERT INTO amendment (original_entry_id, reversal_entry_id, replacement_entry_id, reason, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, reversal, replacement, reason, now_iso()],
    )?;
    Ok((reversal, replacement))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Paths, Store, TempDir, books};
    use books_core::catalog::Facts;
    use books_core::ledger::Basis;

    const OPEN: Gate<'static> = &|_| Ok(());

    fn posted(tag: &str) -> (TempDir, Store, Book, i64) {
        let tmp = TempDir::new(tag);
        let store = Store::open(&Paths::in_dir(tmp.0.join("d"))).unwrap();
        let c = store.conn();
        let bid = books::create(c, "Test Book", Basis::Cash, 1).unwrap();
        let book = books::get(c, bid).unwrap().unwrap();
        let acct = |n: &str| books::account_by_number(c, bid, n).unwrap().unwrap().id;
        let input = DraftInput {
            date: Date::new(2026, 10, 28).unwrap(),
            form: Form::MoneyIn,
            payee: Some("Bayani Foods".into()),
            memo: Some("MEMO-SYNTH-sale".into()),
            reference: Some("Invoice 0142".into()),
            needs_review: false,
            lines: vec![
                LineInput {
                    account_id: acct("11000"),
                    debit: 50_000,
                    credit: 0,
                    facts: Facts::default(),
                },
                LineInput {
                    account_id: acct("41000"),
                    debit: 0,
                    credit: 50_000,
                    facts: Facts::default(),
                },
            ],
        };
        let id = entries::save_draft(c, &book, None, &input, "app").unwrap();
        entries::post(c, &book, id, OPEN).unwrap();
        (tmp, store, book, id)
    }

    #[test]
    fn an_amend_voids_reverses_and_links_a_replacement_draft() {
        let (_t, store, book, id) = posted("amend");
        let c = store.conn();
        let date = Date::new(2026, 10, 30).unwrap();
        let (rev, rep) = amend(c, &book, id, date, "wrong category", OPEN).unwrap();
        let original = entries::get(c, id).unwrap().unwrap();
        assert_eq!(
            (original.status, original.voided_by, original.amended_by),
            (Status::Voided, Some(rev), Some(rep))
        );
        let replacement = entries::get(c, rep).unwrap().unwrap();
        assert_eq!(
            (replacement.status, replacement.amends, replacement.date),
            (Status::Draft, Some(id), date)
        );
        let (a, b) = (
            entries::lines(c, id).unwrap(),
            entries::lines(c, rep).unwrap(),
        );
        assert_eq!(
            a.iter()
                .map(|l| (l.account_id, l.debit, l.credit))
                .collect::<Vec<_>>(),
            b.iter()
                .map(|l| (l.account_id, l.debit, l.credit))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            amend(c, &book, id, date, "again please", OPEN)
                .unwrap_err()
                .code,
            "already_amended"
        );
        assert_eq!(
            amend(c, &book, rev, date, "a reversal", OPEN)
                .unwrap_err()
                .code,
            "is_reversal"
        );
        assert_eq!(
            amend(c, &book, rep, date, "a draft", OPEN)
                .unwrap_err()
                .code,
            "not_posted"
        );
    }

    #[test]
    fn a_short_reason_or_a_closed_month_is_refused() {
        let (_t, store, book, id) = posted("amend-refuse");
        let c = store.conn();
        let date = Date::new(2026, 10, 28).unwrap();
        assert_eq!(
            amend(c, &book, id, date, " ab ", OPEN).unwrap_err().code,
            "invalid_args"
        );
        let closed: Gate<'_> = &|_| Err(error("period_closed", "October is closed."));
        assert_eq!(
            amend(c, &book, id, date, "wrong category", closed)
                .unwrap_err()
                .code,
            "period_closed"
        );
        assert_eq!(entries::get(c, id).unwrap().unwrap().status, Status::Posted);
    }
}
