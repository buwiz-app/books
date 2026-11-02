//! Cross-checks. Each compares two ways of reaching the
//! same total and says ok or how far apart they are:
//!
//! - the four quarters' net sales against the year's (2551Q against
//!   1701-MS IV.19);
//! - sales by account (the P&L) against the gross sales measure, which
//!   picks lines by their facts;
//! - each month's pay runs against that month's 1601-C totals (items 14,
//!   19, 25);
//! - received income-tax 2307s for Q1 to Q3 against 1701-MS V.3;
//! - returns BIR shows as filed, which the assistant reads from the BIR
//!   app and passes as `filed`, against Books now.
//!
//! Display only: a difference opens the lines, it never changes a box.

use std::collections::BTreeMap;

use books_core::date::{Date, MONTH_NAMES, Period, PeriodKind};
use books_core::input::{FormCode, NormInput, normalize_fixture};
use books_core::measures::{Window, compute_measure};
use books_core::money;
use books_core::posting::{Ledger, post_ledger};
use books_core::rules;
use books_store::Store;
use books_store::books::Book;
use books_store::certificates;
use serde_json::{Map, Value, json};

use crate::forms;
use crate::service::{Answer, Refusal, text};

fn window(p: &Period) -> Window {
    Window {
        start: p.start.to_string(),
        end: p.end.to_string(),
    }
}

fn measure(ledger: &Ledger, id: &str, w: &Window) -> i64 {
    compute_measure(id, ledger, w).map_or(0, |r| r.value)
}

fn row(check: &str, left: (&str, i64), right: (&str, i64)) -> Value {
    let diff = left.1 - right.1;
    json!({
        "check": check,
        "left_label": left.0, "left_shown": money::display(left.1),
        "right_label": right.0, "right_shown": money::display(right.1),
        "ok": diff == 0,
        "note": if diff == 0 { "ok".to_string() } else { format!("{} apart · open the lines", money::display(diff.abs())) },
    })
}

/// A filed box this check cannot compare; it is no difference, and the
/// value is not repeated.
fn not_compared(check: &str, why: &str) -> Value {
    json!({
        "check": check,
        "left_label": "filed", "left_shown": "—",
        "right_label": "Books now", "right_shown": "—",
        "ok": true, "compared": false,
        "note": format!("not compared · {why}"),
    })
}

/// A blank or non-amount value filed on a box Books fills with an amount: a
/// difference (the value is not repeated).
fn not_an_amount(check: &str, books: i64) -> Value {
    json!({
        "check": check,
        "left_label": "filed", "left_shown": "not an amount",
        "right_label": "Books now", "right_shown": money::display(books),
        "ok": false,
        "note": "the filed value is not an amount · open the form",
    })
}

/// A filed choice (a radio's option) against Books' own.
fn choice_row(check: &str, filed: &str, books: &str) -> Value {
    let same = filed == books;
    json!({
        "check": check,
        "left_label": "filed", "left_shown": filed,
        "right_label": "Books now", "right_shown": books,
        "ok": same,
        "note": if same { "ok".to_string() } else { "different choice · open the form".to_string() },
    })
}

/// Sales by account: credit less debit on accounts of the gross-sales kinds.
fn sales_by_account(input: &NormInput, year: &Period) -> i64 {
    let data = rules::shared().ok();
    let kinds: Vec<String> = data
        .and_then(|d| d.measures["measures"].as_array().cloned())
        .unwrap_or_default()
        .iter()
        .find(|m| m["id"] == "gross_sales")
        .and_then(|m| m["derived_from"]["categories"].as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    let mut total = 0i64;
    for e in input
        .entries
        .iter()
        .filter(|e| Date::parse(&e.entry_date).is_some_and(|d| year.contains(d)))
    {
        for l in e.lines.iter().filter(|l| kinds.contains(&l.account)) {
            total += money::parse(&l.credit).unwrap_or(0) - money::parse(&l.debit).unwrap_or(0);
        }
    }
    total
}

/// Each month's payroll-register lines, by measure: gross pay, the
/// employee share and tax withheld.
fn pay_runs(input: &NormInput, year: &Period) -> BTreeMap<String, [i64; 3]> {
    let gross = [
        "basic_pay",
        "statutory_minimum_wage",
        "holiday_ot_nsd_hazard",
        "thirteenth_month_and_benefits",
        "de_minimis",
    ];
    let mut out: BTreeMap<String, [i64; 3]> = BTreeMap::new();
    for e in &input.entries {
        let Some(d) = Date::parse(&e.entry_date).filter(|d| year.contains(*d)) else {
            continue;
        };
        for l in e
            .lines
            .iter()
            .filter(|l| l.document_kind == "payroll_register")
        {
            let signed = |side: &str| {
                let (dr, cr) = (
                    money::parse(&l.debit).unwrap_or(0),
                    money::parse(&l.credit).unwrap_or(0),
                );
                if side == "debit" { dr - cr } else { cr - dr }
            };
            let slot = out.entry(Period::of(d, PeriodKind::Month).key).or_default();
            match l.comp_component.as_deref() {
                Some(c) if gross.contains(&c) => slot[0] += signed("debit"),
                Some("contribution") if l.contribution_side == "employee" => {
                    slot[1] += signed("credit")
                }
                Some("tax_withheld") => slot[2] += signed("credit"),
                _ => {}
            }
        }
    }
    out
}

/// `books_cross_checks`: the checks for one year (this year when absent).
pub fn cross_checks(store: &Store, book: Book, args: &Map<String, Value>) -> Answer {
    let c = store.conn();
    let year = match text(args, "year") {
        Some(y) => Period::parse(y)
            .filter(|p| p.kind == PeriodKind::Year)
            .ok_or_else(|| Refusal::invalid("`year` is like 2026."))?,
        None => Period::of(Date::today(), PeriodKind::Year),
    };
    let data = rules::shared().map_err(|e| Refusal::new("engine", e.to_string()))?;
    let (fx, _) = forms::fixture(c, &book)?;
    let input = normalize_fixture(&fx, &data.vocab)
        .map_err(|e| Refusal::new("bridge_input", e.to_string()))?;
    let ledger =
        post_ledger(&input.entries).map_err(|e| Refusal::new("bridge_input", e.to_string()))?;
    let y = year.start.year;
    let mut rows = Vec::new();

    let quarters: i64 = (1..=4)
        .filter_map(|q| Period::parse(&format!("{y}-Q{q}")))
        .map(|p| measure(&ledger, "net_sales", &window(&p)))
        .sum();
    rows.push(row(
        "2551Q Q1+Q2+Q3+Q4 = 1701-MS IV.19",
        ("four quarters", quarters),
        ("the year", measure(&ledger, "net_sales", &window(&year))),
    ));
    rows.push(row(
        "P&L sales = gross sales",
        ("sales accounts", sales_by_account(&input, &year)),
        (
            "gross sales",
            measure(&ledger, "gross_sales", &window(&year)),
        ),
    ));
    for (key, [gross, employee, tax]) in pay_runs(&input, &year) {
        let Some(m) = Period::parse(&key) else {
            continue;
        };
        let w = window(&m);
        let month = MONTH_NAMES[usize::from(m.start.month - 1)];
        let books = [
            measure(&ledger, "gross_compensation", &w),
            measure(&ledger, "employee_mandatory_contributions", &w),
            measure(&ledger, "tax_withheld_from_employees", &w),
        ];
        let apart: i64 = [gross, employee, tax]
            .iter()
            .zip(books)
            .map(|(a, b)| (a - b).abs())
            .sum();
        rows.push(json!({
            "check": format!("pay run {month} = 1601-C 14, 19, 25"),
            "left_label": "pay runs", "left_shown": format!("{} · {} · {}", money::display(gross), money::display(employee), money::display(tax)),
            "right_label": "1601-C", "right_shown": format!("{} · {} · {}", money::display(books[0]), money::display(books[1]), money::display(books[2])),
            "ok": apart == 0,
            "note": if apart == 0 { "ok".to_string() } else { "pay outside a pay run · open the lines".to_string() },
        }));
    }
    let q1_q3 = Window {
        start: year.start.to_string(),
        end: format!("{y}-09-30"),
    };
    let received: i64 = certificates::list(c, book.id)?
        .iter()
        .filter(|x| {
            !x.voided && x.form == "2307" && x.credit_kind == "it" && x.status == "received"
        })
        .filter(|x| x.period_end.is_some_and(|d| d.year == y && d.month <= 9))
        .map(|x| x.centavos)
        .sum();
    rows.push(row(
        "2307s received = 1701-MS V.3",
        ("certificates", received),
        (
            "1701-MS V.3",
            measure(&ledger, "it_withheld_2307_received", &q1_q3),
        ),
    ));

    // filed returns the assistant read from BIR: [{form, period, key, value,
    // form_context?}]. Books' own value decides how a box compares: an
    // amount as an amount (eBIRForms shows thousands separators and, on some
    // forms, whole pesos), a choice such as 1601-C's "Taxes withheld" as
    // text. A 1701-MS box needs the form context (the tax option): the item's
    // own, else the one its latest hand-off read. A box Books can't compare
    // (one it doesn't hand over, a period the form doesn't have, a form
    // still blocked) is a "not compared" row, and its value is not repeated;
    // a blank or non-amount value on an amount box is a difference.
    let handoffs = books_store::handoffs::list(c, book.id)?;
    for f in args
        .get("filed")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let (Some(form), Some(period), Some(key), Some(value)) = (
            f["form"].as_str().and_then(FormCode::parse),
            f["period"].as_str(),
            f["key"].as_str(),
            f["value"].as_str(),
        ) else {
            return Err(Refusal::invalid(
                "Each filed value is {form, period, key, value}.",
            ));
        };
        let label = Period::parse(period).map_or_else(|| period.to_string(), |p| p.label());
        let check = format!("filed {} {label} {key} vs Books", form.as_str());
        let context = match f.get("form_context") {
            Some(context @ Value::Object(_)) => context.clone(),
            _ => handoffs
                .iter()
                .find(|h| h.form == form.as_str() && h.period_key == period)
                .and_then(|h| serde_json::from_str(&h.form_context).ok())
                .unwrap_or_else(|| json!({})),
        };
        let built = match forms::build(c, &book, form, period, &context) {
            Ok(built) => built,
            Err(refusal) => {
                rows.push(not_compared(
                    &check,
                    &format!("Books can't build it: {}", refusal.message),
                ));
                continue;
            }
        };
        let books = built
            .proposal
            .fill_args
            .get(key)
            .cloned()
            .or_else(|| built.preview.get(key).map(|c| money::format(*c)));
        rows.push(match books {
            None => match built.proposal.blocking.first() {
                Some(block) if built.proposal.fill_args.is_empty() => not_compared(
                    &check,
                    &format!("Books can't compare it yet: {}", block.detail),
                ),
                _ => not_compared(&check, "Books doesn't hand this box over"),
            },
            Some(books) => match (money::parse(&books), money::parse_shown(value)) {
                (Ok(now), Ok(filed)) => row(&check, ("filed", filed), ("Books now", now)),
                (Ok(now), Err(_)) => not_an_amount(&check, now),
                (Err(_), _) => choice_row(&check, value.trim(), &books),
            },
        });
    }
    let not_compared_count = rows.iter().filter(|r| r["compared"] == false).count();
    let differences = rows.iter().filter(|r| r["ok"] == false).count();
    Ok(
        json!({ "year": y, "rows": rows, "differences": differences, "not_compared": not_compared_count }),
    )
}
