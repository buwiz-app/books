//! Evidence writes and bounded metadata-only reads.

use books_store::evidence::{self, Input, Metadata, Target};
use books_store::plumbing;
use books_store::rusqlite::{Connection, OptionalExtension as _, params};
use serde_json::{Map, Value, json};

use crate::service::{self, Answer, Books, Origin, Refusal, required, text};
use crate::{oldbooks, writes};

pub fn lookup(conn: &Connection, id: i64) -> Result<Option<String>, Refusal> {
    Ok(conn.query_row("SELECT ticket FROM ticket WHERE subject_kind='evidence' AND subject_id=?1 AND revoked_at IS NULL", [id], |r| r.get(0)).optional().map_err(books_store::StoreError::from)?)
}

pub fn label(e: &Metadata) -> String {
    let kind = match e.kind.as_str() {
        "statement" => "Statement",
        "bill" => "Bill",
        "invoice" => "Invoice",
        "receipt" => "Receipt",
        "payslip" => "Payslip",
        "contract" => "Contract",
        "tax_form" => "Tax form",
        _ => "Evidence",
    };
    let mime = match e.mime.as_str() {
        "application/pdf" => "PDF",
        "image/png" => "PNG",
        "image/jpeg" => "JPEG",
        _ => "WebP",
    };
    match e.page_count {
        Some(1) => format!("{kind} · {mime} · 1 page"),
        Some(n) => format!("{kind} · {mime} · {n} pages"),
        None => format!("{kind} · {mime}"),
    }
}

fn subject(conn: &Connection, kind: &str, value: &str) -> Result<i64, Refusal> {
    match plumbing::subject(conn, value)? {
        Some((k, id)) if k == kind => Ok(id),
        _ => Err(Refusal::new(
            "not_found",
            "That ticket is not of the required kind.",
        )),
    }
}

fn target(conn: &Connection, args: &Map<String, Value>) -> Result<Target, Refusal> {
    let t = args
        .get("target")
        .and_then(Value::as_object)
        .ok_or_else(|| Refusal::invalid("A target is required."))?;
    let kind = required(t, "kind")?;
    let id = subject(conn, kind, required(t, "ticket")?)?;
    match kind {
        "entry" => Ok(Target::Entry(id)),
        "owed" => Ok(Target::Owed(id)),
        _ => Err(Refusal::invalid("The target must be entry or owed.")),
    }
}

fn evidence_id(conn: &Connection, book: i64, value: &str) -> Result<i64, Refusal> {
    let id = subject(conn, "evidence", value)?;
    evidence::get(conn, id)?
        .filter(|e| e.book_id == book)
        .ok_or_else(|| Refusal::new("not_found", "That evidence is not in this book."))?;
    Ok(id)
}

fn valid_mime(mime: &str, b: &[u8]) -> bool {
    match mime {
        "application/pdf" => b.starts_with(b"%PDF-"),
        "image/png" => b.starts_with(b"\x89PNG\r\n\x1a\n"),
        "image/jpeg" => b.starts_with(b"\xff\xd8\xff"),
        "image/webp" => b.starts_with(b"RIFF") && b.get(8..12) == Some(b"WEBP"),
        _ => false,
    }
}

pub fn attach(books: &mut Books, name: &str, origin: Origin, args: &Map<String, Value>) -> Value {
    if !origin.is_app() {
        return Refusal::new("forbidden", "Only the app may attach evidence.").into_result();
    }
    let mut uploads = std::mem::take(&mut books.uploads);
    let out = service::write(books, name, origin, args, |tx| {
        let book = writes::book_in(tx, args)?;
        match required(args, "op")? {
            "part" => oldbooks::upload_part(&mut uploads, args),
            "commit" => {
                let mime = required(args, "mime")?;
                let page_count = match args.get("page_count") {
                    None => None,
                    Some(v) => Some(
                        v.as_i64()
                            .filter(|n| (1..=100000).contains(n))
                            .ok_or_else(|| Refusal::invalid("page_count is 1 to 100000."))?,
                    ),
                };
                let source = text(args, "source").unwrap_or("user");
                let kind = text(args, "kind").unwrap_or("other");
                if !["user", "assistant"].contains(&source)
                    || ![
                        "statement",
                        "bill",
                        "invoice",
                        "receipt",
                        "payslip",
                        "contract",
                        "tax_form",
                        "other",
                    ]
                    .contains(&kind)
                {
                    return Err(Refusal::invalid("Unknown evidence source or kind."));
                }
                let bytes = uploads.take(required(args, "upload")?)?;
                if !valid_mime(mime, &bytes) {
                    return Err(Refusal::invalid(
                        "Evidence MIME and signature must match a supported format.",
                    ));
                }
                let (id, deduplicated) = evidence::insert(
                    tx,
                    book.id,
                    &Input {
                        bytes: &bytes,
                        mime,
                        page_count,
                        source,
                        kind,
                    },
                )?;
                let token = plumbing::ticket_for(tx, "evidence", id)?;
                let e = evidence::get(tx, id)?
                    .ok_or_else(|| Refusal::new("not_found", "Evidence missing."))?;
                if args.contains_key("target") {
                    evidence::link(tx, book.id, id, target(tx, args)?)?;
                }
                Ok(
                    json!({"evidence":token,"kind":e.kind,"mime":e.mime,"page_count":e.page_count,"deduplicated":deduplicated}),
                )
            }
            "link" => {
                let id = evidence_id(tx, book.id, required(args, "evidence")?)?;
                evidence::link(tx, book.id, id, target(tx, args)?)?;
                Ok(json!({"linked":true}))
            }
            _ => Err(Refusal::invalid("op must be part, commit or link.")),
        }
    });
    books.uploads = uploads;
    out
}

pub fn detach(books: &mut Books, name: &str, origin: Origin, args: &Map<String, Value>) -> Value {
    if !origin.is_app() {
        return Refusal::new("forbidden", "Only the app may detach evidence.").into_result();
    }
    service::write(books, name, origin, args, |tx| {
        let book = writes::book_in(tx, args)?;
        let id = evidence_id(tx, book.id, required(args, "evidence")?)?;
        let detached = evidence::detach(tx, book.id, id, target(tx, args)?)?;
        Ok(json!({"detached":detached}))
    })
}

pub fn list(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let book = writes::book_in(conn, args)?;
    let id = text(args, "evidence")
        .map(|v| evidence_id(conn, book.id, v))
        .transpose()?;
    let target = if args.contains_key("target") {
        Some(target(conn, args)?)
    } else {
        None
    };
    if let Some(t) = target {
        evidence::validate_target(conn, book.id, t, false)?;
    }
    let after = match text(args, "cursor") {
        Some(v) => v
            .parse::<i64>()
            .ok()
            .filter(|n| *n >= 0)
            .ok_or_else(|| Refusal::invalid("Invalid cursor."))?,
        None => 0,
    };
    let limit = match args.get("limit") {
        Some(v) => v
            .as_u64()
            .filter(|n| (1..=200).contains(n))
            .ok_or_else(|| Refusal::invalid("limit is 1 to 200."))? as usize,
        None => 50,
    };
    let mut rows = evidence::list(conn, book.id, id, target, after, limit + 1)?;
    let more = rows.len() > limit;
    rows.truncate(limit);
    let next = if more {
        rows.last().map(|r| r.id.to_string())
    } else {
        None
    };
    let mut items = Vec::new();
    for e in rows {
        if let Some(t) = lookup(conn, e.id)? {
            items.push(json!({"evidence":t,"kind":e.kind,"mime":e.mime,"page_count":e.page_count,"source":e.source,"linked_count":evidence::linked_count(conn,e.id)?}));
        }
    }
    Ok(json!({"items":items,"next_cursor":next}))
}

pub fn scope(store: &books_store::Store, book: &books_store::books::Book, value: &str) -> Answer {
    let conn = store.conn();
    if plumbing::is_ticket(value) {
        let id = evidence_id(conn, book.id, value)?;
        let e = evidence::get(conn, id)?
            .ok_or_else(|| Refusal::new("not_found", "Evidence missing."))?;
        return Ok(json!({"result":"token","ticket":value,"title":label(&e)}));
    }
    let mut stmt = conn.prepare("SELECT e.id,e.book_id,e.kind,e.mime,e.page_count,e.source,t.ticket FROM evidence e JOIN ticket t ON t.subject_kind='evidence' AND t.subject_id=e.id AND t.revoked_at IS NULL WHERE e.book_id=?1 ORDER BY e.id LIMIT 200").map_err(books_store::StoreError::from)?;
    let rows = stmt
        .query_map(params![book.id], |r| {
            Ok((
                Metadata {
                    id: r.get(0)?,
                    book_id: r.get(1)?,
                    kind: r.get(2)?,
                    mime: r.get(3)?,
                    page_count: r.get(4)?,
                    source: r.get(5)?,
                },
                r.get::<_, String>(6)?,
            ))
        })
        .map_err(books_store::StoreError::from)?;
    let mut choices = Vec::new();
    for row in rows {
        let (e, t) = row.map_err(books_store::StoreError::from)?;
        let label = label(&e);
        if label.to_lowercase().contains(&value.to_lowercase()) {
            choices.push(json!({"label":label,"value":t}));
        }
        if choices.len() == 20 {
            break;
        }
    }
    if choices.is_empty() {
        return Ok(json!({"result":"error","message":"No evidence in Books matches that."}));
    }
    Ok(json!({"result":"choices","choices":choices}))
}

fn schema() -> Value {
    json!({"type":"object","properties":{
        "client_request_id":{"type":"string"},"book":{"type":"string"},"op":{"type":"string","enum":["part","commit","link"]},
        "upload":{"type":"string"},"part":{"type":"integer"},"parts":{"type":"integer"},"data_base64":{"type":"string"},
        "mime":{"type":"string"},"page_count":{"type":"integer","minimum":1,"maximum":100000},"source":{"type":"string","enum":["user","assistant"]},
        "kind":{"type":"string","enum":["statement","bill","invoice","receipt","payslip","contract","tax_form","other"]},
        "evidence":{"type":"string"},"target":{"type":"object","required":["kind","ticket"],"properties":{"kind":{"type":"string","enum":["entry","owed"]},"ticket":{"type":"string"}}},
        "cursor":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":200}
    }})
}

fn schema_for(keys: &[&str], required: &[&str]) -> Value {
    let mut out = schema();
    out["properties"]
        .as_object_mut()
        .expect("schema properties")
        .retain(|k, _| keys.contains(&k.as_str()));
    out["required"] = json!(required);
    out["additionalProperties"] = json!(false);
    out
}

pub fn attach_schema() -> Value {
    schema_for(
        &[
            "client_request_id",
            "book",
            "op",
            "upload",
            "part",
            "parts",
            "data_base64",
            "mime",
            "page_count",
            "source",
            "kind",
            "evidence",
            "target",
        ],
        &["client_request_id", "op"],
    )
}

pub fn detach_schema() -> Value {
    schema_for(
        &["client_request_id", "book", "evidence", "target"],
        &["client_request_id", "evidence", "target"],
    )
}

pub fn list_schema() -> Value {
    schema_for(&["book", "evidence", "target", "cursor", "limit"], &[])
}
