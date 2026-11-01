//! M10 parity reconcile-2: books_app_reconcile_document, the static pages
//! buwiz.preview shows (the reconciliation report and one statement page).
//! Synthetic data only. Snapshots use reconcile-1's real migration columns
//! (statement.evidence_id, statement_line page and bx0..by1 thousandths).

use books_core::money;
use books_mini_app::service::{Books, Origin};
use books_mini_app::{apps, demo, manifest, tools};
use books_store::{Paths, TempDir};
use serde_json::{Map, Value, json};

const DEMO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/demo/buwiz-trading-2026.json"
);

const CSV: &str = "Date,Description,Debit,Credit,Balance\n2026-09-03,Coffee shop,\"1,250.50\",,\n2026-09-08,<b>x</b>,,\"3,000.00\",\n2026-09-12,Office supplies,420.25,,\n2026-09-20,Client deposit,,\"15,500.75\",\"50,000.00\"\n";
const IN: i64 = 300_000 + 1_550_075;
const OUT: i64 = 125_050 + 42_025;

fn call(books: &mut Books, origin: Origin, args: Value) -> Value {
    let args: Map<String, Value> = args.as_object().unwrap().clone();
    tools::call("books_app_reconcile_document", &args, origin, books).unwrap()
}

fn setup(name: &str) -> (TempDir, Books) {
    books_core::date::pin_today(books_core::date::Date::new(2026, 10, 29));
    let dir = TempDir::new(name);
    let mut books = Books::new(Paths::in_dir(dir.0.join("store")));
    let demo_book: Value = serde_json::from_str(&std::fs::read_to_string(DEMO).unwrap()).unwrap();
    demo::seed(&mut books, &demo_book).unwrap();
    let args = json!({ "client_request_id": format!("{name}-import-0001"), "account": "11010", "csv_base64": books_mini_app::b64::encode(CSV.as_bytes()), "file_name": "synthetic.csv" });
    let out = tools::call(
        "books_app_statement_import",
        args.as_object().unwrap(),
        Origin::App,
        &mut books,
    )
    .unwrap();
    assert_eq!(out["isError"], false, "{out}");
    (dir, books)
}

fn sql(books: &mut Books, text: &str) {
    books.store().unwrap().conn().execute_batch(text).unwrap();
}

/// The four lines boxed on page 1 (thousandths of the page corners).
fn snapshot(books: &mut Books, bytes: &[u8], mime: &str) {
    sql(
        books,
        "UPDATE statement_line SET page = 1, bx0 = 50, by0 = 100 * line_no, bx1 = 950, by1 = 100 * line_no + 45;",
    );
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

fn html(out: &Value) -> String {
    assert_eq!(out["isError"], false, "{out}");
    let content = out["content"].as_array().unwrap();
    assert_eq!(content.len(), 1);
    assert_eq!(content[0]["type"], "resource");
    assert_eq!(content[0]["resource"]["mimeType"], "text/html");
    let text = content[0]["resource"]["text"].as_str().unwrap().to_string();
    assert!(!text.contains("<script"));
    text
}

fn refusal(out: &Value) -> String {
    assert_eq!(out["isError"], true, "{out}");
    out["structuredContent"]["message"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

#[test]
fn the_report_shows_both_tables_with_exact_totals_and_escapes_descriptions() {
    let (_dir, mut b) = setup("rd-report");
    sql(
        &mut b,
        "UPDATE statement SET status = 'finished', finished_at = '2026-10-01T09:00:00Z'",
    );
    let out = call(
        &mut b,
        Origin::App,
        json!({ "account": "11010", "month": "2026-09", "kind": "report" }),
    );
    let text = html(&out);
    assert_eq!(
        out["content"][0]["resource"]["uri"],
        "books://reconcile/report"
    );
    for want in [
        "Money in",
        "Money out",
        "Finished on 2026-10-01",
        "Coffee shop",
        "Statement ending",
        "Books balance",
        "Difference",
        "Prepared by Books on 2026-10-29",
    ] {
        assert!(text.contains(want), "{want}");
    }
    assert!(text.contains("&lt;b&gt;x&lt;/b&gt;"));
    assert!(!text.contains("<b>x</b>"));
    for total in [IN, OUT] {
        let cell = format!(
            "<td colspan=\"2\">Total</td><td class=\"n\">{}</td>",
            money::display(total)
        );
        assert!(text.contains(&cell), "{cell}");
    }
    let title = out["structuredContent"]["title"].as_str().unwrap();
    assert_eq!(title, "Bank reconciliation · Sep 2026");
    let digits: String = title.chars().filter(char::is_ascii_digit).collect();
    assert_eq!(digits, "2026", "the title carries no digit but the year");
}

#[test]
fn a_month_without_a_statement_is_not_found() {
    let (_dir, mut b) = setup("rd-none");
    let out = call(
        &mut b,
        Origin::App,
        json!({ "account": "11010", "month": "2026-08", "kind": "report" }),
    );
    assert_eq!(out["isError"], true);
    assert_eq!(out["structuredContent"]["code"], "not_found");
}

#[test]
fn a_png_page_boxes_each_line_and_highlights_one() {
    let (_dir, mut b) = setup("rd-png");
    // a synthetic 1x1 PNG
    let png: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 0x0d, 0x49, 0x48, 0x44, 0x52, 0,
        0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 0x1f, 0x15, 0xc4, 0x89,
    ];
    snapshot(&mut b, png, "image/png");
    let out = call(
        &mut b,
        Origin::App,
        json!({ "account": "11010", "month": "2026-09", "kind": "page", "page": 1, "line": 2 }),
    );
    let text = html(&out);
    assert!(text.contains("<img src=\"data:image/png;base64,iVBORw0KGgo"));
    assert_eq!(text.matches("<div class=\"box").count(), 4);
    assert_eq!(text.matches("<div class=\"box hl\"").count(), 1);
    assert!(text.contains("left:5.0%;top:20.0%;width:90.0%;height:4.5%"));
    assert_eq!(
        out["structuredContent"]["title"],
        "Statement page 1 · Sep 2026"
    );
    // page 2 has no lines
    let out = call(
        &mut b,
        Origin::App,
        json!({ "account": "11010", "month": "2026-09", "kind": "page", "page": 2 }),
    );
    assert_eq!(html(&out).matches("<div class=\"box").count(), 0);

    // the window's statement view carries the line numbers, pages and boxes
    let args = json!({ "account": "11010", "month": "2026-09" });
    let view = tools::call(
        "books_statement_get",
        args.as_object().unwrap(),
        Origin::App,
        &mut b,
    )
    .unwrap();
    let st = &view["structuredContent"]["statement"];
    assert_eq!(st["snapshot"], true);
    let row = &st["rows"][1];
    assert_eq!(
        (row["no"].as_i64(), row["page"].as_i64()),
        (Some(2), Some(1))
    );
    assert_eq!(row["box"]["top"], "20.0%");
}

#[test]
fn a_pdf_page_is_an_object_with_its_lines_listed() {
    let (_dir, mut b) = setup("rd-pdf");
    snapshot(&mut b, b"%PDF-1.4 synthetic", "application/pdf");
    let out = call(
        &mut b,
        Origin::App,
        json!({ "account": "11010", "month": "2026-09", "kind": "page" }),
    );
    let text = html(&out);
    assert!(text.contains("<object data=\"data:application/pdf;base64,JVBERi0xLjQg"));
    assert_eq!(text.matches("<li").count(), 4);
    assert!(text.contains("&lt;b&gt;x&lt;/b&gt;"));
}

#[test]
fn a_page_needs_a_snapshot_that_fits_the_preview() {
    let (_dir, mut b) = setup("rd-size");
    let args = json!({ "account": "11010", "month": "2026-09", "kind": "page" });
    assert_eq!(
        refusal(&call(&mut b, Origin::App, args.clone())),
        "This statement has no snapshot"
    );
    let mut big = vec![0u8; 6_100_000];
    big[..8].copy_from_slice(&[0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
    snapshot(&mut b, &big, "image/png");
    assert_eq!(
        refusal(&call(&mut b, Origin::App, args)),
        "The snapshot is too large to show here"
    );
}

#[test]
fn the_size_is_checked_before_the_bytes_are_read() {
    let (_dir, mut b) = setup("rd-precheck");
    snapshot(&mut b, b"%PDF-1.4 synthetic", "application/pdf");
    let args = json!({ "account": "11010", "month": "2026-09", "kind": "page" });
    html(&call(&mut b, Origin::App, args.clone()));
    // the stored length alone is over the cap; the bytes would fit
    sql(&mut b, "UPDATE evidence SET byte_length = 7000000");
    assert_eq!(
        refusal(&call(&mut b, Origin::App, args)),
        "The snapshot is too large to show here"
    );
}

#[test]
fn only_png_jpeg_and_pdf_snapshots_are_shown() {
    let (_dir, mut b) = setup("rd-webp");
    snapshot(&mut b, b"RIFF synthetic WEBP", "image/webp");
    let out = call(
        &mut b,
        Origin::App,
        json!({ "account": "11010", "month": "2026-09", "kind": "page" }),
    );
    assert_eq!(out["isError"], true, "{out}");
    assert_eq!(out["structuredContent"]["code"], "not_supported");
}

#[test]
fn a_zero_amount_line_is_listed_on_its_own() {
    let (_dir, mut b) = setup("rd-zero");
    sql(
        &mut b,
        "INSERT INTO statement_line (statement_id, line_no, line_date, description, amount)
         SELECT statement_id, 5, '2026-09-25', 'Fee waived', 0 FROM statement",
    );
    let text = html(&call(
        &mut b,
        Origin::App,
        json!({ "account": "11010", "month": "2026-09", "kind": "report" }),
    ));
    let zero = text.find("<h2>No amount</h2>").expect("a No amount table");
    assert!(text[zero..].contains("Fee waived"));
    assert_eq!(text.matches("Fee waived").count(), 1);
    let (_d2, mut c) = setup("rd-nozero");
    let text = html(&call(
        &mut c,
        Origin::App,
        json!({ "account": "11010", "month": "2026-09", "kind": "report" }),
    ));
    assert!(!text.contains("No amount"));
}

#[test]
fn the_server_refuses_the_document_tool_to_the_assistant() {
    use books_mini_app::jsonrpc::Server;
    use books_mini_app::manifest::Source;
    use ed25519_dalek::SigningKey;
    let dir = TempDir::new("rd-server");
    let mut server = Server::new(
        Source::default(),
        Books::new(Paths::in_dir(dir.0.join("store"))),
    );
    let mut call = |id: i64, name: &str, args: Value, caller: Value| {
        let mut params = json!({ "name": name, "arguments": args });
        if !caller.is_null() {
            params["_meta"] = json!({ "buwiz/caller": caller });
        }
        server
            .handle(
                &json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call", "params": params }),
            )
            .unwrap()["result"]
            .clone()
    };
    let seed = [11u8; 32];
    let public =
        books_mini_app::b64::encode(SigningKey::from_bytes(&seed).verifying_key().as_bytes());
    let p = call(
        1,
        "buwiz.provision",
        json!({ "protocol": "2.0.0", "plugin": manifest::PLUGIN_ID, "key": { "alg": "ed25519", "seed": books_mini_app::b64::encode(&seed) }, "pub": public }),
        Value::Null,
    );
    assert_eq!(p["isError"], false, "{p}");
    let ts = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap();
    let out = call(
        2,
        "books_app_reconcile_document",
        json!({ "account": "11010", "month": "2026-09", "kind": "report" }),
        json!({ "origin": "agent", "plugin": manifest::PLUGIN_ID, "ts": ts }),
    );
    assert_eq!(out["isError"], true, "{out}");
    assert_eq!(
        out["structuredContent"]["code"], "permission_denied",
        "{out}"
    );
}

#[test]
fn the_assistant_cannot_call_the_document_tool() {
    let (_dir, mut b) = setup("rd-agent");
    let out = call(
        &mut b,
        Origin::Agent,
        json!({ "account": "11010", "month": "2026-09", "kind": "report" }),
    );
    assert_eq!(out["isError"], true, "{out}");
    assert!(!out.to_string().contains("Coffee shop"));
    assert!(tools::read_only("books_app_reconcile_document"));
}

#[test]
fn the_window_grants_preview_and_the_document_tool_within_the_budget() {
    let m = manifest::unsigned(manifest::RELEASE_KEY_ID);
    let window = m["apps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == "books")
        .unwrap();
    assert_eq!(
        window["permissions"]["preview"]["max_bytes"],
        json!(books_mini_app::tools::document_review::PREVIEW_MAX_BYTES)
    );
    let granted = window["permissions"]["tools"].as_array().unwrap();
    assert!(granted.iter().any(|t| t == "books_app_reconcile_document"));
    assert!(granted.len() <= 64, "{} window tools", granted.len());
    assert_eq!(
        apps::named_tools(match apps::find("books").unwrap().source {
            apps::AppSource::Tsx(source) => source,
            apps::AppSource::Native { .. } => panic!("the Books window is TSX"),
        })
        .len(),
        granted.len()
    );
    for app in m["apps"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|a| a["id"] != "books")
    {
        assert!(app["permissions"].get("preview").is_none(), "{}", app["id"]);
    }
}
