//! M7 Slice B against a test harness (no scratch BIR profile exists, B2):
//! the demo book's Q3 2551Q goes from Books to a fake BIR draft the way
//! the assistant does it (docs/01 9.2), and a later change marks the
//! hand-off out of date until the owner refreshes it.
//!
//! The fake BIR takes a fill only with source `document` (D3) and only on
//! input keys the 2551Q crosswalk lets Books fill; it has no send.

use std::collections::BTreeMap;

use books_core::input::FormCode;
use books_core::rules;
use books_mini_app::demo;
use books_mini_app::service::{Books, Origin};
use books_mini_app::tools;
use books_store::{Paths, TempDir};
use serde_json::{Map, Value, json};

const DEMO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/demo/buwiz-trading-2026.json"
);

fn call(books: &mut Books, origin: Origin, name: &str, args: Value) -> Value {
    let args: Map<String, Value> = args.as_object().unwrap().clone();
    tools::call(name, &args, origin, books).unwrap()
}

fn ok(books: &mut Books, origin: Origin, name: &str, args: Value) -> Value {
    let out = call(books, origin, name, args);
    assert_eq!(out["isError"], false, "{name}: {out}");
    out["structuredContent"].clone()
}

fn refused(books: &mut Books, origin: Origin, name: &str, args: Value) -> String {
    let out = call(books, origin, name, args);
    assert_eq!(out["isError"], true, "{name}: {out}");
    out["structuredContent"]["code"]
        .as_str()
        .unwrap()
        .to_string()
}

/// A BIR draft as far as the hand-off needs it.
#[derive(Default)]
struct FakeBir {
    opened: Option<(String, i64, i64)>,
    fields: BTreeMap<String, String>,
    fills: usize,
    saved: bool,
}

impl FakeBir {
    fn open(&mut self, form: &str, year: i64, period: i64) {
        self.opened = Some((form.into(), year, period));
    }

    fn fill(&mut self, fields: &Map<String, Value>, source: &str) -> Result<(), String> {
        if source != "document" {
            return Err(format!("source {source} is not the hand-off label"));
        }
        let data = rules::shared().unwrap();
        let crosswalk = data.crosswalk(FormCode::F2551Q);
        for (k, v) in fields {
            let field = crosswalk
                .by_fill_key(k)
                .ok_or(format!("{k} is not a 2551Q box"))?;
            if field.fill_class != "input" {
                return Err(format!("{k} is computed by BIR"));
            }
            self.fields
                .insert(k.clone(), v.as_str().unwrap().to_string());
        }
        self.fills += 1;
        Ok(())
    }
}

#[test]
fn slice_b_q3_2551q_reaches_a_bir_draft_and_a_later_change_marks_it_out_of_date() {
    books_core::date::pin_today(books_core::date::Date::new(2026, 10, 29));
    let dir = TempDir::new("slice-b");
    let mut books = Books::new(Paths::in_dir(dir.0.join("store")));
    let b = &mut books;
    let demo_book: Value = serde_json::from_str(&std::fs::read_to_string(DEMO).unwrap()).unwrap();
    demo::seed(b, &demo_book).unwrap();
    let app = Origin::App;
    let agent = Origin::Agent;
    for m in ["2026-07", "2026-08", "2026-09"] {
        ok(
            b,
            app,
            "books_app_month_close",
            json!({ "client_request_id": format!("close-{m}"), "month": m }),
        );
    }

    // not ready yet: the September return waits for the owner's OK
    let code = refused(
        b,
        app,
        "books_app_handoff_create",
        json!({ "client_request_id": "ho-early-01", "form": "2551Q", "period": "2026-Q3" }),
    );
    assert_eq!(code, "not_ready");
    let view = ok(
        b,
        app,
        "books_form_proposal",
        json!({ "form": "2551Q", "period": "2026-Q3" }),
    );
    for r in view["returns"].as_array().unwrap() {
        ok(
            b,
            app,
            "books_app_return_ok",
            json!({ "client_request_id": "ack-0001", "entry": r["ticket"] }),
        );
    }

    // 1. the owner's click: Hand to BIR draft (the app then attaches the ticket)
    let made = ok(
        b,
        app,
        "books_app_handoff_create",
        json!({ "client_request_id": "handoff-0001", "form": "2551Q", "period": "2026-Q3" }),
    );
    assert_eq!(
        (made["title"].as_str(), made["boxes"].as_i64()),
        (Some("BIR hand-off · 2551Q Q3 2026"), Some(2))
    );
    let ticket = made["ticket"].as_str().unwrap().to_string();
    assert_eq!(
        tools::find("books_app_handoff_create").unwrap().reach,
        tools::Reach::App
    );
    assert_eq!(
        tools::find("books_app_handoff_confirm").unwrap().reach,
        tools::Reach::App
    );

    // the composer's scope: the ticket resolves to a token, no value lists choices
    let token = ok(
        b,
        agent,
        "books_resolve",
        json!({ "kind": "scope", "name": "proposal", "value": ticket }),
    );
    assert_eq!(
        (token["result"].as_str(), token["title"].as_str()),
        (Some("token"), Some("BIR hand-off · 2551Q Q3 2026"))
    );
    let choices = ok(
        b,
        agent,
        "books_resolve",
        json!({ "kind": "scope", "name": "proposal" }),
    );
    assert_eq!(choices["choices"].as_array().unwrap().len(), 1);
    assert_eq!(
        b.ticket_title(&ticket).as_deref(),
        Some("BIR hand-off · 2551Q Q3 2026")
    );

    // 2. the assistant reads it and fills the BIR draft once, source document
    let got = ok(b, agent, "books_handoff_get", json!({ "handoff": ticket }));
    assert_eq!(
        (
            got["form"].as_str(),
            got["year"].as_i64(),
            got["bir_period"].as_i64(),
            got["source"].as_str()
        ),
        (Some("2551Q"), Some(2026), Some(3), Some("document"))
    );
    let fill = got["fill_args"].as_object().unwrap().clone();
    assert_eq!(
        Value::Object(fill.clone()),
        json!({ "txtATCAmt1": "290000.00", "frm2551Qv2018:txt15": "0.00" })
    );
    let text = got.to_string();
    assert!(
        !text.contains("\"e") && !text.contains("tkt_"),
        "no Books ids or tickets in what BIR gets: {text}"
    );
    let mut bir = FakeBir::default();
    bir.open(
        "2551Q",
        got["year"].as_i64().unwrap(),
        got["bir_period"].as_i64().unwrap(),
    );
    assert!(
        bir.fill(&fill, "user").is_err(),
        "a Books value is never the user's"
    );
    bir.fill(&fill, got["source"].as_str().unwrap()).unwrap();
    // read back: every input equals its fill_args string exactly
    for (k, v) in &fill {
        assert_eq!(bir.fields.get(k).map(String::as_str), v.as_str(), "{k}");
    }
    bir.saved = true;
    assert_eq!(
        (bir.opened.clone(), bir.fills, bir.saved),
        (Some(("2551Q".into(), 2026, 3)), 1, true)
    );

    // 3. the owner's click: BIR shows these values
    let confirmed = ok(
        b,
        app,
        "books_app_handoff_confirm",
        json!({ "client_request_id": "ho-conf-01", "handoff": ticket }),
    );
    assert_eq!(confirmed["fill_args_sha256"].as_str().unwrap().len(), 64);
    let view = ok(
        b,
        app,
        "books_form_proposal",
        json!({ "form": "2551Q", "period": "2026-Q3" }),
    );
    assert_eq!(
        (
            view["handoff"]["status"].as_str(),
            view["handoff"]["label"].as_str()
        ),
        (Some("confirmed"), Some("In BIR draft"))
    );
    // the hand-off is stamped by the real clock, not the pinned day: its
    // label is that instant's day in Manila
    let handed = books_core::date::Date::manila_from_unix_ms(books_store::now_ms()).short();
    assert_eq!(
        view["handoff"]["detail"],
        format!("Handed over {handed} · 2 boxes")
    );
    let cards = ok(b, app, "books_forms", json!({}));
    assert_eq!(cards["cards"][0]["status_label"], "In BIR draft");

    // 4. a later change: September reopens for a late sale
    ok(
        b,
        app,
        "books_app_month_reopen",
        json!({ "client_request_id": "reopen-09", "month": "2026-09", "reason": "A late sale" }),
    );
    let view = ok(
        b,
        app,
        "books_form_proposal",
        json!({ "form": "2551Q", "period": "2026-Q3" }),
    );
    assert_eq!(
        (
            view["handoff"]["status"].as_str(),
            view["handoff"]["detail"].as_str()
        ),
        (Some("stale"), Some("September reopened"))
    );
    assert_eq!(
        refused(b, agent, "books_handoff_get", json!({ "handoff": ticket })),
        "out_of_date"
    );
    ok(
        b,
        app,
        "books_app_post_entry",
        json!({ "client_request_id": "late-sale-01", "form": "money_in", "date": "2026-09-20", "amount": "5000.00", "account": "11010", "category": "41000", "payee": "Isla Café" }),
    );
    ok(
        b,
        app,
        "books_app_month_close",
        json!({ "client_request_id": "close-09-again", "month": "2026-09" }),
    );
    let view = ok(
        b,
        app,
        "books_form_proposal",
        json!({ "form": "2551Q", "period": "2026-Q3" }),
    );
    assert_eq!(view["handoff"]["status"], "stale");
    assert_eq!(
        view["handoff"]["detail"],
        "Sep 20 · Isla Café: 1 line changed"
    );
    assert_eq!(
        refused(
            b,
            app,
            "books_app_handoff_confirm",
            json!({ "client_request_id": "ho-conf-02", "handoff": ticket })
        ),
        "out_of_date"
    );

    // Refresh: a new hand-off; the old one is superseded
    let fresh = ok(
        b,
        app,
        "books_app_handoff_create",
        json!({ "client_request_id": "handoff-0002", "form": "2551Q", "period": "2026-Q3" }),
    );
    assert_eq!(
        refused(b, agent, "books_handoff_get", json!({ "handoff": ticket })),
        "superseded"
    );
    let got = ok(
        b,
        agent,
        "books_handoff_get",
        json!({ "handoff": fresh["ticket"] }),
    );
    assert_eq!(got["fill_args"]["txtATCAmt1"], "295000.00");
}

#[test]
fn m7_06_cross_checks_agree_on_the_demo_and_flag_a_filed_difference() {
    books_core::date::pin_today(books_core::date::Date::new(2026, 10, 29));
    let dir = TempDir::new("checks");
    let mut books = Books::new(Paths::in_dir(dir.0.join("store")));
    let b = &mut books;
    let demo_book: Value = serde_json::from_str(&std::fs::read_to_string(DEMO).unwrap()).unwrap();
    demo::seed(b, &demo_book).unwrap();
    let checks = ok(
        b,
        Origin::Agent,
        "books_cross_checks",
        json!({ "year": "2026" }),
    );
    let rows = checks["rows"].as_array().unwrap();
    let names: Vec<&str> = rows.iter().map(|r| r["check"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        [
            "2551Q Q1+Q2+Q3+Q4 = 1701-MS IV.19",
            "P&L sales = gross sales",
            "pay run September = 1601-C 14, 19, 25",
            "2307s received = 1701-MS V.3"
        ]
    );
    assert!(rows.iter().all(|r| r["ok"] == true), "{checks}");
    assert_eq!(rows[3]["left_shown"], "8,900.00");
    assert_eq!(rows[2]["left_shown"], "42,000.00 · 2,000.00 · 2,808.40");
    // a filed value the assistant read from BIR that Books no longer matches
    let filed = ok(
        b,
        Origin::Agent,
        "books_cross_checks",
        json!({ "year": "2026", "filed": [{ "form": "2551Q", "period": "2026-Q1", "key": "txtATCAmt1", "value": "295000.00" }] }),
    );
    let last = filed["rows"].as_array().unwrap().last().unwrap().clone();
    assert_eq!(last["check"], "filed 2551Q Q1 2026 vs Books");
    assert_eq!(
        (last["ok"].as_bool(), filed["differences"].as_i64()),
        (Some(false), Some(1)),
        "{last}"
    );
}
