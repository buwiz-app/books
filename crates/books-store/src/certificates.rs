//! M6: the certificates customers send (2307, and the final-tax 2306). A
//! certificate is a posted line with `document_kind` `certificate_2307` or
//! `certificate_2306`; its status changes later (the paper arrives, or is
//! lost) without touching the posted line, as a `certificate_status` row.
//! The latest row is the status the bridge reads. Only a received 2307
//! counts toward a form; a 2306 never does (the bridge decides that, from
//! the facts; this module only records them).

use std::collections::HashMap;

use books_core::date::Date;
use books_core::ledger::Status;
use rusqlite::{Connection, params};

use crate::books::{self, Book};
use crate::entries::{self, Filter};
use crate::people;
use crate::{Result, error, now_iso};

pub const STATUSES: [&str; 4] = ["received", "pending", "lost", "disputed"];

#[derive(Debug, Clone)]
pub struct Certificate {
    pub line_id: i64,
    pub entry_id: i64,
    pub date: Date,
    /// `2307` or `2306`.
    pub form: &'static str,
    /// `pt` (percentage tax) or `it` (income tax), as the line records it.
    pub credit_kind: String,
    /// The ATC printed on the paper, as recorded.
    pub atc: Option<String>,
    pub period_start: Option<Date>,
    pub period_end: Option<Date>,
    pub centavos: i64,
    pub status: String,
    /// Who sent it: the entry's person, else its payee text.
    pub from: Option<String>,
    /// The entry was voided (with its reversal it nets to nothing).
    pub voided: bool,
}

/// The latest recorded status of each certificate line of a book, by
/// `(entry_id, line_no)`: a void's reversal mirrors its original line by
/// line, so it reads the original's status from the same key.
pub fn statuses(conn: &Connection, book_id: i64) -> Result<HashMap<(i64, i64), String>> {
    let mut stmt = conn.prepare(
        "SELECT l.entry_id, l.line_no, c.status FROM certificate_status c
         JOIN line l ON l.line_id = c.line_id JOIN entry e ON e.entry_id = l.entry_id
         WHERE e.book_id = ?1 ORDER BY c.seq",
    )?;
    let rows = stmt.query_map(params![book_id], |r| {
        Ok((
            (r.get::<_, i64>(0)?, r.get::<_, i64>(1)?),
            r.get::<_, String>(2)?,
        ))
    })?;
    let mut out = HashMap::new();
    for row in rows {
        let (at, status) = row?;
        out.insert(at, status);
    }
    Ok(out)
}

/// Every certificate line on a posted entry of the book, newest first.
pub fn list(conn: &Connection, book_id: i64) -> Result<Vec<Certificate>> {
    let overlay = statuses(conn, book_id)?;
    let accounts: HashMap<i64, books::Account> = books::accounts(conn, book_id)?
        .into_iter()
        .map(|a| (a.id, a))
        .collect();
    let mut out = Vec::new();
    for e in entries::list(conn, book_id, &Filter::default())? {
        // a reversal mirrors its original; the original's row is the certificate
        if !matches!(e.status, Status::Posted | Status::Voided) || e.reverses.is_some() {
            continue;
        }
        let mut from: Option<Option<String>> = None;
        for l in entries::lines(conn, e.id)? {
            let Some(a) = accounts.get(&l.account_id) else {
                continue;
            };
            let f = a.kind.facts_for(&l.own);
            let form = match f.document_kind.as_deref() {
                Some("certificate_2307") => "2307",
                Some("certificate_2306") => "2306",
                _ => continue,
            };
            if from.is_none() {
                let person = match people::links(conn, e.id)?.0 {
                    Some(p) => people::person(conn, p)?.map(|p| p.name),
                    None => None,
                };
                from = Some(person.or_else(|| e.payee.clone()));
            }
            let status = overlay
                .get(&(e.id, l.no))
                .cloned()
                .or(f.certificate_status.clone())
                .unwrap_or_else(|| "pending".into());
            out.push(Certificate {
                line_id: l.id,
                entry_id: e.id,
                date: e.date,
                form,
                credit_kind: f.credit_kind.clone().unwrap_or_else(|| "none".into()),
                atc: f.certificate_atc.clone(),
                period_start: f.period_start.as_deref().and_then(Date::parse),
                period_end: f.period_end.as_deref().and_then(Date::parse),
                centavos: l.debit.max(l.credit),
                status,
                from: from.clone().flatten(),
                voided: e.status == Status::Voided,
            });
        }
    }
    out.sort_by(|a, b| b.date.cmp(&a.date).then(b.line_id.cmp(&a.line_id)));
    Ok(out)
}

/// Record a certificate's new status (the paper arrived, was lost, ...).
pub fn set_status(conn: &Connection, book: &Book, line_id: i64, status: &str) -> Result<()> {
    if !STATUSES.contains(&status) {
        return Err(error(
            "invalid_args",
            "A certificate is received, pending, lost or disputed.",
        ));
    }
    let found = list(conn, book.id)?
        .into_iter()
        .find(|c| c.line_id == line_id);
    let Some(c) = found else {
        return Err(error("not_found", "That certificate is not in this book."));
    };
    if c.voided {
        return Err(error("voided", "That certificate's entry is voided."));
    }
    if c.status == status {
        return Ok(());
    }
    conn.execute(
        "INSERT INTO certificate_status (line_id, status, changed_at) VALUES (?1, ?2, ?3)",
        params![line_id, status, now_iso()],
    )?;
    Ok(())
}
