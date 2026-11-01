//! `books_owed_document`: an owed document as a static HTML print page for
//! `buwiz.preview`, through the server as the Bot calls it. App origin only,
//! no script or network reference, no memo or ticket, every text escaped.
//! Synthetic data only.

use books_mini_app::b64;
use books_mini_app::jsonrpc::Server;
use books_mini_app::manifest::{self, Source};
use books_mini_app::service::Books;
use books_store::{Paths, TempDir};
use ed25519_dalek::SigningKey;
use serde_json::{Value, json};

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

const MEMO: &str = "MEMO-SYNTH-print-check";

fn setup(bot: &mut Bot) -> (String, String) {
    let made = bot.app(
        "books_app_book_create",
        json!({ "client_request_id": "doc-book-0001", "label": "Synthetic Trading" }),
    );
    assert_eq!(made["isError"], false, "{made}");
    let inv = bot.app(
        "books_app_owed_create",
        json!({ "client_request_id": "doc-owed-0001", "side": "collect", "person_name": "Synthetic <b>&Co", "amount": "45000.00", "category": "41000", "issued": "2026-09-01", "due": "2026-09-20", "reference": "SOA-SYN-01", "memo": MEMO }),
    );
    let bill = bot.app(
        "books_app_owed_create",
        json!({ "client_request_id": "doc-owed-0002", "side": "pay", "person_name": "Synthetic Vendor", "amount": "3200.00", "category": "67200", "issued": "2026-10-01" }),
    );
    (
        data(&inv)["owed"].as_str().unwrap().to_string(),
        data(&bill)["owed"].as_str().unwrap().to_string(),
    )
}

fn html(result: &Value) -> String {
    assert_eq!(result["isError"], false, "{result}");
    let c = &result["content"][0];
    assert_eq!(c["type"], "resource");
    assert_eq!(c["resource"]["mimeType"], "text/html");
    assert_eq!(c["resource"]["uri"], "books://print/owed.html");
    c["resource"]["text"].as_str().unwrap().to_string()
}

#[test]
fn a_collect_document_is_a_static_statement_of_account() {
    let dir = TempDir::new("owed-doc");
    let mut bot = Bot::new(&dir);
    let (inv, _) = setup(&mut bot);
    let r = bot.app("books_owed_document", json!({ "owed": inv }));
    let page = html(&r);
    assert!(page.starts_with("<!DOCTYPE html>"));
    let title = r["structuredContent"]["title"].as_str().unwrap();
    assert_eq!(title, "Statement of account");
    assert!(!title.chars().any(|c| c.is_ascii_digit()));
    for bad in [
        "<script",
        "<link",
        "<img",
        "http",
        "url(",
        MEMO,
        inv.as_str(),
    ] {
        assert!(!page.contains(bad), "page carries {bad}");
    }
    assert!(page.contains("Synthetic &lt;b&gt;&amp;Co"));
    assert!(!page.contains("<b>&Co"));
    for want in [
        "Synthetic Trading",
        "<h1>Statement of account</h1>",
        "SOA-SYN-01",
        "2026-09-01",
        "2026-09-20",
        "PHP 45,000.00",
        "Open",
        "This is not an official receipt or invoice.",
    ] {
        assert!(page.contains(want), "page lacks {want}");
    }
    // a settled document says so with its date
    let s = bot.app("books_app_owed_settle", json!({ "client_request_id": "doc-settle-01", "owed": inv, "account": "11000", "date": "2026-10-29" }));
    assert_eq!(s["isError"], false, "{s}");
    let page = html(&bot.app("books_owed_document", json!({ "owed": inv })));
    assert!(page.contains("Settled on 2026-10-29"));
}

#[test]
fn a_pay_document_is_a_bill_record_and_refusals_hold() {
    let dir = TempDir::new("owed-doc-pay");
    let mut bot = Bot::new(&dir);
    let (inv, bill) = setup(&mut bot);
    let r = bot.app("books_owed_document", json!({ "owed": bill }));
    let page = html(&r);
    assert_eq!(r["structuredContent"]["title"], "Bill record");
    assert!(page.contains("<h1>Bill record</h1>"));
    assert!(page.contains("No due date"));
    let c = bot.app(
        "books_app_owed_cancel",
        json!({ "client_request_id": "doc-cancel-01", "owed": bill }),
    );
    assert_eq!(c["isError"], false, "{c}");
    assert!(html(&bot.app("books_owed_document", json!({ "owed": bill }))).contains("Cancelled"));
    // an unknown ticket: the existing not-found refusal
    let unknown = bot.app(
        "books_owed_document",
        json!({ "owed": "tkt_synthetic_unknown_0000" }),
    );
    assert_eq!(code(&unknown), "not_found");
    // a ticket of another kind is not a document
    let book = data(&bot.app("books_book_get", json!({})))["ticket"]
        .as_str()
        .expect("books_book_get returns the book ticket")
        .to_string();
    assert_eq!(
        code(&bot.app("books_owed_document", json!({ "owed": book }))),
        "not_found"
    );
    // the assistant never gets the page
    assert_eq!(
        code(&bot.agent("books_owed_document", json!({ "owed": inv }))),
        "permission_denied"
    );
    // and the tool writes nothing
    assert!(books_mini_app::tools::read_only("books_owed_document"));
}

#[test]
fn the_tool_refuses_a_non_app_origin_behind_the_audience_check() {
    let dir = TempDir::new("owed-doc-origin");
    let books = Books::new(Paths::in_dir(dir.0.join("store")));
    let mut books = books;
    let args = serde_json::Map::new();
    let r =
        books_mini_app::owed_doc::tool(&mut books, &args, books_mini_app::service::Origin::Agent);
    assert_eq!(code(&r), "permission_denied");
    assert!(!dir.0.join("store/books.db").exists());
}

#[test]
fn escaping_covers_every_markup_character() {
    assert_eq!(
        books_mini_app::owed_doc::esc("a&<>\"'b"),
        "a&amp;&lt;&gt;&quot;&#39;b"
    );
}

#[test]
fn only_the_books_window_may_preview() {
    let m = manifest::unsigned(manifest::RELEASE_KEY_ID);
    for app in m["apps"].as_array().unwrap() {
        let tools = app["permissions"]["tools"].as_array().unwrap();
        assert!(
            tools.len() <= 64,
            "{} names {} tools",
            app["id"],
            tools.len()
        );
        if app["id"] == "books" {
            assert_eq!(
                app["permissions"]["preview"],
                json!({ "max_bytes": 1_048_576 })
            );
            assert!(tools.iter().any(|t| t == "books_owed_document"));
        } else {
            assert!(
                app["permissions"].get("preview").is_none(),
                "{} may preview",
                app["id"]
            );
        }
    }
}

#[test]
fn only_owed_print_calls_preview_and_it_hides_without_the_method() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps");
    let mut callers = Vec::new();
    for dir in ["books", "cards"] {
        for f in std::fs::read_dir(root.join(dir)).unwrap() {
            let path = f.unwrap().path();
            let src = std::fs::read_to_string(&path).unwrap();
            if src.contains(".preview(") {
                callers.push(format!(
                    "{dir}/{}",
                    path.file_name().unwrap().to_string_lossy()
                ));
            }
        }
    }
    assert_eq!(callers, ["books/owed_print.tsx"]);
    let src = std::fs::read_to_string(root.join("books/owed_print.tsx")).unwrap();
    assert!(src.contains("typeof buwiz.preview === \"function\""));
    assert!(src.contains("\"books_owed_document\""));
    assert!(src.contains("Print preview needs Buwiz Bot on macOS."));
    let unpaid = std::fs::read_to_string(root.join("books/unpaid.tsx")).unwrap();
    assert_eq!(unpaid.matches("<OwedPrintBtn ").count(), 1);
}
