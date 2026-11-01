//! Receipt extraction provenance on drafts. Synthetic values only.
use books_core::ledger::Basis;
use books_mini_app::{
    b64,
    service::{Books, Origin},
    tools,
};
use books_store::{Paths, TempDir, books};
use serde_json::{Value, json};

fn call_as(b: &mut Books, origin: Origin, tool: &str, args: Value) -> Value {
    tools::call(tool, args.as_object().unwrap(), origin, b).unwrap()
}
fn ok_as(b: &mut Books, origin: Origin, tool: &str, args: Value) -> Value {
    let v = call_as(b, origin, tool, args);
    assert_eq!(v["isError"], false, "{v}");
    v["structuredContent"].clone()
}
fn code(v: &Value) -> String {
    assert_eq!(v["isError"], true, "{v}");
    v["structuredContent"]["code"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}
const DEMO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/demo/buwiz-trading-2026.json"
);
fn setup(name: &str) -> (TempDir, Books) {
    books_core::date::pin_today(books_core::date::Date::new(2026, 10, 29));
    let tmp = TempDir::new(name);
    let mut b = Books::new(Paths::in_dir(tmp.0.join("db")));
    let demo: Value = serde_json::from_str(&std::fs::read_to_string(DEMO).unwrap()).unwrap();
    books_mini_app::demo::seed(&mut b, &demo).unwrap();
    (tmp, b)
}
fn evidence(b: &mut Books, tag: &str) -> String {
    evidence_in(b, tag, None)
}
fn evidence_in(b: &mut Books, tag: &str, book: Option<&str>) -> String {
    let bytes = format!("%PDF-synthetic-{tag}");
    let mut part = json!({"client_request_id":format!("receipt-part-{tag}"),"op":"part","upload":format!("receipt-up-{tag}"),"part":0,"parts":1,"data_base64":b64::encode(bytes.as_bytes())});
    let mut commit = json!({"client_request_id":format!("receipt-commit-{tag}"),"op":"commit","upload":format!("receipt-up-{tag}"),"mime":"application/pdf","kind":"receipt"});
    if let Some(book) = book {
        part["book"] = json!(book);
        commit["book"] = json!(book);
    }
    ok_as(b, Origin::App, tools::EVIDENCE_ATTACH, part);
    let c = ok_as(b, Origin::App, tools::EVIDENCE_ATTACH, commit);
    c["evidence"].as_str().unwrap().to_string()
}
fn draft(crid: &str, extracted: Value) -> Value {
    let mut d = json!({"client_request_id":crid,"form":"money_out","date":"2026-10-18","amount":"150.00","account":"11010","category":"68300","payee":"Synthetic Shop"});
    if !extracted.is_null() {
        d["extracted"] = extracted;
    }
    d
}
fn fields(c: Value) -> Value {
    json!([
        {"field":"date","value":"2026-10-18","confidence":c},
        {"field":"amount","value":"150.00","confidence":c},
        {"field":"person","value":"Synthetic Shop","confidence":c}
    ])
}
fn count(b: &mut Books, sql: &str) -> i64 {
    b.store()
        .unwrap()
        .conn()
        .query_row(sql, [], |r| r.get(0))
        .unwrap()
}
fn get(b: &mut Books, origin: Origin, entry: &Value) -> Value {
    ok_as(b, origin, tools::ENTRY_GET, json!({"entry":entry}))
}

#[test]
fn thresholds_79_and_80_and_missing_confidence() {
    let (_tmp, mut b) = setup("receipt-thresholds");
    let ev = evidence(&mut b, "t");
    for (n, overall, field, review) in [
        (0, json!(80), json!(80), false),
        (1, json!(79), json!(90), true),
        (2, json!(95), json!(79), true),
        (3, Value::Null, json!(95), true),
        (4, json!(100), Value::Null, true),
    ] {
        let mut x = json!({"evidence":ev,"fields":fields(field.clone())});
        if !overall.is_null() {
            x["confidence"] = overall;
        }
        if field.is_null() {
            x["fields"] = json!([{"field":"date","value":"2026-10-18"}]);
        }
        let d = ok_as(
            &mut b,
            Origin::Agent,
            tools::ENTRY_DRAFT,
            draft(&format!("receipt-thr-{n}"), x),
        );
        assert_eq!(d["extraction"]["needs_review"], review, "case {n}: {d}");
        let g = get(&mut b, Origin::App, &d["entry"]);
        assert_eq!(g["extraction"]["needs_review"], review, "case {n}");
        let listed = ok_as(
            &mut b,
            Origin::App,
            "books_ledger_list",
            json!({"filter":"review"}),
        );
        let found = listed["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["ticket"] == d["entry"]);
        assert_eq!(found, review, "the Needs-review filter, case {n}");
    }
    // drafts never post: the posted ledger is untouched
    assert_eq!(
        count(
            &mut b,
            "SELECT count(*) FROM entry WHERE payee = 'Synthetic Shop' AND status = 'posted'"
        ),
        0
    );
}

#[test]
fn refusals_write_nothing() {
    let (_tmp, mut b) = setup("receipt-refusals");
    let ev = evidence(&mut b, "r");
    let bad = [
        ("float overall", json!({"evidence":ev,"confidence":80.5})),
        ("float whole", json!({"evidence":ev,"confidence":80.0})),
        ("over 100", json!({"evidence":ev,"confidence":101})),
        ("negative", json!({"evidence":ev,"confidence":-1})),
        ("unknown key", json!({"evidence":ev,"model":"x"})),
        (
            "unknown field",
            json!({"evidence":ev,"fields":[{"field":"vat","value":"1"}]}),
        ),
        (
            "field key",
            json!({"evidence":ev,"fields":[{"field":"date","value":"2026-10-18","why":"x"}]}),
        ),
        (
            "duplicate",
            json!({"evidence":ev,"fields":[{"field":"date","value":"2026-10-18"},{"field":"date","value":"2026-10-19"}]}),
        ),
        (
            "money no cents",
            json!({"evidence":ev,"fields":[{"field":"amount","value":"150"}]}),
        ),
        (
            "money comma",
            json!({"evidence":ev,"fields":[{"field":"amount","value":"1,150.00"}]}),
        ),
        (
            "money number",
            json!({"evidence":ev,"fields":[{"field":"amount","value":150}]}),
        ),
        (
            "bad date",
            json!({"evidence":ev,"fields":[{"field":"date","value":"18/10/2026"}]}),
        ),
        (
            "oversized",
            json!({"evidence":ev,"fields":[{"field":"reference","value":"x".repeat(201)}]}),
        ),
        (
            "field confidence float",
            json!({"evidence":ev,"fields":[{"field":"date","value":"2026-10-18","confidence":0.9}]}),
        ),
        ("no evidence", json!({"confidence":90})),
        ("not evidence", json!({"evidence":"not-a-ticket"})),
    ];
    for (n, (what, x)) in bad.into_iter().enumerate() {
        let v = call_as(
            &mut b,
            Origin::Agent,
            tools::ENTRY_DRAFT,
            draft(&format!("receipt-bad-{n}"), x),
        );
        assert_eq!(v["isError"], true, "{what}: {v}");
    }
    for name in ["tin", "payee_tin", "tax_id"] {
        let tin = call_as(
            &mut b,
            Origin::Agent,
            tools::ENTRY_DRAFT,
            draft(
                &format!("receipt-tin-{name}"),
                json!({"evidence":ev,"fields":[{"field":name,"value":"synthetic"}]}),
            ),
        );
        assert!(
            ["tin_refused", "invalid_args"].contains(&code(&tin).as_str()),
            "{tin}"
        );
        assert!(!tin.to_string().contains("synthetic"), "{tin}");
    }
    // a TIN-shaped value is refused whatever field carries it; built at run time
    let tin_value = format!("{}-{}-{}-000", "123", "456", "789");
    for field in ["reference", "person", "category"] {
        let v = call_as(
            &mut b,
            Origin::Agent,
            tools::ENTRY_DRAFT,
            draft(
                &format!("receipt-tinval-{field}"),
                json!({"evidence":ev,"confidence":95,"fields":[{"field":field,"value":tin_value,"confidence":95}]}),
            ),
        );
        assert_eq!(code(&v), "tin_refused", "{field}: {v}");
        assert!(!v.to_string().contains(&tin_value), "{v}");
    }
    assert_eq!(
        count(
            &mut b,
            "SELECT count(*) FROM entry WHERE payee = 'Synthetic Shop'"
        ),
        0
    );
    assert_eq!(count(&mut b, "SELECT count(*) FROM entry_extraction"), 0);
    assert_eq!(count(&mut b, "SELECT count(*) FROM evidence_link"), 0);
    assert_eq!(
        count(
            &mut b,
            "SELECT count(*) FROM entry_extraction WHERE fields_json LIKE '%456%'"
        ),
        0
    );
}

#[test]
fn cross_book_and_closed_month_refused_atomically() {
    let (_tmp, mut b) = setup("receipt-cross");
    let ev = evidence(&mut b, "c");
    // evidence kept in another book: the draft itself is valid in book one, so
    // only the evidence's book can refuse it
    let second = books::create(b.store().unwrap().conn(), "Other", Basis::Cash, 1).unwrap();
    let t = books_store::plumbing::ticket_for(b.store().unwrap().conn(), "book", second).unwrap();
    let foreign = evidence_in(&mut b, "c2", Some(&t));
    let v = call_as(
        &mut b,
        Origin::Agent,
        tools::ENTRY_DRAFT,
        draft("cross-book-1", json!({"evidence":foreign,"confidence":90})),
    );
    assert_eq!(code(&v), "not_found", "{v}");
    // the same draft with book one's evidence goes through
    ok_as(
        &mut b,
        Origin::Agent,
        tools::ENTRY_DRAFT,
        draft("cross-book-ok", json!({"evidence":ev,"confidence":90})),
    );
    assert_eq!(count(&mut b, "SELECT count(*) FROM evidence_link"), 1);
    {
        let c = b.store().unwrap().conn();
        let book = books::all(c).unwrap().into_iter().next().unwrap();
        books_store::months::close(c, &book, "2026-09", None).unwrap();
    }
    let mut closed = draft("closed-month-1", json!({"evidence":ev,"confidence":90}));
    closed["date"] = json!("2026-09-10");
    let v = call_as(&mut b, Origin::Agent, tools::ENTRY_DRAFT, closed);
    assert_eq!(code(&v), "period_closed", "{v}");
    assert_eq!(
        count(
            &mut b,
            "SELECT count(*) FROM entry WHERE payee = 'Synthetic Shop'"
        ),
        1
    );
    assert_eq!(count(&mut b, "SELECT count(*) FROM evidence_link"), 1);
    assert_eq!(count(&mut b, "SELECT count(*) FROM entry_extraction"), 1);
    // a draft refused by the ledger after the extraction checked out rolls back too
    let mut unbalanced = draft("rollback-1", json!({"evidence":ev,"confidence":90}));
    unbalanced["category"] = json!("no-such-account");
    assert_eq!(
        call_as(&mut b, Origin::Agent, tools::ENTRY_DRAFT, unbalanced)["isError"],
        true
    );
    assert_eq!(count(&mut b, "SELECT count(*) FROM entry_extraction"), 1);
    assert_eq!(count(&mut b, "SELECT count(*) FROM evidence_link"), 1);
}

fn in_filter(b: &mut Books, filter: &str, entry: &Value) -> bool {
    let listed = ok_as(
        b,
        Origin::App,
        "books_ledger_list",
        json!({"filter":filter}),
    );
    listed["rows"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["ticket"] == *entry)
}

#[test]
fn assistant_edit_keeps_low_confidence_review() {
    let (_tmp, mut b) = setup("receipt-keep-review");
    let ev = evidence(&mut b, "k");
    let d = ok_as(
        &mut b,
        Origin::Agent,
        tools::ENTRY_DRAFT,
        draft(
            "keep-review-1",
            json!({"evidence":ev,"confidence":40,"fields":fields(json!(40))}),
        ),
    );
    assert!(in_filter(&mut b, "review", &d["entry"]));
    // the assistant changes only the memo: the draft still needs review
    let mut edit = draft("keep-review-2", Value::Null);
    edit["entry"] = d["entry"].clone();
    edit["memo"] = json!("synthetic memo");
    ok_as(&mut b, Origin::Agent, tools::ENTRY_DRAFT, edit);
    assert!(in_filter(&mut b, "review", &d["entry"]));
    assert!(!in_filter(&mut b, "draft", &d["entry"]));
    // the user's own edit in Books is the review
    let mut user = draft("keep-review-3", Value::Null);
    user["entry"] = d["entry"].clone();
    ok_as(&mut b, Origin::App, tools::ENTRY_DRAFT, user);
    assert!(!in_filter(&mut b, "review", &d["entry"]));
    assert!(in_filter(&mut b, "draft", &d["entry"]));
}

#[test]
fn replay_correction_and_responses() {
    let (_tmp, mut b) = setup("receipt-replay");
    let ev = evidence(&mut b, "p");
    let args = draft(
        "replay-1",
        json!({"evidence":ev,"confidence":70,"fields":fields(json!(70))}),
    );
    let first = ok_as(&mut b, Origin::Agent, tools::ENTRY_DRAFT, args.clone());
    assert_eq!(
        ok_as(&mut b, Origin::Agent, tools::ENTRY_DRAFT, args),
        first
    );
    assert_eq!(
        count(
            &mut b,
            "SELECT count(*) FROM entry WHERE payee = 'Synthetic Shop'"
        ),
        1
    );
    assert_eq!(count(&mut b, "SELECT count(*) FROM evidence_link"), 1);
    // a second extraction on the same draft is refused; history is not rewritten
    let mut again = draft("replay-2", json!({"evidence":ev,"confidence":99}));
    again["entry"] = first["entry"].clone();
    assert_eq!(
        code(&call_as(&mut b, Origin::Agent, tools::ENTRY_DRAFT, again)),
        "conflict"
    );

    // the user corrects the amount; the original reading survives
    let mut edit = draft("user-edit-1", Value::Null);
    edit["entry"] = first["entry"].clone();
    edit["amount"] = json!("155.00");
    ok_as(&mut b, Origin::App, tools::ENTRY_DRAFT, edit);
    let g = get(&mut b, Origin::App, &first["entry"]);
    let x = &g["extraction"];
    assert_eq!(x["proposed_by"], "Assistant");
    assert_eq!(x["confidence"], 70);
    assert_eq!(x["confidence_label"], "Unsure");
    assert_eq!(x["preview"], Value::Null);
    let amount = x["fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["field"] == "amount")
        .unwrap();
    assert_eq!(amount["suggested"], "150.00");
    assert_eq!(amount["current"], "155.00");
    assert_eq!(amount["corrected"], true);
    let date = x["fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["field"] == "date")
        .unwrap();
    assert_eq!(date["corrected"], false);
    assert_eq!(g["evidence"].as_array().unwrap().len(), 1);
    assert_eq!(g["evidence"][0]["evidence"], ev);
    assert_eq!(
        count(&mut b, "SELECT count(*) FROM entry_extraction_correction"),
        1
    );
    assert!(
        count(
            &mut b,
            "SELECT count(*) FROM entry_extraction WHERE fields_json LIKE '%150.00%'"
        ) == 1
    );

    // the assistant sees counts, not the extraction or evidence labels
    let a = get(&mut b, Origin::Agent, &first["entry"]);
    assert_eq!(a["evidence_count"], 1);
    assert_eq!(a["extracted"], true);
    assert!(a.get("extraction").is_none() && a.get("evidence").is_none());

    // only the app's post makes a ledger fact
    assert_eq!(
        count(
            &mut b,
            "SELECT count(*) FROM entry WHERE payee = 'Synthetic Shop' AND status = 'posted'"
        ),
        0
    );
    ok_as(
        &mut b,
        Origin::App,
        "books_app_post_entry",
        json!({"client_request_id":"receipt-post-1","entry":first["entry"]}),
    );
    assert_eq!(
        count(
            &mut b,
            "SELECT count(*) FROM entry WHERE payee = 'Synthetic Shop' AND status = 'posted'"
        ),
        1
    );
}

#[test]
fn plain_drafts_are_unchanged() {
    let (_tmp, mut b) = setup("receipt-plain");
    let d = ok_as(
        &mut b,
        Origin::Agent,
        tools::ENTRY_DRAFT,
        draft("receipt-plain-1", Value::Null),
    );
    assert!(d.get("extraction").is_none(), "{d}");
    let g = get(&mut b, Origin::App, &d["entry"]);
    assert_eq!(g["evidence"], json!([]));
    assert!(g.get("extraction").is_none());
    assert_eq!(
        count(
            &mut b,
            "SELECT count(*) FROM entry WHERE payee = 'Synthetic Shop' AND needs_review = 1"
        ),
        0
    );
}

#[test]
fn draft_card_reaches_review_drafts_and_hides_payee() {
    let src = books_mini_app::apps::find("action").unwrap().source;
    // the receipt card falls back to drafts waiting for review
    assert!(
        src.contains("filter: \"review\""),
        "review fallback missing"
    );
    assert!(
        src.contains("x.status === \"draft\""),
        "fallback must pick a draft"
    );
    // amount and payee are gated behind the explicit amounts request
    assert!(src.contains("const GATED = [\"amount\", \"person\"]"));
    assert!(!src.contains("import { Image") && !src.contains("<Image"));
}
