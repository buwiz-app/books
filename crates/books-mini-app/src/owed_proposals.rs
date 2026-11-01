//! Assistant-drafted bill and invoice proposals the user reviews in Unpaid.
//!
//! A proposal never touches the ledger, people or totals. Only the app's
//! accept decision runs the existing owed creation, which keeps its cash and
//! accrual semantics and period gates.

use books_core::date::Date;
use books_core::{money, privacy};
use books_store::books::Book;
use books_store::evidence::{self, Target};
use books_store::rusqlite::{Connection, OptionalExtension as _, params};
use books_store::{Store, people, plumbing};
use serde_json::{Map, Value, json};

use crate::records;
use crate::service::{self, Answer, Books, Origin, Refusal, required, text, with_subject};
use crate::writes;

const KIND: &str = "owed_proposal";
const FIELDS: [&str; 6] = ["reference", "issued", "due", "amount", "person", "category"];
/// Field values Books keeps; a person or category is kept as a ticket only.
const KEPT_VALUES: [&str; 4] = ["reference", "issued", "due", "amount"];
const DRAFT_KEYS: &[&str] = &[
    "client_request_id",
    "book",
    "evidence",
    "side",
    "issued",
    "due",
    "amount",
    "category",
    "person_suggestion",
    "confidence",
    "fields",
];
const DECISION_KEYS: &[&str] = &[
    "client_request_id",
    "book",
    "proposal",
    "decision",
    "person",
    "category",
    "issued",
    "due",
    "reference",
];

struct Proposal {
    id: i64,
    book_id: i64,
    evidence_id: i64,
    side: String,
    issued: String,
    due: Option<String>,
    amount: i64,
    category_id: Option<i64>,
    person_id: Option<i64>,
    confidence: i64,
    fields: Value,
    status: String,
}

const COLUMNS: &str = "id,book_id,evidence_id,side,issued,due,amount_centavos,category_id,person_suggestion_id,confidence_int,fields_json,status";

fn row(r: &books_store::rusqlite::Row<'_>) -> books_store::rusqlite::Result<Proposal> {
    let fields: String = r.get(10)?;
    Ok(Proposal {
        id: r.get(0)?,
        book_id: r.get(1)?,
        evidence_id: r.get(2)?,
        side: r.get(3)?,
        issued: r.get(4)?,
        due: r.get(5)?,
        amount: r.get(6)?,
        category_id: r.get(7)?,
        person_id: r.get(8)?,
        confidence: r.get(9)?,
        fields: serde_json::from_str(&fields).unwrap_or_else(|_| json!([])),
        status: r.get(11)?,
    })
}

fn store_err(e: books_store::rusqlite::Error) -> Refusal {
    books_store::StoreError::from(e).into()
}

/// A ticket already issued for a subject; reads never issue one.
fn issued_ticket(conn: &Connection, kind: &str, id: i64) -> Result<Option<String>, Refusal> {
    conn.query_row(
        "SELECT ticket FROM ticket WHERE subject_kind=?1 AND subject_id=?2 AND revoked_at IS NULL",
        params![kind, id],
        |r| r.get(0),
    )
    .optional()
    .map_err(store_err)
}

fn subject_id(conn: &Connection, kind: &str, value: &str, what: &str) -> Result<i64, Refusal> {
    match plumbing::subject(conn, value)? {
        Some((k, id)) if k == kind => Ok(id),
        _ => Err(Refusal::new(
            "not_found",
            format!("That {what} ticket is not one Books issued."),
        )),
    }
}

fn known(args: &Map<String, Value>, keys: &[&str]) -> Result<(), Refusal> {
    if args.keys().all(|k| keys.contains(&k.as_str())) {
        Ok(())
    } else {
        Err(Refusal::invalid("That request has an unknown argument."))
    }
}

fn percent(v: Option<&Value>, what: &str) -> Result<i64, Refusal> {
    v.and_then(Value::as_u64)
        .filter(|n| *n <= 100)
        .map(|n| n as i64)
        .ok_or_else(|| Refusal::invalid(format!("{what} is a whole number 0 to 100.")))
}

fn date(args: &Map<String, Value>, key: &str) -> Result<Option<Date>, Refusal> {
    text(args, key)
        .map(|d| Date::parse(d).ok_or_else(|| Refusal::invalid(format!("`{key}` is YYYY-MM-DD."))))
        .transpose()
}

/// The external side names map onto the store's `collect` and `pay`.
fn store_side(side: &str) -> Result<&'static str, Refusal> {
    match side {
        "owe" => Ok("pay"),
        "collect" => Ok("collect"),
        _ => Err(Refusal::invalid("`side` is owe or collect.")),
    }
}

fn person_kind(side: &str) -> &'static str {
    if side == "collect" {
        "customer"
    } else {
        "vendor"
    }
}

/// Bills print the vendor's TIN; Books never stores one.
fn no_tin(field: &str, value: &str) -> Result<(), Refusal> {
    privacy::refuse_tin(field, value).map_err(|m| Refusal::new("tin_refused", m))
}

fn same_book_person(conn: &Connection, book: &Book, side: &str, id: i64) -> Result<(), Refusal> {
    people::person(conn, id)?
        .filter(|p| p.book_id == book.id && p.active && p.kind == person_kind(side))
        .map(|_| ())
        .ok_or_else(|| Refusal::new("not_found", "That person is not in this book on that side."))
}

/// `books_owed_draft`: the assistant's reviewable proposal. Writes no
/// ledger fact, person or owed item.
pub fn draft(conn: &Connection, args: &Map<String, Value>) -> Answer {
    known(args, DRAFT_KEYS)?;
    let book = writes::book_in(conn, args)?;
    let evidence_id = subject_id(conn, "evidence", required(args, "evidence")?, "evidence")?;
    evidence::get(conn, evidence_id)?
        .filter(|e| e.book_id == book.id)
        .ok_or_else(|| Refusal::new("not_found", "That evidence is not in this book."))?;
    let side = store_side(required(args, "side")?)?;
    let issued =
        date(args, "issued")?.ok_or_else(|| Refusal::invalid("`issued` is YYYY-MM-DD."))?;
    let due = date(args, "due")?;
    if due.is_some_and(|d| d < issued) {
        return Err(Refusal::invalid("`due` is on or after `issued`."));
    }
    let amount = writes::amount(args, "amount")?;
    let category = match text(args, "category") {
        Some(t) => {
            let a = writes::account_in(conn, &book, t)?;
            if a.is_cash() || a.accrual_only() {
                return Err(Refusal::invalid("Pick an income or expense category."));
            }
            Some(a.id)
        }
        None => None,
    };
    let confidence = percent(args.get("confidence"), "`confidence`")?;
    let mut kept = Vec::new();
    let mut person_text = None;
    if let Some(list) = args.get("fields") {
        let list = list
            .as_array()
            .filter(|l| l.len() <= FIELDS.len())
            .ok_or_else(|| Refusal::invalid("`fields` is a list of at most six fields."))?;
        for f in list {
            let f = f
                .as_object()
                .ok_or_else(|| Refusal::invalid("Each field is an object."))?;
            known(f, &["field", "value", "confidence"])?;
            let field = required(f, "field")?;
            if !FIELDS.contains(&field) {
                return Err(Refusal::invalid("Unknown proposal field."));
            }
            let value = required(f, "value")?;
            if value.chars().count() > 120 {
                return Err(Refusal::invalid("A field value is at most 120 characters."));
            }
            let c = percent(f.get("confidence"), "A field confidence")?;
            if field == "person" {
                person_text = Some(value.trim().to_lowercase());
            }
            let mut out = json!({"field": field, "confidence": c});
            if KEPT_VALUES.contains(&field) {
                no_tin(field, value)?;
                out["value"] = json!(value);
            }
            kept.push(out);
        }
    }
    let person = match text(args, "person_suggestion") {
        Some(t) => {
            let id = subject_id(conn, "person", t, "person")?;
            same_book_person(conn, &book, side, id)?;
            Some(id)
        }
        // an exact name match may suggest an existing person; never makes one
        None => match person_text.filter(|n| !n.is_empty()) {
            Some(name) => {
                let found: Vec<i64> = people::people(conn, book.id, Some(person_kind(side)))?
                    .into_iter()
                    .filter(|p| p.name.trim().to_lowercase() == name)
                    .map(|p| p.id)
                    .collect();
                (found.len() == 1).then(|| found[0])
            }
            None => None,
        },
    };
    conn.execute(
        "INSERT INTO owed_proposal(book_id,evidence_id,side,issued,due,amount_centavos,category_id,person_suggestion_id,confidence_int,fields_json,status,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,'draft',?11)",
        params![
            book.id,
            evidence_id,
            side,
            issued.to_string(),
            due.map(|d| d.to_string()),
            amount,
            category,
            person,
            confidence,
            Value::Array(kept).to_string(),
            books_store::now_iso()
        ],
    )
    .map_err(store_err)?;
    let id = conn.last_insert_rowid();
    // tickets are issued in the write, so list reads stay read-only
    plumbing::ticket_for(conn, "evidence", evidence_id)?;
    if let Some(p) = person {
        plumbing::ticket_for(conn, "person", p)?;
    }
    if let Some(a) = category {
        plumbing::ticket_for(conn, "account", a)?;
    }
    Ok(with_subject(
        json!({"proposal": plumbing::ticket_for(conn, KIND, id)?, "status": "draft"}),
        KIND,
        id,
    ))
}

fn proposal(
    conn: &Connection,
    book: &Book,
    args: &Map<String, Value>,
) -> Result<Proposal, Refusal> {
    let id = subject_id(conn, KIND, required(args, "proposal")?, "proposal")?;
    conn.query_row(
        &format!("SELECT {COLUMNS} FROM owed_proposal WHERE id=?1"),
        [id],
        row,
    )
    .optional()
    .map_err(store_err)?
    .filter(|p| p.book_id == book.id)
    .ok_or_else(|| Refusal::new("not_found", "That proposal is not in this book."))
}

fn decide(conn: &Connection, args: &Map<String, Value>) -> Answer {
    known(args, DECISION_KEYS)?;
    let book = writes::book_in(conn, args)?;
    let p = proposal(conn, &book, args)?;
    let decision = required(args, "decision")?;
    let ticket = plumbing::ticket_for(conn, KIND, p.id)?;
    match (decision, p.status.as_str()) {
        ("accept" | "discard", "accepted") => {
            return Err(Refusal::new(
                "already_accepted",
                "That proposal is already accepted.",
            ));
        }
        ("accept", "discarded") => {
            return Err(Refusal::new("discarded", "That proposal was discarded."));
        }
        ("discard", _) => {
            conn.execute(
                "UPDATE owed_proposal SET status='discarded' WHERE id=?1 AND status='draft'",
                [p.id],
            )
            .map_err(store_err)?;
            return Ok(with_subject(
                json!({"proposal": ticket, "status": "discarded"}),
                KIND,
                p.id,
            ));
        }
        ("accept", _) => {}
        _ => return Err(Refusal::invalid("`decision` is accept or discard.")),
    }
    // the user's reviewed person and category; Books never fills them in
    let person = subject_id(
        conn,
        "person",
        text(args, "person").ok_or_else(|| Refusal::invalid("Pick the reviewed person."))?,
        "person",
    )?;
    same_book_person(conn, &book, &p.side, person)?;
    let category =
        text(args, "category").ok_or_else(|| Refusal::invalid("Pick the reviewed category."))?;
    let reference = text(args, "reference").map(str::to_string).or_else(|| {
        p.fields.as_array().and_then(|l| {
            l.iter()
                .find(|f| f["field"] == "reference")
                .and_then(|f| f["value"].as_str().map(str::to_string))
        })
    });
    let mut create = Map::new();
    create.insert(
        "book".into(),
        json!(plumbing::ticket_for(conn, "book", book.id)?),
    );
    create.insert("side".into(), json!(p.side));
    create.insert("amount".into(), json!(money::format(p.amount)));
    create.insert("category".into(), json!(category));
    create.insert(
        "person".into(),
        json!(plumbing::ticket_for(conn, "person", person)?),
    );
    let issued = date(args, "issued")?.map_or(p.issued.clone(), |d| d.to_string());
    create.insert("issued".into(), json!(issued));
    // an empty `due` is the user clearing the suggested due date
    let due = match text(args, "due") {
        Some("") => None,
        Some(_) => date(args, "due")?.map(|d| d.to_string()),
        None => p.due.clone(),
    };
    if let Some(due) = due {
        create.insert("due".into(), json!(due));
    }
    if let Some(r) = reference.filter(|r| !r.trim().is_empty()) {
        no_tin("reference", &r)?;
        create.insert("reference".into(), json!(r));
    }
    let created = records::owed_create(conn, &create)?;
    let owed = created["_subject"][1]
        .as_i64()
        .ok_or_else(|| Refusal::new("store", "The owed item vanished."))?;
    evidence::link(conn, book.id, p.evidence_id, Target::Owed(owed))?;
    let changed = conn
        .execute(
            "UPDATE owed_proposal SET status='accepted', accepted_owed_id=?2 WHERE id=?1 AND status='draft'",
            params![p.id, owed],
        )
        .map_err(store_err)?;
    if changed != 1 {
        return Err(Refusal::new(
            "already_accepted",
            "That proposal is already accepted.",
        ));
    }
    Ok(with_subject(
        json!({"proposal": ticket, "status": "accepted", "owed": created["owed"], "owed_status": created["status"]}),
        "owed",
        owed,
    ))
}

/// `books_owed_draft` runs for the assistant only; the app reviews instead.
pub fn draft_tool(
    books: &mut Books,
    name: &str,
    origin: Origin,
    args: &Map<String, Value>,
) -> Value {
    if origin != Origin::Agent {
        return Refusal::new(
            "forbidden",
            "Only the assistant drafts a proposal; review it in Unpaid.",
        )
        .into_result();
    }
    service::write(books, name, origin, args, |tx| draft(tx, args))
}

/// `books_app_owed_create`: unchanged without a proposal; with one, the
/// user's accept or discard.
pub fn owed_create(
    books: &mut Books,
    name: &str,
    origin: Origin,
    args: &Map<String, Value>,
) -> Value {
    if args.contains_key("proposal") || args.contains_key("decision") {
        if !origin.is_app() {
            return Refusal::new(
                "forbidden",
                "Only the user's click in Books decides a proposal.",
            )
            .into_result();
        }
        return service::write(books, name, origin, args, |tx| decide(tx, args));
    }
    service::write(books, name, origin, args, |tx| {
        records::owed_create(tx, args)
    })
}

fn summary(conn: &Connection, p: &Proposal) -> Result<Value, Refusal> {
    let ticket = |kind: &str, id: Option<i64>| -> Result<Value, Refusal> {
        Ok(match id {
            Some(id) => json!(issued_ticket(conn, kind, id)?),
            None => Value::Null,
        })
    };
    let person = match p
        .person_id
        .and_then(|id| people::person(conn, id).ok().flatten())
    {
        Some(x) => json!({"ticket": issued_ticket(conn, "person", x.id)?, "name": x.name}),
        None => Value::Null,
    };
    let source = evidence::get(conn, p.evidence_id)?
        .map(|e| super::evidence_tools::label(&e))
        .unwrap_or_else(|| "Evidence".into());
    Ok(json!({
        "proposal": issued_ticket(conn, KIND, p.id)?,
        "issued": p.issued, "due": p.due,
        "amount": money::format(p.amount), "amount_shown": money::display(p.amount),
        "confidence": p.confidence, "fields": p.fields,
        "person_suggestion": person, "category": ticket("account", p.category_id)?,
        "evidence": ticket("evidence", Some(p.evidence_id))?, "source": source,
    }))
}

/// Adds pending proposals and each row's evidence to an app's
/// `books_owed_list`; the assistant sees only a count.
pub fn decorate(store: &Store, book: &Book, origin: Origin, mut list: Value) -> Answer {
    let conn = store.conn();
    let side = list["side"].as_str().unwrap_or("collect").to_string();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM owed_proposal WHERE book_id=?1 AND side=?2 AND status='draft'",
            params![book.id, side],
            |r| r.get(0),
        )
        .map_err(store_err)?;
    list["pending_proposals"] = json!(count);
    if !origin.is_app() {
        return Ok(list);
    }
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {COLUMNS} FROM owed_proposal WHERE book_id=?1 AND side=?2 AND status='draft' ORDER BY id LIMIT 50"
        ))
        .map_err(store_err)?;
    let pending: Vec<Proposal> = stmt
        .query_map(params![book.id, side], row)
        .map_err(store_err)?
        .collect::<Result<_, _>>()
        .map_err(store_err)?;
    list["proposals"] = Value::Array(
        pending
            .iter()
            .map(|p| summary(conn, p))
            .collect::<Result<_, _>>()?,
    );
    if let Some(rows) = list["rows"].as_array_mut() {
        for r in rows {
            let id = match r["ticket"].as_str().map(|t| plumbing::subject(conn, t)) {
                Some(Ok(Some((k, id)))) if k == "owed" => id,
                _ => continue,
            };
            let labels: Vec<String> =
                evidence::list(conn, book.id, None, Some(Target::Owed(id)), 0, 20)?
                    .iter()
                    .map(super::evidence_tools::label)
                    .collect();
            r["evidence"] = json!(labels);
        }
    }
    Ok(list)
}

pub fn draft_schema() -> Value {
    let pct = json!({"type": "integer", "minimum": 0, "maximum": 100});
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["client_request_id", "evidence", "side", "issued", "amount", "confidence"],
        "properties": {
            "client_request_id": {"type": "string"}, "book": {"type": "string"},
            "evidence": {"type": "string", "description": "An evidence ticket the user imported in Books."},
            "side": {"type": "string", "enum": ["owe", "collect"]},
            "issued": {"type": "string", "description": "YYYY-MM-DD"},
            "due": {"type": "string", "description": "YYYY-MM-DD"},
            "amount": {"type": "string", "description": "Decimal text such as 1200.00."},
            "category": {"type": "string", "description": "An account ticket (books_owed_options)."},
            "person_suggestion": {"type": "string", "description": "An existing person ticket; Books never makes a person."},
            "confidence": pct,
            "fields": {"type": "array", "maxItems": 6, "items": {
                "type": "object", "additionalProperties": false, "required": ["field", "value", "confidence"],
                "properties": {
                    "field": {"type": "string", "enum": FIELDS},
                    "value": {"type": "string", "maxLength": 120},
                    "confidence": pct
                }
            }}
        }
    })
}
