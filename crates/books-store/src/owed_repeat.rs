//! Monthly repeats of unpaid documents (M10 parity, unpaid-3).
//!
//! A document marked to repeat monthly is a template. Each later month,
//! up to today's, may get one copy, made only when the user adds it: no
//! routine and no assistant writes a copy. A copy is a plain unpaid
//! document made through [`people::create_unpaid`], so an accrual book posts
//! its receivable or payable and the closed-month gate applies as usual.

use books_core::date::{Date, days_in_month};
use rusqlite::{Connection, OptionalExtension as _, params};

use crate::books::Book;
use crate::entries::Gate;
use crate::people::{self, UnpaidInput};
use crate::{Result, error};

/// A template's copy that today's month may still get.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Due {
    pub template_id: i64,
    pub issued: Date,
    pub due: Option<Date>,
}

struct Template {
    id: i64,
    book_id: i64,
    side: String,
    person_id: Option<i64>,
    reference: Option<String>,
    issued: Date,
    due: Option<Date>,
    amount: i64,
    category_account_id: i64,
    status: String,
    repeat: Option<String>,
    repeat_of: Option<i64>,
}

fn template(conn: &Connection, id: i64) -> Result<Option<Template>> {
    let date = |s: Option<String>| s.and_then(|s| Date::parse(&s));
    Ok(conn
        .query_row(
            "SELECT unpaid_id, book_id, side, person_id, reference, issued, due, amount,
                    category_account_id, status, repeat, repeat_of
             FROM unpaid WHERE unpaid_id = ?1",
            params![id],
            |r| {
                Ok(Template {
                    id: r.get(0)?,
                    book_id: r.get(1)?,
                    side: r.get(2)?,
                    person_id: r.get(3)?,
                    reference: r.get(4)?,
                    issued: date(r.get(5)?).unwrap_or(Date {
                        year: 1970,
                        month: 1,
                        day: 1,
                    }),
                    due: date(r.get(6)?),
                    amount: r.get(7)?,
                    category_account_id: r.get(8)?,
                    status: r.get(9)?,
                    repeat: r.get(10)?,
                    repeat_of: r.get(11)?,
                })
            },
        )
        .optional()?)
}

fn in_book(conn: &Connection, book_id: i64, id: i64) -> Result<Template> {
    template(conn, id)?
        .filter(|t| t.book_id == book_id)
        .ok_or_else(|| error("not_found", "That document is not in this book."))
}

/// `YYYY-MM` for a month.
pub fn month_key(year: i32, month: u8) -> String {
    format!("{year:04}-{month:02}")
}

/// A `YYYY-MM` month, if it is one.
pub fn parse_month(text: &str) -> Option<(i32, u8)> {
    let (y, m) = text.split_once('-')?;
    if y.len() != 4 || m.len() != 2 {
        return None;
    }
    let year: i32 = y.parse().ok()?;
    let month: u8 = m.parse().ok()?;
    (1..=12).contains(&month).then_some((year, month))
}

/// The copy's dates for a month: the template's day, clamped to the
/// month's length, and the template's due gap after it.
fn copy_dates(t: &Template, (year, month): (i32, u8)) -> (Date, Option<Date>) {
    let issued = Date {
        year,
        month,
        day: t.issued.day.min(days_in_month(year, month)),
    };
    let due = t
        .due
        .map(|d| issued.add_days(d.to_days() - t.issued.to_days()));
    (issued, due)
}

/// Whether a document repeats monthly.
pub fn repeats(conn: &Connection, id: i64) -> Result<bool> {
    Ok(template(conn, id)?.is_some_and(|t| t.repeat.as_deref() == Some("monthly")))
}

/// Start or stop a monthly repeat. Only an original that is not cancelled
/// repeats; a copy never does.
pub fn set(conn: &Connection, book: &Book, id: i64, on: bool) -> Result<()> {
    let t = in_book(conn, book.id, id)?;
    if t.repeat_of.is_some() {
        return Err(error(
            "is_copy",
            "This is a monthly copy; only the original document repeats.",
        ));
    }
    if on && t.status == "cancelled" {
        return Err(error("not_open", "A cancelled document cannot repeat."));
    }
    conn.execute(
        "UPDATE unpaid SET repeat = ?2 WHERE unpaid_id = ?1",
        params![id, on.then_some("monthly")],
    )?;
    Ok(())
}

/// The copies today's month may still get, oldest template first.
pub fn due(conn: &Connection, book_id: i64, today: Date) -> Result<Vec<Due>> {
    let month = month_key(today.year, today.month);
    let mut stmt = conn.prepare(
        "SELECT u.unpaid_id FROM unpaid u
         WHERE u.book_id = ?1 AND u.repeat = 'monthly' AND u.status <> 'cancelled'
           AND u.repeat_of IS NULL AND substr(u.issued, 1, 7) < ?2
           AND NOT EXISTS (SELECT 1 FROM unpaid c WHERE c.repeat_of = u.unpaid_id AND c.repeat_month = ?2)
         ORDER BY u.unpaid_id",
    )?;
    let ids = stmt
        .query_map(params![book_id, month], |r| r.get::<_, i64>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(t) = template(conn, id)? {
            let (issued, due) = copy_dates(&t, (today.year, today.month));
            out.push(Due {
                template_id: t.id,
                issued,
                due,
            });
        }
    }
    Ok(out)
}

/// Add a template's copy for `month`, which must be after the template's
/// month and not after today's. Returns the copy's id.
pub fn add(
    conn: &Connection,
    book: &Book,
    template_id: i64,
    month: (i32, u8),
    today: Date,
    gate: Gate<'_>,
) -> Result<i64> {
    let t = in_book(conn, book.id, template_id)?;
    if t.repeat.as_deref() != Some("monthly") || t.repeat_of.is_some() {
        return Err(error("not_repeating", "This document does not repeat."));
    }
    if t.status == "cancelled" {
        return Err(error("not_open", "A cancelled document makes no copies."));
    }
    if month <= (t.issued.year, t.issued.month) || month > (today.year, today.month) {
        return Err(error(
            "bad_month",
            "A copy is for a month after the document's, up to this month.",
        ));
    }
    let key = month_key(month.0, month.1);
    let exists = conn
        .query_row(
            "SELECT 1 FROM unpaid WHERE repeat_of = ?1 AND repeat_month = ?2",
            params![t.id, key],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if exists {
        return Err(error(
            "already_added",
            "That month's copy is already added.",
        ));
    }
    let (issued, due) = copy_dates(&t, month);
    let id = people::create_unpaid(
        conn,
        book,
        &UnpaidInput {
            side: &t.side,
            person_id: t.person_id,
            reference: t.reference.as_deref(),
            memo: None,
            issued,
            due,
            amount: t.amount,
            category_account_id: t.category_account_id,
        },
        gate,
    )?;
    conn.execute(
        "UPDATE unpaid SET repeat_of = ?2, repeat_month = ?3 WHERE unpaid_id = ?1",
        params![id, t.id, key],
    )?;
    Ok(id)
}
