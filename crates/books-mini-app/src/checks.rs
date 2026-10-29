//! M7-06: cross-checks (plan 3.4). Each compares two ways of reaching the
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

    // filed returns the assistant read from BIR: [{form, period, key, value}]
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
        let filed =
            money::parse(value).map_err(|e| Refusal::invalid(format!("filed value: {e}")))?;
        let built = forms::build(c, &book, form, period, &json!({}))?;
        let now = built
            .proposal
            .fill_args
            .get(key)
            .and_then(|v| money::parse(v).ok())
            .or_else(|| built.preview.get(key).copied())
            .unwrap_or(0);
        let label = Period::parse(period).map_or_else(|| period.to_string(), |p| p.label());
        rows.push(row(
            &format!("filed {} {label} vs Books", form.as_str()),
            ("filed", filed),
            ("Books now", now),
        ));
    }
    let differences = rows.iter().filter(|r| r["ok"] == false).count();
    Ok(json!({ "year": y, "rows": rows, "differences": differences }))
}
