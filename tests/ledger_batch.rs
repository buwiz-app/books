//! ledger-4: `books_app_batch` posts, discards or recategorizes many
//! entries in one transaction, all or nothing, idempotent on its
//! client_request_id; only the app may call it. Synthetic data only.

use books_mini_app::service::{Books, Origin};
use books_mini_app::tools;
use books_store::{Paths, TempDir};
use serde_json::{Map, Value, json};

fn run(books: &mut Books, name: &str, origin: Origin, args: Value) -> Value {
    let args: Map<String, Value> = args.as_object().unwrap().clone();
    tools::call(name, &args, origin, books).unwrap()
}

fn ok(books: &mut Books, name: &str, args: Value) -> Value {
    let out = run(books, name, Origin::App, args);
    assert_eq!(out["isError"], false, "{name}: {out}");
    out["structuredContent"].clone()
}

fn refused(books: &mut Books, name: &str, origin: Origin, args: Value) -> Value {
    let out = run(books, name, origin, args);
    assert_eq!(out["isError"], true, "{name}: {out}");
    out["structuredContent"].clone()
}

fn setup(dir: &TempDir) -> Books {
    books_core::date::pin_today(books_core::date::Date::new(2026, 10, 29));
    let mut books = Books::new(Paths::in_dir(dir.0.join("store")));
    ok(
        &mut books,
        "books_app_book_create",
        json!({ "client_request_id": "batch-book-0001", "label": "Synthetic Shop" }),
    );
    books
}

/// An assistant draft of money out, for `category`.
fn draft(b: &mut Books, n: u32, date: &str, category: &str) -> String {
    let out = run(
        b,
        "books_entry_draft",
        Origin::Agent,
        json!({ "client_request_id": format!("batch-draft-{n:04}"), "form": "money_out", "date": date, "amount": format!("{}.00", 100 + n), "account": "11000", "category": category }),
    );
    assert_eq!(out["isError"], false, "{out}");
    out["structuredContent"]["entry"]
        .as_str()
        .unwrap()
        .to_string()
}

fn status(b: &mut Books, entry: &str) -> String {
    ok(b, "books_entry_get", json!({ "entry": entry }))["status"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

#[test]
fn batch_post_is_all_or_nothing() {
    let dir = TempDir::new("batch-post");
    let b = &mut setup(&dir);
    let three: Vec<String> = (1..=3)
        .map(|n| draft(b, n, &format!("2026-10-1{n}"), "67200"))
        .collect();
    let three: Vec<String> = three;
    let done = ok(
        b,
        "books_app_batch",
        json!({ "client_request_id": "batch-post-0001", "action": "post", "entries": three }),
    );
    assert_eq!(done["done"], 3);
    assert_eq!(done["entries"], json!(three));
    for e in &three {
        assert_eq!(status(b, e), "posted");
    }
    // posting a posted entry is the wrong status
    let again = refused(
        b,
        "books_app_batch",
        Origin::App,
        json!({ "client_request_id": "batch-post-0002", "action": "post", "entries": [three[0]] }),
    );
    assert_eq!(again["code"], "wrong_status");

    // one draft in a closed month: the batch is refused, naming it, and none post
    ok(
        b,
        "books_app_month_close",
        json!({ "client_request_id": "batch-close-0001", "month": "2026-09" }),
    );
    let open1 = draft(b, 4, "2026-10-20", "67200");
    let closed = draft(b, 5, "2026-09-20", "67200");
    let open2 = draft(b, 6, "2026-10-21", "67200");
    let no = ok(b, "books_entry_get", json!({ "entry": closed }))["no"]
        .as_str()
        .unwrap()
        .to_string();
    let r = refused(
        b,
        "books_app_batch",
        Origin::App,
        json!({ "client_request_id": "batch-post-0003", "action": "post", "entries": [open1, closed, open2] }),
    );
    assert_eq!(r["code"], "period_closed", "{r}");
    assert!(
        r["message"]
            .as_str()
            .unwrap()
            .starts_with(&format!("entry {no}")),
        "{r}"
    );
    for e in [&open1, &closed, &open2] {
        assert_eq!(status(b, e), "draft", "nothing posted");
    }
}

#[test]
fn batch_discard_and_recategorize_match_the_single_tools_and_repeat() {
    let dir = TempDir::new("batch-more");
    let b = &mut setup(&dir);
    // discard: the batch and the single tool leave the same status
    let one = draft(b, 1, "2026-10-05", "67200");
    let two = draft(b, 2, "2026-10-06", "67200");
    let single = draft(b, 3, "2026-10-07", "67200");
    let args = json!({ "client_request_id": "batch-disc-0001", "action": "discard", "entries": [one, two] });
    let first = run(b, "books_app_batch", Origin::App, args.clone());
    assert_eq!(first["structuredContent"]["done"], 2, "{first}");
    ok(
        b,
        "books_app_discard_entry",
        json!({ "client_request_id": "single-disc-001", "entry": single }),
    );
    for e in [&one, &two, &single] {
        assert_eq!(status(b, e), "discarded");
    }
    // a repeat returns the stored result and changes nothing
    let audit = |b: &mut Books| ok(b, "books_settings_get", json!({})).to_string().len();
    let before = audit(b);
    assert_eq!(run(b, "books_app_batch", Origin::App, args), first);
    assert_eq!(audit(b), before);

    // recategorize: "not sure yet" posted lines take a category
    let unsure: Vec<String> = (4..=6)
        .map(|n| draft(b, n, &format!("2026-10-1{n}"), "69999"))
        .collect();
    let single = unsure[2].clone();
    let pair = vec![unsure[0].clone(), unsure[1].clone()];
    ok(
        b,
        "books_app_batch",
        json!({ "client_request_id": "batch-post-0010", "action": "post", "entries": unsure }),
    );
    let without = refused(
        b,
        "books_app_batch",
        Origin::App,
        json!({ "client_request_id": "batch-recat-001", "action": "recategorize", "entries": pair }),
    );
    assert_eq!(without["code"], "invalid_args");
    let recat = ok(
        b,
        "books_app_batch",
        json!({ "client_request_id": "batch-recat-002", "action": "recategorize", "entries": pair, "category": "68300" }),
    );
    assert_eq!(recat["done"], 2);
    ok(
        b,
        "books_app_recategorize",
        json!({ "client_request_id": "single-recat-01", "entry": single, "category": "68300" }),
    );
    let review = ok(
        b,
        "books_ledger_list",
        json!({ "month": "2026-10", "filter": "review" }),
    );
    assert_eq!(review["counts"]["review"], 0, "all three have a category");
    let pl = ok(
        b,
        "books_report",
        json!({ "kind": "pl", "period": "2026-10" }),
    )
    .to_string();
    assert!(!pl.contains("Not sure yet"), "nothing left on not sure yet");
}

#[test]
fn batch_refuses_the_assistant_and_bad_lists() {
    let dir = TempDir::new("batch-refuse");
    let b = &mut setup(&dir);
    let e = draft(b, 1, "2026-10-05", "67200");
    let agent = refused(
        b,
        "books_app_batch",
        Origin::Agent,
        json!({ "client_request_id": "batch-agent-001", "action": "post", "entries": [e] }),
    );
    assert_eq!(agent["code"], "permission_denied");
    assert!(
        agent["message"]
            .as_str()
            .unwrap()
            .contains("needs the user's click")
    );
    assert_eq!(status(b, &e), "draft");
    let many: Vec<String> = (0..51).map(|_| e.clone()).collect();
    let unique: Vec<String> = (0..51).map(|n| format!("{e}{n}")).collect();
    for (n, entries) in [json!([]), json!(unique), json!(many), json!([e, e])]
        .into_iter()
        .enumerate()
    {
        let r = refused(
            b,
            "books_app_batch",
            Origin::App,
            json!({ "client_request_id": format!("batch-bad-{n:04}"), "action": "post", "entries": entries }),
        );
        assert_eq!(r["code"], "invalid_args", "{r}");
    }
    let r = refused(
        b,
        "books_app_batch",
        Origin::App,
        json!({ "client_request_id": "batch-bad-act1", "action": "void", "entries": [e] }),
    );
    assert_eq!(r["code"], "invalid_args");
    // jsonrpc gates the tool to the app too
    assert_eq!(
        tools::find("books_app_batch").unwrap().reach,
        tools::Reach::App
    );
}

/// ledger-4 adds one window tool: 57 at 35b1019, now 58, under the 64 cap.
#[test]
fn the_window_grants_books_app_batch_within_the_cap() {
    let m = books_mini_app::manifest::unsigned(books_mini_app::manifest::RELEASE_KEY_ID);
    let window = m["apps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == "books")
        .unwrap()
        .clone();
    let granted = window["permissions"]["tools"].as_array().unwrap();
    assert!(granted.iter().any(|t| t == "books_app_batch"));
    assert_eq!(
        granted.len(),
        58,
        "57 before ledger-4, plus books_app_batch"
    );
}
