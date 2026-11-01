//! The manifest's shape: identity, every app-called tool granted, every
//! Books tool named `books_*`, neutral titles, JCS output.

use books_mini_app::{apps, jcs, manifest, reserved, tools};

/// The tool names an app source passes to `buwiz.call("…")`, read
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

fn tsx_source(app: &apps::App) -> &'static str {
    match app.source {
        apps::AppSource::Tsx(source) => source,
        apps::AppSource::Native { .. } => panic!("{} is native", app.id),
    }
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
    assert_eq!(m["protocol"], "2.0.0");
    assert_eq!(m["id"], "app.buwiz.books");
    assert_eq!(m["short"], "books");
    assert_eq!(m["version"], "0.5.0");
    assert_eq!(m["publisher"]["id"], "app.buwiz");
    assert_eq!(m["publisher"]["key_id"], "gc-2026-01");
    assert!(m.get("signature").is_none() && m.get("needs").is_none());
    // Four routines (weekly review since M10), eight scopes, /books and five templates.
    assert_eq!(m["routines"].as_array().unwrap().len(), 4);
    assert_eq!(m["scopes"].as_array().unwrap().len(), 8);
    assert_eq!(m["commands"].as_array().unwrap().len(), 6);
    assert!(
        m["commands"][0]["description"]
            .as_str()
            .unwrap()
            .starts_with("Open your books"),
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
fn native_apps_have_no_tsx_source() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_books-mini-app"))
        .args(["app-source", "quiet"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("app quiet is native and has no TSX source")
    );
}

#[test]
fn every_app_called_tool_is_granted_and_nothing_more() {
    let m = manifest::unsigned(manifest::RELEASE_KEY_ID);
    let entries = m["apps"].as_array().unwrap();
    assert_eq!(entries.len(), apps::APPS.len());
    for (entry, app) in entries.iter().zip(apps::APPS) {
        assert_eq!(entry["id"], app.id);
        let mut granted: Vec<String> = entry["permissions"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t.as_str().unwrap().to_string())
            .collect();
        granted.sort();
        let mut expected = match app.source {
            apps::AppSource::Tsx(source) => {
                assert_eq!(entry["entry"], format!("buwiz-app://{}", app.id));
                assert_eq!(entry["mime"], "text/x-nsh-tsx");
                assert_eq!(entry["sdk"], "1");
                assert_eq!(entry["fingerprint"], manifest::fingerprint(source));
                called_tools(source)
            }
            apps::AppSource::Native {
                screen,
                abi,
                tools,
                open_apps,
            } => {
                assert!(entry.get("entry").is_none());
                assert!(entry.get("mime").is_none());
                assert!(entry.get("sdk").is_none());
                assert!(entry.get("fingerprint").is_none());
                assert_eq!(entry["native"]["screen"], screen);
                assert_eq!(entry["native"]["abi"], abi);
                assert_eq!(entry["native"].as_object().unwrap().len(), 2);
                if open_apps.is_empty() {
                    assert!(entry["permissions"].get("open").is_none());
                } else {
                    let mut granted_open: Vec<&str> = entry["permissions"]["open"]["apps"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|app| app.as_str().unwrap())
                        .collect();
                    let mut expected_open = open_apps.to_vec();
                    granted_open.sort();
                    expected_open.sort();
                    assert_eq!(granted_open, expected_open, "{}: permissions.open", app.id);
                }
                tools.iter().map(|tool| (*tool).to_string()).collect()
            }
        };
        expected.sort();
        assert_eq!(granted, expected, "{}: permissions.tools", app.id);
        for tool in &granted {
            assert!(tools::find(tool).is_some(), "{tool} is not a Books tool");
        }
    }
    assert_eq!(
        called_tools(tsx_source(apps::find("home").unwrap())),
        ["books_status"]
    );
    let window = called_tools(tsx_source(apps::find("books").unwrap()));
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
        let app_only = tool["_meta"]["buwiz/audience"] == serde_json::json!(["app"]);
        // the one read-only app-only tool: the print page for buwiz.preview
        let app_read = name == "books_owed_document" && !write;
        assert_eq!(
            name.starts_with("books_app_") || app_read,
            app_only,
            "{name}"
        );
        if write && !app_only {
            assert!(
                [
                    "books_entry_draft",
                    "books_owed_draft",
                    "books_evidence_classify",
                    "books_statement_draft_lines",
                ]
                .contains(&name),
                "the assistant may only draft: {name}"
            );
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
        let apps::AppSource::Tsx(source) = app.source else {
            continue;
        };
        let imports: Vec<&str> = source
            .lines()
            .filter(|l| l.trim_start().starts_with("import "))
            .collect();
        assert!(
            imports.len() <= 2,
            "{}: one import each from nsh and buwiz",
            app.id
        );
        for line in imports {
            assert!(
                line.ends_with("from \"nsh\";") || line.ends_with("from \"buwiz\";"),
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
    let source = tsx_source(apps::find("home").unwrap());
    assert!(source.contains("buwiz.fit("));
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

/// evidence-4: the document library's grants stay on its own apps.
#[test]
fn evidence_grants_stay_on_the_evidence_apps() {
    let m = manifest::unsigned(manifest::RELEASE_KEY_ID);
    let entries = m["apps"].as_array().unwrap();
    let entry = |id: &str| {
        entries
            .iter()
            .find(|e| e["id"] == id)
            .unwrap_or_else(|| panic!("no app {id}"))
            .clone()
    };
    let evidence_only = [
        tools::EVIDENCE_KIND,
        tools::EVIDENCE_PREVIEW,
        tools::EVIDENCE_REVIEW,
    ];
    for (e, app) in entries.iter().zip(apps::APPS) {
        let id = app.id;
        let p = &e["permissions"];
        // preview only where a source calls buwiz.preview
        assert_eq!(
            p.get("preview").is_some(),
            matches!(app.source, apps::AppSource::Tsx(s) if s.contains("buwiz.preview(")),
            "{id}: preview grant"
        );
        // evidence attach scope only on the library
        let attaches_evidence = p["attach"]
            .as_array()
            .is_some_and(|a| a.iter().any(|s| s == "evidence"));
        assert_eq!(attaches_evidence, id == "evidence", "{id}: attach");
        // the library's file types only on the library
        let picks_documents = p["pick_file"]["extensions"]
            .as_array()
            .is_some_and(|x| x.iter().any(|s| s == ".pdf"));
        assert_eq!(picks_documents, id == "evidence", "{id}: pick_file");
        if id != "evidence" && id != "books" {
            assert!(p.get("pick_file").is_none(), "{id}: pick_file");
        }
        assert!(p["tools"].as_array().unwrap().len() <= 64, "{id}: tools");
        if id != "evidence" && id != "document" {
            for tool in evidence_only {
                assert!(
                    !p["tools"].as_array().unwrap().iter().any(|t| t == tool),
                    "{id} names {tool}"
                );
            }
        }
    }
    // Books' way in is one more open target; its tool list gains nothing
    let books = entry("books");
    assert_eq!(
        books["permissions"]["open"],
        serde_json::json!({ "apps": ["ledger", "evidence"] })
    );
    // the window's preview is Unpaid's print page, snapshots and the month pack, at the shared cap
    assert_eq!(
        books["permissions"]["preview"],
        serde_json::json!({ "max_bytes": books_mini_app::tools::document_review::PREVIEW_MAX_BYTES })
    );
    let apps::AppSource::Tsx(books_source) = apps::find("books").unwrap().source else {
        panic!("books is a TSX window")
    };
    let window = called_tools(books_source);
    for tool in evidence_only.iter().chain(&[tools::EVIDENCE_CLASSIFY]) {
        assert!(!window.contains(&tool.to_string()), "books names {tool}");
    }
    assert_eq!(
        books["permissions"]["tools"].as_array().unwrap().len(),
        window.len()
    );
    // the inline card: no file picking, no attach, opens only Books
    let card = entry("document");
    let cp = &card["permissions"];
    assert!(cp.get("pick_file").is_none() && cp.get("attach").is_none());
    assert_eq!(cp["open"], serde_json::json!({ "apps": ["books"] }));
    assert_eq!(card["placement"], serde_json::json!(["inline"]));
    // props: opaque tickets and the mode enum, nothing else
    let props = card["props"]["properties"].as_object().unwrap();
    let mut keys: Vec<&str> = props.keys().map(String::as_str).collect();
    keys.sort();
    assert_eq!(keys, ["evidence", "mode", "proposal", "ticket"]);
    for key in ["evidence", "proposal", "ticket"] {
        assert_eq!(props[key]["type"], "string", "{key}");
        assert_eq!(props[key]["maxLength"], 30, "{key}");
        assert!(props[key].get("enum").is_none(), "{key}");
    }
    assert_eq!(props["mode"]["enum"], serde_json::json!(["review"]));
    // at most three inline actions in any one footer
    let apps::AppSource::Tsx(source) = apps::find("document").unwrap().source else {
        panic!("document is a TSX card")
    };
    let card_body = &source[source.find("function DocumentCard").unwrap()..];
    let mut footers = 0;
    for foot in card_body.split("<Foot>").skip(1) {
        let foot = &foot[..foot.find("</Foot>").expect("closed footer")];
        assert!(foot.matches("<Act ").count() <= 3, "{foot}");
        footers += 1;
    }
    assert!(footers >= 1);
    let manifest_apps = entries.len();
    assert!(manifest_apps <= 16, "apps: {manifest_apps}");
    assert!(m["scopes"].as_array().unwrap().len() <= 16);
}
