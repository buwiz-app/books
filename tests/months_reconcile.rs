//! M4 on the golden book (synthetic): October matches the bank, a closed
//! month refuses changes, reopening needs a reason, and "not sure yet"
//! lines take a category. Exit: "October matches the bank in the demo; a
//! closed month refuses changes."

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

fn run(books: &mut Books, name: &str, args: Value) -> Value {
    let args: Map<String, Value> = args.as_object().unwrap().clone();
    tools::call(name, &args, Origin::App, books).unwrap()
}

fn ok(books: &mut Books, name: &str, args: Value) -> Value {
    let out = run(books, name, args);
    assert_eq!(out["isError"], false, "{name}: {out}");
    out["structuredContent"].clone()
}

fn refused(books: &mut Books, name: &str, args: Value) -> String {
    let out = run(books, name, args);
    assert_eq!(out["isError"], true, "{name}: {out}");
    out["structuredContent"]["code"]
        .as_str()
        .unwrap()
        .to_string()
}

fn pl_row(pl: &Value, name: &str) -> Option<String> {
    pl["sections"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|s| s["rows"].as_array().unwrap())
        .find(|r| r["name"] == name)
        .map(|r| r["shown"].as_str().unwrap().to_string())
}

#[test]
fn m4_october_matches_the_bank_and_a_closed_month_refuses_changes() {
    books_core::date::pin_today(books_core::date::Date::new(2026, 10, 29));
    let dir = TempDir::new("m4");
    let mut books = Books::new(Paths::in_dir(dir.0.join("store")));
    let b = &mut books;
    let demo_book: Value = serde_json::from_str(&std::fs::read_to_string(DEMO).unwrap()).unwrap();
    demo::seed(b, &demo_book).unwrap();

    // import the statement (the app sends the picked file's bytes as base64url)
    let csv = std::fs::read(STATEMENT).unwrap();
    let imported = ok(
        b,
        "books_app_statement_import",
        json!({ "client_request_id": "m4-import-0001", "account": "11010", "csv_base64": books_mini_app::b64::encode(&csv), "file_name": "bank-4021-2026-10.csv" }),
    );
    assert_eq!(
        (imported["month"].as_str(), imported["lines"].as_i64()),
        (Some("2026-10"), Some(11))
    );
    let view = ok(
        b,
        "books_statement_get",
        json!({ "account": "11010", "month": "2026-10" }),
    );
    let st = &view["statement"];
    let kinds: Vec<&str> = st["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["match"].as_str().unwrap())
        .collect();
    assert_eq!(
        kinds,
        ["m", "m", "m", "s", "m", "m", "m", "m", "m", "m", "x"],
        "the fee is a day off; the interest is missing"
    );
    assert_eq!(
        (
            st["closing_shown"].as_str(),
            st["books_shown"].as_str(),
            st["difference_shown"].as_str()
        ),
        (Some("412,392.18"), Some("412,350.00"), Some("42.18"))
    );
    assert_eq!(st["can_finish"], false);
    assert_eq!(
        refused(
            b,
            "books_app_statement_finish",
            json!({ "client_request_id": "m4-finish-001", "account": "11010", "month": "2026-10" })
        ),
        "lines_open"
    );

    let accepted = ok(
        b,
        "books_app_statement_accept",
        json!({ "client_request_id": "m4-accept-001", "account": "11010", "month": "2026-10" }),
    );
    assert_eq!(accepted["accepted"], 1);
    let interest = st["rows"][10]["ticket"].as_str().unwrap().to_string();
    let added = ok(
        b,
        "books_app_statement_add_line",
        json!({ "client_request_id": "m4-add-000001", "account": "11010", "month": "2026-10", "line": interest }),
    );
    let entry = ok(b, "books_entry_get", json!({ "entry": added["entry"] }));
    assert_eq!(
        (
            entry["category"].as_str(),
            entry["amount_shown"].as_str(),
            entry["date"].as_str()
        ),
        (Some("Interest Income"), Some("+42.18"), Some("2026-10-31"))
    );
    let view = ok(
        b,
        "books_statement_get",
        json!({ "account": "11010", "month": "2026-10" }),
    );
    assert_eq!(view["statement"]["difference_shown"], "0.00");
    assert_eq!(view["statement"]["can_finish"], true);
    ok(
        b,
        "books_app_statement_finish",
        json!({ "client_request_id": "m4-finish-002", "account": "11010", "month": "2026-10" }),
    );
    let grid = ok(b, "books_reconcile_grid", json!({}));
    let bank = grid["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == "Bank •••• 4021")
        .unwrap();
    assert_eq!(
        bank["cells"][3]["state"], "done",
        "October matches the bank"
    );
    assert_eq!(
        bank["cells"][2]["state"], "todo",
        "September has entries and no statement"
    );

    // close October: a draft dated in it comes first
    assert_eq!(
        refused(
            b,
            "books_app_month_close",
            json!({ "client_request_id": "m4-close-0001", "month": "2026-10" })
        ),
        "drafts_open"
    );
    let ledger = ok(
        b,
        "books_ledger_list",
        json!({ "month": "2026-10", "filter": "draft" }),
    );
    let draft = ledger["rows"][0]["ticket"].clone();
    ok(
        b,
        "books_app_discard_entry",
        json!({ "client_request_id": "m4-discard-01", "entry": draft }),
    );
    ok(
        b,
        "books_app_month_close",
        json!({ "client_request_id": "m4-close-0002", "month": "2026-10" }),
    );
    // a closed month refuses changes
    assert_eq!(
        refused(
            b,
            "books_app_post_entry",
            json!({ "client_request_id": "m4-post-00001", "form": "money_out", "date": "2026-10-20", "amount": "10.00", "account": "11010", "category": "68300" })
        ),
        "period_closed"
    );
    let rent = ok(b, "books_ledger_list", json!({ "month": "2026-10" }))["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["category"] == "Rent")
        .unwrap()["ticket"]
        .clone();
    assert_eq!(
        refused(
            b,
            "books_app_void_entry",
            json!({ "client_request_id": "m4-void-00001", "entry": rent })
        ),
        "period_closed"
    );
    assert_eq!(
        refused(
            b,
            "books_app_month_reopen",
            json!({ "client_request_id": "m4-reopen-001", "month": "2026-10" })
        ),
        "reason_needed"
    );
    ok(
        b,
        "books_app_month_reopen",
        json!({ "client_request_id": "m4-reopen-002", "month": "2026-10", "reason": "MEMO-SYNTH two bank lines to sort" }),
    );
    let settings = ok(b, "books_settings_get", json!({}));
    let ev: Vec<&str> = settings["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["event"].as_str().unwrap())
        .collect();
    assert_eq!(ev, ["reopen", "close"]);
    assert_eq!(settings["months"][0]["short"], "Oct 2026");
    assert_eq!(settings["months"][0]["closed"], false);

    // the two "not sure yet" bank lines take their categories
    let review = ok(
        b,
        "books_ledger_list",
        json!({ "month": "2026-10", "filter": "review" }),
    );
    assert_eq!(review["counts"]["review"], 2);
    for row in review["rows"].as_array().unwrap() {
        let category = if row["amount_shown"] == "\u{2212}150.00" {
            "68300"
        } else {
            "66200"
        };
        ok(
            b,
            "books_app_recategorize",
            json!({ "client_request_id": format!("m4-recat-{category}"), "entry": row["ticket"], "category": category }),
        );
    }
    let after = ok(b, "books_ledger_list", json!({ "month": "2026-10" }));
    assert_eq!(after["counts"]["review"], 0);
    let pl = ok(
        b,
        "books_report",
        json!({ "kind": "pl", "period": "2026-10" }),
    );
    assert_eq!(pl_row(&pl, "Utilities").as_deref(), Some("2,499.00"));
    assert_eq!(pl_row(&pl, "Bank Fees").as_deref(), Some("150.00"));
    assert_eq!(
        pl_row(&pl, "Uncategorized Expense"),
        None,
        "nothing is left to review in October or September"
    );
    assert_eq!(
        pl["net"]["shown"], "251,193.18",
        "the interest is October income; categories move nothing else"
    );
    let tb = ok(
        b,
        "books_report",
        json!({ "kind": "tb", "period": "2026-10" }),
    );
    assert_eq!(tb["balanced"], true);

    // settings: a basis change waits for an empty year; a rename is fine
    assert_eq!(
        refused(
            b,
            "books_app_book_update",
            json!({ "client_request_id": "m4-basis-0001", "basis": "accrual" })
        ),
        "basis_locked"
    );
    let renamed = ok(
        b,
        "books_app_book_update",
        json!({ "client_request_id": "m4-rename-001", "label": "Buwiz Trading Co" }),
    );
    assert_eq!(renamed["label"], "Buwiz Trading Co");
    assert_eq!(
        refused(
            b,
            "books_app_default_set",
            json!({ "client_request_id": "m4-default-01", "slot": "unpaid_bills", "account": "21000" })
        ),
        "accrual_account"
    );
    ok(
        b,
        "books_app_default_set",
        json!({ "client_request_id": "m4-default-02", "slot": "rent", "account": "66100" }),
    );

    // closing, reopening, matching and categories are the user's clicks
    for name in [
        "books_app_month_close",
        "books_app_month_reopen",
        "books_app_statement_finish",
        "books_app_recategorize",
    ] {
        assert_eq!(
            tools::find(name).unwrap().reach,
            tools::Reach::App,
            "{name}"
        );
    }
}
