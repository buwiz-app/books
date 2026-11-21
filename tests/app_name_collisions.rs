//! Every Books app is one module: `apps.rs` concatenates `kit.tsx` (with its
//! imports) and each page's top-level declarations into a single source. A
//! page that declares a function whose name kit.tsx imports from "nsh" or
//! "nsh/base" does not shadow the import: the call sites mount the imported
//! component with the page's props, which mostly do not fit — an `Avatar`
//! that takes `initials`/`color` instead of `src` renders nothing and the
//! row around it goes blank (the People page's list did). This fails on any
//! redeclared import name, in any file these bundles concatenate.

use std::fs;
use std::path::Path;

const KIT: &str = include_str!("../apps/books/kit.tsx");

/// The names kit.tsx imports from "nsh" and "nsh/base", one `import { … }`
/// line each.
fn imported_names() -> Vec<String> {
    let mut names = Vec::new();
    for line in KIT.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("import {") else {
            continue;
        };
        let Some((list, source)) = rest.split_once('}') else {
            continue;
        };
        let source = source.trim();
        if source != "from \"nsh\";" && source != "from \"nsh/base\";" {
            continue;
        }
        for name in list.split(',') {
            let name = name.trim();
            if !name.is_empty() {
                names.push(name.to_string());
            }
        }
    }
    names
}

/// `function <name>` as a whole word, wherever it is declared.
fn declares(source: &str, name: &str) -> bool {
    let needle = format!("function {name}");
    let mut rest = source;
    while let Some(at) = rest.find(&needle) {
        let after = rest[at + needle.len()..].chars().next();
        if !matches!(after, Some(c) if c.is_alphanumeric() || c == '_' || c == '$') {
            return true;
        }
        rest = &rest[at + needle.len()..];
    }
    false
}

#[test]
fn no_page_redeclares_an_imported_name() {
    let names = imported_names();
    assert!(
        names.contains(&"Avatar".to_string()),
        "kit.tsx imports changed"
    );
    let mut bad = Vec::new();
    for dir in ["apps/books", "apps/cards"] {
        for entry in fs::read_dir(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(dir),
        )
        .unwrap()
        {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("tsx") {
                continue;
            }
            if path.file_name().and_then(|f| f.to_str()) == Some("kit.tsx") {
                continue;
            }
            let source = fs::read_to_string(&path).unwrap();
            for name in &names {
                if declares(&source, name) {
                    bad.push(format!(
                        "{} declares `function {name}`, which kit.tsx imports",
                        path.display()
                    ));
                }
            }
        }
    }
    assert!(
        bad.is_empty(),
        "names colliding with kit.tsx imports:\n{}",
        bad.join("\n")
    );
}
