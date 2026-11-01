//! Match assist in the window: why a suggestion was made, what past
//! matches say about a missing line, and matching one line on a click.

use books_core::date::{Period, PeriodKind};
use books_core::match_assist::{follows_hint, hint_for};
use books_core::statement::Match;
use books_store::Store;
use books_store::books::{self, Book};
use books_store::match_history;
use books_store::months::StatementLine;
use books_store::rusqlite::Connection;
use serde_json::{Map, Value, json};

use crate::service::{self, Answer, Origin, Refusal, with_subject};
use crate::writes;

/// Adds `no`, `why` (suggested rows) and `hint` (missing rows with
/// history) to the statement rows. Returns how many rows a hint shaped.
/// The hint payee is shown to the app only.
pub fn view_extra(
    store: &Store,
    book: &Book,
    account_id: i64,
    lines: &[StatementLine],
    found: &[Match],
    rows: &mut [Value],
    origin: Origin,
) -> Result<i64, Refusal> {
    let conn = store.conn();
    let hints = match_history::hints(conn, book.id, account_id)?;
    let ids: Vec<i64> = found
        .iter()
        .filter_map(|m| match m {
            Match::Suggested(e) => Some(*e),
            _ => None,
        })
        .collect();
    let payees = match_history::payees(conn, &ids)?;
    let mut used = 0;
    for (i, ((l, m), row)) in lines.iter().zip(found).zip(rows.iter_mut()).enumerate() {
        row["no"] = json!(i + 1);
        match m {
            Match::Suggested(e) => {
                row["why"] = json!(if follows_hint(&l.line, *e, &payees, &hints) {
                    used += 1;
                    "Same amount · matched this payee before".to_string()
                } else {
                    apart(conn, *e, &l.line)?
                });
            }
            Match::Missing => {
                if let Some(h) = hint_for(&hints, &l.line.description) {
                    let category = match h.category_account_id {
                        Some(id) => books::account(conn, id)?
                            .filter(|a| a.active && a.book_id == book.id)
                            .map(|a| a.name),
                        None => None,
                    };
                    let payee = h.payee.clone().filter(|_| origin == Origin::App);
                    if category.is_some() || payee.is_some() {
                        used += 1;
                        row["hint"] = json!({ "category": category, "payee": payee });
                    }
                }
            }
            Match::Exact(_) => {}
        }
    }
    Ok(used)
}

fn apart(
    conn: &Connection,
    entry_id: i64,
    line: &books_core::statement::Line,
) -> Result<String, Refusal> {
    let days = books_store::entries::get(conn, entry_id)?
        .map(|e| (e.date.to_days() - line.date.to_days()).abs())
        .unwrap_or(0);
    Ok(match days {
        0 => "Same amount, same day".to_string(),
        1 => "Same amount, 1 day apart".to_string(),
        n => format!("Same amount, {n} days apart"),
    })
}

/// `books_app_statement_accept` with `line` (a row's `no`): match that
/// one line to its suggestion, or to `entry` (an entry ticket) when given.
pub fn accept_one(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let book = writes::book_in(conn, args)?;
    let account = writes::account_in(conn, &book, service::required(args, "account")?)?;
    let month = Period::parse(service::required(args, "month")?)
        .filter(|p| p.kind == PeriodKind::Month)
        .ok_or_else(|| Refusal::invalid("`month` is YYYY-MM."))?;
    let line = args
        .get("line")
        .and_then(Value::as_i64)
        .ok_or_else(|| Refusal::invalid("`line` is a statement row number."))?;
    let entry = match args.get("entry").and_then(Value::as_str) {
        Some(t) => Some(writes::entry_in(conn, &book, t)?.id),
        None => None,
    };
    match_history::accept_line(conn, &book, account.id, &month.key, line, entry)?;
    Ok(with_subject(
        json!({ "accepted": 1 }),
        "account",
        account.id,
    ))
}

/// Whether `books_app_statement_accept` arguments ask for one line. Any
/// `line` or `entry` routes here, so a non-integer or absent line is
/// refused, never accept-all.
pub fn is_one_line(args: &Map<String, Value>) -> bool {
    args.contains_key("line") || args.contains_key("entry")
}
