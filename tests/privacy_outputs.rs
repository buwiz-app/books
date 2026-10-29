//! The privacy gate's Rust part: on a synthetic book, everything the
//! assistant can read carries no memo, payee, reference or TIN-like run;
//! every ticket title is neutral (P3); the store file holds none of them as
//! plain text. A scanner that catches nothing proves nothing, so the needles
//! are planted first and the app's own view must show them.

use books_mini_app::service::{Books, Origin};
use books_mini_app::tools;
use books_store::{Paths, TempDir};
use serde_json::{Map, Value, json};

fn args(v: Value) -> Map<String, Value> {
    v.as_object().unwrap().clone()
}

/// A run of nine digits, dashes allowed, outside a decimal amount.
fn tin_like(text: &str) -> bool {
    books_core::privacy::tin_like(text)
}

#[test]
fn the_assistant_sees_no_memo_payee_or_tin_and_titles_are_neutral() {
    let dir = TempDir::new("privacy");
    let mut books = Books::new(Paths::in_dir(dir.0.join("store")));
    let call = |books: &mut Books, name: &str, a: Value, origin: Origin| -> Value {
        tools::call(name, &args(a), origin, books).unwrap()
    };
    let memo = "MEMO-SYNTH-privacy-canary";
    let payee = "Synthetic Payee Canary";
    let reference = "REF-SYNTH-canary";
    let made = call(
        &mut books,
        "books_app_book_create",
        json!({ "client_request_id": "privacy-book-01", "label": "Canary Book" }),
        Origin::App,
    );
    assert_eq!(made["isError"], false, "{made}");
    for (i, (form, amount, category)) in [
        ("money_in", "120000.00", "41000"),
        ("money_out", "1350.00", "67200"),
    ]
    .iter()
    .enumerate()
    {
        let r = call(
            &mut books,
            "books_app_post_entry",
            json!({ "client_request_id": format!("privacy-post-{i:02}"), "form": form, "date": "2026-10-05", "amount": amount,
                    "account": "11000", "category": category, "payee": payee, "memo": memo, "reference": reference }),
            Origin::App,
        );
        assert_eq!(r["isError"], false, "{r}");
    }
    let draft = call(
        &mut books,
        "books_entry_draft",
        json!({ "client_request_id": "privacy-draft-1", "form": "money_out", "date": "2026-10-06", "amount": "2499.00",
                "account": "11000", "category": "69999", "payee": payee, "memo": memo }),
        Origin::Agent,
    );
    let entry = draft["structuredContent"]["entry"]
        .as_str()
        .unwrap()
        .to_string();

    // the app's own view shows the needles: the scan below can see them
    let app_view = call(
        &mut books,
        "books_ledger_list",
        json!({ "month": "2026-10" }),
        Origin::App,
    )
    .to_string();
    assert!(app_view.contains(memo) && app_view.contains(payee) && app_view.contains(reference));

    let mut tickets = Vec::new();
    for tool in tools::TOOLS
        .iter()
        .filter(|t| t.reach == tools::Reach::Everyone && !t.write)
    {
        for a in [
            json!({}),
            json!({ "month": "2026-10" }),
            json!({ "entry": entry }),
            json!({ "group": "Assets" }),
        ] {
            let out = call(&mut books, tool.name, a, Origin::Agent);
            let text = out.to_string();
            for needle in [memo, payee, reference] {
                assert!(
                    !text.contains(needle),
                    "{} shows {needle} to the assistant",
                    tool.name
                );
            }
            assert!(
                !tin_like(&text.replace("0123456789", "")),
                "{} carries a TIN-like run",
                tool.name
            );
            collect(&out, &mut tickets);
        }
    }
    tickets.sort();
    tickets.dedup();
    assert!(tickets.len() >= 4, "book, entries and accounts were seen");
    for t in &tickets {
        if let Some(title) = books.ticket_title(t) {
            assert!(title.chars().count() <= 60, "{title}");
            for needle in [memo, payee, reference, "Canary Book", "₱"] {
                assert!(!title.contains(needle), "title {title}");
            }
            assert!(!title.contains('.'), "an amount in title {title}");
        }
    }
    drop(books);
    let bytes = std::fs::read(dir.0.join("store/books.db")).unwrap();
    for needle in [memo, payee, reference, "Canary Book"] {
        assert!(
            !bytes.windows(needle.len()).any(|w| w == needle.as_bytes()),
            "{needle} in the store file"
        );
    }
}

fn collect(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) if books_store::plumbing::is_ticket(s) => out.push(s.clone()),
        Value::Array(a) => a.iter().for_each(|x| collect(x, out)),
        Value::Object(m) => m.values().for_each(|x| collect(x, out)),
        _ => {}
    }
}
