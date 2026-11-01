//! Receipt extraction provenance on drafts: what the assistant read off a
//! piece of evidence, how sure it was, and what the user changed after.
//! Review metadata only; ledger facts still come from `writes::draft`.

use books_core::money;
use books_store::evidence::{self, Target};
use books_store::rusqlite::{Connection, OptionalExtension as _, params};
use books_store::{now_iso, plumbing};
use serde_json::{Map, Value, json};

use crate::service::{Origin, Refusal};
use crate::tools::evidence_tools;
use crate::writes;

/// At or above this a confidence counts as sure; below it the draft needs review.
pub const SURE: i64 = 80;
pub const FIELDS: [&str; 5] = ["date", "amount", "category", "person", "reference"];
const MAX_VALUE: usize = 200;
/// Declared by the evidence preview task; absent until then.
const PREVIEW: &str = concat!("books_app_evidence", "_preview");

pub fn schema() -> Value {
    let confidence = json!({ "type": "integer", "minimum": 0, "maximum": 100 });
    json!({
        "type": "object",
        "required": ["evidence"],
        "additionalProperties": false,
        "description": "What the assistant read off evidence the user imported in Books. Review metadata only: it never fills the draft's own fields.",
        "properties": {
            "evidence": { "type": "string", "description": "An evidence ticket from books_evidence_list." },
            "confidence": confidence.clone(),
            "fields": {
                "type": "array",
                "maxItems": 5,
                "items": {
                    "type": "object",
                    "required": ["field", "value"],
                    "additionalProperties": false,
                    "properties": {
                        "field": { "type": "string", "enum": FIELDS },
                        "value": { "type": "string", "maxLength": MAX_VALUE, "description": "Text as read; an amount is decimal text with two places." },
                        "confidence": confidence
                    }
                }
            }
        }
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub field: &'static str,
    pub value: String,
    pub confidence: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extracted {
    pub evidence: i64,
    pub confidence: Option<i64>,
    pub fields: Vec<Field>,
}

impl Extracted {
    /// Missing confidence counts as unsure.
    pub fn needs_review(&self) -> bool {
        std::iter::once(self.confidence)
            .chain(self.fields.iter().map(|f| f.confidence))
            .any(|c| c.is_none_or(|c| c < SURE))
    }
}

fn confidence(v: Option<&Value>) -> Result<Option<i64>, Refusal> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_i64()
            .filter(|c| (0..=100).contains(c))
            .map(Some)
            .ok_or_else(|| Refusal::invalid("A confidence is a whole number from 0 to 100.")),
    }
}

fn tin_shaped(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n == "tin"
        || n.contains("tin_")
        || n.contains("_tin")
        || n.contains("tax_id")
        || n.contains("taxpayer")
}

/// Check `extraction` before anything is written.
pub fn parse(conn: &Connection, book: i64, raw: &Value) -> Result<Extracted, Refusal> {
    let o = raw
        .as_object()
        .ok_or_else(|| Refusal::invalid("`extraction` is an object."))?;
    if !o
        .keys()
        .all(|k| ["evidence", "confidence", "fields"].contains(&k.as_str()))
    {
        return Err(Refusal::invalid("`extraction` has an unknown argument."));
    }
    let ticket = o
        .get("evidence")
        .and_then(Value::as_str)
        .ok_or_else(|| Refusal::invalid("`extraction.evidence` is an evidence ticket."))?;
    let evidence = evidence_tools::evidence_id(conn, book, ticket)?;
    let list = match o.get("fields") {
        None => Vec::new(),
        Some(Value::Array(a)) if a.len() <= FIELDS.len() => a.clone(),
        Some(_) => {
            return Err(Refusal::invalid(
                "`extraction.fields` is a list of at most five fields.",
            ));
        }
    };
    let mut fields: Vec<Field> = Vec::new();
    for f in &list {
        let f = f
            .as_object()
            .ok_or_else(|| Refusal::invalid("Each extracted field is an object."))?;
        if !f
            .keys()
            .all(|k| ["field", "value", "confidence"].contains(&k.as_str()))
        {
            return Err(Refusal::invalid(
                "An extracted field has an unknown argument.",
            ));
        }
        let name = f.get("field").and_then(Value::as_str).unwrap_or_default();
        if tin_shaped(name) {
            return Err(Refusal::new("tin_refused", "Books does not keep a TIN."));
        }
        let field = FIELDS.iter().find(|k| **k == name).ok_or_else(|| {
            Refusal::invalid("An extracted field is date, amount, category, person or reference.")
        })?;
        if fields.iter().any(|x| x.field == *field) {
            return Err(Refusal::invalid("An extracted field appears twice."));
        }
        let value = f
            .get("value")
            .and_then(Value::as_str)
            .ok_or_else(|| Refusal::invalid("An extracted value is text."))?;
        if value.chars().count() > MAX_VALUE || value.chars().any(char::is_control) {
            return Err(Refusal::invalid(
                "An extracted value is at most 200 plain characters.",
            ));
        }
        books_core::privacy::refuse_tin(&format!("extracted {field}"), value)
            .map_err(|m| Refusal::new("tin_refused", m))?;
        let value = match *field {
            "amount" => {
                money::display(money::parse(value).ok().filter(|c| *c > 0).ok_or_else(|| {
                    Refusal::invalid(
                        "An extracted amount is decimal text with two places, more than 0.00.",
                    )
                })?)
                .replace(',', "")
            }
            "date" => books_core::date::Date::parse(value)
                .map(|_| value.to_string())
                .ok_or_else(|| Refusal::invalid("An extracted date is YYYY-MM-DD."))?,
            _ => value.trim().to_string(),
        };
        fields.push(Field {
            field,
            value,
            confidence: confidence(f.get("confidence"))?,
        });
    }
    Ok(Extracted {
        evidence,
        confidence: confidence(o.get("confidence"))?,
        fields,
    })
}

fn fields_json(fields: &[Field]) -> String {
    Value::Array(
        fields
            .iter()
            .map(|f| json!({ "field": f.field, "value": f.value, "confidence": f.confidence }))
            .collect(),
    )
    .to_string()
}

struct Stored {
    id: i64,
    evidence: i64,
    confidence: Option<i64>,
    fields: Vec<Field>,
    proposed_by: String,
}

fn stored(conn: &Connection, entry: i64) -> Result<Option<Stored>, Refusal> {
    let row: Option<(i64, i64, Option<i64>, String, String)> = conn
        .query_row(
            "SELECT id, evidence_id, confidence_int, fields_json, proposed_by FROM entry_extraction WHERE entry_id = ?1",
            [entry],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()
        .map_err(books_store::StoreError::from)?;
    let Some((id, evidence, confidence, raw, proposed_by)) = row else {
        return Ok(None);
    };
    let fields = serde_json::from_str::<Vec<Value>>(&raw)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|f| {
            Some(Field {
                field: FIELDS.iter().find(|k| Some(**k) == f["field"].as_str())?,
                value: f["value"].as_str()?.to_string(),
                confidence: f["confidence"].as_i64(),
            })
        })
        .collect();
    Ok(Some(Stored {
        id,
        evidence,
        confidence,
        fields,
        proposed_by,
    }))
}

/// The entry's present value for an extracted field, as the drawer shows it.
fn current(conn: &Connection, entry: i64, field: &str) -> Result<Option<String>, Refusal> {
    let sql = match field {
        "date" => "SELECT entry_date FROM entry WHERE entry_id = ?1",
        "person" => "SELECT payee FROM entry WHERE entry_id = ?1",
        "reference" => "SELECT reference FROM entry WHERE entry_id = ?1",
        "category" => "SELECT a.number || ' ' || a.name FROM line l JOIN account a ON a.account_id = l.account_id
             WHERE l.entry_id = ?1 AND a.type NOT IN ('asset','liability','equity') ORDER BY l.line_id LIMIT 1",
        _ => {
            let total: i64 = conn
                .query_row("SELECT coalesce(sum(debit),0) FROM line WHERE entry_id = ?1", [entry], |r| r.get(0))
                .map_err(books_store::StoreError::from)?;
            return Ok((total > 0).then(|| money::display(total).replace(',', "")));
        }
    };
    Ok(conn
        .query_row(sql, [entry], |r| r.get::<_, Option<String>>(0))
        .optional()
        .map_err(books_store::StoreError::from)?
        .flatten())
}

/// The top-level draft argument a user edit of a field comes through.
fn edited(args: &Map<String, Value>, field: &str) -> bool {
    match field {
        "person" => args.contains_key("person") || args.contains_key("payee"),
        f => args.contains_key(f),
    }
}

fn latest(conn: &Connection, extraction: i64, field: &str) -> Result<Option<String>, Refusal> {
    Ok(conn
        .query_row(
            "SELECT value FROM entry_extraction_correction WHERE extraction_id = ?1 AND field = ?2 ORDER BY id DESC LIMIT 1",
            params![extraction, field],
            |r| r.get(0),
        )
        .optional()
        .map_err(books_store::StoreError::from)?)
}

/// `books_entry_draft` with optional `extraction`, in the write's transaction.
pub fn draft(
    conn: &Connection,
    args: &Map<String, Value>,
    origin: Origin,
) -> Result<Value, Refusal> {
    let book = writes::book_in(conn, args)?.id;
    let extracted = args
        .get("extraction")
        .map(|raw| parse(conn, book, raw))
        .transpose()?;
    let mut plain = args.clone();
    plain.remove("extraction");
    // an assistant edit must not clear a review flag the entry already had
    let flagged_before = match plain.get("entry").and_then(Value::as_str) {
        Some(t) if !origin.is_app() => match plumbing::subject(conn, t)? {
            Some((k, id)) if k == "entry" => conn
                .query_row(
                    "SELECT needs_review FROM entry WHERE entry_id = ?1",
                    [id],
                    |r| r.get::<_, bool>(0),
                )
                .optional()
                .map_err(books_store::StoreError::from)?
                .unwrap_or(false),
            _ => false,
        },
        _ => false,
    };
    let out = writes::draft(conn, &plain, origin)?;
    let entry = out["_subject"][1]
        .as_i64()
        .ok_or_else(|| Refusal::new("store", "The draft vanished."))?;
    let had = stored(conn, entry)?;
    let mut out = out;
    match (extracted, had) {
        (Some(_), Some(_)) => {
            return Err(Refusal::new(
                "conflict",
                "That draft already has an extraction; edit the draft instead.",
            ));
        }
        (Some(x), None) => {
            evidence::link(conn, book, x.evidence, Target::Entry(entry))?;
            conn.execute(
                "INSERT INTO entry_extraction(entry_id, evidence_id, fields_json, confidence_int, proposed_by, created_at) VALUES (?1,?2,?3,?4,?5,?6)",
                params![entry, x.evidence, fields_json(&x.fields), x.confidence, origin.as_str(), now_iso()],
            )
            .map_err(books_store::StoreError::from)?;
            let review = x.needs_review();
            // writes::draft cleared the flag; an unsure reading sets it, and an assistant
            // edit keeps the review the entry already had (a bank line, say)
            if review || flagged_before {
                conn.execute(
                    "UPDATE entry SET needs_review = 1 WHERE entry_id = ?1",
                    [entry],
                )
                .map_err(books_store::StoreError::from)?;
            }
            out["extraction"] = json!({ "needs_review": review, "fields": x.fields.len() });
        }
        (None, Some(s)) => {
            // a later edit: the assistant's reading stays; the change is recorded beside it
            for f in &s.fields {
                if !edited(&plain, f.field) {
                    continue;
                }
                let now = current(conn, entry, f.field)?.unwrap_or_default();
                let before = latest(conn, s.id, f.field)?.unwrap_or_else(|| f.value.clone());
                if now != before {
                    conn.execute(
                        "INSERT INTO entry_extraction_correction(extraction_id, field, value, corrected_by, created_at) VALUES (?1,?2,?3,?4,?5)",
                        params![s.id, f.field, now, origin.as_str(), now_iso()],
                    )
                    .map_err(books_store::StoreError::from)?;
                }
            }
            // only the user (the app) clears review by editing; posting needs no flag change
            let unsure = Extracted {
                evidence: s.evidence,
                confidence: s.confidence,
                fields: s.fields.clone(),
            }
            .needs_review();
            if !origin.is_app() && (unsure || flagged_before) {
                conn.execute(
                    "UPDATE entry SET needs_review = 1 WHERE entry_id = ?1",
                    [entry],
                )
                .map_err(books_store::StoreError::from)?;
            }
        }
        (None, None) => {}
    }
    Ok(out)
}

fn label(c: Option<i64>) -> &'static str {
    match c {
        Some(c) if c >= SURE => "Sure",
        Some(c) if c >= 50 => "Unsure",
        Some(_) => "Low",
        None => "Not given",
    }
}

/// Extra keys for `books_entry_get`: the app sees the extraction and evidence
/// summaries; anyone else sees counts.
pub fn decorate(
    conn: &Connection,
    book: i64,
    entry_ticket: &str,
    origin: Origin,
    out: &mut Value,
) -> Result<(), Refusal> {
    let entry = match plumbing::subject(conn, entry_ticket)? {
        Some((k, id)) if k == "entry" => id,
        _ => return Ok(()),
    };
    let linked = evidence::list(conn, book, None, Some(Target::Entry(entry)), 0, 20)?;
    let s = stored(conn, entry)?;
    if !origin.is_app() {
        out["evidence_count"] = json!(linked.len());
        out["extracted"] = json!(s.is_some());
        return Ok(());
    }
    let mut summaries = Vec::new();
    for e in &linked {
        if let Some(t) = evidence_tools::lookup(conn, e.id)? {
            summaries.push(
                json!({ "evidence": t, "label": evidence_tools::label(e), "source": e.source }),
            );
        }
    }
    out["evidence"] = json!(summaries);
    let Some(s) = s else {
        return Ok(());
    };
    let mut fields = Vec::new();
    for f in &s.fields {
        let now = current(conn, entry, f.field)?;
        let corrected = latest(conn, s.id, f.field)?.is_some();
        fields.push(json!({
            "field": f.field,
            "suggested": f.value,
            "current": now,
            "corrected": corrected,
            "confidence": f.confidence,
            "confidence_label": label(f.confidence),
        }));
    }
    let x = Extracted {
        evidence: s.evidence,
        confidence: s.confidence,
        fields: s.fields.clone(),
    };
    out["extraction"] = json!({
        "evidence": evidence_tools::lookup(conn, s.evidence)?,
        "proposed_by": if s.proposed_by == "app" { "You" } else { "Assistant" },
        "confidence": s.confidence,
        "confidence_label": label(s.confidence),
        "needs_review": x.needs_review(),
        "fields": fields,
        "preview": crate::tools::find(PREVIEW).map(|t| t.name),
    });
    Ok(())
}
