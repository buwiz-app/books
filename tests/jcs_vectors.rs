//! JCS against the Buwiz Bot's shared RFC 8785 vectors. The vendored copy
//! (`tests/fixtures/bot-jcs-vectors.json`, from Bot main before protocol 1.0)
//! always runs; the Bot's two originals (`broker/buwiz-protocol/src/jcs-vectors.json`,
//! `server/test/fixtures/jcs-vectors.json`) run too when `BUWIZ_BOT_DIR` names
//! a Bot checkout, and must equal the vendored copy.

use books_mini_app::jcs::{self, JcsError};
use serde_json::Value;

const VENDORED: &str = include_str!("fixtures/bot-jcs-vectors.json");
const BOT_FILES: [&str; 2] = [
    "broker/buwiz-protocol/src/jcs-vectors.json",
    "server/test/fixtures/jcs-vectors.json",
];

/// Every vector: signable ones give the exact JCS text, non-signable ones
/// (fractions, unsafe integers) are refused as such, non-I-JSON is refused.
fn check(text: &str, label: &str) -> usize {
    let all: Value = serde_json::from_str(text).expect("the vectors file is JSON");
    let vectors = all["vectors"].as_array().expect("vectors");
    assert!(vectors.len() >= 15, "{label}: {} vectors", vectors.len());
    for v in vectors {
        let name = v["name"].as_str().unwrap();
        let out = jcs::canonical_text(v["input"].as_str().unwrap());
        if v["error"] == true {
            assert!(
                matches!(out, Err(JcsError::NotIJson(_))),
                "{label} / {name}: {out:?}"
            );
        } else if v["signable"] == true {
            assert_eq!(
                out.as_deref(),
                Ok(v["jcs"].as_str().unwrap()),
                "{label} / {name}"
            );
        } else {
            assert!(
                matches!(out, Err(JcsError::NotSignable(_))),
                "{label} / {name}: {out:?}"
            );
        }
    }
    vectors.len()
}

#[test]
fn the_vendored_bot_vectors_pass() {
    check(VENDORED, "vendored");
}

#[test]
fn both_bot_vector_files_pass_when_a_bot_checkout_is_named() {
    let Ok(root) = std::env::var("BUWIZ_BOT_DIR") else {
        eprintln!("BUWIZ_BOT_DIR is not set; the Bot's own vector files are not compared");
        return;
    };
    for file in BOT_FILES {
        let path = std::path::Path::new(&root).join(file);
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                let n = check(&text, file);
                assert_eq!(
                    text, VENDORED,
                    "{file} differs from tests/fixtures/bot-jcs-vectors.json"
                );
                eprintln!("{file}: {n} vectors pass");
            }
            Err(_) => panic!("{file}: not found in the Bot checkout at {root}"),
        }
    }
}
