//! Review findings (M10 parity, ledger-2): what a bookkeeper would look at
//! twice, computed from the ledger each time and never stored, so nothing
//! is written and nothing needs dismissing. Bookkeeping hygiene only: no
//! rates, thresholds of tax law or AI (invariant 1). Not the BIR cross-check
//! (`checks.rs`).
//!
//! `books_checks` gives tickets of the form `entry 0042` and month keys
//! only, for every origin: never an amount, a payee or a memo.

use std::collections::{BTreeSet, HashSet};

use books_core::date::{Date, Period, PeriodKind};
use books_core::ledger::Status;
use books_store::Store;
use books_store::books::Book;
use books_store::entries::{self, Filter, Form};
use books_store::months;
use books_store::people;
use serde_json::{Map, Value, json};

use crate::service::{Answer, Refusal, text};
use crate::views::{Chart, Summary};

/// The check ids, in output order.
pub const CHECKS: [&str; 6] = [
    "uncategorized",
    "stale_draft",
    "unmatched_month",
    "weekend_date",
    "round_amount",
    "missing_person",
];

/// At most this many findings are listed (counts are never capped).
pub const MAX: usize = 200;

/// A draft older than this many days is stale.
const STALE_DAYS: i64 = 30;
/// 1,000.00 and 10,000.00 in centavos.
const ROUND_STEP: i64 = 100_000;
const ROUND_FLOOR: i64 = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub check: &'static str,
    pub entry_id: Option<i64>,
    /// The entry's number, for ordering and the `entry 0042` ticket.
    pub entry_no: Option<i64>,
    pub month: Option<String>,
}

impl Finding {
    fn entry(check: &'static str, e: &entries::Entry) -> Self {
        Self {
            check,
            entry_id: Some(e.id),
            entry_no: Some(e.no),
            month: None,
        }
    }
}

fn is_money(form: Form) -> bool {
    matches!(form, Form::MoneyIn | Form::MoneyOut)
}

/// Every finding in `book` as of `today`, ordered by check then entry
/// number (uncapped; the tool caps).
pub fn findings(store: &Store, book: Book, today: Date) -> Result<Vec<Finding>, Refusal> {
    let c = store.conn();
    let chart = Chart::load(store, book)?;
    let book_id = chart.book.id;
    let has_people = !people::people(c, book_id, None)?.is_empty();
    let no_person: HashSet<i64> = if has_people {
        let mut stmt = c
            .prepare("SELECT entry_id FROM entry WHERE book_id = ?1 AND person_id IS NULL")
            .map_err(books_store::StoreError::from)?;
        stmt.query_map([book_id], |r| r.get::<_, i64>(0))
            .map_err(books_store::StoreError::from)?
            .collect::<Result<_, _>>()
            .map_err(books_store::StoreError::from)?
    } else {
        HashSet::new()
    };
    let this_month = Period::of(today, PeriodKind::Month);
    let mut list = entries::list(c, book_id, &Filter::default())?;
    list.sort_by_key(|e| e.no);

    let mut by_check: Vec<Vec<Finding>> = vec![Vec::new(); CHECKS.len()];
    // (account, month) pairs with posted activity on a cash account
    let mut used: BTreeSet<(String, i64)> = BTreeSet::new();
    for e in list {
        if e.status == Status::Draft {
            let created = Date::manila_from_iso_instant(&e.created_at)
                .or_else(|| Date::parse(e.created_at.get(..10).unwrap_or("")));
            if created.is_some_and(|d| d.add_days(STALE_DAYS) < today) {
                by_check[1].push(Finding::entry(CHECKS[1], &e));
            }
            continue;
        }
        if e.status != Status::Posted || e.form == Form::Reversal {
            continue;
        }
        let s = Summary::of(store, &chart, e)?;
        let e = &s.entry;
        if s.needs_review() {
            by_check[0].push(Finding::entry(CHECKS[0], e));
        }
        if e.date < this_month.start {
            let key = Period::of(e.date, PeriodKind::Month).key;
            for l in &s.lines {
                if chart.by_id.get(&l.account_id).is_some_and(|a| a.is_cash()) {
                    used.insert((key.clone(), l.account_id));
                }
            }
        }
        if e.date.weekday() >= 5 {
            by_check[3].push(Finding::entry(CHECKS[3], e));
        }
        if is_money(e.form) {
            let amount = i128::from(s.amount()).abs();
            if amount >= i128::from(ROUND_FLOOR) && amount % i128::from(ROUND_STEP) == 0 {
                by_check[4].push(Finding::entry(CHECKS[4], e));
            }
            if no_person.contains(&e.id) {
                by_check[5].push(Finding::entry(CHECKS[5], e));
            }
        }
    }
    let statements = months::statements(c, book_id)?;
    let mut unmatched: BTreeSet<String> = BTreeSet::new();
    for (key, account) in used {
        let done = statements
            .iter()
            .any(|s| s.account_id == account && s.period_key == key && s.status == "finished");
        if !done {
            unmatched.insert(key);
        }
    }
    by_check[2] = unmatched
        .into_iter()
        .map(|m| Finding {
            check: CHECKS[2],
            entry_id: None,
            entry_no: None,
            month: Some(m),
        })
        .collect();
    Ok(by_check.into_iter().flatten().collect())
}

/// The entries with at least one finding (for the ledger's Checks filter).
pub fn flagged(store: &Store, book: Book, today: Date) -> Result<HashSet<i64>, Refusal> {
    Ok(findings(store, book, today)?
        .into_iter()
        .filter_map(|f| f.entry_id)
        .collect())
}

/// The check ids each entry carries, for ledger rows.
pub fn by_entry(all: &[Finding]) -> std::collections::HashMap<i64, Vec<&'static str>> {
    let mut out: std::collections::HashMap<i64, Vec<&'static str>> =
        std::collections::HashMap::new();
    for f in all {
        if let Some(id) = f.entry_id {
            out.entry(id).or_default().push(f.check);
        }
    }
    out
}

/// `{check: n}` over every check id.
pub fn counts(all: &[Finding]) -> Map<String, Value> {
    CHECKS
        .iter()
        .map(|k| {
            (
                (*k).to_string(),
                json!(all.iter().filter(|f| f.check == *k).count()),
            )
        })
        .collect()
}

/// `books_checks`: counts per check and the findings as entry numbers and
/// month keys; `check` narrows to one id.
pub fn checks(store: &Store, book: Book, args: &Map<String, Value>) -> Answer {
    let only = match text(args, "check") {
        Some(k) => Some(*CHECKS.iter().find(|c| **c == k).ok_or_else(|| {
            Refusal::invalid(format!("`check` is one of {}.", CHECKS.join(", ")))
        })?),
        None => None,
    };
    let mut all = findings(store, book, Date::today())?;
    if let Some(k) = only {
        all.retain(|f| f.check == k);
    }
    let counts = counts(&all);
    let total = all.len();
    let list: Vec<Value> = all
        .iter()
        .take(MAX)
        .map(|f| {
            json!({
                "check": f.check,
                "entry": f.entry_no.map(|n| format!("entry {n:04}")),
                "month": f.month,
            })
        })
        .collect();
    Ok(json!({
        "counts": counts,
        "total": total,
        "truncated": total > MAX,
        "findings": list,
    }))
}
