//! `books_app_report_xlsx`: the P&L, balance sheet and cash flow as a
//! workbook, read back with office_oxide and checked against the report's own
//! data and the CSV export. Then who may call it, and the `buwiz/save` it
//! answers with. Synthetic data only.

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::atomic::{AtomicU64, Ordering};

use books_mini_app::jsonrpc::Server;
use books_mini_app::manifest::{self, Source};
use books_mini_app::report_xlsx::{self, Cell, MEDIA_TYPE, MONEY_FORMAT, Row, Table};
use books_mini_app::service::{Books, Origin};
use books_mini_app::{b64, demo, tools};
use books_store::{Paths, TempDir};
use ed25519_dalek::SigningKey;
use office_oxide::xlsx::{CellValue, XlsxDocument};
use serde_json::{Map, Value, json};

const XLSX: &str = "books_app_report_xlsx";

/// A client_request_id no other call in this run has used.
fn fresh_id() -> String {
    static N: AtomicU64 = AtomicU64::new(0);
    format!("xlsx-test-{:04}", N.fetch_add(1, Ordering::Relaxed))
}

/// The call's arguments; an export without an id gets a fresh one.
fn with_id(name: &str, mut args: Value) -> Value {
    if name == XLSX && args.get("client_request_id").is_none() {
        args["client_request_id"] = json!(fresh_id());
    }
    args
}

fn raw(books: &mut Books, name: &str, args: Value, origin: Origin) -> Value {
    let args: Map<String, Value> = with_id(name, args).as_object().unwrap().clone();
    tools::call(name, &args, origin, books).unwrap()
}

fn call(books: &mut Books, name: &str, args: Value) -> Value {
    let out = raw(books, name, args, Origin::App);
    assert_eq!(out["isError"], false, "{name}: {out}");
    out
}

fn golden() -> (TempDir, Books) {
    books_core::date::pin_today(books_core::date::Date::new(2026, 10, 29));
    let dir = TempDir::new("xlsx-golden");
    let mut books = Books::new(Paths::in_dir(dir.0.join("store")));
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/demo/buwiz-trading-2026.json"
    ))
    .unwrap();
    let book: Value = serde_json::from_str(&text).unwrap();
    demo::seed(&mut books, &book).unwrap();
    // a host that honours buwiz/save
    books.host_saves = true;
    (dir, books)
}

/// What an export handed over: the answer's data, the file's bytes.
fn exported(books: &mut Books, args: Value) -> (Value, Vec<u8>) {
    let out = call(books, XLSX, args);
    let save = &out["_meta"]["buwiz/save"];
    let path = save["path"].as_str().expect("buwiz/save carries a path");
    let bytes = std::fs::read(path).unwrap();
    (out, bytes)
}

/// A cell read back: its value, its number format and whether its font is bold.
type Read = Option<(CellValue, Option<String>, bool)>;

/// A workbook read back: its sheet names and the cells of the first sheet,
/// `[row][col]`, with the number format each has.
struct Back {
    sheets: Vec<String>,
    cells: Vec<Vec<Read>>,
}

impl Back {
    fn text(&self, r: usize, c: usize) -> String {
        match self.cells.get(r).and_then(|row| row.get(c)) {
            Some(Some((CellValue::String(s), ..))) => s.clone(),
            Some(Some((CellValue::SharedString(_), ..))) => panic!("unresolved string"),
            Some(Some((CellValue::Number(n), ..))) => panic!("a number where text is: {n}"),
            _ => String::new(),
        }
    }

    /// A number cell: its value, its format and whether its font is bold.
    fn number(&self, r: usize, c: usize) -> Option<(f64, String, bool)> {
        match self.cells.get(r).and_then(|row| row.get(c)) {
            Some(Some((CellValue::Number(n), fmt, bold))) => {
                Some((*n, fmt.clone().unwrap_or_default(), *bold))
            }
            Some(Some((CellValue::String(s), ..))) => panic!("text where a number is: {s}"),
            _ => None,
        }
    }

    fn is_blank(&self, r: usize, c: usize) -> bool {
        !matches!(self.cells.get(r).and_then(|row| row.get(c)), Some(Some(_)))
    }

    fn bold(&self, r: usize, c: usize) -> bool {
        matches!(
            self.cells.get(r).and_then(|row| row.get(c)),
            Some(Some((_, _, true)))
        )
    }

    /// The row whose first cell says `Section`.
    fn header(&self) -> usize {
        (0..self.cells.len())
            .find(|r| self.text(*r, 0) == "Section")
            .expect("a header row")
    }

    fn width(&self, r: usize) -> usize {
        self.cells[r]
            .iter()
            .rposition(Option::is_some)
            .map_or(0, |i| i + 1)
    }
}

fn read_back(bytes: &[u8]) -> Back {
    let mut doc = XlsxDocument::from_reader(Cursor::new(bytes)).expect("office_oxide reads it");
    doc.ensure_styles();
    let styles = doc.styles.clone().expect("styles");
    let sheets = doc.worksheets.iter().map(|w| w.name.clone()).collect();
    let mut cells: Vec<Vec<Read>> = Vec::new();
    for row in &doc.worksheets[0].rows {
        for c in &row.cells {
            let (r, col) = (c.reference.row as usize, c.reference.col as usize);
            if cells.len() <= r {
                cells.resize_with(r + 1, Vec::new);
            }
            if cells[r].len() <= col {
                cells[r].resize_with(col + 1, || None);
            }
            let value = match &c.value {
                CellValue::SharedString(i) => {
                    CellValue::String(doc.shared_strings.get(*i).unwrap_or_default().to_string())
                }
                v => v.clone(),
            };
            let fmt = c
                .style_index
                .and_then(|s| styles.number_format_for(s))
                .map(str::to_string);
            let bold = c
                .style_index
                .and_then(|s| styles.font_for(s))
                .is_some_and(|f| f.bold);
            cells[r][col] = Some((value, fmt, bold));
        }
    }
    Back { sheets, cells }
}

/// CSV text as rows of fields (the export's quoting: a quoted field with
/// doubled quotes).
fn parse_csv(text: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let (mut row, mut field, mut quoted) = (Vec::new(), String::new(), false);
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, quoted) {
            ('"', true) if chars.peek() == Some(&'"') => {
                chars.next();
                field.push('"');
            }
            ('"', true) => quoted = false,
            ('"', false) if field.is_empty() => quoted = true,
            (',', false) => row.push(std::mem::take(&mut field)),
            ('\n', false) => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            (c, _) => field.push(c),
        }
    }
    assert!(
        field.is_empty() && row.is_empty(),
        "the CSV ends with a newline"
    );
    rows
}

/// The workbook's rows from its header down, as text and amounts, against
/// the CSV's: every CSV row is there in order with the same label and the
/// same amount to the centavo. The rows the workbook adds are the totals.
/// Returns those added rows' labels.
fn matches_csv(back: &Back, csv: &[Vec<String>]) -> Vec<String> {
    let head = back.header();
    assert_eq!(csv[0].len(), back.width(head), "the same columns");
    for (c, h) in csv[0].iter().enumerate() {
        assert_eq!(&back.text(head, c), h, "header {c}");
        assert!(back.bold(head, c), "the header row is bold");
    }
    let mut at = head + 1;
    let mut added = Vec::new();
    for line in &csv[1..] {
        // a row the CSV lacks: a section total, before the row the CSV has next
        while at < back.cells.len()
            && back.text(at, 0).is_empty()
            && back.text(at, 1).is_empty()
            && back.text(at, 2) != line[2]
        {
            added.push(back.text(at, 2));
            at += 1;
        }
        for (c, want) in line.iter().enumerate() {
            if c < 3 {
                assert_eq!(&back.text(at, c), want, "row {at} col {c}");
            } else if want.is_empty() {
                assert!(back.is_blank(at, c), "row {at} col {c} is blank");
            } else {
                let (n, fmt, _) = back
                    .number(at, c)
                    .unwrap_or_else(|| panic!("row {at} col {c} is a number"));
                assert_eq!(fmt, MONEY_FORMAT);
                assert_eq!(&format!("{n:.2}"), want, "row {at} col {c} to the centavo");
            }
        }
        at += 1;
    }
    assert!(
        at >= back.cells.len(),
        "nothing but the CSV's rows and totals"
    );
    added
}

fn expect_total(back: &Back, label: &str, shown: &[&str]) {
    let r = (0..back.cells.len())
        .find(|r| back.text(*r, 2) == label)
        .unwrap_or_else(|| panic!("no row {label}"));
    assert!(back.bold(r, 2), "{label} is bold");
    for (i, want) in shown.iter().enumerate() {
        let (n, fmt, bold) = back.number(r, 3 + i).unwrap();
        assert_eq!(&format!("{n:.2}"), want, "{label} column {i}");
        assert_eq!(fmt, MONEY_FORMAT);
        assert!(bold);
    }
}

fn plain(v: &Value) -> String {
    v.as_str()
        .unwrap()
        .replace(',', "")
        .replace('\u{2212}', "-")
        .replace('+', "")
}

#[test]
fn each_report_reads_back_with_the_csvs_rows_and_the_reports_totals() {
    let (_dir, mut b) = golden();
    let cases = [
        (
            "pl",
            "Profit and loss",
            json!({ "kind": "pl", "period": "2026-10" }),
        ),
        (
            "bs",
            "Balance sheet",
            json!({ "kind": "bs", "period": "2026-10" }),
        ),
        (
            "cf",
            "Cash flow",
            json!({ "kind": "cf", "period": "2026-10" }),
        ),
        (
            "pl",
            "Profit and loss",
            json!({ "kind": "pl", "period": "2026-Q3", "period_kind": "quarter", "compare_to": "last_year" }),
        ),
        (
            "bs",
            "Balance sheet",
            json!({ "kind": "bs", "period": "2026-10", "compare_to": "previous" }),
        ),
        (
            "cf",
            "Cash flow",
            json!({ "kind": "cf", "period": "2026-10", "compare_to": "previous" }),
        ),
    ];
    for (kind, title, args) in cases {
        let report = call(&mut b, "books_report", args.clone())["structuredContent"].clone();
        let csv = call(&mut b, "books_report_csv", args.clone())["structuredContent"].clone();
        let (out, bytes) = exported(&mut b, args.clone());
        let back = read_back(&bytes);

        // one sheet, named for the report; a title and the period above the table
        assert_eq!(back.sheets, [title], "{kind}");
        assert_eq!(back.text(0, 0), title);
        assert_eq!(
            back.text(1, 0),
            format!("Period: {}", report["period"]["label"].as_str().unwrap())
        );
        assert_eq!(back.text(2, 0), report["basis_label"].as_str().unwrap());

        // the CSV's rows and columns, amounts to the centavo
        let csv_name = csv["file_name"].as_str().unwrap();
        assert_eq!(
            out["structuredContent"]["file_name"],
            csv_name.replace(".csv", ".xlsx")
        );
        let added = matches_csv(&back, &parse_csv(csv["csv"].as_str().unwrap()));

        // the totals are the report's own, as values
        let two = back.width(back.header()) == 5;
        let col = |v: &Value| -> Vec<String> {
            let mut out = vec![plain(&v["shown"])];
            if two {
                out.push(plain(&v["prev_shown"]));
            }
            out
        };
        let net = &report["net"];
        let want = col(net);
        expect_total(
            &back,
            net["label"].as_str().unwrap(),
            &want.iter().map(String::as_str).collect::<Vec<_>>(),
        );
        let mut expected_added = Vec::new();
        for t in report["totals"].as_array().into_iter().flatten() {
            expected_added.push(t["label"].as_str().unwrap().to_string());
            let w = col(t);
            expect_total(
                &back,
                t["label"].as_str().unwrap(),
                &w.iter().map(String::as_str).collect::<Vec<_>>(),
            );
        }
        for s in report["sections"].as_array().unwrap() {
            if s["total"].is_object() {
                let t = &s["total"];
                expected_added.push(t["label"].as_str().unwrap().to_string());
                let w = col(t);
                expect_total(
                    &back,
                    t["label"].as_str().unwrap(),
                    &w.iter().map(String::as_str).collect::<Vec<_>>(),
                );
            }
        }
        assert_eq!(
            added, expected_added,
            "{kind}: the rows beside the CSV's are the report's totals"
        );
    }
}

#[test]
fn the_pl_of_the_golden_book_has_the_known_net() {
    let (_dir, mut b) = golden();
    let (_, bytes) = exported(&mut b, json!({ "kind": "pl", "period": "2026-10" }));
    let back = read_back(&bytes);
    expect_total(&back, "Net income", &["251151.00", "-82199.00"]);
    let head = back.header();
    assert_eq!(back.text(head, 3), "October");
    assert_eq!(back.text(head, 4), "September");
}

/// A small book of its own: a negative expense row, a row that is zero this
/// month, and a large amount.
fn edge_book() -> (TempDir, Books) {
    books_core::date::pin_today(books_core::date::Date::new(2026, 10, 29));
    let dir = TempDir::new("xlsx-edge");
    let mut b = Books::new(Paths::in_dir(dir.0.join("store")));
    b.host_saves = true;
    call(
        &mut b,
        "books_app_book_create",
        json!({ "client_request_id": "edge-book-0001", "label": "Edge Book" }),
    );
    let post = |b: &mut Books, id: &str, date: &str, debit: &str, credit: &str, amount: &str| {
        call(
            b,
            "books_app_post_entry",
            json!({
                "client_request_id": id, "form": "journal", "date": date,
                "lines": [ { "account": debit, "debit": amount }, { "account": credit, "credit": amount } ],
                "memo": "Synthetic"
            }),
        );
    };
    // September: two expenses; October: a refund on one (a negative row), the other untouched
    post(
        &mut b,
        "edge-0001",
        "2026-09-05",
        "67200",
        "11000",
        "100.00",
    );
    post(&mut b, "edge-0002", "2026-09-06", "61100", "11000", "25.00");
    post(
        &mut b,
        "edge-0003",
        "2026-10-07",
        "11000",
        "67200",
        "140.00",
    );
    // October: a large sale
    post(
        &mut b,
        "edge-0004",
        "2026-10-08",
        "11000",
        "41000",
        "9999999999.99",
    );
    (dir, b)
}

#[test]
fn a_negative_a_zero_and_a_large_amount_survive() {
    let (_dir, mut b) = edge_book();
    let args = json!({ "kind": "pl", "period": "2026-10" });
    let report = call(&mut b, "books_report", args.clone())["structuredContent"].clone();
    let csv = call(&mut b, "books_report_csv", args.clone())["structuredContent"].clone();
    let (_, bytes) = exported(&mut b, args);
    let back = read_back(&bytes);
    matches_csv(&back, &parse_csv(csv["csv"].as_str().unwrap()));

    let find = |name: &str| -> usize {
        (0..back.cells.len())
            .find(|r| back.text(*r, 2) == name)
            .unwrap_or_else(|| panic!("no row {name}"))
    };
    let amounts = |name: &str| -> (f64, f64) {
        let r = find(name);
        (back.number(r, 3).unwrap().0, back.number(r, 4).unwrap().0)
    };
    let names: Vec<String> = report["sections"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|s| s["rows"].as_array().unwrap())
        .map(|r| r["name"].as_str().unwrap().to_string())
        .collect();
    let by = |number: &str| -> String {
        report["sections"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|s| s["rows"].as_array().unwrap())
            .find(|r| r["number"] == number)
            .map(|r| r["name"].as_str().unwrap().to_string())
            .unwrap_or_else(|| panic!("no account {number} in {names:?}"))
    };
    // negative: the refund exceeds October's spend on that account
    assert_eq!(amounts(&by("67200")), (-140.0, 100.0));
    // zero this month, something last month
    assert_eq!(amounts(&by("61100")), (0.0, 25.0));
    let zero = back.number(find(&by("61100")), 3).unwrap();
    assert_eq!(format!("{:.2}", zero.0), "0.00");
    assert_eq!(
        zero.1, MONEY_FORMAT,
        "a zero is still a number with the format"
    );
    // large
    let sales = amounts(&by("41000"));
    assert_eq!(format!("{:.2}", sales.0), "9999999999.99");
    // the total is the report's, 9,999,999,999.99 + 140.00 - 0 spent
    expect_total(&back, "Net income", &["10000000139.99", "-125.00"]);
}

#[test]
fn an_empty_period_is_a_valid_workbook_with_headers_and_zero_totals() {
    let (_dir, mut b) = golden();
    for (kind, title) in [
        ("pl", "Profit and loss"),
        ("bs", "Balance sheet"),
        ("cf", "Cash flow"),
    ] {
        let args = json!({ "kind": kind, "period": "2019-03", "compare_to": "none" });
        let csv = call(&mut b, "books_report_csv", args.clone())["structuredContent"].clone();
        let (_, bytes) = exported(&mut b, args);
        let back = read_back(&bytes);
        assert_eq!(back.sheets, [title]);
        let head = back.header();
        assert_eq!(back.text(head, 0), "Section");
        assert_eq!(back.text(head, 2), "Account");
        assert!(
            !back.text(head, 3).is_empty(),
            "{kind}: the amount column is named"
        );
        matches_csv(&back, &parse_csv(csv["csv"].as_str().unwrap()));
        // every amount there is zero
        for r in head + 1..back.cells.len() {
            if let Some((n, ..)) = back.number(r, 3) {
                assert_eq!(format!("{n:.2}"), "0.00", "{kind} row {r}");
            }
        }
        let last = back.cells.len() - 1;
        assert!(
            back.number(last, 3).is_some(),
            "{kind}: the last row is the net, a number"
        );
    }
}

#[test]
fn a_tag_and_a_compare_period_are_in_the_file_and_its_name() {
    let (_dir, mut b) = golden();
    let tags =
        call(&mut b, "books_tags_list", json!({ "kind": "department" }))["structuredContent"]
            .clone();
    let tag = tags["rows"][0]["ticket"].as_str().unwrap().to_string();
    let args = json!({ "kind": "pl", "period": "2026-10", "compare_to": "last_year", "tag": tag });
    let csv = call(&mut b, "books_report_csv", args.clone())["structuredContent"].clone();
    let (out, bytes) = exported(&mut b, args);
    let name = out["structuredContent"]["file_name"].as_str().unwrap();
    assert_eq!(
        name,
        csv["file_name"].as_str().unwrap().replace(".csv", ".xlsx")
    );
    assert!(
        name.starts_with("books-pl-2026-10-vs-2025-10-dept") && name.ends_with(".xlsx"),
        "{name}"
    );
    let back = read_back(&bytes);
    assert!(
        back.text(3, 0).starts_with("Only the department: "),
        "{}",
        back.text(3, 0)
    );
    matches_csv(&back, &parse_csv(csv["csv"].as_str().unwrap()));
}

#[test]
fn an_amount_a_cell_cannot_hold_exactly_is_refused_not_rounded() {
    let table = |v: &str| Table {
        title: "Cash flow",
        notes: vec![],
        header: vec![
            "Section".into(),
            "Number".into(),
            "Account".into(),
            "Oct".into(),
        ],
        rows: vec![Row {
            cells: vec![
                Cell::Text(String::new()),
                Cell::Text(String::new()),
                Cell::Text("X".into()),
                Cell::Amount(v.into()),
            ],
            role: report_xlsx::Role::Item,
        }],
    };
    let ok = report_xlsx::workbook(&table("-9999999999999.99")).unwrap();
    let back = read_back(&ok);
    assert_eq!(
        format!("{:.2}", back.number(back.header() + 1, 3).unwrap().0),
        "-9999999999999.99"
    );
    for bad in ["99999999999999.99", "12.5", "abc", "1,000.00", ""] {
        assert!(report_xlsx::workbook(&table(bad)).is_err(), "{bad}");
    }
}

#[test]
fn a_formula_is_never_written() {
    let (_dir, mut b) = golden();
    for kind in ["pl", "bs", "cf"] {
        let (_, bytes) = exported(&mut b, json!({ "kind": kind, "period": "2026-10" }));
        let doc = XlsxDocument::from_reader(Cursor::new(&bytes[..])).unwrap();
        let cells: Vec<_> = doc.worksheets[0]
            .rows
            .iter()
            .flat_map(|r| &r.cells)
            .collect();
        assert!(cells.len() > 10);
        assert!(
            cells.iter().all(|c| c.formula().is_none()),
            "{kind}: totals are values"
        );
    }
}

// ---- who may call it, and what it answers with ----------------------------

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
        books_core::date::pin_today(books_core::date::Date::new(2026, 10, 29));
        let mut books = Books::new(Paths::in_dir(dir.0.join("store")));
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../fixtures/demo/buwiz-trading-2026.json"
        ))
        .unwrap();
        demo::seed(&mut books, &serde_json::from_str(&text).unwrap()).unwrap();
        let mut bot = Self {
            server: Server::new(Source::default(), books),
            next: 1,
            sessions: HashMap::new(),
        };
        bot.initialize(
            json!({ "buwiz/documents": ["word", "sheet", "image", "pdf", "open_with", "save"] }),
        );
        let seed = [11u8; 32];
        let public = b64::encode(SigningKey::from_bytes(&seed).verifying_key().as_bytes());
        let p = bot.raw("buwiz.provision", json!({ "protocol": "1.0.0", "plugin": manifest::PLUGIN_ID, "key": { "alg": "ed25519", "seed": b64::encode(&seed) }, "pub": public }), Value::Null);
        assert_eq!(p["isError"], false, "{p}");
        bot
    }

    /// The host's `initialize`, with these experimental capabilities.
    fn initialize(&mut self, experimental: Value) {
        let reply = self
            .server
            .handle(
                &json!({ "jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {
                "protocolVersion": "2025-11-25",
                "capabilities": { "experimental": experimental },
                "clientInfo": { "name": "test", "version": "0" } } }),
            )
            .unwrap();
        assert!(reply.get("result").is_some(), "{reply}");
    }

    fn open_session(&mut self, app_id: &str) -> String {
        if let Some(id) = self.sessions.get(app_id) {
            return id.clone();
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
        self.sessions.insert(app_id.to_string(), open_id.clone());
        open_id
    }

    fn raw(&mut self, name: &str, args: Value, caller: Value) -> Value {
        let id = self.next;
        self.next += 1;
        let mut params = json!({ "name": name, "arguments": with_id(name, args) });
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

    fn app_on(&mut self, app: &str, name: &str, args: Value) -> Value {
        let open_id = self.open_session(app);
        let caller = json!({ "origin": "app", "plugin": manifest::PLUGIN_ID, "app": app, "open_id": open_id, "ts": now_ms() });
        self.raw(name, args, caller)
    }

    /// A call with the origin `origin`, on `app`'s open when it names one.
    fn origin(
        &mut self,
        origin: &str,
        app: Option<&str>,
        extra: Value,
        name: &str,
        args: Value,
    ) -> Value {
        let mut caller = json!({ "origin": origin, "plugin": manifest::PLUGIN_ID, "ts": now_ms() });
        if let Some(app) = app {
            caller["app"] = json!(app);
            caller["open_id"] = json!(self.open_session(app));
        }
        if let (Some(c), Some(e)) = (caller.as_object_mut(), extra.as_object()) {
            c.extend(e.clone());
        }
        self.raw(name, args, caller)
    }
}

#[test]
fn only_a_persons_app_call_runs_it_and_the_answer_carries_buwiz_save() {
    let dir = TempDir::new("xlsx-contract");
    let mut bot = Bot::new(&dir);
    let args = json!({ "kind": "pl", "period": "2026-10" });

    // the person's click in the Books app
    let out = bot.app_on("books", XLSX, args.clone());
    assert_eq!(out["isError"], false, "{out}");
    let save = &out["_meta"]["buwiz/save"];
    assert_eq!(
        out["_meta"].as_object().unwrap().len(),
        1,
        "nothing else rides on _meta"
    );
    assert_eq!(save["name"], "books-pl-2026-10.xlsx");
    assert_eq!(save["mime"], MEDIA_TYPE);
    let path = std::path::PathBuf::from(save["path"].as_str().unwrap());
    // where the Bot takes a hand-off from: <data folder>/open-<hex>/<folder>/<file>
    let store = dir.0.join("store");
    let rel = path
        .strip_prefix(&store)
        .expect("inside the plugin's data folder");
    let parts: Vec<_> = rel
        .components()
        .map(|c| c.as_os_str().to_str().unwrap().to_string())
        .collect();
    assert_eq!(parts.len(), 3, "{parts:?}");
    assert!(
        parts[0]
            .strip_prefix("open-")
            .is_some_and(|h| !h.is_empty() && h.bytes().all(|b| b.is_ascii_hexdigit()))
    );
    assert_eq!(parts[2], "books-pl-2026-10.xlsx");
    assert!(path.is_absolute());
    let meta = std::fs::symlink_metadata(&path).unwrap();
    assert!(meta.is_file(), "a regular file, no link");
    let bytes = std::fs::read(&path).unwrap();
    assert!(bytes.starts_with(b"PK\x03\x04"));
    // the text and the data the app and the model see name the file, never the path
    assert!(
        !out.to_string()
            .replace(save["path"].as_str().unwrap(), "")
            .contains("open-"),
        "the path is only in _meta"
    );
    assert!(out["structuredContent"].get("_save").is_none());
    assert!(
        !out["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("open-")
    );
    assert_eq!(
        out["structuredContent"]["file_name"],
        "books-pl-2026-10.xlsx"
    );

    // every other caller is refused, and no file is made
    let before = std::fs::read_dir(&store).unwrap().count();
    let denied = |r: &Value| {
        assert_eq!(r["isError"], true, "{r}");
        assert_eq!(r["structuredContent"]["code"], "permission_denied", "{r}");
        assert!(r.get("_meta").is_none(), "{r}");
    };
    // the model, on its own and pressing a screen
    denied(&bot.origin("agent", None, json!({}), XLSX, args.clone()));
    denied(&bot.origin(
        "agent",
        Some("books"),
        json!({ "press": { "by": "ai", "who": "agent.broker" } }),
        XLSX,
        args.clone(),
    ));
    denied(&bot.origin("agent", Some("books"), json!({ "press": { "by": "ai_approved", "who": "agent.broker", "once": "x", "args_hash": "y" } }), XLSX, args.clone()));
    // a person's app call that carries a press record is the assistant's, not the person's
    denied(&bot.origin(
        "app",
        Some("books"),
        json!({ "press": { "by": "ai", "who": "agent.broker" } }),
        XLSX,
        args.clone(),
    ));
    // a routine, a resolve, a widget, an origin nobody knows, and no caller at all
    denied(&bot.origin(
        "routine",
        None,
        json!({ "routine": "weekly-tidy-up" }),
        XLSX,
        args.clone(),
    ));
    denied(&bot.origin("resolve", None, json!({}), XLSX, args.clone()));
    denied(&bot.origin("widget", None, json!({}), XLSX, args.clone()));
    denied(&bot.origin("somebody", None, json!({}), XLSX, args.clone()));
    denied(&bot.raw(XLSX, args.clone(), Value::Null));
    // another Books screen that does not list the tool
    denied(&bot.app_on("home", XLSX, args.clone()));
    assert_eq!(
        std::fs::read_dir(&store).unwrap().count(),
        before,
        "a refused call makes no file"
    );
}

#[test]
fn a_report_that_is_not_one_of_the_three_is_refused() {
    let dir = TempDir::new("xlsx-kinds");
    let mut bot = Bot::new(&dir);
    for kind in ["tb", "ag", "ac", "ps", "ledger", "nope"] {
        let r = bot.app_on("books", XLSX, json!({ "kind": kind, "period": "2026-10" }));
        assert_eq!(r["isError"], true, "{kind}: {r}");
        assert!(r.get("_meta").is_none(), "{kind}");
    }
}

#[test]
fn the_tool_is_the_apps_alone_and_a_write() {
    let listed = tools::list();
    let t = listed.iter().find(|t| t["name"] == XLSX).unwrap();
    assert_eq!(
        t["_meta"]["buwiz/audience"],
        json!(["app"]),
        "not for the model"
    );
    assert_eq!(t["annotations"]["readOnlyHint"], false, "it writes a file");
    assert!(
        t["_meta"].get("buwiz/press").is_none(),
        "not on the assistant's press, with or without a yes"
    );
    assert!(!books_mini_app::press::APPROVED.contains(&XLSX));
    // a direct call as anything but the app is refused too
    let dir = TempDir::new("xlsx-direct");
    let mut b = Books::new(Paths::in_dir(dir.0.join("store")));
    for o in [
        Origin::Agent,
        Origin::Routine,
        Origin::Resolve,
        Origin::Widget,
        Origin::Unknown,
    ] {
        let r = raw(&mut b, XLSX, json!({}), o);
        assert_eq!(r["isError"], true);
        assert_eq!(r["structuredContent"]["code"], "forbidden");
    }
    // only the Books window lists it
    let apps = manifest::unsigned(manifest::RELEASE_KEY_ID)["apps"]
        .as_array()
        .unwrap()
        .clone();
    for a in apps {
        let has = a["permissions"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n == XLSX);
        assert_eq!(has, a["id"] == "books", "{}", a["id"]);
    }
}

#[test]
fn an_old_hand_off_is_swept_and_a_fresh_one_is_kept() {
    let tmp = TempDir::new("xlsx-sweep");
    let dir = tmp.0.join("data");
    std::fs::create_dir_all(&dir).unwrap();
    let first = report_xlsx::hand_over(&dir, "a.xlsx", b"PK").unwrap();
    let second = report_xlsx::hand_over(&dir, "b.xlsx", b"PK").unwrap();
    assert!(first.exists() && second.exists(), "a fresh hand-off stays");
    assert_ne!(first.parent(), second.parent());
    // a folder that is not a hand-off is left alone
    std::fs::create_dir_all(dir.join("open-not-hex")).unwrap();
    std::fs::create_dir_all(dir.join("state")).unwrap();
    // one from two hours ago is swept
    let old = dir.join("open-abc123/1");
    std::fs::create_dir_all(&old).unwrap();
    std::fs::write(old.join("old.xlsx"), b"PK").unwrap();
    let long_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(2 * 3600);
    std::fs::File::open(dir.join("open-abc123"))
        .unwrap()
        .set_modified(long_ago)
        .unwrap();
    report_xlsx::hand_over(&dir, "c.xlsx", b"PK").unwrap();
    assert!(
        !dir.join("open-abc123").exists(),
        "an old hand-off is swept"
    );
    assert!(first.exists() && second.exists());
    assert!(dir.join("open-not-hex").exists() && dir.join("state").exists());
}

#[test]
fn nothing_private_is_in_the_file() {
    let (_dir, mut b) = golden();
    let (_, bytes) = exported(&mut b, json!({ "kind": "bs", "period": "2026-10" }));
    let all = (0..bytes.len().saturating_sub(4)).any(|i| &bytes[i..i + 4] == b"tkt_");
    assert!(!all, "no ticket in the file");
    let back = read_back(&bytes);
    for r in 0..back.cells.len() {
        for c in 0..back.cells[r].len() {
            if let Some((CellValue::String(s), ..)) = &back.cells[r][c] {
                assert!(!s.contains("tkt_"), "{s}");
            }
        }
    }
}

#[test]
fn the_csv_and_the_workbook_come_from_one_row_builder_and_count_rows_alike() {
    let (_dir, mut b) = golden();
    for kind in ["pl", "bs", "cf"] {
        for compare in ["none", "previous"] {
            let args = json!({ "kind": kind, "period": "2026-10", "compare_to": compare });
            let report = call(&mut b, "books_report", args.clone())["structuredContent"].clone();
            let csv = call(&mut b, "books_report_csv", args.clone())["structuredContent"].clone();
            let (out, _) = exported(&mut b, args);
            let table = report_xlsx::table(&report).unwrap();
            // the CSV is the builder's header and its rows but a section's total
            let q = |t: &str| {
                if t.contains([',', '"', '\n']) {
                    format!("\"{}\"", t.replace('"', "\"\""))
                } else {
                    t.to_string()
                }
            };
            let mut want = vec![
                table
                    .header
                    .iter()
                    .map(|h| q(h))
                    .collect::<Vec<_>>()
                    .join(","),
            ];
            for r in table
                .rows
                .iter()
                .filter(|r| r.role != report_xlsx::Role::SectionTotal)
            {
                want.push(
                    r.cells
                        .iter()
                        .map(|c| match c {
                            Cell::Text(t) => q(t),
                            Cell::Amount(t) => t.clone(),
                            Cell::Blank => String::new(),
                        })
                        .collect::<Vec<_>>()
                        .join(","),
                );
            }
            assert_eq!(
                csv["csv"].as_str().unwrap(),
                want.join("\n") + "\n",
                "{kind} {compare}"
            );
            assert_eq!(
                out["structuredContent"]["rows"], csv["rows"],
                "{kind} {compare}: rows"
            );
            assert_eq!(csv["rows"], want.len());
        }
    }
}

// ---- a write: one id, one file; and a host that saves --------------------

/// The hand-off folders in the plugin's data folder.
fn hand_offs(dir: &TempDir) -> usize {
    std::fs::read_dir(dir.0.join("store"))
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("open-"))
        .count()
}

#[test]
fn the_same_id_twice_is_one_file_and_one_hand_off_and_two_ids_are_two() {
    let dir = TempDir::new("xlsx-repeat");
    let mut bot = Bot::new(&dir);
    let args = json!({ "client_request_id": "xlsx-same-0001", "kind": "pl", "period": "2026-10" });

    let first = bot.app_on("books", XLSX, args.clone());
    assert_eq!(first["isError"], false, "{first}");
    assert!(first["_meta"]["buwiz/save"]["path"].is_string());
    assert!(first["structuredContent"].get("already_sent").is_none());
    assert_eq!(hand_offs(&dir), 1);

    // the same id: no second file, nothing for the Bot to save, said so
    let again = bot.app_on("books", XLSX, args.clone());
    assert_eq!(again["isError"], false, "{again}");
    assert!(again.get("_meta").is_none(), "a repeat hands nothing over");
    assert_eq!(again["structuredContent"]["already_sent"], true);
    assert_eq!(
        again["structuredContent"]["file_name"],
        first["structuredContent"]["file_name"]
    );
    assert!(
        again["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("already_sent")
    );
    assert!(!again.to_string().contains("open-"), "no path in a repeat");
    assert_eq!(hand_offs(&dir), 1, "no second file");

    // the same id for another report is refused, and makes no file
    let other = bot.app_on(
        "books",
        XLSX,
        json!({ "client_request_id": "xlsx-same-0001", "kind": "bs", "period": "2026-10" }),
    );
    assert_eq!(other["isError"], true, "{other}");
    assert_eq!(other["structuredContent"]["code"], "request_id_reused");
    assert!(other.get("_meta").is_none());
    assert_eq!(hand_offs(&dir), 1);

    // a new id is a new file and a new hand-off
    let mut next = args.clone();
    next["client_request_id"] = json!("xlsx-same-0002");
    let second = bot.app_on("books", XLSX, next);
    assert_eq!(second["isError"], false, "{second}");
    assert!(second["structuredContent"].get("already_sent").is_none());
    assert_ne!(
        second["_meta"]["buwiz/save"]["path"],
        first["_meta"]["buwiz/save"]["path"]
    );
    assert_eq!(hand_offs(&dir), 2);

    // no id: refused, no file
    let mut bare = Books::new(Paths::in_dir(dir.0.join("store")));
    bare.host_saves = true;
    let args: Map<String, Value> = json!({ "kind": "pl", "period": "2026-10" })
        .as_object()
        .unwrap()
        .clone();
    let r = tools::call(XLSX, &args, Origin::App, &mut bare).unwrap();
    assert_eq!(r["isError"], true, "{r}");
    assert_eq!(r["structuredContent"]["code"], "invalid_args");
    assert!(r.get("_meta").is_none());
}

#[test]
fn the_tool_asks_for_a_client_request_id() {
    let listed = tools::list();
    let t = listed.iter().find(|t| t["name"] == XLSX).unwrap();
    assert_eq!(t["inputSchema"]["required"], json!(["client_request_id"]));
    assert!(t["inputSchema"]["properties"]["client_request_id"].is_object());
}

#[test]
fn a_host_that_does_not_save_gets_a_plain_error_and_no_file() {
    let dir = TempDir::new("xlsx-nosave");
    let mut bot = Bot::new(&dir);
    let args = json!({ "kind": "pl", "period": "2026-10" });
    for said in [
        json!({}),
        Value::Null,
        json!({ "buwiz/documents": [] }),
        json!({ "buwiz/documents": ["word", "sheet", "image", "pdf", "open_with"] }),
        json!({ "buwiz/documents": "save" }),
    ] {
        bot.initialize(said.clone());
        let r = bot.app_on("books", XLSX, args.clone());
        assert_eq!(r["isError"], true, "{said}: {r}");
        assert_eq!(r["structuredContent"]["code"], "not_supported", "{r}");
        assert!(r.get("_meta").is_none(), "{r}");
        assert!(
            r["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("cannot save a file")
        );
        assert_eq!(hand_offs(&dir), 0, "no file when the host cannot save");
    }
    // a host that says "save" gets the file
    bot.initialize(json!({ "buwiz/documents": ["save"] }));
    let r = bot.app_on("books", XLSX, args.clone());
    assert_eq!(r["isError"], false, "{r}");
    assert!(r["_meta"]["buwiz/save"]["path"].is_string());
    assert_eq!(hand_offs(&dir), 1);
    // and one that says nothing at all, as the first test host did not
    let mut quiet = Books::new(Paths::in_dir(dir.0.join("store")));
    let a: Map<String, Value> = with_id(XLSX, args).as_object().unwrap().clone();
    let r = tools::call(XLSX, &a, Origin::App, &mut quiet).unwrap();
    assert_eq!(r["structuredContent"]["code"], "not_supported", "{r}");
}

/// Centavos of a two-place amount text, with no float.
fn centavos(t: &str) -> i64 {
    let (neg, body) = t.strip_prefix('-').map_or((false, t), |b| (true, b));
    let n: i64 = body.replace('.', "").parse().unwrap();
    if neg { -n } else { n }
}

/// Every section-total row is the sum of the item rows of the section it
/// sits under, in each amount column, to the centavo.
fn assert_totals_are_their_sections(t: &Table) -> usize {
    let cols = t.header.len() - 3;
    let mut sums = vec![0i64; cols];
    let mut seen = 0;
    for row in &t.rows {
        let amounts: Vec<i64> = (0..cols)
            .map(|c| match &row.cells[3 + c] {
                Cell::Amount(x) => centavos(x),
                _ => 0,
            })
            .collect();
        match row.role {
            report_xlsx::Role::Item => {
                for (s, a) in sums.iter_mut().zip(&amounts) {
                    *s += a;
                }
            }
            report_xlsx::Role::SectionTotal => {
                assert_eq!(amounts, sums, "{:?} is its own section's sum", row.cells[2]);
                sums = vec![0; cols];
                seen += 1;
            }
            report_xlsx::Role::Net => {}
        }
    }
    seen
}

#[test]
fn each_pl_section_total_is_the_sum_of_its_own_rows() {
    let (_dir, mut b) = golden();
    for compare in ["none", "previous", "last_year"] {
        let args = json!({ "kind": "pl", "period": "2026-10", "compare_to": compare });
        let report = call(&mut b, "books_report", args)["structuredContent"].clone();
        let t = report_xlsx::table(&report).unwrap();
        assert_eq!(t.header.len(), if compare == "none" { 4 } else { 5 });
        assert_eq!(
            assert_totals_are_their_sections(&t),
            2,
            "revenue and expenses, {compare}"
        );
    }
    // a book where revenue and expenses are not in the same order of size
    let (_dir, mut e) = edge_book();
    let args = json!({ "kind": "pl", "period": "2026-10" });
    let report = call(&mut e, "books_report", args)["structuredContent"].clone();
    let t = report_xlsx::table(&report).unwrap();
    assert_eq!(assert_totals_are_their_sections(&t), 2);
}

#[test]
fn a_pl_total_is_paired_by_its_label_not_its_place() {
    let (_dir, mut b) = golden();
    let args = json!({ "kind": "pl", "period": "2026-10", "compare_to": "none" });
    let mut report = call(&mut b, "books_report", args)["structuredContent"].clone();
    let before = report_xlsx::table(&report).unwrap();
    // the two totals in the other order: each still lands under its section
    report["totals"].as_array_mut().unwrap().reverse();
    assert_eq!(report_xlsx::table(&report).unwrap(), before);
    report["totals"] = json!([]);
    assert!(report_xlsx::table(&report).is_err());
}

#[test]
fn initialize_sweeps_without_another_export() {
    let dir = TempDir::new("xlsx-initialize-sweep");
    let mut bot = Bot::new(&dir);
    let old = dir.0.join("store/open-abc123");
    std::fs::create_dir_all(old.join("1")).unwrap();
    std::fs::write(old.join("1/old.xlsx"), b"PK").unwrap();
    std::fs::File::open(&old)
        .unwrap()
        .set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(7200))
        .unwrap();
    bot.initialize(json!({ "buwiz/documents": ["save"] }));
    assert!(!old.exists());
}

#[test]
fn a_failed_remember_leaves_no_workbook() {
    let (dir, mut books) = golden();
    books.store().unwrap().conn().execute_batch(
        "CREATE TRIGGER refuse_export BEFORE INSERT ON idempotency BEGIN SELECT RAISE(ABORT, 'synthetic failure'); END;",
    ).unwrap();
    let out = raw(
        &mut books,
        XLSX,
        json!({ "kind": "pl", "period": "2026-10" }),
        Origin::App,
    );
    assert_eq!(out["isError"], true);
    assert!(out.get("_meta").is_none());
    assert_eq!(hand_offs(&dir), 0);
}

#[test]
fn a_replay_does_not_prepare_a_report() {
    let (dir, mut books) = golden();
    let args =
        json!({ "client_request_id": "xlsx-replay-no-report", "kind": "pl", "period": "2026-10" });
    call(&mut books, XLSX, args.clone());
    // Any attempt to read the chart now fails, but a replay needs no chart.
    books
        .store()
        .unwrap()
        .conn()
        .execute_batch("ALTER TABLE account RENAME TO hidden_account;")
        .unwrap();
    let again = call(&mut books, XLSX, args);
    assert_eq!(again["structuredContent"]["already_sent"], true);
    assert_eq!(hand_offs(&dir), 1);
}

#[test]
fn a_non_utf8_data_folder_is_refused_without_a_file() {
    use std::os::unix::ffi::OsStringExt;
    let dir = TempDir::new("xlsx-path");
    let path = dir.0.join(std::ffi::OsString::from_vec(vec![b'd', 0xff]));
    std::fs::create_dir(&path).unwrap();
    let err = report_xlsx::hand_over(&path, "report.xlsx", b"PK").unwrap_err();
    assert!(err.message.contains("UTF-8"));
    assert_eq!(std::fs::read_dir(path).unwrap().count(), 0);
}

#[test]
fn an_empty_pl_has_every_section_total() {
    let dir = TempDir::new("xlsx-empty-totals");
    let mut books = Books::new(Paths::in_dir(dir.0.join("store")));
    call(
        &mut books,
        "books_app_book_create",
        json!({ "client_request_id": "empty-totals-book", "label": "Synthetic" }),
    );
    let report = call(
        &mut books,
        "books_report",
        json!({ "kind": "pl", "period": "2026-10" }),
    )["structuredContent"]
        .clone();
    let table = report_xlsx::table(&report).unwrap();
    assert_eq!(
        assert_totals_are_their_sections(&table),
        report["sections"].as_array().unwrap().len()
    );
}
