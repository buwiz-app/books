//! M7: the hand-off to the BIR draft, model-mediated (docs/01 9.2, DD-11).
//!
//! 1. The owner clicks "Hand to BIR draft": `books_app_handoff_create`
//!    records the ready proposal's input boxes and the app attaches the
//!    hand-off to the chat (`rox.attach("proposal", ticket)`).
//! 2. The assistant reads it with `books_handoff_get` and fills the BIR
//!    draft once with source `document` (D3), reads it back, validates and
//!    saves a draft. Books never calls BIR, and nothing is ever sent.
//! 3. The owner clicks "BIR shows these values":
//!    `books_app_handoff_confirm` records `fill_args_sha256` (B5).
//!
//! A hand-off is current while the proposal's inputs hash is unchanged; a
//! later change (a line, a reopened month, a certificate's status) makes it
//! out of date, says what changed, and `books_handoff_get` refuses it.

use std::collections::{BTreeMap, BTreeSet};

use books_core::date::{MONTH_NAMES, Period};
use books_core::input::FormCode;
use books_store::Store;
use books_store::books::Book;
use books_store::entries;
use books_store::handoffs::{self, Handoff, NewHandoff};
use books_store::months;
use books_store::plumbing;
use books_store::rusqlite::Connection;
use serde_json::{Map, Value, json};

use crate::forms::{self, Built};
use crate::service::{Answer, Origin, Refusal, required, text, with_subject};
use crate::writes;

/// The source label the BIR fill takes (D3; never `user`, never `from_card`).
pub const SOURCE: &str = "document";

fn form_arg(args: &Map<String, Value>) -> Result<FormCode, Refusal> {
    FormCode::parse(required(args, "form")?)
        .ok_or_else(|| Refusal::invalid("`form` is 2551Q, 1601C or 1701MS."))
}

/// `{key: [[line_id, amount], ...]}`: what each box was made of.
fn lineage_of(built: &Built) -> Value {
    let mut out = Map::new();
    for key in built.proposal.fill_args.keys() {
        let lines: Vec<Value> = built
            .proposal
            .lineage
            .get(key)
            .map(|l| {
                l.lines
                    .iter()
                    .map(|x| json!([x.line_id, x.amount]))
                    .collect()
            })
            .unwrap_or_default();
        out.insert(key.clone(), Value::Array(lines));
    }
    Value::Object(out)
}

/// The neutral title of a hand-off (no amount): `BIR hand-off · 2551Q Q3 2026`.
pub fn title(form: &str, period: &str) -> String {
    let label = Period::parse(period).map_or_else(|| period.to_string(), |p| p.label());
    let code = match form {
        "1601C" => "1601-C",
        "1701MS" => "1701-MS",
        f => f,
    };
    format!("BIR hand-off · {code} {label}")
}

fn rebuild(c: &Connection, book: &Book, h: &Handoff) -> Result<Built, Refusal> {
    let form = FormCode::parse(&h.form)
        .ok_or_else(|| Refusal::new("store", "A hand-off names an unknown form."))?;
    let ctx: Value = serde_json::from_str(&h.form_context).unwrap_or_else(|_| json!({}));
    forms::build(c, book, form, &h.period_key, &ctx)
}

/// Whether a hand-off still matches the books, and if not, what changed.
pub fn changed(
    c: &Connection,
    book: &Book,
    h: &Handoff,
    now: &Built,
) -> Result<Option<String>, Refusal> {
    let p = &now.proposal;
    if p.inputs_hash == h.inputs_hash && p.status == "ready" {
        return Ok(None);
    }
    // a month of the period reopened
    if let Some(period) = Period::parse(&h.period_key) {
        for (m, closed) in months::months(c, book.id, books_core::date::Date::today())? {
            if !closed && period.contains(m.start) {
                return Ok(Some(format!(
                    "{} reopened",
                    MONTH_NAMES[usize::from(m.start.month - 1)]
                )));
            }
        }
    }
    // lines added, removed or changed
    let then: Value = serde_json::from_str(&h.lineage).unwrap_or_else(|_| json!({}));
    let mut before: BTreeMap<String, String> = BTreeMap::new();
    for lines in then.as_object().into_iter().flat_map(|o| o.values()) {
        for l in lines.as_array().into_iter().flatten() {
            before.insert(
                l[0].as_str().unwrap_or("").to_string(),
                l[1].as_str().unwrap_or("").to_string(),
            );
        }
    }
    let mut after: BTreeMap<String, String> = BTreeMap::new();
    for key in now.proposal.lineage.keys() {
        for l in &now.proposal.lineage[key].lines {
            after.insert(l.line_id.clone(), l.amount.clone());
        }
    }
    let ids: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
    let moved: Vec<&String> = ids
        .into_iter()
        .filter(|id| before.get(*id) != after.get(*id))
        .collect();
    if let Some(first) = moved.first() {
        let n = moved.len();
        let what = first
            .strip_prefix('e')
            .and_then(|x| x.split('.').next())
            .and_then(|x| x.parse::<i64>().ok())
            .map(|id| entries::get(c, id))
            .transpose()?
            .flatten()
            .map_or_else(
                || "An opening figure".to_string(),
                |e| {
                    format!(
                        "{}{}",
                        e.date.short(),
                        e.payee.map(|p| format!(" · {p}")).unwrap_or_default()
                    )
                },
            );
        return Ok(Some(format!(
            "{what}: {n} line{} changed",
            if n == 1 { "" } else { "s" }
        )));
    }
    Ok(Some("the inputs of these totals changed".into()))
}

/// The hand-off of a form and period as the Forms page shows it.
pub fn state(
    c: &Connection,
    book: &Book,
    form: FormCode,
    period: &str,
    now: &Built,
) -> Result<Value, Refusal> {
    let Some(h) = handoffs::current(c, book.id, form.as_str(), period)? else {
        return Ok(Value::Null);
    };
    let ticket = plumbing::ticket_for(c, "handoff", h.id)?;
    let boxes = serde_json::from_str::<Map<String, Value>>(&h.fill_args).map_or(0, |m| m.len());
    // the stored instant is UTC; the person reads the day in Manila
    let day = |iso: &str| {
        books_core::date::Date::manila_from_iso_instant(iso).map_or_else(String::new, |d| d.short())
    };
    let (status, label, detail) = match changed(c, book, &h, now)? {
        Some(why) => ("stale", "Out of date".to_string(), why),
        None if h.status == "confirmed" => (
            "confirmed",
            "In BIR draft".to_string(),
            format!(
                "Handed over {} · {boxes} box{}",
                day(h.confirmed_at.as_deref().unwrap_or(&h.created_at)),
                if boxes == 1 { "" } else { "es" }
            ),
        ),
        None => (
            "handed",
            "Handed over".to_string(),
            "Attached to the chat. When the BIR draft shows these values, confirm here."
                .to_string(),
        ),
    };
    Ok(
        json!({ "ticket": ticket, "status": status, "label": label, "detail": detail, "boxes": boxes }),
    )
}

/// `books_app_handoff_create`: the owner's click on "Hand to BIR draft".
pub fn create(c: &Connection, args: &Map<String, Value>) -> Answer {
    let book = writes::book_in(c, args)?;
    let form = form_arg(args)?;
    let period = text(args, "period").map_or_else(|| forms::default_period(form), str::to_string);
    let ctx = args
        .get("form_context")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let built = forms::build(c, &book, form, &period, &ctx)?;
    let p = &built.proposal;
    if p.status != "ready" {
        let why = p
            .blocking
            .first()
            .map_or_else(|| "it is not ready".to_string(), |b| b.detail.clone());
        return Err(Refusal::new(
            "not_ready",
            format!("This form cannot be handed over yet: {why}."),
        ));
    }
    if p.fill_args.is_empty() {
        return Err(Refusal::new(
            "not_ready",
            "Every box of this form is held, so there is nothing to hand over.",
        ));
    }
    let fill_args =
        serde_json::to_string(&p.fill_args).map_err(|e| Refusal::new("internal", e.to_string()))?;
    let id = handoffs::create(
        c,
        &book,
        &NewHandoff {
            form: form.as_str(),
            period_key: &period,
            form_context: &ctx.to_string(),
            fill_args: &fill_args,
            inputs_hash: &p.inputs_hash,
            lineage: &lineage_of(&built).to_string(),
        },
    )?;
    let ticket = plumbing::ticket_for(c, "handoff", id)?;
    Ok(with_subject(
        json!({ "ticket": ticket, "title": title(form.as_str(), &period), "boxes": p.fill_args.len(), "status": "handed" }),
        "handoff",
        id,
    ))
}

fn handoff_in(c: &Connection, book: &Book, args: &Map<String, Value>) -> Result<Handoff, Refusal> {
    let id = match plumbing::subject(c, required(args, "handoff")?)? {
        Some((k, id)) if k == "handoff" => id,
        _ => {
            return Err(Refusal::new(
                "not_found",
                "That hand-off ticket is not one Books issued.",
            ));
        }
    };
    let h = handoffs::get(c, id)?
        .filter(|h| h.book_id == book.id)
        .ok_or_else(|| Refusal::new("not_found", "That hand-off is not in this book."))?;
    if h.status == "superseded" {
        return Err(Refusal::new(
            "superseded",
            "A newer hand-off replaced this one. Use the one attached last.",
        ));
    }
    Ok(h)
}

/// `books_handoff_get`: what the assistant fills into the BIR draft. It
/// refuses a hand-off the books no longer match.
pub fn get(store: &Store, book: Book, args: &Map<String, Value>, _origin: Origin) -> Answer {
    let c = store.conn();
    let h = handoff_in(c, &book, args)?;
    let now = rebuild(c, &book, &h)?;
    if let Some(why) = changed(c, &book, &h, &now)? {
        return Err(Refusal::new(
            "out_of_date",
            format!(
                "This hand-off is out of date ({why}). Do not fill it; the owner refreshes it on the Forms page."
            ),
        ));
    }
    let fill_args: Value = serde_json::from_str(&h.fill_args).unwrap_or_else(|_| json!({}));
    let period = Period::parse(&h.period_key)
        .ok_or_else(|| Refusal::new("store", "A hand-off names a bad period."))?;
    let bir_period = match period.kind {
        books_core::date::PeriodKind::Quarter => json!(period.start.quarter()),
        books_core::date::PeriodKind::Month => json!(period.start.month),
        books_core::date::PeriodKind::Year => Value::Null,
    };
    Ok(json!({
        "form": h.form,
        "year": period.start.year,
        "period": h.period_key,
        "bir_period": bir_period,
        "status": h.status,
        "fill_args": fill_args,
        "source": SOURCE,
        "form_context": serde_json::from_str::<Value>(&h.form_context).unwrap_or(Value::Null),
        "how": "Open the BIR draft for this form and period, fill these boxes once with source \"document\", read them back and compare each value exactly, then validate and save the draft. Never send or file. Then the owner confirms in Books.",
    }))
}

/// `books_app_handoff_confirm`: the owner's click on "BIR shows these values".
pub fn confirm(c: &Connection, args: &Map<String, Value>) -> Answer {
    let book = writes::book_in(c, args)?;
    let h = handoff_in(c, &book, args)?;
    let now = rebuild(c, &book, &h)?;
    if let Some(why) = changed(c, &book, &h, &now)? {
        return Err(Refusal::new(
            "out_of_date",
            format!("The books changed since this hand-off ({why}). Refresh it first."),
        ));
    }
    let fill_args: Value = serde_json::from_str(&h.fill_args).unwrap_or_else(|_| json!({}));
    let digest =
        crate::jcs::sha256_jcs(&fill_args).map_err(|e| Refusal::new("internal", e.to_string()))?;
    handoffs::confirm(c, &book, h.id, &digest)?;
    Ok(with_subject(
        json!({ "status": "confirmed", "fill_args_sha256": digest }),
        "handoff",
        h.id,
    ))
}

/// The `proposal` scope's choices: current hand-offs, newest first.
pub fn choices(store: &Store) -> Result<Vec<Value>, Refusal> {
    let c = store.conn();
    let mut out = Vec::new();
    for b in books_store::books::all(c)? {
        for h in handoffs::list(c, b.id)?.into_iter().take(20) {
            out.push(json!({ "label": title(&h.form, &h.period_key), "detail": if h.status == "confirmed" { "In the BIR draft" } else { "Handed over" }, "value": plumbing::ticket_for(c, "handoff", h.id)? }));
        }
    }
    out.truncate(20);
    Ok(out)
}

/// A hand-off ticket's title, for the `proposal` scope's token.
pub fn ticket_title(store: &Store, ticket: &str) -> Option<String> {
    let c = store.conn();
    match plumbing::subject(c, ticket).ok()?? {
        (k, id) if k == "handoff" => handoffs::get(c, id)
            .ok()?
            .map(|h| title(&h.form, &h.period_key)),
        _ => None,
    }
}
