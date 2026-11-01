//! Amend by reversal through `books_app_void_entry {amend: true}` (no new
//! window tool): the original is voided with a posted reversal and a draft
//! replacement with the same lines opens for the user to correct and post.
//! App-only like every void (DD-20). The reason goes to the store's
//! `amendment` table and nowhere else: no result, title, card or log.

use books_core::date::Date;
use books_store::Store;
use books_store::amend;
use books_store::books::Book;
use books_store::entries::{self, Entry};
use books_store::plumbing;
use books_store::rusqlite::Connection;
use serde_json::{Map, Value, json};

use crate::service::{self, Answer, Refusal, text, ticket, with_subject};
use crate::writes;

/// Whether a void request asks for an amend.
pub fn asked(args: &Map<String, Value>) -> bool {
    args.get("amend").and_then(Value::as_bool) == Some(true)
}

/// `books_app_void_entry {amend: true, reason, date?}`: voids `e`, posts its
/// reversal on `date` (default the original's date) and saves the
/// replacement draft. Returns the three tickets.
pub fn void(conn: &Connection, args: &Map<String, Value>, book: &Book, e: &Entry) -> Answer {
    let reason = service::required(args, "reason")?;
    let date = match text(args, "date") {
        Some(d) => Date::parse(d).ok_or_else(|| Refusal::invalid("`date` is YYYY-MM-DD."))?,
        None => e.date,
    };
    let g = writes::gate(conn, book);
    let (reversal, replacement) = amend::amend(conn, book, e.id, date, reason, &g)?;
    let no = entries::get(conn, replacement)?
        .map(|r| format!("{:04}", r.no))
        .unwrap_or_default();
    let t = |id: i64| -> Result<String, Refusal> { Ok(plumbing::ticket_for(conn, "entry", id)?) };
    Ok(with_subject(
        json!({
            "entry": t(e.id)?,
            "status": "voided",
            "voided": t(e.id)?,
            "reversal": t(reversal)?,
            "replacement": t(replacement)?,
            "replacement_no": no,
        }),
        "entry",
        e.id,
    ))
}

/// The drawer's amend links: `links {amends, amended_by}` (tickets or
/// null), `link_labels` for the chips, and the "Replaces entry 0042" line.
pub fn links(store: &Store, e: &Entry) -> Result<(Value, Value, Option<String>), Refusal> {
    let mut tickets = json!({ "amends": null, "amended_by": null });
    let mut labels = json!({ "amends": null, "amended_by": null });
    let mut related = None;
    for (key, id, label) in [
        ("amends", e.amends, "Amends entry"),
        ("amended_by", e.amended_by, "Amended by entry"),
    ] {
        let Some(other) = id.and_then(|i| entries::get(store.conn(), i).ok().flatten()) else {
            continue;
        };
        tickets[key] = json!(ticket(store, "entry", other.id)?);
        labels[key] = json!(format!("{label} {:04}", other.no));
        if key == "amends" {
            related = Some(format!("Replaces entry {:04}", other.no));
        }
    }
    Ok((tickets, labels, related))
}
