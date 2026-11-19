//! books_app_statement_page, the snapshot image the Books window draws beside
//! the match list, and the journal lines `books_statement_get` gives the app
//! for a matched row. Synthetic data only.

use std::collections::BTreeMap;
use std::path::Path;

use books_mini_app::reconcile_document::{MAX_PAGE_IMAGE, PAGE_PART};
use books_mini_app::service::{Books, Origin};
use books_mini_app::{apps, demo, html, manifest, tools};
use books_store::{Paths, TempDir};
use serde_json::{Map, Value, json};

const DEMO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/demo/buwiz-trading-2026.json"
);
const CSV: &str = "Date,Description,Debit,Credit,Balance\n2026-09-03,Coffee shop,\"1,250.50\",,\n2026-09-20,Client deposit,,\"15,500.75\",\"50,000.00\"\n";
const PAGE: &str = "books_app_statement_page";

fn call(books: &mut Books, name: &str, origin: Origin, args: Value) -> Value {
    let args: Map<String, Value> = args.as_object().unwrap().clone();
    tools::call(name, &args, origin, books).unwrap()
}

fn setup(name: &str) -> (TempDir, Books) {
    books_core::date::pin_today(books_core::date::Date::new(2026, 10, 29));
    let dir = TempDir::new(name);
    let mut books = Books::new(Paths::in_dir(dir.0.join("store")));
    let demo_book: Value = serde_json::from_str(&std::fs::read_to_string(DEMO).unwrap()).unwrap();
    demo::seed(&mut books, &demo_book).unwrap();
    let out = call(
        &mut books,
        "books_app_statement_import",
        Origin::App,
        json!({ "client_request_id": format!("{name}-import-0001"), "account": "11010", "csv_base64": books_mini_app::b64::encode(CSV.as_bytes()), "file_name": "synthetic.csv" }),
    );
    assert_eq!(out["isError"], false, "{out}");
    (dir, books)
}

/// A PNG header of `width` by `height`, then filler up to `len` bytes.
fn png(width: u32, height: u32, len: usize) -> Vec<u8> {
    let mut b = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x0dIHDR".to_vec();
    b.extend(width.to_be_bytes());
    b.extend(height.to_be_bytes());
    let mut n = 0u8;
    while b.len() < len {
        n = n.wrapping_mul(31).wrapping_add(7);
        b.push(n);
    }
    b
}

/// A JPEG with an application segment before its frame header.
fn jpeg(width: u16, height: u16) -> Vec<u8> {
    let mut b = b"\xff\xd8\xff\xe0\x00\x04\x00\x00\xff\xc2\x00\x11\x08".to_vec();
    b.extend(height.to_be_bytes());
    b.extend(width.to_be_bytes());
    b.extend([0u8; 12]);
    b
}

fn snapshot(books: &mut Books, bytes: &[u8], mime: &str) {
    let conn = books.store().unwrap().conn();
    let book: i64 = conn
        .query_row("SELECT book_id FROM statement LIMIT 1", [], |r| r.get(0))
        .unwrap();
    let (id, _) = books_store::evidence::insert(
        conn,
        book,
        &books_store::evidence::Input {
            bytes,
            mime,
            page_count: Some(1),
            source: "user",
            kind: "statement",
        },
    )
    .unwrap();
    conn.execute("UPDATE statement SET evidence_id = ?1", [id])
        .unwrap();
}

fn page(books: &mut Books, part: Option<u64>) -> Value {
    let mut args = json!({ "account": "11010", "month": "2026-09" });
    if let Some(part) = part {
        args["part"] = json!(part);
    }
    call(books, PAGE, Origin::App, args)
}

fn code(out: &Value) -> String {
    assert_eq!(out["isError"], true, "{out}");
    out["structuredContent"]["code"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

fn statement(books: &mut Books, origin: Origin) -> Value {
    let out = call(
        books,
        "books_statement_get",
        origin,
        json!({ "account": "11010", "month": "2026-09" }),
    );
    assert_eq!(out["isError"], false, "{out}");
    out["structuredContent"]["statement"].clone()
}

/// Every file of the data folder, byte for byte.
fn files(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    for e in std::fs::read_dir(dir).unwrap() {
        let e = e.unwrap();
        if e.path().is_dir() {
            out.extend(files(&e.path()));
        } else {
            out.insert(
                e.path().display().to_string(),
                std::fs::read(e.path()).unwrap(),
            );
        }
    }
    out
}

#[test]
fn a_small_png_comes_in_one_part_with_its_size() {
    let (_dir, mut b) = setup("sp-small");
    let bytes = png(850, 1100, 400);
    snapshot(&mut b, &bytes, "image/png");
    let out = page(&mut b, None);
    assert_eq!(out["isError"], false, "{out}");
    let got = &out["structuredContent"];
    assert_eq!(got["mime"], "image/png");
    assert_eq!(got["part"], 0);
    assert_eq!(got["parts"], 1);
    assert_eq!(got["width"], 850);
    assert_eq!(got["height"], 1100);
    assert_eq!(got["data_base64"], json!(html::base64(&bytes)));
    // nothing of the statement rides along
    let text = out.to_string();
    for absent in ["Coffee shop", "tkt_", "1,250.50", "15,500.75"] {
        assert!(!text.contains(absent), "{absent}");
    }
}

#[test]
fn a_large_image_comes_in_parts_that_join_and_each_fits_one_call() {
    let (_dir, mut b) = setup("sp-parts");
    let bytes = png(1700, 2200, PAGE_PART * 2 + 12_345);
    snapshot(&mut b, &bytes, "image/png");
    let mut joined = String::new();
    for part in 0..3u64 {
        let out = page(&mut b, Some(part));
        assert_eq!(out["isError"], false, "{out}");
        assert_eq!(out["structuredContent"]["parts"], 3);
        assert_eq!(out["structuredContent"]["part"], part);
        assert!(
            out.to_string().len() < 1_048_576,
            "a call's result is at most 1 MiB"
        );
        joined.push_str(out["structuredContent"]["data_base64"].as_str().unwrap());
    }
    // the parts' base64 is the whole image's: the window joins text, not bytes
    assert_eq!(joined, html::base64(&bytes));
    assert_eq!(code(&page(&mut b, Some(3))), "invalid_args");
}

#[test]
fn a_jpeg_reports_the_size_in_its_frame_header() {
    let (_dir, mut b) = setup("sp-jpeg");
    snapshot(&mut b, &jpeg(640, 480), "image/jpeg");
    let out = page(&mut b, Some(0));
    assert_eq!(out["isError"], false, "{out}");
    assert_eq!(out["structuredContent"]["mime"], "image/jpeg");
    assert_eq!(out["structuredContent"]["width"], 640);
    assert_eq!(out["structuredContent"]["height"], 480);
}

#[test]
fn an_image_without_a_readable_header_still_comes_with_no_size() {
    let (_dir, mut b) = setup("sp-nosize");
    snapshot(&mut b, b"\x89PNG\r\n\x1a\nsynthetic-image", "image/png");
    let out = page(&mut b, None);
    assert_eq!(out["isError"], false, "{out}");
    assert_eq!(out["structuredContent"]["width"], Value::Null);
    assert_eq!(out["structuredContent"]["height"], Value::Null);
}

#[test]
fn only_a_png_or_jpeg_snapshot_within_the_bound_is_given() {
    let (_dir, mut b) = setup("sp-refusals");
    assert_eq!(code(&page(&mut b, None)), "no_snapshot");
    assert_eq!(statement(&mut b, Origin::App)["snapshot_kind"], Value::Null);
    snapshot(&mut b, b"%PDF-1.7 synthetic", "application/pdf");
    assert_eq!(code(&page(&mut b, None)), "not_supported");
    assert_eq!(statement(&mut b, Origin::App)["snapshot_kind"], "pdf");
    snapshot(&mut b, b"RIFF0000WEBPsynthetic", "image/webp");
    assert_eq!(code(&page(&mut b, None)), "not_supported");
    assert_eq!(statement(&mut b, Origin::App)["snapshot_kind"], "other");
    snapshot(&mut b, &png(10, 10, MAX_PAGE_IMAGE + 1), "image/png");
    assert_eq!(code(&page(&mut b, None)), "too_large");
    assert_eq!(statement(&mut b, Origin::App)["snapshot_kind"], "large");
    snapshot(&mut b, &png(10, 10, 64), "image/png");
    assert_eq!(statement(&mut b, Origin::App)["snapshot_kind"], "image");
    let out = call(
        &mut b,
        PAGE,
        Origin::App,
        json!({ "account": "11010", "month": "2026-08" }),
    );
    assert_eq!(code(&out), "not_found");
    let out = call(
        &mut b,
        PAGE,
        Origin::App,
        json!({ "account": "11010", "month": "2026-09", "part": "0" }),
    );
    assert_eq!(code(&out), "invalid_args");
}

#[test]
fn the_page_is_the_apps_alone_and_reading_it_changes_nothing() {
    let (dir, mut b) = setup("sp-read");
    snapshot(&mut b, &png(850, 1100, 2_000), "image/png");
    let out = call(
        &mut b,
        PAGE,
        Origin::Agent,
        json!({ "account": "11010", "month": "2026-09" }),
    );
    assert_eq!(out["isError"], true, "{out}");
    assert!(out["structuredContent"]["data_base64"].is_null());
    assert!(tools::read_only(PAGE));
    let def = tools::list()
        .into_iter()
        .find(|t| t["name"] == PAGE)
        .expect("the tool is listed");
    assert_eq!(def["_meta"]["buwiz/audience"], json!(["app"]));

    // settle the store, then read: the folder stays byte for byte
    drop(b);
    let store = dir.0.join("store");
    let mut b = Books::new(Paths::in_dir(store.clone()));
    assert_eq!(page(&mut b, None)["isError"], false);
    drop(b);
    let before = files(&store);
    let mut b = Books::new(Paths::in_dir(store.clone()));
    assert_eq!(page(&mut b, None)["isError"], false);
    assert_eq!(code(&page(&mut b, Some(9))), "invalid_args");
    drop(b);
    assert!(
        before == files(&store),
        "reading the page changed the folder"
    );
}

#[test]
fn a_matched_row_carries_its_entrys_journal_lines_for_the_app_only() {
    let (_dir, mut b) = setup("sp-lines");
    let before = statement(&mut b, Origin::App);
    let row = &before["rows"][0];
    assert_eq!(row["match"], "x");
    assert!(row["books"]["lines"].is_null(), "no entry, no lines");
    let line = row["ticket"].as_str().unwrap().to_string();
    let out = call(
        &mut b,
        "books_app_statement_add_line",
        Origin::App,
        json!({ "client_request_id": "sp-lines-add-0001", "account": "11010", "month": "2026-09", "line": line }),
    );
    assert_eq!(out["isError"], false, "{out}");

    let after = statement(&mut b, Origin::App);
    let row = &after["rows"][0];
    assert_ne!(row["match"], "x");
    let lines = row["books"]["lines"].as_array().expect("journal lines");
    assert_eq!(lines.len(), 2);
    // money out: the cash account is credited, the other side debited, both
    // as display text of the statement's amount
    let shown = |key: &str| -> Vec<&str> {
        lines
            .iter()
            .map(|l| l[key].as_str().unwrap())
            .filter(|t| !t.is_empty())
            .collect()
    };
    assert_eq!(shown("debit_shown"), ["1,250.50"]);
    assert_eq!(shown("credit_shown"), ["1,250.50"]);
    let cash = lines
        .iter()
        .find(|l| !l["credit_shown"].as_str().unwrap().is_empty())
        .unwrap();
    assert!(cash["account"].as_str().unwrap().starts_with("11010 "));
    for l in lines {
        assert!(!l["account"].as_str().unwrap().is_empty());
        let keys: Vec<&String> = l.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["account", "credit_shown", "debit_shown"]);
    }
    // the unmatched row beside it still has none
    assert!(after["rows"][1]["books"]["lines"].is_null());

    // the assistant's read of the same statement gets no journal lines
    let agent = statement(&mut b, Origin::Agent);
    assert!(agent["rows"][0]["books"]["lines"].is_null());
    assert!(agent["rows"][0]["books"]["title"].is_string());
}

#[test]
fn a_statement_pdf_or_picture_is_kept_as_a_document_and_reads_no_line() {
    let (_dir, mut b) = setup("sp-document");
    let lines = |b: &mut Books| -> i64 {
        b.store()
            .unwrap()
            .conn()
            .query_row("SELECT COUNT(*) FROM statement_line", [], |r| r.get(0))
            .unwrap()
    };
    let before = lines(&mut b);
    let send = |b: &mut Books, n: u32, bytes: &[u8], mime: &str| -> Value {
        let upload = format!("up-doc-{n}");
        let out = call(
            b,
            "books_app_upload_part",
            Origin::App,
            json!({ "upload": upload, "part": 0, "parts": 1, "data_base64": books_mini_app::b64::encode(bytes) }),
        );
        assert_eq!(out["isError"], false, "{out}");
        call(
            b,
            "books_app_statement_import",
            Origin::App,
            json!({ "client_request_id": format!("sp-document-000{n}"), "account": "11010", "document_upload": upload, "document_mime": mime }),
        )
    };
    let out = send(&mut b, 1, b"%PDF-1.7 synthetic", "application/pdf");
    assert_eq!(out["isError"], false, "{out}");
    assert_eq!(out["structuredContent"]["mime"], "application/pdf");
    assert!(out["structuredContent"]["document"].is_string());
    let (kind, source): (String, String) = b
        .store()
        .unwrap()
        .conn()
        .query_row(
            "SELECT kind, source FROM evidence ORDER BY id DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!((kind.as_str(), source.as_str()), ("statement", "user"));
    assert_eq!(lines(&mut b), before, "Books read no line from the page");
    // the bytes must be what the type says, and a type Reconcile keeps
    assert_eq!(
        code(&send(&mut b, 2, b"not a picture", "image/png")),
        "invalid_args"
    );
    assert_eq!(
        code(&send(&mut b, 3, b"RIFF0000WEBPsynthetic", "image/webp")),
        "invalid_args"
    );
    assert_eq!(
        send(&mut b, 4, &png(8, 8, 64), "image/png")["isError"],
        false
    );
}

#[test]
fn the_window_names_the_page_tool_and_picks_statement_documents() {
    let m = manifest::unsigned(manifest::RELEASE_KEY_ID);
    let window = m["apps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == "books")
        .unwrap();
    let granted = window["permissions"]["tools"].as_array().unwrap();
    assert!(granted.iter().any(|t| t == PAGE));
    assert!(granted.len() <= 64, "{} window tools", granted.len());
    let picks = window["permissions"]["pick_file"]["extensions"]
        .as_array()
        .unwrap();
    for ext in [".csv", ".xlsx", ".xls", ".pdf", ".png", ".jpg", ".jpeg"] {
        assert!(picks.iter().any(|e| e == ext), "{ext}");
    }
    let source = match apps::find("books").unwrap().source {
        apps::AppSource::Tsx(source) => source,
        apps::AppSource::Native { .. } => panic!("the Books window is TSX"),
    };
    // the page is drawn by nsh from a data: source the app holds
    assert!(source.contains("<Image src={img()?.src"));
    assert!(source.contains("`data:${String(first.mime)};base64,${data}`"));
    assert!(!source.contains("http://") && !source.contains("https://"));
}
