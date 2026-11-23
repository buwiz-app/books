//! Statement actions for the reconciliation detail page: the ledger
//! entries beside the lines, unmatch, manual link, remove, and the
//! comments + activity feed behind the sidebar tabs.

use books_mini_app::demo;
use books_mini_app::service::{Books, Origin};
use books_mini_app::tools;
use books_store::{Paths, TempDir};
use serde_json::{Map, Value, json};

const DEMO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/demo/buwiz-trading-2026.json"
);
const STATEMENT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/demo/bank-4021-2026-10.csv"
);
const ACCT: &str = "11010";
const MONTH: &str = "2026-10";

fn run(books: &mut Books, origin: Origin, name: &str, args: Value) -> Value {
    let args: Map<String, Value> = args.as_object().unwrap().clone();
    tools::call(name, &args, origin, books).unwrap()
}
fn ok(books: &mut Books, origin: Origin, name: &str, args: Value) -> Value {
    let out = run(books, origin, name, args);
    assert_eq!(out["isError"], false, "{name}: {out}");
    out["structuredContent"].clone()
}
fn app(books: &mut Books, name: &str, args: Value) -> Value {
    ok(books, Origin::App, name, args)
}
fn refused(books: &mut Books, name: &str, args: Value) -> String {
    let out = run(books, Origin::App, name, args);
    assert_eq!(out["isError"], true, "{name}: {out}");
    out["structuredContent"]["code"]
        .as_str()
        .unwrap()
        .to_string()
}
fn get(books: &mut Books, origin: Origin) -> Value {
    ok(
        books,
        origin,
        tools::STATEMENT_GET,
        json!({ "account": ACCT, "month": MONTH }),
    )
}
fn write_args(crid: &str) -> Value {
    json!({ "client_request_id": crid, "account": ACCT, "month": MONTH })
}

/// Seed and import the October CSV; returns the books with a draft
/// statement that has one suggested line and one missing line.
fn seeded(dir: &TempDir) -> Books {
    books_core::date::pin_today(books_core::date::Date::new(2026, 10, 29));
    let mut b = Books::new(Paths::in_dir(dir.0.join("store")));
    let book: Value = serde_json::from_str(&std::fs::read_to_string(DEMO).unwrap()).unwrap();
    demo::seed(&mut b, &book).unwrap();
    let csv = std::fs::read(STATEMENT).unwrap();
    app(
        &mut b,
        "books_app_statement_import",
        json!({ "client_request_id": "sa-import", "account": ACCT,
            "csv_base64": books_mini_app::b64::encode(&csv), "file_name": "bank-4021-2026-10.csv" }),
    );
    b
}

fn row_of(view: &Value, match_kind: &str) -> Option<Value> {
    view["statement"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["match"] == match_kind)
        .cloned()
}

/// The row `accept` stores a match for. A row can also read "m" when the
/// ledger only computes an exact match — it stores nothing until Finish.
fn suggested(view: &Value) -> Value {
    row_of(view, "s").expect("a suggested row")
}

#[test]
fn sa_ledger_entries_list_the_months_movements_with_their_matches() {
    let dir = TempDir::new("sa-ledger");
    let mut b = seeded(&dir);
    let view = get(&mut b, Origin::App);
    let ledger = view["statement"]["ledger_entries"].as_array().unwrap();
    assert!(!ledger.is_empty());
    // every row carries the ledger-list fields
    for e in ledger {
        for f in [
            "ticket",
            "no",
            "date",
            "date_short",
            "form",
            "status",
            "category",
            "amount_shown",
            "matched",
            "linked_line_ticket",
            "linked_line_no",
            "match_kind",
            "flags",
        ] {
            assert!(e.get(f).is_some(), "ledger entry lacks `{f}`: {e}");
        }
    }
    // the entry the suggested line points at appears, unmatched for now
    let s = suggested(&view);
    let entry = s["books"]["ticket"].as_str().unwrap().to_string();
    let accepted = app(
        &mut b,
        "books_app_statement_accept",
        write_args("sa-accept"),
    );
    assert_eq!(accepted["accepted"], 1);
    let view = get(&mut b, Origin::App);
    let ledger = view["statement"]["ledger_entries"].as_array().unwrap();
    let twin = ledger
        .iter()
        .find(|e| e["ticket"] == entry)
        .expect("the matched entry is in the ledger list");
    assert_eq!(twin["matched"], true);
    assert_eq!(twin["linked_line_no"], s["no"]);
    assert_eq!(twin["match_kind"], "accepted");
    // the model sees no payee or memo
    let agent = get(&mut b, Origin::Agent);
    let ledger = agent["statement"]["ledger_entries"].as_array().unwrap();
    assert!(!ledger.is_empty());
    assert!(ledger[0].get("payee").is_none());
    assert!(ledger[0].get("memo").is_none());
    assert!(ledger[0].get("title").is_none());
}

#[test]
fn sa_unmatch_clears_and_link_restores() {
    let dir = TempDir::new("sa-link");
    let mut b = seeded(&dir);
    let view = get(&mut b, Origin::App);
    let s = suggested(&view);
    let entry = s["books"]["ticket"].as_str().unwrap().to_string();
    let no = s["no"].as_i64().unwrap();
    let sline = s["ticket"].as_str().unwrap().to_string();
    app(
        &mut b,
        "books_app_statement_accept",
        write_args("sa-accept-2"),
    );

    // unmatch by row number
    let out = app(
        &mut b,
        "books_app_statement_unmatch",
        json!({ "client_request_id": "sa-unmatch-1", "account": ACCT, "month": MONTH, "line": no }),
    );
    assert_eq!(out["unmatched"], true);
    let view = get(&mut b, Origin::App);
    let row = &view["statement"]["rows"].as_array().unwrap()[usize::try_from(no - 1).unwrap()];
    assert_ne!(row["match"], "m");
    let ledger = view["statement"]["ledger_entries"].as_array().unwrap();
    let twin = ledger.iter().find(|e| e["ticket"] == entry).unwrap();
    assert_eq!(twin["matched"], false);

    // unmatch again is a quiet no-op
    let out = app(
        &mut b,
        "books_app_statement_unmatch",
        json!({ "client_request_id": "sa-unmatch-2", "account": ACCT, "month": MONTH, "line": sline }),
    );
    assert_eq!(out["unmatched"], false);

    // manual link: the same line back to the same entry, by ticket
    let out = app(
        &mut b,
        "books_app_statement_link",
        json!({ "client_request_id": "sa-link-1", "account": ACCT, "month": MONTH,
            "line": sline, "entry": entry }),
    );
    assert_eq!(out["matched"], true);
    let view = get(&mut b, Origin::App);
    let row = &view["statement"]["rows"].as_array().unwrap()[usize::try_from(no - 1).unwrap()];
    assert_eq!(row["match"], "m");
    assert_eq!(row["books"]["ticket"], entry);
}

#[test]
fn sa_link_refuses_a_taken_entry_and_a_taken_line() {
    let dir = TempDir::new("sa-taken");
    let mut b = seeded(&dir);
    let view = get(&mut b, Origin::App);
    let s = suggested(&view);
    let entry = s["books"]["ticket"].as_str().unwrap().to_string();
    let no = s["no"].as_i64().unwrap();
    app(
        &mut b,
        "books_app_statement_accept",
        write_args("sa-accept-3"),
    );
    // the line is taken
    assert_eq!(
        refused(
            &mut b,
            "books_app_statement_link",
            json!({ "client_request_id": "sa-link-t1", "account": ACCT, "month": MONTH,
                "line": no, "entry": entry }),
        ),
        "already_matched"
    );
    // the entry is taken: unmatch the line, then try a different line
    app(
        &mut b,
        "books_app_statement_unmatch",
        json!({ "client_request_id": "sa-unmatch-3", "account": ACCT, "month": MONTH, "line": no }),
    );
    let view = get(&mut b, Origin::App);
    let other = view["statement"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["no"].as_i64() != Some(no) && r["match"] != "m")
        .expect("another unsettled line");
    let code = refused(
        &mut b,
        "books_app_statement_link",
        json!({ "client_request_id": "sa-link-t2", "account": ACCT, "month": MONTH,
            "line": other["no"].as_i64().unwrap(), "entry": entry }),
    );
    assert_eq!(code, "entry_not_matchable", "{code}");
}

#[test]
fn sa_remove_drops_an_open_statement_and_a_finished_one_stays() {
    let dir = TempDir::new("sa-remove");
    let mut b = seeded(&dir);
    app(
        &mut b,
        "books_app_statement_accept",
        write_args("sa-accept-4"),
    );
    let missing = get(&mut b, Origin::App)["statement"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["match"] == "x")
        .unwrap()["ticket"]
        .clone();
    app(
        &mut b,
        "books_app_statement_add_line",
        json!({ "client_request_id": "sa-add-line-01", "account": ACCT, "month": MONTH, "line": missing }),
    );
    let view = get(&mut b, Origin::App);
    assert_eq!(view["statement"]["can_finish"], true);
    app(
        &mut b,
        "books_app_statement_finish",
        write_args("sa-finish"),
    );
    // a finished statement refuses unmatch and remove; comments still work
    let view = get(&mut b, Origin::App);
    let no = view["statement"]["rows"].as_array().unwrap()[0]["no"]
        .as_i64()
        .unwrap();
    assert_eq!(
        refused(
            &mut b,
            "books_app_statement_unmatch",
            json!({ "client_request_id": "sa-unmatch-fn", "account": ACCT, "month": MONTH, "line": no }),
        ),
        "already_finished"
    );
    assert_eq!(
        refused(
            &mut b,
            "books_app_statement_remove",
            write_args("sa-remove-fin"),
        ),
        "already_finished"
    );
    // reopen, then remove clears it all
    app(
        &mut b,
        "books_app_statement_finish",
        json!({ "client_request_id": "sa-reopen", "account": ACCT, "month": MONTH,
            "reopen": true, "reason": "Reviewing the match again" }),
    );
    let out = app(
        &mut b,
        "books_app_statement_remove",
        write_args("sa-remove-001"),
    );
    assert_eq!(out["status"], "removed");
    let view = get(&mut b, Origin::App);
    assert!(view["statement"].is_null());
}

#[test]
fn sa_comments_and_the_activity_feed() {
    let dir = TempDir::new("sa-feed");
    let mut b = seeded(&dir);
    // an empty comment is refused; a real one lands
    assert_eq!(
        refused(
            &mut b,
            "books_app_statement_comment_add",
            json!({ "client_request_id": "sa-comment-00", "account": ACCT, "month": MONTH, "body": "  " }),
        ),
        "invalid_args"
    );
    let out = app(
        &mut b,
        "books_app_statement_comment_add",
        json!({ "client_request_id": "sa-comment-01", "account": ACCT, "month": MONTH,
            "body": "Check this against the vendor invoice" }),
    );
    assert!(out["comment"].is_string());
    app(
        &mut b,
        "books_app_statement_accept",
        write_args("sa-accept-5"),
    );
    let view = get(&mut b, Origin::App);
    let comments = view["statement"]["comments"].as_array().unwrap();
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0]["body"], "Check this against the vendor invoice");
    // the feed shows what happened, oldest first
    let feed = view["statement"]["activity"].as_array().unwrap();
    let events: Vec<&str> = feed.iter().map(|e| e["event"].as_str().unwrap()).collect();
    assert_eq!(events[0], "import", "{events:?}");
    assert!(events.contains(&"comment"), "{events:?}");
    assert!(events.contains(&"match"), "{events:?}");
    // the model never sees the notes
    let agent = get(&mut b, Origin::Agent);
    assert!(agent["statement"].get("comments").is_none());
}
