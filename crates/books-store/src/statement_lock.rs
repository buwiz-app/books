//! M10 parity: a finished statement locks the entries matched to it. A
//! void of such an entry is refused until the statement is reopened, with
//! a reason, which keeps every match and writes a `statement_event`.

use books_core::date::{MONTHS, Period};
use rusqlite::{Connection, OptionalExtension as _, params};

use crate::books::Book;
use crate::entries::check_text;
use crate::{Result, error, now_iso};

/// Refuse a change to an entry matched to a finished statement of the book.
pub fn guard_entry(conn: &Connection, book_id: i64, entry_id: i64) -> Result<()> {
    let key: Option<String> = conn
        .query_row(
            "SELECT s.period_key FROM statement_line l JOIN statement s ON s.statement_id = l.statement_id
             WHERE l.matched_entry_id = ?2 AND s.book_id = ?1 AND s.status = 'finished'
             ORDER BY s.period_key LIMIT 1",
            params![book_id, entry_id],
            |r| r.get(0),
        )
        .optional()?;
    match key {
        Some(key) => Err(error(
            "statement_finished",
            format!(
                "This entry is matched to the bank in a finished month ({}). Reopen that month's statement in Reconcile first.",
                short_label(&key)
            ),
        )),
        None => Ok(()),
    }
}

/// "Oct 2026" for "2026-10".
fn short_label(key: &str) -> String {
    Period::parse(key).map_or_else(
        || key.to_string(),
        |p| {
            format!(
                "{} {}",
                MONTHS[usize::from(p.start.month - 1)],
                p.start.year
            )
        },
    )
}

/// Record that a statement was finished.
pub fn record_finish(conn: &Connection, statement_id: i64) -> Result<()> {
    conn.execute(
        "INSERT INTO statement_event (statement_id, event, reason, at) VALUES (?1, 'finish', NULL, ?2)",
        params![statement_id, now_iso()],
    )?;
    Ok(())
}

/// Reopen a finished statement with a reason of 3 to 200 characters. Every
/// match stays; the statement can be finished again.
pub fn reopen(
    conn: &Connection,
    book: &Book,
    account_id: i64,
    period_key: &str,
    reason: &str,
) -> Result<()> {
    let found: Option<(i64, String)> = conn
        .query_row(
            "SELECT statement_id, status FROM statement WHERE book_id = ?1 AND account_id = ?2 AND period_key = ?3",
            params![book.id, account_id, period_key],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((id, status)) = found else {
        return Err(error(
            "not_found",
            "There is no statement for that account and month.",
        ));
    };
    if status != "finished" {
        return Err(error(
            "not_finished",
            "That month's statement is not finished.",
        ));
    }
    let reason = check_text("reason", Some(reason), 200)?
        .filter(|r| r.chars().count() >= 3)
        .ok_or_else(|| {
            error(
                "reason_needed",
                "Reopening a statement needs a reason of at least 3 characters.",
            )
        })?;
    let now = now_iso();
    conn.execute(
        "UPDATE statement SET status = 'open', finished_at = NULL WHERE statement_id = ?1",
        params![id],
    )?;
    conn.execute(
        "INSERT INTO statement_event (statement_id, event, reason, at) VALUES (?1, 'reopen', ?2, ?3)",
        params![id, reason, now],
    )?;
    Ok(())
}

/// A statement's events, oldest first: (event, at). Reasons stay in the store.
pub fn events(conn: &Connection, statement_id: i64) -> Result<Vec<(String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT event, at FROM statement_event WHERE statement_id = ?1 ORDER BY event_id",
    )?;
    let rows = stmt.query_map(params![statement_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}
