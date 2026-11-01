//! Synthetic owed-proposal invariants: drafts write nothing, the user accepts.
use books_core::ledger::Basis;
use books_mini_app::{
    b64,
    service::{Books, Origin},
    tools,
};
use books_store::{Paths, TempDir, books, months, plumbing};
use serde_json::{Value, json};

const DRAFT: &str = "books_owed_draft";
const CREATE: &str = "books_app_owed_create";
const LIST: &str = "books_owed_list";

fn call(b: &mut Books, origin: Origin, tool: &str, mut args: Value) -> Value {
    if let Some(id) = args["client_request_id"].as_str() {
        args["client_request_id"] = json!(format!("request-{id}"));
    }
    tools::call(tool, args.as_object().unwrap(), origin, b).unwrap()
}
fn ok(b: &mut Books, origin: Origin, tool: &str, args: Value) -> Value {
    let v = call(b, origin, tool, args);
    assert_eq!(v["isError"], false, "{v}");
    v["structuredContent"].clone()
}
fn refused(b: &mut Books, origin: Origin, tool: &str, args: Value) -> String {
    let v = call(b, origin, tool, args);
    assert_eq!(v["isError"], true, "{v}");
    v["structuredContent"]["code"]
        .as_str()
        .unwrap_or("")
        .to_string()
}

struct Book {
    ticket: String,
    evidence: String,
    vendor: String,
    customer: String,
    expense: String,
    income: String,
}

fn book(b: &mut Books, label: &str, basis: Basis) -> Book {
    let conn = b.store().unwrap().conn();
    let id = books::create(conn, label, basis, 1).unwrap();
    let ticket = plumbing::ticket_for(conn, "book", id).unwrap();
    let up = format!("upload-{label}-synthetic");
    ok(
        b,
        Origin::App,
        tools::EVIDENCE_ATTACH,
        json!({"client_request_id":format!("part-{label}"),"book":ticket,"op":"part","upload":up,"part":0,"parts":1,"data_base64":b64::encode(format!("%PDF-{label}").as_bytes())}),
    );
    let evidence = ok(
        b,
        Origin::App,
        tools::EVIDENCE_ATTACH,
        json!({"client_request_id":format!("commit-{label}"),"book":ticket,"op":"commit","upload":up,"mime":"application/pdf","kind":"bill"}),
    )["evidence"]
        .as_str()
        .unwrap()
        .to_string();
    let person = |b: &mut Books, kind: &str| {
        let plural = format!("{kind}s");
        ok(
            b,
            Origin::App,
            "books_app_person_create",
            json!({"client_request_id":format!("p-{label}-{kind}"),"book":ticket,"kind":plural,"name":format!("Synthetic {kind} {label}")}),
        )["person"]
            .as_str()
            .unwrap()
            .to_string()
    };
    let vendor = person(b, "vendor");
    let customer = person(b, "customer");
    let conn = b.store().unwrap().conn();
    let first = |t: &str| {
        let a: i64 = conn
            .query_row("SELECT account_id FROM account WHERE book_id=?1 AND type=?2 AND active=1 ORDER BY number LIMIT 1", books_store::rusqlite::params![id, t], |r| r.get(0))
            .unwrap();
        plumbing::ticket_for(conn, "account", a).unwrap()
    };
    let (expense, income) = (first("expense"), first("revenue"));
    Book {
        expense,
        income,
        ticket,
        evidence,
        vendor,
        customer,
    }
}

/// unpaid items, entries, lines, debit total, people
fn counts(b: &mut Books) -> [i64; 5] {
    let conn = b.store().unwrap().conn();
    let one = |sql: &str| conn.query_row(sql, [], |r| r.get::<_, i64>(0)).unwrap();
    [
        one("SELECT COUNT(*) FROM unpaid"),
        one("SELECT COUNT(*) FROM entry"),
        one("SELECT COUNT(*) FROM line"),
        one("SELECT COALESCE(SUM(debit),0) FROM line"),
        one("SELECT COUNT(*) FROM person"),
    ]
}

fn draft_args(k: &Book, crid: &str) -> Value {
    json!({
        "client_request_id": crid, "book": k.ticket, "evidence": k.evidence, "side": "owe",
        "issued": "2026-02-10", "due": "2026-03-10", "amount": "1200.00", "confidence": 87,
        "fields": [
            {"field":"reference","value":"SYN-0001","confidence":90},
            {"field":"person","value":format!("synthetic VENDOR {}", "A"),"confidence":70},
            {"field":"amount","value":"1200.00","confidence":95}
        ]
    })
}

fn accept(k: &Book, crid: &str, proposal: &str) -> Value {
    json!({"client_request_id":crid,"book":k.ticket,"proposal":proposal,"decision":"accept","person":k.vendor,"category":k.expense})
}

#[test]
fn draft_writes_nothing_and_acceptance_creates_one_owed_item() {
    for (basis, posts) in [(Basis::Cash, 0), (Basis::Accrual, 1)] {
        let tmp = TempDir::new("owed-proposals");
        let mut b = Books::new(Paths::in_dir(tmp.0.join("db")));
        let k = book(&mut b, "A", basis);
        let before = counts(&mut b);
        let d = ok(&mut b, Origin::Agent, DRAFT, draft_args(&k, "d1"));
        assert_eq!(d["status"], "draft");
        let proposal = d["proposal"].as_str().unwrap().to_string();
        assert_eq!(ok(&mut b, Origin::Agent, DRAFT, draft_args(&k, "d1")), d);
        assert_eq!(
            counts(&mut b),
            before,
            "a draft changes no totals, people or entries"
        );
        // the app and other origins never draft
        assert_eq!(
            refused(&mut b, Origin::App, DRAFT, draft_args(&k, "d2")),
            "forbidden"
        );

        let app = ok(
            &mut b,
            Origin::App,
            LIST,
            json!({"book":k.ticket,"side":"pay"}),
        );
        assert_eq!(app["pending_proposals"], 1);
        let p = &app["proposals"][0];
        assert_eq!(p["proposal"], proposal.as_str());
        assert_eq!(p["confidence"], 87);
        assert_eq!(p["amount"], "1200.00");
        assert_eq!(
            p["person_suggestion"]["ticket"],
            k.vendor.as_str(),
            "an exact name match suggests"
        );
        assert!(p["category"].is_null());
        assert!(
            p["fields"].to_string().find("VENDOR").is_none(),
            "the vendor text is not kept"
        );
        let agent = ok(
            &mut b,
            Origin::Agent,
            LIST,
            json!({"book":k.ticket,"side":"pay"}),
        );
        assert_eq!(agent["pending_proposals"], 1);
        assert!(agent.get("proposals").is_none(), "agent reads stay opaque");
        assert_eq!(counts(&mut b), before);

        // only the app decides, and only with the reviewed person and category
        for o in [
            Origin::Agent,
            Origin::Routine,
            Origin::Resolve,
            Origin::Unknown,
        ] {
            assert_eq!(
                refused(&mut b, o, CREATE, accept(&k, "a0", &proposal)),
                "forbidden"
            );
        }
        let mut no_person = accept(&k, "a1", &proposal);
        no_person.as_object_mut().unwrap().remove("person");
        assert_eq!(
            refused(&mut b, Origin::App, CREATE, no_person),
            "invalid_args"
        );
        let mut named = accept(&k, "a1b", &proposal);
        named.as_object_mut().unwrap().remove("person");
        named["person_name"] = json!("Synthetic New");
        refused(&mut b, Origin::App, CREATE, named);
        let mut no_category = accept(&k, "a2", &proposal);
        no_category.as_object_mut().unwrap().remove("category");
        assert_eq!(
            refused(&mut b, Origin::App, CREATE, no_category),
            "invalid_args"
        );
        assert_eq!(counts(&mut b), before);

        let a = ok(&mut b, Origin::App, CREATE, accept(&k, "a3", &proposal));
        assert_eq!(a["status"], "accepted");
        let after = counts(&mut b);
        assert_eq!(after[0], before[0] + 1, "exactly one owed item");
        assert_eq!(after[1], before[1] + posts, "only the accrual path posts");
        assert_eq!(after[4], before[4], "no person made");
        assert_eq!(
            ok(&mut b, Origin::App, CREATE, accept(&k, "a3", &proposal)),
            a
        );
        assert_eq!(
            refused(&mut b, Origin::App, CREATE, accept(&k, "a4", &proposal)),
            "already_accepted"
        );
        let discard = json!({"client_request_id":"x1","book":k.ticket,"proposal":proposal,"decision":"discard"});
        assert_eq!(
            refused(&mut b, Origin::App, CREATE, discard),
            "already_accepted"
        );
        assert_eq!(
            counts(&mut b),
            after,
            "replay never makes a second owed item"
        );

        let app = ok(
            &mut b,
            Origin::App,
            LIST,
            json!({"book":k.ticket,"side":"pay"}),
        );
        assert_eq!(app["pending_proposals"], 0);
        let row = app["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["ticket"] == a["owed"])
            .unwrap();
        assert_eq!(row["evidence"].as_array().unwrap().len(), 1, "{row}");
        let conn = b.store().unwrap().conn();
        let linked: i64 = conn
            .query_row("SELECT COUNT(*) FROM evidence_link l JOIN owed_proposal p ON p.accepted_owed_id = l.owed_id AND p.evidence_id = l.evidence_id", [], |r| r.get(0))
            .unwrap();
        assert_eq!(linked, 1);
        // the existing create path without a proposal is unchanged
        let plain = json!({"client_request_id":"plain","book":k.ticket,"side":"collect","amount":"10.00","category":k.income,"person":k.customer});
        ok(&mut b, Origin::App, CREATE, plain);
        assert_eq!(counts(&mut b)[0], after[0] + 1);
    }
}

#[test]
fn discard_changes_the_proposal_only() {
    let tmp = TempDir::new("owed-proposals-discard");
    let mut b = Books::new(Paths::in_dir(tmp.0.join("db")));
    let k = book(&mut b, "A", Basis::Accrual);
    let mut args = draft_args(&k, "d1");
    args["side"] = json!("collect");
    args["person_suggestion"] = json!(k.customer);
    args["category"] = json!(k.income);
    let proposal = ok(&mut b, Origin::Agent, DRAFT, args)["proposal"]
        .as_str()
        .unwrap()
        .to_string();
    let before = counts(&mut b);
    let discard =
        json!({"client_request_id":"x1","book":k.ticket,"proposal":proposal,"decision":"discard"});
    let d = ok(&mut b, Origin::App, CREATE, discard.clone());
    assert_eq!(d["status"], "discarded");
    assert_eq!(ok(&mut b, Origin::App, CREATE, discard), d);
    let mut acc = accept(&k, "a1", &proposal);
    acc["person"] = json!(k.customer);
    acc["category"] = json!(k.income);
    assert_eq!(refused(&mut b, Origin::App, CREATE, acc), "discarded");
    assert_eq!(counts(&mut b), before);
    let app = ok(
        &mut b,
        Origin::App,
        LIST,
        json!({"book":k.ticket,"side":"collect"}),
    );
    assert_eq!(app["proposals"].as_array().unwrap().len(), 0);
}

#[test]
fn invalid_and_cross_book_drafts_and_decisions_are_refused() {
    let tmp = TempDir::new("owed-proposals-invalid");
    let mut b = Books::new(Paths::in_dir(tmp.0.join("db")));
    let k = book(&mut b, "A", Basis::Cash);
    let other = book(&mut b, "B", Basis::Cash);
    let before = counts(&mut b);
    const BAD: &str = "invalid_args";
    const GONE: &str = "not_found";
    let bad: Vec<(&str, Value, &str)> = vec![
        ("issued", json!("2026-02-30"), BAD),
        ("issued", json!("10/02/2026"), BAD),
        ("due", json!("2026-01-01"), BAD),
        ("amount", json!("0.00"), BAD),
        ("amount", json!("-5.00"), BAD),
        ("amount", json!("12.345"), BAD),
        ("amount", json!(12), BAD),
        ("confidence", json!(101), BAD),
        ("confidence", json!(50.5), BAD),
        ("confidence", json!("50"), BAD),
        ("confidence", json!(-1), BAD),
        ("side", json!("pay"), BAD),
        (
            "fields",
            json!([{"field":"vendor_tin","value":"x","confidence":1}]),
            BAD,
        ),
        (
            "fields",
            json!([{"field":"amount","value":"1.00","confidence":101}]),
            BAD,
        ),
        ("unknown", json!("x"), BAD),
        ("evidence", json!(other.evidence), GONE),
        ("person_suggestion", json!(other.vendor), GONE),
        ("person_suggestion", json!(k.customer), GONE),
        ("category", json!(other.expense), GONE),
    ];
    for (n, (key, value, code)) in bad.into_iter().enumerate() {
        let mut args = draft_args(&k, &format!("bad-{n}"));
        args[key] = value.clone();
        assert_eq!(
            refused(&mut b, Origin::Agent, DRAFT, args),
            code,
            "{key}={value}"
        );
    }
    let mut missing = draft_args(&k, "missing");
    missing.as_object_mut().unwrap().remove("confidence");
    assert_eq!(refused(&mut b, Origin::Agent, DRAFT, missing), BAD);
    let conn = b.store().unwrap().conn();
    let n: i64 = conn
        .query_row("SELECT COUNT(*) FROM owed_proposal", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 0);

    let proposal = ok(&mut b, Origin::Agent, DRAFT, draft_args(&k, "good"))["proposal"]
        .as_str()
        .unwrap()
        .to_string();
    for (n, (key, value, code)) in [
        ("person", json!(other.vendor), GONE),
        ("person", json!(k.customer), GONE),
        ("category", json!(other.expense), GONE),
        ("book", json!(other.ticket), GONE),
        ("issued", json!("2026-13-01"), BAD),
        ("due", json!("31/03/2026"), BAD),
        ("decision", json!("maybe"), BAD),
    ]
    .into_iter()
    .enumerate()
    {
        let mut args = accept(&k, &format!("acc-{n}"), &proposal);
        args[key] = value.clone();
        assert_eq!(
            refused(&mut b, Origin::App, CREATE, args),
            code,
            "{key}={value}"
        );
    }
    assert_eq!(counts(&mut b), before);
}

/// A TIN shape built at run time, so no source file carries one.
fn synthetic_tin() -> String {
    let (a, b, c) = ("123", "456", "789");
    format!("{a}-{b}-{c}")
}

fn proposal_rows(b: &mut Books) -> i64 {
    let conn = b.store().unwrap().conn();
    conn.query_row("SELECT COUNT(*) FROM owed_proposal", [], |r| r.get(0))
        .unwrap()
}

#[test]
fn tin_shaped_text_is_never_stored() {
    let tmp = TempDir::new("owed-proposals-tin");
    let mut b = Books::new(Paths::in_dir(tmp.0.join("db")));
    let k = book(&mut b, "A", Basis::Cash);
    for (n, field) in ["reference", "issued", "due", "amount"]
        .into_iter()
        .enumerate()
    {
        let mut args = draft_args(&k, &format!("tin-{n}"));
        args["fields"] =
            json!([{"field":field,"value":format!("TIN {}", synthetic_tin()),"confidence":80}]);
        assert_eq!(
            refused(&mut b, Origin::Agent, DRAFT, args),
            "tin_refused",
            "{field}"
        );
    }
    assert_eq!(proposal_rows(&mut b), 0);

    let proposal = ok(&mut b, Origin::Agent, DRAFT, draft_args(&k, "clean"))["proposal"]
        .as_str()
        .unwrap()
        .to_string();
    let before = counts(&mut b);
    let mut args = accept(&k, "tin-accept", &proposal);
    args["reference"] = json!(synthetic_tin());
    assert_eq!(refused(&mut b, Origin::App, CREATE, args), "tin_refused");
    assert_eq!(counts(&mut b), before);
    let app = ok(
        &mut b,
        Origin::App,
        LIST,
        json!({"book":k.ticket,"side":"pay"}),
    );
    assert_eq!(app["pending_proposals"], 1, "the proposal stays a draft");
}

#[test]
fn same_name_people_get_no_suggestion_and_due_can_be_cleared() {
    let tmp = TempDir::new("owed-proposals-twins");
    let mut b = Books::new(Paths::in_dir(tmp.0.join("db")));
    let k = book(&mut b, "A", Basis::Cash);
    let conn = b.store().unwrap().conn();
    let id = match plumbing::subject(conn, &k.ticket).unwrap() {
        Some((_, id)) => id,
        None => unreachable!(),
    };
    // the store's name key is case-sensitive; the matcher is not
    for name in ["Synthetic Twin", "synthetic twin"] {
        conn.execute(
            "INSERT INTO person(book_id,kind,name,created_at) VALUES (?1,'vendor',?2,'2026-01-01T00:00:00Z')",
            books_store::rusqlite::params![id, name],
        )
        .unwrap();
    }
    let mut args = draft_args(&k, "twins");
    args["fields"] = json!([{"field":"person","value":"SYNTHETIC TWIN","confidence":90}]);
    ok(&mut b, Origin::Agent, DRAFT, args);
    let mut one = draft_args(&k, "single");
    one["fields"] = json!([{"field":"person","value":" synthetic VENDOR a ","confidence":90}]);
    ok(&mut b, Origin::Agent, DRAFT, one);
    let app = ok(
        &mut b,
        Origin::App,
        LIST,
        json!({"book":k.ticket,"side":"pay"}),
    );
    let list = app["proposals"].as_array().unwrap();
    assert_eq!(list.len(), 2);
    assert!(
        list[0]["person_suggestion"].is_null(),
        "two matches suggest no one"
    );
    assert_eq!(list[1]["person_suggestion"]["ticket"], json!(k.vendor));

    let proposal = list[0]["proposal"].as_str().unwrap().to_string();
    let mut cleared = accept(&k, "cleared", &proposal);
    cleared["due"] = json!("");
    ok(&mut b, Origin::App, CREATE, cleared);
    let conn = b.store().unwrap().conn();
    let due: Option<String> = conn
        .query_row("SELECT due FROM unpaid", [], |r| r.get(0))
        .unwrap();
    assert_eq!(due, None, "the user cleared the suggested due date");
}

#[test]
fn a_closed_month_refuses_acceptance() {
    let tmp = TempDir::new("owed-proposals-closed");
    let mut b = Books::new(Paths::in_dir(tmp.0.join("db")));
    for basis in [Basis::Cash, Basis::Accrual] {
        let label = format!("{basis:?}");
        let k = book(&mut b, &label, basis);
        let proposal = ok(
            &mut b,
            Origin::Agent,
            DRAFT,
            draft_args(&k, &format!("d-{label}")),
        )["proposal"]
            .as_str()
            .unwrap()
            .to_string();
        let conn = b.store().unwrap().conn();
        let id = match plumbing::subject(conn, &k.ticket).unwrap() {
            Some((_, id)) => id,
            None => unreachable!(),
        };
        let bk = books::get(conn, id).unwrap().unwrap();
        months::close(conn, &bk, "2026-02", Some("synthetic close")).unwrap();
        let before = counts(&mut b);
        assert_eq!(
            refused(
                &mut b,
                Origin::App,
                CREATE,
                accept(&k, &format!("a-{label}"), &proposal),
            ),
            "period_closed"
        );
        assert_eq!(counts(&mut b), before);
        let app = ok(
            &mut b,
            Origin::App,
            LIST,
            json!({"book":k.ticket,"side":"pay"}),
        );
        assert_eq!(app["pending_proposals"], 1, "the proposal stays a draft");
        // a corrected issue date in an open month is the user's choice
        let mut moved = accept(&k, &format!("m-{label}"), &proposal);
        moved["issued"] = json!("2026-03-01");
        moved["due"] = json!("2026-03-31");
        ok(&mut b, Origin::App, CREATE, moved);
        assert_eq!(counts(&mut b)[0], before[0] + 1);
    }
}

#[test]
fn the_assistant_sees_the_draft_tool_but_not_app_acceptance() {
    let list = tools::list();
    let find = |n: &str| list.iter().find(|t| t["name"] == n).unwrap().clone();
    let draft = find(DRAFT);
    assert!(draft.get("_meta").is_none());
    assert_eq!(draft["annotations"]["readOnlyHint"], false);
    assert_eq!(find(CREATE)["_meta"]["buwiz/audience"], json!(["app"]));
    assert!(find(CREATE)["inputSchema"]["properties"]["decision"].is_object());
}

#[test]
fn unpaid_reviews_proposals_without_making_people() {
    let page = include_str!("../apps/books/unpaid.tsx");
    let row =
        &page[page.find("function ProposalRow").unwrap()..page.find("function OwedSheet").unwrap()];
    for needle in [
        "decision",
        "\"accept\"",
        "\"discard\"",
        "p.p.confidence",
        "p.p.source",
        "args.issued",
        "args.due",
        "args.category",
        "args.person",
    ] {
        assert!(row.contains(needle), "{needle}");
    }
    assert!(
        !row.contains("person_name"),
        "a proposal never makes a person"
    );
    assert!(page.contains("d()?.proposals"));
    assert!(page.contains("r().evidence"));
}
