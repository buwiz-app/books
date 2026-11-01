//! Evidence classification review and the static preview (evidence-4); synthetic data only.
use books_core::date::Date;
use books_core::ledger::Basis;
use books_mini_app::{
    b64,
    service::{Books, Origin},
    tools::{self, document_review as dr},
};
use books_store::{Paths, TempDir, books, entries, months, plumbing};
use serde_json::{Value, json};

fn call_as(b: &mut Books, origin: Origin, tool: &str, args: Value) -> Value {
    tools::call(tool, args.as_object().unwrap(), origin, b).unwrap()
}
fn call(b: &mut Books, tool: &str, args: Value) -> Value {
    call_as(b, Origin::App, tool, args)
}
fn ok(b: &mut Books, tool: &str, args: Value) -> Value {
    let v = call(b, tool, args);
    assert_eq!(v["isError"], false, "{v}");
    v["structuredContent"].clone()
}
fn setup() -> (TempDir, Books, i64) {
    let tmp = TempDir::new("document-review");
    let mut b = Books::new(Paths::in_dir(tmp.0.join("db")));
    let book = books::create(b.store().unwrap().conn(), "Synthetic", Basis::Cash, 1).unwrap();
    (tmp, b, book)
}
fn add(b: &mut Books, upload: &str, mime: &str, bytes: &[u8]) -> String {
    ok(
        b,
        tools::EVIDENCE_ATTACH,
        json!({"client_request_id":format!("part-{upload}"),"op":"part","upload":upload,"part":0,"parts":1,"data_base64":b64::encode(bytes)}),
    );
    ok(
        b,
        tools::EVIDENCE_ATTACH,
        json!({"client_request_id":format!("commit-{upload}"),"op":"commit","upload":upload,"mime":mime,"kind":"other"}),
    )["evidence"]
    .as_str()
    .unwrap()
    .to_string()
}
fn kind_of(b: &mut Books, evidence: &str) -> String {
    ok(b, tools::EVIDENCE_LIST, json!({"evidence":evidence}))["items"][0]["kind"]
        .as_str()
        .unwrap()
        .to_string()
}
fn propose(b: &mut Books, crid: &str, evidence: &str, kind: &str, confidence: Value) -> Value {
    call_as(
        b,
        Origin::Agent,
        tools::EVIDENCE_CLASSIFY,
        json!({"client_request_id":crid,"evidence":evidence,"kind":kind,"confidence":confidence}),
    )
}
const PNG: &[u8] = b"\x89PNG\r\n\x1a\nsynthetic-image";

#[test]
fn assistant_proposal_never_changes_the_kind() {
    let (_tmp, mut b, _) = setup();
    let ev = add(&mut b, "document-a", "image/png", PNG);
    let p = propose(&mut b, "classify-1", &ev, "receipt", json!(87));
    assert_eq!(p["isError"], false, "{p}");
    let s = &p["structuredContent"];
    assert_eq!(s["status"], "draft");
    assert!(plumbing::is_ticket(s["proposal"].as_str().unwrap()));
    assert!(s.get("_open").is_none());
    let open = &p["_meta"]["buwiz/open"];
    assert_eq!(open["app"], dr::CARD_APP);
    assert_eq!(open["props"]["mode"], "review");
    assert_eq!(open["props"]["proposal"], s["proposal"]);
    assert_eq!(open["placement"], "inline");
    // a repeat replays; no second proposal
    let again = propose(&mut b, "classify-1", &ev, "receipt", json!(87));
    assert_eq!(again["structuredContent"]["proposal"], s["proposal"]);
    assert_eq!(kind_of(&mut b, &ev), "other");
    let rows = ok(&mut b, tools::EVIDENCE_REVIEW, json!({"evidence":ev}))["rows"].clone();
    assert_eq!(rows.as_array().unwrap().len(), 1);
    assert_eq!(rows[0]["confidence"], 87);
    assert_eq!(rows[0]["route"], "ledger_draft");
}

#[test]
fn classify_refuses_bad_requests() {
    let (_tmp, mut b, _) = setup();
    let ev = add(&mut b, "document-a", "image/png", PNG);
    for (n, kind, conf) in [
        (0, "tax doc", json!(50)),
        (1, "receipt", json!(50.5)),
        (2, "receipt", json!(101)),
        (3, "receipt", json!(-1)),
        (4, "receipt", json!("50")),
    ] {
        let v = propose(&mut b, &format!("bad-request-{n}"), &ev, kind, conf);
        assert_eq!(v["isError"], true, "{n}: {v}");
        assert_eq!(v["structuredContent"]["code"], "invalid_args");
    }
    // the app cannot pose as the assistant, and the assistant cannot decide
    let v = call(
        &mut b,
        tools::EVIDENCE_CLASSIFY,
        json!({"client_request_id":"app-poses","evidence":ev,"kind":"bill","confidence":10}),
    );
    assert_eq!(v["structuredContent"]["code"], "forbidden");
    let p = propose(&mut b, "classify-ok", &ev, "bill", json!(10))["structuredContent"]["proposal"]
        .clone();
    let v = call_as(
        &mut b,
        Origin::Agent,
        tools::EVIDENCE_KIND,
        json!({"client_request_id":"agent-decides","proposal":p,"decision":"accept"}),
    );
    assert_eq!(v["isError"], true);
    assert_eq!(kind_of(&mut b, &ev), "other");
    // cross-book evidence and proposals refuse
    let c = b.store().unwrap().conn();
    let foreign = books::create(c, "Synthetic two", Basis::Cash, 1).unwrap();
    let fb = plumbing::ticket_for(c, "book", foreign).unwrap();
    let v = call_as(
        &mut b,
        Origin::Agent,
        tools::EVIDENCE_CLASSIFY,
        json!({"client_request_id":"cross-book","book":fb,"evidence":ev,"kind":"bill","confidence":10}),
    );
    assert_eq!(v["structuredContent"]["code"], "not_found");
    let v = call(
        &mut b,
        tools::EVIDENCE_KIND,
        json!({"client_request_id":"x-book-kind","book":fb,"proposal":p,"decision":"accept"}),
    );
    assert_eq!(v["structuredContent"]["code"], "not_found");
    let v = call(
        &mut b,
        tools::EVIDENCE_KIND,
        json!({"client_request_id":"bad-kind","proposal":p,"decision":"accept","kind":"tax doc"}),
    );
    assert_eq!(v["structuredContent"]["code"], "invalid_args");
    assert_eq!(kind_of(&mut b, &ev), "other");
}

#[test]
fn accept_and_discard_are_idempotent_and_audited() {
    let (_tmp, mut b, _) = setup();
    let ev = add(&mut b, "document-a", "image/png", PNG);
    let p =
        propose(&mut b, "classify-c1", &ev, "receipt", json!(60))["structuredContent"]["proposal"]
            .clone();
    let accept =
        json!({"client_request_id":"accept-1","proposal":p,"decision":"accept","kind":"bill"});
    let first = ok(&mut b, tools::EVIDENCE_KIND, accept.clone());
    assert_eq!(
        first,
        json!({"proposal":p,"status":"accepted","kind":"bill","route":"unpaid_review"})
    );
    assert_eq!(ok(&mut b, tools::EVIDENCE_KIND, accept), first);
    assert_eq!(kind_of(&mut b, &ev), "bill");
    // a decided proposal cannot be decided again under a new request
    let v = call(
        &mut b,
        tools::EVIDENCE_KIND,
        json!({"client_request_id":"discard-late","proposal":p,"decision":"discard"}),
    );
    assert_eq!(v["structuredContent"]["code"], "conflict");
    let (prev, proposed): (String, String) = b
        .store()
        .unwrap()
        .conn()
        .query_row(
            "SELECT previous_kind,proposed_kind FROM evidence_classification",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!((prev.as_str(), proposed.as_str()), ("other", "receipt"));
    // discard keeps the kind; repeats replay
    let q =
        propose(&mut b, "classify-c2", &ev, "tax_form", json!(40))["structuredContent"]["proposal"]
            .clone();
    let discard = json!({"client_request_id":"discard-1","proposal":q,"decision":"discard"});
    let d = ok(&mut b, tools::EVIDENCE_KIND, discard.clone());
    assert_eq!(d, json!({"proposal":q,"status":"discarded"}));
    assert_eq!(ok(&mut b, tools::EVIDENCE_KIND, discard), d);
    assert_eq!(kind_of(&mut b, &ev), "bill");
    // routing is navigation only: nothing was recorded anywhere
    let c = b.store().unwrap().conn();
    for table in ["entry", "unpaid", "statement_line", "certificate"] {
        let n: i64 = c
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap_or(0);
        assert_eq!(n, 0, "{table}");
    }
    for (kind, route) in [
        ("receipt", "ledger_draft"),
        ("invoice", "ledger_draft"),
        ("bill", "unpaid_review"),
        ("statement", "statement_import"),
        ("tax_form", "certificate_review"),
        ("contract", "stored"),
        ("payslip", "stored"),
        ("other", "stored"),
    ] {
        assert_eq!(dr::route(kind), route);
    }
}

fn draft(b: &mut Books, book_id: i64) -> (i64, String) {
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
    (entry, plumbing::ticket_for(c, "entry", entry).unwrap())
}

#[test]
fn accept_refuses_a_closed_linked_target() {
    let (_tmp, mut b, book_id) = setup();
    let ev = add(&mut b, "document-a", "image/png", PNG);
    let (entry, et) = draft(&mut b, book_id);
    ok(
        &mut b,
        tools::EVIDENCE_ATTACH,
        json!({"client_request_id":"link-entry","op":"link","evidence":ev,"target":{"kind":"entry","ticket":et}}),
    );
    let p =
        propose(&mut b, "classify-c1", &ev, "receipt", json!(90))["structuredContent"]["proposal"]
            .clone();
    let c = b.store().unwrap().conn();
    let book = books::get(c, book_id).unwrap().unwrap();
    entries::discard(c, &book, entry).unwrap();
    months::close(c, &book, "2026-01", None).unwrap();
    let v = call(
        &mut b,
        tools::EVIDENCE_KIND,
        json!({"client_request_id":"closed-accept","proposal":p,"decision":"accept"}),
    );
    assert_eq!(v["isError"], true, "{v}");
    assert_eq!(kind_of(&mut b, &ev), "other");
    let status: String = b
        .store()
        .unwrap()
        .conn()
        .query_row("SELECT status FROM evidence_classification", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(status, "draft");
}

#[test]
fn preview_is_a_static_text_html_resource() {
    let (_tmp, mut b, book_id) = setup();
    let ev = add(&mut b, "document-a", "image/png", PNG);
    let v = call(&mut b, tools::EVIDENCE_PREVIEW, json!({"evidence":ev}));
    assert_eq!(v["isError"], false, "{v}");
    let res = &v["content"][0];
    assert_eq!(res["type"], "resource");
    assert_eq!(res["resource"]["mimeType"], "text/html");
    let uri = res["resource"]["uri"].as_str().unwrap();
    assert!(
        uri.starts_with("books-preview://evidence/") && !uri.contains(&ev) && !uri.contains("tkt_")
    );
    let html = res["resource"]["text"].as_str().unwrap();
    let data = format!("data:image/png;base64,{}", b64::encode(PNG));
    assert!(html.contains(&format!("<img src=\"{data}\"")));
    let lower = html.to_lowercase();
    for bad in [
        "<script",
        "http:",
        "https:",
        "javascript:",
        "src=\"/",
        "file:",
        "onerror",
        "onload",
    ] {
        assert!(!lower.contains(bad), "{bad}");
    }
    assert_eq!(v["structuredContent"], json!({"title":"Evidence"}));
    // PDF uses a data object with a stated fallback, never a raster claim
    let pdf = add(&mut b, "document-b", "application/pdf", b"%PDF-synthetic");
    let html = call(&mut b, tools::EVIDENCE_PREVIEW, json!({"evidence":pdf}))["content"][0]["resource"]["text"].as_str().unwrap().to_string();
    assert!(
        html.contains("<object data=\"data:application/pdf;base64,")
            && html.contains("cannot show this PDF")
    );
    // the requested entry must be linked to the evidence
    let (_, et) = draft(&mut b, book_id);
    let v = call(
        &mut b,
        tools::EVIDENCE_PREVIEW,
        json!({"evidence":ev,"entry":et}),
    );
    assert_eq!(v["structuredContent"]["code"], "not_found");
    ok(
        &mut b,
        tools::EVIDENCE_ATTACH,
        json!({"client_request_id":"link-entry","op":"link","evidence":ev,"target":{"kind":"entry","ticket":et}}),
    );
    let v = call(
        &mut b,
        tools::EVIDENCE_PREVIEW,
        json!({"evidence":ev,"entry":et}),
    );
    assert!(
        v["content"][0]["resource"]["text"]
            .as_str()
            .unwrap()
            .contains("<table>")
    );
    // app only, and cross-book refuses
    let v = call_as(
        &mut b,
        Origin::Agent,
        tools::EVIDENCE_PREVIEW,
        json!({"evidence":ev}),
    );
    assert_eq!(v["isError"], true);
    let c = b.store().unwrap().conn();
    let foreign = books::create(c, "Synthetic two", Basis::Cash, 1).unwrap();
    let fb = plumbing::ticket_for(c, "book", foreign).unwrap();
    let v = call(
        &mut b,
        tools::EVIDENCE_PREVIEW,
        json!({"book":fb,"evidence":ev}),
    );
    assert_eq!(v["structuredContent"]["code"], "not_found");
    // no ordinary response carries the bytes
    let encoded = b64::encode(PNG);
    for (tool, args) in [
        (tools::EVIDENCE_LIST, json!({})),
        (tools::EVIDENCE_REVIEW, json!({})),
    ] {
        assert!(
            !call(&mut b, tool, args).to_string().contains(&encoded),
            "{tool}"
        );
    }
    let p = propose(&mut b, "classify-c1", &ev, "receipt", json!(5));
    assert!(!p.to_string().contains(&encoded));
}

#[test]
fn preview_escapes_and_enforces_the_post_encoding_limit() {
    let rows = [dr::TableRow {
        number: "<1>".into(),
        name: "A&B \"q\" 'x' <script>".into(),
        debit: 12345,
        credit: 0,
    }];
    let html = dr::render("image/png", PNG, Some(&rows), usize::MAX).unwrap();
    assert!(html.contains("&lt;1&gt; A&amp;B &quot;q&quot; &#39;x&#39; &lt;script&gt;"));
    assert!(!html.contains("<script"));
    assert!(html.contains("123.45"));
    // the limit counts the encoded page: exactly at it passes, one byte under refuses
    let exact = html.len();
    assert!(dr::render("image/png", PNG, Some(&rows), exact).is_ok());
    let r = dr::render("image/png", PNG, Some(&rows), exact - 1).unwrap_err();
    assert_eq!(r.code, "too_large");
    // raw bytes under 8 MiB whose base64 page exceeds it refuse, and the evidence stays
    let (_tmp, mut b, _) = setup();
    let mut big = PNG.to_vec();
    big.resize(dr::PREVIEW_MAX_BYTES * 3 / 4, b'x');
    assert!(big.len() < dr::PREVIEW_MAX_BYTES);
    let ev = add(&mut b, "document-small", "image/png", PNG);
    b.store()
        .unwrap()
        .conn()
        .execute(
            "UPDATE evidence SET bytes=?1, byte_length=?2",
            books_store::rusqlite::params![big, big.len() as i64],
        )
        .unwrap();
    let v = call(&mut b, tools::EVIDENCE_PREVIEW, json!({"evidence":ev}));
    assert_eq!(
        v["structuredContent"]["code"], "too_large",
        "{}",
        v["structuredContent"]
    );
    let n: i64 = b
        .store()
        .unwrap()
        .conn()
        .query_row("SELECT length(bytes) FROM evidence", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n as usize, big.len());
}

#[test]
fn preview_on_a_fresh_install_refuses_without_creating_the_store() {
    let tmp = TempDir::new("document-review-fresh");
    let paths = Paths::in_dir(tmp.0.join("db"));
    let db = paths.db();
    let mut b = Books::new(paths);
    let v = call(
        &mut b,
        tools::EVIDENCE_PREVIEW,
        json!({"evidence":"tkt_000000000000000000000000"}),
    );
    assert_eq!(v["isError"], true, "{v}");
    assert_eq!(v["structuredContent"]["code"], "not_set_up", "{v}");
    assert!(!db.exists(), "a read must not create the encrypted store");
}
