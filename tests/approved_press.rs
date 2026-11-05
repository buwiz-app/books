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

    /// The Bot reading a card: a read, as the app's, with the AI's record.
    fn card(&mut self, tool: &str, args: Value) -> Value {
        let open_id = self.open_session("books").to_string();
        let caller = json!({ "origin": "app", "plugin": manifest::PLUGIN_ID, "app": "books", "open_id": open_id, "ts": now_ms(), "press": { "by": "ai", "who": "agent.broker" } });
        self.raw("books_app_press_card", json!({ "tool": tool, "args": args }), caller)
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
        "by": "ai_approved", "who": "agent.broker", "approved_by": "person.owner", "once": "press-0001",
        "args_hash": format!("sha256:{}", jcs::sha256_jcs(args).unwrap()),
        "shown_hash": format!("sha256:{}", "b2".repeat(32)), "at": 1_793_745_245_000_i64,
    })
}

/// A book with one draft of ₱1,350.00 on 2026-10-18: `(bot, entry ticket)`.
fn with_draft(dir: &TempDir) -> (Bot, String) {
    let mut bot = Bot::new(dir);
    data(&bot.app(
        "books_app_book_create",
        json!({ "client_request_id": "create-book-0001", "label": "Buwiz Trading" }),
    ));
    let d = bot.app(
        "books_entry_draft",
        json!({ "client_request_id": "draft-rent-0001", "form": "money_out", "date": "2026-10-18",
                "amount": "1350.00", "account": "11000", "category": "67200", "payee": "Pixel Print Shop" }),
    );
    let entry = data(&d)["entry"].as_str().unwrap().to_string();
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
        shown.as_str().unwrap().starts_with("Posted by the AI, allowed by you, "),
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

#[test]
fn a_replayed_yes_does_nothing_new_and_other_arguments_are_refused() {
    let dir = TempDir::new("press-replay");
    let (mut bot, entry) = with_draft(&dir);
    let first = bot.approved("books_app_post_entry", post(&entry));
    assert_eq!(bot.approved("books_app_post_entry", post(&entry)), first, "the stored result");
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
    let caller = json!({ "origin": "agent", "plugin": manifest::PLUGIN_ID, "ts": now_ms(), "press": whole });
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
    let discard = json!({ "client_request_id": "press-batch-001", "action": "discard", "entries": [entry] });
    assert_eq!(
        code(&bot.approved("books_app_batch", discard)),
        "permission_denied"
    );
    assert_eq!(status(&mut bot, &entry), "draft");
    let post_all = json!({ "client_request_id": "press-batch-002", "action": "post", "entries": [entry] });
    assert_eq!(data(&bot.approved("books_app_batch", post_all))["done"], 1);
}

#[test]
fn what_d1_doesnt_name_stays_the_persons_click_with_a_yes_or_not() {
    let dir = TempDir::new("press-strict");
    let (mut bot, entry) = with_draft(&dir);
    data(&bot.app("books_app_post_entry", post(&entry)));
    let closed = json!({ "client_request_id": "strict-close-001", "month": "2026-09" });
    data(&bot.app("books_app_month_close", closed));
    for (tool, args) in [
        ("books_app_void_entry", json!({ "client_request_id": "strict-void-0001", "entry": entry })),
        ("books_app_month_reopen", json!({ "client_request_id": "strict-open-0001", "month": "2026-09", "reason": "x" })),
        ("books_app_return_ok", json!({ "client_request_id": "strict-rtrn-0001" })),
        ("books_app_handoff_create", json!({ "client_request_id": "strict-hand-0001" })),
        ("books_app_handoff_confirm", json!({ "client_request_id": "strict-conf-0001" })),
        ("books_app_discard_entry", json!({ "client_request_id": "strict-disc-0001", "entry": entry })),
    ] {
        assert_eq!(code(&bot.approved(tool, args)), "permission_denied", "{tool}");
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
    for name in ["books_app_post_entry", "books_app_month_close", "books_app_batch"] {
        let t = of(name);
        assert_eq!(t["_meta"]["buwiz/audience"], json!(["app"]), "{name}");
        assert_eq!(t["_meta"]["buwiz/press"], json!({ "approved": true, "describe": "books_app_press_card" }), "{name}");
    }
    assert_eq!(of("books_entry_draft")["_meta"]["buwiz/press"], json!({ "describe": "books_app_press_card" }));
    for name in ["books_app_void_entry", "books_app_month_reopen", "books_app_return_ok", "books_app_handoff_create", "books_app_handoff_confirm"] {
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
    assert!(title.starts_with("Post entry 0001") && title.contains("1,350.00"), "{title}");
    let lines = card["lines"].to_string();
    for shown in ["2026-10-18", "Pixel Print Shop", "11000", "67200", "debit", "credit"] {
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
    assert!(data(&got)["title"].as_str().unwrap().contains("99.50"), "{got}");
    let got = bot.card("books_entry_draft", new);
    assert!(data(&got)["title"].as_str().unwrap().starts_with("Save a draft for"), "{got}");
    let got = bot.card("books_app_month_close", json!({ "client_request_id": "x", "month": "2026-10" }));
    assert_eq!(data(&got)["title"], "Close October 2026?");
    assert!(data(&got)["lines"].to_string().contains("Drafts still in it: 1"), "{got}");
    let got = bot.card("books_app_batch", json!({ "action": "post", "entries": [entry] }));
    assert!(data(&got)["title"].as_str().unwrap().starts_with("Post 1 entry for"), "{got}");
    let got = bot.card("books_app_batch", json!({ "action": "discard", "entries": [entry] }));
    assert_eq!(code(&got), "permission_denied");
    let got = bot.card("books_app_void_entry", json!({ "entry": entry }));
    assert_eq!(code(&got), "permission_denied", "no card, so no press");
}

#[test]
fn a_card_is_no_tool_of_the_assistants() {
    let dir = TempDir::new("press-card-agent");
    let (mut bot, entry) = with_draft(&dir);
    let ask = json!({ "tool": "books_app_post_entry", "args": post(&entry) });
    assert_eq!(code(&bot.agent("books_app_press_card", ask)), "permission_denied");
}
