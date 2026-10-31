//! M9: Buwiz sign-in and the tax-profile link, against a fake cloud (no
//! network, no real account). Device sign-in as `buwiz-books`, scope
//! `books`; each new refresh token is in the store before its access token
//! is used; a refused refresh signs out; a profile list with a TIN is
//! refused; the book links by id only; no token or code reaches a result.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use books_mini_app::cloud::{self, Transport};
use books_mini_app::service::{Books, Origin};
use books_mini_app::tools;
use books_store::{Paths, TempDir};
use serde_json::{Map, Value, json};

const PROFILE: &str = "7c9d1e3f-5a7b-4c9d-8e1f-2a3b4c5d6e7f";

#[derive(Clone, Default)]
struct Fake {
    answers: Rc<RefCell<VecDeque<(u16, Value)>>>,
    log: Rc<RefCell<Vec<String>>>,
}

impl Fake {
    fn answer(&self, status: u16, v: Value) {
        self.answers.borrow_mut().push_back((status, v));
    }
}

impl Transport for Fake {
    fn post_form(&self, path: &str, form: &[(&str, &str)]) -> Result<(u16, Value), String> {
        let f: Vec<String> = form.iter().map(|(k, v)| format!("{k}={v}")).collect();
        self.log
            .borrow_mut()
            .push(format!("POST {path} {}", f.join("&")));
        Ok(self
            .answers
            .borrow_mut()
            .pop_front()
            .expect("an answer for each call"))
    }

    fn get(&self, path: &str, bearer: &str) -> Result<(u16, Value), String> {
        self.log.borrow_mut().push(format!("GET {path} {bearer}"));
        Ok(self
            .answers
            .borrow_mut()
            .pop_front()
            .expect("an answer for each call"))
    }
}

fn call(books: &mut Books, name: &str, args: Value) -> Value {
    let args: Map<String, Value> = args.as_object().unwrap().clone();
    tools::call(name, &args, Origin::App, books).unwrap()
}

fn ok(books: &mut Books, name: &str, args: Value) -> Value {
    let out = call(books, name, args);
    assert_eq!(out["isError"], false, "{name}: {out}");
    out["structuredContent"].clone()
}

fn refused(books: &mut Books, name: &str, args: Value) -> String {
    let out = call(books, name, args);
    assert_eq!(out["isError"], true, "{name}: {out}");
    out["structuredContent"]["code"]
        .as_str()
        .unwrap()
        .to_string()
}

fn profiles(extra: Value) -> Value {
    json!([{ "id": PROFILE, "tin_masked": "***-***-***-00000", "registered_name": "Isla Demo Trading", "permissions": ["profile.view"], "activated_at": 1_790_000_000, "extra": extra }])
}

fn book(dir: &TempDir, fake: &Fake) -> Books {
    let mut books = Books::new(Paths::in_dir(dir.0.join("store")));
    books.transport = Some(Box::new(fake.clone()));
    books
}

#[test]
fn m9_sign_in_rotate_refresh_and_link_by_id() {
    let dir = TempDir::new("m9-cloud");
    let fake = Fake::default();
    let mut b = Books::new(Paths::in_dir(dir.0.join("store")));
    ok(
        &mut b,
        "books_app_book_create",
        json!({ "client_request_id": "m9-book-0001", "label": "Isla Demo Trading" }),
    );
    // no cloud configured: plain, and Books still works
    assert_eq!(
        refused(&mut b, "books_app_signin", json!({ "step": "start" })),
        "cloud_off"
    );
    assert_eq!(
        ok(&mut b, "books_app_tax_list", json!({}))["signed_in"],
        false
    );
    drop(b);

    let mut b = book(&dir, &fake);
    fake.answer(200, json!({ "device_code": "DEV-secret-1", "user_code": "WDJB-MJHT", "verification_uri": "https://cloud.example/device", "interval": 5, "expires_in": 600 }));
    let s = ok(&mut b, "books_app_signin", json!({ "step": "start" }));
    assert_eq!(
        (s["user_code"].as_str(), s["status"].as_str()),
        (Some("WDJB-MJHT"), Some("waiting"))
    );
    assert!(
        !s.to_string().contains("DEV-secret-1"),
        "the device code stays in Books"
    );
    fake.answer(400, json!({ "error": "authorization_pending" }));
    assert_eq!(
        ok(&mut b, "books_app_signin", json!({ "step": "poll" }))["status"],
        "waiting"
    );
    fake.answer(200, json!({ "access_token": "ACC-1", "refresh_token": "REF-1", "token_type": "Bearer", "expires_in": 3600 }));
    let done = ok(&mut b, "books_app_signin", json!({ "step": "poll" }));
    assert_eq!(done["status"], "signed_in");
    assert!(!done.to_string().contains("ACC-1") && !done.to_string().contains("REF-1"));
    assert_eq!(
        books_store::link::refresh_token(b.store().unwrap().conn())
            .unwrap()
            .as_deref(),
        Some("REF-1")
    );
    assert!(
        fake.log.borrow()[0].contains("client_id=buwiz-books")
            && fake.log.borrow()[0].contains("scope=books")
    );

    // the list: four fields read, the rest ignored
    fake.answer(200, profiles(json!("ok")));
    let list = ok(&mut b, "books_app_tax_list", json!({}));
    let p = &list["profiles"][0];
    assert_eq!(
        (
            p["id"].as_str(),
            p["tin_masked"].as_str(),
            p["linked"].as_bool()
        ),
        (Some(PROFILE), Some("***-***-***-00000"), Some(false))
    );
    assert!(p.get("activated_at").is_none() && p.get("extra").is_none());
    assert_eq!(
        fake.log.borrow().last().unwrap(),
        "GET /api/tax-profiles ACC-1"
    );

    // a link names a profile the cloud lists now, by id
    fake.answer(200, profiles(json!("ok")));
    ok(
        &mut b,
        "books_app_tax_link",
        json!({ "client_request_id": "m9-link-00001", "profile": PROFILE }),
    );
    let linked = books_store::books::all(b.store().unwrap().conn()).unwrap()[0]
        .cloud_tax_profile_id
        .clone();
    assert_eq!(linked.as_deref(), Some(PROFILE));
    fake.answer(200, profiles(json!("ok")));
    assert_eq!(
        refused(
            &mut b,
            "books_app_tax_link",
            json!({ "client_request_id": "m9-link-00002", "profile": "00000000-0000-4000-8000-000000000000" })
        ),
        "not_found"
    );
    drop(b);

    // a new process: refresh rotates, and the new token is saved before use
    let mut b = book(&dir, &fake);
    fake.answer(
        200,
        json!({ "access_token": "ACC-2", "refresh_token": "REF-2" }),
    );
    fake.answer(200, profiles(json!("ok")));
    let list = ok(&mut b, "books_app_tax_list", json!({}));
    assert_eq!(list["profiles"][0]["linked"], true);
    let log = fake.log.borrow().clone();
    let n = log.len();
    assert!(
        log[n - 2].contains("grant_type=refresh_token")
            && log[n - 2].contains("refresh_token=REF-1"),
        "{log:?}"
    );
    assert_eq!(log[n - 1], "GET /api/tax-profiles ACC-2");
    assert_eq!(
        books_store::link::refresh_token(b.store().unwrap().conn())
            .unwrap()
            .as_deref(),
        Some("REF-2")
    );

    // a list with a TIN is refused whole: a tin field, or a TIN-shaped value
    fake.answer(
        200,
        json!([{ "id": PROFILE, "tin": "123456789000", "tin_masked": "***-***-***-00000" }]),
    );
    assert_eq!(
        refused(&mut b, "books_app_tax_list", json!({})),
        "tin_refused"
    );
    fake.answer(200, profiles(json!("Registered as 123-456-789-000")));
    assert_eq!(
        refused(&mut b, "books_app_tax_list", json!({})),
        "tin_refused"
    );

    // an expired access token refreshes once; a refused refresh signs out
    fake.answer(401, json!({}));
    fake.answer(400, json!({ "error": "invalid_grant" }));
    assert_eq!(
        refused(&mut b, "books_app_tax_list", json!({})),
        "signed_out"
    );
    assert_eq!(
        books_store::link::refresh_token(b.store().unwrap().conn()).unwrap(),
        None
    );
    // the link stays (by id); unlinking is the owner's click
    ok(
        &mut b,
        "books_app_tax_link",
        json!({ "client_request_id": "m9-unlink-0001", "unlink": true }),
    );
    assert_eq!(
        books_store::books::all(b.store().unwrap().conn()).unwrap()[0].cloud_tax_profile_id,
        None
    );

    // the app's only: the assistant never signs in or links
    for t in [
        "books_app_signin",
        "books_app_tax_list",
        "books_app_tax_link",
    ] {
        assert_eq!(tools::find(t).unwrap().reach, tools::Reach::App, "{t}");
    }
    // nothing token-like went into the audit or the replay table
    let c = b.store().unwrap().conn();
    let stored: String = c
        .query_row(
            "SELECT group_concat(result, ' ') FROM idempotency",
            [],
            |r| r.get(0),
        )
        .unwrap_or_default();
    assert!(
        !stored.contains("REF-") && !stored.contains("ACC-"),
        "{stored}"
    );
}

#[test]
fn profile_ids_are_opaque_uuids_and_masked_tins_pass() {
    assert!(books_store::link::is_profile_id(PROFILE));
    assert!(!books_store::link::is_profile_id("123456789000"));
    let list = cloud::read_profiles(&profiles(json!(null))).unwrap();
    assert_eq!(list.len(), 1);
}

#[test]
fn a_release_build_signs_in_to_api_buwiz_com_unless_told_otherwise() {
    assert_eq!(
        cloud::base_url(None, true).as_deref(),
        Some(cloud::RELEASE_URL)
    );
    assert_eq!(cloud::RELEASE_URL, "https://api.buwiz.com");
    // tests and debug builds never reach the network on their own
    assert_eq!(cloud::base_url(None, false), None);
    // BOOKS_CLOUD_URL wins either way; empty turns the cloud off
    for release in [true, false] {
        assert_eq!(
            cloud::base_url(Some("https://cloud.example/"), release).as_deref(),
            Some("https://cloud.example")
        );
        assert_eq!(cloud::base_url(Some(""), release), None);
        assert_eq!(cloud::base_url(Some("http://cloud.example"), release), None);
    }
}
