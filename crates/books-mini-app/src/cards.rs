//! M8: Books in the Bot chat (drawings: Books Inline Cards, Books Guided
//! Chat). Four kinds of inline card, each its own app with only the tools
//! it calls:
//!
//! - `answer` (360): cash now, this month, a person, unpaid: amounts,
//!   because the user asked (owner decision);
//! - `report` (480): the P&L, this month against last;
//! - `action` (360): one click to finish: post a draft, categorize bank
//!   lines, mark paid, match the bank, close a month, a 2307 received;
//! - `pair` (480): a form's totals for the BIR app;
//! - `quiet` (360): cards a routine leaves; counts and status only.
//!
//! The assistant offers a card with `books_show` (its result carries
//! `_meta["buwiz/open"]`; the Bot shows it on the user's click). `/books <page or
//! card>` and the five templates (commands, since protocol 2.0 has no
//! template field) open them too. Six `@books:` scopes point at a person,
//! account, period, entry, form or tag; three routines draft at most.

use books_core::date::{Date, MONTH_NAMES, Period, PeriodKind};
use books_core::input::FormCode;
use books_core::ledger::{KindRef, Status};
use books_store::Store;
use books_store::books::{self, Book};
use books_store::entries::{self, Filter};
use books_store::handoffs;
use books_store::months;
use books_store::people;
use books_store::plumbing;
use books_store::rusqlite::Connection;
use serde_json::{Map, Value, json};

use crate::forms;
use crate::service::{Answer, Refusal, text};

/// `(card, app, title)`. Titles are neutral: no name, amount or TIN (D1).
pub const CARDS: [(&str, &str, &str); 15] = [
    ("cash", "answer", "Books · Cash now"),
    ("month", "answer", "Books · This month"),
    ("person", "answer", "Books · A person"),
    ("unpaid", "answer", "Books · Unpaid"),
    ("pl", "report", "Books · P&L"),
    ("draft", "action", "Books · Draft to post"),
    ("bank_lines", "action", "Books · Bank lines"),
    ("owed", "action", "Books · Mark paid"),
    ("reconcile", "action", "Books · Match the bank"),
    ("close", "action", "Books · Close the month"),
    ("cert", "action", "Books · 2307 received"),
    ("form", "pair", "Books · For eBIRForms"),
    ("tidy", "quiet", "Books · Weekly tidy-up"),
    ("month_end", "quiet", "Books · Month end"),
    ("handoff", "quiet", "Books · BIR hand-off"),
];

/// The cards an app draws (its props enum).
pub fn cards_of(app: &str) -> Vec<&'static str> {
    CARDS.iter().filter(|c| c.1 == app).map(|c| c.0).collect()
}

/// The props of a card, from a request: tickets and keys only.
const CARD_PROPS: [&str; 11] = [
    "person",
    "account",
    "entry",
    "owed",
    "certificate",
    "month",
    "period",
    "form",
    "side",
    "tax_option",
    "deduction",
];

/// `_meta["buwiz/open"]` for a card (spec 9.6): the Bot shows it on a click.
pub fn open(
    card: &str,
    args: &Map<String, Value>,
    ticket: Option<String>,
) -> Result<Value, Refusal> {
    let (_, app, title) = CARDS.iter().find(|c| c.0 == card).ok_or_else(|| {
        Refusal::invalid(format!(
            "`card` is one of {}.",
            CARDS.map(|c| c.0).join(", ")
        ))
    })?;
    let mut props = Map::new();
    props.insert("card".into(), json!(card));
    for k in CARD_PROPS {
        if let Some(v) = text(args, k) {
            props.insert(k.into(), json!(v));
        }
    }
    let mut out = json!({ "app": app, "props": props, "title": title, "placement": "inline" });
    if let Some(t) = ticket {
        out["ticket"] = json!(t);
    }
    Ok(out)
}

/// `books_show`: offer a card in the chat. The result says what it shows;
/// `_open` becomes the result's `_meta["buwiz/open"]`.
pub fn show(store: &Store, book: Book, args: &Map<String, Value>) -> Answer {
    let card = text(args, "card").ok_or_else(|| Refusal::invalid("Name the `card` to show."))?;
    let ticket = crate::service::ticket(store, "book", book.id)?;
    let open = open(card, args, Some(ticket))?;
    Ok(json!({ "card": card, "title": open["title"], "_open": open }))
}

// --- nudges: what a routine or a quiet card reads (counts only) -------------

/// Lines waiting for a category: posted on "not sure yet" and not moved,
/// and statement drafts to review.
fn to_review(c: &Connection, book: &Book) -> Result<Vec<i64>, Refusal> {
    let accounts: std::collections::HashMap<i64, books::Account> = books::accounts(c, book.id)?
        .into_iter()
        .map(|a| (a.id, a))
        .collect();
    let mut out = Vec::new();
    for e in entries::list(c, book.id, &Filter::default())? {
        let review = match e.status {
            Status::Draft => e.needs_review,
            Status::Posted => {
                e.reverses.is_none()
                    && e.recategorized_by.is_none()
                    && e.recategorizes.is_none()
                    && entries::lines(c, e.id)?.iter().any(|l| {
                        accounts
                            .get(&l.account_id)
                            .is_some_and(|a| a.kind == KindRef::Unsure)
                    })
            }
            _ => false,
        };
        if review {
            out.push(e.id);
        }
    }
    Ok(out)
}

/// The month-end checks for `month`: `(label, done)`.
pub fn checklist(
    c: &Connection,
    book: &Book,
    month: &Period,
) -> Result<Vec<(String, bool)>, Refusal> {
    let mut review = 0;
    let mut drafts = 0;
    for e in entries::list(
        c,
        book.id,
        &Filter {
            from: Some(month.start),
            to: Some(month.end),
            ..Filter::default()
        },
    )? {
        if e.status == Status::Draft {
            drafts += 1;
        }
    }
    for id in to_review(c, book)? {
        if entries::get(c, id)?.is_some_and(|e| month.contains(e.date)) {
            review += 1;
        }
    }
    let statements = months::statements(c, book.id)?;
    let mut matched = true;
    for a in books::accounts(c, book.id)?
        .into_iter()
        .filter(|a| a.active && a.is_cash())
    {
        let used = !months::candidates(c, book.id, a.id, month)?.is_empty();
        let done = statements
            .iter()
            .any(|s| s.account_id == a.id && s.period_key == month.key && s.status == "finished");
        if used && !done {
            matched = false;
        }
    }
    let name = MONTH_NAMES[usize::from(month.start.month - 1)];
    Ok(vec![
        ("Bank lines reviewed".into(), review == 0),
        (format!("Bank matched for {name}"), matched),
        ("No drafts left".into(), drafts == 0),
    ])
}

/// `books_nudge`: what a routine looks for, and what its quiet card shows.
/// `text` is empty when there is nothing to do (no turn runs); each item
/// has an id the Bot remembers, so a routine fires once per thing.
pub fn nudge(store: &Store, book: Book, args: &Map<String, Value>) -> Answer {
    let c = store.conn();
    let today = Date::today();
    match text(args, "kind").unwrap_or("tidy") {
        "tidy" => {
            let lines = to_review(c, &book)?;
            let drafts: usize = entries::list(c, book.id, &Filter::default())?
                .iter()
                .filter(|e| e.status == Status::Draft)
                .count();
            let n = lines.len();
            // the routine's week starts Monday; asked any day (a card), it answers
            let due = today.weekday() == 0 || args.get("any_day") == Some(&Value::Bool(true));

            Ok(json!({
                "kind": "tidy",
                "text": if n == 0 || !due { String::new() } else { format!("{n} bank line{} to categorize", if n == 1 { "" } else { "s" }) },
                "items": if due { lines.iter().map(|id| json!({ "id": format!("review-e{id}") })).collect::<Vec<_>>() } else { Vec::new() },
                "counts": { "lines": n, "drafts": drafts, "posted": 0 },
            }))
        }
        "month_end" => {
            let month = match text(args, "month") {
                Some(m) => Period::parse(m)
                    .filter(|p| p.kind == PeriodKind::Month)
                    .ok_or_else(|| Refusal::invalid("`month` is YYYY-MM."))?,
                None => Period::of(today, PeriodKind::Month).previous(),
            };
            let closed = months::is_closed(c, book.id, &month.key)?;
            let checks = checklist(c, &book, &month)?;
            let left = checks.iter().filter(|(_, done)| !done).count();
            let name = MONTH_NAMES[usize::from(month.start.month - 1)];
            // the nudge fires in the first week of the next month, once
            let due = !closed && month.end < today && today.day <= 7;
            Ok(json!({
                "kind": "month_end",
                "month": month.key,
                "month_label": month.label(),
                "closed": closed,
                "text": if due { format!("{name} is still open. {left} step{} left before closing.", if left == 1 { "" } else { "s" }) } else { String::new() },
                "items": if due { vec![json!({ "id": format!("close-{}", month.key) })] } else { Vec::new() },
                "checks": checks.iter().map(|(l, d)| json!({ "label": l, "done": d })).collect::<Vec<_>>(),
                "left": left,
            }))
        }
        "bir_pair" => {
            let mut stale = Vec::new();
            for h in handoffs::list(c, book.id)? {
                let Some(form) = FormCode::parse(&h.form) else {
                    continue;
                };
                let ctx: Value =
                    serde_json::from_str(&h.form_context).unwrap_or_else(|_| json!({}));
                let now = forms::build(c, &book, form, &h.period_key, &ctx)?;
                if let Some(why) = crate::handoff::changed(c, &book, &h, &now)? {
                    stale.push((h, now.proposal.inputs_hash.clone(), why));
                }
            }
            let n = stale.len();
            Ok(json!({
                "kind": "bir_pair",
                "text": if n == 0 { String::new() } else { format!("{n} BIR hand-off{} out of date", if n == 1 { "" } else { "s" }) },
                "items": stale.iter().map(|(h, hash, _)| json!({ "id": format!("stale-{}-{}", h.id, &hash[..12.min(hash.len())]) })).collect::<Vec<_>>(),
                "forms": stale.iter().map(|(h, _, why)| json!({ "title": crate::handoff::title(&h.form, &h.period_key), "form": h.form, "period": h.period_key, "why": why })).collect::<Vec<_>>(),
                "count": n,
            }))
        }
        "review" => {
            // the routine runs daily and speaks on Mondays; a card asks any day
            let due = today.weekday() == 0 || args.get("any_day") == Some(&Value::Bool(true));
            let all = if due {
                crate::findings::findings(store, book, today)?
            } else {
                Vec::new()
            };
            let n = all.len();
            Ok(json!({
                "kind": "review",
                "text": if n == 0 || !due { String::new() } else { format!("{n} Books finding{} to look at", if n == 1 { "" } else { "s" }) },
                "items": if due && n > 0 { vec![json!({ "id": format!("review-{today}-{n}") })] } else { Vec::new() },
                "counts": crate::findings::counts(&all),
                "count": n,
                "open": { "page": "ledger", "filter": "checks" },
            }))
        }
        _ => Err(Refusal::invalid(
            "`kind` is tidy, month_end, bir_pair or review.",
        )),
    }
}

// --- scopes ------------------------------------------------------------------

/// The `@books:` scopes, and the hint each carries for the assistant.
pub fn scopes() -> Value {
    let hint = |what: &str| {
        format!(
            "The user points at {what} in Books. Read it with Books' read tools (pass the ticket as given). You may draft an entry with books_entry_draft; you never post, void, close or hand over: those are the user's clicks in Books. Show amounts only when the user asked."
        )
    };
    let scope = |name: &str, description: &str, what: &str| json!({ "name": name, "description": description, "resolve": crate::tools::RESOLVE, "attach": true, "hint": hint(what) });
    json!([
        scope("person", "person in Books: a vendor, customer, employee or other party", "a person (books_person_get)"),
        scope("account", "account in the Books chart", "an account (books_ledger_list with account)"),
        scope("period", "period: a month, quarter or year", "a period (books_report, books_today)"),
        scope("entry", "entry in the Books ledger", "one entry (books_entry_get)"),
        scope("form", "form and period Books feeds for eBIRForms", "a BIR form and period (books_form_proposal)"),
        scope("tag", "tag: a department or location", "a department or location (books_tags_list)"),
        {
            "name": "proposal",
            "description": "proposal of Books totals for a BIR form",
            "resolve": crate::tools::RESOLVE,
            "attach": true,
            "hint": "A Books proposal holds the input boxes of one BIR form. Read it with books_handoff_get. Open that form's BIR draft, fill the boxes once with source \"document\" (never \"user\"), read them back and compare each value exactly, then validate and save the draft. Never send or file. If Books says it is out of date, do not fill it."
        }
    ])
}

/// A period as a ticket subject id: `202610` a month, `20263` a quarter,
/// `2026` a year.
fn period_id(p: &Period) -> i64 {
    let y = i64::from(p.start.year);
    match p.kind {
        PeriodKind::Month => y * 100 + i64::from(p.start.month),
        PeriodKind::Quarter => y * 10 + i64::from(p.start.quarter()),
        PeriodKind::Year => y,
    }
}

fn period_of_id(id: i64) -> Option<Period> {
    let key = match id {
        100_000..=999_999 => format!("{:04}-{:02}", id / 100, id % 100),
        10_000..=99_999 => format!("{:04}-Q{}", id / 10, id % 10),
        _ => format!("{id:04}"),
    };
    Period::parse(&key)
}

const FORMS: [FormCode; 3] = [FormCode::F2551Q, FormCode::F1601C, FormCode::F1701MS];

fn form_id(form: FormCode, p: &Period) -> i64 {
    let n = FORMS
        .iter()
        .position(|f| *f == form)
        .map_or(1, |i| i as i64 + 1);
    n * 1_000_000 + period_id(p)
}

/// A `form` ticket's form and period.
pub fn form_of_id(id: i64) -> Option<(FormCode, Period)> {
    let form = *FORMS.get(usize::try_from(id / 1_000_000 - 1).ok()?)?;
    Some((form, period_of_id(id % 1_000_000)?))
}

/// The title of a `period` or `form` ticket.
pub fn title(kind: &str, id: i64) -> Option<String> {
    match kind {
        "period" => period_of_id(id).map(|p| p.label()),
        "form" => form_of_id(id).map(|(f, p)| format!("{} {}", code(f), p.label())),
        _ => None,
    }
}

fn code(f: FormCode) -> &'static str {
    match f {
        FormCode::F2551Q => "2551Q",
        FormCode::F1601C => "1601-C",
        FormCode::F1701MS => "1701-MS",
    }
}

/// Replace `period`, `month` and `form` tickets in a request with the keys
/// the tools read (a scope token the assistant passes as it got it).
pub fn untick(c: &Connection, args: &Map<String, Value>) -> Map<String, Value> {
    let mut out = args.clone();
    for key in ["period", "month", "form"] {
        let Some(t) = text(args, key).filter(|t| plumbing::is_ticket(t)) else {
            continue;
        };
        match plumbing::subject(c, t).ok().flatten() {
            Some((k, id)) if k == "period" => {
                if let Some(p) = period_of_id(id) {
                    out.insert(key.into(), json!(p.key));
                }
            }
            Some((k, id)) if k == "form" => {
                if let Some((f, p)) = form_of_id(id) {
                    out.insert("form".into(), json!(f.as_str()));
                    out.insert("period".into(), json!(p.key));
                }
            }
            _ => {}
        }
    }
    out
}

fn person_label(kind: &str) -> &'static str {
    match kind {
        "vendor" => "Vendor",
        "customer" => "Customer",
        "employee" => "Employee",
        "owner" => "Owner",
        "lender" => "Lender",
        _ => "Government",
    }
}

fn matches(label: &str, typed: &str) -> bool {
    typed.is_empty() || label.to_lowercase().contains(&typed.to_lowercase())
}

/// Resolve an `@books:` scope (spec 9.2): a ticket gives its token; text
/// gives up to 20 choices whose values are tickets.
pub fn scope(store: &Store, name: &str, value: &str) -> Value {
    let c = store.conn();
    if plumbing::is_ticket(value) {
        let kind_ok = match plumbing::subject(c, value).ok().flatten() {
            Some((k, _)) => k == name || (name == "tag" && k == "tag"),
            None => false,
        };
        return match (kind_ok, crate::service::title_of(store, value)) {
            (true, Some(title)) => json!({ "result": "token", "ticket": value, "title": title }),
            _ => json!({ "result": "error", "message": format!("That is not a Books {name}.") }),
        };
    }
    let Ok(all) = books::all(c) else {
        return json!({ "result": "error", "message": "Books could not open right now." });
    };
    let Some(book) = all.into_iter().next() else {
        return json!({ "result": "error", "message": "There is no book yet." });
    };
    let choices = choices(store, &book, name, value.trim()).unwrap_or_default();
    if choices.is_empty() {
        return json!({ "result": "error", "message": format!("No {name} in Books matches that.") });
    }
    json!({ "result": "choices", "choices": choices })
}

fn choices(store: &Store, book: &Book, name: &str, typed: &str) -> Result<Vec<Value>, Refusal> {
    let c = store.conn();
    let t = |kind: &str, id: i64| crate::service::ticket(store, kind, id);
    let mut out = Vec::new();
    match name {
        "person" => {
            for p in people::people(c, book.id, None)?
                .into_iter()
                .filter(|p| p.active && matches(&p.name, typed))
            {
                out.push(json!({ "label": p.name, "detail": person_label(&p.kind), "value": t("person", p.id)? }));
            }
        }
        "account" => {
            for a in books::accounts(c, book.id)?
                .into_iter()
                .filter(|a| a.active)
            {
                let label = format!("{} {}", a.number, a.name);
                if matches(&label, typed) {
                    out.push(json!({ "label": label, "detail": a.account_type.group(), "value": t("account", a.id)? }));
                }
            }
        }
        "period" => {
            let today = Date::today();
            let m = Period::of(today, PeriodKind::Month);
            let q = Period::of(today, PeriodKind::Quarter);
            let y = Period::of(today, PeriodKind::Year);
            for (p, detail) in [
                (m.clone(), "this month"),
                (m.previous(), "last month"),
                (q.clone(), "this quarter"),
                (q.previous(), "last quarter"),
                (y.clone(), "this year"),
                (y.previous(), "last year"),
            ] {
                if matches(&p.label(), typed) || matches(detail, typed) {
                    out.push(json!({ "label": p.label(), "detail": detail, "value": t("period", period_id(&p))? }));
                }
            }
        }
        "entry" => {
            let mut list = entries::list(c, book.id, &Filter::default())?;
            list.retain(|e| e.status != Status::Discarded);
            list.sort_by(|a, b| b.date.cmp(&a.date).then(b.no.cmp(&a.no)));
            for e in list {
                let what = e
                    .payee
                    .clone()
                    .unwrap_or_else(|| format!("Entry {:04}", e.no));
                let label = format!("{} · {what}", e.date.short());
                if matches(&label, typed) {
                    out.push(json!({ "label": label, "detail": format!("Entry {:04}", e.no), "value": t("entry", e.id)? }));
                }
                if out.len() >= 20 {
                    break;
                }
            }
        }
        "form" => {
            for f in FORMS {
                let p = Period::parse(&forms::default_period(f))
                    .ok_or_else(|| Refusal::new("internal", "bad period"))?;
                for p in [p.clone(), p.previous()] {
                    let label = format!("{} {}", code(f), p.label());
                    if matches(&label, typed) {
                        out.push(json!({ "label": label, "detail": "Books totals for eBIRForms", "value": t("form", form_id(f, &p))? }));
                    }
                }
            }
        }
        "tag" => {
            for tag in people::tags(c, book.id, None)?
                .into_iter()
                .filter(|x| matches(&x.name, typed))
            {
                out.push(json!({ "label": tag.name, "detail": if tag.kind == "department" { "Department" } else { "Location" }, "value": t("tag", tag.id)? }));
            }
        }
        _ => {}
    }
    out.truncate(20);
    Ok(out)
}

// --- routines ----------------------------------------------------------------

/// The four routines (docs/08 §3; review is M10 parity). Only the weekly tidy-up writes, and only
/// drafts.
pub fn routines() -> Value {
    json!([
        {
            "id": "weekly-tidy",
            "name": "Weekly tidy-up",
            "description": "Reads new bank lines and drafts a category for each. Posts nothing.",
            // every_s is at most a day: it runs daily and finds work on Mondays
            "every_s": 86_400,
            "query": { "tool": crate::tools::NUDGE, "field": "text", "args": { "kind": "tidy" } },
            "prompt": "Books has bank lines waiting for a category. For each one, read it with books_entry_get and draft the likely category with books_entry_draft (a journal that moves the amount from the not-sure-yet account to the category). Never post: the owner posts in Books. Then show the quiet card with books_show {\"card\": \"tidy\"}. Say counts only, never amounts.",
            "writes": [crate::tools::ENTRY_DRAFT]
        },
        {
            "id": "month-end",
            "name": "Month-end nudge",
            "description": "Checks what is left before closing last month and leaves a quiet card. Changes nothing.",
            "every_s": 86_400,
            "query": { "tool": crate::tools::NUDGE, "field": "text", "args": { "kind": "month_end" } },
            "prompt": "Last month is still open in Books. Show the quiet card with books_show {\"card\": \"month_end\"} and say how many steps are left. Do not close the month: closing is the owner's click in Books. Say counts only, never amounts."
        },
        {
            "id": "bir-pair",
            "name": "BIR pair check",
            "description": "Flags BIR hand-offs that went out of date after the books changed. Changes nothing.",
            "every_s": 86_400,
            "query": { "tool": crate::tools::NUDGE, "field": "text", "args": { "kind": "bir_pair" } },
            "prompt": "A Books hand-off to the BIR draft is out of date. Show the quiet card with books_show {\"card\": \"handoff\"}. Do not fill the BIR draft from it; the owner refreshes it on the Forms page. Say counts only, never amounts."
        },
        {
            "id": "review",
            "name": "Books review",
            "description": "Looks over the ledger for findings to check twice (stale drafts, unmatched months, weekend dates, round amounts) and sums up the counts in chat. Changes nothing.",
            // every_s is at most a day: it runs daily and speaks on Mondays
            "every_s": 86_400,
            "query": { "tool": crate::tools::NUDGE, "field": "text", "args": { "kind": "review" } },
            "prompt": "Books has review findings. Call books_checks and summarise the counts per check only. Change nothing: the owner fixes entries in Books. Say counts only, never amounts, names or memos."
        }
    ])
}

/// The writes a routine declares (the broker checks too; Books checks
/// again, P6). Books lets a routine write drafts only.
pub fn routine_writes(routine: &str) -> Vec<String> {
    routines()
        .as_array()
        .into_iter()
        .flatten()
        .filter(|r| r["id"] == routine)
        .flat_map(|r| r["writes"].as_array().cloned().unwrap_or_default())
        .filter_map(|w| w.as_str().map(str::to_string))
        .filter(|w| w == crate::tools::ENTRY_DRAFT)
        .collect()
}

// --- /books and the templates -------------------------------------------------

/// The pages `/books <page>` opens.
const PAGES: [&str; 12] = [
    "today",
    "ledger",
    "unpaid",
    "reconcile",
    "reports",
    "forms",
    "people",
    "accounts",
    "tags",
    "payroll",
    "certificates",
    "settings",
];

/// What `/books <value>` or a template command opens: a page of the Books
/// window, or an inline card. `None`: the Books window at Today.
pub fn command_open(command: &str, value: &str) -> Option<Value> {
    let empty = Map::new();
    let card = |card: &str, extra: &[(&str, &str)]| {
        let mut args = empty.clone();
        for (k, v) in extra {
            args.insert((*k).into(), json!(v));
        }
        open(card, &args, None).ok()
    };
    let today = Date::today();
    let last_month = Period::of(today, PeriodKind::Month).previous();
    match command {
        "books-receipt" => card("draft", &[]),
        "books-tidy" => card("bank_lines", &[]),
        "books-close" => card("close", &[("month", &last_month.key)]),
        "books-quarter" => card("form", &[("form", "2551Q")]),
        "books-results" => card("pl", &[]),
        _ => {
            let v = value.trim().to_lowercase();
            if v.is_empty() {
                return None;
            }
            if PAGES.contains(&v.as_str()) {
                return Some(
                    json!({ "app": "books", "props": { "page": v }, "title": "Books", "placement": "full" }),
                );
            }
            let alias = match v.as_str() {
                "p&l" | "pnl" | "results" => "pl",
                "this month" | "month" => "month",
                "bank" | "bank lines" | "categorize" => "bank_lines",
                "match" | "bank match" => "reconcile",
                "paid" | "mark paid" => "owed",
                "collect" | "to collect" | "who owes" => "unpaid",
                "receipt" => "draft",
                "close" => "close",
                "2307" => "cert",
                "form" | "bir" | "quarter" => "form",
                "month end" | "month-end" => "month_end",
                "hand-off" | "handoff" => "handoff",
                other => other,
            };
            CARDS
                .iter()
                .any(|c| c.0 == alias)
                .then(|| card(alias, &[]))
                .flatten()
        }
    }
}

/// The `/books` choices: the pages (values are tickets, spec 9.2).
pub fn page_choices(store: &Store, typed: &str) -> Result<Vec<Value>, Refusal> {
    let detail = |p: &str| match p {
        "today" => "cash, month, what needs you",
        "ledger" => "transactions by account",
        "unpaid" => "to collect, to pay",
        "reconcile" => "match the bank",
        "forms" => "totals for eBIRForms",
        "reports" => "P&L, balance sheet, cash flow",
        _ => "",
    };
    let mut out = Vec::new();
    for (i, p) in PAGES.iter().enumerate() {
        if matches(p, typed) {
            let label = format!("{}{}", p[..1].to_uppercase(), &p[1..]);
            out.push(json!({ "label": label, "detail": detail(p), "value": crate::service::ticket(store, "page", i64::try_from(i).unwrap_or(0))? }));
        }
    }
    Ok(out)
}

/// A `page` ticket's page.
pub fn page_of(id: i64) -> Option<&'static str> {
    PAGES.get(usize::try_from(id).ok()?).copied()
}
