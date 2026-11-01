//! The pages' data, shaped for the app: every amount arrives as display
//! text (`80,000.00`, `+80,000.00`, `−1,350.00`) and every bar as a whole
//! percent, so an app never does arithmetic on money (invariant 2).
//!
//! The model sees totals and tickets only (D11 option a): payees, memos,
//! references and lines are for the app.

use std::collections::HashMap;

use books_core::catalog::AccountType;
use books_core::date::{Date, MONTHS, Period, PeriodKind};
use books_core::ledger::{KindRef, Status};
use books_core::money;
use books_store::Store;
use books_store::books::{self, Account, Book};
use books_store::entries::{self, Entry, Filter, Form, LineRow};
use serde_json::{Map, Value, json};

use crate::service::{self, Origin, Refusal, ticket};

const WEEKDAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

/// `+80,000.00` or `−1,350.00`; zero is `0.00`.
pub fn signed(c: i64) -> String {
    if c > 0 {
        format!("+{}", money::display(c))
    } else {
        money::display(c)
    }
}

/// A whole percent of `part` in `whole` (0 to 100), for a bar.
pub fn pct(part: i64, whole: i64) -> i64 {
    if whole == 0 {
        return 0;
    }
    let p = i128::from(part).abs() * 100 / i128::from(whole).abs();
    i64::try_from(p.min(100)).unwrap_or(100)
}

/// A category's dot color: by kind, as the drawing colors them.
pub fn color(a: &Account) -> &'static str {
    match &a.kind {
        KindRef::Unsure | KindRef::Mixed => "#ff7d75",
        KindRef::Kind(k) => match k.as_str() {
            "sales_revenue" | "subscription_revenue" | "transaction_revenue" => "#3fcf89",
            "refunds_and_discounts" => "#8fd9b0",
            "rent" | "utilities" | "repairs_and_maintenance" => "#7fb2ff",
            "salaries"
            | "employee_benefits"
            | "retirement_contributions"
            | "employer_payroll_taxes" => "#b79bff",
            "business_applications_and_software" | "hosting_fees" => "#5fd0d6",
            "office_supplies" | "inventory" | "cost_of_goods_sold" => "#e89b6c",
            "loan_payable" => "#a0a0a0",
            "bank_fees" | "payment_processing_fees" => "#f3b64c",
            _ => match a.account_type {
                AccountType::Revenue | AccountType::OtherIncome => "#3fcf89",
                AccountType::Asset => "#7fb2ff",
                AccountType::Liability => "#a0a0a0",
                AccountType::Equity => "#c9b27a",
                _ => "#d0a1e8",
            },
        },
    }
}

/// The status pill: `(tone, label)`.
pub fn status_pill(e: &Entry) -> (&'static str, &'static str) {
    match e.status {
        Status::Draft if e.needs_review => ("rev", "Needs review"),
        Status::Draft => ("draft", "Draft"),
        Status::Posted if e.form == Form::Reversal => ("in", "Reversal"),
        Status::Posted => ("ok", "Posted"),
        Status::Voided => ("mute", "Voided"),
        Status::Discarded => ("mute", "Discarded"),
    }
}

/// The accounts of a book, by id.
pub struct Chart {
    pub book: Book,
    pub by_id: HashMap<i64, Account>,
    pub list: Vec<Account>,
}

impl Chart {
    pub fn load(store: &Store, book: Book) -> Result<Self, Refusal> {
        let list = books::accounts(store.conn(), book.id)?;
        let by_id = list.iter().map(|a| (a.id, a.clone())).collect();
        Ok(Self { book, by_id, list })
    }

    pub fn hidden(&self, a: &Account) -> bool {
        a.accrual_only() && self.book.basis == books_core::ledger::Basis::Cash
    }
}

/// One entry as a row: the category, the cash effect and the title.
pub struct Summary {
    pub entry: Entry,
    pub lines: Vec<LineRow>,
    pub category: Option<Account>,
    /// Debits minus credits on cash accounts: money in is positive.
    pub cash: i64,
    pub debits: i64,
    /// For a reversal: what the voided entry was.
    pub voided_what: Option<String>,
    /// A line sits on a "not sure yet" account: it holds every box it
    /// could feed until the user picks a category (plan 2.2).
    pub unsure: bool,
}

impl Summary {
    pub fn of(store: &Store, chart: &Chart, entry: Entry) -> Result<Self, Refusal> {
        let lines = entries::lines(store.conn(), entry.id)?;
        let is_cash = |l: &LineRow| chart.by_id.get(&l.account_id).is_some_and(Account::is_cash);
        let cash = lines
            .iter()
            .filter(|l| is_cash(l))
            .map(|l| l.debit - l.credit)
            .sum();
        let debits = lines.iter().map(|l| l.debit).sum();
        // the category: a transfer's destination; else the first income or
        // expense line, then the first line off the money accounts
        let pl_line = |l: &&LineRow| {
            chart
                .by_id
                .get(&l.account_id)
                .is_some_and(|a| a.account_type.is_pl())
        };
        let pick = match entry.form {
            Form::Transfer => lines.iter().find(|l| l.debit > 0),
            _ => lines
                .iter()
                .find(pl_line)
                .or_else(|| lines.iter().find(|l| !is_cash(l)))
                .or(lines.first()),
        };
        let category = pick.and_then(|l| chart.by_id.get(&l.account_id).cloned());
        let unsure = lines.iter().any(|l| {
            chart
                .by_id
                .get(&l.account_id)
                .is_some_and(|a| a.kind == KindRef::Unsure)
        });
        let voided_what = match entry.reverses {
            Some(id) => entries::get(store.conn(), id)?.map(|o| {
                o.memo
                    .or_else(|| category.as_ref().map(|a| a.name.clone()))
                    .unwrap_or_else(|| format!("entry {:04}", o.no))
            }),
            None => None,
        };
        Ok(Self {
            entry,
            lines,
            category,
            cash,
            debits,
            voided_what,
            unsure,
        })
    }

    /// The amount a row shows: the cash effect, else the debit total.
    /// Waits for the user: a draft from a statement, or a posted entry on
    /// a "not sure yet" account that no void has cancelled.
    pub fn needs_review(&self) -> bool {
        let e = &self.entry;
        match e.status {
            Status::Draft => e.needs_review,
            Status::Posted => {
                self.unsure
                    && e.form != Form::Reversal
                    && e.recategorized_by.is_none()
                    && e.recategorizes.is_none()
            }
            _ => false,
        }
    }

    pub fn amount(&self) -> i64 {
        if self.cash != 0 {
            self.cash
        } else {
            self.debits
        }
    }

    pub fn what(&self) -> String {
        let e = &self.entry;
        match e.form {
            Form::Reversal => format!(
                "Void of {}",
                self.voided_what.as_deref().unwrap_or("an entry")
            ),
            _ => e
                .memo
                .clone()
                .or_else(|| self.category.as_ref().map(|a| a.name.clone()))
                .unwrap_or_else(|| "Entry".into()),
        }
    }

    pub fn row(&self, store: &Store, origin: Origin) -> Result<Value, Refusal> {
        let e = &self.entry;
        let (tone, label) = if self.needs_review() {
            ("rev", "Needs review")
        } else {
            status_pill(e)
        };
        let amount = self.amount();
        let mut row = json!({
            "ticket": ticket(store, "entry", e.id)?,
            "no": format!("{:04}", e.no),
            "date": e.date.to_string(),
            "date_short": e.date.short(),
            "status": e.status.as_str(),
            "pill": tone,
            "status_label": label,
            "form": e.form.as_str(),
            "category": self.category.as_ref().map(|a| a.name.clone()),
            "category_color": self.category.as_ref().map(color),
            "amount": money::format(amount),
            "amount_shown": signed(amount),
            "tone": if amount > 0 { "pos" } else if amount < 0 { "neg" } else { "" },
        });
        if origin.is_app() {
            let who = e.payee.clone().unwrap_or_default();
            let what = self.what();
            row["title"] = json!(if who.is_empty() {
                what
            } else {
                format!("{what} · {who}")
            });
            row["sub"] = json!(
                e.reference
                    .clone()
                    .or_else(|| e.memo.as_ref().map(|_| String::new()))
                    .unwrap_or_default()
            );
            row["payee"] = json!(e.payee);
        }
        crate::inbox::row_keys(store.conn(), e, &mut row)?;
        Ok(row)
    }
}

fn entries_in(store: &Store, chart: &Chart, filter: &Filter) -> Result<Vec<Summary>, Refusal> {
    entries::list(store.conn(), chart.book.id, filter)?
        .into_iter()
        .map(|e| Summary::of(store, chart, e))
        .collect()
}

/// Money in and out of the cash accounts over posted entries in `period`,
/// leaving transfers out; a void and its reversal net to zero.
pub fn cash_flow(store: &Store, chart: &Chart, period: &Period) -> Result<(i64, i64), Refusal> {
    let list = entries_in(
        store,
        chart,
        &Filter {
            from: Some(period.start),
            to: Some(period.end),
            ..Filter::default()
        },
    )?;
    let (mut inflow, mut outflow) = (0i64, 0i64);
    for s in list.iter().filter(|s| {
        matches!(s.entry.status, Status::Posted | Status::Voided) && s.entry.form != Form::Transfer
    }) {
        if s.cash > 0 {
            inflow += s.cash;
        } else {
            outflow += s.cash;
        }
    }
    Ok((inflow, outflow))
}

pub fn book_json(store: &Store, book: &Book) -> Result<Value, Refusal> {
    let today = Date::today();
    Ok(json!({
        "ticket": ticket(store, "book", book.id)?,
        "label": book.label,
        "basis": book.basis.as_str(),
        "basis_label": if book.basis == books_core::ledger::Basis::Cash { "Cash basis" } else { "Accrual basis" },
        "year": today.year,
        "fiscal_year_start": book.fiscal_year_start,
    }))
}

/// The Today page.
pub fn today(store: &Store, book: Book, origin: Origin) -> Result<Value, Refusal> {
    let today = Date::today();
    let chart = Chart::load(store, book)?;
    let balances = books::movements(store.conn(), chart.book.id, None, Some(today))?;
    let bal = |a: &Account| a.natural(*balances.get(&a.id).unwrap_or(&0));

    let cash_accounts: Vec<&Account> = chart
        .list
        .iter()
        .filter(|a| a.is_cash() && a.active)
        .collect();
    let cash_total: i64 = cash_accounts.iter().map(|a| bal(a)).sum();
    let owed: Vec<&Account> = chart
        .list
        .iter()
        .filter(|a| a.kind == KindRef::Kind("loan_payable".into()) && bal(a) != 0)
        .collect();
    let biggest = cash_accounts
        .iter()
        .map(|a| bal(a).abs())
        .chain(owed.iter().map(|a| bal(a).abs()))
        .max()
        .unwrap_or(0);
    let shown_cash: Vec<&&Account> = cash_accounts.iter().filter(|a| bal(a) != 0).collect();
    let mut cash_rows: Vec<Value> = if shown_cash.is_empty() {
        cash_accounts.iter().take(1).collect::<Vec<_>>()
    } else {
        shown_cash
    }
    .iter()
    .map(|a| json!({ "label": a.name, "shown": money::display(bal(a)), "pct": pct(bal(a), biggest), "tone": "pos" }))
    .collect();
    for a in &owed {
        cash_rows.push(json!({ "label": format!("{} owed", a.name), "shown": money::display(-bal(a)), "pct": pct(bal(a), biggest), "tone": "neg" }));
    }

    // the month so far is its P&L: income less expenses (a void and its
    // reversal net to zero; transfers and owner money are not income)
    let month = Period::of(today, PeriodKind::Month);
    let (inflow, spent) = crate::reports::pl_totals(store, &chart, &month)?;
    let outflow = -spent;
    let net = inflow + outflow;
    let prev = month.previous();
    let (pin, pout) = crate::reports::pl_totals(store, &chart, &prev)?;
    let prev_net = pin - pout;
    let scale = inflow.abs().max(outflow.abs()).max(prev_net.abs());

    let all = entries_in(store, &chart, &Filter::default())?;
    let review = all.iter().filter(|s| s.needs_review()).count();
    let drafts: Vec<&Summary> = all
        .iter()
        .filter(|s| s.entry.status == Status::Draft && !s.entry.needs_review)
        .collect();
    let mut needs = Vec::new();
    if review > 0 {
        needs.push(json!({ "kind": "review", "count": review, "title": "Bank lines to categorize", "detail": "Not sure yet · pick a category for each", "go": "ledger", "filter": "review", "tone": "rev" }));
    }
    // flagged entries of this month: the ledger the item opens lists them
    let this_month = Period::of(today, PeriodKind::Month);
    let checks = crate::findings::flagged(store, chart.book.clone(), today, &this_month)?.len();
    if checks > 0 {
        needs.push(json!({ "kind": "checks", "count": checks, "title": "Entries to look at twice", "detail": "Review findings · open the Checks filter", "go": "ledger", "filter": "checks", "tone": "rev" }));
    }
    if !drafts.is_empty() {
        let detail = if origin.is_app() {
            drafts
                .iter()
                .take(2)
                .map(|s| {
                    let who = s.entry.payee.clone().unwrap_or_else(|| s.what());
                    format!("{who} · {}", money::display(s.amount().abs()))
                })
                .collect::<Vec<_>>()
                .join(" · ")
                + " · post or edit"
        } else {
            "Post or edit them in Books".into()
        };
        let title = if drafts.len() == 1 {
            "A draft to post"
        } else {
            "Drafts to post"
        };
        needs.push(json!({ "kind": "draft", "count": drafts.len(), "title": title, "detail": detail, "go": "ledger", "filter": "draft", "tone": "draft" }));
    }

    let weekday = WEEKDAYS[usize::from(today.weekday())];
    let month_name = MONTHS[usize::from(today.month - 1)];
    let prev_name = MONTHS[usize::from(prev.start.month - 1)];
    Ok(json!({
        "book": book_json(store, &chart.book)?,
        "today": today.to_string(),
        "today_label": format!("{weekday}, {month_name} {}", today.day),
        "month_name": books_core::date::MONTH_NAMES[usize::from(today.month - 1)],
        "cash": { "label": "Cash now", "total": money::format(cash_total), "shown": money::display(cash_total), "rows": cash_rows },
        "month": {
            "label": format!("{} so far", books_core::date::MONTH_NAMES[usize::from(today.month - 1)]),
            "net": money::format(net),
            "shown": signed(net).replace('+', "+ "),
            "tone": if net >= 0 { "pos" } else { "neg" },
            "rows": [
                { "label": "Money in", "shown": money::display(inflow), "pct": pct(inflow, scale), "tone": "pos" },
                { "label": "Money out", "shown": money::display(outflow), "pct": pct(outflow, scale), "tone": "neg" },
                { "label": format!("vs {prev_name}"), "shown": money::display(prev_net), "pct": pct(prev_net, scale), "tone": "blue" },
            ],
        },
        "unpaid": crate::records::owed_summary(store, &chart.book)?,
        "needs": needs,
        "needs_count": needs.iter().map(|n| n["count"].as_i64().unwrap_or(0)).sum::<i64>(),
        "counts": { "review": review, "drafts": drafts.len(), "entries": all.len() },
    }))
}

/// The month a page shows: `month` (`2026-10`) or this month.
pub fn month_arg(args: &Map<String, Value>) -> Result<Period, Refusal> {
    match service::text(args, "month") {
        Some(m) => Period::parse(m)
            .filter(|p| p.kind == PeriodKind::Month)
            .ok_or_else(|| Refusal::invalid("`month` is YYYY-MM.")),
        None => Ok(Period::of(Date::today(), PeriodKind::Month)),
    }
}

fn account_row(store: &Store, chart: &Chart, a: &Account, balance: i64) -> Result<Value, Refusal> {
    let off = chart.hidden(a);
    let (tag, tag_label) = match a.account_type.group() {
        "Assets" => ("as", "Asset"),
        "Liabilities" => ("li", "Liability"),
        "Equity" => ("eq", "Equity"),
        "Revenue" => ("re", "Revenue"),
        _ => ("ex", "Expense"),
    };
    Ok(json!({
        "ticket": ticket(store, "account", a.id)?,
        "number": a.number,
        "name": a.name,
        "type": a.account_type.as_str(),
        "group": a.account_type.group(),
        "tag": tag,
        "tag_label": tag_label,
        "kind": a.kind.as_str(),
        "accrual_only": a.accrual_only(),
        "off": off,
        "cash": a.is_cash(),
        "balance": if off { Value::Null } else { json!(money::format(balance)) },
        "balance_shown": if off { "—".to_string() } else { money::display(balance) },
    }))
}

const GROUPS: [&str; 5] = ["Assets", "Liabilities", "Equity", "Revenue", "Expenses"];

/// The Accounts page: the chart with balances, filtered by group.
pub fn accounts(store: &Store, book: Book, args: &Map<String, Value>) -> Result<Value, Refusal> {
    let chart = Chart::load(store, book)?;
    let balances = books::movements(store.conn(), chart.book.id, None, Some(Date::today()))?;
    let group = service::text(args, "group").unwrap_or("all");
    let mut rows = Vec::new();
    for a in &chart.list {
        if group != "all" && a.account_type.group() != group {
            continue;
        }
        rows.push(account_row(
            store,
            &chart,
            a,
            a.natural(*balances.get(&a.id).unwrap_or(&0)),
        )?);
    }
    let groups: Vec<&str> = ["all"].into_iter().chain(GROUPS).collect();
    Ok(json!({
        "book": book_json(store, &chart.book)?,
        "group": group,
        "groups": groups,
        "rows": rows,
        "note": if chart.book.basis == books_core::ledger::Basis::Cash { "Receivable and payable accounts are for accrual books only." } else { "" },
    }))
}

/// The Ledger page: the account list, the month's entries of one account
/// (or all), filter counts.
pub fn ledger(
    store: &Store,
    book: Book,
    args: &Map<String, Value>,
    origin: Origin,
) -> Result<Value, Refusal> {
    let chart = Chart::load(store, book)?;
    let month = month_arg(args)?;
    let selected = match service::text(args, "account") {
        Some(r) if !r.is_empty() && r != "all" => Some(service::account(store, &chart.book, r)?),
        _ => None,
    };
    let balances = books::movements(store.conn(), chart.book.id, None, Some(month.end))?;
    // the accounts in use (and every cash account), unless all are asked for
    let used: std::collections::HashSet<i64> = {
        let mut stmt = store
            .conn()
            .prepare("SELECT DISTINCT l.account_id FROM line l JOIN entry e ON e.entry_id = l.entry_id WHERE e.book_id = ?1 AND e.status <> 'discarded'")
            .map_err(books_store::StoreError::from)?;
        stmt.query_map([chart.book.id], |r| r.get::<_, i64>(0))
            .map_err(books_store::StoreError::from)?
            .collect::<Result<_, _>>()
            .map_err(books_store::StoreError::from)?
    };
    let every = args
        .get("all_accounts")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut groups = Vec::new();
    for g in GROUPS {
        let rows: Vec<Value> = chart
            .list
            .iter()
            .filter(|a| a.account_type.group() == g && !chart.hidden(a) && a.active)
            .filter(|a| {
                every
                    || used.contains(&a.id)
                    || (a.is_cash() && used.is_empty())
                    || selected.as_ref().is_some_and(|s| s.id == a.id)
            })
            .map(|a| -> Result<Value, Refusal> {
                Ok(json!({ "ticket": ticket(store, "account", a.id)?, "number": a.number, "name": a.name }))
            })
            .collect::<Result<_, _>>()?;
        if !rows.is_empty() {
            groups.push(json!({ "group": g, "rows": rows }));
        }
    }
    let list = entries_in(
        store,
        &chart,
        &Filter {
            from: Some(month.start),
            to: Some(month.end),
            account_id: selected.as_ref().map(|a| a.id),
            ..Filter::default()
        },
    )?;
    let flagged = crate::inbox::flagged(store.conn(), list.iter().map(|s| &s.entry))?;
    let found =
        crate::findings::findings_in(store, chart.book.clone(), Date::today(), Some(&month))?;
    let marks = crate::findings::by_entry(&found);
    let is = |s: &Summary, f: &str| match f {
        "checks" => marks.contains_key(&s.entry.id),
        "review" => s.needs_review() || flagged.contains(&s.entry.id),
        "draft" => s.entry.status == Status::Draft && !s.entry.needs_review,
        "posted" => matches!(s.entry.status, Status::Posted | Status::Voided),
        _ => true,
    };
    // one account picked: the balance after each posted or voided entry,
    // the same walk as account activity (drafts carry none)
    let running = match &selected {
        Some(a) if !chart.hidden(a) => {
            Some(crate::activity::balances_after(store, &chart, a, &month)?)
        }
        _ => None,
    };
    let filter = service::text(args, "filter").unwrap_or("all");
    let counts = json!({
        "all": list.len(),
        "review": list.iter().filter(|s| is(s, "review")).count(),
        "draft": list.iter().filter(|s| is(s, "draft")).count(),
        "posted": list.iter().filter(|s| is(s, "posted")).count(),
        "checks": list.iter().filter(|s| is(s, "checks")).count(),
    });
    let rows = list
        .iter()
        .filter(|s| is(s, filter))
        .take(200)
        .map(|s| {
            let mut row = s.row(store, origin)?;
            row["checks"] = json!(marks.get(&s.entry.id).cloned().unwrap_or_default());
            if let Some(running) = &running {
                row["balance_shown"] = json!(running.get(&s.entry.id).map(|b| money::display(*b)));
            }
            Ok(row)
        })
        .collect::<Result<Vec<_>, Refusal>>()?;
    let header = match &selected {
        Some(a) => json!({
            "ticket": ticket(store, "account", a.id)?, "number": a.number, "name": a.name,
            "balance_shown": money::display(a.natural(*balances.get(&a.id).unwrap_or(&0))),
        }),
        None => {
            let cash: i64 = chart
                .list
                .iter()
                .filter(|a| a.is_cash())
                .map(|a| a.natural(*balances.get(&a.id).unwrap_or(&0)))
                .sum();
            json!({ "ticket": null, "number": "", "name": "All accounts", "balance_shown": money::display(cash), "balance_label": "Cash" })
        }
    };
    let (s, e) = (month.start, month.end);
    Ok(json!({
        "book": book_json(store, &chart.book)?,
        "month": { "key": month.key, "label": month.label(), "range": format!("{} {} – {} {}", MONTHS[usize::from(s.month - 1)], s.day, MONTHS[usize::from(e.month - 1)], e.day), "prev": month.previous().key, "next": Period::of(month.end.add_days(1), PeriodKind::Month).key },
        "accounts": groups,
        "all_accounts": every,
        "account": header,
        "filter": filter,
        "counts": counts,
        "running_balance": running.is_some(),
        "rows": rows,
    }))
}

/// One entry: the drawer.
pub fn entry(
    store: &Store,
    book: Book,
    ticket_text: &str,
    origin: Origin,
) -> Result<Value, Refusal> {
    let id = service::resolve(store, "entry", ticket_text)?;
    let chart = Chart::load(store, book)?;
    let e = entries::get(store.conn(), id)?
        .filter(|e| e.book_id == chart.book.id)
        .ok_or_else(|| Refusal::new("not_found", "That entry is not in this book."))?;
    let s = Summary::of(store, &chart, e)?;
    let mut out = s.row(store, origin)?;
    out["review"] = crate::inbox::review_json(store.conn(), &s.entry, origin)?;
    let lines: Vec<Value> = s
        .lines
        .iter()
        .map(|l| {
            let a = chart.by_id.get(&l.account_id);
            json!({
                "account": a.map(|a| format!("{} {}", a.number, a.name)),
                "debit_shown": if l.debit > 0 { money::display(l.debit) } else { String::new() },
                "credit_shown": if l.credit > 0 { money::display(l.credit) } else { String::new() },
            })
        })
        .collect();
    let e = &s.entry;
    let actions: Vec<&str> = match e.status {
        Status::Draft => vec!["discard", "edit", "post"],
        Status::Posted if e.form != Form::Reversal => vec!["void", "amend", "copy"],
        _ => vec![],
    };
    // whether a post would go through, without changing anything
    let blocker = if e.status == Status::Draft {
        let core = entries::core_lines(store.conn(), &s.lines)?;
        books_core::ledger::check(&core, chart.book.basis)
            .err()
            .map(|x| x.message)
    } else {
        None
    };
    let related = match (e.reverses, e.voided_by) {
        (Some(o), _) => {
            entries::get(store.conn(), o)?.map(|o| format!("Reverses entry {:04}", o.no))
        }
        (_, Some(r)) => {
            entries::get(store.conn(), r)?.map(|r| format!("Cancelled by entry {:04}", r.no))
        }
        _ => None,
    };
    let (links, link_labels, replaces) = crate::amend::links(store, e)?;
    let related = related.or(replaces);
    // for a line that needs review: the categories this payee used before,
    // then what the bank's words name (books_app_recategorize)
    let mut suggest: Vec<Value> = Vec::new();
    if s.needs_review() && e.status == Status::Posted && origin.is_app() {
        let mut ids: Vec<i64> = Vec::new();
        if let Some(payee) = &e.payee {
            let mut stmt = store
                .conn()
                .prepare(
                    "SELECT l.account_id, count(*) n FROM entry x JOIN line l ON l.entry_id = x.entry_id JOIN account a ON a.account_id = l.account_id
                     WHERE x.book_id = ?1 AND x.payee = ?2 AND x.status IN ('posted','voided') AND a.kind NOT IN ('unsure', 'mixed')
                       AND a.type NOT IN ('asset','liability','equity')
                     GROUP BY l.account_id ORDER BY n DESC LIMIT 2",
                )
                .map_err(books_store::StoreError::from)?;
            ids = stmt
                .query_map(books_store::rusqlite::params![chart.book.id, payee], |r| {
                    r.get::<_, i64>(0)
                })
                .map_err(books_store::StoreError::from)?
                .collect::<Result<_, _>>()
                .map_err(books_store::StoreError::from)?;
        }
        let words = format!(
            "{} {}",
            e.memo.clone().unwrap_or_default(),
            e.payee.clone().unwrap_or_default()
        );
        let words = if words.to_ascii_lowercase().contains("fee") {
            format!("{words} BANK FEE")
        } else {
            words
        };
        if let Some(kind) = books_core::statement::suggest_kind(&words, s.amount())
            && let Some(a) = chart
                .list
                .iter()
                .find(|a| a.kind == KindRef::Kind(kind.into()))
            && !ids.contains(&a.id)
        {
            ids.push(a.id);
        }
        for (i, id) in ids.iter().take(3).enumerate() {
            if let Some(a) = chart.by_id.get(id) {
                suggest.push(json!({ "ticket": ticket(store, "account", a.id)?, "name": a.name, "best": i == 0 }));
            }
        }
    }
    let amount = s.amount();
    let obj = out.as_object_mut().expect("a row is an object");
    obj.insert("suggest".into(), json!(suggest));
    obj.insert(
        "reviewable".into(),
        json!(s.needs_review() && e.status == Status::Posted),
    );
    obj.insert("big".into(), json!(signed(amount)));
    obj.insert(
        "date_label".into(),
        json!(format!("{}, {}", e.date.short(), e.date.year)),
    );
    obj.insert("actions".into(), json!(actions));
    obj.insert("post_blocker".into(), json!(blocker));
    obj.insert("related".into(), json!(related));
    obj.insert("links".into(), links);
    obj.insert("link_labels".into(), link_labels);
    if origin.is_app() {
        obj.insert(
            "who_label".into(),
            json!(if amount > 0 { "From" } else { "To" }),
        );
        obj.insert("memo".into(), json!(e.memo));
        obj.insert("reference".into(), json!(e.reference));
        obj.insert("lines".into(), json!(lines));
    }
    Ok(out)
}

/// What the new-entry sheet offers: the money accounts, the categories for
/// money in and out, every account for a journal, and today.
pub fn entry_options(store: &Store, book: Book) -> Result<Value, Refusal> {
    let chart = Chart::load(store, book)?;
    let pick = |f: &dyn Fn(&Account) -> bool| -> Result<Vec<Value>, Refusal> {
        chart
            .list
            .iter()
            .filter(|a| a.active && !chart.hidden(a) && f(a))
            .map(|a| {
                Ok(json!({ "ticket": ticket(store, "account", a.id)?, "number": a.number, "name": a.name, "label": format!("{} {}", a.number, a.name) }))
            })
            .collect()
    };
    let money_side = |a: &Account| a.is_cash() || a.kind == KindRef::Kind("loan_payable".into());
    Ok(json!({
        "book": book_json(store, &chart.book)?,
        "today": Date::today().to_string(),
        "money": pick(&money_side)?,
        "income": pick(&|a: &Account| matches!(a.account_type, AccountType::Revenue | AccountType::OtherIncome) || (a.account_type == AccountType::Equity))?,
        "expense": pick(&|a: &Account| matches!(a.account_type, AccountType::Expense | AccountType::CostOfRevenue | AccountType::OtherExpense) || (a.account_type == AccountType::Asset && !a.is_cash()) || (a.account_type == AccountType::Equity))?,
        "all": pick(&|_| true)?,
    }))
}
