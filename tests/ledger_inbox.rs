//! The needs-review inbox (M10 parity, ledger-1) through the server: field
//! confidence and what-changed notes on drafts, the likely-duplicate flag,
//! and the post gate the user clears with "Keep both". Synthetic data only.

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
        let p = bot.raw("buwiz.provision", json!({ "protocol": "1.0.0", "plugin": manifest::PLUGIN_ID, "key": { "alg": "ed25519", "seed": b64::encode(&seed) }, "pub": public }), Value::Null);
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

fn setup(bot: &mut Bot) {
    let made = bot.app(
        "books_app_book_create",
        json!({ "client_request_id": "inbox-book-0001", "label": "Synthetic Trading" }),
    );
    data(&made);
}

fn spend(crid: &str, date: &str, amount: &str, payee: &str) -> Value {
    json!({
        "client_request_id": crid, "form": "money_out", "date": date, "amount": amount,
        "account": "11000", "category": "67200", "payee": payee, "memo": "MEMO-SYNTH-inbox"
    })
}

fn ticket(result: &Value) -> String {
    data(result)["entry"].as_str().unwrap().to_string()
}

fn row(bot: &mut Bot, entry: &str, filter: &str) -> Option<Value> {
    // the ledger is the native books.ledger app now; the Books window lacks the grant
    let list = bot.agent(
        "books_ledger_list",
        json!({ "month": "2026-10", "filter": filter }),
    );
    data(&list)["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["ticket"] == entry)
        .cloned()
}

fn note(bot: &mut Bot, entry: &str, field: &str) -> Value {
    let got = bot.app("books_entry_get", json!({ "entry": entry }));
    data(&got)["review"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["field"] == field)
        .cloned()
        .unwrap_or(Value::Null)
}

#[test]
fn agent_draft_notes_confidence_and_shows_under_review() {
    let dir = TempDir::new("inbox-confidence");
    let mut bot = Bot::new(&dir);
    setup(&mut bot);
    let mut args = spend("inbox-draft-0001", "2026-10-18", "1350.00", "Synthetic Co");
    args["extracted"] = json!({ "fields": { "category": { "confidence": 55 } } });
    let drafted = bot.agent("books_entry_draft", args);
    let entry = ticket(&drafted);
    assert_eq!(data(&drafted)["likely_duplicates"], json!([]));

    let r = row(&mut bot, &entry, "all").unwrap();
    assert_eq!(r["proposed_by"], "agent");
    assert_eq!(r["low_confidence"], true);
    assert_eq!(r["likely_duplicate"], false);
    assert!(row(&mut bot, &entry, "review").is_some());
    let n = note(&mut bot, &entry, "category");
    assert_eq!(n["confidence"], 55);
    assert_eq!(n["changed_by"], "agent");
    assert_eq!(note(&mut bot, &entry, "payee")["changed_by"], "agent");
    assert_eq!(note(&mut bot, &entry, "to_account"), Value::Null);

    // an app-made entry is nobody's proposal
    let mine = bot.app(
        "books_entry_draft",
        spend("inbox-draft-0002", "2026-10-05", "80.00", "Other Synthetic"),
    );
    let r = row(&mut bot, &ticket(&mine), "all").unwrap();
    assert_eq!(r["proposed_by"], Value::Null);
    assert_eq!(r["low_confidence"], false);
}

#[test]
fn agent_redraft_marks_only_changed_fields_and_app_save_clears_them() {
    let dir = TempDir::new("inbox-redraft");
    let mut bot = Bot::new(&dir);
    setup(&mut bot);
    let made = bot.app(
        "books_entry_draft",
        spend("inbox-redraft-01", "2026-10-18", "1350.00", "Synthetic Co"),
    );
    let entry = ticket(&made);
    let mut again = spend(
        "inbox-redraft-02",
        "2026-10-18",
        "1350.00",
        "Synthetic Supply",
    );
    again["entry"] = json!(entry);
    again["extracted"] =
        json!({ "fields": { "payee": { "confidence": 92 }, "date": { "confidence": 80 } } });
    data(&bot.agent("books_entry_draft", again));
    let payee = note(&mut bot, &entry, "payee");
    assert_eq!(payee["changed_by"], "agent");
    assert_eq!(payee["confidence"], 92);
    // the date was read but not changed: its confidence is kept, unmarked
    let date = note(&mut bot, &entry, "date");
    assert_eq!(date["changed_by"], Value::Null);
    assert_eq!(date["confidence"], 80);
    for field in ["amount", "account", "category", "memo"] {
        assert_eq!(note(&mut bot, &entry, field), Value::Null, "{field}");
    }
    assert_eq!(
        row(&mut bot, &entry, "all").unwrap()["low_confidence"],
        false
    );

    // the user saves the payee in the app: its marks are gone
    let mut fix = spend(
        "inbox-redraft-03",
        "2026-10-18",
        "1350.00",
        "Synthetic Supply",
    );
    fix["entry"] = json!(entry);
    data(&bot.app("books_entry_draft", fix));
    assert_eq!(note(&mut bot, &entry, "payee"), Value::Null);
}

#[test]
fn likely_duplicates_follow_amount_date_and_payee_rules() {
    let dir = TempDir::new("inbox-dupes");
    let mut bot = Bot::new(&dir);
    setup(&mut bot);
    let first = bot.app(
        "books_app_post_entry",
        spend("inbox-dupe-0001", "2026-10-16", "1350.00", "Synthetic Co"),
    );
    let first_entry = ticket(&first);
    let second = bot.agent(
        "books_entry_draft",
        spend("inbox-dupe-0002", "2026-10-18", "1350.00", " synthetic co"),
    );
    let second_entry = ticket(&second);
    assert_eq!(data(&second)["likely_duplicates"], json!(["entry 0001"]));
    assert_eq!(
        row(&mut bot, &second_entry, "all").unwrap()["likely_duplicate"],
        true
    );
    assert!(row(&mut bot, &second_entry, "review").is_some());

    // four days away, or another amount: no match
    let far = bot.agent(
        "books_entry_draft",
        spend("inbox-dupe-0003", "2026-10-12", "1350.00", "Synthetic Co"),
    );
    assert_eq!(data(&far)["likely_duplicates"], json!([]));
    let other = bot.agent(
        "books_entry_draft",
        spend("inbox-dupe-0004", "2026-10-17", "1351.00", "Synthetic Co"),
    );
    assert_eq!(data(&other)["likely_duplicates"], json!([]));
    // a copy is never its source's duplicate
    let copy = bot.app(
        "books_entry_draft",
        json!({ "client_request_id": "inbox-dupe-0005", "copy_of": first_entry, "date": "2026-10-17" }),
    );
    assert!(
        !data(&copy)["likely_duplicates"]
            .to_string()
            .contains("0001"),
        "{copy}"
    );
    data(&bot.app(
        "books_app_discard_entry",
        json!({ "client_request_id": "inbox-dupe-0006", "entry": ticket(&copy) }),
    ));

    // a voided entry and its reversal never match
    data(&bot.app(
        "books_app_void_entry",
        json!({ "client_request_id": "inbox-dupe-0007", "entry": first_entry }),
    ));
    assert_eq!(
        row(&mut bot, &second_entry, "all").unwrap()["likely_duplicate"],
        false
    );

    // a discarded draft never matches
    let a = bot.agent(
        "books_entry_draft",
        spend("inbox-dupe-0008", "2026-10-25", "500.00", "Other Synthetic"),
    );
    let b = bot.app(
        "books_entry_draft",
        spend("inbox-dupe-0009", "2026-10-25", "500.00", "Other Synthetic"),
    );
    let b_entry = ticket(&b);
    assert_eq!(
        row(&mut bot, &b_entry, "all").unwrap()["likely_duplicate"],
        true
    );
    data(&bot.app(
        "books_app_discard_entry",
        json!({ "client_request_id": "inbox-dupe-0010", "entry": ticket(&a) }),
    ));
    assert_eq!(
        row(&mut bot, &b_entry, "all").unwrap()["likely_duplicate"],
        false
    );
}

#[test]
fn post_refuses_a_likely_duplicate_until_keep_both() {
    let dir = TempDir::new("inbox-keep-both");
    let mut bot = Bot::new(&dir);
    setup(&mut bot);
    let first = bot.app(
        "books_app_post_entry",
        spend("inbox-keep-0001", "2026-10-16", "1350.00", "Synthetic Co"),
    );
    let first_entry = ticket(&first);
    let draft = bot.agent(
        "books_entry_draft",
        spend("inbox-keep-0002", "2026-10-18", "1350.00", "Synthetic Co"),
    );
    let entry = ticket(&draft);

    // the assistant can never post or keep both
    let agent = bot.agent(
        "books_app_post_entry",
        json!({ "client_request_id": "inbox-keep-0003", "entry": entry, "keep_both": true }),
    );
    assert_eq!(code(&agent), "permission_denied");

    let refused = bot.app(
        "books_app_post_entry",
        json!({ "client_request_id": "inbox-keep-0004", "entry": entry }),
    );
    assert_eq!(code(&refused), "likely_duplicate");
    let text = refused.to_string();
    assert!(text.contains("0001"), "{text}");
    assert!(
        !text.contains("Synthetic Co") && !text.contains("1350"),
        "{text}"
    );

    let keep = json!({ "client_request_id": "inbox-keep-0005", "entry": entry, "keep_both": true });
    let posted = bot.app("books_app_post_entry", keep.clone());
    assert_eq!(data(&posted)["status"], "posted");
    assert_eq!(bot.app("books_app_post_entry", keep), posted);
    // kept both: neither side is flagged any more
    assert_eq!(
        row(&mut bot, &entry, "all").unwrap()["likely_duplicate"],
        false
    );
    assert_eq!(
        row(&mut bot, &first_entry, "all").unwrap()["likely_duplicate"],
        false
    );

    drop(bot);
    let store = books_store::Store::open(&Paths::in_dir(dir.0.join("store"))).unwrap();
    let acks: i64 = store
        .conn()
        .query_row("SELECT count(*) FROM duplicate_ack", [], |r| r.get(0))
        .unwrap();
    assert_eq!(acks, 1);
}

#[test]
fn extracted_names_known_fields_only() {
    let dir = TempDir::new("inbox-invalid");
    let mut bot = Bot::new(&dir);
    setup(&mut bot);
    for bad in [
        json!({ "fields": { "tin": { "confidence": 90 } } }),
        json!({ "fields": { "payee": { "confidence": 101 } } }),
        json!({ "fields": "payee" }),
    ] {
        let mut args = spend("inbox-bad-00001", "2026-10-18", "10.00", "Synthetic Co");
        args["extracted"] = bad;
        assert_eq!(code(&bot.agent("books_entry_draft", args)), "invalid_args");
    }
}

#[test]
fn the_assistant_sees_no_payee_memo_or_amount_of_duplicates() {
    let dir = TempDir::new("inbox-privacy");
    let mut bot = Bot::new(&dir);
    setup(&mut bot);
    let mut first = spend(
        "inbox-priv-0001",
        "2026-10-16",
        "777.00",
        "Synthetic Candidate",
    );
    first["memo"] = json!("MEMO-SYNTH-candidate");
    first["reference"] = json!("REF-SYNTH-77");
    data(&bot.app("books_app_post_entry", first));
    let mut mine = spend("inbox-priv-0002", "2026-10-17", "777.00", "Synthetic Other");
    mine["reference"] = json!("REF-SYNTH-77");
    let drafted = bot.agent("books_entry_draft", mine);
    let entry = ticket(&drafted);
    let got = bot.agent("books_entry_get", json!({ "entry": entry }));
    let dupes = data(&got)["review"]["likely_duplicates"].clone();
    assert_eq!(dupes.as_array().unwrap().len(), 1, "{got}");
    let keys: Vec<&String> = dupes[0].as_object().unwrap().keys().collect();
    assert_eq!(keys, ["date", "no", "status", "ticket"]);
    for out in [drafted.to_string(), got.to_string()] {
        for secret in [
            "Synthetic Candidate",
            "MEMO-SYNTH-candidate",
            "MEMO-SYNTH-inbox",
        ] {
            assert!(!out.contains(secret), "{secret} in {out}");
        }
    }
    // the app sees the candidate's payee and amount to decide
    let app = bot.app("books_entry_get", json!({ "entry": entry }));
    let d = &data(&app)["review"]["likely_duplicates"][0];
    assert_eq!(d["payee"], "Synthetic Candidate");
    assert!(d["amount_shown"].is_string());
}

#[test]
fn the_window_places_review_badges_and_panel_without_new_tools() {
    let books_mini_app::apps::AppSource::Tsx(source) =
        books_mini_app::apps::find("books").unwrap().source
    else {
        panic!("the Books window is TSX");
    };
    let review = source.find("function ReviewBadges").unwrap();
    let panel = source.find("function ReviewPanel").unwrap();
    let drawer = source.find("function EntryDrawer").unwrap();
    assert!(review < drawer && panel < drawer);
    // the ledger rows are the native books.ledger screen now; its review badges are a follow-up
    assert!(source.contains("<ReviewPanel entry={e()}"));
    assert!(source.contains("keep_both: true"));
    assert!(source.contains(r#"has("copy")"#) && !source.contains(r#"has("duplicate")"#));
}
