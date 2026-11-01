//! Monthly repeats of unpaid documents (M10 parity, unpaid-3): the
//! `books_app_owed_repeat` write and the repeat fields `books_owed_list`
//! carries. The user adds each month's copy from the app; no routine and
//! no assistant writes one (cards.rs routines change nothing here).

use books_core::date::Date;
use books_core::money;
use books_store::Store;
use books_store::books::Book;
use books_store::owed_repeat::{self, parse_month};
use books_store::people;
use books_store::plumbing;
use books_store::rusqlite::Connection;
use serde_json::{Map, Value, json};

use crate::service::{self, Answer, Origin, Refusal, text, ticket, with_subject};
use crate::writes;

fn owed_in(conn: &Connection, args: &Map<String, Value>) -> Result<i64, Refusal> {
    match plumbing::subject(conn, service::required(args, "owed")?)? {
        Some((k, id)) if k == "owed" => Ok(id),
        _ => Err(Refusal::new(
            "not_found",
            "That document ticket is not one Books issued.",
        )),
    }
}

/// `books_app_owed_repeat`: `action` start, stop or add (`month`, this
/// month when absent). Returns `{owed, repeat, added?}`.
pub fn repeat(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let book = writes::book_in(conn, args)?;
    let id = owed_in(conn, args)?;
    let today = Date::today();
    let mut out = json!({});
    match service::required(args, "action")? {
        a @ ("start" | "stop") => {
            if text(args, "month").is_some() {
                return Err(Refusal::invalid("`month` goes with add only."));
            }
            owed_repeat::set(conn, &book, id, a == "start")?;
        }
        "add" => {
            let month = match text(args, "month") {
                Some(m) => parse_month(m).ok_or_else(|| Refusal::invalid("`month` is YYYY-MM."))?,
                None => (today.year, today.month),
            };
            let g = writes::gate(conn, &book);
            let copy = owed_repeat::add(conn, &book, id, month, today, &g)?;
            out["added"] = json!(plumbing::ticket_for(conn, "owed", copy)?);
        }
        _ => return Err(Refusal::invalid("`action` is start, stop or add.")),
    }
    out["owed"] = json!(plumbing::ticket_for(conn, "owed", id)?);
    out["repeat"] = if owed_repeat::repeats(conn, id)? {
        json!("monthly")
    } else {
        Value::Null
    };
    Ok(with_subject(out, "owed", id))
}

/// Add the repeat fields to a `books_owed_list` result: `repeat` on each
/// row, and `repeats: {count, due}` for the listed side.
pub fn decorate(store: &Store, book: Book, origin: Origin, mut v: Value) -> Answer {
    let conn = store.conn();
    if let Some(rows) = v["rows"].as_array_mut() {
        for row in rows {
            let id = match row["ticket"].as_str() {
                Some(t) => plumbing::subject(conn, t)?
                    .filter(|(k, _)| k == "owed")
                    .map(|(_, id)| id),
                None => None,
            };
            let on = match id {
                Some(id) => owed_repeat::repeats(conn, id)?,
                None => false,
            };
            row["repeat"] = if on { json!("monthly") } else { Value::Null };
        }
    }
    let side = v["side"].as_str().unwrap_or("collect").to_string();
    let mut due = Vec::new();
    for d in owed_repeat::due(conn, book.id, Date::today())? {
        let Some(t) = people::unpaid(conn, d.template_id)? else {
            continue;
        };
        if t.side != side {
            continue;
        }
        let mut item = json!({
            "owed": ticket(store, "owed", t.id)?,
            "issued": d.issued.to_string(),
            "due": d.due.map(|x| x.to_string()),
            "amount_shown": money::display(t.amount),
        });
        if origin.is_app() {
            let who = match t.person_id {
                Some(p) => people::person(conn, p)?.map(|p| p.name),
                None => None,
            };
            item["who"] = json!(who.unwrap_or_else(|| "—".into()));
            item["doc"] = json!(t.reference.clone().unwrap_or_default());
        }
        due.push(item);
    }
    v["repeats"] = json!({ "count": due.len(), "due": due });
    Ok(v)
}
