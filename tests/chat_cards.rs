//! M8 through the server, as Rox calls it: the assistant offers cards and
//! never posts or hands over (M8-07), a routine's turn writes only the
//! drafts it declares (M8-06), the six scopes and `/books` resolve to
//! tickets and cards (M8-01, M8-04), the templates open their cards
//! (M8-05), and quiet cards carry counts, never amounts. Synthetic data.

use books_mini_app::demo;
use books_mini_app::jsonrpc::Server;
use books_mini_app::manifest::{self, Source};
use books_mini_app::service::Books;
use books_mini_app::{b64, tools};
use books_store::{Paths, TempDir};
use ed25519_dalek::SigningKey;
use serde_json::{Value, json};

const DEMO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/demo/buwiz-trading-2026.json"
);

struct Rox {
    server: Server,
    next: i64,
}

/// The tests here move the pinned date, which every test in this binary
/// reads: one at a time.
static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn now_ms() -> i64 {
    i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap()
}

impl Rox {
    fn new(dir: &TempDir) -> Self {
        books_core::date::pin_today(books_core::date::Date::new(2026, 10, 29));
        let mut books = Books::new(Paths::in_dir(dir.0.join("store")));
        let demo_book: Value =
            serde_json::from_str(&std::fs::read_to_string(DEMO).unwrap()).unwrap();
        demo::seed(&mut books, &demo_book).unwrap();
        let mut rox = Self {
            server: Server::new(Source::default(), books),
            next: 1,
        };
        let seed = [11u8; 32];
        let public = b64::encode(SigningKey::from_bytes(&seed).verifying_key().as_bytes());
        let p = rox.raw("rox.provision", json!({ "protocol": "1.0.0", "plugin": manifest::PLUGIN_ID, "key": { "alg": "ed25519", "seed": b64::encode(&seed) }, "pub": public }), Value::Null);
        assert_eq!(p["isError"], false, "{p}");
        rox
    }

    fn open(&mut self, app: &str, open_id: &str) {
        let entry = manifest::unsigned(manifest::RELEASE_KEY_ID)["apps"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["id"] == app)
            .unwrap()
            .clone();
        let perms = manifest::permissions_digest(&entry).unwrap();
        let open = json!({ "app": app, "open_id": open_id, "ks_pub": b64::encode(&[5u8; 32]), "perms_sha256": perms, "not_after": now_ms() + 3_600_000 });
        let o = self.raw("rox.session.open", open, Value::Null);
        assert_eq!(o["isError"], false, "{o}");
    }

    fn raw(&mut self, name: &str, args: Value, caller: Value) -> Value {
        let id = self.next;
        self.next += 1;
        let mut params = json!({ "name": name, "arguments": args });
        if !caller.is_null() {
            params["_meta"] = json!({ "rox/caller": caller });
        }
        self.server
            .handle(
                &json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call", "params": params }),
            )
            .unwrap()["result"]
            .clone()
    }

    fn agent(&mut self, name: &str, args: Value) -> Value {
        self.raw(
            name,
            args,
            json!({ "origin": "agent", "plugin": manifest::PLUGIN_ID, "ts": now_ms() }),
        )
    }

    fn routine(&mut self, routine: &str, name: &str, args: Value) -> Value {
        self.raw(name, args, json!({ "origin": "agent", "routine": routine, "plugin": manifest::PLUGIN_ID, "ts": now_ms() }))
    }

    fn app(&mut self, app: &str, open_id: &str, name: &str, args: Value) -> Value {
        self.raw(name, args, json!({ "origin": "app", "plugin": manifest::PLUGIN_ID, "app": app, "open_id": open_id, "ts": now_ms() }))
    }
}

fn data(r: &Value) -> Value {
    assert_eq!(r["isError"], false, "{r}");
    r["structuredContent"].clone()
}

fn refused(r: &Value) -> String {
    assert_eq!(r["isError"], true, "{r}");
    r["content"][0]["text"].as_str().unwrap_or("").to_string()
}

#[test]
fn m8_07_the_assistant_cannot_post_close_or_hand_over() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let dir = TempDir::new("m8-guard");
    let mut rox = Rox::new(&dir);
    // every app-only tool, called by the assistant, is refused before it runs
    for t in tools::TOOLS.iter().filter(|t| t.reach == tools::Reach::App) {
        let text = refused(&rox.agent(t.name, json!({ "client_request_id": "agent-try-0001" })));
        assert!(
            text.contains("needs the user's click"),
            "{}: {text}",
            t.name
        );
    }
    // the assistant may draft
    let d = rox.agent("books_entry_draft", json!({ "client_request_id": "agent-draft-01", "form": "money_out", "date": "2026-10-20", "amount": "150.00", "account": "11010", "category": "68300", "payee": "Bank" }));
    assert_eq!(data(&d)["status"], "draft");
}

#[test]
fn m8_06_routines_draft_only() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let dir = TempDir::new("m8-routine");
    let mut rox = Rox::new(&dir);
    // the weekly tidy-up declares books_entry_draft, and may call it
    let d = rox.routine("weekly-tidy", "books_entry_draft", json!({ "client_request_id": "tidy-draft-0001", "form": "money_out", "date": "2026-10-21", "amount": "150.00", "account": "11010", "category": "68300" }));
    assert_eq!(data(&d)["status"], "draft");
    // nothing else, and no other routine writes at all
    for (routine, tool) in [
        ("weekly-tidy", "books_app_post_entry"),
        ("weekly-tidy", "books_app_month_close"),
        ("month-end", "books_entry_draft"),
        ("bir-pair", "books_app_handoff_create"),
        ("bir-pair", "books_entry_draft"),
    ] {
        let text = refused(&rox.routine(
            routine,
            tool,
            json!({ "client_request_id": "routine-try-01" }),
        ));
        assert!(
            text.contains("routine") || text.contains("click"),
            "{routine} {tool}: {text}"
        );
    }
    // the query call (origin routine) only reads
    let q = rox.raw("books_entry_draft", json!({ "client_request_id": "query-try-0001" }), json!({ "origin": "routine", "routine": "weekly-tidy", "plugin": manifest::PLUGIN_ID, "ts": now_ms() }));
    assert!(refused(&q).contains("does not run here"));
    // the manifest says the same: only the tidy-up writes, and only drafts
    let m = manifest::unsigned(manifest::RELEASE_KEY_ID);
    let writes: Vec<(String, Value)> = m["routines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["id"].as_str().unwrap().to_string(),
                r.get("writes").cloned().unwrap_or(json!([])),
            )
        })
        .collect();
    assert_eq!(
        writes,
        [
            ("weekly-tidy".into(), json!(["books_entry_draft"])),
            ("month-end".into(), json!([])),
            ("bir-pair".into(), json!([]))
        ]
    );
}

#[test]
fn m8_nudges_find_work_and_quiet_cards_carry_no_amounts() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let dir = TempDir::new("m8-nudge");
    let mut rox = Rox::new(&dir);
    // Thursday Oct 29: the tidy-up finds work on Mondays only; asked, it answers
    let thursday = data(&rox.routine("weekly-tidy", "books_nudge", json!({ "kind": "tidy" })));
    assert_eq!(thursday["text"], "");
    assert_eq!(thursday["counts"]["lines"], 2);
    let tidy = data(&rox.routine(
        "weekly-tidy",
        "books_nudge",
        json!({ "kind": "tidy", "any_day": true }),
    ));
    assert_eq!(tidy["text"], "2 bank lines to categorize", "{tidy}");
    assert_eq!(tidy["items"].as_array().unwrap().len(), 2);
    // Oct 29: September is open but the nudge waits for the first week
    let month = data(&rox.agent("books_nudge", json!({ "kind": "month_end" })));
    assert_eq!(
        (month["month"].as_str(), month["text"].as_str()),
        (Some("2026-09"), Some(""))
    );
    books_core::date::pin_today(books_core::date::Date::new(2026, 11, 2));
    let month = data(&rox.agent("books_nudge", json!({ "kind": "month_end" })));
    assert_eq!(month["month"], "2026-10");
    assert!(
        month["text"]
            .as_str()
            .unwrap()
            .starts_with("October is still open."),
        "{month}"
    );
    assert_eq!(month["items"][0]["id"], "close-2026-10");
    books_core::date::pin_today(books_core::date::Date::new(2026, 10, 29));
    let pair = data(&rox.agent("books_nudge", json!({ "kind": "bir_pair" })));
    assert_eq!(
        (pair["text"].as_str(), pair["count"].as_i64()),
        (Some(""), Some(0))
    );
    // no money text in any nudge (quiet cards read only these)
    for n in [&tidy, &month, &pair] {
        let s = n.to_string();
        assert!(!s.contains(".00") && !s.contains('₱'), "{s}");
    }
    // the quiet app reaches books_nudge and nothing else
    let m = manifest::unsigned(manifest::RELEASE_KEY_ID);
    let quiet = m["apps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == "quiet")
        .unwrap();
    assert_eq!(quiet["permissions"]["tools"], json!(["books_nudge"]));
}

#[test]
fn m8_cards_scopes_books_choices_and_templates() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let dir = TempDir::new("m8-scopes");
    let mut rox = Rox::new(&dir);
    // books_show offers a card on the result's _meta, not in its text
    let shown = rox.agent("books_show", json!({ "card": "cash" }));
    assert_eq!(shown["isError"], false, "{shown}");
    let open = &shown["_meta"]["rox/open"];
    assert_eq!(
        (
            open["app"].as_str(),
            open["props"]["card"].as_str(),
            open["placement"].as_str()
        ),
        (Some("answer"), Some("cash"), Some("inline"))
    );
    assert!(
        !shown["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("rox/open")
    );
    assert!(
        refused(&rox.agent("books_show", json!({ "card": "nope" }))).contains("`card` is one of")
    );

    let resolve = |rox: &mut Rox, kind: &str, name: &str, value: &str, complete: bool| {
        data(&rox.raw("books_resolve", json!({ "kind": kind, "name": name, "value": value, "complete": complete, "context": { "attached": [] } }), json!({ "origin": "resolve", "plugin": manifest::PLUGIN_ID, "ts": now_ms() })))
    };
    // six scopes: text gives choices whose values are tickets; a ticket gives its token
    for (name, typed, label) in [
        ("person", "Lumen", "Lumen Power Co."),
        ("account", "Bank Acc", "11000 Bank Accounts"),
        ("period", "October", "October 2026"),
        ("entry", "Isla", ""),
        ("form", "2551Q", "2551Q Q3 2026"),
        ("tag", "Makati", "Makati office"),
    ] {
        let c = resolve(&mut rox, "scope", name, typed, true);
        assert_eq!(c["result"], "choices", "{name}: {c}");
        let first = &c["choices"][0];
        if !label.is_empty() {
            assert_eq!(first["label"], label, "{name}: {c}");
        }
        let t = resolve(
            &mut rox,
            "scope",
            name,
            first["value"].as_str().unwrap(),
            false,
        );
        assert_eq!(t["result"], "token", "{name}: {t}");
        let title = t["title"].as_str().unwrap();
        assert!(
            !title.contains("Lumen") && !title.contains(".00"),
            "a token title is neutral: {title}"
        );
    }
    // a form token works where a form and period go
    let f = resolve(&mut rox, "scope", "form", "2551Q Q3", true);
    let ticket = f["choices"][0]["value"].as_str().unwrap().to_string();
    let p = data(&rox.agent("books_form_proposal", json!({ "form": ticket })));
    assert_eq!(
        (p["form"].as_str(), p["period"].as_str()),
        (Some("2551Q"), Some("2026-Q3"))
    );
    // a ticket of another kind is not a token of this scope
    assert_eq!(
        resolve(&mut rox, "scope", "person", &ticket, false)["result"],
        "error"
    );

    // /books: a page, a card, the choices, and plain /books
    let ledger = resolve(&mut rox, "command", "books", "ledger", false);
    assert_eq!(
        (
            ledger["result"].as_str(),
            ledger["props"]["page"].as_str(),
            ledger["placement"].as_str()
        ),
        (Some("open"), Some("ledger"), Some("full"))
    );
    let cash = resolve(&mut rox, "command", "books", "cash", false);
    assert_eq!(
        (cash["app"].as_str(), cash["placement"].as_str()),
        (Some("answer"), Some("inline"))
    );
    let choices = resolve(&mut rox, "command", "books", "", true);
    let labels: Vec<&str> = choices["choices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["label"].as_str().unwrap())
        .collect();
    assert_eq!(
        &labels[..5],
        ["Today", "Ledger", "Unpaid", "Reconcile", "Reports"]
    );
    let picked = resolve(
        &mut rox,
        "command",
        "books",
        choices["choices"][1]["value"].as_str().unwrap(),
        false,
    );
    assert_eq!(picked["props"]["page"], "ledger");
    assert_eq!(
        resolve(&mut rox, "command", "books", "", false)["props"]["page"],
        "today"
    );

    // the five templates open their cards inline
    for (command, app, card) in [
        ("books-receipt", "action", "draft"),
        ("books-tidy", "action", "bank_lines"),
        ("books-close", "action", "close"),
        ("books-quarter", "pair", "form"),
        ("books-results", "report", "pl"),
    ] {
        let o = resolve(&mut rox, "command", command, "", false);
        assert_eq!(
            (
                o["result"].as_str(),
                o["app"].as_str(),
                o["props"]["card"].as_str(),
                o["placement"].as_str()
            ),
            (Some("open"), Some(app), Some(card), Some("inline")),
            "{command}: {o}"
        );
    }
    assert_eq!(
        resolve(&mut rox, "command", "books-close", "", false)["props"]["month"],
        "2026-09"
    );

    // a card app reaches only its own tools: the answer card cannot post
    let oid = b64::encode(&[7u8; 16]);
    rox.open("answer", &oid);
    let text = refused(&rox.app(
        "answer",
        &oid,
        "books_app_post_entry",
        json!({ "client_request_id": "answer-post-01" }),
    ));
    assert!(text.contains("not in this app's permissions"), "{text}");
    assert_eq!(
        data(&rox.app("answer", &oid, "books_today", json!({})))["cash"]["shown"],
        "417,350.00"
    );
}
