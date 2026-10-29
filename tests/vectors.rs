//! The PoC vectors (`fixtures/vectors/`), reproduced by the Rust engine
//! byte for byte (M6-05). Port of the vector half of
//! `poc/test/vectors.test.ts`.
//!
//! For each of the 16 case vectors: the Rust proposal built from the
//! vector's `input`, `form`, `period` and `form_context` has the same
//! canonical JSON as `expected`; the input's canonical SHA-256 is the
//! vector's `fixture_sha256`; and the whole vector, re-emitted with the
//! Rust proposal in place of `expected`, equals the file's bytes. For
//! `money-and-mapping.json`: every money text round trip goes through
//! `money::parse` and `money::format`, every item-17 case through the Rust
//! split, and the file re-emitted from the Rust results equals its bytes.

use std::path::PathBuf;

use books_core::bridge::nontaxable_thirteenth_centavos;
use books_core::canonical::{canonical, pretty, sha256_of};
use books_core::input::{FormCode, NormInput, form_context_from_value, normalize_fixture};
use books_core::money;
use books_core::proposal::{build_proposal, only_profile_id, scope_input};
use books_core::rules::shared;
use serde_json::Value;

const CASES: [&str; 16] = [
    "a-1601c-2026-09",
    "a-1701ms-2026-eight",
    "a-1701ms-2026",
    "a-2551q-2026-Q3",
    "b-1601c-2026-09",
    "b-1701ms-2026",
    "b-2551q-2026-Q3",
    "c-2551q-2026-Q2",
    "c-2551q-2026-Q3",
    "c-ack-2551q-2026-Q3",
    "d-1701ms-2026",
    "d-2551q-2026-Q3",
    "e-2551q-2026-Q3",
    "f-2551q-2026-Q3",
    "g-2551q-2026-Q3",
    "g-overpayment-2551q-2026-Q3",
];

fn vectors_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/vectors")
}

fn read(name: &str) -> (String, Value) {
    let path = vectors_dir().join(format!("{name}.json"));
    let bytes =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let value = serde_json::from_str(&bytes).unwrap();
    (bytes, value)
}

fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("{key} is not text"))
}

fn check_case(name: &str) {
    let data = shared().unwrap();
    let (bytes, vector) = read(name);
    assert_eq!(text(&vector, "vector_format"), "buwiz-poc-vector/3");
    assert_eq!(text(&vector, "vector_id"), name);

    let input = NormInput::from_value(&vector["input"]).unwrap();
    // The typed input writes back to the same JSON it was read from.
    assert_eq!(
        canonical(&input.to_value()).unwrap(),
        canonical(&vector["input"]).unwrap()
    );
    let form = FormCode::parse(text(&vector, "form")).unwrap();
    let ctx = form_context_from_value(&vector["form_context"]).unwrap();
    let proposal = build_proposal(data, &input, form, text(&vector, "period"), &ctx, None).unwrap();
    let rust = proposal.to_value();

    // (a) the proposal, canonical form, byte for byte.
    assert_eq!(
        canonical(&rust).unwrap(),
        canonical(&vector["expected"]).unwrap(),
        "{name}: proposal"
    );
    // (b) the input digest.
    assert_eq!(
        sha256_of(&input.to_value()).unwrap(),
        text(&vector, "fixture_sha256"),
        "{name}: fixture_sha256"
    );
    assert_eq!(proposal.fixture_sha256, text(&vector, "fixture_sha256"));
    // (c) the whole file, re-emitted with the Rust proposal.
    let mut emitted = vector.clone();
    emitted["expected"] = rust;
    assert_eq!(pretty(&emitted).unwrap(), bytes, "{name}: file bytes");
}

macro_rules! case_tests {
    ($($test:ident => $name:expr),* $(,)?) => {
        $(
            #[test]
            fn $test() {
                check_case($name);
            }
        )*
    };
}

case_tests! {
    vector_a_1601c_2026_09 => CASES[0],
    vector_a_1701ms_2026_eight => CASES[1],
    vector_a_1701ms_2026 => CASES[2],
    vector_a_2551q_2026_q3 => CASES[3],
    vector_b_1601c_2026_09 => CASES[4],
    vector_b_1701ms_2026 => CASES[5],
    vector_b_2551q_2026_q3 => CASES[6],
    vector_c_2551q_2026_q2 => CASES[7],
    vector_c_2551q_2026_q3 => CASES[8],
    vector_c_ack_2551q_2026_q3 => CASES[9],
    vector_d_1701ms_2026 => CASES[10],
    vector_d_2551q_2026_q3 => CASES[11],
    vector_e_2551q_2026_q3 => CASES[12],
    vector_f_2551q_2026_q3 => CASES[13],
    vector_g_2551q_2026_q3 => CASES[14],
    vector_g_overpayment_2551q_2026_q3 => CASES[15],
}

/// `runMoneyCase()` and `buildMoneyVectors()`: every field the engine can
/// produce is recomputed in Rust before the file is re-emitted.
#[test]
fn vector_money_and_mapping() {
    let data = shared().unwrap();
    let (bytes, vector) = read("money-and-mapping");
    let rules = data.rules(FormCode::F1601C);
    let rule = rules
        .mappings
        .iter()
        .find(|m| m.id == "1601C.II.17")
        .unwrap();
    let cap_text = &rule.aggregation_params.as_ref().unwrap()["annual_cap_text"];
    let cap = money::parse(cap_text).unwrap();

    let mut emitted = vector.clone();
    emitted["rules_sources"]["rule_set_id"] = Value::String(rules.rule_set_id.clone());
    let cases = emitted["cases"].as_array_mut().unwrap();
    let (mut round_trips, mut splits) = (0, 0);
    for case in cases.iter_mut() {
        match text(case, "op") {
            "money_text_round_trip" => {
                let centavos = money::parse(text(case, "text")).unwrap();
                case["value_centavos"] = Value::String(centavos.to_string());
                case["text"] = Value::String(money::format(centavos));
                round_trips += 1;
            }
            "nontaxable_13th_month_cap" => {
                let int = |key: &str| text(case, key).parse::<i128>().unwrap();
                let split = nontaxable_thirteenth_centavos(
                    int("paid_centavos"),
                    int("paid_earlier_centavos"),
                    i128::from(cap),
                );
                case["cap_centavos"] = Value::String(cap.to_string());
                case["nontaxable_centavos"] = Value::String(split.to_string());
                splits += 1;
            }
            other => panic!("unknown money case op {other}"),
        }
    }
    assert_eq!((round_trips, splits), (6, 4));
    assert_eq!(canonical(&emitted).unwrap(), canonical(&vector).unwrap());
    assert_eq!(pretty(&emitted).unwrap(), bytes);
}

/// `normalizeFixture()` and `scopeInput()`: each vector's `input` is the
/// normalized, profile-scoped form of the synthetic fixture it names.
#[test]
fn every_vector_input_is_its_fixture_normalized() {
    let data = shared().unwrap();
    for name in CASES {
        let (_, vector) = read(name);
        let path = vectors_dir().join(format!("../synthetic/{}.json", text(&vector, "fixture")));
        let raw: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let input = normalize_fixture(&raw, &data.vocab).unwrap();
        let scope = scope_input(&input, &only_profile_id(&input).unwrap()).unwrap();
        assert_eq!(
            canonical(&scope.to_value()).unwrap(),
            canonical(&vector["input"]).unwrap(),
            "{name}"
        );
    }
}

/// The directory holds exactly the 16 case vectors and the money vector.
#[test]
fn the_vector_directory_holds_exactly_these_files() {
    let mut names: Vec<String> = std::fs::read_dir(vectors_dir())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.ends_with(".json"))
        .collect();
    names.sort();
    let mut expected: Vec<String> = CASES.iter().map(|c| format!("{c}.json")).collect();
    expected.push("money-and-mapping.json".to_string());
    expected.sort();
    assert_eq!(names, expected);
}
