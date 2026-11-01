//! Synthetic evidence foundation invariants; no host or financial data.
use books_core::date::Date;
use books_core::ledger::Basis;
use books_mini_app::{
    b64, cards,
    service::{Books, Origin},
    tools,
};
use books_store::{Paths, TempDir, books, entries, evidence, months, plumbing};
use serde_json::{Value, json};

fn call(b: &mut Books, tool: &str, args: Value) -> Value {
    tools::call(tool, args.as_object().unwrap(), Origin::App, b).unwrap()
}
fn ok(b: &mut Books, tool: &str, args: Value) -> Value {
    let v = call(b, tool, args);
    assert_eq!(v["isError"], false, "{v}");
    v["structuredContent"].clone()
}
fn setup() -> (TempDir, Books, i64) {
    let tmp = TempDir::new("evidence-foundation");
    let mut b = Books::new(Paths::in_dir(tmp.0.join("db")));
    let book = books::create(b.store().unwrap().conn(), "Synthetic", Basis::Cash, 1).unwrap();
    (tmp, b, book)
}
fn stage(b: &mut Books, upload: &str, bytes: &[u8]) {
    ok(
        b,
        tools::EVIDENCE_ATTACH,
        json!({"client_request_id":format!("part-{upload}"),"op":"part","upload":upload,"part":0,"parts":1,"data_base64":b64::encode(bytes)}),
    );
}
fn commit(b: &mut Books, upload: &str) -> Value {
    ok(
        b,
        tools::EVIDENCE_ATTACH,
        json!({"client_request_id":format!("commit-{upload}"),"op":"commit","upload":upload,"mime":"application/pdf","kind":"receipt","page_count":2}),
    )
}

#[test]
fn multipart_retry_validation_and_limits() {
    use books_mini_app::oldbooks::{MAX_UPLOAD, Uploads};
    let mut u = Uploads::default();
    assert_eq!(u.stage("synthetic", 1, 2, b"B".to_vec()).unwrap(), 1);
    assert_eq!(u.stage("synthetic", 1, 2, b"B".to_vec()).unwrap(), 1);
    assert!(u.stage("synthetic", 0, 3, b"A".to_vec()).is_err());
    assert!(u.take("synthetic").is_err());
    assert_eq!(u.stage("synthetic", 0, 2, b"A".to_vec()).unwrap(), 2);
    assert_eq!(u.take("synthetic").unwrap(), b"AB");
    assert!(u.stage("boundary", 0, 1, vec![0; MAX_UPLOAD]).is_ok());
    assert!(u.stage("boundary", 0, 1, vec![0; MAX_UPLOAD]).is_ok());
    assert_eq!(u.take("boundary").unwrap().len(), MAX_UPLOAD);
    assert!(u.stage("boundary", 0, 1, vec![0; MAX_UPLOAD + 1]).is_err());
    let (_tmp, mut b, _) = setup();
    assert_eq!(
        call(
            &mut b,
            tools::EVIDENCE_ATTACH,
            json!({"client_request_id":"invalid-b64","op":"part","upload":"synthetic","part":0,"parts":1,"data_base64":"!"})
        )["isError"],
        true
    );
    for (n, mime, bytes) in [
        (0, "text/plain", b"%PDF-x".as_slice()),
        (1, "application/pdf", b"not a PDF".as_slice()),
        (2, "image/png", b"%PDF-x".as_slice()),
    ] {
        let upload = format!("invalid-{n}");
        stage(&mut b, &upload, bytes);
        assert_eq!(
            call(
                &mut b,
                tools::EVIDENCE_ATTACH,
                json!({"client_request_id":format!("bad-commit-{n}"),"op":"commit","upload":upload,"mime":mime})
            )["isError"],
            true
        );
    }
    for tool in [tools::EVIDENCE_ATTACH, tools::EVIDENCE_DETACH] {
        let v = tools::call(tool, json!({}).as_object().unwrap(), Origin::Agent, &mut b).unwrap();
        assert_eq!(v["isError"], true);
        assert_eq!(tools::find(tool).unwrap().reach, tools::Reach::App);
    }
}

#[test]
fn commits_dedup_reads_pagination_and_scope_do_not_write() {
    let (_tmp, mut b, book) = setup();
    assert_eq!(
        ok(&mut b, tools::EVIDENCE_LIST, json!({}))["items"],
        json!([])
    );
    stage(&mut b, "document-a", b"%PDF-synthetic-a");
    let first = commit(&mut b, "document-a");
    assert_eq!(commit(&mut b, "document-a"), first);
    stage(&mut b, "document-b", b"%PDF-synthetic-a");
    let duplicate = commit(&mut b, "document-b");
    assert_eq!(duplicate["evidence"], first["evidence"]);
    assert_eq!(duplicate["deduplicated"], true);
    stage(&mut b, "document-c", b"%PDF-synthetic-c");
    commit(&mut b, "document-c");
    let before = b.store().unwrap().conn().total_changes();
    let list = ok(&mut b, tools::EVIDENCE_LIST, json!({"limit":1}));
    assert_eq!(list["items"].as_array().unwrap().len(), 1);
    assert!(list["next_cursor"].is_string());
    let second = ok(
        &mut b,
        tools::EVIDENCE_LIST,
        json!({"limit":1,"cursor":list["next_cursor"]}),
    );
    assert_eq!(second["items"].as_array().unwrap().len(), 1);
    assert!(second["next_cursor"].is_null());
    assert!(list.to_string().len() < 1024 * 1024);
    for item in list["items"].as_array().unwrap() {
        assert_eq!(item.as_object().unwrap().len(), 6);
        for key in ["bytes", "sha256", "filename", "hash", "text"] {
            assert!(item.get(key).is_none());
        }
    }
    let store = b.store().unwrap();
    assert_eq!(
        cards::scope(store, "evidence", first["evidence"].as_str().unwrap())["result"],
        "token"
    );
    assert_eq!(
        cards::scope(store, "evidence", "Receipt")["choices"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        cards::scope(store, "evidence", &plumbing::new_ticket().unwrap())["result"],
        "error"
    );
    assert_eq!(store.conn().total_changes(), before);
    let foreign = books::create(store.conn(), "Synthetic other", Basis::Cash, 1).unwrap();
    let t = plumbing::ticket_for(store.conn(), "book", foreign).unwrap();
    assert_eq!(
        call(
            &mut b,
            tools::EVIDENCE_LIST,
            json!({"book":t,"evidence":first["evidence"]})
        )["isError"],
        true
    );
    assert_eq!(
        evidence::list(b.store().unwrap().conn(), book, None, None, 0, 50)
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn links_detach_closed_month_and_cross_book_rollback() {
    let (_tmp, mut b, book_id) = setup();
    stage(&mut b, "document-a", b"%PDF-synthetic-a");
    let a = commit(&mut b, "document-a");
    stage(&mut b, "document-b", b"%PDF-synthetic-b");
    let other = commit(&mut b, "document-b");
    let c = b.store().unwrap().conn();
    let book = books::get(c, book_id).unwrap().unwrap();
    let entry = entries::save_draft(
        c,
        &book,
        None,
        &entries::DraftInput {
            date: Date::new(2026, 1, 2).unwrap(),
            form: entries::Form::Journal,
            payee: None,
            memo: None,
            reference: None,
            needs_review: false,
            lines: vec![],
        },
        "app",
    )
    .unwrap();
    let entry_t = plumbing::ticket_for(c, "entry", entry).unwrap();
    let category = books::account_by_number(c, book_id, "41000")
        .unwrap()
        .unwrap();
    let owed = books_store::people::create_unpaid(
        c,
        &book,
        &books_store::people::UnpaidInput {
            side: "collect",
            person_id: None,
            reference: None,
            memo: None,
            issued: Date::new(2026, 1, 2).unwrap(),
            due: None,
            amount: 100,
            category_account_id: category.id,
        },
        &|_| Ok(()),
    )
    .unwrap();
    let owed_t = plumbing::ticket_for(c, "owed", owed).unwrap();
    let target = json!({"kind":"entry","ticket":entry_t});
    for (n, e) in [(0, &a), (1, &other)] {
        ok(
            &mut b,
            tools::EVIDENCE_ATTACH,
            json!({"client_request_id":format!("link-request-{n}"),"op":"link","evidence":e["evidence"],"target":target}),
        );
    }
    ok(
        &mut b,
        tools::EVIDENCE_ATTACH,
        json!({"client_request_id":"link-owed-request","op":"link","evidence":a["evidence"],"target":{"kind":"owed","ticket":owed_t}}),
    );
    let linked = ok(
        &mut b,
        tools::EVIDENCE_LIST,
        json!({"evidence":a["evidence"]}),
    );
    assert_eq!(linked["items"][0]["linked_count"], 2);
    assert_eq!(
        ok(&mut b, tools::EVIDENCE_LIST, json!({"target":target}))["items"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        ok(
            &mut b,
            tools::EVIDENCE_LIST,
            json!({"target":{"kind":"owed","ticket":owed_t}})
        )["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let detach =
        json!({"client_request_id":"detach-request","evidence":a["evidence"],"target":target});
    let result = ok(&mut b, tools::EVIDENCE_DETACH, detach.clone());
    assert_eq!(ok(&mut b, tools::EVIDENCE_DETACH, detach), result);
    let c = b.store().unwrap().conn();
    entries::discard(c, &book, entry).unwrap();
    months::close(c, &book, "2026-01", None).unwrap();
    for (tool, op) in [
        (tools::EVIDENCE_ATTACH, "link"),
        (tools::EVIDENCE_DETACH, "detach"),
    ] {
        let count: i64 = b
            .store()
            .unwrap()
            .conn()
            .query_row("SELECT count(*) FROM evidence_link", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            call(
                &mut b,
                tool,
                json!({"client_request_id":format!("closed-{op}"),"op":op,"evidence":other["evidence"],"target":target})
            )["isError"],
            true
        );
        assert_eq!(
            b.store()
                .unwrap()
                .conn()
                .query_row("SELECT count(*) FROM evidence_link", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            count
        );
    }
    let c = b.store().unwrap().conn();
    assert_eq!(
        evidence::list(c, book_id, None, None, 0, 50).unwrap().len(),
        2
    );
    let foreign = books::create(c, "Synthetic other", Basis::Cash, 1).unwrap();
    let foreign_t = plumbing::ticket_for(c, "book", foreign).unwrap();
    let foreign_book = books::get(c, foreign).unwrap().unwrap();
    let foreign_entry = entries::save_draft(
        c,
        &foreign_book,
        None,
        &entries::DraftInput {
            date: Date::new(2026, 2, 1).unwrap(),
            form: entries::Form::Journal,
            payee: None,
            memo: None,
            reference: None,
            needs_review: false,
            lines: vec![],
        },
        "app",
    )
    .unwrap();
    let foreign_entry_t = plumbing::ticket_for(c, "entry", foreign_entry).unwrap();
    assert_eq!(
        call(
            &mut b,
            tools::EVIDENCE_ATTACH,
            json!({"client_request_id":"foreign-target","op":"link","evidence":a["evidence"],"target":{"kind":"entry","ticket":foreign_entry_t}})
        )["isError"],
        true
    );
    assert_eq!(
        call(
            &mut b,
            tools::EVIDENCE_ATTACH,
            json!({"client_request_id":"foreign-link","book":foreign_t,"op":"link","evidence":a["evidence"],"target":target})
        )["isError"],
        true
    );
    stage(&mut b, "rollback-document", b"%PDF-synthetic-rollback");
    let before: i64 = b
        .store()
        .unwrap()
        .conn()
        .query_row("SELECT count(*) FROM evidence", [], |r| r.get(0))
        .unwrap();
    let tickets: i64 = b
        .store()
        .unwrap()
        .conn()
        .query_row("SELECT count(*) FROM ticket", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        call(
            &mut b,
            tools::EVIDENCE_ATTACH,
            json!({"client_request_id":"rollback-commit","op":"commit","upload":"rollback-document","mime":"application/pdf","target":target})
        )["isError"],
        true
    );
    let c = b.store().unwrap().conn();
    assert_eq!(
        c.query_row("SELECT count(*) FROM evidence", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        before
    );
    assert_eq!(
        c.query_row("SELECT count(*) FROM ticket", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        tickets
    );
    for tool in [tools::EVIDENCE_ATTACH, tools::EVIDENCE_DETACH] {
        assert_eq!(
            call(
                &mut b,
                tool,
                json!({"client_request_id":format!("owed-closed-{tool}"),"op":"link","evidence":a["evidence"],"target":{"kind":"owed","ticket":owed_t}})
            )["isError"],
            true
        );
    }
}
