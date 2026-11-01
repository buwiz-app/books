//! Amend by reversal through `books_app_void_entry {amend: true}`: the
//! original is voided, a mirrored reversal posts, a replacement draft with
//! the same lines is linked both ways; closed months, drafts, reversals and
//! the assistant are refused; the reason never reaches a tool output.
//! Synthetic data only.

use books_mini_app::b64;
use books_mini_app::jsonrpc::Server;
use books_mini_app::manifest::{self, Source};
use books_mini_app::service::Books;
use books_store::{Paths, Store, TempDir};
use ed25519_dalek::SigningKey;
use serde_json::{Value, json};

const MEMO: &str = "MEMO-SYNTH-office-supplies";

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

const REASON: &str = "REASON-SYNTH-wrong-category";

fn sale(bot: &mut Bot, crid: &str, date: &str) -> String {
    let r = bot.app("books_app_post_entry", json!({
        "client_request_id": crid, "form": "money_in", "date": date, "amount": "800.00",
        "account": "11000", "category": "41000", "payee": "Bayani Foods", "reference": "Invoice 0142", "memo": MEMO,
        "facts": { "document_kind": "sale_invoice" }
    }));
    data(&r)["entry"].as_str().unwrap().to_string()
}

fn booked(tag: &str) -> (TempDir, Bot) {
    let dir = TempDir::new(tag);
    let mut bot = Bot::new(&dir);
    let made = bot.app(
        "books_app_book_create",
        json!({ "client_request_id": "create-book-0001", "label": "Buwiz Trading" }),
    );
    data(&made);
    (dir, bot)
}

fn balances(bot: &mut Bot) -> Vec<(String, String)> {
    let a = bot.agent("books_accounts_list", json!({}));
    data(&a)["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| (r["number"].to_string(), r["balance"].to_string()))
        .collect()
}

fn store(dir: &TempDir) -> Store {
    Store::open(&Paths::in_dir(dir.0.join("store"))).unwrap()
}

fn rows(dir: &TempDir) -> Vec<String> {
    let s = store(dir);
    let mut st = s.conn().prepare("SELECT e.entry_id || '|' || e.status || '|' || e.entry_date || '|' || ifnull(e.voided_by_entry_id, '') || '|' || ifnull(e.amended_by_entry_id, '') || '|' || l.account_id || '|' || l.debit || '|' || l.credit FROM entry e JOIN line l ON l.entry_id = e.entry_id ORDER BY e.entry_id, l.line_id").unwrap();
    st.query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn count(dir: &TempDir, table: &str) -> i64 {
    store(dir)
        .conn()
        .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
}

fn amend(bot: &mut Bot, crid: &str, entry: &str, extra: Value) -> Value {
    let mut args =
        json!({ "client_request_id": crid, "entry": entry, "amend": true, "reason": REASON });
    for (k, v) in extra.as_object().unwrap() {
        args[k] = v.clone();
    }
    bot.app("books_app_void_entry", args)
}

#[test]
fn an_amend_voids_reverses_and_links_a_replacement_draft() {
    let (dir, mut bot) = booked("amend-ok");
    let before = balances(&mut bot);
    let original = sale(&mut bot, "post-sale-0142", "2026-10-28");
    assert_ne!(balances(&mut bot), before);
    let r = amend(
        &mut bot,
        "amend-sale-0142",
        &original,
        json!({ "date": "2026-10-30" }),
    );
    let out = data(&r).clone();
    assert_eq!(out["status"], "voided");
    let (voided, reversal, replacement) = (
        out["voided"].as_str().unwrap(),
        out["reversal"].as_str().unwrap(),
        out["replacement"].as_str().unwrap(),
    );
    assert_eq!(voided, original);
    // a void and its reversal net to zero; the replacement is a draft
    assert_eq!(balances(&mut bot), before);

    let o = bot.agent("books_entry_get", json!({ "entry": original }));
    assert_eq!(data(&o)["status"], "voided");
    assert_eq!(data(&o)["links"]["amended_by"], replacement);
    assert!(data(&o)["links"]["amends"].is_null());
    assert_eq!(data(&o)["actions"], json!([]));
    let d = bot.agent("books_entry_get", json!({ "entry": replacement }));
    assert_eq!(data(&d)["status"], "draft");
    assert_eq!(data(&d)["links"]["amends"], original);
    let replaces = data(&d)["related"].as_str().unwrap();
    assert!(replaces.starts_with("Replaces entry "), "{replaces}");
    let p = bot.agent("books_entry_get", json!({ "entry": reversal }));
    assert_eq!(data(&p)["status"], "posted");

    let s = store(&dir);
    let c = s.conn();
    let id = |t: &str| -> i64 {
        c.query_row(
            "SELECT subject_id FROM ticket WHERE ticket = ?1",
            [t],
            |r| r.get(0),
        )
        .unwrap()
    };
    let (oid, rid, did) = (id(voided), id(reversal), id(replacement));
    let link: (Option<i64>, Option<i64>, Option<i64>) = c.query_row("SELECT voided_by_entry_id, amended_by_entry_id, amends_entry_id FROM entry WHERE entry_id = ?1", [oid], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap();
    assert_eq!(link, (Some(rid), Some(did), None));
    let rev: (Option<i64>, String) = c
        .query_row(
            "SELECT reverses_entry_id, status FROM entry WHERE entry_id = ?1",
            [rid],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(rev, (Some(oid), "posted".into()));
    let rep: (Option<i64>, String, String, String) = c
        .query_row(
            "SELECT amends_entry_id, status, origin, entry_date FROM entry WHERE entry_id = ?1",
            [did],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap();
    assert_eq!(
        rep,
        (Some(oid), "draft".into(), "app".into(), "2026-10-30".into())
    );
    let lines = |e: i64| -> Vec<(i64, i64, i64)> {
        let mut st = c
            .prepare(
                "SELECT account_id, debit, credit FROM line WHERE entry_id = ?1 ORDER BY line_id",
            )
            .unwrap();
        st.query_map([e], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    };
    assert_eq!(lines(did), lines(oid));
    let mirrored: Vec<_> = lines(oid).into_iter().map(|(a, d, k)| (a, k, d)).collect();
    let mut got = lines(rid);
    got.sort_unstable();
    let mut want = mirrored;
    want.sort_unstable();
    assert_eq!(got, want);
    let stored: (i64, i64, String) = c.query_row("SELECT reversal_entry_id, replacement_entry_id, reason FROM amendment WHERE original_entry_id = ?1", [oid], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap();
    assert_eq!(stored, (rid, did, REASON.to_string()));
}

#[test]
fn an_amend_is_refused_for_drafts_reversals_repeats_short_reasons_and_closed_months() {
    let (dir, mut bot) = booked("amend-refuse");
    let original = sale(&mut bot, "post-sale-0142", "2026-10-28");
    let draft = bot.app("books_entry_draft", json!({
        "client_request_id": "draft-0143", "form": "money_out", "date": "2026-10-29", "amount": "120.00",
        "account": "11000", "category": "67200"
    }));
    let draft = data(&draft)["entry"].as_str().unwrap().to_string();
    assert_eq!(
        code(&amend(&mut bot, "amend-draft", &draft, json!({}))),
        "not_posted"
    );
    for reason in [
        json!(null),
        json!("ab"),
        json!("   "),
        json!("x".repeat(201)),
    ] {
        let mut args = json!({ "client_request_id": format!("amend-short-{}", reason.to_string().len()), "entry": original, "amend": true });
        if !reason.is_null() {
            args["reason"] = reason;
        }
        assert_eq!(code(&bot.app("books_app_void_entry", args)), "invalid_args");
    }
    let tin = bot.app("books_app_void_entry", json!({ "client_request_id": "amend-tin", "entry": original, "amend": true, "reason": "see 123-456-789-000" }));
    assert!(tin["isError"] == true, "{tin}");
    let r = amend(&mut bot, "amend-sale-0142", &original, json!({}));
    let reversal = data(&r)["reversal"].as_str().unwrap().to_string();
    assert_eq!(
        code(&amend(&mut bot, "amend-again", &original, json!({}))),
        "already_amended"
    );
    assert_eq!(
        code(&bot.app(
            "books_app_void_entry",
            json!({ "client_request_id": "void-again", "entry": original })
        )),
        "already_voided"
    );
    assert_eq!(
        code(&amend(&mut bot, "amend-reversal", &reversal, json!({}))),
        "is_reversal"
    );

    // closed months: the original's month, then the amend date's month
    let september = sale(&mut bot, "post-sale-0901", "2026-09-15");
    let october = sale(&mut bot, "post-sale-1001", "2026-10-01");
    let close = bot.app(
        "books_app_month_close",
        json!({ "client_request_id": "close-2026-09", "month": "2026-09" }),
    );
    data(&close);
    let before = (rows(&dir), count(&dir, "amendment"));
    let a = amend(
        &mut bot,
        "amend-closed-original",
        &september,
        json!({ "date": "2026-10-02" }),
    );
    assert_eq!(code(&a), "period_closed");
    let b = amend(
        &mut bot,
        "amend-closed-date",
        &october,
        json!({ "date": "2026-09-30" }),
    );
    assert!(b["isError"] == true, "{b}");
    assert_eq!(
        (rows(&dir), count(&dir, "amendment")),
        before,
        "closed-month entries stay byte-identical"
    );
}

#[test]
fn only_the_app_amends_and_a_repeat_returns_the_same_tickets() {
    let (dir, mut bot) = booked("amend-origin");
    let original = sale(&mut bot, "post-sale-0142", "2026-10-28");
    let args = json!({ "client_request_id": "amend-sale-0142", "entry": original, "amend": true, "reason": REASON });
    let refused = bot.agent("books_app_void_entry", args.clone());
    assert_eq!(code(&refused), "permission_denied");
    assert!(
        refused.to_string().contains("needs the user's click"),
        "{refused}"
    );
    assert!(!refused.to_string().contains(REASON));

    let first = bot.app("books_app_void_entry", args.clone());
    let entries = count(&dir, "entry");
    let again = bot.app("books_app_void_entry", args);
    for k in ["voided", "reversal", "replacement"] {
        assert_eq!(data(&again)[k], data(&first)[k]);
    }
    assert_eq!(count(&dir, "entry"), entries);
    assert_eq!(count(&dir, "amendment"), 1);

    // the reason is in no tool output and not in the audit rows
    let replacement = data(&first)["replacement"].as_str().unwrap().to_string();
    for out in [
        first.clone(),
        bot.agent("books_entry_get", json!({ "entry": original })),
        bot.agent("books_entry_get", json!({ "entry": replacement })),
        bot.agent("books_ledger_list", json!({})),
        bot.agent("books_today", json!({})),
    ] {
        assert!(!out.to_string().contains(REASON), "{out}");
    }
    let s = store(&dir);
    let mut st = s.conn().prepare("SELECT * FROM audit_event").unwrap();
    let n = st.column_count();
    let audit: Vec<String> = st
        .query_map([], |r| {
            Ok((0..n)
                .map(|i| format!("{:?}", r.get_ref(i).unwrap()))
                .collect::<Vec<_>>()
                .join("|"))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert!(!audit.is_empty());
    assert!(audit.iter().all(|row| !row.contains("REASON-SYNTH")));
}

#[test]
fn a_plain_void_is_unchanged_and_the_drawer_offers_amend() {
    let (_dir, mut bot) = booked("amend-plain");
    let original = sale(&mut bot, "post-sale-0142", "2026-10-28");
    let o = bot.agent("books_entry_get", json!({ "entry": original }));
    assert_eq!(data(&o)["actions"], json!(["void", "amend", "copy"]));
    assert_eq!(
        data(&o)["links"],
        json!({ "amends": null, "amended_by": null })
    );
    let v = bot.app(
        "books_app_void_entry",
        json!({ "client_request_id": "void-sale-0142", "entry": original }),
    );
    assert_eq!(data(&v)["status"], "voided");
    assert!(data(&v).get("replacement").is_none());
    let o = bot.agent("books_entry_get", json!({ "entry": original }));
    assert_eq!(
        data(&o)["links"],
        json!({ "amends": null, "amended_by": null })
    );
}
