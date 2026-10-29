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
use books_store::rusqlite::params;
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

/// The P&L of `period` beside the previous period of the same kind.
fn pl(store: &Store, chart: &Chart, period: &Period) -> Answer {
    let prev = period.previous();
    let cur_m = books::movements(
        store.conn(),
        chart.book.id,
        Some(period.start),
        Some(period.end),
    )?;
    let prev_m = books::movements(
        store.conn(),
        chart.book.id,
        Some(prev.start),
        Some(prev.end),
    )?;
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
            let mut r = row(store, a, c, Some(p), &drill)?;
            if a.kind == KindRef::Unsure {
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
    Ok(json!({
        "columns": [short_label(period), short_label(&prev)],
        "sections": sections,
        "totals": [
            { "label": "Total revenue", "shown": money::display(totals.0), "prev_shown": money::display(totals.1) },
            { "label": "Total expenses", "shown": money::display(totals.2), "prev_shown": money::display(totals.3) },
        ],
        "net": { "label": "Net income", "value": money::format(net), "shown": money::display(net), "prev_shown": money::display(prev_net) },
    }))
}

/// The first day of the fiscal year that holds `d`.
fn fiscal_start(book: &Book, d: Date) -> Date {
    let m = book.fiscal_year_start.max(1);
    let year = if d.month >= m { d.year } else { d.year - 1 };
    Date::new(year, m, 1).unwrap_or(d)
}

/// The balance sheet at the end of `period`.
fn bs(store: &Store, chart: &Chart, period: &Period) -> Answer {
    let end = period.end;
    let all = books::movements(store.conn(), chart.book.id, None, Some(end))?;
    let fy = fiscal_start(&chart.book, end);
    let year = books::movements(store.conn(), chart.book.id, Some(fy), Some(end))?;
    let drill = Period::of(end, PeriodKind::Month);
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
    let mut sections = Vec::new();
    let mut sums = Vec::new();
    for (title, ty) in [
        ("Assets", AccountType::Asset),
        ("Liabilities", AccountType::Liability),
        ("Equity", AccountType::Equity),
    ] {
        let mut rows = Vec::new();
        let mut total = 0i64;
        for a in chart.list.iter().filter(|a| a.account_type == ty) {
            let v = natural(a, &all);
            if v == 0 {
                continue;
            }
            total += v;
            rows.push(row(store, a, v, None, &drill)?);
        }
        if ty == AccountType::Equity {
            if earlier != 0 {
                rows.push(
                    json!({ "name": "Income of earlier years", "shown": money::display(earlier) }),
                );
                total += earlier;
            }
            rows.insert(
                0,
                json!({ "name": "Net income this year", "shown": money::display(this_year) }),
            );
            total += this_year;
        }
        sums.push(total);
        sections.push(json!({ "title": title, "rows": rows, "total": { "label": format!("Total {}", title.to_lowercase()), "shown": money::display(total) } }));
    }
    let (assets, liabilities, equity) = (sums[0], sums[1], sums[2]);
    Ok(json!({
        "columns": [format!("As of {}", long_date(end))],
        "sections": sections,
        "net": { "label": "Liabilities + equity", "shown": money::display(liabilities + equity) },
        "balanced": assets == liabilities + equity,
        "check": if assets == liabilities + equity { "Balanced ✓".to_string() } else { format!("Off by {}", money::display(assets - liabilities - equity)) },
    }))
}

/// Cash flow over `period`: cash at the start, money in, money out,
/// transfers to cards and loans, cash at the end.
fn cf(store: &Store, chart: &Chart, period: &Period) -> Answer {
    let before = books::movements(
        store.conn(),
        chart.book.id,
        None,
        Some(period.start.add_days(-1)),
    )?;
    let after = books::movements(store.conn(), chart.book.id, None, Some(period.end))?;
    let cash = |m: &HashMap<i64, i64>| -> i64 {
        chart
            .list
            .iter()
            .filter(|a| a.is_cash())
            .map(|a| natural(a, m))
            .sum()
    };
    let list: Vec<Summary> = entries::list(
        store.conn(),
        chart.book.id,
        &Filter {
            from: Some(period.start),
            to: Some(period.end),
            ..Filter::default()
        },
    )?
    .into_iter()
    .filter(|e| {
        matches!(
            e.status,
            books_core::ledger::Status::Posted | books_core::ledger::Status::Voided
        )
    })
    .map(|e| Summary::of(store, chart, e))
    .collect::<Result<_, _>>()?;
    let (mut inflow, mut outflow, mut moved) = (0i64, 0i64, 0i64);
    for s in &list {
        if s.entry.form == Form::Transfer {
            moved += s.cash;
        } else if s.cash > 0 {
            inflow += s.cash;
        } else {
            outflow += s.cash;
        }
    }
    let (start, end) = (cash(&before), cash(&after));
    let mut rows = vec![
        json!({ "name": format!("Cash at {} {}", MONTHS[usize::from(period.start.month - 1)], period.start.day), "shown": money::display(start) }),
        json!({ "name": "Money in from customers and others", "shown": signed(inflow), "tone": "pos" }),
        json!({ "name": "Money out for running the business", "shown": signed(outflow) }),
    ];
    if moved != 0 {
        rows.push(json!({ "name": "Paid cards and loans", "shown": signed(moved) }));
    }
    Ok(json!({
        "columns": [period.label()],
        "sections": [{ "title": "", "rows": rows }],
        "net": { "label": format!("Cash at {} {}", MONTHS[usize::from(period.end.month - 1)], period.end.day), "shown": money::display(end) },
        "balanced": start + inflow + outflow + moved == end,
    }))
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
            Some((_, b)) => b[bucket] += u.amount,
            None => {
                let mut b = [0i64; 4];
                b[bucket] = u.amount;
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
    let period = period_arg(args)?;
    let kind = text(args, "kind").unwrap_or("pl");
    let mut out = match kind {
        "pl" => pl(store, &chart, &period)?,
        "bs" => bs(store, &chart, &period)?,
        "cf" => cf(store, &chart, &period)?,
        "tb" => tb(store, &chart, &period)?,
        "ag" => ag(store, &chart, &period, args)?,
        _ => return Err(Refusal::invalid("`kind` is pl, bs, cf, tb or ag.")),
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
            let two = r["kind"] == "pl";
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
    let name = format!(
        "books-{}-{}.csv",
        r["kind"].as_str().unwrap_or("report"),
        r["period"]["key"].as_str().unwrap_or("")
    );
    Ok(json!({ "file_name": name, "csv": lines.join("\n") + "\n", "rows": lines.len() }))
}
