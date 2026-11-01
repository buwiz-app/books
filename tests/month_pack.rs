//! M10 reports-4: the month pack, a static print preview for `buwiz.preview`
//! (synthetic golden book, `fixtures/demo/buwiz-trading-2026.json`).

use books_mini_app::service::{Books, Origin};
use books_mini_app::{demo, manifest, tools};
use books_store::{Paths, TempDir};
use serde_json::{Map, Value, json};

fn raw(books: &mut Books, name: &str, args: Value, origin: Origin) -> Value {
    let args: Map<String, Value> = args.as_object().unwrap().clone();
    tools::call(name, &args, origin, books).unwrap()
}

fn call(books: &mut Books, name: &str, args: Value) -> Value {
    let out = raw(books, name, args, Origin::App);
    assert_eq!(out["isError"], false, "{name}: {out}");
    out["structuredContent"].clone()
}

fn seeded() -> (TempDir, Books) {
    books_core::date::pin_today(books_core::date::Date::new(2026, 10, 29));
    let dir = TempDir::new("month-pack");
    let mut books = Books::new(Paths::in_dir(dir.0.join("store")));
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/demo/buwiz-trading-2026.json"
    ))
    .unwrap();
    let demo_book: Value = serde_json::from_str(&text).unwrap();
    demo::seed(&mut books, &demo_book).unwrap();
    (dir, books)
}

#[test]
fn the_month_pack_is_one_static_html_resource_built_from_the_reports() {
    let (_dir, mut books) = seeded();
    let b = &mut books;
    let made = call(
        b,
        "books_app_account_create",
        json!({ "client_request_id": "pack-acct-0001", "number": "68991", "name": "<script>x</script> & \"q\"", "kind": "unsure", "type": "expense" }),
    );
    assert!(made.is_object());
    // an October line on it, so the name shows in the P&L and the trial balance
    call(
        b,
        "books_app_post_entry",
        json!({ "client_request_id": "pack-post-0001", "form": "money_out", "date": "2026-10-20", "amount": "150.00", "account": "11010", "category": "68991", "payee": "Shop A" }),
    );
    let out = raw(
        b,
        "books_month_pack",
        json!({ "month": "2026-10" }),
        Origin::App,
    );
    assert_eq!(out["isError"], false, "{out}");
    let res = &out["content"][0];
    assert_eq!(res["type"], "resource");
    assert_eq!(res["resource"]["mimeType"], "text/html");
    assert_eq!(
        res["resource"]["uri"],
        "books://print/month-pack-2026-10.html"
    );
    assert_eq!(out["content"].as_array().unwrap().len(), 1);
    let sc = &out["structuredContent"];
    assert_eq!(sc["title"], "Month pack · October 2026");
    assert_eq!(
        sc["month"],
        json!({ "key": "2026-10", "label": "October 2026" })
    );
    assert!(sc.get("html").is_none(), "the html is not duplicated");
    let html = res["resource"]["text"].as_str().unwrap();
    assert_eq!(sc["bytes"], json!(html.len()));
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains("<meta charset=utf-8>"));

    // figures read from the tools, not written here
    let pl = call(
        b,
        "books_report",
        json!({ "kind": "pl", "period": "2026-10" }),
    );
    let tb = call(
        b,
        "books_report",
        json!({ "kind": "tb", "period": "2026-10" }),
    );
    let net = pl["net"]["shown"].as_str().unwrap();
    let debit = tb["net"]["debit_shown"].as_str().unwrap();
    assert!(!net.is_empty() && html.contains(net), "net income {net}");
    assert!(
        !debit.is_empty() && html.contains(debit),
        "tb debit {debit}"
    );
    let grid = call(b, "books_reconcile_grid", json!({}));
    let bank = &grid["rows"][0];
    let cell = bank["cells"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["month"] == "2026-10")
        .unwrap();
    let label = cell["label"].as_str().unwrap();
    let name = bank["name"].as_str().unwrap();
    assert!(
        html.contains(&format!("<tr><td>{name}</td><td>{label}</td></tr>")),
        "reconcile {name} {label}"
    );
    assert!(html.contains("Drafts are not included. Prepared by Books; not a tax return."));

    // static: nothing runs, nothing loads
    for bad in ["<script", "http://", "https://", "<link", "src=\"http"] {
        assert!(!html.contains(bad), "{bad}");
    }
    assert!(html.contains("&lt;script&gt;x&lt;/script&gt; &amp; &quot;q&quot;"));
}

#[test]
fn the_month_pack_refuses_a_bad_month_and_the_assistant() {
    let (_dir, mut books) = seeded();
    for m in ["2026-13", "2026-Q3"] {
        let out = raw(
            &mut books,
            "books_month_pack",
            json!({ "month": m }),
            Origin::App,
        );
        assert_eq!(out["isError"], true, "{m}: {out}");
        assert_eq!(
            out["structuredContent"]["code"], "invalid_args",
            "{m}: {out}"
        );
    }
}

#[test]
fn only_the_window_may_preview_and_the_tool_is_app_only() {
    let m = manifest::unsigned("books-test-01");
    for app in m["apps"].as_array().unwrap() {
        let p = &app["permissions"];
        if app["id"] == "books" {
            assert_eq!(p["preview"], json!({}));
            let names = p["tools"].as_array().unwrap();
            assert!(names.iter().any(|t| t == "books_month_pack"));
            assert!(names.len() <= 64, "{} window tools", names.len());
        } else {
            assert!(p.get("preview").is_none(), "{}", app["id"]);
        }
    }
    let entry = tools::list()
        .into_iter()
        .find(|t| t["name"] == "books_month_pack")
        .unwrap();
    assert_eq!(entry["annotations"]["readOnlyHint"], true);
    assert_eq!(entry["_meta"]["buwiz/audience"], json!(["app"]));
    let t = tools::find("books_month_pack").unwrap();
    assert_eq!(t.reach, tools::Reach::App);
    for verb in ["submit", "queue", "pay", "paid", "file", "send"] {
        assert!(!t.name.contains(verb));
    }
}
