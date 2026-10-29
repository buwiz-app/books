//! The manifest's shape: identity, every app-called tool granted, every
//! Books tool named `books_*`, neutral titles, JCS output.

use books_mini_app::{apps, jcs, manifest, reserved, tools};

/// The tool names an app source passes to `rox.call("…")`, read
/// independently of `apps::named_tools`: every `"books_…"` literal.
fn called_tools(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (i, _) in source.match_indices("\"books_") {
        let rest = &source[i + 1..];
        let end = rest.find('"').expect("closing quote");
        found.push(rest[..end].to_string());
    }
    found.sort();
    found.dedup();
    found
}

/// A neutral title (D1, RK-10, invariant 13): no digits (amounts, TINs),
/// no currency sign, and short.
fn neutral(title: &str) {
    assert!(
        !title.is_empty() && title.chars().count() <= 60,
        "{title:?}"
    );
    assert!(
        !title.chars().any(|c| c.is_ascii_digit()),
        "a digit in title {title:?}"
    );
    assert!(
        !title.contains(['₱', '$', '€']) && !title.contains("PHP"),
        "a currency in title {title:?}"
    );
}

#[test]
fn the_manifest_has_the_books_identity() {
    let m = manifest::unsigned(manifest::RELEASE_KEY_ID);
    assert_eq!(m["protocol"], "1.0.0");
    assert_eq!(m["id"], "dev.goldcoders.books");
    assert_eq!(m["short"], "books");
    assert_eq!(m["version"], "0.1.0");
    assert_eq!(m["publisher"]["id"], "dev.goldcoders");
    assert_eq!(m["publisher"]["key_id"], "gc-2026-01");
    assert!(m.get("signature").is_none() && m.get("needs").is_none());
    assert!(m.get("routines").is_none(), "no routines yet (M8)");
    assert_eq!(
        m["commands"][0]["description"], "Open your books",
        "M1-10: not the plugin title again"
    );
    // a test run signs under a test key id; nothing else changes
    let test = manifest::unsigned("books-test-01");
    assert_eq!(test["publisher"]["key_id"], "books-test-01");
    assert_eq!(test["apps"], m["apps"]);
}

#[test]
fn the_generator_output_is_jcs_canonical() {
    let text = manifest::unsigned_jcs(manifest::RELEASE_KEY_ID);
    assert_eq!(jcs::canonical_text(&text).unwrap(), text);
    assert!(text.len() < 256 * 1024);
}

#[test]
fn every_app_called_tool_is_granted_and_nothing_more() {
    let m = manifest::unsigned(manifest::RELEASE_KEY_ID);
    let entries = m["apps"].as_array().unwrap();
    assert_eq!(entries.len(), apps::APPS.len());
    for (entry, app) in entries.iter().zip(apps::APPS) {
        assert_eq!(entry["id"], app.id);
        assert_eq!(entry["entry"], format!("rox-app://{}", app.id));
        assert_eq!(entry["mime"], "text/x-nsh-tsx");
        assert_eq!(entry["sdk"], "1");
        assert_eq!(entry["fingerprint"], manifest::fingerprint(app.source));
        let mut granted: Vec<String> = entry["permissions"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t.as_str().unwrap().to_string())
            .collect();
        granted.sort();
        assert_eq!(
            granted,
            called_tools(app.source),
            "{}: permissions.tools",
            app.id
        );
        for tool in &granted {
            assert!(tools::find(tool).is_some(), "{tool} is not a Books tool");
        }
    }
    assert_eq!(
        called_tools(apps::find("home").unwrap().source),
        ["books_status"]
    );
    let window = called_tools(apps::find("books").unwrap().source);
    for tool in [
        "books_app_post_entry",
        "books_app_void_entry",
        "books_entry_draft",
        "books_today",
    ] {
        assert!(
            window.contains(&tool.to_string()),
            "the window calls {tool}"
        );
    }
}

#[test]
fn every_books_tool_starts_with_books_and_every_named_tool_exists() {
    for tool in tools::list() {
        let name = tool["name"].as_str().unwrap();
        assert!(name.starts_with("books_"), "{name} (RK-5)");
        assert!(
            name.len() <= 63
                && name
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        );
        assert!(tool["description"].as_str().unwrap().chars().count() <= 1024);
        let write = !tools::read_only(name);
        assert_eq!(tool["annotations"]["readOnlyHint"], !write, "{name}");
        // DD-20: every write but the draft is the user's click in the app
        let app_only = tool["_meta"]["rox/audience"] == serde_json::json!(["app"]);
        assert_eq!(name.starts_with("books_app_"), app_only, "{name}");
        if write && !app_only {
            assert_eq!(name, "books_entry_draft", "the assistant may only draft");
        }
        // P7 forbidden verbs (invariant 7)
        for verb in ["submit", "queue", "pay", "paid", "file", "send"] {
            assert!(!name.contains(verb), "{name} contains {verb}");
        }
    }
    let m = manifest::unsigned(manifest::RELEASE_KEY_ID);
    let mut named: Vec<&str> = m["commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["resolve"].as_str().unwrap())
        .collect();
    for app in m["apps"].as_array().unwrap() {
        named.extend(
            app["permissions"]["tools"]
                .as_array()
                .unwrap()
                .iter()
                .map(|t| t.as_str().unwrap()),
        );
    }
    for name in named {
        assert!(
            tools::find(name).is_some(),
            "the manifest names {name}, which tools/list lacks"
        );
        assert!(!reserved::RESERVED.contains(&name));
    }
}

#[test]
fn titles_are_neutral() {
    let m = manifest::unsigned(manifest::RELEASE_KEY_ID);
    neutral(m["title"].as_str().unwrap());
    for app in m["apps"].as_array().unwrap() {
        neutral(app["title"].as_str().unwrap());
        assert!(app["description"].as_str().unwrap().chars().count() <= 300);
    }
    neutral(books_mini_app::service::HOME_TITLE);
}

#[test]
fn the_apps_import_only_nsh_and_rox_and_cards_call_rox_fit() {
    for app in apps::APPS {
        let source = app.source;
        let imports: Vec<&str> = source
            .lines()
            .filter(|l| l.trim_start().starts_with("import "))
            .collect();
        assert!(
            imports.len() <= 2,
            "{}: one import each from nsh and rox",
            app.id
        );
        for line in imports {
            assert!(
                line.ends_with("from \"nsh\";") || line.ends_with("from \"rox\";"),
                "{}: {line}",
                app.id
            );
        }
        assert!(source.contains("export default function"), "{}", app.id);
        for word in [
            "fetch(",
            "XMLHttpRequest",
            "WebSocket",
            "localStorage",
            "sessionStorage",
            "parseFloat",
            "toFixed",
        ] {
            assert!(!source.contains(word), "{}: {word}", app.id);
        }
    }
    let source = apps::find("home").unwrap().source;
    assert!(source.contains("rox.fit("));
    for word in [
        "fetch",
        "XMLHttpRequest",
        "WebSocket",
        "localStorage",
        "sessionStorage",
    ] {
        assert!(!source.contains(word), "{word}");
    }
}

#[test]
fn no_float_type_appears_in_the_crates() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
    for krate in ["books-mini-app", "books-core", "books-store"] {
        let dir = format!("{root}/{krate}/src");
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            let text = std::fs::read_to_string(&path).unwrap();
            for ty in ["f32", "f64", "as_f64"] {
                assert!(!text.contains(ty), "{} mentions {ty}", path.display());
            }
        }
    }
}
