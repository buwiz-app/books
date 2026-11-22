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

#[path = "support/owed_tool.rs"]
mod owed_tool;

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
        let (name, args) = owed_tool::normalize(name, args);
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

fn app_data(bot: &mut Bot, name: &str, args: Value) -> Value {
    let result = bot.app(name, args);
    data(&result).clone()
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

/// Every table's rows, for comparing a store before and after.
fn dump(dir: &TempDir) -> String {
    let store = books_store::Store::open(&Paths::in_dir(dir.0.join("store"))).unwrap();
    let c = store.conn();
    let mut names = c
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .unwrap();
    let names: Vec<String> = names
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let mut out = String::new();
    for name in names {
        let mut q = c.prepare(&format!("SELECT * FROM \"{name}\"")).unwrap();
        let columns = q.column_count();
        let rows: Vec<String> = q
            .query_map([], |r| {
                let mut row = String::new();
                for i in 0..columns {
                    row.push_str(&format!("{:?}|", r.get_ref(i).unwrap()));
                }
                Ok(row)
            })
            .unwrap()
            .map(Result::unwrap)
            .collect();
        out.push_str(&format!("{name}: {}\n", rows.join("\n")));
    }
    out
}

/// The path the draft-tools check doesn't cover: the AI's press reaching an app-only read, which
/// goes out as `origin: app` with the press record.
#[test]
fn a_press_reaches_the_app_only_reads_and_they_change_nothing() {
    use books_mini_app::tools;
    let dir = TempDir::new("press-reads");
    let mut bot = Bot::new(&dir);
    data(&bot.app(
        "books_app_book_create",
        json!({ "client_request_id": "create-book-0001", "label": "Buwiz Trading" }),
    ));
    let owed = bot.app(
        "books_app_owed_create",
        json!({ "client_request_id": "owed-create-01", "side": "collect", "person_name": "Sample Customer", "amount": "45000.00", "category": "41000", "issued": "2026-09-01", "due": "2026-09-20", "reference": "Invoice 9001" }),
    );
    let owed = owed["structuredContent"]["owed"]
        .as_str()
        .map(str::to_string);
    let before = dump(&dir);

    let apps = manifest::unsigned(manifest::RELEASE_KEY_ID)["apps"]
        .as_array()
        .unwrap()
        .clone();
    let reads: Vec<(&str, Value, bool)> = vec![
        ("books_app_month_pack", json!({ "month": "2026-10" }), true),
        ("books_app_evidence_review", json!({}), true),
        (
            "books_app_reconcile_document",
            json!({ "month": "2026-10" }),
            false,
        ),
        (
            "books_app_form_lineage",
            json!({ "form": "2550Q", "key": "1" }),
            false,
        ),
        (
            "books_app_evidence_preview",
            json!({ "evidence": "tkt_00000000000000000000000000" }),
            false,
        ),
        ("books_owed_document", json!({ "owed": owed }), true),
    ];
    for (tool, args, runs) in reads {
        assert!(!tools::find(tool).unwrap().write, "{tool}");
        // the open of an app that lists it, as a screen's call has
        let app = apps
            .iter()
            .find(|a| {
                a["permissions"]["tools"]
                    .as_array()
                    .is_some_and(|t| t.iter().any(|n| n == tool))
            })
            .unwrap_or_else(|| panic!("no app lists {tool}"));
        let app = app["id"].as_str().unwrap();
        let open_id = bot.open_session(app).to_string();
        let press = json!({ "origin": "app", "plugin": manifest::PLUGIN_ID, "app": app, "open_id": open_id, "ts": now_ms(), "press": { "by": "ai", "who": "agent.broker" } });
        let got = bot.raw(tool, args, press);
        assert_ne!(
            got["structuredContent"]["code"], "permission_denied",
            "{tool}: {got}"
        );
        if runs {
            assert_eq!(got["isError"], false, "{tool}: {got}");
        }
    }
    assert_eq!(dump(&dir), before, "a read under a press changes nothing");
}

#[test]
fn a_tool_that_reaches_the_cloud_is_a_write() {
    use books_mini_app::tools;
    let t = tools::find("books_app_tax_list").unwrap();
    assert!(t.write);
    let listed = tools::list()
        .into_iter()
        .find(|t| t["name"] == "books_app_tax_list")
        .unwrap();
    assert_eq!(listed["annotations"]["readOnlyHint"], false);
    assert_eq!(listed["_meta"]["buwiz/audience"], json!(["app"]));
}

/// However the Bot came to send it: `origin: app` with a press record is the AI's.
#[test]
fn an_app_only_write_with_a_press_record_is_refused_even_as_origin_app() {
    let dir = TempDir::new("press-write");
    let mut bot = Bot::new(&dir);
    data(&bot.app(
        "books_app_book_create",
        json!({ "client_request_id": "create-book-0001", "label": "Buwiz Trading" }),
    ));
    let open_id = bot.open_session("books").to_string();
    let press = json!({ "origin": "app", "plugin": manifest::PLUGIN_ID, "app": "books", "open_id": open_id, "ts": now_ms(), "press": { "by": "ai", "who": "agent.broker" } });
    let post = json!({ "client_request_id": "press-post-0001", "form": "money_out", "date": "2026-10-19", "amount": "100.00", "account": "11000", "category": "67200" });
    assert_eq!(
        code(&bot.raw("books_app_post_entry", post.clone(), press)),
        "permission_denied"
    );
    assert_eq!(
        data(&bot.app("books_app_post_entry", post))["status"],
        "posted",
        "the person's own click still runs"
    );
}

#[test]
fn allocations_keep_effective_links_in_entry_and_report_views() {
    let dir = TempDir::new("line-allocation-links");
    let mut bot = Bot::new(&dir);
    let book = app_data(
        &mut bot,
        "books_app_book_create",
        json!({ "client_request_id": "alloc-book-01", "label": "Synthetic Allocation Book" }),
    )["book"]
        .as_str()
        .unwrap()
        .to_string();
    let make_person = |bot: &mut Bot, id: &str, name: &str| {
        app_data(
            bot,
            "books_app_person_create",
            json!({ "client_request_id": id, "kind": "vendors", "name": name, "book": book }),
        )["person"]
            .as_str()
            .unwrap()
            .to_string()
    };
    let make_tag = |bot: &mut Bot, id: &str, kind: &str, name: &str| {
        app_data(
            bot,
            "books_app_tag_create",
            json!({ "client_request_id": id, "kind": kind, "name": name, "book": book }),
        )["tag"]
            .as_str()
            .unwrap()
            .to_string()
    };
    let entry_person = make_person(&mut bot, "alloc-person1", "Synthetic Entry Vendor");
    let line_person = make_person(&mut bot, "alloc-person2", "Synthetic Line Vendor");
    let entry_department = make_tag(
        &mut bot,
        "alloc-dept-01",
        "department",
        "Synthetic Entry Department",
    );
    let line_department = make_tag(
        &mut bot,
        "alloc-dept-02",
        "department",
        "Synthetic Line Department",
    );
    let entry_location = make_tag(
        &mut bot,
        "alloc-loc-001",
        "location",
        "Synthetic Entry Location",
    );
    let line_location = make_tag(
        &mut bot,
        "alloc-loc-002",
        "location",
        "Synthetic Line Location",
    );

    let mismatch = bot.app(
        "books_entry_draft",
        json!({
            "client_request_id": "alloc-mismatch-1", "book": book, "form": "money_out",
            "date": "2026-10-18", "amount": "100.00", "account": "11000",
            "allocations": [
                { "category": "67200", "amount": "60.00" },
                { "category": "66100", "amount": "39.99" }
            ]
        }),
    );
    assert_eq!(code(&mismatch), "invalid_args");

    let other_book = app_data(
        &mut bot,
        "books_app_book_create",
        json!({ "client_request_id": "alloc-book-02", "label": "Synthetic Foreign Book" }),
    )["book"]
        .as_str()
        .unwrap()
        .to_string();
    let foreign_person = app_data(
        &mut bot,
        "books_app_person_create",
        json!({
            "client_request_id": "alloc-foreign1", "book": other_book,
            "kind": "vendors", "name": "Synthetic Foreign Vendor"
        }),
    )["person"]
        .as_str()
        .unwrap()
        .to_string();
    let wrong_kind = bot.app(
        "books_entry_draft",
        json!({
            "client_request_id": "alloc-wrong-tag", "book": book, "form": "money_out",
            "date": "2026-10-18", "amount": "100.00", "account": "11000",
            "allocations": [{ "category": "67200", "amount": "100.00", "department": line_location }]
        }),
    );
    assert_eq!(code(&wrong_kind), "not_found");
    let foreign = bot.app(
        "books_entry_draft",
        json!({
            "client_request_id": "alloc-foreign2", "book": book, "form": "money_out",
            "date": "2026-10-18", "amount": "100.00", "account": "11000",
            "allocations": [{ "category": "67200", "amount": "100.00", "person": foreign_person }]
        }),
    );
    assert_eq!(code(&foreign), "invalid_args");

    let draft = app_data(
        &mut bot,
        "books_entry_draft",
        json!({
            "client_request_id": "alloc-valid-001", "book": book, "form": "money_out",
            "date": "2026-10-18", "amount": "100.00", "account": "11000",
            "department": entry_department, "location": entry_location,
            "allocations": [
                { "category": "67200", "amount": "60.00", "person": line_person,
                  "department": line_department, "location": line_location },
                { "category": "66100", "amount": "40.00", "person": entry_person }
            ]
        }),
    );
    let entry = draft["entry"].as_str().unwrap().to_string();
    let detail = app_data(&mut bot, "books_entry_get", json!({ "entry": entry }));
    let lines = detail["lines"].as_array().unwrap();
    assert_eq!(lines.len(), 3);
    assert!(lines[0]["person"]["ticket"] == line_person);
    assert_eq!(lines[0]["person"]["name"], "Synthetic Line Vendor");
    assert!(lines[0]["department"]["ticket"] == line_department);
    assert_eq!(lines[0]["department"]["name"], "Synthetic Line Department");
    assert!(lines[0]["location"]["ticket"] == line_location);
    assert_eq!(lines[0]["location"]["name"], "Synthetic Line Location");
    assert!(lines[1]["person"]["ticket"] == entry_person);
    assert!(lines[1]["department"]["ticket"] == entry_department);
    assert!(lines[1]["location"]["ticket"] == entry_location);
    assert!(!detail["title"].as_str().unwrap().contains("tkt_"));
    let model_detail = bot.agent("books_entry_get", json!({ "entry": entry }));
    let model_text = model_detail.to_string();
    assert!(!model_text.contains("Synthetic Line Vendor"));
    assert!(!model_text.contains("Synthetic Line Department"));
    assert!(!model_text.contains(line_person.as_str()));
    assert!(!model_text.contains(line_department.as_str()));
    assert!(!model_text.contains(line_location.as_str()));
    let csv = app_data(
        &mut bot,
        "books_report_csv",
        json!({ "book": book, "kind": "ledger", "filter": "draft" }),
    );
    let csv_rows: Vec<_> = csv["csv"].as_str().unwrap().lines().collect();
    assert_eq!(csv_rows.len(), 4);
    assert_eq!(csv_rows[1].split(',').nth(6), Some("Synthetic Line Vendor"));
    assert_eq!(
        csv_rows[2].split(',').nth(6),
        Some("Synthetic Entry Vendor")
    );
    assert_eq!(csv_rows[3].split(',').nth(6), Some(""));
    let card = app_data(
        &mut bot,
        "books_app_press_card",
        json!({
            "tool": "books_app_post_entry",
            "args": { "client_request_id": "alloc-card-001", "entry": entry }
        }),
    );
    let card_lines = card["lines"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(card_lines.contains("Line 1 person: Synthetic Line Vendor"));
    assert!(card_lines.contains("Line 1 Department: Synthetic Line Department"));
    assert!(card_lines.contains("Line 1 Location: Synthetic Line Location"));
    assert!(card_lines.contains("Line 2 person: Synthetic Entry Vendor"));
    assert!(!card_lines.contains(line_person.as_str()));
    assert!(!card_lines.contains(line_department.as_str()));
    assert!(!card_lines.contains(line_location.as_str()));
    let shown_cents = |text: &Value| {
        let text = text.as_str().unwrap();
        if text.is_empty() {
            0
        } else {
            books_core::money::parse(text).unwrap()
        }
    };
    let debits: i64 = lines
        .iter()
        .map(|line| shown_cents(&line["debit_shown"]))
        .sum();
    let credits: i64 = lines
        .iter()
        .map(|line| shown_cents(&line["credit_shown"]))
        .sum();
    assert_eq!((debits, credits), (10_000, 10_000));

    data(&bot.app(
        "books_app_post_entry",
        json!({ "client_request_id": "alloc-post-001", "entry": entry }),
    ));
    let copy = app_data(
        &mut bot,
        "books_entry_draft",
        json!({ "client_request_id": "alloc-copy-001", "book": book, "copy_of": entry }),
    );
    let copied_entry = copy["entry"].as_str().unwrap().to_string();
    let copied = app_data(
        &mut bot,
        "books_entry_get",
        json!({ "entry": copied_entry }),
    );
    assert!(
        copied["lines"][0]["person"]["ticket"] == line_person
            && copied["lines"][0]["department"]["ticket"] == line_department
            && copied["lines"][0]["location"]["ticket"] == line_location
    );
    let tagged = app_data(
        &mut bot,
        "books_tags_list",
        json!({ "kind": "department", "month": "2026-10", "book": book }),
    );
    let line_tag = tagged["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["ticket"] == line_department)
        .unwrap();
    assert_eq!(line_tag["spent_shown"], "60.00");
    assert_eq!(line_tag["count"], 1);
    let entry_tag = tagged["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["ticket"] == entry_department)
        .unwrap();
    assert_eq!(entry_tag["spent_shown"], "40.00");
    let profit_and_loss = app_data(
        &mut bot,
        "books_report",
        json!({ "book": book, "kind": "pl", "period_kind": "month", "period": "2026-10", "tag": line_department }),
    );
    let line_expense = profit_and_loss["sections"][1]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["number"] == "67200")
        .unwrap();
    assert_eq!(line_expense["shown"], "60.00");

    let cash_flow = app_data(
        &mut bot,
        "books_report",
        json!({ "book": book, "kind": "cf", "period_kind": "month", "period": "2026-10", "tag": line_department }),
    );
    assert_eq!(cash_flow["sections"][0]["rows"][1]["shown"], "−60.00");
    let entry_cash_flow = app_data(
        &mut bot,
        "books_report",
        json!({ "book": book, "kind": "cf", "period_kind": "month", "period": "2026-10", "tag": entry_department }),
    );
    assert_eq!(entry_cash_flow["sections"][0]["rows"][1]["shown"], "−40.00");
    let checks = bot.agent(
        "books_checks",
        json!({ "book": book, "check": "missing_person" }),
    );
    assert_eq!(data(&checks)["total"], 0);
    let person_statement = app_data(
        &mut bot,
        "books_report",
        json!({ "book": book, "kind": "ps", "person": line_person, "period_kind": "month", "period": "2026-10" }),
    );
    assert_eq!(person_statement["activity"]["total_out_shown"], "60.00");
    assert_eq!(
        person_statement["activity"]["rows"][0]["what"],
        "Office Supplies"
    );
}

#[test]
fn transfer_parties_persist_show_in_views_and_refuse_foreign_books() {
    let dir = TempDir::new("transfer-line-people");
    let mut bot = Bot::new(&dir);
    let book = app_data(
        &mut bot,
        "books_app_book_create",
        json!({ "client_request_id": "transfer-book-01", "label": "Synthetic Transfer Book" }),
    )["book"]
        .as_str()
        .unwrap()
        .to_string();
    let make_person = |bot: &mut Bot, request_id: &str, book: &str, kind: &str, name: &str| {
        app_data(
            bot,
            "books_app_person_create",
            json!({
                "client_request_id": request_id, "book": book, "kind": kind, "name": name
            }),
        )["person"]
            .as_str()
            .unwrap()
            .to_string()
    };
    let from_person = make_person(
        &mut bot,
        "transfer-source-01",
        &book,
        "vendors",
        "Synthetic Transfer Source",
    );
    let to_person = make_person(
        &mut bot,
        "transfer-target-01",
        &book,
        "customers",
        "Synthetic Transfer Destination",
    );
    app_data(
        &mut bot,
        "books_app_account_create",
        json!({
            "client_request_id": "transfer-account-01", "book": book,
            "number": "11010", "name": "Synthetic Transfer Bank",
            "kind": "bank_accounts", "type": "asset"
        }),
    );
    let accounts = app_data(&mut bot, "books_accounts_list", json!({ "book": book }));
    assert!(
        accounts["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|account| account["number"] == "11000")
    );
    assert!(
        accounts["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|account| account["number"] == "11010")
    );
    let draft_args = json!({
        "client_request_id": "transfer-draft-01", "book": book, "form": "transfer",
        "date": "2026-10-18", "amount": "75.00", "account": "11000", "to_account": "11010",
        "from_person": from_person, "to_person": to_person
    });
    let card = app_data(
        &mut bot,
        "books_app_press_card",
        json!({ "book": book, "tool": "books_app_post_entry", "args": draft_args }),
    );
    let card_lines = card["lines"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(card_lines.contains("Transfer to party: Synthetic Transfer Destination"));
    assert!(card_lines.contains("Transfer from party: Synthetic Transfer Source"));
    assert!(!card_lines.contains(from_person.as_str()));
    assert!(!card_lines.contains(to_person.as_str()));

    let draft_args = json!({
        "client_request_id": "transfer-draft-01", "book": book, "form": "transfer",
        "date": "2026-10-18", "amount": "75.00", "account": "11000", "to_account": "11010",
        "from_person": from_person, "to_person": to_person
    });
    let draft = bot.app("books_entry_draft", draft_args.clone());
    assert_eq!(data(&draft)["status"], "draft");
    assert_eq!(bot.app("books_entry_draft", draft_args.clone()), draft);
    let mut changed = draft_args.clone();
    changed["from_person"] = json!(to_person);
    assert_eq!(
        code(&bot.app("books_entry_draft", changed)),
        "request_id_reused"
    );

    let entry = data(&draft)["entry"].as_str().unwrap().to_string();
    let detail = app_data(&mut bot, "books_entry_get", json!({ "entry": entry }));
    assert_eq!(detail["lines"].as_array().unwrap().len(), 2);
    assert_eq!(detail["lines"][0]["debit_shown"], "75.00");
    assert_eq!(detail["lines"][0]["person"]["ticket"], to_person);
    assert_eq!(
        detail["lines"][0]["person"]["name"],
        "Synthetic Transfer Destination"
    );
    assert_eq!(detail["lines"][1]["credit_shown"], "75.00");
    assert_eq!(detail["lines"][1]["person"]["ticket"], from_person);
    assert_eq!(
        detail["lines"][1]["person"]["name"],
        "Synthetic Transfer Source"
    );

    let unsupported_entry_link = bot.app(
        "books_app_press_card",
        json!({
            "tool": "books_app_post_entry",
            "args": {
                "client_request_id": "transfer-entry-card-01",
                "entry": entry,
                "from_person": from_person
            }
        }),
    );
    assert_eq!(code(&unsupported_entry_link), "permission_denied");

    let foreign_book = app_data(
        &mut bot,
        "books_app_book_create",
        json!({
            "client_request_id": "transfer-book-02", "label": "Synthetic Foreign Transfer Book"
        }),
    )["book"]
        .as_str()
        .unwrap()
        .to_string();
    let foreign_person = make_person(
        &mut bot,
        "transfer-foreign-01",
        &foreign_book,
        "vendors",
        "Synthetic Foreign Transfer Person",
    );
    for (request_id, field) in [
        ("transfer-foreign-from", "from_person"),
        ("transfer-foreign-to", "to_person"),
    ] {
        let mut args = json!({
            "client_request_id": request_id, "book": book, "form": "transfer",
            "date": "2026-10-18", "amount": "75.00", "account": "11000", "to_account": "11010"
        });
        args[field] = json!(foreign_person);
        assert_eq!(code(&bot.app("books_entry_draft", args)), "invalid_args");
    }

    drop(bot);
    let store = books_store::Store::open(&Paths::in_dir(dir.0.join("store"))).unwrap();
    let (kind, entry_id) = books_store::plumbing::subject(store.conn(), &entry)
        .unwrap()
        .unwrap();
    assert_eq!(kind, "entry");
    let (_, from_person_id) = books_store::plumbing::subject(store.conn(), &from_person)
        .unwrap()
        .unwrap();
    let (_, to_person_id) = books_store::plumbing::subject(store.conn(), &to_person)
        .unwrap()
        .unwrap();
    let lines = books_store::entries::lines(store.conn(), entry_id).unwrap();
    assert_eq!(lines[0].person_id, Some(to_person_id));
    assert_eq!(lines[1].person_id, Some(from_person_id));
}

#[test]
fn a_person_linked_only_on_the_money_line_still_counts() {
    let dir = TempDir::new("bank-line-person");
    let mut bot = Bot::new(&dir);
    let book = app_data(
        &mut bot,
        "books_app_book_create",
        json!({ "client_request_id": "bankp-book-1", "label": "Synthetic Bank-Link Book" }),
    )["book"]
        .as_str()
        .unwrap()
        .to_string();
    let vendor = app_data(
        &mut bot,
        "books_app_person_create",
        json!({ "client_request_id": "bankp-per-1", "kind": "vendors", "name": "Synthetic Bank Vendor", "book": book }),
    )["person"]
        .as_str()
        .unwrap()
        .to_string();
    // a payment journal whose only party link sits on the bank line
    let draft = app_data(
        &mut bot,
        "books_entry_draft",
        json!({
            "client_request_id": "bankp-draft-1", "book": book, "form": "journal",
            "date": "2026-10-19",
            "lines": [
                { "account": "66100", "debit": "500.00" },
                { "account": "11000", "credit": "500.00", "person": vendor }
            ]
        }),
    );
    let entry = draft["entry"].as_str().unwrap().to_string();
    data(&bot.app(
        "books_app_post_entry",
        json!({ "client_request_id": "bankp-post-1", "entry": entry }),
    ));
    let statement = app_data(
        &mut bot,
        "books_report",
        json!({ "book": book, "kind": "ps", "person": vendor, "period_kind": "month", "period": "2026-10" }),
    );
    assert_eq!(
        statement["activity"]["total_out_shown"], "500.00",
        "a vendor linked on the money line paid 500.00"
    );
    let detail = app_data(
        &mut bot,
        "books_person_get",
        json!({ "book": book, "person": vendor }),
    );
    let paid = detail["kpi"]
        .as_array()
        .unwrap()
        .iter()
        .find(|k| k["label"] == "Paid this year")
        .expect("the paid-this-year figure");
    assert_eq!(paid["shown"], "500.00");
}

#[test]
fn a_press_card_preview_makes_no_tags_or_people() {
    let dir = TempDir::new("preview-writes");
    let mut bot = Bot::new(&dir);
    let book = app_data(
        &mut bot,
        "books_app_book_create",
        json!({ "client_request_id": "prev-book-1", "label": "Synthetic Preview Book" }),
    )["book"]
        .as_str()
        .unwrap()
        .to_string();
    let card = app_data(
        &mut bot,
        "books_app_press_card",
        json!({
            "tool": "books_app_post_entry",
            "args": {
                "client_request_id": "prev-card-1", "book": book, "form": "money_out",
                "date": "2026-10-18", "amount": "25.00", "account": "11000",
                "category": "66100", "department": "Synthetic Preview Dept",
                "payee": "Synthetic Preview Vendor", "person_kind": "vendors"
            }
        }),
    );
    let shown = card["lines"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        shown.contains("Department: Synthetic Preview Dept"),
        "{shown}"
    );
    assert!(shown.contains("Synthetic Preview Vendor"), "{shown}");
    let tags = app_data(
        &mut bot,
        "books_tags_list",
        json!({ "kind": "department", "month": "2026-10", "book": book }),
    );
    assert!(
        tags["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["name"] != "Synthetic Preview Dept"),
        "the preview must not save the tag: {tags}"
    );
    let people = app_data(
        &mut bot,
        "books_people_list",
        json!({ "book": book, "kind": "vendors" }),
    );
    assert!(
        people["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["name"] != "Synthetic Preview Vendor"),
        "the preview must not save the person: {people}"
    );
}

#[test]
fn a_person_on_the_entry_header_still_counts_the_payment() {
    let dir = TempDir::new("entry-level-person");
    let mut bot = Bot::new(&dir);
    let book = app_data(
        &mut bot,
        "books_app_book_create",
        json!({ "client_request_id": "entp-book-1", "label": "Synthetic Header-Person Book" }),
    )["book"]
        .as_str()
        .unwrap()
        .to_string();
    let vendor = app_data(
        &mut bot,
        "books_app_person_create",
        json!({ "client_request_id": "entp-per-1", "kind": "vendors", "name": "Synthetic Header Vendor", "book": book }),
    )["person"]
        .as_str()
        .unwrap()
        .to_string();
    // a plain payment with the vendor on the draft's header, no per-line link
    let draft = app_data(
        &mut bot,
        "books_entry_draft",
        json!({
            "client_request_id": "entp-draft-1", "book": book, "form": "money_out",
            "date": "2026-10-19", "amount": "300.00", "account": "11000",
            "category": "66100", "person": vendor
        }),
    );
    let entry = draft["entry"].as_str().unwrap().to_string();
    data(&bot.app(
        "books_app_post_entry",
        json!({ "client_request_id": "entp-post-1", "entry": entry }),
    ));
    let statement = app_data(
        &mut bot,
        "books_report",
        json!({ "book": book, "kind": "ps", "person": vendor, "period_kind": "month", "period": "2026-10" }),
    );
    assert_eq!(
        statement["activity"]["total_out_shown"], "300.00",
        "the header-linked vendor paid 300.00"
    );
    let detail = app_data(
        &mut bot,
        "books_person_get",
        json!({ "book": book, "person": vendor }),
    );
    let paid = detail["kpi"]
        .as_array()
        .unwrap()
        .iter()
        .find(|k| k["label"] == "Paid this year")
        .expect("the paid-this-year figure");
    assert_eq!(paid["shown"], "300.00");
}
