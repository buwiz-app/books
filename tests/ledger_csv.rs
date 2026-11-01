//! ledger-4: `books_report_csv` with `kind: "ledger"` exports one row per
//! line in date and entry order, two-place amounts, RFC 4180 with CRLF and
//! a formula guard; the assistant gets no payee, memo, reference or person
//! columns; the report kinds are unchanged. Synthetic data only.

use books_mini_app::demo;
use books_mini_app::service::{Books, Origin};
use books_mini_app::tools;
use books_store::{Paths, TempDir};
use serde_json::{Map, Value, json};

const DEMO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/demo/buwiz-trading-2026.json"
);
const GOLDEN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/golden/report-csv");

fn run(books: &mut Books, name: &str, origin: Origin, args: Value) -> Value {
    let args: Map<String, Value> = args.as_object().unwrap().clone();
    let out = tools::call(name, &args, origin, books).unwrap();
    assert_eq!(out["isError"], false, "{name}: {out}");
    out["structuredContent"].clone()
}

fn demo_books(dir: &TempDir) -> Books {
    books_core::date::pin_today(books_core::date::Date::new(2026, 10, 29));
    let mut books = Books::new(Paths::in_dir(dir.0.join("store")));
    let demo_book: Value = serde_json::from_str(&std::fs::read_to_string(DEMO).unwrap()).unwrap();
    demo::seed(&mut books, &demo_book).unwrap();
    books
}

fn cents(text: &str) -> i64 {
    books_core::money::parse(text).unwrap()
}

#[test]
fn ledger_csv_rows_totals_guard_and_assistant_columns() {
    let dir = TempDir::new("ledger-csv");
    let b = &mut demo_books(&dir);
    run(
        b,
        "books_app_post_entry",
        Origin::App,
        json!({ "client_request_id": "csv-formula-0001", "form": "money_out", "date": "2026-10-03", "amount": "12.34", "account": "11000", "category": "67200", "memo": "=SUM(A1)", "payee": "Synthetic, Co" }),
    );
    let args = json!({ "kind": "ledger", "from": "2026-10-01", "to": "2026-10-31" });
    let out = run(b, "books_report_csv", Origin::App, args.clone());
    let csv = out["csv"].as_str().unwrap();
    assert!(csv.ends_with("\r\n"));
    let lines: Vec<&str> = csv.trim_end_matches("\r\n").split("\r\n").collect();
    assert_eq!(
        lines[0],
        "date,entry,status,form,account,category,person,payee,memo,reference,debit,credit"
    );
    assert_eq!(out["rows"], lines.len());
    assert!(lines.len() > 2);
    assert!(
        csv.contains(",\"'=SUM(A1)\",") || csv.contains(",'=SUM(A1),"),
        "{csv}"
    );
    assert!(csv.contains("\"Synthetic, Co\""), "RFC 4180 quoting");
    // date then entry order; amounts two-place; debits equal credits
    let mut last = (String::new(), String::new());
    let (mut dr, mut cr) = (0i64, 0i64);
    for l in &lines[1..] {
        let cells: Vec<&str> = l.split(',').collect();
        let key = (cells[0].to_string(), cells[1].to_string());
        assert!(key >= last, "{l}");
        last = key;
        assert!(["posted", "draft"].contains(&cells[2]), "{l}");
        let (d, c) = (cells[cells.len() - 2], cells[cells.len() - 1]);
        for amount in [d, c] {
            assert_eq!(amount.split('.').nth(1).map(str::len), Some(2), "{l}");
        }
        dr += cents(d);
        cr += cents(c);
    }
    assert_eq!(dr, cr, "debits equal credits");

    // the assistant's copy has no person, payee, memo or reference
    let agent = run(b, "books_report_csv", Origin::Agent, args);
    let agent_csv = agent["csv"].as_str().unwrap();
    assert!(agent_csv.starts_with("date,entry,status,form,account,category,debit,credit\r\n"));
    assert!(!agent_csv.contains("SUM(A1)") && !agent_csv.contains("Synthetic, Co"));
    assert_eq!(agent["rows"], out["rows"]);

    // a bad date is refused
    let bad: Map<String, Value> = json!({ "kind": "ledger", "from": "2026-13-01" })
        .as_object()
        .unwrap()
        .clone();
    let r = tools::call("books_report_csv", &bad, Origin::App, b).unwrap();
    assert_eq!(r["structuredContent"]["code"], "invalid_args", "{r}");
}

fn fresh_books(dir: &TempDir) -> Books {
    books_core::date::pin_today(books_core::date::Date::new(2026, 10, 29));
    let mut books = Books::new(Paths::in_dir(dir.0.join("store")));
    run(
        &mut books,
        "books_app_book_create",
        Origin::App,
        json!({ "client_request_id": "csv-book-0001", "label": "Synthetic Shop" }),
    );
    books
}

fn spend(b: &mut Books, id: &str, date: &str, memo: &str) -> String {
    let out = run(
        b,
        "books_app_post_entry",
        Origin::App,
        json!({ "client_request_id": id, "form": "money_out", "date": date, "amount": "10.00", "account": "11000", "category": "67200", "memo": memo }),
    );
    out["entry"].as_str().unwrap().to_string()
}

fn statuses(csv: &str) -> Vec<String> {
    let mut s: Vec<String> = csv
        .trim_end_matches("\r\n")
        .split("\r\n")
        .skip(1)
        .map(|l| l.split(',').nth(2).unwrap().to_string())
        .collect();
    s.sort();
    s.dedup();
    s
}

/// Without a filter the export is posted and draft entries only, like the
/// ledger; `filter: "all"` opts in to voided entries, their reversals and
/// discarded drafts.
#[test]
fn ledger_csv_leaves_out_voided_and_discarded_unless_all() {
    let dir = TempDir::new("ledger-csv-status");
    let b = &mut fresh_books(&dir);
    spend(b, "csv-keep-0001", "2026-10-02", "kept");
    let gone = spend(b, "csv-void-0001", "2026-10-03", "voided");
    run(
        b,
        "books_app_void_entry",
        Origin::App,
        json!({ "client_request_id": "csv-void-0002", "entry": gone }),
    );
    let drop = run(
        b,
        "books_entry_draft",
        Origin::App,
        json!({ "client_request_id": "csv-disc-0001", "form": "money_out", "date": "2026-10-04", "amount": "11.00", "account": "11000", "category": "67200" }),
    )["entry"]
        .as_str()
        .unwrap()
        .to_string();
    run(
        b,
        "books_app_discard_entry",
        Origin::App,
        json!({ "client_request_id": "csv-disc-0002", "entry": drop }),
    );
    run(
        b,
        "books_entry_draft",
        Origin::App,
        json!({ "client_request_id": "csv-draft-0001", "form": "money_out", "date": "2026-10-05", "amount": "12.00", "account": "11000", "category": "67200" }),
    );
    let month = json!({ "kind": "ledger", "from": "2026-10-01", "to": "2026-10-31" });
    let plain = run(b, "books_report_csv", Origin::App, month.clone());
    let csv = plain["csv"].as_str().unwrap();
    assert_eq!(statuses(csv), ["draft", "posted"], "{csv}");
    assert!(
        !csv.contains("voided"),
        "no voided original or its reversal: {csv}"
    );
    // 1 header + 2 lines of the kept entry + 2 lines of the draft
    assert_eq!(plain["rows"], 5, "{csv}");
    let mut all = month.clone();
    all["filter"] = json!("all");
    let every = run(b, "books_report_csv", Origin::App, all);
    let csv = every["csv"].as_str().unwrap();
    assert_eq!(
        statuses(csv),
        ["discarded", "draft", "posted", "voided"],
        "{csv}"
    );
    // plus the voided original, its reversal and the discarded draft
    assert_eq!(every["rows"], 11, "{csv}");
    let mut posted = month;
    posted["filter"] = json!("posted");
    let csv = run(b, "books_report_csv", Origin::App, posted)["csv"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(statuses(&csv), ["posted"], "{csv}");
    assert!(!csv.contains("voided"), "{csv}");
}

/// What the host counts against its 1 MiB result limit: every text
/// content plus the structuredContent JSON.
fn host_bytes(result: &Value) -> usize {
    let text: usize = result["content"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| c["text"].as_str())
        .map(str::len)
        .sum();
    text + result["structuredContent"].to_string().len()
}

/// A CSV the host would refuse is refused by Books with `too_large`, and
/// the largest one Books returns still fits the host's limit.
#[test]
fn ledger_csv_fits_the_tool_result_limit() {
    const LIMIT: usize = books_mini_app::ledger_csv::RESULT_MAX;
    let dir = TempDir::new("ledger-csv-size");
    let b = &mut fresh_books(&dir);
    let memo = "synthetic memo ".repeat(32);
    for day in 1..=28u32 {
        for n in 0..20u32 {
            spend(
                b,
                &format!("csv-size-{day:02}-{n:02}"),
                &format!("2026-10-{day:02}"),
                &memo,
            );
        }
    }
    let mut largest = 0usize;
    let mut refused = false;
    for day in 1..=28u32 {
        let args: Map<String, Value> =
            json!({ "kind": "ledger", "from": "2026-10-01", "to": format!("2026-10-{day:02}") })
                .as_object()
                .unwrap()
                .clone();
        let out = tools::call("books_report_csv", &args, Origin::App, b).unwrap();
        if out["isError"] == true {
            assert_eq!(
                out["structuredContent"]["code"], "too_large",
                "{}",
                out["structuredContent"]
            );
            refused = true;
            continue;
        }
        assert!(
            !refused,
            "a longer range fits after a shorter one was refused"
        );
        let bytes = host_bytes(&out);
        assert!(bytes <= LIMIT, "{bytes} bytes for 2026-10-01..{day}");
        largest = largest.max(bytes);
    }
    assert!(refused, "the whole month is over the limit");
    assert!(
        largest > LIMIT - 64 * 1024,
        "largest result {largest} is just under the limit"
    );
}

#[test]
fn ledger_csv_cell_guard() {
    use books_mini_app::ledger_csv::cell;
    assert_eq!(cell("=SUM(A1)"), "'=SUM(A1)");
    assert_eq!(cell("+1"), "'+1");
    assert_eq!(cell("-x"), "'-x");
    assert_eq!(cell("@x"), "'@x");
    assert_eq!(cell("a,b"), "\"a,b\"");
    assert_eq!(cell("say \"hi\""), "\"say \"\"hi\"\"\"");
    assert_eq!(cell("plain"), "plain");
}

/// pl, bs, cf, tb and ag CSV are byte-identical to the goldens recorded
/// from main before ledger-4. `BOOKS_BLESS=1` rewrites them.
#[test]
fn report_kinds_are_unchanged() {
    let dir = TempDir::new("report-csv-golden");
    let b = &mut demo_books(&dir);
    let bless = std::env::var("BOOKS_BLESS").is_ok_and(|v| v == "1");
    for (kind, extra) in [
        ("pl", json!({})),
        ("bs", json!({})),
        ("cf", json!({})),
        ("tb", json!({})),
        ("ag", json!({ "side": "collect" })),
        ("ag", json!({ "side": "pay" })),
    ] {
        let mut args = json!({ "kind": kind, "period_kind": "month", "period": "2026-10" });
        for (k, v) in extra.as_object().unwrap() {
            args[k] = v.clone();
        }
        let csv = run(b, "books_report_csv", Origin::App, args)["csv"]
            .as_str()
            .unwrap()
            .to_string();
        let side = extra["side"]
            .as_str()
            .map_or(String::new(), |s| format!("-{s}"));
        let path = format!("{GOLDEN}/{kind}{side}-2026-10.csv");
        if bless {
            std::fs::create_dir_all(GOLDEN).unwrap();
            std::fs::write(&path, &csv).unwrap();
        }
        assert_eq!(csv, std::fs::read_to_string(&path).unwrap(), "{path}");
    }
}
