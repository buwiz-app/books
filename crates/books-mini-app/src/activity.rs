//! Account activity (books_report kind `ac`): one account's posted and
//! voided entries in a period, oldest first, each with its change and the
//! running balance in the account's own sense (a card shows owed rising).
//! A void and its reversal both appear and net to zero; drafts and
//! discarded entries never do.
//!
//! `books_report` is assistant-callable and takes no origin, so a row
//! carries no payee, memo or reference: `what` is the other side's
//! account name. The app shows the rest through the entry drawer.

use books_core::date::Period;
use books_core::ledger::Status;
use books_core::money;
use books_store::Store;
use books_store::books::{self, Account};
use books_store::entries::{self, Filter, Form};
use serde_json::{Map, Value, json};

use crate::service::{self, Answer, Refusal, ticket};
use crate::views::{Chart, signed};

/// Rows a result carries at most; the closing is still the true closing.
pub const MAX_ROWS: usize = 500;

/// `kind: "ac"`: resolve `account`, then [`activity`].
pub fn report(store: &Store, chart: &Chart, period: &Period, args: &Map<String, Value>) -> Answer {
    let reference = service::text(args, "account")
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Refusal::invalid("`account` is required for account activity."))?;
    let account = service::account(store, &chart.book, reference)?;
    activity(store, chart, &account, period)
}

fn amount(c: i64) -> Value {
    json!({ "value": money::format(c), "shown": money::display(c) })
}

fn blank_or(c: i64) -> String {
    if c == 0 {
        String::new()
    } else {
        money::display(c)
    }
}

/// The activity of `account` over `period`.
pub fn activity(store: &Store, chart: &Chart, account: &Account, period: &Period) -> Answer {
    if chart.hidden(account) {
        return Err(Refusal::invalid("That account is for accrual books only."));
    }
    let before = books::movements(
        store.conn(),
        chart.book.id,
        None,
        Some(period.start.add_days(-1)),
    )?;
    let through = books::movements(store.conn(), chart.book.id, None, Some(period.end))?;
    let opening = account.natural(*before.get(&account.id).unwrap_or(&0));
    let closing = account.natural(*through.get(&account.id).unwrap_or(&0));
    let mut list = entries::list(
        store.conn(),
        chart.book.id,
        &Filter {
            from: Some(period.start),
            to: Some(period.end),
            account_id: Some(account.id),
            ..Filter::default()
        },
    )?;
    list.retain(|e| matches!(e.status, Status::Posted | Status::Voided));
    // newest first from the store: oldest first here, ties by entry number
    list.reverse();
    let mut balance = opening;
    let mut rows = Vec::new();
    for e in &list {
        let lines = entries::lines(store.conn(), e.id)?;
        let (mut debit, mut credit) = (0i64, 0i64);
        for l in lines.iter().filter(|l| l.account_id == account.id) {
            debit += l.debit;
            credit += l.credit;
        }
        let change = account.natural(debit - credit);
        balance += change;
        if rows.len() == MAX_ROWS {
            continue;
        }
        let other = lines
            .iter()
            .filter(|l| l.account_id != account.id)
            .filter_map(|l| chart.by_id.get(&l.account_id))
            .min_by_key(|a| !a.account_type.is_pl())
            .map_or_else(|| "Entry".to_string(), |a| a.name.clone());
        let reversal = e.form == Form::Reversal;
        rows.push(json!({
            "date": e.date.to_string(),
            "date_shown": e.date.short(),
            "entry": ticket(store, "entry", e.id)?,
            "no": format!("{:04}", e.no),
            "what": if reversal { format!("Void · {other}") } else { other },
            "reversal": reversal,
            "voided": e.status == Status::Voided,
            "debit": if debit == 0 { String::new() } else { money::format(debit) },
            "credit": if credit == 0 { String::new() } else { money::format(credit) },
            "debit_shown": blank_or(debit),
            "credit_shown": blank_or(credit),
            "change": money::format(change),
            "change_shown": signed(change),
            "tone": if change > 0 { "pos" } else if change < 0 { "neg" } else { "" },
            "balance": money::format(balance),
            "balance_shown": money::display(balance),
        }));
    }
    let balanced = balance == closing;
    debug_assert!(balanced, "opening plus changes must equal closing");
    Ok(json!({
        "account": {
            "ticket": ticket(store, "account", account.id)?,
            "number": account.number,
            "name": account.name,
            "type": account.account_type.as_str(),
        },
        "columns": ["Date", "What", "Change", "Balance"],
        "opening": amount(opening),
        "closing": amount(closing),
        "rows": rows,
        "count": list.len(),
        "more": list.len() > MAX_ROWS,
        "balanced": balanced,
    }))
}

fn cell(text: &str) -> String {
    if text.contains([',', '"', '\n']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

/// `books_report_csv` for kind `ac`: entry numbers, never tickets.
pub fn csv(r: &Value) -> Value {
    let s = |v: &Value| v.as_str().unwrap_or("").to_string();
    let mut lines = vec![
        "Date,Entry,What,Debit,Credit,Balance".to_string(),
        format!("Opening,,,,,{}", s(&r["opening"]["value"])),
    ];
    for row in r["rows"].as_array().into_iter().flatten() {
        lines.push(
            [
                s(&row["date"]),
                s(&row["no"]),
                cell(&s(&row["what"])),
                s(&row["debit"]),
                s(&row["credit"]),
                s(&row["balance"]),
            ]
            .join(","),
        );
    }
    lines.push(format!("Closing,,,,,{}", s(&r["closing"]["value"])));
    let name = format!(
        "books-ac-{}-{}.csv",
        s(&r["account"]["number"]),
        s(&r["period"]["key"])
    );
    json!({ "file_name": name, "csv": lines.join("\n") + "\n", "rows": lines.len() })
}
