//! The Books → eBIRForms hand-off contract, pinned by a shared fixture.
//!
//! `books_handoff_get` answers the exact arguments of eBIRForms'
//! `ebirforms_form_open` (all but the taxpayer's `tin`) and
//! `ebirforms_form_fill`. One hand-off per form, made from the demo book, is
//! kept in `tests/fixtures/handoff/`; eBIRForms keeps the same files in
//! `tests/fixtures/books-handoff/` and fills each into a real draft. Both
//! repos pin [`CONTRACT_SHA256`] over the four files, so a change on either
//! side fails until both carry it. With `EBIRFORMS_DIR` set, this test also
//! compares the files with the eBIRForms checkout's copy.
//!
//! `BOOKS_WRITE_HANDOFF_FIXTURES=1` rewrites the three payloads (keeping each
//! file's inputs hash, which differs per store).

use std::path::{Path, PathBuf};

use books_mini_app::demo;
use books_mini_app::service::{Books, Origin};
use books_mini_app::tools;
use books_store::{Paths, TempDir};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

const DEMO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/demo/buwiz-trading-2026.json"
);

/// The forms, in fixture order: (form, period, form_context, file).
const FORMS: [(&str, &str, &str, &str); 3] = [
    ("2551Q", "2026-Q3", "{}", "2551q.json"),
    ("1601C", "2026-09", "{}", "1601c.json"),
    (
        "1701MS",
        "2026",
        r#"{"/taxpayer/tax_option":"graduated","/taxpayer/deduction":"osd"}"#,
        "1701ms.json",
    ),
];

/// eBIRForms' input schemas for the two tools, as its tools/list answers them.
const SCHEMA_FILE: &str = "form-tools.schema.json";

/// SHA-256 over the four files (names sorted, each `name\0bytes\0`). The same
/// constant is in eBIRForms' `tests/fill_sources.rs`.
const CONTRACT_SHA256: &str = "101d25a3f8197376de3368122ce23de1accfb394df444356317da7cbda74be77";

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/handoff")
}

fn call(books: &mut Books, origin: Origin, name: &str, args: Value) -> Value {
    let args: Map<String, Value> = args.as_object().unwrap().clone();
    tools::call(name, &args, origin, books).unwrap()
}

fn ok(books: &mut Books, origin: Origin, name: &str, args: Value) -> Value {
    let out = call(books, origin, name, args);
    assert_eq!(out["isError"], false, "{name}: {out}");
    out["structuredContent"].clone()
}

/// The demo book with every 2026 month closed and every return acknowledged.
fn demo_books(dir: &TempDir) -> Books {
    books_core::date::pin_today(books_core::date::Date::new(2027, 1, 20));
    let mut books = Books::new(Paths::in_dir(dir.0.join("store")));
    let demo_book: Value = serde_json::from_str(&std::fs::read_to_string(DEMO).unwrap()).unwrap();
    demo::seed(&mut books, &demo_book).unwrap();
    for month in 1..=12 {
        let m = format!("2026-{month:02}");
        // a draft the demo leaves in a month is the owner's to discard
        let ledger = ok(
            &mut books,
            Origin::App,
            "books_ledger_list",
            json!({ "month": m, "filter": "all" }),
        );
        for (i, r) in ledger["rows"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["status"] == "draft")
            .enumerate()
        {
            ok(
                &mut books,
                Origin::App,
                "books_app_discard_entry",
                json!({ "client_request_id": format!("discard-{m}-{i:04}"), "entry": r["ticket"] }),
            );
        }
        ok(
            &mut books,
            Origin::App,
            "books_app_month_close",
            json!({ "client_request_id": format!("close-{m}"), "month": m }),
        );
    }
    books
}

/// What eBIRForms takes from a hand-off: `{form_open, form_fill}`.
fn handed(books: &mut Books, form: &str, period: &str, context: &str) -> Value {
    let context: Value = serde_json::from_str(context).unwrap();
    let view = ok(
        books,
        Origin::App,
        "books_form_proposal",
        json!({ "form": form, "period": period, "form_context": context }),
    );
    for r in view["returns"].as_array().into_iter().flatten() {
        ok(
            books,
            Origin::App,
            "books_app_return_ok",
            json!({ "client_request_id": format!("ack-{form}-{}", r["ticket"].as_str().unwrap_or_default().len()), "entry": r["ticket"] }),
        );
    }
    let made = call(
        books,
        Origin::App,
        "books_app_handoff_create",
        json!({ "client_request_id": format!("handoff-{form}"), "form": form, "period": period, "form_context": context }),
    );
    assert_eq!(made["isError"], false, "{form} {period}: {made}\n{view}");
    let ticket = made["structuredContent"]["ticket"].clone();
    let got = ok(
        books,
        Origin::Agent,
        "books_handoff_get",
        json!({ "handoff": ticket }),
    );
    json!({ "form_open": got["form_open"], "form_fill": got["form_fill"] })
}

fn pretty(value: &Value) -> String {
    let mut text = serde_json::to_string_pretty(value).unwrap();
    text.push('\n');
    text
}

fn contract_sha256(dir: &Path) -> String {
    let mut names: Vec<&str> = FORMS.iter().map(|f| f.3).collect();
    names.push(SCHEMA_FILE);
    names.sort_unstable();
    let mut hash = Sha256::new();
    for name in names {
        hash.update(name.as_bytes());
        hash.update([0]);
        hash.update(std::fs::read(dir.join(name)).unwrap());
        hash.update([0]);
    }
    hash.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// `value` fits `schema`: the JSON Schema keywords eBIRForms' tools use.
fn check(schema: &Value, value: &Value, at: &str) -> Result<(), String> {
    let kind = |v: &Value| match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(n) if n.is_i64() || n.is_u64() => "integer",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    };
    let fits = |want: &str| want == kind(value) || (want == "number" && kind(value) == "integer");
    match &schema["type"] {
        Value::String(want) if !fits(want) => return Err(format!("{at}: not a {want}")),
        Value::Array(wants) if !wants.iter().filter_map(Value::as_str).any(fits) => {
            return Err(format!("{at}: none of {wants:?}"));
        }
        _ => {}
    }
    if let Some(options) = schema["enum"].as_array()
        && !options.contains(value)
    {
        return Err(format!("{at}: {value} is not one of {options:?}"));
    }
    if let Some(text) = value.as_str() {
        let len = text.chars().count() as u64;
        if schema["minLength"].as_u64().is_some_and(|min| len < min)
            || schema["maxLength"].as_u64().is_some_and(|max| len > max)
        {
            return Err(format!("{at}: length {len}"));
        }
        match schema["pattern"].as_str() {
            None => {}
            Some("^[0-9a-f]{64}$") => {
                if text.len() != 64
                    || !text
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                {
                    return Err(format!("{at}: not 64 lowercase hex digits"));
                }
            }
            Some(other) => return Err(format!("{at}: this check has no pattern {other}")),
        }
    }
    if let Some(n) = value.as_f64()
        && (schema["minimum"].as_f64().is_some_and(|min| n < min)
            || schema["maximum"].as_f64().is_some_and(|max| n > max))
    {
        return Err(format!("{at}: {n} out of range"));
    }
    if let Some(map) = value.as_object() {
        for key in schema["required"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if !map.contains_key(key) {
                return Err(format!("{at}: missing {key}"));
            }
        }
        if schema["minProperties"]
            .as_u64()
            .is_some_and(|min| (map.len() as u64) < min)
        {
            return Err(format!("{at}: too few properties"));
        }
        for (key, item) in map {
            let at = format!("{at}.{key}");
            match (
                schema["properties"].get(key),
                &schema["additionalProperties"],
            ) {
                (Some(inner), _) => check(inner, item, &at)?,
                (None, Value::Bool(false)) => return Err(format!("{at}: not allowed")),
                (None, extra @ Value::Object(_)) => check(extra, item, &at)?,
                (None, _) => {}
            }
        }
    }
    Ok(())
}

#[test]
fn each_hand_off_is_exactly_what_ebirforms_takes() {
    let temp = TempDir::new("handoff-contract");
    let mut books = demo_books(&temp);
    let write = std::env::var_os("BOOKS_WRITE_HANDOFF_FIXTURES").is_some();
    let schemas: Value =
        serde_json::from_str(&std::fs::read_to_string(dir().join(SCHEMA_FILE)).unwrap()).unwrap();
    for (form, period, context, file) in FORMS {
        let got = handed(&mut books, form, period, context);
        let path = dir().join(file);
        if write && !path.exists() {
            std::fs::write(&path, pretty(&got)).unwrap();
        }
        let kept =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        // the inputs hash covers the store's own id, so each store's differs:
        // compare the rest exactly
        let hash = got["form_fill"]["source_ref"]["inputs_hash"]
            .as_str()
            .unwrap();
        assert!(
            hash.len() == 64
                && hash
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "{form}: {hash}"
        );
        let kept_value: Value = serde_json::from_str(&kept).unwrap();
        let mut same = got.clone();
        same["form_fill"]["source_ref"]["inputs_hash"] =
            kept_value["form_fill"]["source_ref"]["inputs_hash"].clone();
        if write {
            // a rewrite keeps the stored hash, so only a real change moves the contract
            std::fs::write(&path, pretty(&same)).unwrap();
            continue;
        }
        assert_eq!(
            pretty(&same),
            kept,
            "{form}: the hand-off changed; see the module doc"
        );
        // what eBIRForms takes: its form_open (the taxpayer's ticket added) and form_fill
        let mut open = got["form_open"].clone();
        open["tin"] = json!("a taxpayer ticket");
        check(&schemas["ebirforms_form_open"], &open, "form_open")
            .unwrap_or_else(|e| panic!("{form}: {e}"));
        check(
            &schemas["ebirforms_form_fill"],
            &got["form_fill"],
            "form_fill",
        )
        .unwrap_or_else(|e| panic!("{form}: {e}"));
        // and the boxes are the hand-off's own: input keys, decimal text, source document
        assert_eq!(got["form_fill"]["source"], "document", "{form}");
        assert_eq!(
            got["form_fill"]["source_ref"]["plugin"], "app.buwiz.books",
            "{form}"
        );
        assert!(
            got["form_fill"]["fields"]
                .as_object()
                .unwrap()
                .values()
                .all(Value::is_string),
            "{form}"
        );
    }
    assert_eq!(
        contract_sha256(&dir()),
        CONTRACT_SHA256,
        "the hand-off contract files changed: give eBIRForms the same files and update CONTRACT_SHA256 in both repos"
    );
    if let Some(ebirforms) = std::env::var_os("EBIRFORMS_DIR") {
        let theirs = Path::new(&ebirforms).join("tests/fixtures/books-handoff");
        assert_eq!(
            contract_sha256(&theirs),
            contract_sha256(&dir()),
            "eBIRForms' copy of the hand-off fixtures differs"
        );
    }
}

#[test]
fn the_schema_check_refuses_what_ebirforms_would() {
    let schemas: Value =
        serde_json::from_str(&std::fs::read_to_string(dir().join(SCHEMA_FILE)).unwrap()).unwrap();
    let fill = &schemas["ebirforms_form_fill"];
    let good = json!({ "fields": { "txtATCAmt1": "1.00" }, "source": "document", "source_ref": { "plugin": "app.buwiz.books", "handoff": "1", "inputs_hash": "a".repeat(64) } });
    assert_eq!(check(fill, &good, "f"), Ok(()));
    for (bad, why) in [
        (json!({ "fields": {} }), "too few"),
        (
            json!({ "fields": { "k": "1" }, "source": "books" }),
            "not one of",
        ),
        (
            json!({ "fields": { "k": "1" }, "boxes": [] }),
            "not allowed",
        ),
        (
            json!({ "fields": { "k": "1" }, "source_ref": { "plugin": "app.buwiz.books", "handoff": "1" } }),
            "missing inputs_hash",
        ),
        (
            json!({ "fields": { "k": "1" }, "source_ref": { "plugin": "app.buwiz.books", "handoff": "1", "inputs_hash": "A".repeat(64) } }),
            "hex",
        ),
        (json!({ "fields": { "k": [] } }), "none of"),
    ] {
        let refused = check(fill, &bad, "f").unwrap_err();
        assert!(refused.contains(why), "{bad}: {refused}");
    }
    assert!(
        check(
            &schemas["ebirforms_form_open"],
            &json!({ "code": "2551Q", "year": 1999 }),
            "o"
        )
        .is_err()
    );
}
