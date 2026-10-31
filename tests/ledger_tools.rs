//! The M1 tools through the server, as the Bot calls them: the app makes the
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

const MEMO: &str = "MEMO-SYNTH-office-supplies";
const PAYEE: &str = "Pixel Print Shop";

struct Bot {
    server: Server,
    next: i64,
    open_id: String,
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
            open_id: b64::encode(&[4u8; 16]),
        };
        let seed = [11u8; 32];
        let public = b64::encode(SigningKey::from_bytes(&seed).verifying_key().as_bytes());
        let p = bot.raw("buwiz.provision", json!({ "protocol": "2.0.0", "plugin": manifest::PLUGIN_ID, "key": { "alg": "ed25519", "seed": b64::encode(&seed) }, "pub": public }), Value::Null);
        assert_eq!(p["isError"], false, "{p}");
        let app = manifest::unsigned(manifest::RELEASE_KEY_ID)["apps"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["id"] == "books")
            .unwrap()
            .clone();
        let perms = manifest::permissions_digest(&app).unwrap();
        let open = json!({ "app": "books", "open_id": bot.open_id, "ks_pub": b64::encode(&[5u8; 32]), "perms_sha256": perms, "not_after": now_ms() + 3_600_000 });
        let o = bot.raw("buwiz.session.open", open, Value::Null);
        assert_eq!(o["isError"], false, "{o}");
        bot
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
        let caller = json!({ "origin": "app", "plugin": manifest::PLUGIN_ID, "app": "books", "open_id": self.open_id, "ts": now_ms() });
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
    // a TIN in the label is refused (Q-4 option a)
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

    // the assistant cannot post or void (DD-20); the refusal says why
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

    // the model's ledger view: amounts and tickets, no payee or memo (D11)
    let seen = bot.agent("books_ledger_list", json!({ "month": "2026-10" }));
    let text = seen.to_string();
    assert!(!text.contains(PAYEE) && !text.contains(MEMO), "{text}");
    assert_eq!(data(&seen)["counts"]["draft"], 1);
    assert_eq!(data(&seen)["rows"][0]["amount_shown"], "\u{2212}1,350.00");
    // the app's view has them
    let app_view = bot.app("books_ledger_list", json!({ "month": "2026-10" }));
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
    let all = bot.app("books_ledger_list", json!({ "month": "2026-10" }));
    assert_eq!(
        data(&all)["counts"]["all"],
        3,
        "supplies, the voided sale and its reversal"
    );

    // the ticket titles are neutral (P3)
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

    // the encrypted file holds no plain text (M1 exit)
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
