//! The golden book (`fixtures/demo/buwiz-trading-2026.json`, synthetic):
//! seeded through the tools as the app, its reports must give the
//! drawing's numbers (M3-04): the October P&L nets 251,151.00 and the trial
//! balance at Oct 31 totals 1,224,548.40 a side. Then the M2 exits: People
//! shows totals per person, and marking an invoice paid records the money
//! on that day.

use books_mini_app::demo;
use books_mini_app::service::{Books, Origin};
use books_mini_app::tools;
use books_store::{Paths, TempDir};
use serde_json::{Map, Value, json};

fn call(books: &mut Books, name: &str, args: Value, origin: Origin) -> Value {
    let args: Map<String, Value> = args.as_object().unwrap().clone();
    let out = tools::call(name, &args, origin, books).unwrap();
    assert_eq!(out["isError"], false, "{name}: {out}");
    out["structuredContent"].clone()
}

fn seeded() -> (TempDir, Books) {
    // the demo's day, for every test in this binary
    books_core::date::pin_today(books_core::date::Date::new(2026, 10, 29));
    let dir = TempDir::new("golden");
    let mut books = Books::new(Paths::in_dir(dir.0.join("store")));
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/demo/buwiz-trading-2026.json"
    ))
    .unwrap();
    let demo_book: Value = serde_json::from_str(&text).unwrap();
    let n = demo::seed(&mut books, &demo_book).unwrap();
    assert_eq!(n, demo_book["steps"].as_array().unwrap().len());
    (dir, books)
}

fn row<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["sections"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|s| s["rows"].as_array().unwrap())
        .find(|r| r["name"] == name)
        .unwrap_or_else(|| panic!("no row {name}"))
}

#[test]
fn m3_the_golden_book_gives_the_drawing_numbers() {
    let (_dir, mut books) = seeded();
    let b = &mut books;
    let pl = call(
        b,
        "books_report",
        json!({ "kind": "pl", "period": "2026-10" }),
        Origin::App,
    );
    assert_eq!(pl["net"]["shown"], "251,151.00");
    assert_eq!(pl["net"]["value"], "251151.00");
    assert_eq!(pl["columns"], json!(["October", "September"]));
    assert_eq!(row(&pl, "Sales Revenue")["shown"], "300,000.00");
    assert_eq!(row(&pl, "Rent")["shown"], "25,000.00");
    let uncat = row(&pl, "Uncategorized Expense");
    assert_eq!(uncat["shown"], "2,649.00");
    assert_eq!(uncat["review"], "2 to review");
    assert!(
        books_store::plumbing::is_ticket(uncat["ticket"].as_str().unwrap()),
        "a line opens its account"
    );
    assert_eq!(uncat["month"], "2026-10");

    let tb = call(
        b,
        "books_report",
        json!({ "kind": "tb", "period": "2026-10" }),
        Origin::App,
    );
    assert_eq!(tb["balanced"], true);
    assert_eq!(tb["net"]["debit_shown"], "1,224,548.40");
    assert_eq!(tb["net"]["credit_shown"], "1,224,548.40");

    let bs = call(
        b,
        "books_report",
        json!({ "kind": "bs", "period": "2026-10" }),
        Origin::App,
    );
    assert_eq!(bs["balanced"], true);
    assert_eq!(row(&bs, "Bank •••• 4021")["shown"], "412,350.00");
    assert_eq!(row(&bs, "Net income this year")["shown"], "480,361.00");
    assert_eq!(bs["sections"][0]["total"]["shown"], "429,250.00");
    assert_eq!(bs["net"]["shown"], "429,250.00");

    let cf = call(
        b,
        "books_report",
        json!({ "kind": "cf", "period": "2026-10" }),
        Origin::App,
    );
    assert_eq!(cf["balanced"], true);
    assert_eq!(cf["net"]["shown"], "417,350.00");

    let q3 = call(
        b,
        "books_report",
        json!({ "kind": "pl", "period_kind": "quarter", "period": "2026-Q3" }),
        Origin::App,
    );
    assert_eq!(q3["columns"], json!(["Q3 2026", "Q2 2026"]));
    let sales = row(&q3, "Sales Revenue")["shown"]
        .as_str()
        .unwrap()
        .to_string();
    let refunds = row(&q3, "Refunds & Discounts")["shown"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        (sales.as_str(), refunds.as_str()),
        ("300,000.00", "\u{2212}10,000.00")
    );

    let ag = call(
        b,
        "books_report",
        json!({ "kind": "ag", "period": "2026-10" }),
        Origin::App,
    );
    assert_eq!(ag["cash_book"], true, "aging is for accrual books only");

    let csv = call(
        b,
        "books_report_csv",
        json!({ "kind": "pl", "period": "2026-10" }),
        Origin::App,
    );
    let text = csv["csv"].as_str().unwrap();
    assert!(
        text.starts_with("Section,Number,Account,October,September\n"),
        "{text}"
    );
    assert!(
        text.contains(",,Net income,251151.00,217801.00\n")
            || text.contains("Net income,251151.00"),
        "{text}"
    );
    assert!(
        !text.contains('\u{2212}') && !text.contains("1,"),
        "plain decimal text: {text}"
    );
    assert_eq!(csv["file_name"], "books-pl-2026-10.csv");
}

#[test]
fn m1_m2_today_people_and_mark_paid_on_the_golden_book() {
    let (_dir, mut books) = seeded();
    let b = &mut books;
    let today = call(b, "books_today", json!({}), Origin::App);
    assert_eq!(today["cash"]["shown"], "417,350.00");
    assert_eq!(
        today["month"]["shown"], "+ 251,151.00",
        "the month so far is its P&L"
    );
    assert_eq!(today["month"]["rows"][2]["label"], "vs Sep");
    assert_eq!(today["unpaid"]["rows"][0]["label"], "To collect · 2");
    assert_eq!(today["unpaid"]["rows"][0]["shown"], "105,000.00");
    assert_eq!(today["unpaid"]["rows"][1]["shown"], "\u{2212}5,800.00");
    assert_eq!(today["unpaid"]["rows"][2]["shown"], "Nov 05");
    let kinds: Vec<&str> = today["needs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["review", "draft"]);
    assert_eq!(today["needs"][0]["count"], 2);

    let ledger = call(
        b,
        "books_ledger_list",
        json!({ "month": "2026-10" }),
        Origin::App,
    );
    assert_eq!(ledger["counts"]["review"], 2);
    assert_eq!(ledger["counts"]["draft"], 1);
    let titles: Vec<&str> = ledger["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["title"].as_str().unwrap())
        .collect();
    assert!(titles.contains(&"Sale · Bayani Foods"), "{titles:?}");
    let short: Vec<&str> = ledger["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|g| g["rows"].as_array().unwrap())
        .map(|a| a["number"].as_str().unwrap())
        .collect();
    assert!(
        short.len() < 30 && short.contains(&"11010") && !short.contains(&"13800"),
        "accounts in use: {short:?}"
    );

    // People: one page, totals per person, no TIN
    let customers = call(
        b,
        "books_people_list",
        json!({ "kind": "customers" }),
        Origin::App,
    );
    assert_eq!(customers["rows"].as_array().unwrap().len(), 3);
    let isla = customers["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "Isla Café")
        .unwrap()
        .clone();
    let person = call(
        b,
        "books_person_get",
        json!({ "person": isla["ticket"] }),
        Origin::App,
    );
    assert_eq!(person["kpi"][0]["label"], "Received this year");
    assert_eq!(person["kpi"][0]["shown"], "640,000.00");
    assert_eq!(
        person["kpi"][1]["shown"], "60,000.00",
        "Invoice 0143 is unpaid"
    );
    assert_eq!(person["spark"].as_array().unwrap().len(), 10);
    let model = call(
        b,
        "books_person_get",
        json!({ "person": isla["ticket"] }),
        Origin::Agent,
    );
    assert!(
        model.get("name").is_none() && !model.to_string().contains("Isla"),
        "the model sees no name"
    );
    let banks = call(
        b,
        "books_people_list",
        json!({ "kind": "banks" }),
        Origin::App,
    );
    assert!(
        banks["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["name"] == "Credit Cards")
    );
    let tags = call(
        b,
        "books_tags_list",
        json!({ "kind": "department", "month": "2026-10" }),
        Origin::App,
    );
    let ops = tags["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "Operations")
        .unwrap();
    assert_eq!(ops["spent_shown"], "46,200.00");

    // Unpaid: mark paid records the money on that day (M2 exit)
    let owed = call(
        b,
        "books_owed_list",
        json!({ "side": "collect" }),
        Origin::App,
    );
    assert_eq!(owed["total_shown"], "105,000.00");
    let first = owed["rows"][0].clone();
    assert_eq!(
        (
            first["who"].as_str(),
            first["doc"].as_str(),
            first["due_short"].as_str()
        ),
        (Some("Isla Café"), Some("Invoice 0143"), Some("Nov 15"))
    );
    let paid = call(
        b,
        "books_app_owed_settle",
        json!({ "client_request_id": "golden-settle-1", "owed": first["ticket"], "account": "11010", "date": "2026-10-29" }),
        Origin::App,
    );
    assert_eq!(paid["status"], "paid");
    let entry = call(
        b,
        "books_entry_get",
        json!({ "entry": paid["entry"] }),
        Origin::App,
    );
    assert_eq!(
        (
            entry["date"].as_str(),
            entry["status"].as_str(),
            entry["amount_shown"].as_str()
        ),
        (Some("2026-10-29"), Some("posted"), Some("+60,000.00"))
    );
    assert_eq!(entry["category"], "Sales Revenue");
    let after = call(
        b,
        "books_owed_list",
        json!({ "side": "collect" }),
        Origin::App,
    );
    assert_eq!(after["counts"]["collect"], 1);
    let pl = call(
        b,
        "books_report",
        json!({ "kind": "pl", "period": "2026-10" }),
        Origin::App,
    );
    assert_eq!(
        pl["net"]["shown"], "311,151.00",
        "the paid invoice is October income on a cash book"
    );
    // marking paid is the user's click in the app (DD-20)
    assert_eq!(
        tools::find("books_app_owed_settle").unwrap().reach,
        tools::Reach::App
    );
}

#[test]
fn m2_m3_an_accrual_book_posts_receivables_ages_them_and_clears_them() {
    books_core::date::pin_today(books_core::date::Date::new(2026, 10, 29));
    let dir = TempDir::new("accrual");
    let mut books = Books::new(Paths::in_dir(dir.0.join("store")));
    let b = &mut books;
    call(
        b,
        "books_app_book_create",
        json!({ "client_request_id": "acc-book-0001", "label": "Accrual Test", "basis": "accrual" }),
        Origin::App,
    );
    let inv = call(
        b,
        "books_app_owed_create",
        json!({ "client_request_id": "acc-owed-0001", "side": "collect", "person_name": "Synthetic Customer", "amount": "45000.00", "category": "41000", "issued": "2026-09-01", "due": "2026-09-20", "reference": "Invoice 9001" }),
        Origin::App,
    );
    let tb = call(
        b,
        "books_report",
        json!({ "kind": "tb", "period": "2026-09" }),
        Origin::App,
    );
    assert_eq!(
        row(&tb, "Accounts Receivable - Gross")["debit_shown"],
        "45,000.00",
        "accrual: the receivable posts at invoice"
    );
    let pl = call(
        b,
        "books_report",
        json!({ "kind": "pl", "period": "2026-09" }),
        Origin::App,
    );
    assert_eq!(
        pl["net"]["shown"], "45,000.00",
        "accrual: income at invoice"
    );
    let ag = call(
        b,
        "books_report",
        json!({ "kind": "ag", "period": "2026-10" }),
        Origin::App,
    );
    assert_eq!(ag["cash_book"], false);
    assert_eq!(ag["rows"][0]["name"], "Synthetic Customer");
    assert_eq!(
        ag["rows"][0]["cells"],
        json!(["", "", "45,000.00", ""]),
        "41 days late at Oct 31"
    );
    call(
        b,
        "books_app_owed_settle",
        json!({ "client_request_id": "acc-settle-001", "owed": inv["owed"], "account": "11000", "date": "2026-10-10" }),
        Origin::App,
    );
    let tb = call(
        b,
        "books_report",
        json!({ "kind": "tb", "period": "2026-10" }),
        Origin::App,
    );
    assert!(
        tb["sections"][0]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["name"] != "Accounts Receivable - Gross"),
        "paid: the receivable clears"
    );
    let pl = call(
        b,
        "books_report",
        json!({ "kind": "pl", "period": "2026-10" }),
        Origin::App,
    );
    assert_eq!(
        pl["net"]["shown"], "0.00",
        "accrual: collecting is not income again"
    );
    // cancelling an open accrual document voids its receivable
    let bill = call(
        b,
        "books_app_owed_create",
        json!({ "client_request_id": "acc-owed-0002", "side": "pay", "person_name": "Synthetic Vendor", "amount": "3200.00", "category": "67200", "issued": "2026-10-01" }),
        Origin::App,
    );
    call(
        b,
        "books_app_owed_cancel",
        json!({ "client_request_id": "acc-cancel-01", "owed": bill["owed"] }),
        Origin::App,
    );
    let tb = call(
        b,
        "books_report",
        json!({ "kind": "tb", "period": "2026-10" }),
        Origin::App,
    );
    assert!(
        tb["sections"][0]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["name"] != "Accounts Payable")
    );
    assert_eq!(tb["balanced"], true);
}
