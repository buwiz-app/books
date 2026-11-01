//! Evidence classification review and the sandboxed snapshot preview.
//!
//! The assistant proposes a kind; only the user's click in an app accepts it.
//! Books never classifies a document itself. The preview is static HTML that
//! only `books_app_evidence_preview` builds; no other result carries bytes.

use books_core::money;
use books_store::evidence::{self, Target};
use books_store::plumbing;
use books_store::rusqlite::{Connection, OptionalExtension as _, params};
use serde_json::{Map, Value, json};

use crate::service::{self, Answer, Books, Origin, Refusal, required, text};
use crate::tools::evidence_tools::{KINDS, evidence_id, known, lookup, subject};
use crate::writes;

/// The ticket kind of a classification proposal.
pub const PROPOSAL: &str = "evidence_proposal";
/// The host's preview ceiling (Bot manifest.md: `permissions.preview.max_bytes`).
pub const PREVIEW_MAX_BYTES: usize = 8 * 1024 * 1024;
/// The card the assistant's proposal offers in the chat.
pub const CARD_APP: &str = "document";
pub const CARD_TITLE: &str = "What is this document?";

const CLASSIFY_KEYS: &[&str] = &[
    "client_request_id",
    "book",
    "evidence",
    "kind",
    "confidence",
];
const KIND_KEYS: &[&str] = &["client_request_id", "book", "proposal", "decision", "kind"];
const PREVIEW_KEYS: &[&str] = &["book", "evidence", "entry"];
const REVIEW_KEYS: &[&str] = &["book", "evidence", "proposal"];

fn kind_arg(args: &Map<String, Value>, key: &str) -> Result<Option<&'static str>, Refusal> {
    match args.get(key) {
        None => Ok(None),
        Some(v) => v
            .as_str()
            .and_then(|k| KINDS.iter().copied().find(|x| *x == k))
            .map(Some)
            .ok_or_else(|| Refusal::invalid(format!("`{key}` is one of {}.", KINDS.join(", ")))),
    }
}

/// Where an accepted kind leads in Books. Only navigation: it creates nothing.
pub fn route(kind: &str) -> &'static str {
    match kind {
        "receipt" | "invoice" => "ledger_draft",
        "bill" => "unpaid_review",
        "statement" => "statement_import",
        "tax_form" => "certificate_review",
        _ => "stored",
    }
}

/// `books_evidence_classify`: the assistant's proposal, a draft until the user decides.
pub fn classify(books: &mut Books, name: &str, origin: Origin, args: &Map<String, Value>) -> Value {
    if origin != Origin::Agent {
        return Refusal::new("forbidden", "Only the assistant proposes a document kind.")
            .into_result();
    }
    if let Err(r) = known(args, CLASSIFY_KEYS) {
        return r.into_result();
    }
    let mut out = service::write(books, name, origin, args, |tx| {
        let book = writes::book_in(tx, args)?;
        let kind =
            kind_arg(args, "kind")?.ok_or_else(|| Refusal::invalid("A `kind` is required."))?;
        let confidence = args
            .get("confidence")
            .and_then(Value::as_i64)
            .filter(|c| (0..=100).contains(c))
            .ok_or_else(|| Refusal::invalid("`confidence` is a whole number from 0 to 100."))?;
        let evidence = required(args, "evidence")?;
        let id = evidence_id(tx, book.id, evidence)?;
        tx.execute(
            "INSERT INTO evidence_classification(book_id,evidence_id,proposed_kind,confidence_int,status,created_at) VALUES(?1,?2,?3,?4,'draft',strftime('%Y-%m-%dT%H:%M:%SZ','now'))",
            params![book.id, id, kind, confidence],
        ).map_err(books_store::StoreError::from)?;
        let proposal = plumbing::ticket_for(tx, PROPOSAL, tx.last_insert_rowid())?;
        let evidence = lookup(tx, id)?.unwrap_or_else(|| evidence.to_string());
        let mut open = json!({
            "app": CARD_APP,
            "props": { "proposal": proposal, "evidence": evidence, "mode": "review" },
            "title": CARD_TITLE,
            "placement": "inline",
        });
        open["ticket"] = json!(plumbing::ticket_for(tx, "book", book.id)?);
        Ok(
            json!({ "proposal": proposal, "status": "draft", "kind": kind, "confidence": confidence, "_open": open }),
        )
    });
    // the card offer rides on _meta (spec 9.6), as books_show's does
    if let Some(open) = out["structuredContent"]
        .as_object_mut()
        .and_then(|o| o.remove("_open"))
    {
        out["content"] = json!([{ "type": "text", "text": out["structuredContent"].to_string() }]);
        out["_meta"] = json!({ "buwiz/open": open });
    }
    out
}

struct Proposal {
    id: i64,
    book_id: i64,
    evidence_id: i64,
    proposed_kind: String,
    status: String,
}

fn proposal(conn: &Connection, book: i64, ticket: &str) -> Result<Proposal, Refusal> {
    let id = subject(conn, PROPOSAL, ticket)?;
    conn.query_row(
        "SELECT id,book_id,evidence_id,proposed_kind,status FROM evidence_classification WHERE id=?1",
        [id],
        |r| Ok(Proposal { id: r.get(0)?, book_id: r.get(1)?, evidence_id: r.get(2)?, proposed_kind: r.get(3)?, status: r.get(4)? }),
    )
    .optional()
    .map_err(books_store::StoreError::from)?
    .filter(|p| p.book_id == book)
    .ok_or_else(|| Refusal::new("not_found", "That proposal is not in this book."))
}

fn links(conn: &Connection, evidence: i64) -> Result<Vec<Target>, Refusal> {
    let mut st = conn
        .prepare("SELECT entry_id,owed_id FROM evidence_link WHERE evidence_id=?1 ORDER BY id")
        .map_err(books_store::StoreError::from)?;
    let rows = st
        .query_map([evidence], |r| {
            Ok((r.get::<_, Option<i64>>(0)?, r.get::<_, Option<i64>>(1)?))
        })
        .map_err(books_store::StoreError::from)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(books_store::StoreError::from)?;
    Ok(rows
        .into_iter()
        .filter_map(|(e, o)| e.map(Target::Entry).or(o.map(Target::Owed)))
        .collect())
}

/// `books_app_evidence_kind`: the user accepts or discards a proposal.
pub fn decide(books: &mut Books, name: &str, origin: Origin, args: &Map<String, Value>) -> Value {
    if !origin.is_app() {
        return Refusal::new("forbidden", "Only the app may decide a document kind.").into_result();
    }
    if let Err(r) = known(args, KIND_KEYS) {
        return r.into_result();
    }
    service::write(books, name, origin, args, |tx| {
        let book = writes::book_in(tx, args)?;
        let ticket = required(args, "proposal")?;
        let p = proposal(tx, book.id, ticket)?;
        let chosen = kind_arg(args, "kind")?;
        if p.status != "draft" {
            return Err(Refusal::new(
                "conflict",
                "That proposal is already decided.",
            ));
        }
        match required(args, "decision")? {
            "accept" => {
                let kind = chosen.unwrap_or(p.proposed_kind.as_str()).to_string();
                for target in links(tx, p.evidence_id)? {
                    evidence::validate_target(tx, book.id, target, true)?;
                }
                let previous: String = tx
                    .query_row(
                        "SELECT kind FROM evidence WHERE id=?1",
                        [p.evidence_id],
                        |r| r.get(0),
                    )
                    .map_err(books_store::StoreError::from)?;
                tx.execute(
                    "UPDATE evidence SET kind=?1 WHERE id=?2",
                    params![kind, p.evidence_id],
                )
                .map_err(books_store::StoreError::from)?;
                tx.execute(
                    "UPDATE evidence_classification SET status='accepted',accepted_kind=?1,previous_kind=?2,decided_at=strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id=?3",
                    params![kind, previous, p.id],
                ).map_err(books_store::StoreError::from)?;
                Ok(
                    json!({ "proposal": ticket, "status": "accepted", "kind": kind, "route": route(&kind) }),
                )
            }
            "discard" => {
                if chosen.is_some() {
                    return Err(Refusal::invalid("A discard takes no `kind`."));
                }
                tx.execute(
                    "UPDATE evidence_classification SET status='discarded',decided_at=strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE id=?1",
                    [p.id],
                ).map_err(books_store::StoreError::from)?;
                Ok(json!({ "proposal": ticket, "status": "discarded" }))
            }
            _ => Err(Refusal::invalid("`decision` is accept or discard.")),
        }
    })
}

/// `books_app_evidence_review`: proposals as generic kind, confidence and status.
/// Read-only: tickets were issued by the writes that made the subjects.
pub fn review(conn: &Connection, args: &Map<String, Value>) -> Answer {
    known(args, REVIEW_KEYS)?;
    let book = writes::book_in(conn, args)?;
    let only = match text(args, "proposal") {
        Some(t) => Some(proposal(conn, book.id, t)?.id),
        None => None,
    };
    let evidence = text(args, "evidence")
        .map(|v| evidence_id(conn, book.id, v))
        .transpose()?;
    let mut st = conn
        .prepare("SELECT c.id,c.evidence_id,c.proposed_kind,c.confidence_int,c.status,c.accepted_kind,(SELECT COUNT(*) FROM evidence_link l WHERE l.evidence_id=c.evidence_id) FROM evidence_classification c WHERE c.book_id=?1 AND (?2 IS NULL OR c.id=?2) AND (?3 IS NULL OR c.evidence_id=?3) ORDER BY c.id DESC LIMIT 200")
        .map_err(books_store::StoreError::from)?;
    let rows = st
        .query_map(params![book.id, only, evidence], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, i64>(6)?,
            ))
        })
        .map_err(books_store::StoreError::from)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(books_store::StoreError::from)?;
    let mut out = Vec::new();
    for (id, ev, kind, confidence, status, accepted, linked) in rows {
        let (Some(p), Some(e)) = (ticket_of(conn, PROPOSAL, id)?, lookup(conn, ev)?) else {
            continue;
        };
        out.push(json!({ "proposal": p, "evidence": e, "kind": kind, "confidence": confidence, "status": status, "accepted_kind": accepted, "links": linked, "route": route(accepted.as_deref().unwrap_or(&kind)) }));
    }
    Ok(json!({ "rows": out }))
}

fn ticket_of(conn: &Connection, kind: &str, id: i64) -> Result<Option<String>, Refusal> {
    Ok(conn
        .query_row(
            "SELECT ticket FROM ticket WHERE subject_kind=?1 AND subject_id=?2 AND revoked_at IS NULL",
            params![kind, id],
            |r| r.get(0),
        )
        .optional()
        .map_err(books_store::StoreError::from)?)
}

/// Escapes text for HTML element content and attribute values.
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// One row of the optional double-entry table: account number, account name, debit, credit.
pub struct TableRow {
    pub number: String,
    pub name: String,
    pub debit: i64,
    pub credit: i64,
}

const STYLE: &str = "body{font:13px -apple-system,Helvetica,sans-serif;margin:24px;color:#111}h1{font-size:15px}img,object{max-width:100%;border:1px solid #ddd}table{border-collapse:collapse;margin-top:16px}td,th{border:1px solid #ddd;padding:4px 8px}td.n{text-align:right;font-variant-numeric:tabular-nums}";

/// The static page: no script, no remote URL, only `data:` content. Refuses
/// past `limit` bytes measured after base64 and HTML encoding.
/// Standard padded base64 (RFC 4648 §4), the alphabet a `data:` URI needs:
/// a browser drops an image written in the crate's unpadded base64url.
pub fn data_base64(bytes: &[u8]) -> String {
    let mut out: String = crate::b64::encode(bytes)
        .chars()
        .map(|c| match c {
            '-' => '+',
            '_' => '/',
            c => c,
        })
        .collect();
    while out.len() % 4 != 0 {
        out.push('=');
    }
    out
}

pub fn render(
    mime: &str,
    bytes: &[u8],
    table: Option<&[TableRow]>,
    limit: usize,
) -> Result<String, Refusal> {
    let data = format!("data:{};base64,{}", escape(mime), data_base64(bytes));
    let mut html = format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><style>{STYLE}</style></head><body><h1>Evidence</h1>"
    );
    if mime == "application/pdf" {
        html.push_str(&format!("<object data=\"{data}\" type=\"application/pdf\" width=\"100%\" height=\"900\"><p>This Buwiz Bot cannot show this PDF here. It is kept in Books unchanged.</p></object>"));
    } else {
        html.push_str(&format!("<img src=\"{data}\" alt=\"Evidence image\">"));
    }
    if let Some(rows) = table {
        html.push_str("<table><tr><th>Account</th><th>Debit</th><th>Credit</th></tr>");
        for r in rows {
            let amt = |c: i64| {
                if c == 0 {
                    String::new()
                } else {
                    escape(&money::format(c))
                }
            };
            html.push_str(&format!(
                "<tr><td>{} {}</td><td class=\"n\">{}</td><td class=\"n\">{}</td></tr>",
                escape(&r.number),
                escape(&r.name),
                amt(r.debit),
                amt(r.credit)
            ));
        }
        html.push_str("</table>");
    }
    html.push_str("</body></html>");
    if html.len() > limit {
        return Err(Refusal::new(
            "too_large",
            "This document is too large to preview. It is kept in Books unchanged.",
        ));
    }
    Ok(html)
}

fn entry_table(conn: &Connection, entry: i64) -> Result<Vec<TableRow>, Refusal> {
    let mut st = conn
        .prepare("SELECT a.number,a.name,l.debit,l.credit FROM line l JOIN account a ON a.account_id=l.account_id WHERE l.entry_id=?1 ORDER BY l.line_no")
        .map_err(books_store::StoreError::from)?;
    let rows = st
        .query_map([entry], |r| {
            Ok(TableRow {
                number: r.get(0)?,
                name: r.get(1)?,
                debit: r.get(2)?,
                credit: r.get(3)?,
            })
        })
        .map_err(books_store::StoreError::from)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(books_store::StoreError::from)?;
    Ok(rows)
}

/// `books_app_evidence_preview`: an MCP embedded `text/html` resource for `buwiz.preview`.
pub fn preview(books: &mut Books, origin: Origin, args: &Map<String, Value>) -> Value {
    if !origin.is_app() {
        return Refusal::new("forbidden", "Only the app may preview evidence.").into_result();
    }
    // a read: never open (and so create) the store on a fresh install
    if !books.exists() {
        return Refusal::new(
            "not_set_up",
            "There is no book yet. Open Books to make one.",
        )
        .into_result();
    }
    let page = (|| {
        known(args, PREVIEW_KEYS)?;
        let store = books.store()?;
        let conn = store.conn();
        let book = writes::book_in(conn, args)?;
        let id = evidence_id(conn, book.id, required(args, "evidence")?)?;
        let table = match text(args, "entry") {
            Some(t) => {
                let entry = subject(conn, "entry", t)?;
                if !links(conn, id)?
                    .iter()
                    .any(|l| matches!(l, Target::Entry(e) if *e == entry))
                {
                    return Err(Refusal::new(
                        "not_found",
                        "That evidence is not linked to that entry.",
                    ));
                }
                Some(entry_table(conn, entry)?)
            }
            None => None,
        };
        let (mime, bytes): (String, Vec<u8>) = conn
            .query_row("SELECT mime,bytes FROM evidence WHERE id=?1", [id], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .map_err(books_store::StoreError::from)?;
        render(&mime, &bytes, table.as_deref(), PREVIEW_MAX_BYTES)
    })();
    match page {
        Ok(html) => {
            let uri = format!(
                "books-preview://evidence/{}",
                plumbing::new_ticket()
                    .unwrap_or_default()
                    .trim_start_matches("tkt_")
            );
            json!({
                "content": [{ "type": "resource", "resource": { "uri": uri, "mimeType": "text/html", "text": html } }],
                "structuredContent": { "title": "Evidence" },
                "isError": false,
            })
        }
        Err(r) => r.into_result(),
    }
}

fn schema() -> Value {
    let kind = json!({ "type": "string", "enum": KINDS });
    let ticket = json!({ "type": "string", "maxLength": 30 });
    json!({ "type": "object", "properties": {
        "client_request_id": { "type": "string" }, "book": ticket, "evidence": ticket, "proposal": ticket, "entry": ticket,
        "kind": kind, "confidence": { "type": "integer", "minimum": 0, "maximum": 100 },
        "decision": { "type": "string", "enum": ["accept", "discard"] }
    }})
}

fn schema_for(keys: &[&str], required: &[&str]) -> Value {
    let mut out = schema();
    if let Some(p) = out["properties"].as_object_mut() {
        p.retain(|k, _| keys.contains(&k.as_str()));
    }
    out["required"] = json!(required);
    out["additionalProperties"] = json!(false);
    out
}

pub fn classify_schema() -> Value {
    schema_for(
        CLASSIFY_KEYS,
        &["client_request_id", "evidence", "kind", "confidence"],
    )
}
pub fn kind_schema() -> Value {
    schema_for(KIND_KEYS, &["client_request_id", "proposal", "decision"])
}
pub fn preview_schema() -> Value {
    schema_for(PREVIEW_KEYS, &["evidence"])
}
pub fn review_schema() -> Value {
    schema_for(REVIEW_KEYS, &[])
}
