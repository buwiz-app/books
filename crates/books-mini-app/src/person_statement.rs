//! Person statement (`books_report` kind `ps`): one person's money in and
//! out over a period, their open invoices and bills with days late, and the
//! aging strip for each side. `books_report` is assistant-callable and has
//! no origin, so nothing here names the person, a payee, a memo or an
//! unpaid reference; the People page shows the name from `books_person_get`.
//! On a cash book the open items are reminders, not entries (ADR 0008), so
//! the strip is shown on both bases while kind `ag` stays accrual-only.

use books_core::date::{Date, Period};
use books_core::ledger::Basis;
use books_core::money;
use books_store::Store;
use books_store::people::{self, Person};
use books_store::plumbing;
use serde_json::{Map, Value, json};

use crate::records::person_entries;
use crate::reports::period_arg;
use crate::service::{self, Answer, Refusal, ticket};
use crate::views::Chart;

/// Activity rows shown at most; the totals still cover the whole period.
pub const MAX_ROWS: usize = 500;

pub const BUCKETS: [&str; 4] = ["Current", "1\u{2013}30", "31\u{2013}60", "61+"];

pub const CASH_NOTE: &str = "On a cash book these are reminders, not in the books.";

/// The statement's period: like every report, but a year when no
/// `period_kind` is given.
pub fn period(args: &Map<String, Value>) -> Result<Period, Refusal> {
    if args.contains_key("period_kind") {
        return period_arg(args);
    }
    let mut with_year = args.clone();
    with_year.insert("period_kind".into(), json!("year"));
    period_arg(&with_year)
}

/// The aging bucket for days past due, as in kind `ag`.
pub fn bucket(late: i64) -> usize {
    match late {
        i64::MIN..=0 => 0,
        1..=30 => 1,
        31..=60 => 2,
        _ => 3,
    }
}

/// `books_report` kind `ps`: resolve `person` (a person ticket of this
/// book) and build the statement.
pub fn report(store: &Store, chart: &Chart, period: &Period, args: &Map<String, Value>) -> Answer {
    let t = service::required(args, "person")?;
    let (kind, id) = plumbing::subject(store.conn(), t)?
        .ok_or_else(|| Refusal::new("not_found", "That ticket is not one Books issued."))?;
    if kind != "person" {
        return Err(Refusal::invalid("`person` is a person ticket."));
    }
    let person = people::person(store.conn(), id)?
        .filter(|p| p.book_id == chart.book.id)
        .ok_or_else(|| Refusal::new("not_found", "That person is not in this book."))?;
    statement(store, chart, &person, period, args)
}

fn shown_or_blank(c: i64) -> String {
    if c == 0 {
        String::new()
    } else {
        money::display(c)
    }
}

fn narrow(c: i128) -> Result<i64, Refusal> {
    i64::try_from(c).map_err(|_| Refusal::invalid("Too large to show."))
}

/// The statement of `person` over `period`.
pub fn statement(
    store: &Store,
    chart: &Chart,
    person: &Person,
    period: &Period,
    _args: &Map<String, Value>,
) -> Answer {
    let mut list = person_entries(store, chart, person.id)?;
    list.retain(|s| period.contains(s.entry.date));
    list.reverse();
    let (mut total_in, mut total_out) = (0i128, 0i128);
    for s in &list {
        if s.cash > 0 {
            total_in += i128::from(s.cash);
        } else {
            total_out -= i128::from(s.cash);
        }
    }
    let total_in = narrow(total_in)?;
    let total_out = narrow(total_out)?;
    let mut rows = Vec::new();
    for s in list.iter().take(MAX_ROWS) {
        rows.push(json!({
            "date": s.entry.date.to_string(),
            "date_shown": s.entry.date.short(),
            "entry": ticket(store, "entry", s.entry.id)?,
            "no": format!("{:04}", s.entry.no),
            "what": s.category.as_ref().map_or_else(|| "\u{2014}".to_string(), |a| a.name.clone()),
            "in_shown": shown_or_blank(s.cash.max(0)),
            "out_shown": shown_or_blank(-s.cash.min(0)),
        }));
    }
    let at = period.end.min(Date::today());
    let mut open = Map::new();
    let mut buckets = Map::new();
    for side in ["collect", "pay"] {
        let mut items = Vec::new();
        let mut sums = [0i128; 4];
        let mut total = 0i128;
        for u in people::open_unpaid(store.conn(), chart.book.id, side)?
            .iter()
            .filter(|u| u.person_id == Some(person.id))
        {
            let late = u.due.map_or(0, |d| at.to_days() - d.to_days());
            let b = bucket(late);
            sums[b] += i128::from(u.amount);
            total += i128::from(u.amount);
            items.push(json!({
                "owed": ticket(store, "owed", u.id)?,
                "side": side,
                "issued": u.issued.to_string(),
                "issued_shown": u.issued.short(),
                "due": u.due.map(|d| d.to_string()),
                "due_shown": u.due.map_or_else(|| "\u{2014}".to_string(), Date::short),
                "days_late": late.max(0),
                "bucket": b,
                "amount_shown": money::display(u.amount),
            }));
        }
        open.insert(side.into(), json!(items));
        open.insert(
            format!("total_{side}_shown"),
            json!(money::display(narrow(total)?)),
        );
        let sums = sums
            .into_iter()
            .map(narrow)
            .collect::<Result<Vec<_>, _>>()?;
        buckets.insert(
            side.into(),
            json!(sums.iter().map(|c| money::display(*c)).collect::<Vec<_>>()),
        );
    }
    buckets.insert("columns".into(), json!(BUCKETS));
    let mut out = json!({
        "person": { "ticket": ticket(store, "person", person.id)?, "kind": person.kind },
        "columns": [period.label()],
        "activity": {
            "rows": rows,
            "more": list.len() > MAX_ROWS,
            "total_in_shown": money::display(total_in),
            "total_out_shown": money::display(total_out),
            "net_shown": money::display(total_in - total_out),
        },
        "open": open,
        "buckets": buckets,
        "at": at.to_string(),
    });
    if chart.book.basis == Basis::Cash {
        out["basis_note"] = json!(CASH_NOTE);
    }
    Ok(out)
}

/// The CSV lines of a kind `ps` report: the activity block, a blank line,
/// then the open items.
pub fn csv_lines(r: &Value, cell: impl Fn(&str) -> String) -> Vec<String> {
    let plain = |v: &Value| -> String {
        v.as_str()
            .unwrap_or("")
            .replace(',', "")
            .replace('\u{2212}', "-")
    };
    let mut lines = vec!["Date,Entry,What,In,Out".to_string()];
    let a = &r["activity"];
    for row in a["rows"].as_array().into_iter().flatten() {
        lines.push(format!(
            "{},{},{},{},{}",
            cell(row["date"].as_str().unwrap_or("")),
            cell(row["no"].as_str().unwrap_or("")),
            cell(row["what"].as_str().unwrap_or("")),
            plain(&row["in_shown"]),
            plain(&row["out_shown"]),
        ));
    }
    lines.push(format!(
        "Totals,,,{},{}",
        plain(&a["total_in_shown"]),
        plain(&a["total_out_shown"])
    ));
    lines.push(String::new());
    lines.push("Side,Issued,Due,Days late,Amount".to_string());
    for side in ["collect", "pay"] {
        for item in r["open"][side].as_array().into_iter().flatten() {
            lines.push(format!(
                "{},{},{},{},{}",
                if side == "collect" {
                    "To collect"
                } else {
                    "To pay"
                },
                cell(item["issued"].as_str().unwrap_or("")),
                cell(item["due"].as_str().unwrap_or("")),
                item["days_late"],
                plain(&item["amount_shown"]),
            ));
        }
    }
    lines
}
