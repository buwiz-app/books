//! ledger-2 review findings through the server: one finding per check, tickets
//! and months only, the review routine writes nothing, and Today / the
//! ledger's Checks filter carry them. Synthetic data only.

use books_core::date::{Date, Period, PeriodKind};
use books_mini_app::b64;
use books_mini_app::jsonrpc::Server;
use books_mini_app::manifest::{self, Source};
use books_mini_app::service::{self, Books};
use books_mini_app::{cards, findings};
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

const MEMO: &str = "MEMO-SYNTH-findings";
const PAYEE: &str = "Synthetic Supply Co";

impl Bot {
    fn routine(&mut self, routine: &str, name: &str, args: Value) -> Value {
        self.raw(name, args, json!({ "origin": "agent", "routine": routine, "plugin": manifest::PLUGIN_ID, "ts": now_ms() }))
    }

    /// A posted money_out on a day of the previous month.
    fn spend(
        &mut self,
        n: u32,
        date: Date,
        amount: &str,
        category: &str,
        person: Option<&str>,
    ) -> String {
        let mut args = json!({ "client_request_id": format!("draft-{n:04}"), "form": "money_out", "date": date.to_string(), "amount": amount, "account": "11000", "category": category, "payee": PAYEE, "memo": MEMO });
        if let Some(p) = person {
            args["person"] = json!(p);
        }
        let d = self.app("books_entry_draft", args);
        let entry = data(&d)["entry"].as_str().unwrap().to_string();
        let p = self.app(
            "books_app_post_entry",
            // the duplicate check needs lookalikes posted (ledger-1 refuses them without keep_both)
            json!({ "client_request_id": format!("post-{n:04}"), "entry": entry, "keep_both": true }),
        );
        data(&p);
        entry
    }
}

fn weekday_in(start: Date, want: impl Fn(u8) -> bool, skip: i64) -> Date {
    (skip..28)
        .map(|i| start.add_days(i))
        .find(|d| want(d.weekday()))
        .unwrap()
}

struct Seeded {
    dir: TempDir,
    bot: Bot,
    flagged: Vec<String>,
}

fn seed() -> Seeded {
    let dir = TempDir::new("findings");
    let mut bot = Bot::new(&dir);
    data(&bot.app(
        "books_app_book_create",
        json!({ "client_request_id": "book-0001", "label": "Synthetic Trading" }),
    ));
    let who = data(&bot.app("books_app_person_create", json!({ "client_request_id": "person-0001", "kind": "vendors", "name": "Vendor Synthetic" })))["person"].as_str().unwrap().to_string();
    let prev = Period::of(Date::today(), PeriodKind::Month)
        .previous()
        .start;
    let tue = weekday_in(prev, |w| w == 1, 0);
    let wed = weekday_in(prev, |w| w == 2, 0);
    let sat = weekday_in(prev, |w| w == 5, 0);
    let mut flagged = Vec::new();
    // clean lines: a person, a weekday, a category, amounts that are not round
    bot.spend(1, tue, "150.00", "68300", Some(&who));
    bot.spend(2, wed, "9000.00", "68300", Some(&who));
    bot.spend(3, wed, "10050.00", "68300", Some(&who));
    // one finding each
    flagged.push(bot.spend(4, tue, "10000.00", "68300", Some(&who))); // round_amount
    flagged.push(bot.spend(5, sat, "150.00", "68300", Some(&who))); // weekend_date
    flagged.push(bot.spend(6, wed, "150.00", "68300", None)); // missing_person
    flagged.push(bot.spend(7, tue, "150.00", "69999", Some(&who))); // uncategorized
    // a draft: stale only when 31 days old
    data(&bot.app("books_entry_draft", json!({ "client_request_id": "draft-0099", "form": "money_out", "date": tue.to_string(), "amount": "150.00", "account": "11000", "category": "68300", "person": who })));
    Seeded { dir, bot, flagged }
}

fn direct(dir: &TempDir, today: Date) -> Vec<findings::Finding> {
    let mut books = Books::new(Paths::in_dir(dir.0.join("store")));
    let store = books.store().unwrap();
    let book = service::book(store, &serde_json::Map::new()).unwrap();
    findings::findings(store, book, today).unwrap()
}

#[test]
fn one_finding_per_check_and_stale_after_thirty_days() {
    let s = seed();
    let today = Date::today();
    let late = direct(&s.dir, today.add_days(31));
    let counts = findings::counts(&late);
    for k in findings::CHECKS {
        assert_eq!(counts[k], 1, "{k}: {late:?}");
    }
    assert_eq!(late.len(), 6);
    let early = findings::counts(&direct(&s.dir, today.add_days(29)));
    assert_eq!(early["stale_draft"], 0);
    // ordered by check, in vocabulary order
    let order: Vec<&str> = late.iter().map(|f| f.check).collect();
    assert_eq!(order, findings::CHECKS.to_vec());
}

#[test]
fn a_book_without_people_has_no_missing_person() {
    let dir = TempDir::new("findings-nobody");
    let mut bot = Bot::new(&dir);
    data(&bot.app(
        "books_app_book_create",
        json!({ "client_request_id": "book-0001", "label": "Synthetic Solo" }),
    ));
    let prev = Period::of(Date::today(), PeriodKind::Month)
        .previous()
        .start;
    let wed = weekday_in(prev, |w| w == 2, 0);
    bot.spend(1, wed, "150.00", "68300", None);
    let found = direct(&dir, Date::today());
    assert_eq!(findings::counts(&found)["missing_person"], 0, "{found:?}");
    let out = bot.agent("books_checks", json!({}));
    let n = data(&out)["counts"]["missing_person"].as_i64().unwrap_or(0);
    assert_eq!(n, 0, "{out}");
}

#[test]
fn checks_show_tickets_and_months_only() {
    let mut s = seed();
    let money = regex_free_two_decimals;
    // the window holds no grant for books_checks (slot budget): the app is
    // refused, and the refusal carries nothing either
    let app = s.bot.app("books_checks", json!({}));
    assert_eq!(code(&app), "permission_denied");
    assert!(!app.to_string().contains(PAYEE) && !money(&app.to_string()));
    {
        let out = s.bot.agent("books_checks", json!({}));
        let d = data(&out).clone();
        let text = d.to_string();
        assert!(
            !text.contains(PAYEE) && !text.contains(MEMO) && !text.contains("Vendor Synthetic"),
            "{text}"
        );
        assert!(!money(&text), "{text}");
        assert_eq!(d["counts"]["round_amount"], 1);
        assert_eq!(d["counts"]["unmatched_month"], 1);
        assert_eq!(d["counts"]["stale_draft"], 0);
        let rows = d["findings"].as_array().unwrap();
        assert_eq!(rows.len(), 5);
        assert!(
            rows.iter()
                .all(|r| r["entry"].as_str().is_none_or(|e| e.starts_with("entry ")))
        );
    }
    let one = s
        .bot
        .agent("books_checks", json!({ "check": "weekend_date" }));
    assert_eq!(data(&one)["findings"].as_array().unwrap().len(), 1);
    assert_eq!(
        code(&s.bot.agent("books_checks", json!({ "check": "tax" }))),
        "invalid_args"
    );
}

/// True when the text holds a two-decimal amount (\d+\.\d\d).
fn regex_free_two_decimals(text: &str) -> bool {
    let b = text.as_bytes();
    (1..b.len().saturating_sub(2)).any(|i| {
        b[i] == b'.'
            && b[i - 1].is_ascii_digit()
            && b[i + 1].is_ascii_digit()
            && b[i + 2].is_ascii_digit()
    })
}

#[test]
fn review_routine_writes_nothing() {
    let mut s = seed();
    assert!(cards::routine_writes("review").is_empty());
    let r = s.bot.routine("review", "books_entry_draft", json!({ "client_request_id": "routine-0001", "form": "money_out", "date": "2026-10-21", "amount": "150.00", "account": "11000", "category": "68300" }));
    assert!(
        r.to_string()
            .contains("is not a write this routine declares"),
        "{r}"
    );
    let n = s.bot.routine(
        "review",
        "books_nudge",
        json!({ "kind": "review", "any_day": true }),
    );
    let d = data(&n);
    assert_eq!(d["count"], 5);
    assert_eq!(d["open"]["filter"], "checks");
    assert!(!regex_free_two_decimals(&d.to_string()));
    let routines = cards::routines();
    assert_eq!(routines.as_array().unwrap().len(), 4);
}

#[test]
fn today_and_ledger_show_checks() {
    let mut s = seed();
    // Today counts the flagged entries of this month, the ledger it opens
    let this_month = Period::of(Date::today(), PeriodKind::Month);
    let t = s.bot.app("books_today", json!({}));
    let needs = data(&t)["needs"].as_array().unwrap().clone();
    assert!(needs.iter().all(|n| n["kind"] != "checks"), "{needs:?}");
    let now = s.bot.spend(8, this_month.start, "20000.00", "68300", None);
    let t = s.bot.app("books_today", json!({}));
    let needs = data(&t)["needs"].as_array().unwrap().clone();
    let item = needs.iter().find(|n| n["kind"] == "checks").unwrap();
    assert_eq!(item["count"], 1, "one entry, however many findings it has");
    assert_eq!(item["filter"], "checks");
    let l = s
        .bot
        .agent("books_ledger_list", json!({ "filter": "checks" }));
    let d = data(&l);
    assert_eq!(d["counts"]["checks"], 1);
    assert_eq!(d["rows"][0]["ticket"].as_str(), Some(now.as_str()));
    let month = Period::of(Date::today(), PeriodKind::Month).previous().key;
    let l = s.bot.agent(
        "books_ledger_list",
        json!({ "month": month, "filter": "checks" }),
    );
    let d = data(&l);
    let mut got: Vec<String> = d["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["ticket"].as_str().unwrap_or_default().to_string())
        .collect();
    got.sort();
    let mut want = s.flagged.clone();
    want.sort();
    assert_eq!(got, want, "{d}");
    assert_eq!(d["counts"]["checks"], 4);
}

#[test]
fn a_finished_statement_clears_unmatched_month() {
    let mut s = seed();
    let before = s.bot.agent("books_checks", json!({}));
    assert_eq!(data(&before)["counts"]["unmatched_month"], 1);
    // one statement for the one bank account the seed posts to, every
    // posted line on it, then finished
    let prev = Period::of(Date::today(), PeriodKind::Month).previous();
    let entries = s.bot.agent(
        "books_ledger_list",
        json!({ "month": prev.key, "filter": "posted" }),
    );
    let rows = data(&entries)["rows"].as_array().unwrap().clone();
    assert_eq!(rows.len(), 7);
    let mut csv = String::from("Date,Description,Debit,Credit,Balance\n");
    let mut total: i64 = 0;
    for (i, r) in rows.iter().enumerate() {
        let e = data(
            &s.bot
                .app("books_entry_get", json!({ "entry": r["ticket"] })),
        )
        .clone();
        let amount = e["amount"]
            .as_str()
            .unwrap()
            .trim_start_matches('-')
            .to_string();
        let cents: i64 = amount.replace(['.', ','], "").parse().unwrap();
        total += cents;
        csv.push_str(&format!(
            "{},SYNTH LINE {i},\"{amount}\",,\n",
            e["date"].as_str().unwrap()
        ));
    }
    let closing = format!("-{}.{:02}", total / 100, total % 100);
    let import = s.bot.app(
        "books_app_statement_import",
        json!({ "client_request_id": "stmt-import-01", "account": "11000", "csv_base64": b64::encode(csv.as_bytes()), "file_name": "synthetic.csv", "closing": closing }),
    );
    data(&import);
    let accept = s.bot.app(
        "books_app_statement_accept",
        json!({ "client_request_id": "stmt-accept-01", "account": "11000", "month": prev.key }),
    );
    data(&accept);
    let done = s.bot.app(
        "books_app_statement_finish",
        json!({ "client_request_id": "stmt-finish-01", "account": "11000", "month": prev.key }),
    );
    data(&done);
    let after = s.bot.agent("books_checks", json!({}));
    let d = data(&after);
    assert_eq!(d["counts"]["unmatched_month"], 0, "{d}");
    assert_eq!(d["counts"]["round_amount"], 1);
}

#[test]
fn checks_stays_off_the_window() {
    let m = manifest::unsigned(manifest::RELEASE_KEY_ID);
    assert_eq!(m["routines"].as_array().unwrap().len(), 4);
    let books = m["apps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == "books")
        .unwrap();
    assert!(
        !books["permissions"]["tools"]
            .to_string()
            .contains("books_checks")
    );
    assert!(
        m["tools"].to_string().contains("books_checks") || m.to_string().contains("books_checks")
    );
}
