//! Monthly repeats of unpaid documents (M10 parity, unpaid-3): the user
//! adds each month's copy from the app; nothing writes one by itself.
//! Synthetic people, references and amounts only.

use std::sync::Mutex;

use books_core::date::{Date, pin_today};
use books_mini_app::b64;
use books_mini_app::jsonrpc::Server;
use books_mini_app::manifest::{self, Source};
use books_mini_app::service::{Books, Origin};
use books_mini_app::tools;
use books_store::{Paths, TempDir};
use ed25519_dalek::SigningKey;
use serde_json::{Map, Value, json};

/// `pin_today` is process-wide; the tests here take turns.
static DAY: Mutex<()> = Mutex::new(());

fn raw(books: &mut Books, name: &str, args: Value, origin: Origin) -> Value {
    let args: Map<String, Value> = args.as_object().unwrap().clone();
    tools::call(name, &args, origin, books).unwrap()
}

fn call(books: &mut Books, name: &str, args: Value) -> Value {
    let out = raw(books, name, args, Origin::App);
    assert_eq!(out["isError"], false, "{name}: {out}");
    out["structuredContent"].clone()
}

fn refused(books: &mut Books, name: &str, args: Value, origin: Origin) -> Value {
    let out = raw(books, name, args, origin);
    assert_eq!(out["isError"], true, "{name}: {out}");
    out["structuredContent"]["code"].clone()
}

fn today(y: i32, m: u8, d: u8) {
    pin_today(Date::new(y, m, d));
}

/// A book with one bill to pay issued Jan 31, due Feb 10.
fn book(dir: &TempDir, basis: &str) -> (Books, String) {
    let mut books = Books::new(Paths::in_dir(dir.0.join("store")));
    call(
        &mut books,
        "books_app_book_create",
        json!({ "client_request_id": "rep-book-0001", "label": "Repeat Test", "basis": basis }),
    );
    let bill = call(
        &mut books,
        "books_app_owed_create",
        json!({ "client_request_id": "rep-owed-0001", "side": "pay", "person_name": "Synthetic Vendor", "amount": "3200.00", "category": "67200", "issued": "2026-01-31", "due": "2026-02-10", "reference": "Bill SV-0001", "memo": "Synthetic memo" }),
    );
    (books, bill["owed"].as_str().unwrap().to_string())
}

fn repeat(books: &mut Books, rid: &str, owed: &str, action: &str) -> Value {
    call(
        books,
        "books_app_owed_repeat",
        json!({ "client_request_id": rid, "owed": owed, "action": action }),
    )
}

fn list(books: &mut Books) -> Value {
    call(books, "books_owed_list", json!({ "side": "pay" }))
}

/// (issued, due, amount, category, memo, repeat_of, repeat_month) of a document.
type Row = (
    String,
    Option<String>,
    i64,
    i64,
    Option<String>,
    Option<i64>,
    Option<String>,
);

fn row(books: &mut Books, ticket: &str) -> Row {
    let store = books.store().unwrap();
    let (_, id) = books_store::plumbing::subject(store.conn(), ticket)
        .unwrap()
        .unwrap();
    store
        .conn()
        .query_row(
            "SELECT issued, due, amount, category_account_id, memo, repeat_of, repeat_month FROM unpaid WHERE unpaid_id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
        )
        .unwrap()
}

fn count(books: &mut Books) -> i64 {
    books
        .store()
        .unwrap()
        .conn()
        .query_row("SELECT COUNT(*) FROM unpaid", [], |r| r.get(0))
        .unwrap()
}

#[test]
fn a_monthly_bill_offers_this_months_copy_and_the_user_adds_it() {
    let _day = DAY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    today(2026, 2, 15);
    let dir = TempDir::new("owed-repeat");
    let (mut books, owed) = book(&dir, "cash");
    let b = &mut books;

    let before = list(b);
    assert_eq!(before["rows"][0]["repeat"], Value::Null);
    assert_eq!(before["repeats"]["count"], 0);

    let on = repeat(b, "rep-start-001", &owed, "start");
    assert_eq!(
        (on["owed"].as_str(), on["repeat"].as_str()),
        (Some(owed.as_str()), Some("monthly"))
    );
    assert!(on.get("added").is_none());

    let due = list(b);
    assert_eq!(due["rows"][0]["repeat"], "monthly");
    assert_eq!(due["repeats"]["count"], 1);
    let first = &due["repeats"]["due"][0];
    assert_eq!(
        (
            first["owed"].as_str(),
            first["issued"].as_str(),
            first["due"].as_str(),
            first["amount_shown"].as_str()
        ),
        (
            Some(owed.as_str()),
            Some("2026-02-28"),
            Some("2026-03-10"),
            Some("3,200.00")
        )
    );
    assert_eq!(
        (first["who"].as_str(), first["doc"].as_str()),
        (Some("Synthetic Vendor"), Some("Bill SV-0001"))
    );
    // the assistant sees the due list without names or references
    let seen = raw(
        b,
        "books_owed_list",
        json!({ "side": "pay" }),
        Origin::Agent,
    );
    let seen = &seen["structuredContent"]["repeats"]["due"][0];
    assert!(
        seen.get("who").is_none() && seen.get("doc").is_none(),
        "{seen}"
    );
    // the other side has nothing due
    let collect = call(b, "books_owed_list", json!({ "side": "collect" }));
    assert_eq!(collect["repeats"]["count"], 0);

    let added = repeat(b, "rep-add-0001", &owed, "add");
    let copy = added["added"].as_str().unwrap().to_string();
    assert_ne!(copy, owed);
    assert_eq!(added["repeat"], "monthly");
    let (t_issued, _, t_amount, t_category, t_memo, _, _) = row(b, &owed);
    assert_eq!(
        (t_issued.as_str(), t_memo.as_deref()),
        ("2026-01-31", Some("Synthetic memo"))
    );
    let (issued, due_on, amount, category, memo, of, month) = row(b, &copy);
    assert_eq!(
        (issued.as_str(), due_on.as_deref()),
        ("2026-02-28", Some("2026-03-10"))
    );
    assert_eq!((amount, category, memo), (t_amount, t_category, None));
    assert_eq!(month.as_deref(), Some("2026-02"));
    assert!(of.is_some());

    let after = list(b);
    assert_eq!(after["repeats"]["count"], 0);
    assert_eq!(after["counts"]["pay"], 2);
    let copy_row = after["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["ticket"] == copy.as_str())
        .unwrap();
    assert_eq!(copy_row["repeat"], Value::Null, "a copy does not repeat");

    // the same month again is refused; a repeated request returns its answer
    let rows = count(b);
    assert_eq!(
        refused(
            b,
            "books_app_owed_repeat",
            json!({ "client_request_id": "rep-add-0002", "owed": owed, "action": "add", "month": "2026-02" }),
            Origin::App
        ),
        "already_added"
    );
    let again = repeat(b, "rep-add-0001", &owed, "add");
    assert_eq!(again["added"], copy.as_str());
    assert_eq!(count(b), rows);

    // a month after today's, or not after the template's, is refused
    for month in ["2026-03", "2026-01", "2025-12"] {
        assert_eq!(
            refused(
                b,
                "books_app_owed_repeat",
                json!({ "client_request_id": format!("rep-bad-{month}"), "owed": owed, "action": "add", "month": month }),
                Origin::App
            ),
            "bad_month",
            "{month}"
        );
    }
    assert_eq!(
        refused(
            b,
            "books_app_owed_repeat",
            json!({ "client_request_id": "rep-bad-text", "owed": owed, "action": "add", "month": "Feb" }),
            Origin::App
        ),
        "invalid_args"
    );
    // only the original repeats
    assert_eq!(
        refused(
            b,
            "books_app_owed_repeat",
            json!({ "client_request_id": "rep-start-cp", "owed": copy, "action": "start" }),
            Origin::App
        ),
        "is_copy"
    );

    // stop: next month nothing is due
    let off = repeat(b, "rep-stop-0001", &owed, "stop");
    assert_eq!(off["repeat"], Value::Null);
    today(2026, 3, 15);
    assert_eq!(list(b)["repeats"]["count"], 0);
    // start again: March's copy is due, the 31st clamped to March's 31
    repeat(b, "rep-start-002", &owed, "start");
    let march = list(b);
    assert_eq!(march["repeats"]["due"][0]["issued"], "2026-03-31");
    // a cancelled template is never due
    call(
        b,
        "books_app_owed_cancel",
        json!({ "client_request_id": "rep-cancel-01", "owed": owed }),
    );
    assert_eq!(list(b)["repeats"]["count"], 0);
    assert_eq!(
        refused(
            b,
            "books_app_owed_repeat",
            json!({ "client_request_id": "rep-start-003", "owed": owed, "action": "start" }),
            Origin::App
        ),
        "not_open"
    );
    pin_today(None);
}

#[test]
fn an_accrual_copy_obeys_the_closed_month() {
    let _day = DAY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    today(2026, 3, 5);
    let dir = TempDir::new("owed-repeat-accrual");
    let (mut books, owed) = book(&dir, "accrual");
    let b = &mut books;
    repeat(b, "acc-start-001", &owed, "start");
    call(
        b,
        "books_app_month_close",
        json!({ "client_request_id": "acc-close-feb", "month": "2026-02" }),
    );
    let rows = count(b);
    assert_eq!(
        refused(
            b,
            "books_app_owed_repeat",
            json!({ "client_request_id": "acc-add-feb", "owed": owed, "action": "add", "month": "2026-02" }),
            Origin::App
        ),
        "period_closed"
    );
    assert_eq!(count(b), rows, "a refused copy leaves no row");
    // March is open: the copy posts its payable
    let added = call(
        b,
        "books_app_owed_repeat",
        json!({ "client_request_id": "acc-add-mar", "owed": owed, "action": "add" }),
    );
    assert!(added["added"].is_string());
    pin_today(None);
}

#[test]
fn the_repeat_tool_is_the_apps_alone() {
    let _day = DAY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    today(2026, 2, 15);
    let dir = TempDir::new("owed-repeat-reach");
    let (books, owed) = book(&dir, "cash");
    // through the Bot's door, as the assistant would call it
    let mut server = Server::new(Source::default(), books);
    let ts = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap();
    let seed = [11u8; 32];
    let public = b64::encode(SigningKey::from_bytes(&seed).verifying_key().as_bytes());
    let provision = json!({ "name": "buwiz.provision", "arguments": { "protocol": "2.0.0", "plugin": manifest::PLUGIN_ID, "key": { "alg": "ed25519", "seed": b64::encode(&seed) }, "pub": public } });
    let p = server
        .handle(&json!({ "jsonrpc": "2.0", "id": 0, "method": "tools/call", "params": provision }))
        .unwrap();
    assert_eq!(p["result"]["isError"], false, "{p}");
    let caller = json!({ "origin": "agent", "plugin": manifest::PLUGIN_ID, "ts": ts });
    let params = json!({ "name": "books_app_owed_repeat", "arguments": { "client_request_id": "rep-asst-001", "owed": owed, "action": "start" }, "_meta": { "buwiz/caller": caller } });
    let reply = server
        .handle(&json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": params }))
        .unwrap();
    assert_eq!(reply["result"]["isError"], true, "{reply}");
    assert_eq!(
        reply["result"]["structuredContent"]["code"],
        "permission_denied"
    );
    let tool = tools::find("books_app_owed_repeat").unwrap();
    assert_eq!((tool.reach, tool.write), (tools::Reach::App, true));
    let listed = tools::list()
        .into_iter()
        .find(|t| t["name"] == "books_app_owed_repeat")
        .unwrap();
    assert_eq!(listed["_meta"]["buwiz/audience"], json!(["app"]));
    pin_today(None);
}
