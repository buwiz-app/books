//! `books_app_batch`: post, discard or recategorize up to 50 ledger
//! entries after one click. Each entry goes through the single-entry
//! function (`writes::post`, `writes::discard`, `reconcile::recategorize`),
//! so every gate those apply applies here too. The batch is one
//! transaction: the first refusal rolls it all back and names the entry.

use books_core::ledger::Status;
use books_store::plumbing;
use books_store::rusqlite::Connection;
use serde_json::{Map, Value, json};

use crate::service::{self, Answer, Origin, Refusal, text, with_subject};
use crate::{reconcile, writes};

pub const MAX_ENTRIES: usize = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Post,
    Discard,
    Recategorize,
}

impl Action {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "post" => Some(Self::Post),
            "discard" => Some(Self::Discard),
            "recategorize" => Some(Self::Recategorize),
            _ => None,
        }
    }
}

/// The entry tickets, 1 to 50 and each once.
fn tickets(args: &Map<String, Value>) -> Result<Vec<&str>, Refusal> {
    let list = args
        .get("entries")
        .and_then(Value::as_array)
        .ok_or_else(|| Refusal::invalid("`entries` is a list of entry tickets."))?;
    if list.is_empty() || list.len() > MAX_ENTRIES {
        return Err(Refusal::invalid(format!(
            "A batch takes 1 to {MAX_ENTRIES} entries."
        )));
    }
    let mut out: Vec<&str> = Vec::with_capacity(list.len());
    for v in list {
        let t = v
            .as_str()
            .ok_or_else(|| Refusal::invalid("Each entry is a ticket."))?;
        if out.contains(&t) {
            return Err(Refusal::invalid("An entry is in the batch twice."));
        }
        out.push(t);
    }
    Ok(out)
}

/// What the entry is called in a refusal: its number, never its content.
fn label(conn: &Connection, ticket: &str) -> String {
    match plumbing::subject(conn, ticket) {
        Ok(Some((kind, id))) if kind == "entry" => books_store::entries::get(conn, id)
            .ok()
            .flatten()
            .map_or_else(|| "an entry".into(), |e| format!("entry {:04}", e.no)),
        _ => "an entry".into(),
    }
}

/// Run the batch inside the write's transaction.
pub fn run(conn: &Connection, args: &Map<String, Value>, origin: Origin) -> Answer {
    if !origin.is_app() {
        return Err(Refusal::new(
            "permission_denied",
            "A batch needs the user's click in the Books app.",
        ));
    }
    let action = Action::parse(service::required(args, "action")?)
        .ok_or_else(|| Refusal::invalid("`action` is post, discard or recategorize."))?;
    let entries = tickets(args)?;
    let category = text(args, "category");
    if action == Action::Recategorize && category.is_none_or(str::is_empty) {
        return Err(Refusal::invalid("Pick a category to recategorize."));
    }
    let book = writes::book_in(conn, args)?;
    let book_ticket = plumbing::ticket_for(conn, "book", book.id)?;
    let mut done = Vec::with_capacity(entries.len());
    for ticket in &entries {
        let mut one = Map::new();
        one.insert("book".into(), json!(book_ticket));
        one.insert("entry".into(), json!(ticket));
        if let Some(c) = category {
            one.insert("category".into(), json!(c));
        }
        if let Some(k) = args.get("keep_both") {
            one.insert("keep_both".into(), k.clone());
        }
        let status = writes::entry_in(conn, &book, ticket)
            .map(|e| e.status)
            .map_err(|r| refused(conn, ticket, r))?;
        let wrong = match action {
            Action::Post | Action::Discard => status != Status::Draft,
            Action::Recategorize => status != Status::Posted,
        };
        if wrong {
            return Err(refused(
                conn,
                ticket,
                Refusal::new(
                    "wrong_status",
                    format!(
                        "It is {}, so it cannot be changed this way.",
                        status.as_str()
                    ),
                ),
            ));
        }
        let result = match action {
            Action::Post => writes::post(conn, &one),
            Action::Discard => writes::discard(conn, &one),
            Action::Recategorize => reconcile::recategorize(conn, &one),
        };
        let r = result.map_err(|r| refused(conn, ticket, r))?;
        done.push(r.get("entry").cloned().unwrap_or_else(|| json!(ticket)));
    }
    Ok(with_subject(
        json!({ "action": action_name(action), "done": done.len(), "entries": done }),
        "book",
        book.id,
    ))
}

fn action_name(a: Action) -> &'static str {
    match a {
        Action::Post => "post",
        Action::Discard => "discard",
        Action::Recategorize => "recategorize",
    }
}

/// The same refusal, naming the entry it is about in `refused` ("entry 0042").
/// The code and the sentence stay the ones the single-entry write gave.
fn refused(conn: &Connection, ticket: &str, r: Refusal) -> Refusal {
    let subject = label(conn, ticket);
    r.about(subject)
}
