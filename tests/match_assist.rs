//! Match assist from matched history (synthetic): a user's accept teaches
//! which payee a bank description is, one line can be matched on a click,
//! and an added line takes the category its description used before.

use books_mini_app::demo;
use books_mini_app::service::{Books, Origin};
use books_mini_app::tools;
use books_store::{Paths, TempDir};
use serde_json::{Map, Value, json};

const DEMO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/demo/buwiz-trading-2026.json"
);

fn run(books: &mut Books, name: &str, args: Value, origin: Origin) -> Value {
    let args: Map<String, Value> = args.as_object().unwrap().clone();
    tools::call(name, &args, origin, books).unwrap()
}

fn ok(books: &mut Books, name: &str, args: Value) -> Value {
    let out = run(books, name, args, Origin::App);
    assert_eq!(out["isError"], false, "{name}: {out}");
    out["structuredContent"].clone()
}

fn refused(books: &mut Books, name: &str, args: Value) -> String {
    let out = run(books, name, args, Origin::App);
    assert_eq!(out["isError"], true, "{name}: {out}");
    out["structuredContent"]["code"]
        .as_str()
        .unwrap()
        .to_string()
}

fn book(tag: &str) -> (TempDir, Books) {
    books_core::date::pin_today(books_core::date::Date::new(2026, 12, 31));
    let dir = TempDir::new(tag);
    let mut books = Books::new(Paths::in_dir(dir.0.join("store")));
    let demo_book: Value = serde_json::from_str(&std::fs::read_to_string(DEMO).unwrap()).unwrap();
    demo::seed(&mut books, &demo_book).unwrap();
    (dir, books)
}

/// A posted money-out entry on the bank account; returns its ticket.
fn spend(b: &mut Books, id: &str, date: &str, amount: &str, payee: &str) -> String {
    let d = ok(
        b,
        "books_entry_draft",
        json!({ "client_request_id": format!("{id}-draft"), "form": "money_out", "date": date, "amount": amount, "account": "11010", "category": "68300", "payee": payee }),
    );
    let entry = d["entry"]
        .as_str()
        .unwrap_or_else(|| panic!("{d}"))
        .to_string();
    ok(
        b,
        "books_app_post_entry",
        json!({ "client_request_id": format!("{id}-post"), "entry": entry }),
    );
    entry
}

fn import(b: &mut Books, id: &str, rows: &str) {
    ok(
        b,
        "books_app_statement_import",
        json!({ "client_request_id": id, "account": "11010", "csv_text": format!("Date,Description,Debit,Credit,Balance\n{rows}"), "file_name": "synthetic.csv", "closing": "1000.00" }),
    );
}

fn statement(b: &mut Books, month: &str) -> Value {
    ok(
        b,
        "books_statement_get",
        json!({ "account": "11010", "month": month }),
    )["statement"]
        .clone()
}

fn kinds(st: &Value) -> Vec<String> {
    st["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["match"].as_str().unwrap().to_string())
        .collect()
}

fn payee_of(b: &mut Books, entry: &Value) -> Value {
    ok(b, "books_entry_get", json!({ "entry": entry }))["payee"].clone()
}

fn accept(b: &mut Books, id: &str, month: &str, extra: Value) -> Value {
    let mut args = json!({ "client_request_id": id, "account": "11010", "month": month });
    for (k, v) in extra.as_object().unwrap() {
        args[k] = v.clone();
    }
    run(b, "books_app_statement_accept", args, Origin::App)
}

#[test]
fn a_past_accept_ranks_the_same_payee_first() {
    let (_dir, mut books) = book("ma-rank");
    let b = &mut books;
    spend(b, "ma-nov-coffee", "2026-11-10", "450.00", "Coffee shop");
    import(
        b,
        "ma-nov-import",
        "2026-11-12,COFFEE SHOP 00123,450.00,,\n",
    );
    assert_eq!(kinds(&statement(b, "2026-11")), ["s"]);
    let out = accept(b, "ma-nov-accept", "2026-11", json!({ "line": 1 }));
    assert_eq!(out["structuredContent"]["accepted"], 1, "{out}");

    spend(
        b,
        "ma-dec-hardware",
        "2026-12-11",
        "450.00",
        "Hardware store",
    );
    spend(b, "ma-dec-coffee", "2026-12-14", "450.00", "Coffee shop");
    import(
        b,
        "ma-dec-import",
        "2026-12-10,COFFEE SHOP 00456,450.00,,\n",
    );
    let st = statement(b, "2026-12");
    let row = &st["rows"][0];
    assert_eq!(row["match"], "s", "{st}");
    assert_eq!(payee_of(b, &row["books"]["ticket"]), "Coffee shop", "{st}");
    assert_eq!(row["why"], "Same amount · matched this payee before");
    assert_eq!(st["hints_used"], 1);
}

#[test]
fn without_history_the_nearest_entry_is_suggested() {
    let (_dir, mut books) = book("ma-plain");
    let b = &mut books;
    spend(b, "ma-far", "2026-12-14", "450.00", "Coffee shop");
    spend(b, "ma-near", "2026-12-12", "450.00", "Hardware store");
    import(
        b,
        "ma-plain-import",
        "2026-12-10,COFFEE SHOP 00456,450.00,,\n",
    );
    let st = statement(b, "2026-12");
    let row = &st["rows"][0];
    assert_eq!(
        payee_of(b, &row["books"]["ticket"]),
        "Hardware store",
        "{st}"
    );
    assert_eq!(row["why"], "Same amount, 2 days apart");
    assert_eq!(row["no"], 1);
    assert_eq!(st["hints_used"], 0);
}

#[test]
fn one_line_accept_matches_that_line_only_and_checks_a_picked_entry() {
    let (_dir, mut books) = book("ma-one");
    let b = &mut books;
    spend(b, "ma-a", "2026-12-12", "450.00", "Coffee shop");
    spend(b, "ma-b", "2026-12-17", "120.00", "Hardware store");
    let other = spend(b, "ma-c", "2026-12-20", "75.00", "Hardware store");
    // Same amount as line 1 and inside its window: only the guarded
    // property (voided, too far from the month, matched elsewhere) differs.
    let voided = spend(b, "ma-d", "2026-12-13", "450.00", "Hardware store");
    let far = spend(b, "ma-e", "2026-11-20", "450.00", "Hardware store");
    let taken = spend(b, "ma-f", "2026-12-27", "450.00", "Hardware store");
    ok(
        b,
        "books_app_void_entry",
        json!({ "client_request_id": "ma-void-0001", "entry": voided, "reason": "Synthetic duplicate" }),
    );
    import(
        b,
        "ma-one-import",
        "2026-12-10,COFFEE SHOP 00456,450.00,,\n2026-12-15,HARDWARE STORE 7781,120.00,,\n2026-12-22,BAKERY 5521,75.00,,\n2026-12-28,HARDWARE STORE 7790,450.00,,\n",
    );
    assert_eq!(kinds(&statement(b, "2026-12")), ["s", "s", "s", "s"]);

    let first = accept(b, "ma-one-0001", "2026-12", json!({ "line": 2 }));
    assert_eq!(first["structuredContent"]["accepted"], 1, "{first}");
    assert_eq!(kinds(&statement(b, "2026-12")), ["s", "m", "s", "s"]);
    assert_eq!(
        accept(b, "ma-one-0001", "2026-12", json!({ "line": 2 })),
        first
    );
    assert_eq!(
        refused(
            b,
            "books_app_statement_accept",
            json!({ "client_request_id": "ma-one-0002", "account": "11010", "month": "2026-12", "line": 2 })
        ),
        "already_matched"
    );
    for (id, line) in [
        ("ma-bad-0001", json!("1")),
        ("ma-bad-0002", serde_json::from_str::<Value>("1.0").unwrap()),
        ("ma-bad-0003", Value::Null),
    ] {
        assert_eq!(
            refused(
                b,
                "books_app_statement_accept",
                json!({ "client_request_id": id, "account": "11010", "month": "2026-12", "line": line })
            ),
            "invalid_args",
            "{id}"
        );
    }
    assert_eq!(kinds(&statement(b, "2026-12")), ["s", "m", "s", "s"]);

    let fourth = accept(
        b,
        "ma-one-0004",
        "2026-12",
        json!({ "line": 4, "entry": taken }),
    );
    assert_eq!(fourth["structuredContent"]["accepted"], 1, "{fourth}");
    assert_eq!(kinds(&statement(b, "2026-12")), ["s", "m", "s", "m"]);

    // a picked entry: wrong amount, voided, too far, or matched elsewhere
    let pick = |id: &str, entry: &str| json!({ "client_request_id": id, "account": "11010", "month": "2026-12", "line": 1, "entry": entry });
    for (id, entry) in [
        ("ma-pick-01", other.as_str()),
        ("ma-pick-02", voided.as_str()),
        ("ma-pick-03", far.as_str()),
        ("ma-pick-04", taken.as_str()),
    ] {
        assert_eq!(
            refused(b, "books_app_statement_accept", pick(id, entry)),
            "entry_not_matchable",
            "{id}"
        );
    }
    assert_eq!(kinds(&statement(b, "2026-12")), ["s", "m", "s", "m"]);
}

fn other_accounts(b: &mut Books, entry: &Value) -> Vec<String> {
    let e = ok(b, "books_entry_get", json!({ "entry": entry }));
    e["lines"]
        .as_array()
        .unwrap_or_else(|| panic!("{e}"))
        .iter()
        .filter_map(|l| l["account"].as_str())
        .filter_map(|a| a.split(' ').next())
        .filter(|n| *n != "11010")
        .map(str::to_string)
        .collect()
}

#[test]
fn an_added_line_takes_the_category_and_payee_history_used() {
    let (_dir, mut books) = book("ma-add");
    let b = &mut books;
    spend(b, "ma-hw", "2026-11-10", "300.00", "Hardware store");
    import(b, "ma-add-nov", "2026-11-12,HARDWARE STORE 7781,300.00,,\n");
    assert_eq!(kinds(&statement(b, "2026-11")), ["s"]);
    let out = accept(b, "ma-add-acc", "2026-11", json!({ "line": 1 }));
    assert_eq!(out["structuredContent"]["accepted"], 1, "{out}");

    import(
        b,
        "ma-add-dec",
        "2026-12-09,HARDWARE STORE 9912,180.00,,\n2026-12-11,BAKERY 5521,60.00,,\n",
    );
    let st = statement(b, "2026-12");
    assert_eq!(kinds(&st), ["x", "x"]);
    assert_eq!(st["rows"][0]["hint"]["payee"], "Hardware store", "{st}");
    assert!(st["rows"][0]["hint"]["category"].is_string(), "{st}");
    assert!(st["rows"][1].get("hint").is_none(), "{st}");
    let agent = run(
        b,
        "books_statement_get",
        json!({ "account": "11010", "month": "2026-12" }),
        Origin::Agent,
    );
    assert!(
        agent["structuredContent"]["statement"]["rows"][0]["hint"]["payee"].is_null(),
        "{agent}"
    );

    let hw = st["rows"][0]["ticket"].clone();
    let added = ok(
        b,
        "books_app_statement_add_line",
        json!({ "client_request_id": "ma-add-0001", "account": "11010", "month": "2026-12", "line": hw }),
    );
    assert_eq!(other_accounts(b, &added["entry"]), ["68300"]);
    assert_eq!(payee_of(b, &added["entry"]), "Hardware store");

    let bakery = st["rows"][1]["ticket"].clone();
    let plain = ok(
        b,
        "books_app_statement_add_line",
        json!({ "client_request_id": "ma-add-0002", "account": "11010", "month": "2026-12", "line": bakery }),
    );
    // no history: suggest_kind names nothing for a bakery, so the book's
    // "not sure yet" money-out default (69999) is used.
    assert_eq!(other_accounts(b, &plain["entry"]), ["69999"]);
    assert!(payee_of(b, &plain["entry"]).is_null());
}

#[test]
fn the_assistant_cannot_match_one_line() {
    use books_mini_app::jsonrpc::Server;
    use books_mini_app::manifest::{self, Source};
    let dir = TempDir::new("ma-agent");
    let mut server = Server::new(
        Source::default(),
        Books::new(Paths::in_dir(dir.0.join("store"))),
    );
    let ts = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap();
    let seed = [11u8; 32];
    let public = books_mini_app::b64::encode(
        ed25519_dalek::SigningKey::from_bytes(&seed)
            .verifying_key()
            .as_bytes(),
    );
    let provision = json!({ "jsonrpc": "2.0", "id": 0, "method": "tools/call", "params": { "name": "buwiz.provision", "arguments": { "protocol": "2.0.0", "plugin": manifest::PLUGIN_ID, "key": { "alg": "ed25519", "seed": books_mini_app::b64::encode(&seed) }, "pub": public } } });
    let p = server.handle(&provision).unwrap();
    assert_eq!(p["result"]["isError"], false, "{p}");
    let params = json!({
        "name": "books_app_statement_accept",
        "arguments": { "client_request_id": "ma-agent-01", "account": "11010", "month": "2026-12", "line": 1 },
        "_meta": { "buwiz/caller": { "origin": "agent", "plugin": manifest::PLUGIN_ID, "ts": ts } }
    });
    let reply = server
        .handle(&json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": params }))
        .unwrap();
    let result = &reply["result"];
    assert_eq!(result["isError"], true, "{reply}");
    assert_eq!(
        result["structuredContent"]["code"], "permission_denied",
        "{reply}"
    );
}
