//! M7: hand-offs of a form's Books totals to the BIR draft (schema 6).
//!
//! The store keeps what was handed and when; whether a hand-off is still
//! current is the mini app's question (it recomputes the proposal and
//! compares the inputs hash). Books never reaches BIR: the assistant fills
//! the BIR draft from what `books_handoff_get` gives it, with source
//! `document` (D3), and the owner's click here confirms it (B5).

use rusqlite::{Connection, OptionalExtension as _, Row, params};

use crate::books::Book;
use crate::{Result, error, now_iso};

#[derive(Debug, Clone)]
pub struct Handoff {
    pub id: i64,
    pub book_id: i64,
    pub form: String,
    pub period_key: String,
    pub form_context: String,
    pub fill_args: String,
    pub inputs_hash: String,
    pub lineage: String,
    /// `handed`, `confirmed` or `superseded`.
    pub status: String,
    pub created_at: String,
    pub fill_args_sha256: Option<String>,
    pub confirmed_at: Option<String>,
}

const COLUMNS: &str = "handoff_id, book_id, form, period_key, form_context, fill_args, inputs_hash, lineage, status, created_at, fill_args_sha256, confirmed_at";

fn row(r: &Row<'_>) -> rusqlite::Result<Handoff> {
    Ok(Handoff {
        id: r.get(0)?,
        book_id: r.get(1)?,
        form: r.get(2)?,
        period_key: r.get(3)?,
        form_context: r.get(4)?,
        fill_args: r.get(5)?,
        inputs_hash: r.get(6)?,
        lineage: r.get(7)?,
        status: r.get(8)?,
        created_at: r.get(9)?,
        fill_args_sha256: r.get(10)?,
        confirmed_at: r.get(11)?,
    })
}

/// What a new hand-off records.
pub struct NewHandoff<'a> {
    pub form: &'a str,
    pub period_key: &'a str,
    pub form_context: &'a str,
    pub fill_args: &'a str,
    pub inputs_hash: &'a str,
    pub lineage: &'a str,
}

/// Record a hand-off; an earlier one for the same form and period is
/// superseded (it stays for the record).
pub fn create(conn: &Connection, book: &Book, h: &NewHandoff<'_>) -> Result<i64> {
    conn.execute(
        "UPDATE handoff SET status = 'superseded' WHERE book_id = ?1 AND form = ?2 AND period_key = ?3 AND status <> 'superseded'",
        params![book.id, h.form, h.period_key],
    )?;
    conn.execute(
        "INSERT INTO handoff (book_id, form, period_key, form_context, fill_args, inputs_hash, lineage, status, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'handed', ?8)",
        params![book.id, h.form, h.period_key, h.form_context, h.fill_args, h.inputs_hash, h.lineage, now_iso()],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get(conn: &Connection, id: i64) -> Result<Option<Handoff>> {
    Ok(conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM handoff WHERE handoff_id = ?1"),
            params![id],
            row,
        )
        .optional()?)
}

/// The current (not superseded) hand-off of a form and period.
pub fn current(
    conn: &Connection,
    book_id: i64,
    form: &str,
    period_key: &str,
) -> Result<Option<Handoff>> {
    Ok(conn
        .query_row(
            &format!(
                "SELECT {COLUMNS} FROM handoff WHERE book_id = ?1 AND form = ?2 AND period_key = ?3 AND status <> 'superseded' ORDER BY handoff_id DESC LIMIT 1"
            ),
            params![book_id, form, period_key],
            row,
        )
        .optional()?)
}

/// Every current hand-off of a book, newest first.
pub fn list(conn: &Connection, book_id: i64) -> Result<Vec<Handoff>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM handoff WHERE book_id = ?1 AND status <> 'superseded' ORDER BY handoff_id DESC"
    ))?;
    let rows = stmt.query_map(params![book_id], row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// The owner's click: BIR shows these values. Records the digest of what
/// was handed and the time.
pub fn confirm(conn: &Connection, book: &Book, id: i64, fill_args_sha256: &str) -> Result<()> {
    let h = get(conn, id)?
        .filter(|h| h.book_id == book.id)
        .ok_or_else(|| error("not_found", "That hand-off is not in this book."))?;
    match h.status.as_str() {
        "superseded" => Err(error("superseded", "A newer hand-off replaced this one.")),
        "confirmed" => Ok(()),
        _ => {
            conn.execute(
                "UPDATE handoff SET status = 'confirmed', fill_args_sha256 = ?2, confirmed_at = ?3 WHERE handoff_id = ?1",
                params![id, fill_args_sha256, now_iso()],
            )?;
            Ok(())
        }
    }
}
