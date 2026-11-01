//! Account activity (books_report kind `ac`) on the synthetic demo book:
//! rows oldest first with a running balance in the account's own sense,
//! opening plus changes equal to the closing, voids and reversals netting
//! to zero, drafts left out, and nothing the assistant reads carries a
//! payee or memo.

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

fn code(out: &Value) -> String {
    assert_eq!(out["isError"], true, "{out}");
    out["structuredContent"]["code"]
        .as_str()
        .unwrap_or("")
        .to_string()
}

/// Two-decimal text to centavos, for sums in the test only.
fn cents(text: &str) -> i64 {
    let neg = text.starts_with('-');
    let t = text.trim_start_matches('-');
    let (w, f) = t.split_once('.').unwrap();
    let c = w.parse::<i64>().unwrap() * 100 + f.parse::<i64>().unwrap();
    if neg { -c } else { c }
}

/// The demo's bank account (the 11000 group's posting account).
const BANK: &str = "11010";

fn seeded() -> (TempDir, Books) {
    books_core::date::pin_today(books_core::date::Date::new(2026, 10, 29));
    let dir = TempDir::new("activity");
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

fn ac(books: &mut Books, account: &str, period: &str, origin: Origin) -> Value {
    call(
        books,
        "books_report",
        json!({ "kind": "ac", "account": account, "period": period }),
        origin,
    )
}

fn check_sums(r: &Value) {
    let rows = r["rows"].as_array().unwrap();
    let mut bal = cents(r["opening"]["value"].as_str().unwrap());
    for row in rows {
        bal += cents(row["change"].as_str().unwrap());
        assert_eq!(cents(row["balance"].as_str().unwrap()), bal, "{row}");
    }
    if r["more"] == false {
        assert_eq!(bal, cents(r["closing"]["value"].as_str().unwrap()));
    }
    assert_eq!(r["balanced"], true);
}

#[test]
fn bank_activity_runs_oldest_first_and_closes_at_the_accounts_balance() {
    let (_dir, mut books) = seeded();
    let r = ac(&mut books, BANK, "2026-10", Origin::App);
    assert_eq!(r["kind"], "ac");
    assert_eq!(r["account"]["number"], BANK);
    let rows = r["rows"].as_array().unwrap();
    assert!(!rows.is_empty(), "{r}");
    let dates: Vec<&str> = rows.iter().map(|x| x["date"].as_str().unwrap()).collect();
    let mut sorted = dates.clone();
    sorted.sort_unstable();
    assert_eq!(dates, sorted, "oldest first");
    check_sums(&r);
    // the Accounts page balance (through today, Oct 29) plus anything dated later
    let list = call(&mut books, "books_accounts_list", json!({}), Origin::App);
    let bank = list["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["number"] == BANK)
        .unwrap()
        .clone();
    let later: i64 = rows
        .iter()
        .filter(|x| x["date"].as_str().unwrap() > "2026-10-29")
        .map(|x| cents(x["change"].as_str().unwrap()))
        .sum();
    assert_eq!(
        cents(r["closing"]["value"].as_str().unwrap()),
        cents(bank["balance"].as_str().unwrap()) + later
    );
    // the CSV: opening first, closing last, no ticket
    let csv = call(
        &mut books,
        "books_report_csv",
        json!({ "kind": "ac", "account": BANK, "period": "2026-10" }),
        Origin::App,
    );
    assert_eq!(csv["file_name"], "books-ac-11010-2026-10.csv");
    let text = csv["csv"].as_str().unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0], "Date,Entry,What,Debit,Credit,Balance");
    assert!(lines[1].starts_with("Opening"));
    assert!(lines.last().unwrap().starts_with("Closing"));
    assert_eq!(lines.len(), rows.len() + 3);
    assert!(!text.contains("tkt_"), "{text}");
}

#[test]
fn voids_net_to_zero_drafts_stay_out_cards_rise_and_the_assistant_sees_no_payee() {
    let (_dir, mut books) = seeded();
    let memo = "MEMO-SYNTH-activity-canary";
    let payee = "Synthetic Payee Activity";
    let posted = call(
        &mut books,
        "books_app_post_entry",
        json!({ "client_request_id": "ac-post-01", "form": "money_out", "date": "2026-10-20", "amount": "777.00",
                "account": BANK, "category": "67200", "payee": payee, "memo": memo }),
        Origin::App,
    );
    let entry = posted["entry"].as_str().unwrap().to_string();
    let voided = call(
        &mut books,
        "books_app_void_entry",
        json!({ "client_request_id": "ac-void-01", "entry": entry }),
        Origin::App,
    );
    let reversal = voided["reversal"].as_str().unwrap().to_string();
    let draft = call(
        &mut books,
        "books_entry_draft",
        json!({ "client_request_id": "ac-draft-01", "form": "money_out", "date": "2026-10-21", "amount": "333.00",
                "account": BANK, "category": "67200", "payee": payee, "memo": memo }),
        Origin::Agent,
    );
    let draft_entry = draft["entry"].as_str().unwrap().to_string();

    let r = ac(&mut books, BANK, "2026-10", Origin::App);
    check_sums(&r);
    let rows = r["rows"].as_array().unwrap();
    let pair: Vec<&Value> = rows
        .iter()
        .filter(|x| x["entry"] == entry.as_str() || x["entry"] == reversal.as_str())
        .collect();
    assert_eq!(pair.len(), 2, "the void and its reversal both appear");
    assert_eq!(
        pair.iter()
            .map(|x| cents(x["change"].as_str().unwrap()))
            .sum::<i64>(),
        0
    );
    assert!(pair.iter().any(|x| x["reversal"] == true));
    assert!(
        !rows.iter().any(|x| x["entry"] == draft_entry.as_str()),
        "drafts never appear"
    );

    let seen = ac(&mut books, BANK, "2026-10", Origin::Agent).to_string();
    let csv = call(
        &mut books,
        "books_report_csv",
        json!({ "kind": "ac", "account": BANK, "period": "2026-10" }),
        Origin::Agent,
    )
    .to_string();
    for t in [&seen, &csv] {
        assert!(!t.contains(memo) && !t.contains(payee), "{t}");
    }

    // a card (a liability) reads owed: a charge raises its balance
    let list = call(&mut books, "books_accounts_list", json!({}), Origin::App);
    let card = list["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| {
            a["type"] == "liability"
                && (a["cash"] == true || a["kind"] == "loan_payable")
                && a["off"] == false
        })
        .expect("the chart has a card account")
        .clone();
    let card_no = card["number"].as_str().unwrap().to_string();
    let before = ac(&mut books, &card_no, "2026-10", Origin::App);
    call(
        &mut books,
        "books_app_post_entry",
        json!({ "client_request_id": "ac-card-01", "form": "money_out", "date": "2026-10-22", "amount": "450.00",
                "account": card_no, "category": "67200" }),
        Origin::App,
    );
    let after = ac(&mut books, &card_no, "2026-10", Origin::App);
    check_sums(&after);
    let charge = after["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["date"] == "2026-10-22")
        .unwrap()
        .clone();
    assert_eq!(charge["change"], "450.00");
    assert_eq!(
        cents(after["closing"]["value"].as_str().unwrap()),
        cents(before["closing"]["value"].as_str().unwrap()) + 45000
    );
}

#[test]
fn bad_accounts_are_refused() {
    let (_dir, mut books) = seeded();
    assert_eq!(
        code(&raw(
            &mut books,
            "books_report",
            json!({ "kind": "ac" }),
            Origin::App
        )),
        "invalid_args"
    );
    let list = call(&mut books, "books_accounts_list", json!({}), Origin::App);
    let off = list["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["off"] == true)
        .expect("a cash book hides accrual-only accounts")
        .clone();
    let out = raw(
        &mut books,
        "books_report",
        json!({ "kind": "ac", "account": off["ticket"] }),
        Origin::App,
    );
    assert_eq!(code(&out), "invalid_args");
    let person = call(
        &mut books,
        "books_app_person_create",
        json!({ "client_request_id": "ac-person-01", "kind": "customers", "name": "Synthetic Customer" }),
        Origin::App,
    );
    let ticket = person["person"]
        .as_str()
        .or_else(|| person["ticket"].as_str())
        .expect("a person ticket")
        .to_string();
    let out = raw(
        &mut books,
        "books_report",
        json!({ "kind": "ac", "account": ticket }),
        Origin::App,
    );
    assert!(matches!(code(&out).as_str(), "not_found" | "invalid_args"));
}

#[test]
fn more_than_500_rows_keeps_the_oldest_and_the_true_closing() {
    books_core::date::pin_today(books_core::date::Date::new(2026, 10, 29));
    let dir = TempDir::new("activity-many");
    let mut books = Books::new(Paths::in_dir(dir.0.join("store")));
    call(
        &mut books,
        "books_app_book_create",
        json!({ "client_request_id": "ac-book-01", "label": "Many Rows" }),
        Origin::App,
    );
    for i in 0..501 {
        call(
            &mut books,
            "books_app_post_entry",
            json!({ "client_request_id": format!("ac-many-{i:03}"), "form": "money_in", "date": format!("2026-10-{:02}", 1 + i % 28),
                    "amount": "1.00", "account": "11000", "category": "41000" }),
            Origin::App,
        );
    }
    let r = ac(&mut books, "11000", "2026-10", Origin::App);
    assert_eq!(r["more"], true);
    assert_eq!(r["count"], 501);
    assert_eq!(r["rows"].as_array().unwrap().len(), 500);
    assert_eq!(r["closing"]["value"], "501.00");
    assert_eq!(r["balanced"], true);
    check_sums(&r);
    assert_eq!(r["rows"][0]["date"], "2026-10-01");
}

#[test]
fn the_ledger_lists_running_balance_matches_account_activity() {
    let (_dir, mut books) = seeded();
    let draft = call(
        &mut books,
        "books_entry_draft",
        json!({ "client_request_id": "ac-ledger-draft-01", "form": "money_out", "date": "2026-10-21", "amount": "55.00",
                "account": BANK, "category": "67200" }),
        Origin::Agent,
    );
    let activity = ac(&mut books, BANK, "2026-10", Origin::App);
    let by_entry: std::collections::HashMap<&str, &str> = activity["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["entry"].as_str().unwrap(),
                r["balance_shown"].as_str().unwrap(),
            )
        })
        .collect();
    let list = call(
        &mut books,
        "books_ledger_list",
        json!({ "account": BANK, "month": "2026-10", "filter": "all" }),
        Origin::App,
    );
    assert_eq!(list["running_balance"], true);
    let rows = list["rows"].as_array().unwrap();
    assert!(
        rows.iter().any(|r| r["ticket"] == draft["entry"]),
        "the new draft is listed: {list}"
    );
    let mut with_balance = 0;
    for row in rows {
        let ticket = row["ticket"].as_str().unwrap();
        if row["status"] == "draft" {
            assert!(
                row["balance_shown"].is_null(),
                "a draft has no balance: {row}"
            );
            continue;
        }
        assert_eq!(
            row["balance_shown"].as_str(),
            by_entry.get(ticket).copied(),
            "{row}"
        );
        with_balance += 1;
    }
    assert_eq!(with_balance, by_entry.len());
    // newest first: the first posted row carries the closing
    let first = rows.iter().find(|r| !r["balance_shown"].is_null()).unwrap();
    assert_eq!(first["balance_shown"], activity["closing"]["shown"]);
    // all accounts: no running balance
    let all = call(
        &mut books,
        "books_ledger_list",
        json!({ "month": "2026-10", "filter": "all" }),
        Origin::App,
    );
    assert_eq!(all["running_balance"], false);
    assert!(all["rows"][0].get("balance_shown").is_none());
}
