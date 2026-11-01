//! books_report compare_to and tag (reports-1), on the golden book
//! (fixtures/demo/buwiz-trading-2026.json, synthetic).

use books_mini_app::demo;
use books_mini_app::service::{Books, Origin};
use books_mini_app::tools;
use books_store::{Paths, TempDir};
use serde_json::{Map, Value, json};

fn raw(books: &mut Books, name: &str, args: Value, origin: Origin) -> Value {
    let args: Map<String, Value> = args.as_object().unwrap().clone();
    tools::call(name, &args, origin, books).unwrap()
}

fn call(books: &mut Books, name: &str, args: Value) -> Value {
    let out = raw(books, name, args, Origin::App);
    assert_eq!(out["isError"], false, "{name}: {out}");
    out["structuredContent"].clone()
}

fn refused(books: &mut Books, args: Value) -> String {
    let out = raw(books, "books_report", args, Origin::App);
    assert_eq!(out["isError"], true, "{out}");
    out["structuredContent"]["code"]
        .as_str()
        .or_else(|| out["structuredContent"]["error"]["code"].as_str())
        .unwrap_or_else(|| panic!("no code: {out}"))
        .to_string()
}

fn seeded() -> (TempDir, Books) {
    books_core::date::pin_today(books_core::date::Date::new(2026, 10, 29));
    let dir = TempDir::new("compare");
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

fn rows(report: &Value) -> Vec<&Value> {
    report["sections"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|s| s["rows"].as_array().unwrap())
        .collect()
}

fn total<'a>(report: &'a Value, title: &str) -> &'a Value {
    &report["sections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["title"] == title)
        .unwrap_or_else(|| panic!("no section {title}"))["total"]
}

#[test]
fn the_pl_keeps_its_default_and_compares_on_request() {
    let (_dir, mut books) = seeded();
    let b = &mut books;
    let pl = call(
        b,
        "books_report",
        json!({ "kind": "pl", "period": "2026-10" }),
    );
    assert_eq!(pl["net"]["shown"], "251,151.00");
    assert_eq!(pl["columns"], json!(["October", "September"]));
    assert_eq!(pl["compare"]["key"], "2026-09");
    assert_eq!(pl["compare"]["mode"], "previous");
    assert!(pl["tag"].is_null());
    let explicit = call(
        b,
        "books_report",
        json!({ "kind": "pl", "period": "2026-10", "compare_to": "previous" }),
    );
    assert_eq!(explicit["net"], pl["net"]);
    assert_eq!(explicit["sections"], pl["sections"]);

    let one = call(
        b,
        "books_report",
        json!({ "kind": "pl", "period": "2026-10", "compare_to": "none" }),
    );
    assert_eq!(one["columns"].as_array().unwrap().len(), 1);
    assert!(one["compare"].is_null());
    assert!(rows(&one).iter().all(|r| r.get("prev_shown").is_none()));
    assert!(one["net"].get("prev_shown").is_none());
    assert_eq!(one["net"]["shown"], "251,151.00");

    let ly = call(
        b,
        "books_report",
        json!({ "kind": "pl", "period": "2026-10", "compare_to": "last_year" }),
    );
    assert_eq!(ly["compare"]["key"], "2025-10");
    assert_eq!(ly["compare"]["mode"], "last_year");
    assert!(!rows(&ly).is_empty());
    assert!(rows(&ly).iter().all(|r| r["prev_shown"] == "0.00"), "{ly}");
    assert_eq!(ly["net"]["prev_shown"], "0.00");
    let q = call(
        b,
        "books_report",
        json!({ "kind": "pl", "period_kind": "quarter", "period": "2026-Q3", "compare_to": "last_year" }),
    );
    assert_eq!(q["compare"]["key"], "2025-Q3");
    let y = call(
        b,
        "books_report",
        json!({ "kind": "pl", "period_kind": "year", "period": "2026", "compare_to": "last_year" }),
    );
    assert_eq!(y["compare"]["key"], "2025");
}

#[test]
fn the_balance_sheet_compares_to_the_previous_as_of_date() {
    let (_dir, mut books) = seeded();
    let b = &mut books;
    let plain = call(
        b,
        "books_report",
        json!({ "kind": "bs", "period": "2026-10" }),
    );
    assert!(
        plain["compare"].is_null(),
        "bs compares to nothing by default"
    );
    assert_eq!(plain["columns"].as_array().unwrap().len(), 1);
    let bs = call(
        b,
        "books_report",
        json!({ "kind": "bs", "period": "2026-10", "compare_to": "previous" }),
    );
    let sep = call(
        b,
        "books_report",
        json!({ "kind": "bs", "period": "2026-09" }),
    );
    assert_eq!(bs["columns"].as_array().unwrap().len(), 2);
    assert_eq!(
        total(&bs, "Assets")["shown"],
        total(&plain, "Assets")["shown"]
    );
    assert_eq!(
        total(&bs, "Assets")["prev_shown"],
        total(&sep, "Assets")["shown"]
    );
    assert_eq!(bs["net"]["prev_shown"], sep["net"]["shown"]);
    assert!(rows(&bs).iter().all(|r| r.get("prev_shown").is_some()));
    assert_eq!(bs["balanced"], true);

    let cf = call(
        b,
        "books_report",
        json!({ "kind": "cf", "period": "2026-10", "compare_to": "previous" }),
    );
    let cf_sep = call(
        b,
        "books_report",
        json!({ "kind": "cf", "period": "2026-09" }),
    );
    assert_eq!(cf["net"]["prev_shown"], cf_sep["net"]["shown"]);
    assert!(rows(&cf).iter().all(|r| r.get("prev_shown").is_some()));

    let csv = call(
        b,
        "books_report_csv",
        json!({ "kind": "bs", "period": "2026-10", "compare_to": "previous" }),
    );
    let header = csv["csv"]
        .as_str()
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .to_string();
    assert_eq!(
        header, r#"Section,Number,Account,"As of Oct 31, 2026","As of Sep 30, 2026""#,
        "two amount columns"
    );
    assert_eq!(csv["file_name"], "books-bs-2026-10-vs-2026-09.csv");
}

#[test]
fn comparing_and_tagging_are_refused_where_they_do_not_apply() {
    let (_dir, mut books) = seeded();
    let b = &mut books;
    assert_eq!(
        refused(
            b,
            json!({ "kind": "tb", "period": "2026-10", "compare_to": "previous" })
        ),
        "invalid_args"
    );
    assert_eq!(
        refused(b, json!({ "kind": "ag", "compare_to": "last_year" })),
        "invalid_args"
    );
    assert_eq!(
        refused(b, json!({ "kind": "pl", "compare_to": "budget" })),
        "invalid_args"
    );
    let tag = call(
        b,
        "books_app_tag_create",
        json!({ "client_request_id": "rc-tag-0001", "kind": "department", "name": "Synthetic Kitchen" }),
    )["tag"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(
        refused(b, json!({ "kind": "bs", "tag": tag })),
        "invalid_args"
    );
    let accounts = call(b, "books_accounts_list", json!({}));
    let account = accounts.to_string();
    let acct = account
        .split('"')
        .find(|s| books_store::plumbing::is_ticket(s))
        .unwrap()
        .to_string();
    let code = refused(b, json!({ "kind": "pl", "tag": acct }));
    assert!(code == "not_found" || code == "invalid_args", "{code}");
    let code = refused(b, json!({ "kind": "pl", "tag": "no-such-tag" }));
    assert!(code == "not_found" || code == "invalid_args", "{code}");
}

#[test]
fn a_tag_narrows_the_pl_and_cash_flow_to_its_entries() {
    let (_dir, mut books) = seeded();
    let b = &mut books;
    let before = call(
        b,
        "books_report",
        json!({ "kind": "pl", "period": "2026-10" }),
    );
    let tag = call(
        b,
        "books_app_tag_create",
        json!({ "client_request_id": "rc-tag-0002", "kind": "department", "name": "Synthetic Kitchen" }),
    )["tag"]
        .as_str()
        .unwrap()
        .to_string();
    call(
        b,
        "books_app_post_entry",
        json!({ "client_request_id": "rc-spend-0001", "form": "money_out", "date": "2026-10-20", "amount": "1234.00", "account": "11000", "category": "66100", "payee": "Synthetic Landlord", "department": tag }),
    );
    let pl = call(
        b,
        "books_report",
        json!({ "kind": "pl", "period": "2026-10", "tag": tag }),
    );
    assert_eq!(pl["tag"]["ticket"], tag.as_str());
    assert_eq!(pl["tag"]["kind"], "department");
    assert_eq!(pl["tag"]["name"], "Synthetic Kitchen");
    let shown: Vec<_> = rows(&pl)
        .iter()
        .filter(|r| r["shown"] != "0.00")
        .cloned()
        .cloned()
        .collect();
    assert_eq!(shown.len(), 1, "{pl}");
    assert_eq!(shown[0]["name"], "Rent");
    assert_eq!(shown[0]["shown"], "1,234.00");
    assert_eq!(pl["totals"][0]["shown"], "0.00");
    assert_eq!(pl["net"]["shown"], "\u{2212}1,234.00");

    let after = call(
        b,
        "books_report",
        json!({ "kind": "pl", "period": "2026-10" }),
    );
    assert_eq!(after["totals"][0], before["totals"][0]);
    assert_eq!(after["net"]["prev_shown"], before["net"]["prev_shown"]);
    assert_eq!(after["net"]["shown"], "249,917.00");

    let cf = call(
        b,
        "books_report",
        json!({ "kind": "cf", "period": "2026-10", "tag": tag }),
    );
    assert_eq!(
        cf["tag_note"],
        "Cash at the start and end is for the whole book."
    );
    assert_eq!(rows(&cf).len(), 2, "{cf}");
    assert_eq!(cf["net"]["label"], "Net cash movement");
    let cf_row = |report: &Value, name: &str| -> Value {
        rows(report)
            .into_iter()
            .find(|r| r["name"] == name)
            .cloned()
            .unwrap_or_else(|| panic!("no {name} row in {report}"))
    };
    let cf_in = "Money in from customers and others";
    let cf_out = "Money out for running the business";
    assert_eq!(cf_row(&cf, cf_in)["shown"], "0.00", "{cf}");
    assert_eq!(cf_row(&cf, cf_out)["shown"], "\u{2212}1,234.00", "{cf}");
    assert_eq!(cf["net"]["shown"], "\u{2212}1,234.00");

    let whole = call(
        b,
        "books_report",
        json!({ "kind": "cf", "period": "2026-10" }),
    );
    assert_ne!(cf_row(&whole, cf_in)["shown"], cf_row(&cf, cf_in)["shown"]);
    assert_ne!(
        cf_row(&whole, cf_out)["shown"],
        cf_row(&cf, cf_out)["shown"]
    );

    let csv = call(
        b,
        "books_report_csv",
        json!({ "kind": "pl", "period": "2026-10", "tag": tag, "compare_to": "none" }),
    );
    let name = csv["file_name"].as_str().unwrap();
    assert_eq!(
        name,
        "books-pl-2026-10-vs-2026-09.csv".replace("-vs-2026-09", "-dept")
    );
    assert!(!name.contains(&tag) && !name.contains("tkt"), "{name}");
}
