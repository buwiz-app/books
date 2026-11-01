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
