//! JCS against Rox's shared RFC 8785 vectors. The vendored copy
//! (`tests/fixtures/rox-jcs-vectors.json`) always runs; Rox's two originals
//! (`app/src/mini_v1/jcs-vectors.json`, `server/test/fixtures/jcs-vectors.json`)
//! run too when a Rox checkout is present (`ROX_DIR`, default
//! `/Volumes/goldcoders/Projects/rox`), and must equal the vendored copy.

use books_mini_app::jcs::{self, JcsError};
use serde_json::Value;

const VENDORED: &str = include_str!("fixtures/rox-jcs-vectors.json");
const ROX_FILES: [&str; 2] = [
    "app/src/mini_v1/jcs-vectors.json",
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
fn the_vendored_rox_vectors_pass() {
    check(VENDORED, "vendored");
}

#[test]
fn both_rox_vector_files_pass_when_rox_is_checked_out() {
    let root =
        std::env::var("ROX_DIR").unwrap_or_else(|_| "/Volumes/goldcoders/Projects/rox".into());
    for file in ROX_FILES {
        let path = std::path::Path::new(&root).join(file);
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                let n = check(&text, file);
                assert_eq!(
                    text, VENDORED,
                    "{file} differs from tests/fixtures/rox-jcs-vectors.json"
                );
                eprintln!("{file}: {n} vectors pass");
            }
            Err(_) => eprintln!("{file}: no Rox checkout at {root}; skipped"),
        }
    }
}
