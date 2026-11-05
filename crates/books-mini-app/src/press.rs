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

use std::collections::HashSet;

use books_core::date::{Period, PeriodKind};
use books_core::ledger::Status;
use books_core::money;
use books_store::Store;
use books_store::books::Book;
use books_store::entries::{self, LineInput};
use books_store::{people, plumbing};
use serde_json::{Map, Value, json};

use crate::batch::MAX_ENTRIES;
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

/// What a press on a screen can't put in a draft, at any switch: overwriting
/// a draft (`entry`), copying an entry (`copy_of`) and what a reading of a
/// receipt adds (`extracted`, `extraction`) change what is saved in ways a card
/// can't show in a line. They are a person's, or the assistant's direct call.
pub const DRAFT_REFUSED: [&str; 4] = ["entry", "copy_of", "extracted", "extraction"];

/// Whether a press on a screen may save this draft.
pub fn draft_pressable(args: &Map<String, Value>) -> bool {
    !DRAFT_REFUSED.iter().any(|k| args.contains_key(*k))
}

/// What a batch may do under an approved press.
const BATCH_POST: &str = "post";
/// How old a yes may be when its call arrives: the Bot sends it straight
/// after the press, so a minute is a queue, and anything more a replay.
const MAX_AGE_MS: i64 = 60_000;
/// How far ahead of this clock a yes may be: the same Mac, so next to nothing.
const MAX_AHEAD_MS: i64 = 5_000;
/// What a plain warning says of `keep_both`.
const DUPLICATES: &str = "Posts duplicates: it looks like another entry, and both stay.";
/// What a card says of `keep_both` on a batch.
const BATCH_DUPLICATES: &str = "Posts duplicates: entries that look like others post anyway.";

/// Every argument a card may be asked about, by the tool's shape. A card
/// accounts for all of them or refuses, so a press can't carry a value the
/// person didn't read: `client_request_id` changes nothing that posts, and
/// each of the others is on the card.
pub const POST_ENTRY_ARGS: [&str; 4] = ["client_request_id", "book", "entry", "keep_both"];
pub const POST_NEW_ARGS: [&str; 17] = [
    "client_request_id",
    "book",
    "form",
    "date",
    "amount",
    "account",
    "to_account",
    "category",
    "payee",
    "memo",
    "reference",
    "facts",
    "lines",
    "person",
    "person_kind",
    "department",
    "location",
];
pub const CLOSE_ARGS: [&str; 4] = ["client_request_id", "book", "month", "reason"];
pub const BATCH_ARGS: [&str; 5] = [
    "client_request_id",
    "book",
    "action",
    "entries",
    "keep_both",
];
/// What a journal line may hold.
const LINE_ARGS: [&str; 4] = ["account", "debit", "credit", "facts"];

/// A person's yes to the AI's press, as the Bot recorded it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Approved {
    pub who: String,
    pub approved_by: String,
    pub once: String,
    pub args_hash: String,
    pub shown_hash: String,
    pub at: i64,
}

impl Approved {
    /// The record in `press`, if it is whole, names an agent, a person and
    /// `args`, what came with it, and is fresh at `now_ms`.
    ///
    /// Books holds no key, so this can't tell a record the Bot made from one
    /// made by whatever can write to Books' stdin: that is the broker's
    /// signature check, and Books trusts `origin` the same way. What it does
    /// stop is a record kept and sent again: for other arguments, later, or
    /// a second time ([`crate::service::write`] spends `once`).
    pub fn of(press: Option<&Value>, args: &Map<String, Value>, now_ms: i64) -> Option<Approved> {
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
            shown_hash: field("shown_hash").filter(|h| is_hash(h))?,
            at: press.get("at").and_then(Value::as_i64)?,
        };
        let fresh = (now_ms - MAX_AGE_MS..=now_ms + MAX_AHEAD_MS).contains(&record.at);
        (fresh && !record.once.is_empty() && record.args_hash == args_hash(args)?).then_some(record)
    }

    /// Whether this yes may run `tool` with `args`.
    pub fn covers(&self, tool: &str, args: &Map<String, Value>) -> bool {
        (APPROVED.contains(&tool) || tool == ENTRY_DRAFT)
            && (tool != BATCH || text(args, "action") == Some(BATCH_POST))
    }
}

/// Whether `text` is `sha256:` and 64 lowercase hex digits.
fn is_hash(text: &str) -> bool {
    text.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
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

/// Refuses a call with an argument outside `known`: the card would not show it.
fn accounted(call: &Map<String, Value>, known: &[&str]) -> Result<(), Refusal> {
    match call.keys().find(|k| !known.contains(&k.as_str())) {
        Some(k) => Err(Refusal::new(
            "permission_denied",
            format!("This press holds `{k}`, which its card can't show, so it can't be allowed."),
        )),
        None => Ok(()),
    }
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

/// `label: value`, if there is a value.
fn line(label: &str, value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| format!("{label}: {v}"))
}

/// The book, if the call names one: it decides where this lands.
fn book_line(chart: &Chart, call: &Map<String, Value>) -> Option<String> {
    call.contains_key("book")
        .then(|| format!("Book: {}", chart.book.label))
}

/// A tag argument as the entry will hold it: a ticket by its name, a name as
/// it is, which Books makes if it is new.
fn tag_of(store: &Store, value: &str) -> Result<String, Refusal> {
    if !plumbing::is_ticket(value) {
        return Ok(value.to_string());
    }
    match plumbing::subject(store.conn(), value)? {
        Some((kind, id)) if kind == "tag" => Ok(people::tag(store.conn(), id)?
            .map(|t| t.name)
            .unwrap_or_default()),
        _ => Err(Refusal::new(
            "not_found",
            "That tag ticket is not one Books issued.",
        )),
    }
}

/// Who and what entry `id` links to, by name: person, department, location.
fn names_of(store: &Store, id: i64) -> Result<[Option<String>; 3], Refusal> {
    let (person, department, location) = people::links(store.conn(), id)?;
    let tag = |t: Option<i64>| -> Result<Option<String>, Refusal> {
        Ok(match t {
            Some(t) => people::tag(store.conn(), t)?.map(|t| t.name),
            None => None,
        })
    };
    let person = match person {
        Some(p) => people::person(store.conn(), p)?.map(|p| p.name),
        None => None,
    };
    Ok([person, tag(department)?, tag(location)?])
}

/// The person a new entry links to.
fn person_line(store: &Store, call: &Map<String, Value>) -> Result<Option<String>, Refusal> {
    if let Some(ticket) = text(call, "person") {
        return match plumbing::subject(store.conn(), ticket)? {
            Some((kind, id)) if kind == "person" => {
                Ok(people::person(store.conn(), id)?.map(|p| format!("Linked person: {}", p.name)))
            }
            _ => Err(Refusal::new(
                "not_found",
                "That person ticket is not one Books issued.",
            )),
        };
    }
    Ok(match (text(call, "person_kind"), text(call, "payee")) {
        (Some(kind), Some(name)) if !name.trim().is_empty() => {
            Some(format!("Linked person: {name} ({kind})"))
        }
        (Some(kind), _) => Some(format!("Person kind: {kind}, with nobody named")),
        _ => None,
    })
}

/// A journal's lines may hold only what the card shows.
fn lines_accounted(call: &Map<String, Value>) -> Result<Vec<String>, Refusal> {
    let mut facts = Vec::new();
    for (i, l) in call
        .get("lines")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        let l = l
            .as_object()
            .ok_or_else(|| Refusal::invalid("Each journal line is an object."))?;
        accounted(l, &LINE_ARGS)?;
        if let Some(f) = l.get("facts").filter(|f| !f.is_null()) {
            facts.push(format!("Facts on line {}: {f}", i + 1));
        }
    }
    Ok(facts)
}

/// What a new entry comes to, every argument shown: `(amount, card lines)`.
fn new_entry(
    store: &Store,
    chart: &Chart,
    call: &Map<String, Value>,
) -> Result<(i64, Vec<String>), Refusal> {
    let input = writes::draft_input(store.conn(), &chart.book, call)?;
    let (rows, total) = lines_of(chart, &new_rows(&input.lines));
    let mut lines = vec![format!("Date: {}", input.date)];
    lines.extend(book_line(chart, call));
    lines.extend(line("With", input.payee.as_deref()));
    lines.extend(person_line(store, call)?);
    lines.extend(rows);
    lines.extend(line("For", input.memo.as_deref()));
    lines.extend(line("Reference", input.reference.as_deref()));
    for (label, key) in [("Department", "department"), ("Location", "location")] {
        if let Some(v) = text(call, key) {
            lines.extend(line(label, Some(tag_of(store, v)?.as_str())));
        }
    }
    if let Some(f) = call.get("facts").filter(|f| !f.is_null()) {
        lines.push(format!("Facts: {f}"));
    }
    lines.extend(lines_accounted(call)?);
    Ok((total, lines))
}

/// An entry that exists, as it stands: `(amount, card lines)`.
fn saved_entry(
    store: &Store,
    chart: &Chart,
    e: &entries::Entry,
) -> Result<(i64, Vec<String>), Refusal> {
    let rows: Vec<(i64, i64, i64)> = entries::lines(store.conn(), e.id)?
        .iter()
        .map(|l| (l.account_id, l.debit, l.credit))
        .collect();
    let (rows, total) = lines_of(chart, &rows);
    let mut lines = vec![format!("Date: {}", e.date)];
    lines.extend(line("With", e.payee.as_deref()));
    let [person, department, location] = names_of(store, e.id)?;
    lines.extend(line("Linked person", person.as_deref()));
    lines.extend(rows);
    lines.extend(line("For", e.memo.as_deref()));
    lines.extend(line("Reference", e.reference.as_deref()));
    lines.extend(line("Department", department.as_deref()));
    lines.extend(line("Location", location.as_deref()));
    Ok((total, lines))
}

fn post_card(store: &Store, chart: &Chart, call: &Map<String, Value>) -> Answer {
    let (title, mut lines) = match text(call, "entry") {
        Some(ticket) => {
            accounted(call, &POST_ENTRY_ARGS)?;
            let e = writes::entry_in(store.conn(), &chart.book, ticket)?;
            let (total, mut lines) = saved_entry(store, chart, &e)?;
            lines.extend(book_line(chart, call));
            (
                format!("Post entry {:04} for {}?", e.no, peso(total)),
                lines,
            )
        }
        None => {
            accounted(call, &POST_NEW_ARGS)?;
            let (total, lines) = new_entry(store, chart, call)?;
            (
                format!("Save and post this entry for {}?", peso(total)),
                lines,
            )
        }
    };
    if call.get("keep_both").and_then(Value::as_bool) == Some(true) {
        lines.push(DUPLICATES.to_string());
    }
    Ok(json!({ "title": title, "lines": lines }))
}

/// A held draft is a new one, with [`POST_NEW_ARGS`]: see [`DRAFT_REFUSED`].
fn draft_card(store: &Store, chart: &Chart, call: &Map<String, Value>) -> Answer {
    accounted(call, &POST_NEW_ARGS)?;
    let (total, mut lines) = new_entry(store, chart, call)?;
    lines.push("A draft counts in no total until a person posts it.".to_string());
    let title = format!("Save a draft for {}?", peso(total));
    Ok(json!({ "title": title, "lines": lines }))
}

fn close_card(store: &Store, chart: &Chart, call: &Map<String, Value>) -> Answer {
    accounted(call, &CLOSE_ARGS)?;
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
    lines.extend(line("Reason", text(call, "reason")));
    lines.extend(book_line(chart, call));
    Ok(json!({ "title": format!("Close {}?", period.label()), "lines": lines }))
}

fn batch_card(store: &Store, chart: &Chart, call: &Map<String, Value>) -> Answer {
    accounted(call, &BATCH_ARGS)?;
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
    if list.len() > MAX_ENTRIES {
        return Err(Refusal::invalid(format!(
            "A batch takes 1 to {MAX_ENTRIES} entries."
        )));
    }
    let (mut total, mut named) = (0_i64, Vec::new());
    let mut seen = HashSet::new();
    for ticket in list {
        let ticket = ticket
            .as_str()
            .ok_or_else(|| Refusal::invalid("Each entry is a ticket."))?;
        if !seen.insert(ticket) {
            return Err(Refusal::invalid("An entry is in the batch twice."));
        }
        let e = writes::entry_in(store.conn(), &chart.book, ticket)?;
        let amount: i64 = entries::lines(store.conn(), e.id)?
            .iter()
            .map(|l| l.debit)
            .sum();
        total += amount;
        let about: Vec<String> = names_of(store, e.id)?.into_iter().flatten().collect();
        let about = if about.is_empty() {
            String::new()
        } else {
            format!(" ({})", about.join(", "))
        };
        named.push(format!(
            "Entry {:04}, {}: {}{about}",
            e.no,
            e.date,
            peso(amount)
        ));
    }
    named.extend(book_line(chart, call));
    if call.get("keep_both").and_then(Value::as_bool) == Some(true) {
        named.push(BATCH_DUPLICATES.to_string());
    }
    let plural = if list.len() == 1 { "entry" } else { "entries" };
    let title = format!("Post {} {plural} for {} in all?", list.len(), peso(total));
    Ok(json!({ "title": title, "lines": named }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools;

    /// What a tool's schema takes, sorted.
    fn schema_args(tool: &str) -> Vec<String> {
        let listed = tools::list();
        let t = listed.iter().find(|t| t["name"] == tool).unwrap();
        let mut keys: Vec<String> = t["inputSchema"]["properties"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        keys.sort();
        keys
    }

    /// Whatever a tool's schema takes has to be named here, as shown on the
    /// card or as what its card refuses: a new argument fails until it is.
    fn all_accounted(tool: &str, shown: &[&[&str]], refused: &[&str]) {
        for arg in schema_args(tool) {
            let on_card = shown.iter().any(|s| s.contains(&arg.as_str()));
            assert!(
                on_card || refused.contains(&arg.as_str()),
                "{tool} takes `{arg}`: show it on the card, or list it as refused"
            );
            assert!(!(on_card && refused.contains(&arg.as_str())), "{arg}");
        }
    }

    /// Post's arguments that change nothing it posts: `copy_of` and the
    /// extraction belong to a draft, so a press holding one is refused.
    const POST_REFUSED: [&str; 3] = ["copy_of", "extracted", "extraction"];

    #[test]
    fn every_argument_of_post_is_on_its_card_or_refused() {
        all_accounted(
            POST_ENTRY,
            &[&POST_ENTRY_ARGS, &POST_NEW_ARGS],
            &POST_REFUSED,
        );
        let call = |key: &str| json!({ key: "x" }).as_object().unwrap().clone();
        for arg in POST_REFUSED {
            assert!(accounted(&call(arg), &POST_NEW_ARGS).is_err(), "{arg}");
            assert!(accounted(&call(arg), &POST_ENTRY_ARGS).is_err(), "{arg}");
        }
        for arg in ["date", "amount", "memo", "person", "department", "lines"] {
            assert!(
                accounted(&call(arg), &POST_ENTRY_ARGS).is_err(),
                "an entry's post takes no `{arg}`: it wouldn't use it"
            );
        }
    }

    #[test]
    fn every_argument_of_a_draft_is_on_its_card_or_refused() {
        all_accounted(ENTRY_DRAFT, &[&POST_NEW_ARGS], &DRAFT_REFUSED);
        for arg in DRAFT_REFUSED {
            let call = json!({ arg: "x" }).as_object().unwrap().clone();
            assert!(accounted(&call, &POST_NEW_ARGS).is_err(), "{arg}");
        }
    }

    #[test]
    fn every_argument_of_close_month_is_on_its_card() {
        all_accounted(MONTH_CLOSE, &[&CLOSE_ARGS], &[]);
    }

    /// `category` is for a recategorize, which takes no yes.
    #[test]
    fn every_argument_of_a_batch_is_on_its_card_or_refused() {
        all_accounted(BATCH, &[&BATCH_ARGS], &["category"]);
    }

    #[test]
    fn an_argument_nobody_knows_refuses_the_card() {
        let call = json!({ "client_request_id": "x", "new_thing": 1 })
            .as_object()
            .unwrap()
            .clone();
        for known in [
            &POST_ENTRY_ARGS[..],
            &POST_NEW_ARGS,
            &CLOSE_ARGS,
            &BATCH_ARGS,
        ] {
            assert_eq!(
                accounted(&call, known).unwrap_err().code,
                "permission_denied"
            );
        }
    }

    const NOW: i64 = 1_793_745_245_000;

    fn record(args: &Map<String, Value>) -> Value {
        json!({ "by": "ai_approved", "who": "agent.broker", "approved_by": "person.owner", "once": "press-0001",
            "args_hash": args_hash(args).unwrap(), "shown_hash": format!("sha256:{}", "a1".repeat(32)), "at": NOW })
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
        let got = Approved::of(Some(&record(&args())), &args(), NOW).unwrap();
        assert_eq!(
            (got.who.as_str(), got.approved_by.as_str()),
            ("agent.broker", "person.owner")
        );
    }

    #[test]
    fn a_record_for_other_arguments_or_with_a_hole_is_no_yes() {
        let mut other = args();
        other.insert("month".into(), json!("2026-09"));
        assert_eq!(Approved::of(Some(&record(&args())), &other, NOW), None);
        assert_eq!(Approved::of(None, &args(), NOW), None);
        for (key, value) in [
            ("by", json!("ai")),
            ("who", json!("person.owner")),
            ("approved_by", json!("agent.broker")),
            ("once", Value::Null),
            ("at", json!("now")),
            ("shown_hash", Value::Null),
            ("shown_hash", json!("")),
            ("shown_hash", json!("sha256:")),
            ("shown_hash", json!(format!("sha256:{}", "A1".repeat(32)))),
            ("once", json!("")),
        ] {
            let mut bad = record(&args());
            bad[key] = value;
            assert_eq!(Approved::of(Some(&bad), &args(), NOW), None, "{key}");
        }
    }

    #[test]
    fn a_yes_is_good_for_a_minute_and_not_from_the_future() {
        let at = |ms: i64| {
            let mut r = record(&args());
            r["at"] = json!(ms);
            Approved::of(Some(&r), &args(), NOW).is_some()
        };
        assert!(at(NOW) && at(NOW - MAX_AGE_MS) && at(NOW + MAX_AHEAD_MS));
        assert!(!at(NOW - MAX_AGE_MS - 1), "stale");
        assert!(!at(NOW + MAX_AHEAD_MS + 1), "from the future");
    }

    #[test]
    fn a_yes_covers_post_close_and_a_batch_that_only_posts() {
        let yes = Approved::of(Some(&record(&args())), &args(), NOW).unwrap();
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
