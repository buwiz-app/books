//! M10 parity reconcile-1: a statement the assistant read from a page the
//! user attached. The assistant drafts the lines (`books_statement_draft_lines`);
//! the user accepts or dismisses them in the app (`books_app_statement_accept`
//! with `draft`). Books never reads the page: it stores what it is given.

use books_core::date::{Date, Period, PeriodKind};
use books_core::ledger::KindRef;
use books_core::money;
use books_core::statement::{self, Parsed};
use books_store::Store;
use books_store::books::Book;
use books_store::months::{self, Statement};
use books_store::plumbing;
use books_store::rusqlite::Connection;
use books_store::statement_drafts::{self as drafts, Bbox, Draft, DraftLine, Extra, NewDraft};
use serde_json::{Map, Value, json};

use crate::service::{self, Answer, Origin, Refusal, text, ticket, with_subject};
use crate::views::signed;
use crate::writes;

pub const LOW: i64 = 80;
const MAX_LINES: usize = 2000;

fn month_arg(args: &Map<String, Value>) -> Result<Period, Refusal> {
    Period::parse(service::required(args, "month")?)
        .filter(|p| p.kind == PeriodKind::Month)
        .ok_or_else(|| Refusal::invalid("`month` is YYYY-MM."))
}

fn money_text(v: Option<&Value>, what: &str) -> Result<Option<i64>, Refusal> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => money::parse(s)
            .map(Some)
            .map_err(|_| Refusal::invalid(format!("`{what}` is two-place decimal text."))),
        Some(_) => Err(Refusal::invalid(format!(
            "`{what}` is text such as \"12.50\", not a number."
        ))),
    }
}

fn small_int(v: Option<&Value>, what: &str, lo: i64, hi: i64) -> Result<Option<i64>, Refusal> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_i64()
            .filter(|n| (lo..=hi).contains(n))
            .map(Some)
            .ok_or_else(|| {
                Refusal::invalid(format!("`{what}` is a whole number from {lo} to {hi}."))
            }),
    }
}

fn bbox_arg(v: Option<&Value>) -> Result<Option<Bbox>, Refusal> {
    let bad = || Refusal::invalid("`bbox` is four whole numbers 0 to 1000, left top right bottom.");
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(a)) if a.len() == 4 => {
            let mut b = [0i64; 4];
            for (slot, x) in b.iter_mut().zip(a) {
                *slot = x.as_i64().ok_or_else(bad)?;
            }
            drafts::bbox_ok(&b).then_some(Some(b)).ok_or_else(bad)
        }
        Some(_) => Err(bad()),
    }
}

fn line_arg(no: usize, v: &Value) -> Result<DraftLine, Refusal> {
    let l = v
        .as_object()
        .ok_or_else(|| Refusal::invalid("Each line is an object."))?;
    let date = text(l, "date")
        .and_then(Date::parse)
        .ok_or_else(|| Refusal::invalid("A line's `date` is YYYY-MM-DD."))?;
    let description = text(l, "description").unwrap_or("").trim();
    if description.chars().count() > 80 {
        return Err(Refusal::invalid(
            "A line's description is at most 80 characters.",
        ));
    }
    let description = statement::mask_digit_runs(description);
    let amount = money_text(l.get("amount"), "amount")?
        .filter(|a| *a != 0)
        .ok_or_else(|| Refusal::invalid("Each line has an `amount` other than 0.00."))?;
    Ok(DraftLine {
        line_no: i64::try_from(no + 1).unwrap_or(0),
        date,
        description: if description.is_empty() {
            "Bank line".into()
        } else {
            description
        },
        amount,
        balance: money_text(l.get("balance"), "balance")?,
        page: small_int(l.get("page"), "page", 1, 500)?,
        bbox: bbox_arg(l.get("bbox"))?,
        confidence: small_int(l.get("confidence"), "confidence", 0, 100)?,
    })
}

fn evidence_arg(
    conn: &Connection,
    book: &Book,
    args: &Map<String, Value>,
) -> Result<Option<i64>, Refusal> {
    let Some(t) = text(args, "evidence") else {
        return Ok(None);
    };
    let not_ours = || Refusal::new("not_found", "That evidence ticket is not one Books issued.");
    let id = match plumbing::subject(conn, t)? {
        Some((k, id)) if k == "evidence" => id,
        _ => return Err(not_ours()),
    };
    books_store::evidence::get(conn, id)?
        .filter(|e| e.book_id == book.id)
        .map(|e| Some(e.id))
        .ok_or_else(not_ours)
}

fn draft_arg(conn: &Connection, book: &Book, args: &Map<String, Value>) -> Result<Draft, Refusal> {
    let not_ours = || Refusal::new("not_found", "That statement read is not one Books issued.");
    let id = match plumbing::subject(conn, service::required(args, "draft")?)? {
        Some((k, id)) if k == "sdraft" => id,
        _ => return Err(not_ours()),
    };
    drafts::get(conn, id)?
        .filter(|d| d.book_id == book.id)
        .ok_or_else(not_ours)
}

struct Counts {
    low: usize,
    outside: usize,
    breaks: usize,
}

fn outside(l: &DraftLine, key: &str) -> bool {
    Period::of(l.date, PeriodKind::Month).key != key
}

fn low(confidence: Option<i64>) -> bool {
    confidence.is_some_and(|c| c < LOW)
}

fn counts(d: &Draft) -> Counts {
    let pairs: Vec<(i64, Option<i64>)> = d.lines.iter().map(|l| (l.amount, l.balance)).collect();
    Counts {
        low: d.lines.iter().filter(|l| low(l.confidence)).count(),
        outside: d.lines.iter().filter(|l| outside(l, &d.period_key)).count(),
        breaks: drafts::balance_breaks(&pairs),
    }
}

/// `books_statement_draft_lines`: the assistant (or the app) hands Books the
/// lines it read. Stored as a pending draft; nothing reaches the statement.
/// The result carries no description and no amount, for every origin.
pub fn draft_lines(conn: &Connection, args: &Map<String, Value>, origin: Origin) -> Answer {
    let proposed_by = match origin {
        Origin::Agent => "agent",
        Origin::App => "app",
        _ => {
            return Err(Refusal::new(
                "permission_denied",
                "Only the assistant or the Books app drafts a statement read.",
            ));
        }
    };
    let book = writes::book_in(conn, args)?;
    let account = writes::account_in(conn, &book, service::required(args, "account")?)?;
    if !(account.is_cash() || account.kind == KindRef::Kind("loan_payable".into())) {
        return Err(Refusal::invalid(
            "A statement belongs to a bank, cash or card account.",
        ));
    }
    let month = month_arg(args)?;
    let raw = args
        .get("lines")
        .and_then(Value::as_array)
        .filter(|a| !a.is_empty())
        .ok_or_else(|| Refusal::invalid("`lines` lists 1 to 2,000 lines."))?;
    if raw.len() > MAX_LINES {
        return Err(Refusal::invalid(
            "A statement read has at most 2,000 lines.",
        ));
    }
    let lines = raw
        .iter()
        .enumerate()
        .map(|(i, v)| line_arg(i, v))
        .collect::<Result<Vec<_>, _>>()?;
    let closing = money_text(args.get("closing"), "closing")?;
    let evidence = evidence_arg(conn, &book, args)?;
    let id = drafts::insert(
        conn,
        &NewDraft {
            book_id: book.id,
            account_id: account.id,
            period_key: &month.key,
            closing,
            evidence_id: evidence,
            proposed_by,
            lines: &lines,
        },
    )?;
    let d = drafts::get(conn, id)?.ok_or_else(|| Refusal::new("store", "The draft vanished."))?;
    let c = counts(&d);
    Ok(with_subject(
        json!({
            "draft": plumbing::ticket_for(conn, "sdraft", id)?,
            "status": "pending",
            "lines": d.lines.len(),
            "low_confidence": c.low,
            "outside_month": c.outside,
            "balance_breaks": c.breaks,
            "title": "Statement read",
        }),
        "sdraft",
        id,
    ))
}

/// `books_app_statement_accept` with `draft`: the user's click uses the
/// lines (less `exclude`) as the month's statement, or dismisses them.
pub fn accept(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let book = writes::book_in(conn, args)?;
    let account = writes::account_in(conn, &book, service::required(args, "account")?)?;
    let month = month_arg(args)?;
    let d = draft_arg(conn, &book, args)?;
    if d.account_id != account.id || d.period_key != month.key {
        return Err(Refusal::invalid(
            "That statement read is for another account or month.",
        ));
    }
    if d.status != "pending" {
        return Err(Refusal::new(
            "not_pending",
            "That statement read was already decided.",
        ));
    }
    let draft_ticket = plumbing::ticket_for(conn, "sdraft", d.id)?;
    if args.get("dismiss").and_then(Value::as_bool) == Some(true) {
        drafts::decide(conn, d.id, "dismissed")?;
        return Ok(with_subject(
            json!({ "draft": draft_ticket, "status": "dismissed" }),
            "sdraft",
            d.id,
        ));
    }
    let exclude: Vec<i64> = match args.get("exclude") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(a)) => a
            .iter()
            .map(|v| {
                v.as_i64()
                    .ok_or_else(|| Refusal::invalid("`exclude` lists line numbers."))
            })
            .collect::<Result<_, _>>()?,
        Some(_) => return Err(Refusal::invalid("`exclude` lists line numbers.")),
    };
    let kept: Vec<&DraftLine> = d
        .lines
        .iter()
        .filter(|l| !exclude.contains(&l.line_no))
        .collect();
    if kept.iter().any(|l| outside(l, &d.period_key)) {
        return Err(Refusal::invalid(
            "Some lines are dated outside this month; leave them out first.",
        ));
    }
    let closing = money_text(args.get("closing"), "closing")?.or(d.closing);
    let parsed = Parsed {
        lines: kept
            .iter()
            .map(|l| statement::Line {
                date: l.date,
                description: l.description.clone(),
                amount: l.amount,
            })
            .collect(),
        closing: None,
        balances: kept.iter().map(|l| l.balance).collect(),
    };
    let id = months::import(
        conn,
        &book,
        &account,
        &parsed,
        closing,
        Some("Statement read"),
    )?;
    let extras: Vec<Extra> = kept
        .iter()
        .map(|l| Extra {
            balance: l.balance,
            page: l.page,
            bbox: l.bbox,
            confidence: l.confidence,
        })
        .collect();
    drafts::keep_extras(conn, id, d.evidence_id, &extras)?;
    drafts::decide(conn, d.id, "accepted")?;
    Ok(with_subject(
        json!({ "month": month.key, "lines": parsed.lines.len(), "status": "open" }),
        "statement",
        id,
    ))
}

fn percent(thousandths: i64) -> String {
    format!("{}.{}%", thousandths / 10, thousandths % 10)
}

fn box_json(b: Option<Bbox>) -> Value {
    b.map_or(Value::Null, |b| {
        json!({ "left": percent(b[0]), "top": percent(b[1]), "width": percent(b[2] - b[0]), "height": percent(b[3] - b[1]) })
    })
}

fn draft_json(store: &Store, d: &Draft, origin: Origin) -> Result<Value, Refusal> {
    let c = counts(d);
    let mut out = json!({
        "ticket": ticket(store, "sdraft", d.id)?,
        "lines": d.lines.len(),
        "low_confidence": c.low,
        "outside_month": c.outside,
        "balance_breaks": c.breaks,
        "has_closing": d.closing.is_some(),
        "proposed_by": d.proposed_by,
    });
    if origin == Origin::App {
        out["rows"] = d
            .lines
            .iter()
            .map(|l| {
                json!({
                    "no": l.line_no,
                    "date_short": l.date.short(),
                    "description": l.description,
                    "amount_shown": signed(l.amount),
                    "confidence": l.confidence,
                    "low": low(l.confidence),
                    "outside_month": outside(l, &d.period_key),
                    "page": l.page,
                })
            })
            .collect();
    }
    Ok(out)
}

/// What `books_statement_get` adds: the statement's evidence, each row's
/// page, box and confidence, and the pending statement read (`draft`, at the
/// top level and, when a statement exists, inside it). Draft rows go to the
/// app only.
pub fn view_extra(
    store: &Store,
    book: &Book,
    account_id: i64,
    key: &str,
    statement: Option<&Statement>,
    origin: Origin,
    out: &mut Value,
) -> Result<(), Refusal> {
    let c = store.conn();
    let draft = match drafts::pending(c, book.id, account_id, key)? {
        Some(d) => draft_json(store, &d, origin)?,
        None => Value::Null,
    };
    out["draft"] = draft.clone();
    let Some(s) = statement else { return Ok(()) };
    let (evidence, extras) = drafts::extras(c, s.id)?;
    let st = &mut out["statement"];
    st["evidence"] = match evidence {
        Some(id) => json!(ticket(store, "evidence", id)?),
        None => Value::Null,
    };
    st["draft"] = draft;
    if let Some(rows) = st["rows"].as_array_mut() {
        for (row, e) in rows.iter_mut().zip(&extras) {
            row["page"] = json!(e.page);
            row["box"] = box_json(e.bbox);
            row["confidence"] = json!(e.confidence);
            row["low"] = json!(low(e.confidence));
        }
    }
    Ok(())
}
