//! Person statement (books_report kind ps) on the golden book
//! (fixtures/demo/buwiz-trading-2026.json, synthetic): activity totals and
//! open items match the People page, aging lands in the right bucket, the
//! assistant's view carries no name, payee, memo or reference, and the CSV
//! has both blocks.

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
    let dir = TempDir::new("statement");
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

fn kpi(person: &Value, label: &str) -> Value {
    person["kpi"]
        .as_array()
        .unwrap()
        .iter()
        .find(|k| k["label"] == label)
        .unwrap_or_else(|| panic!("no kpi {label}: {person}"))["shown"]
        .clone()
}

fn people(books: &mut Books, kind: &str) -> Vec<Value> {
    call(
        books,
        "books_people_list",
        json!({ "kind": kind }),
        Origin::App,
    )["rows"]
        .as_array()
        .unwrap()
        .clone()
}

fn ticket_of(books: &mut Books, kind: &str, name: &str) -> Value {
    people(books, kind)
        .into_iter()
        .find(|r| r["name"] == name)
        .unwrap_or_else(|| panic!("no {name}"))["ticket"]
        .clone()
}

#[test]
fn a_customer_statement_matches_the_people_page() {
    let (_dir, mut books) = seeded();
    let b = &mut books;
    let mut checked = 0;
    for row in people(b, "customers") {
        let person = call(
            b,
            "books_person_get",
            json!({ "person": row["ticket"] }),
            Origin::App,
        );
        let ps = call(
            b,
            "books_report",
            json!({ "kind": "ps", "person": row["ticket"], "period": "2026" }),
            Origin::App,
        );
        assert_eq!(ps["kind"], "ps");
        assert_eq!(ps["period"]["key"], "2026", "{ps}");
        assert_eq!(ps["person"]["kind"], "customer");
        assert_eq!(
            ps["activity"]["total_in_shown"],
            kpi(&person, "Received this year")
        );
        assert_eq!(ps["open"]["total_collect_shown"], kpi(&person, "Unpaid"));
        assert_eq!(
            ps["basis_note"],
            "On a cash book these are reminders, not in the books."
        );
        let open = ps["open"]["collect"].as_array().unwrap();
        for item in open {
            let b = item["bucket"].as_u64().unwrap();
            let late = item["days_late"].as_i64().unwrap();
            let want = match late {
                0 => 0,
                1..=30 => 1,
                31..=60 => 2,
                _ => 3,
            };
            assert_eq!(b, want, "{item}");
        }
        if !open.is_empty() {
            checked += 1;
        }
    }
    assert!(checked > 0, "the demo has a customer with an open invoice");
}

#[test]
fn the_period_is_a_year_by_default() {
    let (_dir, mut books) = seeded();
    let t = people(&mut books, "customers")[0]["ticket"].clone();
    let ps = call(
        &mut books,
        "books_report",
        json!({ "kind": "ps", "person": t }),
        Origin::App,
    );
    assert_eq!(ps["period"]["key"], "2026", "{ps}");
    let pl = call(
        &mut books,
        "books_report",
        json!({ "kind": "pl" }),
        Origin::App,
    );
    assert_eq!(pl["period"]["key"], "2026-10", "other kinds keep the month");
}

#[test]
fn a_bill_45_days_late_ages_31_to_60_and_the_assistant_sees_no_names() {
    let (_dir, mut books) = seeded();
    let b = &mut books;
    let name = "Synthetic Vendor Canary";
    let memo = "MEMO-SYNTH-statement-canary";
    let reference = "REF-SYNTH-statement-canary";
    let payee = "Synthetic Payee Statement Canary";
    call(
        b,
        "books_app_owed_create",
        json!({ "client_request_id": "ps-owed-0001", "side": "pay", "person_name": name, "amount": "3200.00",
                "category": "67200", "issued": "2026-08-01", "due": "2026-08-16", "reference": reference, "memo": memo }),
        Origin::App,
    );
    let vendor = ticket_of(b, "vendors", name);
    call(
        b,
        "books_app_post_entry",
        json!({ "client_request_id": "ps-post-0001", "form": "money_out", "date": "2026-09-05", "amount": "1350.00",
                "account": "11000", "category": "67200", "person": vendor, "payee": payee, "memo": memo, "reference": reference }),
        Origin::App,
    );
    let args =
        json!({ "kind": "ps", "person": vendor, "period_kind": "month", "period": "2026-09" });
    let ps = call(b, "books_report", args.clone(), Origin::App);
    let item = &ps["open"]["pay"][0];
    assert_eq!(item["days_late"], 45, "{ps}");
    assert_eq!(item["bucket"], 2);
    assert_eq!(ps["buckets"]["pay"][2], "3,200.00");
    assert_eq!(ps["buckets"]["pay"][0], "0.00");
    assert_eq!(ps["open"]["total_pay_shown"], "3,200.00");
    assert_eq!(ps["activity"]["rows"].as_array().unwrap().len(), 1);
    assert_eq!(ps["activity"]["total_out_shown"], "1,350.00");
    assert_eq!(ps["activity"]["net_shown"], "\u{2212}1,350.00");
    let agent = raw(b, "books_report", args.clone(), Origin::Agent);
    assert_eq!(agent["isError"], false, "{agent}");
    let seen = agent.to_string();
    for needle in [name, memo, reference, payee] {
        assert!(!seen.contains(needle), "the assistant sees {needle}");
    }
    assert!(!books_core::privacy::tin_like(&seen), "no TIN-like run");
    let csv = call(b, "books_report_csv", args, Origin::App);
    let text = csv["csv"].as_str().unwrap_or_else(|| panic!("{csv}"));
    assert!(text.contains("Date,Entry,What,In,Out"), "{text}");
    assert!(
        text.contains("\n\nSide,Issued,Due,Days late,Amount")
            || text.contains("\r\n\r\nSide,Issued,Due,Days late,Amount"),
        "{text}"
    );
    assert!(
        text.contains("To pay,2026-08-01,2026-08-16,45,3200.00"),
        "{text}"
    );
    assert!(
        !text.contains("tkt_") && !text.contains(vendor.as_str().unwrap()),
        "entry numbers, not tickets"
    );
    for needle in [name, memo, reference, payee] {
        assert!(!text.contains(needle), "the CSV carries {needle}");
    }
    let file = csv["file_name"].as_str().unwrap();
    assert_eq!(file, "books-ps-2026-09.csv");
}

#[test]
fn bad_person_arguments_are_refused() {
    let (_dir, mut books) = seeded();
    let b = &mut books;
    assert_eq!(
        refusal(b, "books_report", json!({ "kind": "ps" })),
        "invalid_args"
    );
    assert_eq!(
        refusal(
            b,
            "books_report",
            json!({ "kind": "ps", "person": "not-a-ticket" })
        ),
        "not_found"
    );
    let bank = people(b, "banks")[0]["ticket"].clone();
    assert_eq!(
        refusal(b, "books_report", json!({ "kind": "ps", "person": bank })),
        "invalid_args"
    );
    let customer = people(b, "customers")[0]["ticket"].clone();
    let other = call(
        b,
        "books_app_book_create",
        json!({ "client_request_id": "ps-book-0002", "label": "Other Book" }),
        Origin::App,
    );
    let other_book = other["book"].clone();
    assert!(other_book.is_string(), "{other}");
    let code = refusal(
        b,
        "books_report",
        json!({ "kind": "ps", "book": other_book, "person": customer }),
    );
    assert!(code == "not_found" || code == "invalid_args", "{code}");
}

#[test]
fn a_cash_book_keeps_aging_accrual_only() {
    let (_dir, mut books) = seeded();
    let ag = call(
        &mut books,
        "books_report",
        json!({ "kind": "ag" }),
        Origin::App,
    );
    assert_eq!(ag["cash_book"], true);
}
