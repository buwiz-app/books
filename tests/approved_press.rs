//! The AI's press on Post and Close month, through the server as the Bot
//! sends it: only with the record of a person's yes to that very call. Void,
//! reopen, return confirm and hand-off stay the person's own click, and the
//! audit row names the AI and the person. Synthetic data only.

use books_mini_app::jsonrpc::Server;
use books_mini_app::manifest::{self, Source};
use books_mini_app::service::Books;
use books_mini_app::{b64, jcs};
use books_store::{Paths, TempDir};
use ed25519_dalek::SigningKey;
use serde_json::{Value, json};
use std::collections::HashMap;

struct Bot {
    server: Server,
    next: i64,
    sessions: HashMap<String, String>,
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

impl Bot {
    fn new(dir: &TempDir) -> Self {
        let books = Books::new(Paths::in_dir(dir.0.join("store")));
        let mut bot = Self {
            server: Server::new(Source::default(), books),
            next: 1,
            sessions: HashMap::new(),
        };
        let seed = [11u8; 32];
        let public = b64::encode(SigningKey::from_bytes(&seed).verifying_key().as_bytes());
        let p = bot.raw("buwiz.provision", json!({ "protocol": "1.0.0", "plugin": manifest::PLUGIN_ID, "key": { "alg": "ed25519", "seed": b64::encode(&seed) }, "pub": public }), Value::Null);
        assert_eq!(p["isError"], false, "{p}");
        bot.open_session("books");
        bot
    }

    fn open_session(&mut self, app_id: &str) -> &str {
        if self.sessions.contains_key(app_id) {
            return &self.sessions[app_id];
        }
        let app = manifest::unsigned(manifest::RELEASE_KEY_ID)["apps"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["id"] == app_id)
            .unwrap()
            .clone();
        let perms = manifest::permissions_digest(&app).unwrap();
        let mut open_bytes = [7u8; 16];
        open_bytes[..app_id.len().min(16)]
            .copy_from_slice(&app_id.as_bytes()[..app_id.len().min(16)]);
        let open_id = b64::encode(&open_bytes);
        let open = json!({ "app": app_id, "open_id": open_id, "ks_pub": b64::encode(&[5u8; 32]), "perms_sha256": perms, "not_after": now_ms() + 3_600_000 });
        let o = self.raw("buwiz.session.open", open, Value::Null);
        assert_eq!(o["isError"], false, "{o}");
        self.sessions.insert(app_id.to_string(), open_id);
        &self.sessions[app_id]
    }

    fn raw(&mut self, name: &str, args: Value, caller: Value) -> Value {
        let id = self.next;
        self.next += 1;
        let mut params = json!({ "name": name, "arguments": args });
        if !caller.is_null() {
            params["_meta"] = json!({ "buwiz/caller": caller });
        }
        let reply = self
            .server
            .handle(
                &json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call", "params": params }),
            )
            .unwrap();
        reply["result"].clone()
    }

    fn app(&mut self, name: &str, args: Value) -> Value {
        self.app_on("books", name, args)
    }

    fn app_on(&mut self, app: &str, name: &str, args: Value) -> Value {
        let open_id = self.open_session(app).to_string();
        let caller = json!({ "origin": "app", "plugin": manifest::PLUGIN_ID, "app": app, "open_id": open_id, "ts": now_ms() });
        self.raw(name, args, caller)
    }

    /// A call the Bot let go on the person's Allow, as the broker sends it.
    fn approved(&mut self, name: &str, args: Value) -> Value {
        let press = record(&args);
        self.pressed(name, args, press)
    }

    fn pressed(&mut self, name: &str, args: Value, press: Value) -> Value {
        let open_id = self.open_session("books").to_string();
        let caller = json!({ "origin": "agent", "plugin": manifest::PLUGIN_ID, "app": "books", "open_id": open_id, "ts": now_ms(), "press": press });
        self.raw(name, args, caller)
    }

    fn agent(&mut self, name: &str, args: Value) -> Value {
        let caller = json!({ "origin": "agent", "plugin": manifest::PLUGIN_ID, "ts": now_ms() });
        self.raw(name, args, caller)
    }

    /// The Bot reading a card: a read, as the app's, with the AI's record.
    fn card(&mut self, tool: &str, args: Value) -> Value {
        let open_id = self.open_session("books").to_string();
        let caller = json!({ "origin": "app", "plugin": manifest::PLUGIN_ID, "app": "books", "open_id": open_id, "ts": now_ms(), "press": { "by": "ai", "who": "agent.broker" } });
        self.raw(
            "books_app_press_card",
            json!({ "tool": tool, "args": args }),
            caller,
        )
    }
}

fn data(result: &Value) -> &Value {
    assert_eq!(result["isError"], false, "{result}");
    &result["structuredContent"]
}

fn code(result: &Value) -> &str {
    assert_eq!(result["isError"], true, "{result}");
    result["structuredContent"]["code"].as_str().unwrap()
}

fn record(args: &Value) -> Value {
    json!({
        "by": "ai_approved", "who": "agent.broker", "approved_by": "person.owner",
        "once": format!("press-{}", args["client_request_id"].as_str().unwrap()),
        "args_hash": format!("sha256:{}", jcs::sha256_jcs(args).unwrap()),
        "shown_hash": format!("sha256:{}", "b2".repeat(32)), "at": now_ms(),
    })
}

/// A book with one draft of ₱1,350.00 on 2026-10-18: `(bot, entry ticket)`.
fn with_draft(dir: &TempDir) -> (Bot, String) {
    let mut bot = Bot::new(dir);
    data(&bot.app(
        "books_app_book_create",
        json!({ "client_request_id": "create-book-0001", "label": "Buwiz Trading" }),
    ));
    let entry = draft(&mut bot, "draft-rent-0001", "2026-10-18", "1350.00");
    (bot, entry)
}

fn status(bot: &mut Bot, entry: &str) -> String {
    data(&bot.app("books_entry_get", json!({ "entry": entry })))["status"]
        .as_str()
        .unwrap()
        .to_string()
}

fn post(entry: &str) -> Value {
    json!({ "client_request_id": "press-post-0001", "entry": entry })
}

#[test]
fn a_yes_to_this_call_posts_it_and_the_audit_row_names_the_ai_and_the_person() {
    let dir = TempDir::new("press-post");
    let (mut bot, entry) = with_draft(&dir);
    assert_eq!(
        data(&bot.approved("books_app_post_entry", post(&entry)))["status"],
        "posted"
    );
    assert_eq!(status(&mut bot, &entry), "posted");
    let shown = data(&bot.app("books_entry_get", json!({ "entry": entry })))["pressed"].clone();
    assert!(
        shown
            .as_str()
            .unwrap()
            .starts_with("Posted by the AI, allowed by you, "),
        "{shown}"
    );
}

#[test]
fn a_yes_never_shows_to_the_assistant() {
    let dir = TempDir::new("press-model");
    let (mut bot, entry) = with_draft(&dir);
    data(&bot.approved("books_app_post_entry", post(&entry)));
    let seen = bot.agent("books_entry_get", json!({ "entry": entry }));
    assert_eq!(data(&seen).get("pressed"), None);
}

fn audit_rows(dir: &TempDir, tool: &str) -> i64 {
    let store = books_store::Store::open(&Paths::in_dir(dir.0.join("store"))).unwrap();
    books_store::plumbing::audit_count(store.conn(), tool).unwrap()
}

#[test]
fn a_used_once_is_refused_however_it_comes_back() {
    let dir = TempDir::new("press-once");
    let (mut bot, entry) = with_draft(&dir);
    data(&bot.approved("books_app_post_entry", post(&entry)));
    let again = bot.approved("books_app_post_entry", post(&entry));
    assert_eq!(
        code(&again),
        "press_reused",
        "even the same call, which a request id would replay"
    );
    // the same id on a call of its own, with a good hash
    let close = json!({ "client_request_id": "press-close-001", "month": "2026-10" });
    let mut press = record(&close);
    press["once"] = record(&post(&entry))["once"].clone();
    assert_eq!(
        code(&bot.pressed("books_app_month_close", close, press)),
        "press_reused"
    );
    assert_eq!(
        audit_rows(&dir, "books_app_month_close"),
        1,
        "refused, and on the record"
    );
}

#[test]
fn a_refused_write_still_spends_its_once() {
    let dir = TempDir::new("press-once-refused");
    let (mut bot, _) = with_draft(&dir);
    let close = json!({ "client_request_id": "press-close-001", "month": "2026-10" });
    assert_eq!(
        code(&bot.approved("books_app_month_close", close.clone())),
        "drafts_open"
    );
    assert_eq!(
        code(&bot.approved("books_app_month_close", close)),
        "press_reused"
    );
}

#[test]
fn a_stale_or_future_yes_is_refused() {
    let dir = TempDir::new("press-stale");
    let (mut bot, entry) = with_draft(&dir);
    for at in [now_ms() - 61_000, now_ms() + 6_000, 0] {
        let mut press = record(&post(&entry));
        press["at"] = json!(at);
        assert_eq!(
            code(&bot.pressed("books_app_post_entry", post(&entry), press)),
            "permission_denied",
            "{at}"
        );
    }
    assert_eq!(status(&mut bot, &entry), "draft");
}

#[test]
fn a_yes_with_an_empty_card_hash_is_refused() {
    let dir = TempDir::new("press-shown");
    let (mut bot, entry) = with_draft(&dir);
    for shown in [json!(""), json!("sha256:"), json!("abc")] {
        let mut press = record(&post(&entry));
        press["shown_hash"] = shown.clone();
        assert_eq!(
            code(&bot.pressed("books_app_post_entry", post(&entry), press)),
            "permission_denied",
            "{shown}"
        );
    }
    assert_eq!(status(&mut bot, &entry), "draft");
}

#[test]
fn other_arguments_are_refused() {
    let dir = TempDir::new("press-replay");
    let (mut bot, entry) = with_draft(&dir);
    data(&bot.approved("books_app_post_entry", post(&entry)));
    // the record of the first call, on another call
    let second = bot.app(
        "books_entry_draft",
        json!({ "client_request_id": "draft-rent-0002", "form": "money_out", "date": "2026-10-19",
                "amount": "9.00", "account": "11000", "category": "67200" }),
    );
    let other = data(&second)["entry"].as_str().unwrap().to_string();
    let press = record(&post(&entry));
    assert_eq!(
        code(&bot.pressed("books_app_post_entry", post(&other), press)),
        "permission_denied"
    );
    assert_eq!(status(&mut bot, &other), "draft");
}

#[test]
fn a_forged_or_partial_record_is_refused_and_nothing_posts() {
    let dir = TempDir::new("press-forged");
    let (mut bot, entry) = with_draft(&dir);
    let whole = record(&post(&entry));
    let mut forged = vec![
        json!({ "by": "ai", "who": "agent.broker" }),
        json!({ "by": "person", "who": "person.owner" }),
        Value::Null,
    ];
    for (key, value) in [
        ("approved_by", json!("agent.broker")),
        ("who", json!("person.owner")),
        ("once", Value::Null),
        ("args_hash", json!("sha256:0")),
        ("shown_hash", Value::Null),
        ("at", Value::Null),
    ] {
        let mut bad = whole.clone();
        bad[key] = value;
        forged.push(bad);
    }
    for press in forged {
        assert_eq!(
            code(&bot.pressed("books_app_post_entry", post(&entry), press.clone())),
            "permission_denied",
            "{press}"
        );
    }
    // the model's direct call carries a record of its own making
    let caller =
        json!({ "origin": "agent", "plugin": manifest::PLUGIN_ID, "ts": now_ms(), "press": whole });
    assert_eq!(
        code(&bot.raw("books_app_post_entry", post(&entry), caller)),
        "permission_denied"
    );
    // a record on origin app: still not the person's own click
    let open_id = bot.open_session("books").to_string();
    let caller = json!({ "origin": "app", "plugin": manifest::PLUGIN_ID, "app": "books", "open_id": open_id, "ts": now_ms(), "press": record(&post(&entry)) });
    assert_eq!(
        code(&bot.raw("books_app_post_entry", post(&entry), caller)),
        "permission_denied"
    );
    assert_eq!(status(&mut bot, &entry), "draft");
}

#[test]
fn close_month_takes_a_yes() {
    let dir = TempDir::new("press-close");
    let (mut bot, entry) = with_draft(&dir);
    data(&bot.app("books_app_post_entry", post(&entry)));
    let close = json!({ "client_request_id": "press-close-001", "month": "2026-10" });
    assert_eq!(
        data(&bot.approved("books_app_month_close", close))["status"],
        "closed"
    );
}

#[test]
fn a_batch_takes_a_yes_only_to_post() {
    let dir = TempDir::new("press-batch");
    let (mut bot, entry) = with_draft(&dir);
    let discard =
        json!({ "client_request_id": "press-batch-001", "action": "discard", "entries": [entry] });
    assert_eq!(
        code(&bot.approved("books_app_batch", discard)),
        "permission_denied"
    );
    assert_eq!(status(&mut bot, &entry), "draft");
    let post_all =
        json!({ "client_request_id": "press-batch-002", "action": "post", "entries": [entry] });
    assert_eq!(data(&bot.approved("books_app_batch", post_all))["done"], 1);
}

/// One more draft in `bot`'s book, under its own request id: its entry ticket.
fn draft(bot: &mut Bot, id: &str, date: &str, amount: &str) -> String {
    let d = bot.app(
        "books_entry_draft",
        json!({ "client_request_id": id, "form": "money_out", "date": date,
                "amount": amount, "account": "11000", "category": "67200", "payee": "Pixel Print Shop" }),
    );
    data(&d)["entry"].as_str().unwrap().to_string()
}

/// The audit rows that carry one press's one-time id.
fn rows_on_press(dir: &TempDir, once: &str) -> i64 {
    let store = books_store::Store::open(&Paths::in_dir(dir.0.join("store"))).unwrap();
    store
        .conn()
        .query_row(
            "SELECT count(*) FROM audit_event WHERE press_once = ?1",
            [once],
            |r| r.get(0),
        )
        .unwrap()
}

fn record_line(bot: &mut Bot, entry: &str) -> Value {
    data(&bot.app("books_entry_get", json!({ "entry": entry })))["pressed"].clone()
}

#[test]
fn every_entry_of_an_approved_batch_says_the_ai_posted_it() {
    let dir = TempDir::new("press-batch-line");
    let (mut bot, first) = with_draft(&dir);
    let second = draft(&mut bot, "draft-rent-0002", "2026-10-19", "250.00");
    let both = json!({ "client_request_id": "press-batch-003", "action": "post", "entries": [first, second] });
    let once = record(&both)["once"].as_str().unwrap().to_string();
    assert_eq!(data(&bot.approved("books_app_batch", both))["done"], 2);
    assert_eq!(
        rows_on_press(&dir, &once),
        3,
        "one row per entry the batch posted, and one for the batch, all on one press"
    );
    for entry in [&first, &second] {
        let line = record_line(&mut bot, entry);
        assert!(
            line.as_str()
                .is_some_and(|l| l.starts_with("Posted by the AI, allowed by you, ")),
            "{line}"
        );
    }
}

#[test]
fn a_persons_own_batch_and_a_refused_yes_leave_no_ai_line() {
    let dir = TempDir::new("press-batch-none");
    let (mut bot, own) = with_draft(&dir);
    let by_hand =
        json!({ "client_request_id": "press-batch-004", "action": "post", "entries": [own] });
    assert_eq!(data(&bot.app("books_app_batch", by_hand))["done"], 1);
    assert_eq!(record_line(&mut bot, &own), Value::Null);
    // the draft goes first, then the posted entry refuses the whole batch
    let kept = draft(&mut bot, "draft-rent-0003", "2026-10-19", "250.00");
    let refused =
        json!({ "client_request_id": "press-batch-005", "action": "post", "entries": [kept, own] });
    assert_eq!(
        code(&bot.approved("books_app_batch", refused)),
        "wrong_status"
    );
    assert_eq!(status(&mut bot, &kept), "draft");
    let later = json!({ "client_request_id": "press-post-0002", "entry": kept });
    data(&bot.app("books_app_post_entry", later));
    assert_eq!(record_line(&mut bot, &kept), Value::Null);
}

#[test]
fn what_d1_doesnt_name_stays_the_persons_click_with_a_yes_or_not() {
    let dir = TempDir::new("press-strict");
    let (mut bot, entry) = with_draft(&dir);
    data(&bot.app("books_app_post_entry", post(&entry)));
    let closed = json!({ "client_request_id": "strict-close-001", "month": "2026-09" });
    data(&bot.app("books_app_month_close", closed));
    for (tool, args) in [
        (
            "books_app_void_entry",
            json!({ "client_request_id": "strict-void-0001", "entry": entry }),
        ),
        (
            "books_app_month_reopen",
            json!({ "client_request_id": "strict-open-0001", "month": "2026-09", "reason": "x" }),
        ),
        (
            "books_app_return_ok",
            json!({ "client_request_id": "strict-rtrn-0001" }),
        ),
        (
            "books_app_handoff_create",
            json!({ "client_request_id": "strict-hand-0001" }),
        ),
        (
            "books_app_handoff_confirm",
            json!({ "client_request_id": "strict-conf-0001" }),
        ),
        (
            "books_app_discard_entry",
            json!({ "client_request_id": "strict-disc-0001", "entry": entry }),
        ),
    ] {
        assert_eq!(
            code(&bot.approved(tool, args)),
            "permission_denied",
            "{tool}"
        );
    }
    assert_eq!(status(&mut bot, &entry), "posted");
}

#[test]
fn the_tools_say_who_takes_a_yes_and_the_model_still_sees_none_of_them() {
    let dir = TempDir::new("press-list");
    let mut bot = Bot::new(&dir);
    let reply = bot
        .server
        .handle(&json!({ "jsonrpc": "2.0", "id": 900, "method": "tools/list", "params": {} }))
        .unwrap();
    let tools = reply["result"]["tools"].as_array().unwrap();
    let of = |name: &str| tools.iter().find(|t| t["name"] == name).unwrap().clone();
    for name in [
        "books_app_post_entry",
        "books_app_month_close",
        "books_app_batch",
    ] {
        let t = of(name);
        assert_eq!(t["_meta"]["buwiz/audience"], json!(["app"]), "{name}");
        let effect = if name == "books_app_month_close" {
            "close"
        } else {
            "post"
        };
        assert_eq!(
            t["_meta"]["buwiz/press"],
            json!({ "approved": true, "effect": effect, "describe": "books_app_press_card" }),
            "{name}"
        );
    }
    assert_eq!(
        of("books_entry_draft")["_meta"]["buwiz/press"],
        json!({ "describe": "books_app_press_card" })
    );
    for name in [
        "books_app_void_entry",
        "books_app_month_reopen",
        "books_app_return_ok",
        "books_app_handoff_create",
        "books_app_handoff_confirm",
    ] {
        assert_eq!(of(name)["_meta"].get("buwiz/press"), None, "{name}");
    }
    let card = of("books_app_press_card");
    assert_eq!(card["annotations"]["readOnlyHint"], true);
    assert_eq!(card["_meta"]["buwiz/audience"], json!(["app"]));
}

#[test]
fn a_card_shows_the_real_amount_date_and_accounts_and_changes_nothing() {
    let dir = TempDir::new("press-card");
    let (mut bot, entry) = with_draft(&dir);
    let card = bot.card("books_app_post_entry", post(&entry));
    let card = data(&card);
    let title = card["title"].as_str().unwrap();
    assert!(
        title.starts_with("Post entry 0001") && title.contains("1,350.00"),
        "{title}"
    );
    let lines = card["lines"].to_string();
    for shown in [
        "2026-10-18",
        "Pixel Print Shop",
        "11000",
        "67200",
        "debit",
        "credit",
    ] {
        assert!(lines.contains(shown), "{shown} in {lines}");
    }
    assert!(!card.to_string().contains("tkt_"), "{card}");
    assert_eq!(status(&mut bot, &entry), "draft");
}

#[test]
fn a_card_for_a_new_entry_a_month_a_batch_and_a_draft() {
    let dir = TempDir::new("press-cards");
    let (mut bot, entry) = with_draft(&dir);
    let new = json!({ "client_request_id": "press-new-00001", "form": "money_out", "date": "2026-10-20",
                      "amount": "99.50", "account": "11000", "category": "67200" });
    let got = bot.card("books_app_post_entry", new.clone());
    assert!(
        data(&got)["title"].as_str().unwrap().contains("99.50"),
        "{got}"
    );
    let got = bot.card("books_entry_draft", new);
    assert!(
        data(&got)["title"]
            .as_str()
            .unwrap()
            .starts_with("Save a draft for"),
        "{got}"
    );
    let got = bot.card(
        "books_app_month_close",
        json!({ "client_request_id": "x", "month": "2026-10" }),
    );
    assert_eq!(data(&got)["title"], "Close October 2026?");
    assert!(
        data(&got)["lines"]
            .to_string()
            .contains("Drafts still in it: 1"),
        "{got}"
    );
    let got = bot.card(
        "books_app_batch",
        json!({ "action": "post", "entries": [entry] }),
    );
    assert!(
        data(&got)["title"]
            .as_str()
            .unwrap()
            .starts_with("Post 1 entry for"),
        "{got}"
    );
    let got = bot.card(
        "books_app_batch",
        json!({ "action": "discard", "entries": [entry] }),
    );
    assert_eq!(code(&got), "permission_denied");
    let got = bot.card("books_app_void_entry", json!({ "entry": entry }));
    assert_eq!(code(&got), "permission_denied", "no card, so no press");
}

/// A book with a linked person and tags on a draft: `(bot, entry, person ticket)`.
fn with_links(dir: &TempDir) -> (Bot, String, String) {
    let (mut bot, _) = with_draft(dir);
    let made = bot.app(
        "books_app_person_create",
        json!({ "client_request_id": "person-pixel-001", "kind": "vendors", "name": "Pixel Print Shop" }),
    );
    let person = data(&made)["person"].as_str().unwrap().to_string();
    let d = bot.app(
        "books_entry_draft",
        json!({ "client_request_id": "draft-links-0001", "form": "money_out", "date": "2026-10-20", "amount": "500.00",
                "account": "11000", "category": "67200", "person": person, "department": "Sales", "location": "Olongapo",
                "memo": "Printing", "reference": "OR-77" }),
    );
    let entry = data(&d)["entry"].as_str().unwrap().to_string();
    (bot, entry, person)
}

#[test]
fn a_saved_entrys_card_shows_its_person_tags_memo_and_reference() {
    let dir = TempDir::new("press-card-links");
    let (mut bot, entry, _) = with_links(&dir);
    let card = bot.card("books_app_post_entry", post(&entry));
    let lines = data(&card)["lines"].to_string();
    for shown in [
        "Linked person: Pixel Print Shop",
        "Department: Sales",
        "Location: Olongapo",
        "For: Printing",
        "Reference: OR-77",
    ] {
        assert!(lines.contains(shown), "{shown} in {lines}");
    }
}

#[test]
fn a_new_entrys_card_shows_every_value_the_post_would_write() {
    let dir = TempDir::new("press-card-new");
    let (mut bot, _, person) = with_links(&dir);
    let call = json!({ "client_request_id": "press-new-00002", "form": "money_out", "date": "2026-10-21", "amount": "12.00",
                       "account": "11000", "category": "67200", "person": person, "department": "Sales", "location": "Olongapo",
                       "memo": "Ink", "reference": "OR-78", "facts": { "document_kind": "sale_invoice" } });
    let card = bot.card("books_app_post_entry", call);
    let lines = data(&card)["lines"].to_string();
    for shown in [
        "Linked person: Pixel Print Shop",
        "Department: Sales",
        "Location: Olongapo",
        "For: Ink",
        "Reference: OR-78",
        "sale_invoice",
    ] {
        assert!(lines.contains(shown), "{shown} in {lines}");
    }
    assert!(!lines.contains("tkt_"), "{lines}");
    let call = json!({ "client_request_id": "press-new-00003", "form": "money_out", "date": "2026-10-21", "amount": "12.00",
                       "account": "11000", "category": "67200", "payee": "New Vendor", "person_kind": "vendors" });
    let card = bot.card("books_app_post_entry", call);
    assert!(
        data(&card)["lines"]
            .to_string()
            .contains("Linked person: New Vendor (vendors)"),
        "{card}"
    );
}

#[test]
fn keep_both_says_posts_duplicates_on_the_card() {
    let dir = TempDir::new("press-card-dupes");
    let (mut bot, entry) = with_draft(&dir);
    let mut call = post(&entry);
    call["keep_both"] = json!(true);
    let card = bot.card("books_app_post_entry", call);
    assert!(
        data(&card)["lines"]
            .to_string()
            .contains("Posts duplicates"),
        "{card}"
    );
    let card = bot.card(
        "books_app_batch",
        json!({ "action": "post", "entries": [entry], "keep_both": true }),
    );
    assert!(
        data(&card)["lines"]
            .to_string()
            .contains("Posts duplicates"),
        "{card}"
    );
    let card = bot.card(
        "books_app_batch",
        json!({ "action": "post", "entries": [entry] }),
    );
    assert!(
        !data(&card)["lines"]
            .to_string()
            .contains("Posts duplicates"),
        "{card}"
    );
}

#[test]
fn a_batch_card_names_each_entrys_person_and_tags() {
    let dir = TempDir::new("press-card-batch");
    let (mut bot, entry, _) = with_links(&dir);
    let card = bot.card(
        "books_app_batch",
        json!({ "action": "post", "entries": [entry] }),
    );
    let lines = data(&card)["lines"].to_string();
    assert!(
        lines.contains("Pixel Print Shop") && lines.contains("Sales") && lines.contains("Olongapo"),
        "{lines}"
    );
}

#[test]
fn an_argument_the_card_doesnt_show_refuses_the_card_so_no_press_can_carry_it() {
    let dir = TempDir::new("press-card-unknown");
    let (mut bot, entry) = with_draft(&dir);
    let mut call = post(&entry);
    call["new_thing"] = json!(1);
    assert_eq!(
        code(&bot.card("books_app_post_entry", call)),
        "permission_denied"
    );
    let mut call = post(&entry);
    call["memo"] = json!("not what the entry says");
    assert_eq!(
        code(&bot.card("books_app_post_entry", call)),
        "permission_denied"
    );
    let batch = json!({ "action": "post", "entries": [entry], "category": "67200" });
    assert_eq!(
        code(&bot.card("books_app_batch", batch)),
        "permission_denied"
    );
    let close = json!({ "month": "2026-10", "new_thing": 1 });
    assert_eq!(
        code(&bot.card("books_app_month_close", close)),
        "permission_denied"
    );
    let line = json!({ "client_request_id": "x", "form": "journal", "date": "2026-10-20", "lines": [
        { "account": "11000", "debit": "1.00", "new_thing": 1 }, { "account": "67200", "credit": "1.00" } ] });
    assert_eq!(
        code(&bot.card("books_app_post_entry", line)),
        "permission_denied"
    );
}

#[test]
fn a_held_draft_is_a_new_one_and_nothing_else() {
    let dir = TempDir::new("press-card-draft");
    let (mut bot, entry) = with_draft(&dir);
    let base = json!({ "client_request_id": "x", "form": "money_out", "date": "2026-10-21", "amount": "1.00", "account": "11000", "category": "67200" });
    assert!(
        data(&bot.card("books_entry_draft", base.clone()))["title"]
            .as_str()
            .unwrap()
            .starts_with("Save a draft")
    );
    for (arg, value) in [
        ("entry", json!(entry)),
        ("copy_of", json!(entry)),
        ("extracted", json!({ "fields": {} })),
        ("extraction", json!({})),
        ("new_thing", json!(1)),
    ] {
        let mut call = base.clone();
        call[arg] = value;
        assert_eq!(
            code(&bot.card("books_entry_draft", call)),
            "permission_denied",
            "{arg}"
        );
    }
}

#[test]
fn a_batch_card_names_every_entry_up_to_what_a_batch_takes() {
    let dir = TempDir::new("press-card-all");
    let (mut bot, first) = with_draft(&dir);
    let mut entries = vec![first];
    for i in 2..=9 {
        let d = bot.app(
            "books_entry_draft",
            json!({ "client_request_id": format!("draft-many-{i:04}"), "form": "money_out", "date": "2026-10-18",
                    "amount": format!("{i}.00"), "account": "11000", "category": "67200" }),
        );
        entries.push(data(&d)["entry"].as_str().unwrap().to_string());
    }
    let card = bot.card(
        "books_app_batch",
        json!({ "action": "post", "entries": entries }),
    );
    let lines = data(&card)["lines"].as_array().unwrap().clone();
    assert_eq!(lines.len(), 9, "{lines:?}");
    assert!(
        lines[8].as_str().unwrap().contains("9.00"),
        "the last one too"
    );
    let many: Vec<String> = (0..51).map(|_| entries[0].clone()).collect();
    assert_eq!(
        code(&bot.card(
            "books_app_batch",
            json!({ "action": "post", "entries": many })
        )),
        "invalid_args"
    );
    let bad = json!({ "action": "post", "entries": [entries[0], 7] });
    assert_eq!(
        code(&bot.card("books_app_batch", bad)),
        "invalid_args",
        "no entry is skipped"
    );
}

/// The assistant's press on a screen, of a tool that needs no yes.
fn pressed_ai(bot: &mut Bot, name: &str, args: Value) -> Value {
    bot.pressed(name, args, json!({ "by": "ai", "who": "agent.broker" }))
}

fn new_draft(crid: &str) -> Value {
    json!({ "client_request_id": crid, "form": "money_out", "date": "2026-10-21", "amount": "1.00", "account": "11000", "category": "67200" })
}

/// With the switch on there is no card, and the rule stands all the same.
#[test]
fn a_press_on_a_screen_cant_overwrite_or_copy_at_any_switch() {
    let dir = TempDir::new("press-draft-on");
    let (mut bot, entry) = with_draft(&dir);
    assert_eq!(
        data(&pressed_ai(
            &mut bot,
            "books_entry_draft",
            new_draft("press-draft-0001")
        ))["status"],
        "draft"
    );
    for (crid, arg, value) in [
        ("press-draft-0002", "entry", json!(entry)),
        ("press-draft-0003", "copy_of", json!(entry)),
        ("press-draft-0004", "extracted", json!({ "fields": {} })),
        ("press-draft-0005", "extraction", json!({})),
    ] {
        let mut call = new_draft(crid);
        call[arg] = value;
        assert_eq!(
            code(&pressed_ai(&mut bot, "books_entry_draft", call.clone())),
            "permission_denied",
            "{arg}"
        );
        assert_eq!(
            code(&bot.approved("books_entry_draft", call)),
            "permission_denied",
            "{arg}, with a yes"
        );
    }
    let mut own = new_draft("press-draft-0006");
    own["entry"] = json!(entry);
    assert_eq!(
        data(&bot.app("books_entry_draft", own))["status"],
        "draft",
        "the person's own edit still runs"
    );
}

#[test]
fn a_draft_allowed_on_a_card_saves_and_its_once_is_spent() {
    let dir = TempDir::new("press-draft-yes");
    let (mut bot, _) = with_draft(&dir);
    let call = new_draft("press-draft-0001");
    assert_eq!(
        data(&bot.approved("books_entry_draft", call.clone()))["status"],
        "draft"
    );
    assert_eq!(
        code(&bot.approved("books_entry_draft", call)),
        "press_reused"
    );
}

#[test]
fn a_draft_saved_on_a_yes_isnt_called_posted_by_the_ai() {
    let dir = TempDir::new("press-draft-word");
    let (mut bot, _) = with_draft(&dir);
    let saved = bot.approved("books_entry_draft", new_draft("press-draft-0001"));
    let entry = data(&saved)["entry"].as_str().unwrap().to_string();
    assert_eq!(
        data(&bot.app("books_entry_get", json!({ "entry": entry })))["pressed"],
        Value::Null
    );
}

#[test]
fn the_audit_row_keeps_the_hash_of_what_the_card_showed() {
    let dir = TempDir::new("press-shown-row");
    let (mut bot, entry) = with_draft(&dir);
    let press = record(&post(&entry));
    data(&bot.pressed("books_app_post_entry", post(&entry), press.clone()));
    drop(bot);
    let store = books_store::Store::open(&Paths::in_dir(dir.0.join("store"))).unwrap();
    let kept: String = store
        .conn()
        .query_row(
            "SELECT press_shown_hash FROM audit_event WHERE tool = 'books_app_post_entry'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(kept, press["shown_hash"].as_str().unwrap());
}

#[test]
fn a_batch_card_refuses_an_entry_twice() {
    let dir = TempDir::new("press-card-twice");
    let (mut bot, entry) = with_draft(&dir);
    let twice = json!({ "action": "post", "entries": [entry, entry] });
    assert_eq!(code(&bot.card("books_app_batch", twice)), "invalid_args");
}

#[test]
fn a_card_is_no_tool_of_the_assistants() {
    let dir = TempDir::new("press-card-agent");
    let (mut bot, entry) = with_draft(&dir);
    let ask = json!({ "tool": "books_app_post_entry", "args": post(&entry) });
    assert_eq!(
        code(&bot.agent("books_app_press_card", ask)),
        "permission_denied"
    );
}
