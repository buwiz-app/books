//! JCS against the SDK's shared RFC 8785 vectors. The vendored copy
//! (`tests/fixtures/jcs-vectors.json`, from the pinned SDK's `vectors/jcs.json`)
//! always runs; the SDK's own file runs too when `BUWIZ_SDK_DIR` names a
//! checkout, and must equal the vendored copy. The Bot's own vectors
//! (`broker/buwiz-protocol/src/jcs-vectors.json`) are compared the same way
//! when `BUWIZ_BOT_DIR` names a Bot checkout that has the file.

use books_mini_app::jcs::{self, JcsError};
use serde_json::Value;

const VENDORED: &str = include_str!("fixtures/jcs-vectors.json");

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
fn the_vendored_vectors_pass() {
    check(VENDORED, "vendored");
}

#[test]
fn the_sdk_vector_file_passes_when_a_checkout_is_named() {
    let Ok(root) = std::env::var("BUWIZ_SDK_DIR") else {
        eprintln!("BUWIZ_SDK_DIR is not set; the SDK's own vector file is not compared");
        return;
    };
    let path = std::path::Path::new(&root).join("vectors/jcs.json");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("vectors/jcs.json: not found in the SDK checkout at {root}"));
    let n = check(&text, "vectors/jcs.json");
    assert_eq!(
        text, VENDORED,
        "the SDK's vectors/jcs.json differs from tests/fixtures/jcs-vectors.json"
    );
    eprintln!("vectors/jcs.json: {n} vectors pass");
}

#[test]
fn the_bot_vectors_equal_the_vendored_vectors_when_a_bot_checkout_has_them() {
    let Ok(root) = std::env::var("BUWIZ_BOT_DIR") else {
        eprintln!("BUWIZ_BOT_DIR is not set; the Bot's vectors are not compared (not named)");
        return;
    };
    let path = std::path::Path::new(&root).join("broker/buwiz-protocol/src/jcs-vectors.json");
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!(
            "broker/buwiz-protocol/src/jcs-vectors.json: not in the Bot checkout at {root}; not compared"
        );
        return;
    };
    let bot: Value = serde_json::from_str(&text).expect("the Bot's vectors file is JSON");
    let ours: Value = serde_json::from_str(VENDORED).unwrap();
    assert_eq!(
        bot["vectors"], ours["vectors"],
        "the Bot's jcs-vectors.json `vectors` differ from tests/fixtures/jcs-vectors.json"
    );
    eprintln!("the Bot's vectors equal the vendored ones");
}
