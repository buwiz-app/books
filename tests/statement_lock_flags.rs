//! M10 parity (reconcile-3): a finished statement locks its matched
//! entries until it is reopened with a reason; flags are display only.
//! Synthetic demo book and statement only.

use books_mini_app::demo;
use books_mini_app::jsonrpc::Server;
use books_mini_app::manifest::{self, Source};
use books_mini_app::service::{Books, Origin};
use books_mini_app::{b64, tools};
use books_store::{Paths, TempDir};
use ed25519_dalek::SigningKey;
use serde_json::{Map, Value, json};

const DEMO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/demo/buwiz-trading-2026.json"
);
const STATEMENT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/demo/bank-4021-2026-10.csv"
);
const REASON: &str = "Synthetic reason zq7";

fn run(books: &mut Books, name: &str, args: Value) -> Value {
    let args: Map<String, Value> = args.as_object().unwrap().clone();
    tools::call(name, &args, Origin::App, books).unwrap()
}

fn ok(books: &mut Books, name: &str, args: Value) -> Value {
    let out = run(books, name, args);
    assert_eq!(out["isError"], false, "{name}: {out}");
    out["structuredContent"].clone()
}

fn refused(books: &mut Books, name: &str, args: Value) -> (String, String) {
    let out = run(books, name, args);
    assert_eq!(out["isError"], true, "{name}: {out}");
    (
        out["structuredContent"]["code"]
            .as_str()
            .unwrap()
            .to_string(),
        out.to_string(),
    )
}

fn seeded(tag: &str) -> (TempDir, Books) {
    books_core::date::pin_today(books_core::date::Date::new(2026, 10, 29));
    let dir = TempDir::new(tag);
    let mut books = Books::new(Paths::in_dir(dir.0.join("store")));
    let demo_book: Value = serde_json::from_str(&std::fs::read_to_string(DEMO).unwrap()).unwrap();
    demo::seed(&mut books, &demo_book).unwrap();
    (dir, books)
}

fn month() -> Value {
    json!({ "account": "11010", "month": "2026-10" })
}

fn with(extra: Value) -> Value {
    let mut a = month();
    for (k, v) in extra.as_object().unwrap() {
        a[k] = v.clone();
    }
    a
}

/// Import `csv`, accept the suggestion and add every missing line.
fn match_all(b: &mut Books, csv: &str, tag: &str) -> Value {
    ok(
        b,
        "books_app_statement_import",
        with(
            json!({ "client_request_id": format!("{tag}-import"), "csv_base64": b64::encode(csv.as_bytes()), "file_name": "bank.csv" }),
        ),
    );
    ok(
        b,
        "books_app_statement_accept",
        with(json!({ "client_request_id": format!("{tag}-accept") })),
    );
    let view = ok(b, "books_statement_get", month());
    for (i, r) in view["statement"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        if r["match"] == "x" {
            ok(
                b,
                "books_app_statement_add_line",
                with(json!({ "client_request_id": format!("{tag}-add-{i}"), "line": r["ticket"] })),
            );
        }
    }
    ok(b, "books_statement_get", month())
}

fn events(b: &mut Books, event: &str) -> i64 {
    b.store()
        .unwrap()
        .conn()
        .query_row(
            "SELECT count(*) FROM statement_event WHERE event = ?1",
            [event],
            |r| r.get(0),
        )
        .unwrap()
}

fn grid_state(b: &mut Books) -> String {
    let grid = ok(b, "books_reconcile_grid", json!({}));
    grid["rows"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|r| r["cells"].as_array().unwrap())
        .find(|c| c["month"] == "2026-10" && c["state"] != "none")
        .map(|c| c["state"].as_str().unwrap().to_string())
        .unwrap()
}

/// A fresh entry on the bank, not on any statement.
fn fresh_entry(b: &mut Books, tag: &str) -> String {
    let d = ok(
        b,
        "books_entry_draft",
        json!({ "client_request_id": format!("{tag}-draft"), "form": "money_out", "date": "2026-10-29", "amount": "150.00", "account": "11010", "category": "68300", "payee": "Coffee shop" }),
    );
    let entry = d["entry"].as_str().unwrap().to_string();
    ok(
        b,
        "books_app_post_entry",
        json!({ "client_request_id": format!("{tag}-post"), "entry": entry }),
    );
    entry
}

#[test]
fn a_finished_statement_locks_its_matches_until_reopened_with_a_reason() {
    let (_dir, mut books) = seeded("r3-lock");
    let b = &mut books;
    let view = match_all(b, &std::fs::read_to_string(STATEMENT).unwrap(), "r3");
    assert_eq!(view["statement"]["can_finish"], true, "{view}");
    assert_eq!(view["statement"]["reopenable"], false);
    ok(
        b,
        "books_app_statement_finish",
        with(json!({ "client_request_id": "r3-finish-1" })),
    );
    assert_eq!((events(b, "finish"), events(b, "reopen")), (1, 0));
    assert_eq!(grid_state(b), "done");
    let view = ok(b, "books_statement_get", month());
    assert_eq!(view["statement"]["reopenable"], true);
    let matched = view["statement"]["rows"][0]["books"]["ticket"]
        .as_str()
        .unwrap()
        .to_string();

    // a matched entry of a finished month cannot be voided
    let void = |id: &str, entry: &str| json!({ "client_request_id": id, "entry": entry });
    let (code, text) = refused(b, "books_app_void_entry", void("r3-void-1", &matched));
    assert_eq!(code, "statement_finished");
    assert!(text.contains("finished month (Oct 2026)"), "{text}");
    // an entry on no finished statement still voids
    let free = fresh_entry(b, "r3-free");
    ok(b, "books_app_void_entry", void("r3-void-2", &free));
    // discard never meets the lock: drafts are never matched
    let d = ok(
        b,
        "books_entry_draft",
        json!({ "client_request_id": "r3-draft-2", "form": "money_out", "date": "2026-10-09", "amount": "150.00", "account": "11010", "category": "68300", "payee": "Coffee shop" }),
    );
    ok(
        b,
        "books_app_discard_entry",
        json!({ "client_request_id": "r3-discard", "entry": d["entry"] }),
    );

    // a reopen needs a reason of 3 or more characters
    for (i, reason) in ["", "ab", "  ab  "].iter().enumerate() {
        let (code, _) = refused(
            b,
            "books_app_statement_finish",
            with(
                json!({ "client_request_id": format!("r3-short-{i}"), "reopen": true, "reason": reason }),
            ),
        );
        assert_eq!(code, "reason_needed");
    }
    let reopen =
        with(json!({ "client_request_id": "r3-reopen-1", "reopen": true, "reason": REASON }));
    let first = run(b, "books_app_statement_finish", reopen.clone());
    assert_eq!(first["isError"], false, "{first}");
    let sc = &first["structuredContent"];
    assert_eq!(
        (
            sc["month"].as_str(),
            sc["status"].as_str(),
            sc["reopened"].as_bool()
        ),
        (Some("2026-10"), Some("open"), Some(true))
    );
    assert!(
        !first.to_string().contains(REASON),
        "the reason is never echoed"
    );
    assert_eq!(events(b, "reopen"), 1);
    assert_ne!(grid_state(b), "done");
    // a replay returns the stored result and writes nothing
    assert_eq!(run(b, "books_app_statement_finish", reopen), first);
    assert_eq!(events(b, "reopen"), 1);
    // a reopened statement is no longer finished: it cannot reopen again
    let (code, _) = refused(
        b,
        "books_app_statement_finish",
        with(json!({ "client_request_id": "r3-reopen-2", "reopen": true, "reason": REASON })),
    );
    assert_eq!(code, "not_finished");
    let view = ok(b, "books_statement_get", month());
    assert!(!view.to_string().contains(REASON));
    assert_eq!(view["statement"]["can_finish"], true, "every match is kept");

    // finish again, then the lock is back; reopen and the void goes through
    ok(
        b,
        "books_app_statement_finish",
        with(json!({ "client_request_id": "r3-finish-2" })),
    );
    assert_eq!((events(b, "finish"), events(b, "reopen")), (2, 1));
    assert_eq!(
        refused(b, "books_app_void_entry", void("r3-void-3", &matched)).0,
        "statement_finished"
    );
    ok(
        b,
        "books_app_statement_finish",
        with(json!({ "client_request_id": "r3-reopen-3", "reopen": true, "reason": REASON })),
    );
    assert_eq!((events(b, "finish"), events(b, "reopen")), (2, 2));
    let voided = run(b, "books_app_void_entry", void("r3-void-4", &matched));
    assert_eq!(voided["isError"], false, "{voided}");
}

#[test]
fn a_duplicate_line_is_flagged_and_finish_still_works() {
    let (_dir, mut books) = seeded("r3-flags");
    let b = &mut books;
    let csv = std::fs::read_to_string(STATEMENT)
        .unwrap()
        .replace(
            "2026-10-09,SERVICE CHARGE,150.00,,\n",
            "2026-10-09,SERVICE CHARGE,150.00,,\n2026-10-09,SERVICE CHARGE,150.00,,\n",
        )
        .replace("\"412,392.18\"", "\"412,242.18\"");
    let view = match_all(b, &csv, "r3f");
    let st = &view["statement"];
    assert_eq!(
        st["flags"],
        json!([{ "line": 5, "kind": "duplicate", "note": "Same date and amount as another line" }])
    );
    let rows = st["rows"].as_array().unwrap();
    assert_eq!(rows[4]["flag"], "duplicate");
    assert_eq!(rows[3]["flag"], Value::Null);
    assert_eq!(st["difference"], "0.00", "{st}");
    assert_eq!(st["can_finish"], true, "flags never block Finish");
    ok(
        b,
        "books_app_statement_finish",
        with(json!({ "client_request_id": "r3f-finish" })),
    );
}

#[test]
fn the_assistant_cannot_reopen_a_statement() {
    let (_dir, books) = seeded("r3-agent");
    let mut server = Server::new(Source::default(), books);
    let seed = [11u8; 32];
    let public = b64::encode(SigningKey::from_bytes(&seed).verifying_key().as_bytes());
    let mut call = |id: i64, name: &str, args: Value, caller: Value| {
        let mut params = json!({ "name": name, "arguments": args });
        if !caller.is_null() {
            params["_meta"] = json!({ "buwiz/caller": caller });
        }
        server
            .handle(
                &json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call", "params": params }),
            )
            .unwrap()["result"]
            .clone()
    };
    let p = call(
        1,
        "buwiz.provision",
        json!({ "protocol": "2.0.0", "plugin": manifest::PLUGIN_ID, "key": { "alg": "ed25519", "seed": b64::encode(&seed) }, "pub": public }),
        Value::Null,
    );
    assert_eq!(p["isError"], false, "{p}");
    let ts = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap();
    let out = call(
        2,
        "books_app_statement_finish",
        with(json!({ "client_request_id": "r3-agent-1", "reopen": true, "reason": REASON })),
        json!({ "origin": "agent", "plugin": manifest::PLUGIN_ID, "ts": ts }),
    );
    assert_eq!(out["isError"], true, "{out}");
    let text = out["content"][0]["text"].as_str().unwrap_or("");
    assert!(text.contains("needs the user's click"), "{text}");
    assert!(!out.to_string().contains(REASON));
}
