//! Statements the assistant read (M10 parity reconcile-1): drafted lines
//! with their page, box and confidence, kept apart from the statement until
//! the user accepts them in the app. Books stores what it is given; it never
//! reads the page itself. Money is integer centavos.

use books_core::date::Date;
use rusqlite::{Connection, OptionalExtension as _, params};

use crate::{Result, error, now_iso};

/// A box on a page, in thousandths of the page: `[x0, y0, x1, y1]`.
pub type Bbox = [i64; 4];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftLine {
    pub line_no: i64,
    pub date: Date,
    pub description: String,
    pub amount: i64,
    pub balance: Option<i64>,
    pub page: Option<i64>,
    pub bbox: Option<Bbox>,
    pub confidence: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct Draft {
    pub id: i64,
    pub book_id: i64,
    pub account_id: i64,
    pub period_key: String,
    pub closing: Option<i64>,
    pub evidence_id: Option<i64>,
    pub proposed_by: String,
    pub status: String,
    pub lines: Vec<DraftLine>,
}

/// Where a stored statement line sits on its page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Extra {
    pub balance: Option<i64>,
    pub page: Option<i64>,
    pub bbox: Option<Bbox>,
    pub confidence: Option<i64>,
}

pub struct NewDraft<'a> {
    pub book_id: i64,
    pub account_id: i64,
    pub period_key: &'a str,
    pub closing: Option<i64>,
    pub evidence_id: Option<i64>,
    pub proposed_by: &'a str,
    pub lines: &'a [DraftLine],
}

/// A box is four thousandths with `x0 < x1` and `y0 < y1`.
pub fn bbox_ok(b: &Bbox) -> bool {
    b.iter().all(|v| (0..=1000).contains(v)) && b[0] < b[2] && b[1] < b[3]
}

/// Consecutive lines whose running balances disagree with the amount:
/// `prev.balance + amount != balance` (lines without a balance are skipped).
pub fn balance_breaks(lines: &[(i64, Option<i64>)]) -> usize {
    lines
        .windows(2)
        .filter(|w| match (w[0].1, w[1].1) {
            (Some(prev), Some(now)) => i128::from(prev) + i128::from(w[1].0) != i128::from(now),
            _ => false,
        })
        .count()
}

fn status_of(conn: &Connection, book: i64, account: i64, key: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT status FROM statement WHERE book_id = ?1 AND account_id = ?2 AND period_key = ?3",
            params![book, account, key],
            |r| r.get(0),
        )
        .optional()?)
}

/// Store a draft; an older pending draft for the same month is replaced.
pub fn insert(conn: &Connection, d: &NewDraft<'_>) -> Result<i64> {
    if status_of(conn, d.book_id, d.account_id, d.period_key)?.as_deref() == Some("finished") {
        return Err(error(
            "already_finished",
            "That month is already matched to the bank.",
        ));
    }
    if d.lines.is_empty() || d.lines.len() > 2000 {
        return Err(error(
            "invalid_args",
            "A statement read has 1 to 2,000 lines.",
        ));
    }
    if d.lines.iter().any(|l| l.bbox.is_some_and(|b| !bbox_ok(&b))) {
        return Err(error("invalid_args", "A line's box is outside its page."));
    }
    let now = now_iso();
    conn.execute(
        "UPDATE statement_draft SET status = 'replaced', decided_at = ?4
         WHERE book_id = ?1 AND account_id = ?2 AND period_key = ?3 AND status = 'pending'",
        params![d.book_id, d.account_id, d.period_key, now],
    )?;
    conn.execute(
        "INSERT INTO statement_draft (book_id, account_id, period_key, closing, evidence_id, proposed_by, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![d.book_id, d.account_id, d.period_key, d.closing, d.evidence_id, d.proposed_by, now],
    )?;
    let id = conn.last_insert_rowid();
    let mut ins = conn.prepare(
        "INSERT INTO statement_draft_line (draft_id, line_no, line_date, description, amount, balance, page, bx0, by0, bx1, by1, confidence)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
    )?;
    for l in d.lines {
        let b = l.bbox.map(|b| b.map(Some)).unwrap_or([None; 4]);
        ins.execute(params![
            id,
            l.line_no,
            l.date.to_string(),
            l.description,
            l.amount,
            l.balance,
            l.page,
            b[0],
            b[1],
            b[2],
            b[3],
            l.confidence
        ])?;
    }
    Ok(id)
}

fn bbox_of(v: [Option<i64>; 4]) -> Option<Bbox> {
    Some([v[0]?, v[1]?, v[2]?, v[3]?])
}

pub fn get(conn: &Connection, id: i64) -> Result<Option<Draft>> {
    let head = conn
        .query_row(
            "SELECT draft_id, book_id, account_id, period_key, closing, evidence_id, proposed_by, status FROM statement_draft WHERE draft_id = ?1",
            [id],
            |r| {
                Ok(Draft {
                    id: r.get(0)?,
                    book_id: r.get(1)?,
                    account_id: r.get(2)?,
                    period_key: r.get(3)?,
                    closing: r.get(4)?,
                    evidence_id: r.get(5)?,
                    proposed_by: r.get(6)?,
                    status: r.get(7)?,
                    lines: Vec::new(),
                })
            },
        )
        .optional()?;
    let Some(mut d) = head else { return Ok(None) };
    let mut stmt = conn.prepare(
        "SELECT line_no, line_date, description, amount, balance, page, bx0, by0, bx1, by1, confidence
         FROM statement_draft_line WHERE draft_id = ?1 ORDER BY line_no",
    )?;
    let rows = stmt.query_map([id], |r| {
        let date: String = r.get(1)?;
        Ok((
            date,
            DraftLine {
                line_no: r.get(0)?,
                date: Date {
                    year: 1970,
                    month: 1,
                    day: 1,
                },
                description: r.get(2)?,
                amount: r.get(3)?,
                balance: r.get(4)?,
                page: r.get(5)?,
                bbox: bbox_of([r.get(6)?, r.get(7)?, r.get(8)?, r.get(9)?]),
                confidence: r.get(10)?,
            },
        ))
    })?;
    for row in rows {
        let (date, mut line) = row?;
        line.date = Date::parse(&date)
            .ok_or_else(|| error("store", "A drafted line's date is not a date."))?;
        d.lines.push(line);
    }
    Ok(Some(d))
}

/// The pending draft for an account and month, if any.
pub fn pending(conn: &Connection, book: i64, account: i64, key: &str) -> Result<Option<Draft>> {
    let id: Option<i64> = conn
        .query_row(
            "SELECT draft_id FROM statement_draft WHERE book_id = ?1 AND account_id = ?2 AND period_key = ?3 AND status = 'pending' ORDER BY draft_id DESC LIMIT 1",
            params![book, account, key],
            |r| r.get(0),
        )
        .optional()?;
    id.map_or(Ok(None), |id| get(conn, id))
}

/// Accept or dismiss a pending draft.
pub fn decide(conn: &Connection, id: i64, status: &str) -> Result<()> {
    if !matches!(status, "accepted" | "dismissed") {
        return Err(error("invalid_args", "A draft is accepted or dismissed."));
    }
    let n = conn.execute(
        "UPDATE statement_draft SET status = ?2, decided_at = ?3 WHERE draft_id = ?1 AND status = 'pending'",
        params![id, status, now_iso()],
    )?;
    if n == 0 {
        return Err(error(
            "not_pending",
            "That statement read was already decided.",
        ));
    }
    Ok(())
}

/// Keep a CSV statement's running balances, line by line (`line_no` is the
/// 1-based position, as `months::import` numbers them).
pub fn keep_balances(conn: &Connection, statement_id: i64, balances: &[Option<i64>]) -> Result<()> {
    let mut up = conn.prepare(
        "UPDATE statement_line SET balance = ?3 WHERE statement_id = ?1 AND line_no = ?2",
    )?;
    for (i, b) in balances.iter().enumerate() {
        if b.is_some() {
            up.execute(params![statement_id, i64::try_from(i + 1).unwrap_or(0), b])?;
        }
    }
    Ok(())
}

/// Write each accepted line's page, box, confidence and balance, and the
/// statement's evidence.
pub fn keep_extras(
    conn: &Connection,
    statement_id: i64,
    evidence_id: Option<i64>,
    extras: &[Extra],
) -> Result<()> {
    conn.execute(
        "UPDATE statement SET evidence_id = ?2 WHERE statement_id = ?1",
        params![statement_id, evidence_id],
    )?;
    let mut up = conn.prepare(
        "UPDATE statement_line SET balance = ?3, page = ?4, bx0 = ?5, by0 = ?6, bx1 = ?7, by1 = ?8, confidence = ?9
         WHERE statement_id = ?1 AND line_no = ?2",
    )?;
    for (i, e) in extras.iter().enumerate() {
        let b = e.bbox.map(|b| b.map(Some)).unwrap_or([None; 4]);
        up.execute(params![
            statement_id,
            i64::try_from(i + 1).unwrap_or(0),
            e.balance,
            e.page,
            b[0],
            b[1],
            b[2],
            b[3],
            e.confidence
        ])?;
    }
    Ok(())
}

/// A statement's evidence and each line's extras, in line order.
pub fn extras(conn: &Connection, statement_id: i64) -> Result<(Option<i64>, Vec<Extra>)> {
    let evidence: Option<i64> = conn.query_row(
        "SELECT evidence_id FROM statement WHERE statement_id = ?1",
        [statement_id],
        |r| r.get(0),
    )?;
    let mut stmt = conn.prepare(
        "SELECT balance, page, bx0, by0, bx1, by1, confidence FROM statement_line WHERE statement_id = ?1 ORDER BY line_no",
    )?;
    let rows = stmt.query_map([statement_id], |r| {
        Ok(Extra {
            balance: r.get(0)?,
            page: r.get(1)?,
            bbox: bbox_of([r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?]),
            confidence: r.get(6)?,
        })
    })?;
    Ok((evidence, rows.collect::<rusqlite::Result<_>>()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breaks_and_boxes() {
        assert_eq!(
            balance_breaks(&[(100, Some(100)), (-50, Some(50)), (25, Some(80))]),
            1
        );
        assert_eq!(balance_breaks(&[(100, None), (-50, Some(50))]), 0);
        assert_eq!(
            balance_breaks(&[(i64::MAX, Some(i64::MAX)), (i64::MAX, Some(1))]),
            1
        );
        assert!(bbox_ok(&[0, 0, 1000, 5]));
        assert!(!bbox_ok(&[10, 10, 5, 20]));
        assert!(!bbox_ok(&[0, 0, 1001, 5]));
        assert!(!bbox_ok(&[0, 5, 10, 5]));
    }
}
