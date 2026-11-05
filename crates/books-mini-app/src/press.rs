//! The AI's press on a person-only tool, as Books takes it (invariant 4).
//!
//! The Bot holds the call unsent and shows the person what it will do, in
//! the words of [`card`], and only a person's own press of Allow lets it go.
//! The record it then carries is the Bot's, under its signature, and Books
//! opts in tool by tool: [`APPROVED`]. Everything else a person does stays a
//! person's click. A record is good here only whole, and only for the
//! arguments that came with it ([`Approved::of`]).
//!
//! [`card`] reads and changes nothing. It answers the Bot alone, for the
//! person's eyes, so unlike a reply to the model it carries amounts and
//! account names; the Bot keeps it out of logs and away from the model.

use books_core::date::{Period, PeriodKind};
use books_core::ledger::Status;
use books_core::money;
use books_store::Store;
use books_store::books::Book;
use books_store::entries::{self, LineInput};
use serde_json::{Map, Value, json};

use crate::jcs;
use crate::service::{self, Answer, Refusal, text};
use crate::tools::{BATCH, ENTRY_DRAFT, MONTH_CLOSE, POST_ENTRY};
use crate::views::Chart;
use crate::writes;

/// The person-only tools the AI may press once a person allows that press:
/// Post and Close month, the owner's D1, and a batch that only posts. Void,
/// reopen, return confirm and hand-off are D1's to name, and it doesn't.
pub const APPROVED: [&str; 3] = [POST_ENTRY, MONTH_CLOSE, BATCH];

/// Whether an app reaching `tools` can have a call held, so needs its card.
pub fn needs_card(tools: &[String]) -> bool {
    tools
        .iter()
        .any(|t| APPROVED.contains(&t.as_str()) || t == ENTRY_DRAFT)
}

/// What a batch may do under an approved press.
const BATCH_POST: &str = "post";
/// How many entries of a batch a card names; it counts them all.
const MAX_NAMED: usize = 6;

/// A person's yes to the AI's press, as the Bot recorded it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Approved {
    pub who: String,
    pub approved_by: String,
    pub once: String,
    pub args_hash: String,
    pub at: i64,
}

impl Approved {
    /// The record in `press`, if it is whole and names an agent, a person
    /// and `args`, what came with it.
    pub fn of(press: Option<&Value>, args: &Map<String, Value>) -> Option<Approved> {
        let press = press?;
        let field = |key: &str| press.get(key).and_then(Value::as_str).map(str::to_string);
        if field("by")? != "ai_approved" {
            return None;
        }
        let record = Approved {
            who: field("who").filter(|w| w.starts_with("agent."))?,
            approved_by: field("approved_by").filter(|p| p.starts_with("person."))?,
            once: field("once")?,
            args_hash: field("args_hash")?,
            at: press.get("at").and_then(Value::as_i64)?,
        };
        field("shown_hash")?;
        (record.args_hash == args_hash(args)?).then_some(record)
    }

    /// Whether this yes may run `tool` with `args`.
    pub fn covers(&self, tool: &str, args: &Map<String, Value>) -> bool {
        APPROVED.contains(&tool) && (tool != BATCH || text(args, "action") == Some(BATCH_POST))
    }
}

/// `sha256:` over the arguments' JCS bytes, as the Bot computes it.
fn args_hash(args: &Map<String, Value>) -> Option<String> {
    let hex = jcs::sha256_jcs(&Value::Object(args.clone())).ok()?;
    Some(format!("sha256:{hex}"))
}

fn inner(args: &Map<String, Value>) -> Result<&Map<String, Value>, Refusal> {
    args.get("args")
        .and_then(Value::as_object)
        .ok_or_else(|| Refusal::invalid("`args` is the held call's arguments."))
}

/// The card's arguments with the held call's book: a card reads where the
/// call would write.
pub fn scoped(args: &Map<String, Value>) -> Map<String, Value> {
    let mut out = args.clone();
    if let Some(book) = args.get("args").and_then(|a| a.get("book")) {
        out.insert("book".into(), book.clone());
    }
    out
}

/// `books_app_press_card`: what the held call `tool` with `args` will do, in
/// plain words with its real values.
pub fn card(store: &Store, book: Book, args: &Map<String, Value>) -> Answer {
    let tool = service::required(args, "tool")?;
    let call = inner(args)?;
    let chart = Chart::load(store, book)?;
    match tool {
        POST_ENTRY => post_card(store, &chart, call),
        ENTRY_DRAFT => draft_card(store, &chart, call),
        MONTH_CLOSE => close_card(store, &chart, call),
        BATCH => batch_card(store, &chart, call),
        _ => Err(Refusal::new(
            "permission_denied",
            "Books has no card for that tool.",
        )),
    }
}

fn peso(centavos: i64) -> String {
    format!("₱{}", money::display(centavos))
}

/// An account as a card names it.
fn account(chart: &Chart, id: i64) -> String {
    chart.by_id.get(&id).map_or_else(
        || "An account".to_string(),
        |a| format!("{} {}", a.number, a.name),
    )
}

/// One line of an entry for the card: its account and its side.
fn side(chart: &Chart, id: i64, debit: i64, credit: i64) -> String {
    if debit > 0 {
        format!("{}: debit {}", account(chart, id), peso(debit))
    } else {
        format!("{}: credit {}", account(chart, id), peso(credit))
    }
}

/// An entry's lines, and their total, whichever way they came.
fn lines_of(chart: &Chart, rows: &[(i64, i64, i64)]) -> (Vec<String>, i64) {
    let total = rows.iter().map(|(_, debit, _)| *debit).sum();
    let lines = rows
        .iter()
        .map(|(id, debit, credit)| side(chart, *id, *debit, *credit))
        .collect();
    (lines, total)
}

fn new_rows(lines: &[LineInput]) -> Vec<(i64, i64, i64)> {
    lines
        .iter()
        .map(|l| (l.account_id, l.debit, l.credit))
        .collect()
}

/// What an entry that doesn't exist yet comes to, from its arguments.
fn from_input(
    store: &Store,
    chart: &Chart,
    call: &Map<String, Value>,
) -> Result<(String, Option<String>, Vec<String>, i64), Refusal> {
    let input = writes::draft_input(store.conn(), &chart.book, call)?;
    let (lines, total) = lines_of(chart, &new_rows(&input.lines));
    Ok((input.date.to_string(), input.payee, lines, total))
}

fn post_card(store: &Store, chart: &Chart, call: &Map<String, Value>) -> Answer {
    let (title, date, payee, mut lines) = match text(call, "entry") {
        Some(ticket) => {
            let e = writes::entry_in(store.conn(), &chart.book, ticket)?;
            let rows: Vec<(i64, i64, i64)> = entries::lines(store.conn(), e.id)?
                .iter()
                .map(|l| (l.account_id, l.debit, l.credit))
                .collect();
            let (lines, total) = lines_of(chart, &rows);
            let title = format!("Post entry {:04} for {}?", e.no, peso(total));
            (title, e.date.to_string(), e.payee, lines)
        }
        None => {
            let (date, payee, lines, total) = from_input(store, chart, call)?;
            let title = format!("Save and post this entry for {}?", peso(total));
            (title, date, payee, lines)
        }
    };
    lines.insert(0, format!("Date: {date}"));
    if let Some(payee) = payee.filter(|p| !p.is_empty()) {
        lines.insert(1, format!("With: {payee}"));
    }
    if call.get("keep_both").and_then(Value::as_bool) == Some(true) {
        lines.push("It looks like another entry, and both stay.".to_string());
    }
    Ok(json!({ "title": title, "lines": lines }))
}

fn draft_card(store: &Store, chart: &Chart, call: &Map<String, Value>) -> Answer {
    let (date, payee, mut lines, total) = from_input(store, chart, call)?;
    lines.insert(0, format!("Date: {date}"));
    if let Some(payee) = payee.filter(|p| !p.is_empty()) {
        lines.insert(1, format!("With: {payee}"));
    }
    lines.push("A draft counts in no total until a person posts it.".to_string());
    let title = format!("Save a draft for {}?", peso(total));
    Ok(json!({ "title": title, "lines": lines }))
}

fn close_card(store: &Store, chart: &Chart, call: &Map<String, Value>) -> Answer {
    let period = Period::parse(service::required(call, "month")?)
        .filter(|p| p.kind == PeriodKind::Month)
        .ok_or_else(|| Refusal::invalid("`month` is YYYY-MM."))?;
    let mut posted = 0_i64;
    let mut drafts = 0_i64;
    let filter = entries::Filter {
        from: Some(period.start),
        to: Some(period.end),
        account_id: None,
        discarded: false,
    };
    for e in entries::list(store.conn(), chart.book.id, &filter)? {
        match e.status {
            Status::Draft => drafts += 1,
            Status::Posted => posted += 1,
            _ => {}
        }
    }
    let mut lines = vec![
        format!("Posted entries in it: {posted}"),
        format!("Drafts still in it: {drafts}"),
        "A closed month refuses every post and void dated in it.".to_string(),
    ];
    if let Some(reason) = text(call, "reason").filter(|r| !r.is_empty()) {
        lines.push(format!("Reason: {reason}"));
    }
    Ok(json!({ "title": format!("Close {}?", period.label()), "lines": lines }))
}

fn batch_card(store: &Store, chart: &Chart, call: &Map<String, Value>) -> Answer {
    if text(call, "action") != Some(BATCH_POST) {
        return Err(Refusal::new(
            "permission_denied",
            "Only posting a batch can wait for your OK.",
        ));
    }
    let list = call
        .get("entries")
        .and_then(Value::as_array)
        .ok_or_else(|| Refusal::invalid("`entries` is a list of entry tickets."))?;
    let (mut total, mut named) = (0_i64, Vec::new());
    for ticket in list.iter().filter_map(Value::as_str) {
        let e = writes::entry_in(store.conn(), &chart.book, ticket)?;
        let amount: i64 = entries::lines(store.conn(), e.id)?
            .iter()
            .map(|l| l.debit)
            .sum();
        total += amount;
        if named.len() < MAX_NAMED {
            named.push(format!("Entry {:04}, {}: {}", e.no, e.date, peso(amount)));
        }
    }
    if list.len() > MAX_NAMED {
        named.push(format!("and {} more", list.len() - MAX_NAMED));
    }
    let plural = if list.len() == 1 { "entry" } else { "entries" };
    let title = format!("Post {} {plural} for {} in all?", list.len(), peso(total));
    Ok(json!({ "title": title, "lines": named }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(args: &Map<String, Value>) -> Value {
        json!({ "by": "ai_approved", "who": "agent.broker", "approved_by": "person.owner", "once": "press-0001",
            "args_hash": args_hash(args).unwrap(), "shown_hash": format!("sha256:{}", "a1".repeat(32)), "at": 1_793_745_245_000_i64 })
    }

    fn args() -> Map<String, Value> {
        json!({ "month": "2026-10", "client_request_id": "close-1" })
            .as_object()
            .unwrap()
            .clone()
    }

    #[test]
    fn the_args_hash_is_the_one_the_bot_computes() {
        assert_eq!(
            args_hash(&args()).unwrap(),
            "sha256:cbf98bde1bdab782a73d98e6aa94663ac6786ef8d0e054ec61dd0cbb12a11830"
        );
    }

    #[test]
    fn a_whole_record_for_these_arguments_is_a_yes() {
        let got = Approved::of(Some(&record(&args())), &args()).unwrap();
        assert_eq!(
            (got.who.as_str(), got.approved_by.as_str()),
            ("agent.broker", "person.owner")
        );
    }

    #[test]
    fn a_record_for_other_arguments_or_with_a_hole_is_no_yes() {
        let mut other = args();
        other.insert("month".into(), json!("2026-09"));
        assert_eq!(Approved::of(Some(&record(&args())), &other), None);
        assert_eq!(Approved::of(None, &args()), None);
        for (key, value) in [
            ("by", json!("ai")),
            ("who", json!("person.owner")),
            ("approved_by", json!("agent.broker")),
            ("once", Value::Null),
            ("at", json!("now")),
            ("shown_hash", Value::Null),
        ] {
            let mut bad = record(&args());
            bad[key] = value;
            assert_eq!(Approved::of(Some(&bad), &args()), None, "{key}");
        }
    }

    #[test]
    fn a_yes_covers_post_close_and_a_batch_that_only_posts() {
        let yes = Approved::of(Some(&record(&args())), &args()).unwrap();
        assert!(yes.covers(POST_ENTRY, &args()) && yes.covers(MONTH_CLOSE, &args()));
        assert!(!yes.covers(BATCH, &args()), "a batch that says nothing");
        let post = json!({ "action": "post" }).as_object().unwrap().clone();
        let discard = json!({ "action": "discard" }).as_object().unwrap().clone();
        assert!(yes.covers(BATCH, &post) && !yes.covers(BATCH, &discard));
        for tool in [
            "books_app_void_entry",
            "books_app_month_reopen",
            "books_app_return_ok",
            "books_app_handoff_create",
            "books_app_handoff_confirm",
        ] {
            assert!(!yes.covers(tool, &args()), "{tool}");
        }
    }
}
