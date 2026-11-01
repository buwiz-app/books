//! M2 pages: People (one page, seven kinds), Tags (departments and
//! locations) and Unpaid (to collect and to pay). Tool names avoid the
//! words Books never uses for a tool (invariant 7: pay, paid, file, send,
//! submit, queue), so the Unpaid tools are `books_owed_*` and marking a
//! document paid is "settle".

use books_core::date::{Date, MONTHS, Period, PeriodKind};
use books_core::ledger::{KindRef, Status};
use books_core::money;
use books_store::Store;
use books_store::books::{Account, Book};
use books_store::entries::{self, Filter, Form};
use books_store::owed_payments;
use books_store::people::{self, Person};
use books_store::plumbing;
use books_store::rusqlite::{Connection, params};
use serde_json::{Map, Value, json};

use crate::service::{self, Answer, Origin, Refusal, text, ticket, with_subject};
use crate::views::{self, Chart, Summary, pct, signed};
use crate::writes;

/// The People page's kinds, in the drawing's order.
pub const KINDS: [(&str, &str); 7] = [
    ("banks", "Banks & cards"),
    ("vendors", "Vendors"),
    ("customers", "Customers"),
    ("employees", "Employees"),
    ("owners", "Owners"),
    ("lenders", "Lenders"),
    ("government", "Government"),
];

/// The store's person kind for a page kind.
pub fn person_kind(page_kind: &str) -> Option<&'static str> {
    Some(match page_kind {
        "vendors" => "vendor",
        "customers" => "customer",
        "employees" => "employee",
        "owners" => "owner",
        "lenders" => "lender",
        "government" => "government",
        _ => return None,
    })
}

const AVATARS: [&str; 8] = [
    "#157a48", "#2f6fd6", "#7a4fd6", "#c2603c", "#c98a1a", "#5a6b62", "#a2650f", "#13201a",
];

fn initials(name: &str) -> String {
    let letters: String = name
        .chars()
        .filter(char::is_ascii_alphabetic)
        .take(2)
        .collect();
    letters.to_ascii_uppercase()
}

fn avatar(id: i64) -> &'static str {
    AVATARS[usize::try_from(id.rem_euclid(8)).unwrap_or(0)]
}

fn money_side(a: &Account) -> bool {
    a.is_cash() || a.kind == KindRef::Kind("loan_payable".into())
}

/// Posted entries of a person, newest first.
pub(crate) fn person_entries(
    store: &Store,
    chart: &Chart,
    person_id: i64,
) -> Result<Vec<Summary>, Refusal> {
    let ids: Vec<i64> = {
        let mut stmt = store
            .conn()
            .prepare("SELECT entry_id FROM entry WHERE person_id = ?1 AND status IN ('posted','voided') ORDER BY entry_date DESC, entry_no DESC")
            .map_err(books_store::StoreError::from)?;
        stmt.query_map(params![person_id], |r| r.get::<_, i64>(0))
            .map_err(books_store::StoreError::from)?
            .collect::<Result<_, _>>()
            .map_err(books_store::StoreError::from)?
    };
    ids.into_iter()
        .filter_map(|id| entries::get(store.conn(), id).transpose())
        .map(|e| Summary::of(store, chart, e?))
        .collect()
}

/// Totals by month for the last ten months ending in `month`, as bar heights.
fn spark(list: &[(Date, i64)], month: &Period) -> Vec<Value> {
    let mut months = Vec::new();
    let mut p = month.clone();
    for _ in 0..10 {
        months.push(p.clone());
        p = p.previous();
    }
    months.reverse();
    let totals: Vec<i64> = months
        .iter()
        .map(|m| {
            list.iter()
                .filter(|(d, _)| m.contains(*d))
                .map(|(_, c)| c.abs())
                .sum()
        })
        .collect();
    let max = totals.iter().copied().max().unwrap_or(0);
    months
        .iter()
        .zip(&totals)
        .map(|(m, t)| json!({ "month": MONTHS[usize::from(m.start.month - 1)], "pct": if max == 0 { 0 } else { pct(*t, max).max(4) } }))
        .collect()
}

fn recent_rows(store: &Store, list: &[Summary], origin: Origin) -> Result<Vec<Value>, Refusal> {
    list.iter().take(6).map(|s| s.row(store, origin)).collect()
}

/// The People page: the kinds with counts, and the list of one kind.
pub fn people_list(store: &Store, book: Book, args: &Map<String, Value>) -> Answer {
    let chart = Chart::load(store, book)?;
    let kind = text(args, "kind").unwrap_or("vendors");
    if !KINDS.iter().any(|(k, _)| *k == kind) {
        return Err(Refusal::invalid(
            "`kind` is banks, vendors, customers, employees, owners, lenders or government.",
        ));
    }
    let balances =
        books_store::books::movements(store.conn(), chart.book.id, None, Some(Date::today()))?;
    let banks: Vec<&Account> = chart
        .list
        .iter()
        .filter(|a| a.active && money_side(a) && !chart.hidden(a))
        .collect();
    let mut kinds = Vec::new();
    for (k, label) in KINDS {
        let count = match person_kind(k) {
            Some(pk) => people::people(store.conn(), chart.book.id, Some(pk))?.len(),
            None => banks.len(),
        };
        kinds.push(json!({ "id": k, "label": label, "count": count }));
    }
    let rows: Vec<Value> = match person_kind(kind) {
        None => banks
            .iter()
            .map(|a| -> Result<Value, Refusal> {
                let bal = a.natural(*balances.get(&a.id).unwrap_or(&0));
                Ok(json!({
                    "ticket": ticket(store, "account", a.id)?, "name": a.name,
                    "sub": format!("{} · {}", if a.is_cash() { "Cash and bank" } else { "Card" }, money::display(bal)),
                    "initials": initials(&a.name), "color": avatar(a.id),
                }))
            })
            .collect::<Result<_, _>>()?,
        Some(pk) => people::people(store.conn(), chart.book.id, Some(pk))?
            .iter()
            .map(|p| -> Result<Value, Refusal> {
                let list = person_entries(store, &chart, p.id)?;
                let what = list.first().and_then(|s| s.category.as_ref().map(|a| a.name.clone()));
                let sub = match (what, list.len()) {
                    (_, 0) => p.note.clone().unwrap_or_else(|| "No entries yet".into()),
                    (Some(w), n) => format!("{w} · {n} {}", if n == 1 { "entry" } else { "entries" }),
                    (None, n) => format!("{n} entries"),
                };
                Ok(json!({ "ticket": ticket(store, "person", p.id)?, "name": p.name, "sub": sub, "initials": initials(&p.name), "color": avatar(p.id) }))
            })
            .collect::<Result<_, _>>()?,
    };
    Ok(json!({ "kind": kind, "kinds": kinds, "rows": rows }))
}

/// One person (or one bank or card account): three figures, ten months of
/// bars and the last entries.
pub fn person_get(store: &Store, book: Book, args: &Map<String, Value>, origin: Origin) -> Answer {
    let chart = Chart::load(store, book)?;
    let t = service::required(args, "person")?;
    let today = Date::today();
    let month = Period::of(today, PeriodKind::Month);
    let year = Period::of(today, PeriodKind::Year);
    let (kind, id) = plumbing::subject(store.conn(), t)?
        .ok_or_else(|| Refusal::new("not_found", "That ticket is not one Books issued."))?;
    if kind == "account" {
        let a = chart
            .by_id
            .get(&id)
            .cloned()
            .ok_or_else(|| Refusal::new("not_found", "That account is not in this book."))?;
        let balances =
            books_store::books::movements(store.conn(), chart.book.id, None, Some(today))?;
        let bal = a.natural(*balances.get(&a.id).unwrap_or(&0));
        let list: Vec<Summary> = entries::list(
            store.conn(),
            chart.book.id,
            &Filter {
                account_id: Some(a.id),
                ..Filter::default()
            },
        )?
        .into_iter()
        .filter(|e| matches!(e.status, Status::Posted | Status::Voided))
        .map(|e| Summary::of(store, &chart, e))
        .collect::<Result<_, _>>()?;
        let effect = |s: &Summary| -> i64 {
            s.lines
                .iter()
                .filter(|l| l.account_id == a.id)
                .map(|l| l.debit - l.credit)
                .sum()
        };
        let in_month: i64 = list
            .iter()
            .filter(|s| month.contains(s.entry.date))
            .map(effect)
            .filter(|c| *c > 0)
            .sum();
        let out_month: i64 = list
            .iter()
            .filter(|s| month.contains(s.entry.date))
            .map(effect)
            .filter(|c| *c < 0)
            .sum();
        let card = !a.is_cash();
        // on a card a charge is a credit (more owed) and a payment a debit
        let kpi = if card {
            vec![
                ("Owed", money::display(bal)),
                ("Spent this month", money::display(-out_month)),
                ("Paid this month", money::display(in_month)),
            ]
        } else {
            vec![
                ("Balance", money::display(bal)),
                ("In this month", money::display(in_month)),
                ("Out this month", money::display(out_month)),
            ]
        };
        let points: Vec<(Date, i64)> = list.iter().map(|s| (s.entry.date, effect(s))).collect();
        return Ok(json!({
            "ticket": t, "name": a.name, "sub": if card { "Card" } else { "Cash and bank" },
            "initials": initials(&a.name), "color": avatar(a.id), "bank": true,
            "kpi": kpi.iter().map(|(l, v)| json!({ "label": l, "shown": v })).collect::<Vec<_>>(),
            "spark": spark(&points, &month), "spark_tone": "pos",
            "recent": recent_rows(store, &list, origin)?,
        }));
    }
    if kind != "person" {
        return Err(Refusal::invalid(
            "`person` is a person or bank account ticket.",
        ));
    }
    let p: Person = people::person(store.conn(), id)?
        .filter(|p| p.book_id == chart.book.id)
        .ok_or_else(|| Refusal::new("not_found", "That person is not in this book."))?;
    let list = person_entries(store, &chart, p.id)?;
    let year_list: Vec<&Summary> = list
        .iter()
        .filter(|s| year.contains(s.entry.date))
        .collect();
    let received: i64 = year_list.iter().map(|s| s.cash).filter(|c| *c > 0).sum();
    let paid: i64 = -year_list
        .iter()
        .map(|s| s.cash)
        .filter(|c| *c < 0)
        .sum::<i64>();
    let last = list
        .first()
        .map_or_else(|| "—".to_string(), |s| s.entry.date.short());
    let open_owed = |side: &str| -> Result<i64, Refusal> {
        Ok(people::open_unpaid(store.conn(), chart.book.id, side)?
            .iter()
            .filter(|u| u.person_id == Some(p.id))
            .map(people::Unpaid::balance)
            .sum())
    };
    let kpi: Vec<(&str, String)> = match p.kind.as_str() {
        "customer" => vec![
            ("Received this year", money::display(received)),
            ("Unpaid", money::display(open_owed("collect")?)),
            ("Last", last),
        ],
        "employee" => vec![
            ("Paid this year", money::display(paid)),
            ("Entries", year_list.len().to_string()),
            ("Last", last),
        ],
        "owner" => vec![
            ("Put in", money::display(received)),
            ("Took out this year", money::display(paid)),
            ("Last", last),
        ],
        "lender" => vec![
            ("Received", money::display(received)),
            ("Paid this year", money::display(paid)),
            ("Last", last),
        ],
        _ => vec![
            ("Paid this year", money::display(paid)),
            ("Payments", year_list.len().to_string()),
            ("Last", last),
        ],
    };
    let points: Vec<(Date, i64)> = list.iter().map(|s| (s.entry.date, s.cash)).collect();
    let sub = list
        .first()
        .and_then(|s| s.category.as_ref().map(|a| a.name.clone()))
        .or_else(|| p.note.clone())
        .unwrap_or_else(|| "No entries yet".into());
    let mut out = json!({
        "ticket": t, "kind": p.kind, "initials": initials(&p.name), "color": avatar(p.id), "bank": false,
        "kpi": kpi.iter().map(|(l, v)| json!({ "label": l, "shown": v })).collect::<Vec<_>>(),
        "spark": spark(&points, &month), "spark_tone": if p.kind == "customer" || p.kind == "owner" { "pos" } else { "neg" },
        "recent": recent_rows(store, &list, origin)?,
        "tin_note": "TIN and registration live in eBIRForms. Books links by name only.",
    });
    if origin.is_app() {
        out["name"] = json!(p.name);
        out["sub"] = json!(sub);
        out["note"] = json!(p.note);
    }
    Ok(out)
}

/// The Tags page: one kind, with this month's money out and entry counts.
pub fn tags_list(store: &Store, book: Book, args: &Map<String, Value>) -> Answer {
    let chart = Chart::load(store, book)?;
    let kind = match text(args, "kind").unwrap_or("department") {
        k @ ("department" | "location") => k,
        _ => return Err(Refusal::invalid("`kind` is department or location.")),
    };
    let month = views::month_arg(args)?;
    let column = if kind == "department" {
        "department_id"
    } else {
        "location_id"
    };
    let mut rows = Vec::new();
    for t in people::tags(store.conn(), chart.book.id, Some(kind))? {
        let ids: Vec<i64> = {
            let mut stmt = store
                .conn()
                .prepare(&format!(
                    "SELECT entry_id FROM entry WHERE {column} = ?1 AND status IN ('posted','voided') AND entry_date >= ?2 AND entry_date <= ?3"
                ))
                .map_err(books_store::StoreError::from)?;
            stmt.query_map(
                params![t.id, month.start.to_string(), month.end.to_string()],
                |r| r.get::<_, i64>(0),
            )
            .map_err(books_store::StoreError::from)?
            .collect::<Result<_, _>>()
            .map_err(books_store::StoreError::from)?
        };
        let mut spent = 0i64;
        for id in &ids {
            if let Some(e) = entries::get(store.conn(), *id)? {
                let s = Summary::of(store, &chart, e)?;
                if s.cash < 0 && s.entry.form != Form::Transfer {
                    spent -= s.cash;
                }
            }
        }
        rows.push(json!({ "ticket": ticket(store, "tag", t.id)?, "name": t.name, "spent_shown": money::display(spent), "count": ids.len() }));
    }
    Ok(
        json!({ "kind": kind, "month": { "key": month.key, "label": month.label(), "name": books_core::date::MONTH_NAMES[usize::from(month.start.month - 1)] }, "rows": rows }),
    )
}

/// The Unpaid page: one side's open documents, by due date.
pub fn owed_list(store: &Store, book: Book, args: &Map<String, Value>, origin: Origin) -> Answer {
    let side = match text(args, "side").unwrap_or("collect") {
        s @ ("collect" | "pay") => s,
        _ => return Err(Refusal::invalid("`side` is collect or pay.")),
    };
    let today = Date::today();
    let collect = people::open_unpaid(store.conn(), book.id, "collect")?;
    let pay = people::open_unpaid(store.conn(), book.id, "pay")?;
    let list = if side == "collect" { &collect } else { &pay };
    let total: i64 = list.iter().map(people::Unpaid::balance).sum();
    let mut rows = Vec::new();
    for u in list {
        let who = match u.person_id {
            Some(p) => people::person(store.conn(), p)?.map(|p| p.name),
            None => None,
        };
        let mut row = json!({
            "ticket": ticket(store, "owed", u.id)?,
            "due": u.due.map(|d| d.to_string()),
            "due_short": u.due.map_or_else(|| "—".to_string(), Date::short),
            "late": u.due.is_some_and(|d| d < today),
            "amount": money::format(u.amount),
            "amount_shown": money::display(u.amount),
            "paid": money::format(u.paid),
            "paid_shown": money::display(u.paid),
            "balance": money::format(u.balance()),
            "balance_shown": money::display(u.balance()),
            "state": owed_state(u),
        });
        if origin.is_app() {
            row["who"] = json!(who.unwrap_or_else(|| "—".into()));
            row["doc"] = json!(
                u.reference
                    .clone()
                    .or_else(|| u.memo.clone())
                    .unwrap_or_default()
            );
        }
        rows.push(row);
    }
    let basis = book.basis;
    let detail = match text(args, "owed") {
        Some(t) => crate::owed_detail::detail(store, &book, t, origin)?,
        None => Value::Null,
    };
    Ok(json!({
        "side": side,
        "counts": { "collect": collect.len(), "pay": pay.len() },
        "rows": rows,
        "detail": detail,
        "total_shown": money::display(total),
        "basis": basis.as_str(),
        "note": if basis == books_core::ledger::Basis::Cash {
            "These are reminders, not entries. Mark paid records the money in the Ledger on the day it moves."
        } else {
            "On an accrual book each one is already a receivable or payable. Mark paid clears it."
        },
    }))
}

/// A document's state on the Unpaid page: `open` or `part_paid`.
pub fn owed_state(u: &people::Unpaid) -> &'static str {
    if u.paid > 0 { "part_paid" } else { "open" }
}

/// What the Unpaid sheet offers: people by side and the categories.
pub fn owed_options(store: &Store, book: Book) -> Answer {
    let opts = views::entry_options(store, book.clone())?;
    let pick = |kind: &str| -> Result<Vec<Value>, Refusal> {
        people::people(store.conn(), book.id, Some(kind))?
            .iter()
            .map(|p| Ok(json!({ "ticket": ticket(store, "person", p.id)?, "name": p.name, "label": p.name })))
            .collect()
    };
    Ok(json!({
        "today": Date::today().to_string(),
        "customers": pick("customer")?,
        "vendors": pick("vendor")?,
        "income": opts["income"],
        "expense": opts["expense"],
        "money": opts["money"],
    }))
}

fn person_in(
    conn: &Connection,
    book: &Book,
    args: &Map<String, Value>,
    side_kind: &str,
) -> Result<Option<i64>, Refusal> {
    if let Some(t) = text(args, "person").filter(|t| plumbing::is_ticket(t)) {
        return match plumbing::subject(conn, t)? {
            Some((k, id)) if k == "person" => Ok(Some(id)),
            _ => Err(Refusal::new(
                "not_found",
                "That person ticket is not one Books issued.",
            )),
        };
    }
    match text(args, "person_name")
        .map(str::trim)
        .filter(|n| !n.is_empty())
    {
        Some(name) => Ok(Some(people::person_for(conn, book, side_kind, name)?)),
        None => Ok(None),
    }
}

/// `books_app_person_create`.
pub fn person_create(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let book = writes::book_in(conn, args)?;
    let kind = person_kind(service::required(args, "kind")?).ok_or_else(|| {
        Refusal::invalid("`kind` is vendors, customers, employees, owners, lenders or government.")
    })?;
    let id = people::create_person(
        conn,
        &book,
        kind,
        service::required(args, "name")?,
        text(args, "note"),
    )?;
    Ok(with_subject(
        json!({ "person": plumbing::ticket_for(conn, "person", id)? }),
        "person",
        id,
    ))
}

/// `books_app_tag_create`.
pub fn tag_create(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let book = writes::book_in(conn, args)?;
    let id = people::create_tag(
        conn,
        &book,
        service::required(args, "kind")?,
        service::required(args, "name")?,
    )?;
    Ok(with_subject(
        json!({ "tag": plumbing::ticket_for(conn, "tag", id)? }),
        "tag",
        id,
    ))
}

/// `books_app_owed_create`: an invoice to collect or a bill to pay.
pub fn owed_create(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let book = writes::book_in(conn, args)?;
    let side = service::required(args, "side")?;
    let person = person_in(
        conn,
        &book,
        args,
        if side == "collect" {
            "customer"
        } else {
            "vendor"
        },
    )?;
    let amount = writes::amount(args, "amount")?;
    let category = writes::account_in(conn, &book, service::required(args, "category")?)?;
    let date = |k: &str| -> Result<Option<Date>, Refusal> {
        text(args, k)
            .map(|d| {
                Date::parse(d).ok_or_else(|| Refusal::invalid(format!("`{k}` is YYYY-MM-DD.")))
            })
            .transpose()
    };
    let issued = date("issued")?.unwrap_or_else(Date::today);
    let g = writes::gate(conn, &book);
    let id = people::create_unpaid(
        conn,
        &book,
        &people::UnpaidInput {
            side,
            person_id: person,
            reference: text(args, "reference"),
            memo: text(args, "memo"),
            issued,
            due: date("due")?,
            amount,
            category_account_id: category.id,
        },
        &g,
    )?;
    Ok(with_subject(
        json!({ "owed": plumbing::ticket_for(conn, "owed", id)?, "status": "open" }),
        "owed",
        id,
    ))
}

fn owed_in(conn: &Connection, args: &Map<String, Value>) -> Result<i64, Refusal> {
    match plumbing::subject(conn, service::required(args, "owed")?)? {
        Some((k, id)) if k == "owed" => Ok(id),
        _ => Err(Refusal::new(
            "not_found",
            "That document ticket is not one Books issued.",
        )),
    }
}

/// `books_app_owed_settle`: settle all of a document, or `amount` of it;
/// posts the money on that day.
pub fn owed_settle(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let book = writes::book_in(conn, args)?;
    let id = owed_in(conn, args)?;
    let money = writes::account_in(conn, &book, service::required(args, "account")?)?;
    let on = match text(args, "on").or_else(|| text(args, "date")) {
        Some(d) => Date::parse(d).ok_or_else(|| Refusal::invalid("`on` is YYYY-MM-DD."))?,
        None => Date::today(),
    };
    let amount = text(args, "amount")
        .filter(|t| !t.trim().is_empty())
        .map(|t| {
            money::parse(t)
                .map_err(|_| Refusal::new("bad_amount", "The amount is two-place decimal text."))
        })
        .transpose()?;
    let g = writes::gate(conn, &book);
    let done = owed_payments::settle(conn, &book, id, money.id, on, amount, &g)?;
    let e = entries::get(conn, done.entry_id)?
        .ok_or_else(|| Refusal::new("store", "The entry vanished."))?;
    Ok(with_subject(
        json!({
            "owed": service_ticket(conn, "owed", id)?,
            "status": if done.balance == 0 { "paid" } else { "part_paid" },
            "entry": service_ticket(conn, "entry", done.entry_id)?,
            "no": format!("{:04}", e.no),
            "balance": money::format(done.balance),
            "balance_shown": money::display(done.balance),
        }),
        "owed",
        id,
    ))
}

/// `books_app_owed_cancel`.
pub fn owed_cancel(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let book = writes::book_in(conn, args)?;
    let id = owed_in(conn, args)?;
    let g = writes::gate(conn, &book);
    people::cancel(conn, &book, id, &g)?;
    Ok(with_subject(
        json!({ "owed": service_ticket(conn, "owed", id)?, "status": "cancelled" }),
        "owed",
        id,
    ))
}

fn service_ticket(conn: &Connection, kind: &str, id: i64) -> Result<String, Refusal> {
    Ok(plumbing::ticket_for(conn, kind, id)?)
}

/// Totals for the Today page's Unpaid panel.
pub fn owed_summary(store: &Store, book: &Book) -> Result<Value, Refusal> {
    let collect = people::open_unpaid(store.conn(), book.id, "collect")?;
    let pay = people::open_unpaid(store.conn(), book.id, "pay")?;
    let tc: i64 = collect.iter().map(people::Unpaid::balance).sum();
    let tp: i64 = pay.iter().map(people::Unpaid::balance).sum();
    let next = collect.iter().chain(&pay).filter_map(|u| u.due).min();
    let scale = tc.max(tp);
    Ok(json!({
        "label": "Unpaid",
        "shown": money::display(tc),
        "rows": [
            { "label": format!("To collect · {}", collect.len()), "shown": money::display(tc), "pct": pct(tc, scale), "tone": "pos" },
            { "label": format!("To pay · {}", pay.len()), "shown": money::display(-tp), "pct": pct(tp, scale), "tone": "neg" },
            { "label": "Next due", "shown": next.map_or_else(|| "—".to_string(), Date::short), "pct": 0, "tone": "" },
        ],
        "collect": collect.len(), "pay": pay.len(),
        "signed": signed(tc - tp),
    }))
}
