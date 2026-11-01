//! M4 pages: Reconcile (accounts by month, then a statement beside the
//! ledger) and Settings (book, months, defaults, link and data), with
//! their writes. Every write here is the user's click in the app.

use books_core::date::{Date, MONTHS, Period, PeriodKind};
use books_core::ledger::{Basis, KindRef};
use books_core::money;
use books_core::statement::{self, Match};
use books_store::Store;
use books_store::books::{self, Account, Book};
use books_store::months::{self, SLOTS};
use books_store::plumbing;
use books_store::rusqlite::{Connection, params};
use serde_json::{Map, Value, json};

use crate::service::{self, Answer, Refusal, text, ticket, with_subject};
use crate::views::{Chart, signed};
use crate::writes;

fn money_side(a: &Account) -> bool {
    a.is_cash() || a.kind == KindRef::Kind("loan_payable".into())
}

fn month_short(p: &Period) -> &'static str {
    MONTHS[usize::from(p.start.month - 1)]
}

/// The accounts a reconcile grid shows: money accounts with any entry or
/// statement.
fn grid_accounts(store: &Store, chart: &Chart) -> Result<Vec<Account>, Refusal> {
    let mut out = Vec::new();
    for a in chart.list.iter().filter(|a| a.active && money_side(a)) {
        let used: i64 = store
            .conn()
            .query_row(
                "SELECT (SELECT count(*) FROM line l JOIN entry e ON e.entry_id = l.entry_id WHERE l.account_id = ?1 AND e.status IN ('posted','voided'))
                      + (SELECT count(*) FROM statement WHERE account_id = ?1)",
                params![a.id],
                |r| r.get(0),
            )
            .map_err(books_store::StoreError::from)?;
        if used > 0 {
            out.push(a.clone());
        }
    }
    Ok(out)
}

/// The Reconcile grid: accounts down, the last four months across.
pub fn grid(store: &Store, book: Book) -> Answer {
    let chart = Chart::load(store, book)?;
    let today = Date::today();
    let mut months_list = Vec::new();
    let mut p = Period::of(today, PeriodKind::Month);
    for _ in 0..4 {
        months_list.push(p.clone());
        p = p.previous();
    }
    months_list.reverse();
    let statements = months::statements(store.conn(), chart.book.id)?;
    let (mut done, mut todo, mut drafts, mut cells) = (0, 0, 0, 0);
    let mut oldest: Option<(Period, String)> = None;
    let mut rows = Vec::new();
    for a in grid_accounts(store, &chart)? {
        let mut states = Vec::new();
        for m in &months_list {
            let s = statements
                .iter()
                .find(|s| s.account_id == a.id && s.period_key == m.key);
            let active = !months::candidates(store.conn(), chart.book.id, a.id, m)?.is_empty();
            let state = match s {
                Some(s) if s.status == "finished" => "done",
                Some(_) => "draft",
                None if active => "todo",
                None => "none",
            };
            match state {
                "done" => done += 1,
                "draft" => drafts += 1,
                "todo" => todo += 1,
                _ => {}
            }
            if state != "none" {
                cells += 1;
            }
            if (state == "todo" || state == "draft")
                && oldest.as_ref().is_none_or(|(o, _)| m.start < o.start)
            {
                oldest = Some((m.clone(), a.name.clone()));
            }
            states.push(json!({ "month": m.key, "state": state, "label": match state { "done" => "✓ Matched", "draft" => "Draft", "todo" => "To do", _ => "—" } }));
        }
        rows.push(json!({ "ticket": ticket(store, "account", a.id)?, "name": a.name, "number": a.number, "cells": states }));
    }
    Ok(json!({
        "months": months_list.iter().map(|m| json!({ "key": m.key, "label": month_short(m) })).collect::<Vec<_>>(),
        "rows": rows,
        "kpis": [
            { "label": "Matched", "shown": format!("{done} of {cells}") },
            { "label": "To do", "shown": todo.to_string() },
            { "label": "Drafts", "shown": drafts.to_string() },
            { "label": "Oldest open", "shown": oldest.map_or_else(|| "—".to_string(), |(m, a)| format!("{} · {}", month_short(&m), a)) },
        ],
        "year": today.year,
    }))
}

fn account_arg(store: &Store, book: &Book, args: &Map<String, Value>) -> Result<Account, Refusal> {
    service::account(store, book, service::required(args, "account")?)
}

fn month_arg(args: &Map<String, Value>) -> Result<Period, Refusal> {
    Period::parse(service::required(args, "month")?)
        .filter(|p| p.kind == PeriodKind::Month)
        .ok_or_else(|| Refusal::invalid("`month` is YYYY-MM."))
}

/// One statement beside the ledger: each line, its match, the difference.
pub fn statement_view(store: &Store, book: Book, args: &Map<String, Value>) -> Answer {
    let account = account_arg(store, &book, args)?;
    let month = month_arg(args)?;
    let head = json!({ "account": { "ticket": ticket(store, "account", account.id)?, "name": account.name }, "month": { "key": month.key, "label": month.label(), "short": month_short(&month) } });
    let Some((s, lines)) = months::statement(store.conn(), book.id, account.id, &month.key)? else {
        let mut out = head;
        out["statement"] = Value::Null;
        return Ok(out);
    };
    let found = months::matches(store.conn(), book.id, &s, &lines)?;
    let mut rows = Vec::new();
    let mut suggested = 0;
    let mut missing = 0;
    for (l, m) in lines.iter().zip(&found) {
        let (kind, entry) = match m {
            Match::Exact(e) => ("m", Some(*e)),
            Match::Suggested(e) => {
                suggested += 1;
                ("s", Some(*e))
            }
            Match::Missing => {
                missing += 1;
                ("x", None)
            }
        };
        let books_side = match entry {
            Some(e) => {
                let en = books_store::entries::get(store.conn(), e)?;
                json!({
                    "ticket": ticket(store, "entry", e)?,
                    "date_short": en.as_ref().map(|x| x.date.short()),
                    "title": if kind == "s" { "Suggested match" } else if l.match_kind.as_deref() == Some("added") { "Added from the statement" } else { "Posted" },
                    "sub": if kind == "s" { "Same amount, a few days apart · accept it".to_string() } else { "Same amount and date".to_string() },
                    "amount_shown": signed(l.line.amount),
                })
            }
            None => json!({
                "title": "Not in Books",
                "sub": if l.line.amount > 0 { "Add it as money in" } else { "Add it as money out" },
            }),
        };
        rows.push(json!({
            "ticket": ticket(store, "sline", l.id)?,
            "date_short": l.line.date.short(),
            "description": l.line.description,
            "amount_shown": signed(l.line.amount),
            "match": kind,
            "books": books_side,
        }));
    }
    let (ours, diff) = months::difference(store.conn(), &book, &s)?;
    let mut out = head;
    out["statement"] = json!({
        "status": s.status,
        "file_name": s.source_name,
        "closing_shown": money::display(s.closing),
        "books_shown": money::display(ours),
        "difference": money::format(diff),
        "difference_shown": money::display(diff),
        "suggested": suggested,
        "missing": missing,
        "can_finish": s.status != "finished" && diff == 0 && suggested == 0 && missing == 0,
        "rows": rows,
    });
    Ok(out)
}

/// The Settings page.
pub fn settings(store: &Store, book: Book) -> Answer {
    let chart = Chart::load(store, book)?;
    let today = Date::today();
    let c = store.conn();
    let months_list: Vec<Value> = months::months(c, chart.book.id, today)?
        .into_iter()
        .rev()
        .take(12)
        .map(|(p, closed)| json!({ "key": p.key, "label": p.label(), "short": format!("{} {}", month_short(&p), p.start.year), "closed": closed }))
        .collect();
    let events: Vec<Value> = months::events(c, chart.book.id)?
        .into_iter()
        .take(10)
        .map(|e| json!({ "month": Period::parse(&e.period_key).map(|p| p.label()), "event": e.event, "reason": e.reason, "at": e.at }))
        .collect();
    let mut groups: Vec<(String, Vec<Value>)> = Vec::new();
    for (slot, group, label, _, accrual) in SLOTS {
        let a = months::default_account(c, &chart.book, slot).ok();
        let off = accrual && chart.book.basis == Basis::Cash;
        let row = json!({
            "slot": slot, "label": label, "off": off,
            "account": a.as_ref().map(|a| -> Result<Value, Refusal> { Ok(json!({ "ticket": ticket(store, "account", a.id)?, "label": format!("{} {}", a.number, a.name), "tag": tag(a) })) }).transpose()?,
        });
        match groups.iter_mut().find(|(g, _)| g == group) {
            Some((_, rows)) => rows.push(row),
            None => groups.push((group.to_string(), vec![row])),
        }
    }
    let options = crate::views::entry_options(store, chart.book.clone())?;
    Ok(json!({
        "book": {
            "ticket": ticket(store, "book", chart.book.id)?,
            "label": chart.book.label,
            "basis": chart.book.basis.as_str(),
            "fiscal_year_start": chart.book.fiscal_year_start,
            "fiscal_label": books_core::date::MONTH_NAMES[usize::from(chart.book.fiscal_year_start.max(1) - 1)],
        },
        "months": months_list,
        "events": events,
        "defaults": groups.into_iter().map(|(g, rows)| json!({ "group": g, "rows": rows })).collect::<Vec<_>>(),
        "accounts": options["all"],
        "link": {
            "tax_profile": chart.book.cloud_tax_profile_id.as_ref().map_or("Not linked yet", |_| "Linked by ID through Buwiz sign-in"),
            "bir": "eBIRForms reads Books totals; Books reads its tax option from the BIR draft",
            "stored": "This Mac, encrypted",
        },
    }))
}

fn tag(a: &Account) -> &'static str {
    match a.account_type.group() {
        "Assets" => "as",
        "Liabilities" => "li",
        "Equity" => "eq",
        "Revenue" => "re",
        _ => "ex",
    }
}

// --- writes -------------------------------------------------------------

fn sline_in(conn: &Connection, args: &Map<String, Value>) -> Result<i64, Refusal> {
    match plumbing::subject(conn, service::required(args, "line")?)? {
        Some((k, id)) if k == "sline" => Ok(id),
        _ => Err(Refusal::new(
            "not_found",
            "That statement line ticket is not one Books issued.",
        )),
    }
}

/// `books_app_statement_import`: a CSV statement (`csv_base64`, as the
/// picked file's bytes, or `csv_text`) for one account and month.
pub fn statement_import(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let book = writes::book_in(conn, args)?;
    let account = writes::account_in(conn, &book, service::required(args, "account")?)?;
    let text_in = match (text(args, "csv_base64"), text(args, "csv_text")) {
        (Some(b), _) => {
            let bytes = crate::b64::decode(b)
                .ok_or_else(|| Refusal::invalid("`csv_base64` is not base64url."))?;
            String::from_utf8(bytes)
                .map_err(|_| Refusal::invalid("The statement is not UTF-8 text."))?
        }
        (None, Some(t)) => t.to_string(),
        (None, None) => {
            return Err(Refusal::invalid(
                "Send the statement as `csv_base64` or `csv_text`.",
            ));
        }
    };
    if text_in.len() > 2_000_000 {
        return Err(Refusal::invalid("A statement is at most 2 MB."));
    }
    let parsed = statement::parse_csv(&text_in).map_err(Refusal::invalid)?;
    let closing = text(args, "closing")
        .map(|c| money::parse(c).map_err(|e| Refusal::invalid(format!("`closing`: {e}"))))
        .transpose()?;
    let id = months::import(
        conn,
        &book,
        &account,
        &parsed,
        closing,
        text(args, "file_name"),
    )?;
    let key = Period::of(parsed.lines[0].date, PeriodKind::Month).key;
    Ok(with_subject(
        json!({ "month": key, "lines": parsed.lines.len(), "status": "open" }),
        "statement",
        id,
    ))
}

fn account_month(
    conn: &Connection,
    args: &Map<String, Value>,
) -> Result<(Book, Account, String), Refusal> {
    let book = writes::book_in(conn, args)?;
    let account = writes::account_in(conn, &book, service::required(args, "account")?)?;
    let month = month_arg(args)?;
    Ok((book, account, month.key))
}

/// `books_app_statement_accept`: accept every suggested match.
pub fn statement_accept(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let (book, account, key) = account_month(conn, args)?;
    let n = months::accept_suggestions(conn, &book, account.id, &key)?;
    Ok(with_subject(
        json!({ "accepted": n }),
        "account",
        account.id,
    ))
}

/// `books_app_statement_add_line`: post a bank line that is not in Books.
pub fn statement_add(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let (book, account, key) = account_month(conn, args)?;
    let line = sline_in(conn, args)?;
    let g = writes::gate(conn, &book);
    let entry = months::add_line(conn, &book, account.id, &key, line, &g)?;
    Ok(with_subject(
        json!({ "entry": plumbing::ticket_for(conn, "entry", entry)?, "status": "posted" }),
        "entry",
        entry,
    ))
}

/// `books_app_statement_finish`: the month matches the bank.
pub fn statement_finish(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let (book, account, key) = account_month(conn, args)?;
    months::finish(conn, &book, account.id, &key)?;
    Ok(with_subject(
        json!({ "status": "finished", "month": key }),
        "account",
        account.id,
    ))
}

/// `books_app_recategorize`: give a "not sure yet" line its category.
pub fn recategorize(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let book = writes::book_in(conn, args)?;
    let entry = match plumbing::subject(conn, service::required(args, "entry")?)? {
        Some((k, id)) if k == "entry" => id,
        _ => {
            return Err(Refusal::new(
                "not_found",
                "That entry ticket is not one Books issued.",
            ));
        }
    };
    let category = writes::account_in(conn, &book, service::required(args, "category")?)?;
    let g = writes::gate(conn, &book);
    let id = months::recategorize(conn, &book, entry, &category, &g)?;
    let e = books_store::entries::get(conn, id)?
        .ok_or_else(|| Refusal::new("store", "The entry vanished."))?;
    Ok(with_subject(
        json!({ "entry": plumbing::ticket_for(conn, "entry", id)?, "no": format!("{:04}", e.no), "status": "posted" }),
        "entry",
        id,
    ))
}

/// `books_app_month_close`.
pub fn month_close(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let book = writes::book_in(conn, args)?;
    let month = month_arg(args)?;
    months::close(conn, &book, &month.key, text(args, "reason"))?;
    Ok(with_subject(
        json!({ "month": month.key, "status": "closed" }),
        "book",
        book.id,
    ))
}

/// `books_app_month_reopen`: needs a reason.
pub fn month_reopen(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let book = writes::book_in(conn, args)?;
    let month = month_arg(args)?;
    months::reopen(conn, &book, &month.key, text(args, "reason").unwrap_or(""))?;
    Ok(with_subject(
        json!({ "month": month.key, "status": "open" }),
        "book",
        book.id,
    ))
}

/// `books_app_default_set`.
pub fn default_set(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let book = writes::book_in(conn, args)?;
    let account = writes::account_in(conn, &book, service::required(args, "account")?)?;
    months::set_default(conn, &book, service::required(args, "slot")?, &account)?;
    Ok(with_subject(
        json!({ "slot": service::required(args, "slot")?, "account": plumbing::ticket_for(conn, "account", account.id)? }),
        "book",
        book.id,
    ))
}

/// `books_app_book_update`: name, fiscal year start, basis.
pub fn book_update(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let book = writes::book_in(conn, args)?;
    let basis = text(args, "basis")
        .map(|b| Basis::parse(b).ok_or_else(|| Refusal::invalid("`basis` is cash or accrual.")))
        .transpose()?;
    let fy = args
        .get("fiscal_year_start")
        .map(|v| {
            v.as_i64()
                .and_then(|m| u8::try_from(m).ok())
                .ok_or_else(|| Refusal::invalid("`fiscal_year_start` is a month, 1 to 12."))
        })
        .transpose()?;
    months::update_book(conn, &book, text(args, "label"), fy, basis, Date::today())?;
    let after =
        books::get(conn, book.id)?.ok_or_else(|| Refusal::new("store", "The book vanished."))?;
    Ok(with_subject(
        json!({ "label": after.label, "basis": after.basis.as_str(), "fiscal_year_start": after.fiscal_year_start }),
        "book",
        book.id,
    ))
}
