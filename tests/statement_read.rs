//! M10 parity reconcile-1: a statement the assistant read becomes a draft
//! the user accepts in the app. Synthetic data only.

use books_mini_app::jsonrpc::Server;
use books_mini_app::manifest::{self, Source};
use books_mini_app::service::{Books, Origin};
use books_mini_app::{b64, demo, tools};
use books_store::{Paths, TempDir};
use ed25519_dalek::SigningKey;
use serde_json::{Map, Value, json};

const DEMO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/demo/buwiz-trading-2026.json"
);
const BANK: &str = "11010";
const MONTH: &str = "2026-09";

fn run(b: &mut Books, origin: Origin, name: &str, args: Value) -> Value {
    let args: Map<String, Value> = args.as_object().unwrap().clone();
    tools::call(name, &args, origin, b).unwrap()
}
fn ok(b: &mut Books, origin: Origin, name: &str, args: Value) -> Value {
    let out = run(b, origin, name, args);
    assert_eq!(out["isError"], false, "{name}: {out}");
    out["structuredContent"].clone()
}
fn refused(b: &mut Books, origin: Origin, name: &str, args: Value) {
    let out = run(b, origin, name, args);
    assert_eq!(out["isError"], true, "{name}: {out}");
}
fn seeded(dir: &TempDir) -> Books {
    books_core::date::pin_today(books_core::date::Date::new(2026, 10, 29));
    let mut b = Books::new(Paths::in_dir(dir.0.join("store")));
    let book: Value = serde_json::from_str(&std::fs::read_to_string(DEMO).unwrap()).unwrap();
    demo::seed(&mut b, &book).unwrap();
    b
}
fn lines() -> Value {
    json!([
        { "date": "2026-09-03", "description": "Coffee shop", "amount": "-120.00", "balance": "880.00", "page": 1, "bbox": [125, 100, 900, 130], "confidence": 95 },
        { "date": "2026-09-10", "description": "Synthetic deposit", "amount": "500.00", "balance": "1380.00", "page": 1, "bbox": [125, 140, 900, 170], "confidence": 60 },
        { "date": "2026-09-20", "description": "Office snacks", "amount": "-80.00", "balance": "1300.00", "page": 2, "bbox": [125, 100, 900, 130], "confidence": 90 }
    ])
}
fn draft_args(crid: &str, lines: Value) -> Value {
    json!({ "client_request_id": crid, "account": BANK, "month": MONTH, "closing": "1300.00", "lines": lines })
}
fn count(b: &mut Books, sql: &str) -> i64 {
    b.store()
        .unwrap()
        .conn()
        .query_row(sql, [], |r| r.get(0))
        .unwrap()
}
fn get(b: &mut Books, origin: Origin) -> Value {
    ok(
        b,
        origin,
        tools::STATEMENT_GET,
        json!({ "account": BANK, "month": MONTH }),
    )
}

#[test]
fn a_e_h_draft_lines_store_a_draft_and_say_no_amounts() {
    let dir = TempDir::new("r1-draft");
    let mut b = seeded(&dir);
    let args = draft_args("r1-draft-0001", lines());
    let first = run(
        &mut b,
        Origin::Agent,
        tools::STATEMENT_DRAFT_LINES,
        args.clone(),
    );
    let out = first["structuredContent"].clone();
    assert_eq!(first["isError"], false, "{first}");
    assert!(
        out["draft"]
            .as_str()
            .is_some_and(books_store::plumbing::is_ticket)
    );
    assert_eq!(
        (
            out["lines"].as_i64(),
            out["low_confidence"].as_i64(),
            out["outside_month"].as_i64(),
            out["balance_breaks"].as_i64()
        ),
        (Some(3), Some(1), Some(0), Some(0))
    );
    assert_eq!(
        (out["status"].as_str(), out["title"].as_str()),
        (Some("pending"), Some("Statement read"))
    );
    let text = first.to_string();
    for hidden in ["description", "Coffee shop", "120.00", "500.00", "1300.00"] {
        assert!(!text.contains(hidden), "{hidden} in {text}");
    }
    // (e) a replay is byte-identical and stores nothing new
    let again = run(&mut b, Origin::Agent, tools::STATEMENT_DRAFT_LINES, args);
    assert_eq!(again.to_string(), text);
    assert_eq!(count(&mut b, "SELECT count(*) FROM statement_draft"), 1);
    // no statement yet: the grid and the statement view are unchanged
    let view = get(&mut b, Origin::Agent);
    assert!(view["statement"].is_null(), "{view}");
    assert_eq!(view["draft"]["lines"], 3);
    assert!(
        view["draft"].get("rows").is_none(),
        "(h) the assistant never sees rows"
    );
    assert_eq!(
        count(
            &mut b,
            "SELECT count(*) FROM statement WHERE period_key = '2026-09'"
        ),
        0
    );
    let grid = ok(&mut b, Origin::Agent, tools::RECONCILE_GRID, json!({}));
    assert!(!grid.to_string().contains("\"2026-09\",\"state\":\"done\""));
    let app = get(&mut b, Origin::App);
    assert_eq!(app["draft"]["rows"][0]["description"], "Coffee shop");
    assert_eq!(app["draft"]["rows"][1]["low"], true);
}

#[test]
fn b_bad_lines_are_refused() {
    let dir = TempDir::new("r1-refuse");
    let mut b = seeded(&dir);
    let one = |patch: Value| {
        let mut l = lines()[0].clone();
        for (k, v) in patch.as_object().unwrap() {
            l[k] = v.clone();
        }
        json!([l])
    };
    for (n, bad) in [
        json!({"amount": -120}),
        json!({"bbox": [10, 10, 5, 20]}),
        json!({"bbox": [0, 0, 1001, 5]}),
        json!({"date": "2026-09-31"}),
        json!({"balance": 1.5}),
    ]
    .into_iter()
    .enumerate()
    {
        refused(
            &mut b,
            Origin::Agent,
            tools::STATEMENT_DRAFT_LINES,
            draft_args(&format!("r1-bad-{n:04}"), one(bad)),
        );
    }
    let many: Vec<Value> = (0..2001).map(|_| lines()[0].clone()).collect();
    refused(
        &mut b,
        Origin::Agent,
        tools::STATEMENT_DRAFT_LINES,
        draft_args("r1-bad-many", json!(many)),
    );
    // a finished month refuses a new read
    let id = ok(
        &mut b,
        Origin::Agent,
        tools::STATEMENT_DRAFT_LINES,
        draft_args("r1-fin-draft", lines()),
    )["draft"]
        .clone();
    ok(
        &mut b,
        Origin::App,
        tools::STATEMENT_ACCEPT,
        json!({ "client_request_id": "r1-fin-accept", "account": BANK, "month": MONTH, "draft": id }),
    );
    b.store()
        .unwrap()
        .conn()
        .execute(
            "UPDATE statement SET status = 'finished' WHERE period_key = '2026-09'",
            [],
        )
        .unwrap();
    refused(
        &mut b,
        Origin::Agent,
        tools::STATEMENT_DRAFT_LINES,
        draft_args("r1-fin-again", lines()),
    );
    // a routine never drafts, even called directly
    refused(
        &mut b,
        Origin::Routine,
        tools::STATEMENT_DRAFT_LINES,
        draft_args("r1-routine", lines()),
    );
    assert_eq!(count(&mut b, "SELECT count(*) FROM statement_draft"), 1);
}

#[test]
fn d_e_f_accept_exclude_replace_and_dismiss() {
    let dir = TempDir::new("r1-accept");
    let mut b = seeded(&dir);
    let mut with_other = lines().as_array().unwrap().clone();
    with_other.push(json!({ "date": "2026-10-01", "description": "Coffee shop", "amount": "-15.00", "confidence": 99 }));
    let first = ok(
        &mut b,
        Origin::Agent,
        tools::STATEMENT_DRAFT_LINES,
        draft_args("r1-acc-0001", lines()),
    )["draft"]
        .clone();
    // (f) a second read for the month replaces the first
    let second = ok(
        &mut b,
        Origin::Agent,
        tools::STATEMENT_DRAFT_LINES,
        draft_args("r1-acc-0002", json!(with_other)),
    );
    assert_eq!(second["outside_month"], 1);
    let draft = second["draft"].clone();
    assert_eq!(
        count(
            &mut b,
            "SELECT count(*) FROM statement_draft WHERE status = 'replaced'"
        ),
        1
    );
    refused(
        &mut b,
        Origin::App,
        tools::STATEMENT_ACCEPT,
        json!({ "client_request_id": "r1-acc-old", "account": BANK, "month": MONTH, "draft": first }),
    );
    // the outside-month line must be left out first
    refused(
        &mut b,
        Origin::App,
        tools::STATEMENT_ACCEPT,
        json!({ "client_request_id": "r1-acc-0003", "account": BANK, "month": MONTH, "draft": draft, "exclude": [2] }),
    );
    let args = json!({ "client_request_id": "r1-acc-0004", "account": BANK, "month": MONTH, "draft": draft, "exclude": [2, 4] });
    let used = run(&mut b, Origin::App, tools::STATEMENT_ACCEPT, args.clone());
    assert_eq!(used["isError"], false, "{used}");
    assert_eq!(used["structuredContent"]["lines"], 2);
    assert_eq!(used["structuredContent"]["month"], MONTH);
    // (e) replaying the accept is byte-identical and adds no statement
    assert_eq!(
        run(&mut b, Origin::App, tools::STATEMENT_ACCEPT, args).to_string(),
        used.to_string()
    );
    assert_eq!(
        count(
            &mut b,
            "SELECT count(*) FROM statement WHERE period_key = '2026-09'"
        ),
        1
    );
    let view = get(&mut b, Origin::App);
    let rows = view["statement"]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(
        (
            rows[0]["page"].as_i64(),
            rows[0]["box"]["left"].as_str(),
            rows[0]["box"]["width"].as_str()
        ),
        (Some(1), Some("12.5%"), Some("77.5%"))
    );
    assert_eq!(rows[1]["page"], 2);
    assert!(
        view["statement"]["evidence"].is_null(),
        "no evidence was given"
    );
    assert!(view["draft"].is_null());
    // dismiss: a new read, put aside, is gone from the view
    let third = ok(
        &mut b,
        Origin::App,
        tools::STATEMENT_DRAFT_LINES,
        draft_args("r1-acc-0005", lines()),
    )["draft"]
        .clone();
    assert_eq!(
        get(&mut b, Origin::App)["statement"]["draft"]["proposed_by"],
        "app"
    );
    ok(
        &mut b,
        Origin::App,
        tools::STATEMENT_ACCEPT,
        json!({ "client_request_id": "r1-acc-0006", "account": BANK, "month": MONTH, "draft": third, "dismiss": true }),
    );
    assert_eq!(
        count(
            &mut b,
            "SELECT count(*) FROM statement_draft WHERE status = 'dismissed'"
        ),
        1
    );
    assert!(get(&mut b, Origin::App)["draft"].is_null());
}

#[test]
fn d_evidence_ticket_is_kept_on_the_statement() {
    let dir = TempDir::new("r1-evidence");
    let mut b = seeded(&dir);
    ok(
        &mut b,
        Origin::App,
        tools::EVIDENCE_ATTACH,
        json!({"client_request_id":"r1-ev-part","op":"part","upload":"r1-upload","part":0,"parts":1,"data_base64":b64::encode(b"%PDF-1.4 synthetic")}),
    );
    let ev = ok(
        &mut b,
        Origin::App,
        tools::EVIDENCE_ATTACH,
        json!({"client_request_id":"r1-ev-commit","op":"commit","upload":"r1-upload","mime":"application/pdf","kind":"statement","page_count":2}),
    );
    let evidence = ev["evidence"].clone();
    assert!(evidence.is_string(), "{ev}");
    let mut args = draft_args("r1-ev-draft", lines());
    args["evidence"] = evidence.clone();
    let draft = ok(&mut b, Origin::Agent, tools::STATEMENT_DRAFT_LINES, args)["draft"].clone();
    ok(
        &mut b,
        Origin::App,
        tools::STATEMENT_ACCEPT,
        json!({ "client_request_id": "r1-ev-accept", "account": BANK, "month": MONTH, "draft": draft }),
    );
    assert_eq!(get(&mut b, Origin::App)["statement"]["evidence"], evidence);
    // a CSV that replaces it drops the old snapshot
    let csv = "Date,Description,Amount\n2026-09-02,Coffee shop,-100.00\n";
    ok(
        &mut b,
        Origin::App,
        tools::STATEMENT_IMPORT,
        json!({ "client_request_id": "r1-ev-csv", "account": BANK, "month": MONTH, "csv_text": csv, "closing": "900.00" }),
    );
    assert!(
        get(&mut b, Origin::App)["statement"]["evidence"].is_null(),
        "the replaced statement keeps no snapshot"
    );
}

#[test]
fn g_csv_balances_are_kept_and_breaks_counted() {
    let dir = TempDir::new("r1-csv");
    let mut b = seeded(&dir);
    let csv = "Date,Description,Amount,Balance\n2026-09-02,Coffee shop,-100.00,900.00\n2026-09-05,Synthetic deposit,200.00,1100.00\n";
    ok(
        &mut b,
        Origin::App,
        tools::STATEMENT_IMPORT,
        json!({ "client_request_id": "r1-csv-0001", "account": BANK, "month": MONTH, "csv_text": csv }),
    );
    assert_eq!(
        count(
            &mut b,
            "SELECT count(*) FROM statement_line WHERE balance IS NOT NULL"
        ),
        2
    );
    assert_eq!(
        count(
            &mut b,
            "SELECT balance FROM statement_line WHERE line_no = 2"
        ),
        110000
    );
    let mut broken = lines();
    broken[2]["balance"] = json!("1299.00");
    let out = ok(
        &mut b,
        Origin::Agent,
        tools::STATEMENT_DRAFT_LINES,
        json!({ "client_request_id": "r1-csv-0002", "account": BANK, "month": "2026-08", "lines": broken }),
    );
    assert_eq!(out["balance_breaks"], 1);
}

fn now_ms() -> i64 {
    i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap()
}

#[test]
fn c_the_assistant_cannot_accept_and_a_routine_cannot_draft() {
    let dir = TempDir::new("r1-rpc");
    let mut server = Server::new(Source::default(), seeded(&dir));
    let mut next = 0;
    let mut raw = |name: &str, args: Value, caller: Value| {
        next += 1;
        let mut params = json!({ "name": name, "arguments": args });
        if !caller.is_null() {
            params["_meta"] = json!({ "buwiz/caller": caller });
        }
        server
            .handle(
                &json!({ "jsonrpc": "2.0", "id": next, "method": "tools/call", "params": params }),
            )
            .unwrap()["result"]
            .clone()
    };
    let seed = [11u8; 32];
    let public = b64::encode(SigningKey::from_bytes(&seed).verifying_key().as_bytes());
    let p = raw(
        "buwiz.provision",
        json!({ "protocol": "2.0.0", "plugin": manifest::PLUGIN_ID, "key": { "alg": "ed25519", "seed": b64::encode(&seed) }, "pub": public }),
        Value::Null,
    );
    assert_eq!(p["isError"], false, "{p}");
    let agent = json!({ "origin": "agent", "plugin": manifest::PLUGIN_ID, "ts": now_ms() });
    let drafted = raw(
        tools::STATEMENT_DRAFT_LINES,
        draft_args("r1-rpc-0001", lines()),
        agent.clone(),
    );
    assert_eq!(drafted["isError"], false, "{drafted}");
    let draft = drafted["structuredContent"]["draft"].clone();
    let accept = raw(
        tools::STATEMENT_ACCEPT,
        json!({ "client_request_id": "r1-rpc-0002", "account": BANK, "month": MONTH, "draft": draft }),
        agent,
    );
    assert_eq!(accept["isError"], true, "{accept}");
    assert_eq!(accept["structuredContent"]["code"], "permission_denied");
    let routine = json!({ "origin": "routine", "plugin": manifest::PLUGIN_ID, "routine": "weekly-tidy-up", "ts": now_ms() });
    let r = raw(
        tools::STATEMENT_DRAFT_LINES,
        draft_args("r1-rpc-0003", lines()),
        routine,
    );
    assert_eq!(r["isError"], true, "{r}");
}

#[test]
fn window_does_not_call_the_draft_tool() {
    let app = manifest::unsigned(manifest::RELEASE_KEY_ID)["apps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == "books")
        .unwrap()
        .clone();
    let granted = app["permissions"]["tools"].as_array().unwrap();
    assert!(!granted.iter().any(|t| t == tools::STATEMENT_DRAFT_LINES));
    assert!(granted.iter().any(|t| t == tools::STATEMENT_ACCEPT));
}
