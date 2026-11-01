//! M10 parity (unpaid-1): an owed document settles in parts. Each part is
//! one posted entry and one payment row; voiding a payment's entry puts
//! the balance back. Synthetic book, people and amounts only.

use books_mini_app::service::{Books, Origin};
use books_mini_app::tools;
use books_store::{Paths, TempDir};
use serde_json::{Map, Value, json};

fn raw(books: &mut Books, name: &str, args: Value) -> Value {
    let args: Map<String, Value> = args.as_object().unwrap().clone();
    tools::call(name, &args, Origin::App, books).unwrap()
}

fn call(books: &mut Books, name: &str, args: Value) -> Value {
    let out = raw(books, name, args);
    assert_eq!(out["isError"], false, "{name}: {out}");
    out["structuredContent"].clone()
}

fn refused(books: &mut Books, name: &str, args: Value) -> (String, String) {
    let out = raw(books, name, args);
    assert_eq!(out["isError"], true, "{name}: {out}");
    let s = &out["structuredContent"];
    (
        s["code"].as_str().unwrap().to_string(),
        s["message"].as_str().unwrap_or_default().to_string(),
    )
}

fn book(dir: &TempDir, basis: &str) -> Books {
    books_core::date::pin_today(books_core::date::Date::new(2026, 10, 29));
    let mut books = Books::new(Paths::in_dir(dir.0.join("store")));
    call(
        &mut books,
        "books_app_book_create",
        json!({ "client_request_id": format!("part-book-{basis}"), "label": "Partial Test", "basis": basis }),
    );
    books
}

fn pay_rows(b: &mut Books) -> Value {
    call(b, "books_owed_list", json!({ "side": "pay" }))
}

fn payments(b: &mut Books, owed: &Value) -> Value {
    call(b, "books_owed_list", json!({ "side": "pay", "owed": owed }))["detail"].clone()
}

/// The Rail badge's Unpaid count.
fn badge(b: &mut Books) -> Value {
    call(b, "books_book_get", json!({}))["unpaid"].clone()
}

/// The book's only document's stored status, and every entry in the store.
fn stored(b: &mut Books) -> (String, i64) {
    let conn = b.store().unwrap().conn();
    let status = conn
        .query_row("SELECT status FROM unpaid", [], |r| r.get(0))
        .unwrap();
    let entries = conn
        .query_row("SELECT count(*) FROM entry", [], |r| r.get(0))
        .unwrap();
    (status, entries)
}

#[test]
fn a_bill_settles_in_parts_and_keeps_its_payment_history() {
    let dir = TempDir::new("owed-part");
    let mut books = book(&dir, "cash");
    let b = &mut books;
    let bill = call(
        b,
        "books_app_owed_create",
        json!({ "client_request_id": "part-owed-0001", "side": "pay", "person_name": "Synthetic Vendor", "amount": "1000.00", "category": "67200", "issued": "2026-09-01", "reference": "Bill SYN-0001" }),
    );
    let owed = bill["owed"].clone();
    assert_eq!(badge(b), 1);
    let settle = json!({ "client_request_id": "part-settle-01", "owed": owed, "account": "11000", "amount": "400.00", "on": "2026-10-05" });
    let first = call(b, "books_app_owed_settle", settle.clone());
    assert_eq!(first["status"], "part_paid");
    assert_eq!(first["balance"], "600.00");
    assert_eq!(first["balance_shown"], "600.00");
    assert_eq!(stored(b).0, "open");
    assert_eq!(badge(b), 1);

    let list = pay_rows(b);
    let rows = list["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["amount_shown"], "1,000.00");
    assert_eq!(rows[0]["paid_shown"], "400.00");
    assert_eq!(rows[0]["balance_shown"], "600.00");
    assert_eq!(rows[0]["state"], "part_paid");
    assert_eq!(list["total_shown"], "600.00");
    let d = payments(b, &owed);
    assert_eq!(d["state"], "part_paid");
    assert_eq!(d["payments"][0]["amount_shown"], "400.00");
    assert_eq!(d["payments"][0]["voided"], false);
    assert_eq!(d["payments"][0]["entry"], first["entry"]);

    // a repeat returns the stored result and adds no payment
    let again = call(b, "books_app_owed_settle", settle);
    assert_eq!(again, first);
    assert_eq!(payments(b, &owed)["payments"].as_array().unwrap().len(), 1);

    // refusals carry no amounts
    for (args, want) in [
        (
            json!({ "client_request_id": "part-settle-zero", "owed": owed, "account": "11000", "amount": "0.00", "on": "2026-10-06" }),
            "bad_amount",
        ),
        (
            json!({ "client_request_id": "part-settle-over", "owed": owed, "account": "11000", "amount": "600.01", "on": "2026-10-06" }),
            "over_balance",
        ),
    ] {
        let (code, message) = refused(b, "books_app_owed_settle", args);
        assert_eq!(code, want);
        assert!(!message.chars().any(|c| c.is_ascii_digit()), "{message}");
    }

    // a closed month refuses the settle and writes no payment or entry
    call(
        b,
        "books_app_month_close",
        json!({ "client_request_id": "part-close-09", "month": "2026-09" }),
    );
    let entries_before = stored(b).1;
    let (code, _) = refused(
        b,
        "books_app_owed_settle",
        json!({ "client_request_id": "part-settle-closed", "owed": owed, "account": "11000", "amount": "100.00", "on": "2026-09-20" }),
    );
    assert_eq!(code, "period_closed");
    assert_eq!(payments(b, &owed)["payments"].as_array().unwrap().len(), 1);
    assert_eq!(pay_rows(b)["total_shown"], "600.00");
    assert_eq!(stored(b).1, entries_before);

    // no amount: the rest
    let rest = call(
        b,
        "books_app_owed_settle",
        json!({ "client_request_id": "part-settle-02", "owed": owed, "account": "11000", "on": "2026-10-10" }),
    );
    assert_eq!(rest["status"], "paid");
    assert_eq!(rest["balance_shown"], "0.00");
    assert_eq!(pay_rows(b)["rows"].as_array().unwrap().len(), 0);
    assert_eq!(stored(b).0, "paid");
    assert_eq!(badge(b), 0);
    let d = payments(b, &owed);
    assert_eq!(d["state"], "paid");
    assert_eq!(d["payments"].as_array().unwrap().len(), 2);
    assert_eq!(d["payments"][1]["amount_shown"], "600.00");
    let (code, message) = refused(
        b,
        "books_app_owed_settle",
        json!({ "client_request_id": "part-settle-03", "owed": owed, "account": "11000", "on": "2026-10-11" }),
    );
    assert_eq!(code, "already_settled");
    assert!(!message.chars().any(|c| c.is_ascii_digit()), "{message}");

    // voiding the last payment in the Ledger puts its part back
    call(
        b,
        "books_app_void_entry",
        json!({ "client_request_id": "part-void-02", "entry": rest["entry"] }),
    );
    let list = pay_rows(b);
    assert_eq!(list["rows"][0]["balance_shown"], "600.00");
    assert_eq!(list["rows"][0]["state"], "part_paid");
    assert_eq!(list["total_shown"], "600.00");
    assert_eq!(payments(b, &owed)["payments"][1]["voided"], true);
    // the document is paid in its stored status, but the badge counts it again
    assert_eq!(stored(b).0, "paid");
    assert_eq!(badge(b), 1);
    // and its printout says open again, not settled
    let doc = raw(b, "books_owed_document", json!({ "owed": owed }));
    let page = doc["content"][0]["resource"]["text"].as_str().unwrap();
    assert!(page.contains("<td>Open</td>"), "{page}");
    assert!(!page.contains("Settled"), "{page}");
    let (code, message) = refused(
        b,
        "books_app_owed_cancel",
        json!({ "client_request_id": "part-cancel-01", "owed": owed }),
    );
    assert_eq!(code, "has_payments");
    assert!(!message.chars().any(|c| c.is_ascii_digit()), "{message}");

    // a paid document owed again after a void settles again
    let again = call(
        b,
        "books_app_owed_settle",
        json!({ "client_request_id": "part-settle-04", "owed": owed, "account": "11000", "on": "2026-10-12" }),
    );
    assert_eq!(again["status"], "paid");
    assert_eq!(again["balance_shown"], "0.00");
    assert_eq!(pay_rows(b)["rows"].as_array().unwrap().len(), 0);
    assert_eq!(badge(b), 0);
    let d = payments(b, &owed);
    let history = d["payments"].as_array().unwrap();
    assert_eq!(history.len(), 3);
    assert_eq!(history.iter().filter(|p| p["voided"] == true).count(), 1);
    assert_eq!(history[2]["amount_shown"], "600.00");

    // the assistant's list carries no tickets for payments
    let args: Map<String, Value> = json!({ "side": "pay", "owed": owed })
        .as_object()
        .unwrap()
        .clone();
    let out = tools::call("books_owed_list", &args, Origin::Agent, b).unwrap();
    let p = &out["structuredContent"]["detail"]["payments"][0];
    assert_eq!(p["amount_shown"], "400.00");
    assert!(
        p.get("entry").is_none() && p.get("account").is_none(),
        "{p}"
    );
}

#[test]
fn an_accrual_part_settle_clears_that_much_of_the_receivable() {
    let dir = TempDir::new("owed-part-acc");
    let mut books = book(&dir, "accrual");
    let b = &mut books;
    let inv = call(
        b,
        "books_app_owed_create",
        json!({ "client_request_id": "part-owed-acc1", "side": "collect", "person_name": "Synthetic Customer", "amount": "1000.00", "category": "41000", "issued": "2026-10-01", "reference": "Invoice SYN-0001" }),
    );
    let ar = |b: &mut Books| -> Value {
        let tb = call(
            b,
            "books_report",
            json!({ "kind": "tb", "period": "2026-10" }),
        );
        assert_eq!(tb["balanced"], true);
        tb["sections"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|s| s["rows"].as_array().unwrap())
            .find(|r| r["name"] == "Accounts Receivable - Gross")
            .map(|r| r["debit_shown"].clone())
            .unwrap_or(Value::Null)
    };
    assert_eq!(ar(b), "1,000.00");
    let done = call(
        b,
        "books_app_owed_settle",
        json!({ "client_request_id": "part-settle-acc1", "owed": inv["owed"], "account": "11000", "amount": "300.00", "on": "2026-10-10" }),
    );
    assert_eq!(done["balance_shown"], "700.00");
    assert_eq!(ar(b), "700.00");
}
