//! The needs-review inbox (M10 parity, ledger-1): what the assistant or a
//! routine proposed on a draft, field by field, the confidence the Bot's
//! assistant reported for each (Books computes none), and the likely
//! duplicates a post must be told to keep (`keep_both`). The assistant
//! sees entry numbers and tickets of duplicates, never their payee, memo
//! or amount.

use std::collections::HashSet;

use books_core::ledger::Status;
use books_store::books::Book;
use books_store::entries::{self, DraftInput, Entry, Form};
use books_store::plumbing;
use books_store::review::{self, Field};
use books_store::rusqlite::Connection;
use serde_json::{Map, Value, json};

use crate::service::{Origin, Refusal};

/// Below this confidence a field asks the user to check it.
pub const LOW_CONFIDENCE: u8 = 70;

/// `extracted.fields` of a draft request: each named field with its
/// optional confidence, 0 to 100. Other keys of `extracted` belong to
/// other features and are left alone.
pub fn extracted(args: &Map<String, Value>) -> Result<Vec<(Field, Option<u8>)>, Refusal> {
    let Some(x) = args.get("extracted").filter(|v| !v.is_null()) else {
        return Ok(Vec::new());
    };
    let x = x
        .as_object()
        .ok_or_else(|| Refusal::invalid("`extracted` is an object."))?;
    let Some(fields) = x.get("fields").filter(|v| !v.is_null()) else {
        return Ok(Vec::new());
    };
    let fields = fields
        .as_object()
        .ok_or_else(|| Refusal::invalid("`extracted.fields` is an object."))?;
    let mut out = Vec::new();
    for (name, note) in fields {
        let field = Field::parse(name).ok_or_else(|| {
            Refusal::invalid("`extracted.fields` names date, amount, account, to_account, category, payee, memo, reference or lines.")
        })?;
        let confidence = match note.get("confidence") {
            None | Some(Value::Null) => None,
            Some(c) => Some(
                c.as_u64()
                    .and_then(|c| u8::try_from(c).ok())
                    .filter(|c| *c <= 100)
                    .ok_or_else(|| {
                        Refusal::invalid("A confidence is a whole percent, 0 to 100.")
                    })?,
            ),
        };
        if !note.is_object() {
            return Err(Refusal::invalid("Each extracted field is an object."));
        }
        out.push((field, confidence));
    }
    Ok(out)
}

/// What a draft says for one field, comparable across saves.
fn value(input: &DraftInput, field: Field) -> Option<String> {
    let line = |i: usize| input.lines.get(i).map(|l| l.account_id.to_string());
    let text = |t: &Option<String>| {
        t.as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_string)
    };
    match (field, input.form) {
        (Field::Date, _) => Some(input.date.to_string()),
        (Field::Amount, _) => Some(input.lines.iter().map(|l| l.debit).sum::<i64>().to_string()),
        (Field::Payee, _) => text(&input.payee),
        (Field::Memo, _) => text(&input.memo),
        (Field::Reference, _) => text(&input.reference),
        (Field::Account, Form::MoneyIn) | (Field::Category, Form::MoneyOut) => line(0),
        (Field::Category, Form::MoneyIn)
        | (Field::Account, Form::MoneyOut)
        | (Field::Account, Form::Transfer) => line(1),
        (Field::ToAccount, Form::Transfer) => line(0),
        (Field::Lines, Form::Journal) => Some(
            input
                .lines
                .iter()
                .map(|l| format!("{}:{}:{}", l.account_id, l.debit, l.credit))
                .collect::<Vec<_>>()
                .join(";"),
        ),
        _ => None,
    }
}

/// A stored draft as a [`DraftInput`], to compare with the next save.
pub fn snapshot(conn: &Connection, id: i64) -> Result<Option<DraftInput>, Refusal> {
    let Some(e) = entries::get(conn, id)? else {
        return Ok(None);
    };
    let lines = entries::lines(conn, id)?
        .into_iter()
        .map(|l| entries::LineInput {
            account_id: l.account_id,
            debit: l.debit,
            credit: l.credit,
            facts: l.own,
        })
        .collect();
    Ok(Some(DraftInput {
        date: e.date,
        form: e.form,
        payee: e.payee,
        memo: e.memo,
        reference: e.reference,
        needs_review: e.needs_review,
        lines,
    }))
}

/// The arguments that save a field in the app's edit form.
fn saved_by_app(args: &Map<String, Value>, field: Field) -> bool {
    match field {
        Field::Amount => args.contains_key("amount") || args.contains_key("lines"),
        f => args.contains_key(f.as_str()),
    }
}

/// Note a saved draft. The assistant or a routine marks each field whose
/// value changed (every non-empty one on a new draft) and leaves the
/// confidences it reported; a save in the app clears the notes of the
/// fields it saved, since the user has now set them.
pub fn record_draft(
    conn: &Connection,
    entry_id: i64,
    before: Option<&DraftInput>,
    after: &DraftInput,
    args: &Map<String, Value>,
    noted: &[(Field, Option<u8>)],
    origin: Origin,
) -> Result<(), Refusal> {
    if origin.is_app() {
        let saved: Vec<Field> = Field::ALL
            .into_iter()
            .filter(|f| saved_by_app(args, *f))
            .collect();
        return Ok(review::clear_fields(conn, entry_id, &saved)?);
    }
    let by = match origin {
        Origin::Agent => "agent",
        Origin::Routine => "routine",
        _ => return Ok(()),
    };
    let confidence = |f: Field| noted.iter().find(|(n, _)| *n == f).and_then(|(_, c)| *c);
    let mut changed = Vec::new();
    let mut kept = Vec::new();
    for f in Field::ALL {
        let now = value(after, f);
        let differs = match before {
            Some(b) => value(b, f) != now,
            None => now.is_some(),
        };
        if differs {
            changed.push((f, confidence(f)));
        } else if let Some(c) = confidence(f) {
            kept.push((f, Some(c)));
        }
    }
    review::set_fields(conn, entry_id, &changed, Some(by))?;
    review::set_fields(conn, entry_id, &kept, None)?;
    Ok(())
}

/// The likely duplicates of a draft as the draft result names them.
pub fn duplicate_labels(conn: &Connection, book: &Book, id: i64) -> Result<Value, Refusal> {
    let mut out = Vec::new();
    for other in review::unacked_duplicates(conn, book.id, id)? {
        if let Some(e) = entries::get(conn, other)? {
            out.push(json!(format!("entry {:04}", e.no)));
        }
    }
    Ok(json!(out))
}

/// `books_app_post_entry` on a draft: refuse while it has likely
/// duplicates the user has not kept it alongside, unless `keep_both`,
/// which records the choice.
pub fn duplicate_gate(
    conn: &Connection,
    book: &Book,
    entry_id: i64,
    keep_both: bool,
) -> Result<(), Refusal> {
    let draft = entries::get(conn, entry_id)?.is_some_and(|e| e.status == Status::Draft);
    if !draft {
        return Ok(());
    }
    let open = review::unacked_duplicates(conn, book.id, entry_id)?;
    if open.is_empty() {
        return Ok(());
    }
    if keep_both {
        return Ok(review::ack(conn, entry_id, &open)?);
    }
    let mut names = Vec::new();
    for id in &open {
        if let Some(e) = entries::get(conn, *id)? {
            names.push(format!("{:04}", e.no));
        }
    }
    Err(Refusal::new(
        "likely_duplicate",
        format!(
            "This looks like entry {}. Keep both to post it anyway.",
            names.join(", ")
        ),
    ))
}

/// Who proposed an entry, when it was not the user or an old-books import.
pub fn proposed_by(e: &Entry) -> Value {
    match e.origin.as_str() {
        "app" | "import" => Value::Null,
        o => json!(o),
    }
}

/// A draft with a field below [`LOW_CONFIDENCE`].
pub fn low_confidence(conn: &Connection, e: &Entry) -> Result<bool, Refusal> {
    if e.status != Status::Draft {
        return Ok(false);
    }
    Ok(review::fields(conn, e.id)?
        .iter()
        .any(|n| n.confidence.is_some_and(|c| c < LOW_CONFIDENCE)))
}

/// The drafts of a list that need review here: a field below
/// [`LOW_CONFIDENCE`] or likely duplicates still to decide.
pub fn flagged<'a>(
    conn: &Connection,
    list: impl Iterator<Item = &'a Entry>,
) -> Result<HashSet<i64>, Refusal> {
    let mut out = HashSet::new();
    for e in list.filter(|e| e.status == Status::Draft) {
        if low_confidence(conn, e)?
            || !review::unacked_duplicates(conn, e.book_id, e.id)?.is_empty()
        {
            out.insert(e.id);
        }
    }
    Ok(out)
}

/// The ledger row's review keys.
pub fn row_keys(conn: &Connection, e: &Entry, row: &mut Value) -> Result<(), Refusal> {
    row["proposed_by"] = proposed_by(e);
    row["low_confidence"] = json!(low_confidence(conn, e)?);
    row["likely_duplicate"] = json!(!review::unacked_duplicates(conn, e.book_id, e.id)?.is_empty());
    Ok(())
}

/// The drawer's `review`: who proposed it, the field notes, and the likely
/// duplicates (with their payee and amount for the app only).
pub fn review_json(conn: &Connection, e: &Entry, origin: Origin) -> Result<Value, Refusal> {
    let fields: Vec<Value> = review::fields(conn, e.id)?
        .into_iter()
        .map(|n| json!({ "field": n.field.as_str(), "confidence": n.confidence, "changed_by": n.changed_by }))
        .collect();
    let mut duplicates = Vec::new();
    for id in review::unacked_duplicates(conn, e.book_id, e.id)? {
        let Some(o) = entries::get(conn, id)? else {
            continue;
        };
        let mut d = json!({
            "ticket": plumbing::ticket_for(conn, "entry", o.id)?,
            "no": format!("{:04}", o.no),
            "date": o.date.to_string(),
            "status": o.status.as_str(),
        });
        if origin.is_app() {
            let debits: i64 = entries::lines(conn, o.id)?.iter().map(|l| l.debit).sum();
            d["payee"] = json!(o.payee);
            d["amount_shown"] = json!(books_core::money::display(debits));
        }
        duplicates.push(d);
    }
    Ok(json!({ "proposed_by": proposed_by(e), "fields": fields, "likely_duplicates": duplicates }))
}
