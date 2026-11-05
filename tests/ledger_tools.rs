//! The ledger tools through the server, as the Bot calls them: the app makes the
//! book, the assistant drafts, only the app posts and voids, a repeat
//! returns the stored result, every write leaves an audit row, the model
//! sees totals and tickets but no payee or memo, and the store file holds
//! no plain text. Synthetic data only.

use books_mini_app::jsonrpc::Server;
use books_mini_app::manifest::{self, Source};
use books_mini_app::service::Books;
use books_mini_app::{b64, jcs};
use books_store::{Paths, TempDir};
use ed25519_dalek::SigningKey;
use serde_json::{Value, json};
use std::collections::HashMap;

const MEMO: &str = "MEMO-SYNTH-office-supplies";
const PAYEE: &str = "Pixel Print Shop";

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

    /// The assistant's press on a screen, as the Bot sends it.
    fn press(&mut self, app: &str, name: &str, args: Value) -> Value {
        let open_id = self.open_session(app).to_string();
        let caller = json!({ "origin": "agent", "plugin": manifest::PLUGIN_ID, "app": app, "open_id": open_id, "ts": now_ms(), "press": { "by": "ai", "who": "agent.broker" } });
        self.raw(name, args, caller)
    }

    fn agent(&mut self, name: &str, args: Value) -> Value {
        let caller = json!({ "origin": "agent", "plugin": manifest::PLUGIN_ID, "ts": now_ms() });
        self.raw(name, args, caller)
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

#[test]
fn m1_book_draft_post_void_through_the_server() {
    let dir = TempDir::new("m1");
    let mut bot = Bot::new(&dir);

    // before a book: status says so and makes no store
    assert_eq!(
        data(&bot.app("books_status", json!({})))["status"],
        "not_set_up"
    );
    assert!(!dir.0.join("store/books.db").exists());
    assert_eq!(code(&bot.agent("books_today", json!({}))), "not_set_up");

    // the assistant cannot make the book; the app can, on cash basis
    let create = json!({ "client_request_id": "create-book-0001", "label": "Buwiz Trading" });
    assert_eq!(
        code(&bot.agent("books_app_book_create", create.clone())),
        "permission_denied"
    );
    let made = bot.app("books_app_book_create", create.clone());
    assert_eq!(data(&made)["basis"], "cash");
    let book_ticket = data(&made)["book"].as_str().unwrap().to_string();
    // a repeat returns the stored result and makes no second book
    assert_eq!(bot.app("books_app_book_create", create.clone()), made);
    let status = bot.app("books_status", json!({}));
    assert_eq!(data(&status)["books"], 1);
    assert_eq!(data(&status)["status"], "ready");
    // the same id with another request is refused
    assert_eq!(
        code(&bot.app(
            "books_app_book_create",
            json!({ "client_request_id": "create-book-0001", "label": "Other" })
        )),
        "request_id_reused"
    );
    // a TIN in the label is refused
    assert_eq!(
        code(&bot.app(
            "books_app_book_create",
            json!({ "client_request_id": "create-book-0002", "label": format!("Shop {}-000", ["123", "456", "789"].join("-")) })
        )),
        "tin_refused"
    );

    // the assistant drafts a money-out entry; it is in no total
    let draft = json!({
        "client_request_id": "draft-supplies-01", "form": "money_out", "date": "2026-10-18",
        "amount": "1350.00", "account": "11000", "category": "67200", "payee": PAYEE, "memo": MEMO,
        "reference": "Receipt 1018"
    });
    let d = bot.agent("books_entry_draft", draft.clone());
    assert_eq!(data(&d)["status"], "draft");
    let entry = data(&d)["entry"].as_str().unwrap().to_string();
    assert_eq!(
        bot.agent("books_entry_draft", draft.clone()),
        d,
        "a repeat returns the stored draft"
    );

    // the assistant cannot post or void; the refusal says why
    let post = json!({ "client_request_id": "post-supplies-01", "entry": entry });
    let refused = bot.agent("books_app_post_entry", post.clone());
    assert_eq!(code(&refused), "permission_denied");
    assert!(
        refused["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("click")
    );
    let routine = json!({ "origin": "routine", "plugin": manifest::PLUGIN_ID, "routine": "weekly-tidy-up", "ts": now_ms() });
    assert_eq!(
        code(&bot.raw("books_entry_draft", draft.clone(), routine)),
        "permission_denied",
        "an undeclared routine write"
    );

    // the model's ledger view: amounts and tickets, no payee or memo
    let seen = bot.agent("books_ledger_list", json!({ "month": "2026-10" }));
    let text = seen.to_string();
    assert!(!text.contains(PAYEE) && !text.contains(MEMO), "{text}");
    assert_eq!(data(&seen)["counts"]["draft"], 1);
    assert_eq!(data(&seen)["rows"][0]["amount_shown"], "\u{2212}1,350.00");
    // the app's view has them
    let app_view = bot.app_on("ledger", "books_ledger_list", json!({ "month": "2026-10" }));
    assert_eq!(
        data(&app_view)["rows"][0]["title"],
        format!("{MEMO} · {PAYEE}")
    );
    let detail = bot.app("books_entry_get", json!({ "entry": entry }));
    assert_eq!(
        data(&detail)["lines"][0]["account"],
        "67200 Office Supplies"
    );
    assert_eq!(data(&detail)["actions"], json!(["discard", "edit", "post"]));
    assert!(
        !bot.agent("books_entry_get", json!({ "entry": entry }))
            .to_string()
            .contains(MEMO)
    );

    // the user posts it in the app
    let posted = bot.app("books_app_post_entry", post.clone());
    assert_eq!(data(&posted)["status"], "posted");
    assert_eq!(
        bot.app("books_app_post_entry", post.clone()),
        posted,
        "a repeated post is the stored result"
    );
    let today = bot.app("books_today", json!({}));
    let cash = &data(&today)["cash"];
    assert_eq!(cash["total"], "-1350.00");

    // save and post a sale in one step, then void it: original Voided, reversal posted
    let sale = bot.app("books_app_post_entry", json!({
        "client_request_id": "post-sale-0142", "form": "money_in", "date": "2026-10-28", "amount": "80000.00",
        "account": "11000", "category": "41000", "payee": "Bayani Foods", "reference": "Invoice 0142",
        "facts": { "document_kind": "sale_invoice" }
    }));
    let sale_entry = data(&sale)["entry"].as_str().unwrap().to_string();
    let accounts = bot.agent("books_accounts_list", json!({ "group": "Assets" }));
    let bank = data(&accounts)["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["number"] == "11000")
        .unwrap()
        .clone();
    assert_eq!(bank["balance"], "78650.00");
    let voided = bot.app(
        "books_app_void_entry",
        json!({ "client_request_id": "void-sale-0142", "entry": sale_entry }),
    );
    assert_eq!(data(&voided)["status"], "voided");
    let reversal = data(&voided)["reversal"].as_str().unwrap().to_string();
    let rev = bot.app("books_entry_get", json!({ "entry": reversal }));
    assert_eq!(data(&rev)["status_label"], "Reversal");
    assert_eq!(
        data(&rev)["date"],
        "2026-10-28",
        "dated like the original (DD-15)"
    );
    let after = bot.agent("books_accounts_list", json!({ "group": "Assets" }));
    let bank = data(&after)["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["number"] == "11000")
        .unwrap()
        .clone();
    assert_eq!(
        bank["balance"], "-1350.00",
        "a void and its reversal net to zero"
    );
    assert_eq!(
        code(&bot.app(
            "books_app_void_entry",
            json!({ "client_request_id": "void-sale-0142b", "entry": sale_entry })
        )),
        "already_voided"
    );

    // a cash book refuses a receivable line at post
    let ar = bot.app("books_app_post_entry", json!({
        "client_request_id": "post-ar-000001", "form": "journal", "date": "2026-10-20",
        "lines": [ { "account": "12010", "debit": "100.00" }, { "account": "41000", "credit": "100.00" } ]
    }));
    assert_eq!(code(&ar), "accrual_account");
    // an unbalanced journal is refused and nothing of it stays
    let unbalanced = bot.app("books_app_post_entry", json!({
        "client_request_id": "post-unbal-0001", "form": "journal", "date": "2026-10-20",
        "lines": [ { "account": "11000", "debit": "100.00" }, { "account": "41000", "credit": "99.99" } ]
    }));
    assert_eq!(code(&unbalanced), "unbalanced");
    let all = bot.app_on("ledger", "books_ledger_list", json!({ "month": "2026-10" }));
    assert_eq!(
        data(&all)["counts"]["all"],
        3,
        "supplies, the voided sale and its reversal"
    );

    // the ticket titles are neutral (protocol 1.0 §23.2 P3)
    let t = bot.raw(
        "buwiz.ticket.resolve",
        json!({ "ticket": entry }),
        Value::Null,
    );
    assert_eq!(t["structuredContent"]["title"], "Entry 0001");
    let t = bot.raw(
        "buwiz.ticket.resolve",
        json!({ "ticket": book_ticket }),
        Value::Null,
    );
    assert_eq!(t["structuredContent"]["title"], "Books");

    drop(bot);
    // every write left an audit row; refusals too
    let store = books_store::Store::open(&Paths::in_dir(dir.0.join("store"))).unwrap();
    let c = store.conn();
    let n = |tool: &str| books_store::plumbing::audit_count(c, tool).unwrap();
    assert_eq!(
        n("books_app_book_create"),
        3,
        "one made; the reused id and the TIN refused"
    );
    assert_eq!(n("books_entry_draft"), 1);
    assert_eq!(n("books_app_post_entry"), 4);
    assert_eq!(n("books_app_void_entry"), 2);
    let leaked: i64 = c
        .query_row(
            "SELECT count(*) FROM audit_event WHERE outcome LIKE '%MEMO%' OR tool LIKE '%Pixel%'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(leaked, 0);
    drop(store);

    // the encrypted file holds no plain text
    let bytes = std::fs::read(dir.0.join("store/books.db")).unwrap();
    for needle in [
        MEMO,
        PAYEE,
        "Bayani Foods",
        "Invoice 0142",
        "SQLite format 3",
    ] {
        assert!(
            !bytes.windows(needle.len()).any(|w| w == needle.as_bytes()),
            "{needle} is in the store file as plain text"
        );
    }
    let _ = jcs::sha256_hex(&bytes);
}

#[test]
fn the_assistants_press_on_a_screen_saves_a_draft_and_nothing_else() {
    let dir = TempDir::new("press");
    let mut bot = Bot::new(&dir);
    data(&bot.app(
        "books_app_book_create",
        json!({ "client_request_id": "create-book-0001", "label": "Buwiz Trading" }),
    ));
    let draft = json!({
        "client_request_id": "press-draft-001", "form": "money_out", "date": "2026-10-18",
        "amount": "1350.00", "account": "11000", "category": "67200", "payee": PAYEE, "memo": MEMO,
    });
    let d = bot.press("books", "books_entry_draft", draft);
    assert_eq!(data(&d)["status"], "draft");
    let entry = data(&d)["entry"].as_str().unwrap().to_string();

    // a posted entry to try the rest on, by the person
    let posted = bot.app("books_app_post_entry", json!({ "client_request_id": "post-0002", "form": "money_out", "date": "2026-10-19", "amount": "100.00", "account": "11000", "category": "67200" }));
    let posted = data(&posted)["entry"].as_str().unwrap().to_string();

    let person_only = [
        (
            "books_app_post_entry",
            json!({ "client_request_id": "press-post-001", "entry": entry }),
        ),
        (
            "books_app_void_entry",
            json!({ "client_request_id": "press-void-001", "entry": posted, "reason": "x" }),
        ),
        (
            "books_app_discard_entry",
            json!({ "client_request_id": "press-gone-001", "entry": entry }),
        ),
        (
            "books_app_month_close",
            json!({ "client_request_id": "press-close-01", "month": "2026-10" }),
        ),
        (
            "books_app_month_reopen",
            json!({ "client_request_id": "press-open-001", "month": "2026-10", "reason": "x" }),
        ),
        (
            "books_app_return_ok",
            json!({ "client_request_id": "press-rtrn-001" }),
        ),
        (
            "books_app_handoff_create",
            json!({ "client_request_id": "press-hand-001" }),
        ),
        (
            "books_app_handoff_confirm",
            json!({ "client_request_id": "press-conf-001" }),
        ),
    ];
    for (tool, args) in person_only {
        let refused = bot.press("books", tool, args);
        assert_eq!(code(&refused), "permission_denied", "{tool}");
    }
    assert_eq!(
        data(&bot.app("books_entry_get", json!({ "entry": entry })))["status"],
        "draft",
        "still a draft"
    );
    assert_eq!(
        data(&bot.app("books_entry_get", json!({ "entry": posted })))["status"],
        "posted",
        "still posted"
    );

    drop(bot);
    let store = books_store::Store::open(&Paths::in_dir(dir.0.join("store"))).unwrap();
    let origin: String = store
        .conn()
        .query_row(
            "SELECT origin FROM audit_event WHERE tool = 'books_entry_draft' ORDER BY event_id DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(origin, "agent");
}

#[test]
fn a_press_on_a_screen_needs_the_bots_record_and_a_real_open() {
    let dir = TempDir::new("press-record");
    let mut bot = Bot::new(&dir);
    data(&bot.app(
        "books_app_book_create",
        json!({ "client_request_id": "create-book-0001", "label": "Buwiz Trading" }),
    ));
    let draft = |id: &str| json!({ "client_request_id": id, "form": "money_out", "date": "2026-10-18", "amount": "10.00", "account": "11000", "category": "67200" });
    let open_id = bot.open_session("books").to_string();
    let no_record = json!({ "origin": "agent", "plugin": manifest::PLUGIN_ID, "app": "books", "open_id": open_id, "ts": now_ms() });
    assert_eq!(
        code(&bot.raw("books_entry_draft", draft("no-record-0001"), no_record)),
        "permission_denied"
    );
    let other = json!({ "origin": "agent", "plugin": manifest::PLUGIN_ID, "app": "books", "open_id": open_id, "ts": now_ms(), "press": { "by": "person", "who": "person.owner" } });
    assert_eq!(
        code(&bot.raw("books_entry_draft", draft("wrong-record-01"), other)),
        "permission_denied"
    );
    let forged = json!({ "origin": "agent", "plugin": manifest::PLUGIN_ID, "app": "books", "open_id": "not-an-open", "ts": now_ms(), "press": { "by": "ai", "who": "agent.broker" } });
    assert_eq!(
        code(&bot.raw("books_entry_draft", draft("no-open-000001"), forged)),
        "permission_denied"
    );
    // the assistant's own call, with no screen, is as it was
    assert_eq!(
        data(&bot.agent("books_entry_draft", draft("direct-000001")))["status"],
        "draft"
    );
}

#[test]
fn only_the_draft_tools_are_open_to_the_assistants_press() {
    use books_mini_app::tools;
    let mut open = Vec::new();
    for t in tools::list() {
        let name = t["name"].as_str().unwrap();
        let audience = &t["_meta"]["buwiz/audience"];
        let tool = tools::find(name).unwrap();
        match (tool.reach, tool.write) {
            (tools::Reach::App, _) => assert_eq!(audience, &json!(["app"]), "{name}"),
            (tools::Reach::Everyone, false) => assert!(audience.is_null(), "{name}"),
            (tools::Reach::Everyone, true) => {
                assert_eq!(audience, &json!(["app", "agent"]), "{name}");
                open.push(name.to_string());
            }
        }
    }
    open.sort();
    assert_eq!(
        open,
        [
            "books_entry_draft",
            "books_evidence_classify",
            "books_owed_draft",
            "books_statement_draft_lines"
        ]
    );
}
