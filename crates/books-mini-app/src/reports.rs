//! M3 Reports: P&L with the previous period beside it, the balance sheet,
//! cash flow, the trial balance and aging (accrual books only). Every row
//! of the P&L and the balance sheet names its account, so a click opens
//! its lines in the Ledger. Reports follow the book's basis (ADR 0008);
//! a report on the other basis is a later feature, not a toggle.
//!
//! Posted and voided entries count (a void nets with its reversal); drafts
//! never do.

use std::collections::HashMap;

use books_core::catalog::AccountType;
use books_core::date::{Date, MONTH_NAMES, MONTHS, Period, PeriodKind};
use books_core::ledger::{Basis, KindRef};
use books_core::money;
use books_store::Store;
use books_store::books::{self, Account, Book};
use books_store::entries::{self, Filter, Form};
use books_store::people;
use books_store::plumbing;
use books_store::rusqlite::params;
use books_store::slices::{self, TagColumn};
use serde_json::{Map, Value, json};

use crate::service::{Answer, Refusal, text, ticket};
use crate::views::{Chart, Summary, signed};

/// The period a report covers: `period_kind` (month, quarter, year) and
/// `period` (a key of that kind), else this month.
pub fn period_arg(args: &Map<String, Value>) -> Result<Period, Refusal> {
    let kind = match text(args, "period_kind").unwrap_or("month") {
        "month" => PeriodKind::Month,
        "quarter" => PeriodKind::Quarter,
        "year" => PeriodKind::Year,
        _ => return Err(Refusal::invalid("`period_kind` is month, quarter or year.")),
    };
    match text(args, "period") {
        Some(k) => Period::parse(k)
            .filter(|p| p.kind == kind)
            .ok_or_else(|| Refusal::invalid("`period` does not match `period_kind`.")),
        None => Ok(Period::of(Date::today(), kind)),
    }
}

fn short_label(p: &Period) -> String {
    match p.kind {
        PeriodKind::Month => MONTH_NAMES[usize::from(p.start.month - 1)].to_string(),
        _ => p.label(),
    }
}

/// `Oct 31, 2026`.
fn long_date(d: Date) -> String {
    format!("{} {}, {}", MONTHS[usize::from(d.month - 1)], d.day, d.year)
}

fn natural(a: &Account, m: &HashMap<i64, i64>) -> i64 {
    a.natural(*m.get(&a.id).unwrap_or(&0))
}

fn is_revenue(t: AccountType) -> bool {
    matches!(t, AccountType::Revenue | AccountType::OtherIncome)
}

/// Posted lines on a "not sure yet" account in a range: what waits for a
/// category.
fn to_review(store: &Store, account: &Account, p: &Period) -> Result<i64, Refusal> {
    Ok(store
        .conn()
        .query_row(
            "SELECT count(*) FROM line l JOIN entry e ON e.entry_id = l.entry_id
             WHERE l.account_id = ?1 AND e.status IN ('posted','voided') AND e.form <> 'reversal' AND e.voided_by_entry_id IS NULL
               AND e.recategorized_by_entry_id IS NULL AND e.recategorizes_entry_id IS NULL
               AND e.entry_date >= ?2 AND e.entry_date <= ?3",
            params![account.id, p.start.to_string(), p.end.to_string()],
            |r| r.get(0),
        )
        .map_err(books_store::StoreError::from)?)
}

fn row(
    store: &Store,
    a: &Account,
    cur: i64,
    prev: Option<i64>,
    month: &Period,
) -> Result<Value, Refusal> {
    let mut r = json!({
        "ticket": ticket(store, "account", a.id)?,
        "number": a.number,
        "name": a.name,
        "value": money::format(cur),
        "shown": money::display(cur),
        "month": month.key,
    });
    if let Some(p) = prev {
        r["prev_shown"] = json!(money::display(p));
    }
    Ok(r)
}

/// Income and expenses of `period`, each in its own sense.
pub fn pl_totals(store: &Store, chart: &Chart, period: &Period) -> Result<(i64, i64), Refusal> {
    let m = books::movements(
        store.conn(),
        chart.book.id,
        Some(period.start),
        Some(period.end),
    )?;
    let (mut income, mut expense) = (0i64, 0i64);
    for a in chart.list.iter().filter(|a| a.account_type.is_pl()) {
        if is_revenue(a.account_type) {
            income += natural(a, &m);
        } else {
            expense += natural(a, &m);
        }
    }
    Ok((income, expense))
}

/// What a report compares its period to (`compare_to`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Compare {
    Previous,
    LastYear,
    None,
}

impl Compare {
    fn as_str(self) -> &'static str {
        match self {
            Self::Previous => "previous",
            Self::LastYear => "last_year",
            Self::None => "none",
        }
    }
}

/// The same period key one year earlier: 2026-10 is 2025-10, 2026-Q3 is
/// 2025-Q3 and 2026 is 2025.
pub fn same_period_last_year(p: &Period) -> Period {
    let rest = p.key.find('-').map_or("", |i| &p.key[i..]);
    Period::parse(&format!("{:04}{rest}", p.start.year - 1)).unwrap_or_else(|| p.previous())
}

/// The tag a report is narrowed to (`tag`).
struct TagFilter {
    column: TagColumn,
    id: i64,
    kind: String,
    name: String,
    ticket: String,
}

fn tag_arg(
    store: &Store,
    chart: &Chart,
    args: &Map<String, Value>,
) -> Result<Option<TagFilter>, Refusal> {
    let Some(t) = text(args, "tag").map(str::trim).filter(|t| !t.is_empty()) else {
        return Ok(None);
    };
    let missing = || Refusal::new("not_found", "That tag is not in this book.");
    let Some(("tag", id)) = plumbing::subject(store.conn(), t)?
        .as_ref()
        .map(|(k, id)| (k.as_str(), *id))
    else {
        return Err(missing());
    };
    let tag = people::tag(store.conn(), id)?
        .filter(|g| g.book_id == chart.book.id)
        .ok_or_else(missing)?;
    let column = TagColumn::of_kind(&tag.kind).ok_or_else(missing)?;
    Ok(Some(TagFilter {
        column,
        id,
        kind: tag.kind,
        name: tag.name,
        ticket: t.to_string(),
    }))
}

/// Account movements over `[from, to]`, narrowed to a tag when given.
fn moves(
    store: &Store,
    chart: &Chart,
    from: Option<Date>,
    to: Option<Date>,
    tag: Option<&TagFilter>,
) -> Result<HashMap<i64, i64>, Refusal> {
    Ok(match tag {
        Some(t) => slices::movements_tagged(store.conn(), chart.book.id, from, to, t.column, t.id)?,
        None => books::movements(store.conn(), chart.book.id, from, to)?,
    })
}

/// The P&L of `period`, beside the compare period when there is one.
fn pl(
    store: &Store,
    chart: &Chart,
    period: &Period,
    prev: Option<&Period>,
    tag: Option<&TagFilter>,
) -> Answer {
    let cur_m = moves(store, chart, Some(period.start), Some(period.end), tag)?;
    let prev_m = match prev {
        Some(p) => moves(store, chart, Some(p.start), Some(p.end), tag)?,
        None => HashMap::new(),
    };
    let drill = Period::of(period.end, PeriodKind::Month);
    let mut sections = Vec::new();
    let mut totals = (0i64, 0i64, 0i64, 0i64);
    for (title, revenue) in [("Revenue", true), ("Expenses", false)] {
        let mut rows = Vec::new();
        for a in chart
            .list
            .iter()
            .filter(|a| a.account_type.is_pl() && is_revenue(a.account_type) == revenue)
        {
            let (c, p) = (natural(a, &cur_m), natural(a, &prev_m));
            if c == 0 && p == 0 {
                continue;
            }
            let mut r = row(store, a, c, prev.map(|_| p), &drill)?;
            if a.kind == KindRef::Unsure && tag.is_none() {
                let n = to_review(store, a, period)?;
                if n > 0 {
                    r["review"] = json!(format!("{n} to review"));
                }
            }
            rows.push(r);
            if revenue {
                totals.0 += c;
                totals.1 += p;
            } else {
                totals.2 += c;
                totals.3 += p;
            }
        }
        sections.push(json!({ "title": title, "rows": rows }));
    }
    let (net, prev_net) = (totals.0 - totals.2, totals.1 - totals.3);
    let mut out = json!({
        "columns": [short_label(period)],
        "sections": sections,
        "totals": [
            { "label": "Total revenue", "shown": money::display(totals.0) },
            { "label": "Total expenses", "shown": money::display(totals.2) },
        ],
        "net": { "label": "Net income", "value": money::format(net), "shown": money::display(net) },
    });
    if let Some(p) = prev {
        out["columns"] = json!([short_label(period), short_label(p)]);
        out["totals"][0]["prev_shown"] = json!(money::display(totals.1));
        out["totals"][1]["prev_shown"] = json!(money::display(totals.3));
        out["net"]["prev_shown"] = json!(money::display(prev_net));
    }
    Ok(out)
}

/// The first day of the fiscal year that holds `d`.
fn fiscal_start(book: &Book, d: Date) -> Date {
    let m = book.fiscal_year_start.max(1);
    let year = if d.month >= m { d.year } else { d.year - 1 };
    Date::new(year, m, 1).unwrap_or(d)
}

/// Balances as of `end`, with this fiscal year's net income and the
/// income of earlier years.
fn as_of(
    store: &Store,
    chart: &Chart,
    end: Date,
) -> Result<(HashMap<i64, i64>, i64, i64), Refusal> {
    let all = books::movements(store.conn(), chart.book.id, None, Some(end))?;
    let fy = fiscal_start(&chart.book, end);
    let year = books::movements(store.conn(), chart.book.id, Some(fy), Some(end))?;
    let net_of = |m: &HashMap<i64, i64>| -> i64 {
        chart
            .list
            .iter()
            .filter(|a| a.account_type.is_pl())
            .map(|a| {
                if is_revenue(a.account_type) {
                    natural(a, m)
                } else {
                    -natural(a, m)
                }
            })
            .sum()
    };
    let this_year = net_of(&year);
    let earlier = net_of(&all) - this_year;
    Ok((all, this_year, earlier))
}

fn with_prev(mut v: Value, prev: Option<i64>) -> Value {
    if let Some(p) = prev {
        v["prev_shown"] = json!(money::display(p));
    }
    v
}

/// The balance sheet at the end of `period`, beside the balance sheet at
/// the end of the compare period when there is one.
fn bs(store: &Store, chart: &Chart, period: &Period, prev: Option<&Period>) -> Answer {
    let end = period.end;
    let (all, this_year, earlier) = as_of(store, chart, end)?;
    let (p_all, p_year, p_earlier) = match prev {
        Some(p) => as_of(store, chart, p.end)?,
        None => (HashMap::new(), 0, 0),
    };
    let on = |v: i64| prev.map(|_| v);
    let drill = Period::of(end, PeriodKind::Month);
    let mut sections = Vec::new();
    let mut sums = Vec::new();
    let mut p_sums = Vec::new();
    for (title, ty) in [
        ("Assets", AccountType::Asset),
        ("Liabilities", AccountType::Liability),
        ("Equity", AccountType::Equity),
    ] {
        let mut rows = Vec::new();
        let (mut total, mut p_total) = (0i64, 0i64);
        for a in chart.list.iter().filter(|a| a.account_type == ty) {
            let (v, pv) = (natural(a, &all), natural(a, &p_all));
            if v == 0 && pv == 0 {
                continue;
            }
            total += v;
            p_total += pv;
            rows.push(row(store, a, v, on(pv), &drill)?);
        }
        if ty == AccountType::Equity {
            if earlier != 0 || p_earlier != 0 {
                rows.push(with_prev(
                    json!({ "name": "Income of earlier years", "shown": money::display(earlier) }),
                    on(p_earlier),
                ));
                total += earlier;
                p_total += p_earlier;
            }
            rows.insert(
                0,
                with_prev(
                    json!({ "name": "Net income this year", "shown": money::display(this_year) }),
                    on(p_year),
                ),
            );
            total += this_year;
            p_total += p_year;
        }
        sums.push(total);
        p_sums.push(p_total);
        let total_row = with_prev(
            json!({ "label": format!("Total {}", title.to_lowercase()), "shown": money::display(total) }),
            on(p_total),
        );
        sections.push(json!({ "title": title, "rows": rows, "total": total_row }));
    }
    let (assets, liabilities, equity) = (sums[0], sums[1], sums[2]);
    let mut columns = vec![format!("As of {}", long_date(end))];
    if let Some(p) = prev {
        columns.push(format!("As of {}", long_date(p.end)));
    }
    Ok(json!({
        "columns": columns,
        "sections": sections,
        "net": with_prev(json!({ "label": "Liabilities + equity", "shown": money::display(liabilities + equity) }), on(p_sums[1] + p_sums[2])),
        "balanced": assets == liabilities + equity,
        "check": if assets == liabilities + equity { "Balanced ✓".to_string() } else { format!("Off by {}", money::display(assets - liabilities - equity)) },
    }))
}

/// Cash flow figures of a period: cash at the start and the end (whole
/// book), money in, money out and transfers (narrowed to a tag when given).
struct Flows {
    start: i64,
    end: i64,
    inflow: i64,
    outflow: i64,
    moved: i64,
}

fn flows(
    store: &Store,
    chart: &Chart,
    period: &Period,
    tag: Option<&TagFilter>,
) -> Result<Flows, Refusal> {
    let before = moves(store, chart, None, Some(period.start.add_days(-1)), None)?;
    let after = moves(store, chart, None, Some(period.end), None)?;
    let cash = |m: &HashMap<i64, i64>| -> i64 {
        chart
            .list
            .iter()
            .filter(|a| a.is_cash())
            .map(|a| natural(a, m))
            .sum()
    };
    let mut kept = Vec::new();
    for e in entries::list(
        store.conn(),
        chart.book.id,
        &Filter {
            from: Some(period.start),
            to: Some(period.end),
            ..Filter::default()
        },
    )? {
        if !matches!(
            e.status,
            books_core::ledger::Status::Posted | books_core::ledger::Status::Voided
        ) {
            continue;
        }
        if let Some(t) = tag {
            let (_, dept, loc) = people::links(store.conn(), e.id)?;
            let on = match t.column {
                TagColumn::Department => dept,
                TagColumn::Location => loc,
            };
            if on != Some(t.id) {
                continue;
            }
        }
        kept.push(Summary::of(store, chart, e)?);
    }
    let (mut inflow, mut outflow, mut moved) = (0i64, 0i64, 0i64);
    for s in &kept {
        if s.entry.form == Form::Transfer {
            moved += s.cash;
        } else if s.cash > 0 {
            inflow += s.cash;
        } else {
            outflow += s.cash;
        }
    }
    Ok(Flows {
        start: cash(&before),
        end: cash(&after),
        inflow,
        outflow,
        moved,
    })
}

/// Cash flow over `period`: cash at the start, money in, money out,
/// transfers to cards and loans, cash at the end. With a tag, only the
/// tagged money in and out, and their net.
fn cf(
    store: &Store,
    chart: &Chart,
    period: &Period,
    prev: Option<&Period>,
    tag: Option<&TagFilter>,
) -> Answer {
    let f = flows(store, chart, period, tag)?;
    let p = prev.map(|p| flows(store, chart, p, tag)).transpose()?;
    let pv = |g: fn(&Flows) -> i64| p.as_ref().map(g);
    let signed_prev = |mut v: Value, g: fn(&Flows) -> i64| {
        if let Some(x) = pv(g) {
            v["prev_shown"] = json!(signed(x));
        }
        v
    };
    let mut rows = Vec::new();
    if tag.is_none() {
        rows.push(with_prev(
            json!({ "name": format!("Cash at {} {}", MONTHS[usize::from(period.start.month - 1)], period.start.day), "shown": money::display(f.start) }),
            pv(|x| x.start),
        ));
    }
    rows.push(signed_prev(
        json!({ "name": "Money in from customers and others", "shown": signed(f.inflow), "tone": "pos" }),
        |x| x.inflow,
    ));
    rows.push(signed_prev(
        json!({ "name": "Money out for running the business", "shown": signed(f.outflow) }),
        |x| x.outflow,
    ));
    if f.moved != 0 || pv(|x| x.moved).is_some_and(|m| m != 0) {
        rows.push(signed_prev(
            json!({ "name": "Paid cards and loans", "shown": signed(f.moved) }),
            |x| x.moved,
        ));
    }
    let mut columns = vec![period.label()];
    if let Some(p) = prev {
        columns.push(p.label());
    }
    let mut out = json!({ "columns": columns, "sections": [{ "title": "", "rows": rows }] });
    if tag.is_some() {
        let net = |x: &Flows| x.inflow + x.outflow + x.moved;
        let mut n = json!({ "label": "Net cash movement", "shown": signed(net(&f)) });
        if let Some(x) = &p {
            n["prev_shown"] = json!(signed(net(x)));
        }
        out["net"] = n;
        out["tag_note"] = json!("Cash at the start and end is for the whole book.");
    } else {
        out["net"] = with_prev(
            json!({ "label": format!("Cash at {} {}", MONTHS[usize::from(period.end.month - 1)], period.end.day), "shown": money::display(f.end) }),
            pv(|x| x.end),
        );
        out["balanced"] = json!(f.start + f.inflow + f.outflow + f.moved == f.end);
    }
    Ok(out)
}

/// The trial balance from the start of the book's fiscal year (balance
/// sheet accounts from the beginning) to the end of `period`.
fn tb(store: &Store, chart: &Chart, period: &Period) -> Answer {
    let all = books::movements(store.conn(), chart.book.id, None, Some(period.end))?;
    let fy = fiscal_start(&chart.book, period.end);
    let (mut dr, mut cr) = (0i64, 0i64);
    let mut rows = Vec::new();
    let drill = Period::of(period.end, PeriodKind::Month);
    for a in &chart.list {
        let v = *all.get(&a.id).unwrap_or(&0);
        if v == 0 {
            continue;
        }
        let mut r = row(store, a, v.abs(), None, &drill)?;
        if v > 0 {
            dr += v;
            r["debit_shown"] = json!(money::display(v));
            r["credit_shown"] = json!("");
        } else {
            cr -= v;
            r["debit_shown"] = json!("");
            r["credit_shown"] = json!(money::display(-v));
        }
        rows.push(r);
    }
    Ok(json!({
        "columns": [format!("{} – {}", long_date(fy).replace(&format!(", {}", fy.year), ""), long_date(period.end)), "Debit", "Credit"],
        "sections": [{ "title": "", "rows": rows }],
        "net": { "label": if dr == cr { "Totals · balanced ✓" } else { "Totals · NOT balanced" }, "debit_shown": money::display(dr), "credit_shown": money::display(cr) },
        "balanced": dr == cr,
    }))
}

/// Aging of open receivables or payables at the end of `period`; accrual
/// books only.
fn ag(store: &Store, chart: &Chart, period: &Period, args: &Map<String, Value>) -> Answer {
    if chart.book.basis == Basis::Cash {
        return Ok(json!({
            "cash_book": true,
            "title": "Aging is for accrual books.",
            "note": "On a cash book, unpaid invoices and bills are reminders. See them on the Unpaid page, sorted by due date.",
        }));
    }
    let side = if text(args, "side") == Some("pay") {
        "pay"
    } else {
        "collect"
    };
    let at = period.end;
    let mut by: Vec<(String, [i64; 4])> = Vec::new();
    for u in people::open_unpaid(store.conn(), chart.book.id, side)? {
        let who = match u.person_id {
            Some(p) => people::person(store.conn(), p)?
                .map(|p| p.name)
                .unwrap_or_default(),
            None => "—".into(),
        };
        let late = u.due.map_or(0, |d| at.to_days() - d.to_days());
        let bucket = match late {
            i64::MIN..=0 => 0,
            1..=30 => 1,
            31..=60 => 2,
            _ => 3,
        };
        match by.iter_mut().find(|(n, _)| *n == who) {
            Some((_, b)) => b[bucket] += u.balance(),
            None => {
                let mut b = [0i64; 4];
                b[bucket] = u.balance();
                by.push((who, b));
            }
        }
    }
    let show = |c: i64| {
        if c == 0 {
            String::new()
        } else {
            money::display(c)
        }
    };
    let rows: Vec<Value> = by
        .iter()
        .map(|(who, b)| json!({ "name": who, "cells": b.iter().map(|c| show(*c)).collect::<Vec<_>>() }))
        .collect();
    Ok(json!({
        "cash_book": false,
        "side": side,
        "columns": [if side == "collect" { "Receivables" } else { "Payables" }, "Current", "1–30", "31–60", "61+"],
        "rows": rows,
    }))
}

/// `books_report`.
pub fn report(store: &Store, book: Book, args: &Map<String, Value>) -> Answer {
    let chart = Chart::load(store, book)?;
    let kind = text(args, "kind").unwrap_or("pl");
    let period = if kind == "ps" {
        crate::person_statement::period(args)?
    } else {
        period_arg(args)?
    };
    if !["pl", "bs", "cf", "tb", "ag", "ac", "ps"].contains(&kind) {
        return Err(Refusal::invalid("`kind` is pl, bs, cf, tb, ag, ac or ps."));
    }
    let compare = match text(args, "compare_to") {
        None if kind == "pl" => Compare::Previous,
        None => Compare::None,
        Some("previous") => Compare::Previous,
        Some("last_year") => Compare::LastYear,
        Some("none") => Compare::None,
        Some(_) => {
            return Err(Refusal::invalid(
                "`compare_to` is previous, last_year or none.",
            ));
        }
    };
    if compare != Compare::None && matches!(kind, "tb" | "ag") {
        return Err(Refusal::invalid(
            "Comparing is for the P&L, balance sheet and cash flow.",
        ));
    }
    let tag = tag_arg(store, &chart, args)?;
    if tag.is_some() && !matches!(kind, "pl" | "cf") {
        return Err(Refusal::invalid(
            "A tag filters the P&L and cash flow only.",
        ));
    }
    let prev = match compare {
        Compare::Previous => Some(period.previous()),
        Compare::LastYear => Some(same_period_last_year(&period)),
        Compare::None => None,
    };
    let mut out = match kind {
        "pl" => pl(store, &chart, &period, prev.as_ref(), tag.as_ref())?,
        "bs" => bs(store, &chart, &period, prev.as_ref())?,
        "cf" => cf(store, &chart, &period, prev.as_ref(), tag.as_ref())?,
        "tb" => tb(store, &chart, &period)?,
        "ag" => ag(store, &chart, &period, args)?,
        "ac" => crate::activity::report(store, &chart, &period, args)?,
        "ps" => crate::person_statement::report(store, &chart, &period, args)?,
        _ => return Err(Refusal::invalid("`kind` is pl, bs, cf, tb, ag, ac or ps.")),
    };
    out["compare"] = match &prev {
        Some(p) => json!({ "key": p.key, "label": p.label(), "mode": compare.as_str() }),
        None => Value::Null,
    };
    out["tag"] = match &tag {
        Some(t) => json!({ "ticket": t.ticket, "kind": t.kind, "name": t.name }),
        None => Value::Null,
    };
    let next = Period::of(period.end.add_days(1), period.kind);
    out["kind"] = json!(kind);
    out["basis"] = json!(chart.book.basis.as_str());
    out["basis_label"] = json!(if chart.book.basis == Basis::Cash {
        "Cash basis"
    } else {
        "Accrual basis"
    });
    out["period"] = json!({ "key": period.key, "label": period.label(), "prev": period.previous().key, "next": next.key, "kind": match period.kind { PeriodKind::Month => "month", PeriodKind::Quarter => "quarter", PeriodKind::Year => "year" } });
    Ok(out)
}

fn csv_cell(text: &str) -> String {
    if text.contains([',', '"', '\n']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

/// `books_report_csv`: the same report as CSV text, amounts as two-place
/// decimal text (no thousands separators, a plain minus).
pub fn report_csv(store: &Store, book: Book, args: &Map<String, Value>) -> Answer {
    let r = report(store, book, args)?;
    let plain = |v: &Value| -> String {
        v.as_str()
            .unwrap_or("")
            .replace(',', "")
            .replace('\u{2212}', "-")
            .replace('+', "")
    };
    let mut lines: Vec<String> = Vec::new();
    let cols: Vec<String> = r["columns"]
        .as_array()
        .map(|c| {
            c.iter()
                .map(|x| x.as_str().unwrap_or("").to_string())
                .collect()
        })
        .unwrap_or_default();
    match r["kind"].as_str() {
        Some("ac") => return Ok(crate::activity::csv(&r)),
        Some("ag") => {
            lines.push(
                cols.iter()
                    .map(|c| csv_cell(c))
                    .collect::<Vec<_>>()
                    .join(","),
            );
            for row in r["rows"].as_array().into_iter().flatten() {
                let mut cells = vec![csv_cell(row["name"].as_str().unwrap_or(""))];
                cells.extend(row["cells"].as_array().into_iter().flatten().map(plain));
                lines.push(cells.join(","));
            }
        }
        Some("ps") => lines.extend(crate::person_statement::csv_lines(&r, csv_cell)),
        Some("tb") => {
            lines.push("Number,Account,Debit,Credit".into());
            for row in r["sections"][0]["rows"].as_array().into_iter().flatten() {
                lines.push(format!(
                    "{},{},{},{}",
                    csv_cell(row["number"].as_str().unwrap_or("")),
                    csv_cell(row["name"].as_str().unwrap_or("")),
                    plain(&row["debit_shown"]),
                    plain(&row["credit_shown"])
                ));
            }
            lines.push(format!(
                "Totals,,{},{}",
                plain(&r["net"]["debit_shown"]),
                plain(&r["net"]["credit_shown"])
            ));
        }
        _ => {
            let two = !r["compare"].is_null() && cols.len() > 1;
            lines.push(if two {
                format!(
                    "Section,Number,Account,{},{}",
                    csv_cell(&cols[0]),
                    csv_cell(&cols[1])
                )
            } else {
                format!(
                    "Section,Number,Account,{}",
                    csv_cell(cols.first().map_or("", String::as_str))
                )
            });
            for s in r["sections"].as_array().into_iter().flatten() {
                let section = s["title"].as_str().unwrap_or("");
                for row in s["rows"].as_array().into_iter().flatten() {
                    let mut cells = vec![
                        csv_cell(section),
                        csv_cell(row["number"].as_str().unwrap_or("")),
                        csv_cell(row["name"].as_str().unwrap_or("")),
                        plain(&row["shown"]),
                    ];
                    if two {
                        cells.push(plain(&row["prev_shown"]));
                    }
                    lines.push(cells.join(","));
                }
            }
            let mut net = vec![
                String::new(),
                String::new(),
                csv_cell(r["net"]["label"].as_str().unwrap_or("")),
                plain(&r["net"]["shown"]),
            ];
            if two {
                net.push(plain(&r["net"]["prev_shown"]));
            }
            lines.push(net.join(","));
        }
    }
    let mut name = format!(
        "books-{}-{}",
        r["kind"].as_str().unwrap_or("report"),
        r["period"]["key"].as_str().unwrap_or("")
    );
    if text(args, "compare_to").is_some()
        && let Some(k) = r["compare"]["key"].as_str()
    {
        name.push_str(&format!("-vs-{k}"));
    }
    match r["tag"]["kind"].as_str() {
        Some("department") => name.push_str("-dept"),
        Some("location") => name.push_str("-loc"),
        _ => {}
    }
    name.push_str(".csv");
    Ok(json!({ "file_name": name, "csv": lines.join("\n") + "\n", "rows": lines.len() }))
}
