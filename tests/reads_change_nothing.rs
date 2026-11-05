//! Books' app-only reads run for a screen under the AI's press, so `write: false` has to be
//! strictly true: each leaves the records, the schema and the folder as they were, past
//! its checks and all the way through its body. `books_app_form_lineage` needs a closed quarter,
//! so `forms_bridge` holds it to the same where it has one. Synthetic data only.

use std::collections::BTreeMap;
use std::path::Path;

use books_mini_app::service::{Books, Origin};
use books_mini_app::{demo, tools};
use books_store::{Paths, TempDir};
use serde_json::{Map, Value, json};

const DEMO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/demo/buwiz-trading-2026.json"
);
const CSV: &str = "Date,Description,Debit,Credit,Balance\n2026-09-03,Coffee shop,\"1,250.50\",,\n2026-09-20,Client deposit,,\"15,500.75\",\"50,000.00\"\n";
const PNG: &[u8] = b"\x89PNG\r\n\x1a\nsynthetic-image";

fn call(books: &mut Books, name: &str, args: Value) -> Value {
    let args: Map<String, Value> = args.as_object().unwrap().clone();
    tools::call(name, &args, Origin::App, books).unwrap()
}

fn ok(books: &mut Books, name: &str, args: Value) -> Value {
    let out = call(books, name, args);
    assert_eq!(out["isError"], false, "{name}: {out}");
    out
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

fn versions(books: &mut Books) -> (i64, i64) {
    let c = books.store().unwrap();
    let pragma = |p: &str| c.conn().query_row(p, [], |r| r.get(0)).unwrap();
    (
        pragma("PRAGMA schema_version"),
        pragma("PRAGMA user_version"),
    )
}

/// Every row of every table but `ticket`.
fn rows(books: &mut Books) -> String {
    let c = books.store().unwrap().conn();
    let mut q = c
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name <> 'ticket' ORDER BY name")
        .unwrap();
    let names: Vec<String> = q
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let mut out = String::new();
    for name in names {
        let mut q = c.prepare(&format!("SELECT * FROM \"{name}\"")).unwrap();
        let columns = q.column_count();
        let all = q.query_map([], |r| {
            Ok((0..columns)
                .map(|i| format!("{:?}", r.get_ref(i).unwrap()))
                .collect::<Vec<_>>()
                .join("|"))
        });
        for row in all.unwrap() {
            out.push_str(&format!("{name}: {}\n", row.unwrap()));
        }
    }
    out
}

/// Runs `name`, which has to reach its body, and holds it to changing nothing. A read may hand
/// out a handle for what already exists, once: its first run touches no table but `ticket`, and
/// its second not one byte.
fn unchanged(books: &mut Books, dir: &Path, name: &str, args: Value) -> Value {
    assert!(tools::read_only(name), "{name}");
    let (before, schema) = (rows(books), versions(books));
    ok(books, name, args.clone());
    assert!(rows(books) == before, "{name} changed a table");
    let files_then = files(dir);
    let out = ok(books, name, args);
    assert_eq!(versions(books), schema, "{name} moved the schema");
    let after = files(dir);
    assert_eq!(
        files_then.keys().collect::<Vec<_>>(),
        after.keys().collect::<Vec<_>>(),
        "{name} made or removed a file"
    );
    assert!(files_then == after, "{name} changed the store's bytes");
    out
}

#[test]
fn the_app_only_reads_leave_every_byte_where_it_was() {
    books_core::date::pin_today(books_core::date::Date::new(2026, 10, 29));
    let tmp = TempDir::new("reads-change-nothing");
    let dir = tmp.0.join("store");
    let mut b = Books::new(Paths::in_dir(dir.clone()));
    let book: Value = serde_json::from_str(&std::fs::read_to_string(DEMO).unwrap()).unwrap();
    demo::seed(&mut b, &book).unwrap();
    ok(
        &mut b,
        "books_app_statement_import",
        json!({ "client_request_id": "reads-import-01", "account": "11010", "csv_base64": books_mini_app::b64::encode(CSV.as_bytes()) }),
    );
    ok(
        &mut b,
        tools::EVIDENCE_ATTACH,
        json!({ "client_request_id": "reads-part-0001", "op": "part", "upload": "reads-doc", "part": 0, "parts": 1, "data_base64": books_mini_app::b64::encode(PNG) }),
    );
    let evidence = ok(
        &mut b,
        tools::EVIDENCE_ATTACH,
        json!({ "client_request_id": "reads-commit-01", "op": "commit", "upload": "reads-doc", "mime": "image/png", "kind": "other" }),
    );
    let evidence = evidence["structuredContent"]["evidence"]
        .as_str()
        .unwrap()
        .to_string();
    let owed = ok(
        &mut b,
        "books_app_owed_create",
        json!({ "client_request_id": "reads-owed-0001", "side": "collect", "person_name": "Synthetic Customer", "amount": "450.00", "category": "41000", "issued": "2026-09-01", "due": "2026-09-20" }),
    );
    let owed = owed["structuredContent"]["owed"]
        .as_str()
        .unwrap()
        .to_string();
    let d = &dir;
    unchanged(
        &mut b,
        d,
        "books_app_month_pack",
        json!({ "month": "2026-09" }),
    );
    unchanged(&mut b, d, "books_app_evidence_review", json!({}));
    unchanged(
        &mut b,
        d,
        "books_app_evidence_preview",
        json!({ "evidence": evidence }),
    );
    unchanged(&mut b, d, "books_owed_document", json!({ "owed": owed }));
    unchanged(
        &mut b,
        d,
        "books_app_reconcile_document",
        json!({ "account": "11010", "month": "2026-09", "kind": "report" }),
    );
}

#[test]
fn opening_a_store_that_is_up_to_date_writes_nothing() {
    let tmp = TempDir::new("open-writes-nothing");
    let dir = tmp.0.join("store");
    let paths = Paths::in_dir(dir.clone());
    drop(books_store::Store::open(&paths).unwrap());
    let before = files(&dir);
    drop(books_store::Store::open(&paths).unwrap());
    assert!(before == files(&dir), "opening changed a byte");
}
