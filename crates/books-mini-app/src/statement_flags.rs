//! M10 parity (reconcile-3): display-only anomaly flags on a statement and
//! the reasoned reopen of a finished one. `books_app_statement_finish` with
//! `reopen: true` reopens; otherwise it finishes and records the event.

use std::collections::HashMap;

use books_core::date::{Period, PeriodKind};
use books_core::statement_flags::{self, FlagKind};
use books_store::Store;
use books_store::books::Book;
use books_store::months::{Statement, StatementLine};
use books_store::rusqlite::{Connection, OptionalExtension as _, params};
use books_store::statement_lock;
use serde_json::{Map, Value, json};

use crate::service::{self, Answer, Origin, Refusal, text, with_subject};
use crate::{reconcile, writes};

/// `books_app_statement_finish`'s input: a statement write plus the reopen.
pub fn finish_schema() -> Value {
    json!({
        "type": "object",
        "required": ["client_request_id", "account"],
        "properties": {
            "client_request_id": { "type": "string" }, "book": { "type": "string" },
            "account": { "type": "string" }, "month": { "type": "string" },
            "reopen": { "type": "boolean", "description": "Reopen the finished statement instead." },
            "reason": { "type": "string", "description": "Why it is reopened, 3 to 200 characters. Required to reopen." }
        }
    })
}

/// Finish the month (and record it), or reopen it with a reason.
pub fn finish_or_reopen(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let book = writes::book_in(conn, args)?;
    let account = writes::account_in(conn, &book, service::required(args, "account")?)?;
    let month = Period::parse(service::required(args, "month")?)
        .filter(|p| p.kind == PeriodKind::Month)
        .ok_or_else(|| Refusal::invalid("`month` is YYYY-MM."))?;
    if args.get("reopen").and_then(Value::as_bool) == Some(true) {
        statement_lock::reopen(
            conn,
            &book,
            account.id,
            &month.key,
            text(args, "reason").unwrap_or(""),
        )?;
        return Ok(with_subject(
            json!({ "account": books_store::plumbing::ticket_for(conn, "account", account.id)?, "month": month.key, "status": "open", "reopened": true }),
            "account",
            account.id,
        ));
    }
    let out = reconcile::statement_finish(conn, args)?;
    let id: i64 = conn
        .query_row(
            "SELECT statement_id FROM statement WHERE book_id = ?1 AND account_id = ?2 AND period_key = ?3",
            params![book.id, account.id, month.key],
            |r| r.get(0),
        )
        .map_err(books_store::StoreError::from)?;
    statement_lock::record_finish(conn, id)?;
    Ok(out)
}

/// The account's earlier lines in this book, by payee key.
fn history(
    conn: &Connection,
    book_id: i64,
    s: &Statement,
) -> Result<HashMap<String, Vec<i64>>, Refusal> {
    let mut stmt = conn
        .prepare(
            "SELECT l.description, l.amount FROM statement_line l JOIN statement s ON s.statement_id = l.statement_id
             WHERE s.book_id = ?1 AND s.account_id = ?2 AND s.period_key < ?3 ORDER BY s.period_key, l.line_no",
        )
        .map_err(books_store::StoreError::from)?;
    let rows = stmt
        .query_map(params![book_id, s.account_id, s.period_key], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })
        .map_err(books_store::StoreError::from)?;
    let mut out: HashMap<String, Vec<i64>> = HashMap::new();
    for row in rows {
        let (d, a) = row.map_err(books_store::StoreError::from)?;
        out.entry(statement_flags::payee_key(&d))
            .or_default()
            .push(a);
    }
    Ok(out)
}

fn prev_closing(conn: &Connection, book_id: i64, s: &Statement) -> Result<Option<i64>, Refusal> {
    let Some(prev) = Period::parse(&s.period_key).map(|p| p.previous()) else {
        return Ok(None);
    };
    Ok(conn
        .query_row(
            "SELECT closing FROM statement WHERE book_id = ?1 AND account_id = ?2 AND period_key = ?3",
            params![book_id, s.account_id, prev.key],
            |r| r.get(0),
        )
        .optional()
        .map_err(books_store::StoreError::from)?)
}

/// Add `flags`, each row's `flag` and `reopenable` to a statement view.
pub fn extend(
    store: &Store,
    book: &Book,
    s: &Statement,
    lines: &[StatementLine],
    origin: Origin,
    out: &mut Value,
) -> Result<(), Refusal> {
    let conn = store.conn();
    let keys: Vec<String> = lines
        .iter()
        .map(|l| statement_flags::payee_key(&l.line.description))
        .collect();
    let input: Vec<(i64, books_core::date::Date, i64, &str)> = lines
        .iter()
        .zip(&keys)
        .enumerate()
        .map(|(i, (l, k))| {
            (
                i64::try_from(i + 1).unwrap_or(0),
                l.line.date,
                l.line.amount,
                k.as_str(),
            )
        })
        .collect();
    let found = statement_flags::flags(
        &input,
        &history(conn, book.id, s)?,
        prev_closing(conn, book.id, s)?,
        s.closing,
    );
    if let Some(rows) = out["rows"].as_array_mut() {
        for (i, row) in rows.iter_mut().enumerate() {
            let no = i64::try_from(i + 1).unwrap_or(0);
            let kind: Option<FlagKind> =
                found.iter().find(|f| f.line_no == Some(no)).map(|f| f.kind);
            row["flag"] = json!(kind.map(FlagKind::as_str));
        }
    }
    out["flags"] = found
        .iter()
        .map(|f| json!({ "line": f.line_no, "kind": f.kind.as_str(), "note": f.kind.note() }))
        .collect();
    out["reopenable"] = json!(s.status == "finished" && origin == Origin::App);
    Ok(())
}
