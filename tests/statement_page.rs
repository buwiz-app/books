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
fn a_long_statement_leaves_journal_lines_to_the_drawer() {
    let dir = TempDir::new("sp-long");
    books_core::date::pin_today(books_core::date::Date::new(2026, 10, 29));
    let mut b = Books::new(Paths::in_dir(dir.0.join("store")));
    let demo_book: Value = serde_json::from_str(&std::fs::read_to_string(DEMO).unwrap()).unwrap();
    demo::seed(&mut b, &demo_book).unwrap();
    let mut csv = String::from("Date,Description,Debit,Credit,Balance\n");
    for i in 0..301 {
        csv.push_str(&format!("2026-09-{:02},Line {i},10.00,,\n", i % 28 + 1));
    }
    let out = call(
        &mut b,
        "books_app_statement_import",
        Origin::App,
        json!({ "client_request_id": "sp-long-import-0001", "account": "11010", "csv_text": csv, "closing": "0.00" }),
    );
    assert_eq!(out["isError"], false, "{out}");
    let line = statement(&mut b, Origin::App)["rows"][0]["ticket"]
        .as_str()
        .unwrap()
        .to_string();
    let out = call(
        &mut b,
        "books_app_statement_add_line",
        Origin::App,
        json!({ "client_request_id": "sp-long-add-0001", "account": "11010", "month": "2026-09", "line": line }),
    );
    assert_eq!(out["isError"], false, "{out}");
    let rows = statement(&mut b, Origin::App)["rows"].clone();
    assert_eq!(rows.as_array().unwrap().len(), 301);
    assert!(rows.as_array().unwrap().iter().any(|r| r["match"] != "x"));
    assert!(
        rows.as_array()
            .unwrap()
            .iter()
            .all(|r| r["books"]["lines"].is_null())
    );
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
    // the same request again returns the stored result, though the upload is gone
    let again = call(
        &mut b,
        "books_app_statement_import",
        Origin::App,
        json!({ "client_request_id": "sp-document-0001", "account": "11010", "document_upload": "up-doc-1", "document_mime": "application/pdf" }),
    );
    assert_eq!(again["isError"], false, "{again}");
    assert_eq!(
        again["structuredContent"]["document"],
        out["structuredContent"]["document"]
    );
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
    assert!(source.contains("<Avatar src={img()?.src"));
    assert!(source.contains("`data:${String(first.mime)};base64,${data}`"));
    assert!(!source.contains("http://") && !source.contains("https://"));
}

/// Keep `bytes` as a statement document and return its evidence ticket.
fn document(b: &mut Books, n: u8, bytes: &[u8], mime: &str) -> Value {
    let upload = format!("up-page-{n}");
    let out = call(
        b,
        "books_app_upload_part",
        Origin::App,
        json!({ "upload": upload, "part": 0, "parts": 1, "data_base64": books_mini_app::b64::encode(bytes) }),
    );
    assert_eq!(out["isError"], false, "{out}");
    let out = call(
        b,
        "books_app_statement_import",
        Origin::App,
        json!({ "client_request_id": format!("sp-pages-doc-{n}"), "account": "11010", "document_upload": upload, "document_mime": mime }),
    );
    assert_eq!(out["isError"], false, "{out}");
    out["structuredContent"]["document"].clone()
}

/// Draft two lines, on pages 1 and 2, and accept them as September's statement.
fn read_two_pages(b: &mut Books, n: u8, extra: Value) -> Value {
    let mut args = json!({
        "client_request_id": format!("sp-pages-draft-{n}"), "account": "11010", "month": "2026-09", "closing": "14250.25",
        "lines": [
            { "date": "2026-09-03", "description": "Coffee shop", "amount": "-1250.50", "page": 1, "bbox": [10, 100, 990, 140] },
            { "date": "2026-09-20", "description": "Client deposit", "amount": "15500.75", "page": 2, "bbox": [10, 200, 990, 240] },
        ],
    });
    for (k, v) in extra.as_object().unwrap() {
        args[k] = v.clone();
    }
    let draft = call(b, "books_statement_draft_lines", Origin::App, args);
    if draft["isError"] == true {
        return draft;
    }
    call(
        b,
        "books_app_statement_accept",
        Origin::App,
        json!({ "client_request_id": format!("sp-pages-accept-{n}"), "account": "11010", "month": "2026-09", "draft": draft["structuredContent"]["draft"] }),
    )
}

fn page_no(books: &mut Books, page: u64) -> Value {
    call(
        books,
        PAGE,
        Origin::App,
        json!({ "account": "11010", "month": "2026-09", "page": page }),
    )
}

#[test]
fn each_page_of_a_statement_read_from_pictures_gives_its_own_picture() {
    let (_dir, mut b) = setup("sp-pages");
    let (one, two) = (png(850, 1100, 300), jpeg(640, 480));
    let first = document(&mut b, 1, &one, "image/png");
    let second = document(&mut b, 2, &two, "image/jpeg");
    let out = read_two_pages(&mut b, 1, json!({ "pages": [first, second] }));
    assert_eq!(out["isError"], false, "{out}");
    let p1 = page_no(&mut b, 1);
    assert_eq!(p1["structuredContent"]["page"], 1);
    assert_eq!(
        p1["structuredContent"]["data_base64"],
        json!(html::base64(&one))
    );
    let p2 = page_no(&mut b, 2);
    assert_eq!(p2["structuredContent"]["mime"], "image/jpeg");
    assert_eq!(p2["structuredContent"]["width"], 640);
    assert_eq!(
        p2["structuredContent"]["data_base64"],
        json!(html::base64(&two))
    );
    // with no page named, page 1
    assert_eq!(
        page(&mut b, None)["structuredContent"]["data_base64"],
        json!(html::base64(&one))
    );
    assert_eq!(code(&page_no(&mut b, 3)), "no_snapshot");
    let st = statement(&mut b, Origin::App);
    assert_eq!(st["snapshot_pictures"], json!([1, 2]));
    // a CSV in its place leaves no page picture behind
    let out = call(
        &mut b,
        "books_app_statement_import",
        Origin::App,
        json!({ "client_request_id": "sp-pages-csv-0001", "account": "11010", "csv_base64": books_mini_app::b64::encode(CSV.as_bytes()), "file_name": "synthetic.csv" }),
    );
    assert_eq!(out["isError"], false, "{out}");
    assert_eq!(code(&page_no(&mut b, 2)), "no_snapshot");
    assert_eq!(
        statement(&mut b, Origin::App)["snapshot_pictures"],
        json!([])
    );
}

#[test]
fn a_page_without_a_picture_of_its_own_is_never_given_another_pages() {
    // one picture, lines on two pages: the picture is page 1 only
    let (_dir, mut b) = setup("sp-one-picture");
    let one = png(850, 1100, 300);
    let first = document(&mut b, 1, &one, "image/png");
    let out = read_two_pages(&mut b, 1, json!({ "evidence": first }));
    assert_eq!(out["isError"], false, "{out}");
    assert_eq!(page_no(&mut b, 1)["isError"], false);
    assert_eq!(code(&page_no(&mut b, 2)), "no_snapshot");
    let st = statement(&mut b, Origin::App);
    assert_eq!(st["snapshot_kind"], "image");
    assert_eq!(st["snapshot_pictures"], json!([1]));
    // the preview of page 2 lists its lines and embeds no picture
    let out = call(
        &mut b,
        "books_app_reconcile_document",
        Origin::App,
        json!({ "account": "11010", "month": "2026-09", "kind": "page", "page": 2 }),
    );
    assert_eq!(out["isError"], false, "{out}");
    let text = out["content"][0]["resource"]["text"].as_str().unwrap();
    assert!(text.contains("No picture was kept for this page"));
    assert!(!text.contains("<img"));

    // a PDF's pages are inside the file: no page has a picture to draw
    let (_dir, mut b) = setup("sp-pdf-pages");
    let pdf = document(&mut b, 1, b"%PDF-1.7 synthetic", "application/pdf");
    let out = read_two_pages(&mut b, 1, json!({ "evidence": pdf }));
    assert_eq!(out["isError"], false, "{out}");
    assert_eq!(code(&page_no(&mut b, 1)), "not_supported");
    assert_eq!(code(&page_no(&mut b, 2)), "no_snapshot");
    let st = statement(&mut b, Origin::App);
    assert_eq!(st["snapshot_kind"], "pdf");
    assert_eq!(st["snapshot_pictures"], json!([]));
    // and a PDF is not a page's picture
    assert_eq!(
        code(&read_two_pages(&mut b, 2, json!({ "pages": [pdf] }))),
        "invalid_args"
    );
    assert_eq!(
        code(&read_two_pages(&mut b, 3, json!({ "pages": ["tkt_none"] }))),
        "not_found"
    );
}

#[test]
fn a_later_pages_picture_is_never_drawn_as_page_one() {
    // page 1 has no picture and no document is named: page 2's is page 2's only
    let (_dir, mut b) = setup("sp-no-first");
    let two = jpeg(640, 480);
    let second = document(&mut b, 2, &two, "image/jpeg");
    let out = read_two_pages(&mut b, 1, json!({ "pages": [null, second] }));
    assert_eq!(out["isError"], false, "{out}");
    assert_eq!(code(&page_no(&mut b, 1)), "no_snapshot");
    assert_eq!(code(&page(&mut b, None)), "no_snapshot");
    assert_eq!(
        page_no(&mut b, 2)["structuredContent"]["data_base64"],
        json!(html::base64(&two))
    );
    let st = statement(&mut b, Origin::App);
    // page 2's picture is still something See the page can open
    assert_eq!(st["snapshot"], true);
    assert_eq!(st["snapshot_kind"], Value::Null);
    assert_eq!(st["evidence"], Value::Null);
    assert_eq!(st["snapshot_pictures"], json!([2]));
    // the preview of page 1 embeds no picture; page 2's embeds its own
    let mut preview = |page: u64| {
        let out = call(
            &mut b,
            "books_app_reconcile_document",
            Origin::App,
            json!({ "account": "11010", "month": "2026-09", "kind": "page", "page": page }),
        );
        assert_eq!(out["isError"], false, "{out}");
        out["content"][0]["resource"]["text"]
            .as_str()
            .unwrap()
            .to_string()
    };
    let one = preview(1);
    assert!(one.contains("No picture was kept for this page") && one.contains("Coffee shop"));
    assert!(!one.contains("<img") && !one.contains("Client deposit"));
    assert!(preview(2).contains(&html::base64(&two)));
}

#[test]
fn a_line_without_a_box_still_opens_its_own_page() {
    let (_dir, mut b) = setup("sp-no-box");
    let two = jpeg(640, 480);
    let second = document(&mut b, 2, &two, "image/jpeg");
    let draft = call(
        &mut b,
        "books_statement_draft_lines",
        Origin::App,
        json!({
            "client_request_id": "sp-no-box-draft-1", "account": "11010", "month": "2026-09", "closing": "-1250.50",
            "lines": [{ "date": "2026-09-03", "description": "Coffee shop", "amount": "-1250.50", "page": 1 }],
            "pages": [null, second],
        }),
    );
    assert_eq!(draft["isError"], false, "{draft}");
    let out = call(
        &mut b,
        "books_app_statement_accept",
        Origin::App,
        json!({ "client_request_id": "sp-no-box-accept-1", "account": "11010", "month": "2026-09", "draft": draft["structuredContent"]["draft"] }),
    );
    assert_eq!(out["isError"], false, "{out}");
    // the pane is shown for page 2's picture, and the line is on page 1 with no box
    let st = statement(&mut b, Origin::App);
    assert_eq!(st["snapshot"], true);
    assert_eq!(st["snapshot_pictures"], json!([2]));
    assert_eq!(st["rows"][0]["page"], 1);
    assert_eq!(st["rows"][0]["box"], Value::Null);
    // page 1 opens as its own lines, never page 2's picture
    let out = call(
        &mut b,
        "books_app_reconcile_document",
        Origin::App,
        json!({ "account": "11010", "month": "2026-09", "kind": "page", "page": 1, "line": st["rows"][0]["no"] }),
    );
    assert_eq!(out["isError"], false, "{out}");
    let text = out["content"][0]["resource"]["text"].as_str().unwrap();
    assert!(text.contains("Coffee shop") && !text.contains("<img"));
    // the pane lists every page a line is on, not only the boxed ones
    let source = match apps::find("books").unwrap().source {
        apps::AppSource::Tsx(source) => source,
        apps::AppSource::Native { .. } => panic!("the Books window is TSX"),
    };
    assert!(source.contains(
        "new Set([...(p.rows || []).filter((r: Any) => r.page).map((r: Any) => Number(r.page)), ...pictures()])"
    ));
    assert!(!source.contains("new Set([...paged().map("));
}

#[test]
fn replacing_a_pages_picture_changes_what_the_window_keys_its_pictures_by() {
    let (_dir, mut b) = setup("sp-stamp");
    let (one, two, three) = (png(850, 1100, 300), jpeg(640, 480), png(600, 800, 200));
    let first = document(&mut b, 1, &one, "image/png");
    let second = document(&mut b, 2, &two, "image/jpeg");
    let third = document(&mut b, 3, &three, "image/png");
    let read = |b: &mut Books, n: u8, last: &Value| {
        let out = read_two_pages(b, n, json!({ "evidence": first, "pages": [first, last] }));
        assert_eq!(out["isError"], false, "{out}");
        statement(b, Origin::App)
    };
    let before = read(&mut b, 1, &second);
    // the same pictures read again: the same stamp
    let again = read(&mut b, 2, &second);
    // page 2 replaced, the statement's document unchanged
    let after = read(&mut b, 3, &third);
    assert_eq!(before["evidence"], after["evidence"]);
    assert_eq!(before["snapshot_pictures"], after["snapshot_pictures"]);
    assert_eq!(before["snapshot_stamp"], again["snapshot_stamp"]);
    assert_ne!(before["snapshot_stamp"], after["snapshot_stamp"]);
    assert_eq!(before["snapshot_stamp"].as_str().unwrap().len(), 16);
    assert_eq!(
        page_no(&mut b, 2)["structuredContent"]["data_base64"],
        json!(html::base64(&three))
    );
    // the window keys each kept picture by that stamp
    let source = match apps::find("books").unwrap().source {
        apps::AppSource::Tsx(source) => source,
        apps::AppSource::Native { .. } => panic!("the Books window is TSX"),
    };
    assert!(source.contains("`${p.account}|${p.month}|${p.doc}|${p.stamp}|${String(page())}`"));
    assert!(source.contains("stamp={String(s()?.snapshot_stamp || \"\")}"));
}

/// A small text PDF of letter pages, one line of text on each.
fn pdf(pages: &[&str]) -> Vec<u8> {
    let n = pages.len();
    let kids: String = (0..n).map(|i| format!("{} 0 R ", 4 + 2 * i)).collect();
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        format!("<< /Type /Pages /Kids [{kids}] /Count {n} >>"),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ];
    for (i, text) in pages.iter().enumerate() {
        let stream = format!("BT /F1 24 Tf 72 700 Td ({text}) Tj ET");
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R >>",
            5 + 2 * i
        ));
        objects.push(format!(
            "<< /Length {} >>\nstream\n{stream}\nendstream",
            stream.len()
        ));
    }
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (i, o) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend(format!("{} 0 obj\n{o}\nendobj\n", i + 1).bytes());
    }
    let xref = out.len();
    out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).bytes());
    for o in offsets {
        out.extend(format!("{o:010} 00000 n \n").bytes());
    }
    out.extend(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .bytes(),
    );
    out
}

#[test]
fn a_pdf_statements_pages_are_drawn_when_it_is_kept_and_each_gives_its_picture() {
    let (_dir, mut b) = setup("sp-pdf-drawn");
    let file = pdf(&[
        "Synthetic page one",
        "Synthetic page two",
        "Synthetic page three",
    ]);
    let doc = document(&mut b, 1, &file, "application/pdf");
    // kept with the document, in that write: three pictures, no evidence of their own
    let (pictures, evidence): (i64, i64) = {
        let conn = b.store().unwrap().conn();
        (
            conn.query_row("SELECT COUNT(*) FROM evidence_page", [], |r| r.get(0))
                .unwrap(),
            conn.query_row(
                "SELECT COUNT(*) FROM evidence WHERE mime = 'image/png'",
                [],
                |r| r.get(0),
            )
            .unwrap(),
        )
    };
    assert_eq!((pictures, evidence), (3, 0));
    let out = read_two_pages(&mut b, 1, json!({ "evidence": doc }));
    assert_eq!(out["isError"], false, "{out}");
    let st = statement(&mut b, Origin::App);
    assert_eq!(st["snapshot_kind"], "pdf");
    // every page of the file, the third too though no line is on it
    assert_eq!(st["snapshot_pictures"], json!([1, 2, 3]));
    let mut seen = Vec::new();
    for n in 1..=3 {
        let out = page_no(&mut b, n);
        assert_eq!(out["isError"], false, "{out}");
        let got = &out["structuredContent"];
        assert_eq!(got["mime"], "image/png");
        // a letter page at one and a half pixels a point
        assert_eq!((&got["width"], &got["height"]), (&json!(918), &json!(1188)));
        assert_eq!(got["parts"], 1);
        assert!(out.to_string().len() < 1_048_576);
        let data = got["data_base64"].as_str().unwrap().to_string();
        assert!(data.starts_with("iVBORw0KGgo"), "a PNG");
        assert!(!seen.contains(&data), "each page is its own picture");
        seen.push(data);
    }
    assert_eq!(code(&page_no(&mut b, 4)), "no_snapshot");
    // reading a page draws nothing: the data folder stays byte for byte
    let dir = _dir.0.join("store");
    let before = files(&dir);
    page_no(&mut b, 2);
    statement(&mut b, Origin::App);
    assert_eq!(files(&dir), before);
    // a page the statement read gave its own picture shows that one
    let own = png(850, 1100, 300);
    let ticket = document(&mut b, 2, &own, "image/png");
    let out = read_two_pages(
        &mut b,
        2,
        json!({ "evidence": doc, "pages": [null, ticket] }),
    );
    assert_eq!(out["isError"], false, "{out}");
    assert_eq!(
        page_no(&mut b, 2)["structuredContent"]["data_base64"],
        json!(html::base64(&own))
    );
    assert_eq!(page_no(&mut b, 1)["structuredContent"]["width"], 918);
}

#[test]
fn a_pdf_that_cannot_be_drawn_is_still_kept_and_has_no_picture() {
    let (_dir, mut b) = setup("sp-pdf-undrawn");
    // not a real PDF past its header, and one cut off in the middle
    let whole = pdf(&["Synthetic page"]);
    for (n, bytes) in [
        (1u8, b"%PDF-1.7 synthetic".to_vec()),
        (2, whole[..whole.len() / 2].to_vec()),
    ] {
        document(&mut b, n, &bytes, "application/pdf");
    }
    assert_eq!(
        books_mini_app::page_pictures::pdf_pages(b"%PDF-1.7 synthetic").len(),
        0
    );
}
