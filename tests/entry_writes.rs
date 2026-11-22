//! PR #37 tool-level checks behind the EntryPage fixes, on the demo book
//! (fixtures/demo/buwiz-trading-2026.json, synthetic): a second
//! `books_entry_draft` with `entry` edits rather than adds and
//! `books_app_post_entry` with `entry` posts that draft (finding 6), a
//! draft saved with a header `person` lands on that person's activity once
//! posted (finding 7), and an unbalanced journal saves as a draft but is
//! refused on post (finding 10).

use books_mini_app::demo;
use books_mini_app::service::{Books, Origin};
use books_mini_app::tools;
use books_store::{Paths, TempDir};
use serde_json::{Map, Value, json};

fn raw(books: &mut Books, name: &str, args: Value, origin: Origin) -> Value {
    let args: Map<String, Value> = args.as_object().unwrap().clone();
    tools::call(name, &args, origin, books).unwrap()
}

fn call(books: &mut Books, name: &str, args: Value, origin: Origin) -> Value {
    let out = raw(books, name, args, origin);
    assert_eq!(out["isError"], false, "{name}: {out}");
    out["structuredContent"].clone()
}

fn refusal(books: &mut Books, name: &str, args: Value) -> String {
    let out = raw(books, name, args, Origin::App);
    assert_eq!(out["isError"], true, "{name}: {out}");
    out["structuredContent"]["code"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

fn seeded() -> (TempDir, Books) {
    books_core::date::pin_today(books_core::date::Date::new(2026, 10, 29));
    let dir = TempDir::new("entry-writes");
    let mut books = Books::new(Paths::in_dir(dir.0.join("store")));
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/demo/buwiz-trading-2026.json"
    ))
    .unwrap();
    let demo_book: Value = serde_json::from_str(&text).unwrap();
    demo::seed(&mut books, &demo_book).unwrap();
    (dir, books)
}

fn draft_count(books: &mut Books) -> u64 {
    call(
        books,
        "books_ledger_list",
        json!({ "month": "2026-10" }),
        Origin::App,
    )["counts"]["draft"]
        .as_u64()
        .unwrap()
}

#[test]
fn f6_a_second_draft_with_entry_edits_and_post_with_entry_posts() {
    let (_dir, mut books) = seeded();
    let b = &mut books;
    let before = draft_count(b);
    let first = call(
        b,
        "books_entry_draft",
        json!({ "client_request_id": "ew-f6-0001", "form": "money_out", "date": "2026-10-05",
                "amount": "100.00", "account": "11000", "category": "67200" }),
        Origin::App,
    );
    assert_eq!(first["status"], "draft", "{first}");
    let second = call(
        b,
        "books_entry_draft",
        json!({ "client_request_id": "ew-f6-0002", "entry": first["entry"], "form": "money_out",
                "date": "2026-10-05", "amount": "200.00", "account": "11000", "category": "67200" }),
        Origin::App,
    );
    assert_eq!(
        second["entry"], first["entry"],
        "a draft with `entry` added instead of editing"
    );
    assert_eq!(
        draft_count(b),
        before + 1,
        "editing a draft counted as a new one"
    );
    let got = call(
        b,
        "books_entry_get",
        json!({ "entry": first["entry"] }),
        Origin::App,
    );
    assert_eq!(
        got["amount_shown"], "\u{2212}200.00",
        "the edit did not land: {got}"
    );
    let posted = call(
        b,
        "books_app_post_entry",
        json!({ "client_request_id": "ew-f6-0003", "entry": first["entry"] }),
        Origin::App,
    );
    assert_eq!(posted["entry"], first["entry"], "{posted}");
    assert_eq!(posted["status"], "posted", "{posted}");
    let got = call(
        b,
        "books_entry_get",
        json!({ "entry": first["entry"] }),
        Origin::App,
    );
    assert_eq!(got["status"], "posted", "{got}");
}

#[test]
fn f7_a_draft_with_person_lands_in_their_activity() {
    let (_dir, mut books) = seeded();
    let b = &mut books;
    let made = call(
        b,
        "books_app_person_create",
        json!({ "client_request_id": "ew-f7-0001", "kind": "vendors", "name": "Synthetic Vendor EW" }),
        Origin::App,
    );
    let person = made["person"].clone();
    let draft = call(
        b,
        "books_entry_draft",
        json!({ "client_request_id": "ew-f7-0002", "form": "money_out", "date": "2026-10-12",
                "amount": "500.00", "account": "11000", "category": "67200", "person": person }),
        Origin::App,
    );
    let got = call(
        b,
        "books_entry_get",
        json!({ "entry": draft["entry"] }),
        Origin::App,
    );
    assert!(
        got["lines"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l["person"]["ticket"] == person),
        "the header person never reached the draft's lines: {got}"
    );
    // person activity counts posted entries, so the draft shows only once
    // the owner posts it
    call(
        b,
        "books_app_post_entry",
        json!({ "client_request_id": "ew-f7-0003", "entry": draft["entry"] }),
        Origin::App,
    );
    let ps = call(
        b,
        "books_report",
        json!({ "kind": "ps", "person": person, "period_kind": "month", "period": "2026-10" }),
        Origin::App,
    );
    let rows = ps["activity"]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1, "{ps}");
    assert_eq!(ps["activity"]["total_out_shown"], "500.00", "{ps}");
}

#[test]
fn f10_an_unbalanced_journal_drafts_but_never_posts() {
    let (_dir, mut books) = seeded();
    let b = &mut books;
    let draft = call(
        b,
        "books_entry_draft",
        json!({ "client_request_id": "ew-f10-001", "form": "journal", "date": "2026-10-12",
                "lines": [
                    { "account": "11000", "debit": "100.00" },
                    { "account": "67200", "credit": "80.00" }
                ] }),
        Origin::App,
    );
    assert_eq!(draft["status"], "draft", "{draft}");
    assert_eq!(
        refusal(
            b,
            "books_app_post_entry",
            json!({ "client_request_id": "ew-f10-002", "entry": draft["entry"] })
        ),
        "unbalanced"
    );
    let got = call(
        b,
        "books_entry_get",
        json!({ "entry": draft["entry"] }),
        Origin::App,
    );
    assert_eq!(got["status"], "draft", "the refused post went through: {got}");
}
