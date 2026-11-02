//! The privacy gate's repo part (AGENTS invariant 3): a scanner for
//! TIN-like runs, ticket values and memo markers, its self-test with
//! canaries made at run time, and its scan of every JSON file under
//! `data/`, `fixtures/` and `tests/fixtures/`, the app sources, every
//! vector, every synthetic fixture and every proposal the engine builds
//! from them. A scanner that cannot catch a planted canary proves nothing,
//! so the canaries go first. No TIN-shaped value or ticket value is
//! written in this file; the canaries exist only in memory.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use books_core::canonical::{canonical, sha256_hex};
use books_core::input::{FormCode, FormContext, form_context_from_value, normalize_fixture};
use books_core::proposal::build_proposal;
use books_core::rules::shared;
use serde_json::Value;

const MEMO_NEEDLES: [&str; 1] = ["MEMO-"];
const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

// ---- the scanner ---------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    TinLike,
    Ticket,
    Needle,
}

#[derive(Debug)]
struct Finding {
    path: String,
    kind: Kind,
}

fn digits_at(b: &[u8], i: usize) -> Option<usize> {
    (b.len() >= i + 3 && b[i..i + 3].iter().all(u8::is_ascii_digit)).then_some(i + 3)
}

/// `/\d{3}-?\d{3}-?\d{3}/`, anywhere in the text.
fn tin_like(text: &str) -> bool {
    let b = text.as_bytes();
    let dash = |i: usize| if b.get(i) == Some(&b'-') { i + 1 } else { i };
    (0..b.len()).any(|start| {
        digits_at(b, start)
            .map(dash)
            .and_then(|i| digits_at(b, i))
            .map(dash)
            .and_then(|i| digits_at(b, i))
            .is_some()
    })
}

/// A whole 40-hex commit id or 64-hex SHA-256 digest.
fn is_digest(text: &str) -> bool {
    (text.len() == 40 || text.len() == 64)
        && text
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

/// `/^-?\d+$/`.
fn is_integer_text(text: &str) -> bool {
    let body = text.strip_prefix('-').unwrap_or(text);
    !body.is_empty() && body.bytes().all(|c| c.is_ascii_digit())
}

/// `tkt_` plus 26 Crockford base-32 characters: a real ticket value.
fn has_ticket_value(text: &str) -> bool {
    let b = text.as_bytes();
    text.match_indices("tkt_")
        .any(|(i, _)| b.len() >= i + 30 && b[i + 4..i + 30].iter().all(|c| CROCKFORD.contains(c)))
}

/// The scanner. Findings: `tin_like` (a 3-3-3 digit
/// run, dashed or not), `ticket` (the prefix `tkt_`), `needle` (a
/// caller-supplied marker). Exempt: a whole string that is a 40-hex or
/// 64-hex digest; and, from the TIN check only, plain integer text under a
/// key ending `_centavos` (₱3,000,000.00 is a nine-digit centavos value).
/// Keys are scanned too.
fn scan(value: &Value, needles: &[&str]) -> Vec<Finding> {
    let mut found = Vec::new();
    walk(value, "$", "", needles, &mut found);
    found
}

fn check_text(text: &str, path: &str, is_money: bool, needles: &[&str], out: &mut Vec<Finding>) {
    let mut push = |kind| {
        out.push(Finding {
            path: path.to_string(),
            kind,
        });
    };
    if !is_money && tin_like(text) {
        push(Kind::TinLike);
    }
    if text.contains("tkt_") {
        push(Kind::Ticket);
    }
    for needle in needles {
        if !needle.is_empty() && text.contains(needle) {
            push(Kind::Needle);
        }
    }
}

fn walk(node: &Value, path: &str, key: &str, needles: &[&str], out: &mut Vec<Finding>) {
    match node {
        Value::String(s) => {
            if !is_digest(s) {
                let money = key.ends_with("_centavos") && is_integer_text(s);
                check_text(s, path, money, needles, out);
            }
        }
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                walk(item, &format!("{path}[{i}]"), "", needles, out);
            }
        }
        Value::Object(map) => {
            for (k, child) in map {
                check_text(k, &format!("{path}.<key:{k}>"), false, needles, out);
                walk(child, &format!("{path}.{k}"), k, needles, out);
            }
        }
        _ => {}
    }
}

fn kinds(found: &[Finding]) -> Vec<Kind> {
    found.iter().map(|f| f.kind).collect()
}

fn scan_obj(key: &str, text: &str) -> Vec<Kind> {
    let mut map = serde_json::Map::new();
    map.insert(key.to_string(), Value::String(text.to_string()));
    kinds(&scan(&Value::Object(map), &[]))
}

// ---- run-time canaries -----------------------------------------------------

/// A random u64 from the std hasher seed and the clock (std only).
fn random_u64(salt: u64) -> u64 {
    let mut h = RandomState::new().build_hasher();
    h.write_u64(salt);
    h.write_u128(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    );
    h.finish()
}

struct Canaries {
    tin: String,
    ticket: String,
    memo: String,
}

fn canaries() -> Canaries {
    let low = 10u64.pow(8);
    let tin = (low + random_u64(1) % (9 * low)).to_string();
    let ticket_body: String = (0..26u64)
        .map(|i| char::from(CROCKFORD[(random_u64(100 + i) % 32) as usize]))
        .collect();
    Canaries {
        tin,
        ticket: format!("tkt_{ticket_body}"),
        memo: format!("MEMO-CANARY-{:012x}", random_u64(2) & 0xffff_ffff_ffff),
    }
}

fn dashed(tin: &str) -> String {
    format!("{}-{}-{}", &tin[0..3], &tin[3..6], &tin[6..])
}

/// Nine digits in a row, built at run time.
fn nine_digit_run() -> String {
    (1..=9u8).map(|d| char::from(b'0' + d)).collect()
}

// ---- the scanner's self-test -------------------------------------------------

#[test]
fn a_planted_nine_digit_tin_shaped_run_is_found_dashed_or_not() {
    let c = canaries();
    assert_eq!(c.tin.len(), 9);
    assert!(scan_obj("note", &format!("value {}", c.tin)).contains(&Kind::TinLike));
    assert!(scan_obj("note", &dashed(&c.tin)).contains(&Kind::TinLike));
}

#[test]
fn the_centavos_exemption_is_narrow_integer_text_under_a_centavos_key_only() {
    let c = canaries();
    let three_million_pesos = (3 * 10i64.pow(8)).to_string();
    assert!(scan_obj("base_centavos", &three_million_pesos).is_empty());
    assert!(scan_obj("base", &c.tin).contains(&Kind::TinLike));
    assert!(scan_obj("base_centavos", &dashed(&c.tin)).contains(&Kind::TinLike));
    assert!(scan_obj("base_centavos", &c.ticket).contains(&Kind::Ticket));
}

#[test]
fn a_planted_ticket_value_is_found() {
    let c = canaries();
    assert!(scan_obj("title", &c.ticket).contains(&Kind::Ticket));
    assert!(has_ticket_value(&c.ticket));
}

#[test]
fn a_planted_memo_marker_is_found_when_it_is_a_needle() {
    let c = canaries();
    let v = serde_json::json!({ "memo": c.memo });
    assert!(kinds(&scan(&v, &[&c.memo])).contains(&Kind::Needle));
}

#[test]
fn a_finding_inside_a_key_is_found_too() {
    let c = canaries();
    let mut map = serde_json::Map::new();
    map.insert(format!("k{}", c.tin), Value::String("x".into()));
    assert!(!scan(&Value::Object(map), &[]).is_empty());
}

#[test]
fn peso_amounts_with_up_to_seven_integer_digits_are_not_flagged() {
    let v = serde_json::json!({"fill": {"txtATCAmt1": "3000000.00", "x": "290000.00", "y": "-1000.00"}});
    assert!(scan(&v, &[]).is_empty());
}

#[test]
fn a_whole_40_hex_commit_id_or_64_hex_digest_is_exempt_even_with_a_digit_run() {
    let commit = format!("{}{}", nine_digit_run(), "a".repeat(31));
    assert_eq!(commit.len(), 40);
    assert!(tin_like(&commit));
    assert!(scan_obj("schema_pin", &commit).is_empty());
    assert!(scan_obj("fixture_sha256", &sha256_hex("anything")).is_empty());
}

#[test]
fn the_digest_exemption_is_narrow_a_digest_with_other_text_is_scanned() {
    let digest = sha256_hex("narrow exemption");
    assert!(scan_obj("input_sha256", &digest).is_empty());
    assert!(scan_obj("note", &format!("{digest} {}", nine_digit_run())).contains(&Kind::TinLike));
}

#[test]
fn known_limit_a_peso_amount_of_100000000_00_or_more_is_flagged() {
    let hundred_million = format!("{}.00", 10i64.pow(8));
    assert!(scan_obj("txtATCAmt1", &hundred_million).contains(&Kind::TinLike));
}

// ---- the repo scan -----------------------------------------------------------

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn files(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries {
        let path = e.unwrap().path();
        if path.is_dir() {
            files(&path, ext, out);
        } else if path.extension().and_then(|x| x.to_str()) == Some(ext) {
            out.push(path);
        }
    }
}

fn read_json(path: &Path) -> Value {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn every_json_file_under_data_fixtures_and_test_fixtures_is_clean() {
    let mut paths = Vec::new();
    for dir in ["data", "fixtures", "tests/fixtures"] {
        files(&repo().join(dir), "json", &mut paths);
    }
    assert!(paths.len() > 20, "{} files", paths.len());
    let mut findings = Vec::new();
    for path in &paths {
        let value = read_json(path);
        // The prefix alone is allowed (names such as a ticket field); a
        // whole ticket value anywhere in the file is not.
        let real_ticket = has_ticket_value(&value.to_string());
        // The RFC 8785 vectors shared byte for byte with the Bot are number
        // tests (safe-integer bounds, the RFC 3.2.2 sample, a unix-ms
        // timestamp): their digit runs are numbers by design. They are still
        // scanned for tickets and memo markers.
        let number_vectors = path
            .file_name()
            .is_some_and(|n| n == "bot-jcs-vectors.json");
        for f in scan(&value, &[]) {
            if f.kind == Kind::TinLike && number_vectors {
                continue;
            }
            if f.kind != Kind::Ticket || real_ticket {
                findings.push(format!("{}: {:?} at {}", path.display(), f.kind, f.path));
            }
        }
    }
    assert!(findings.is_empty(), "{findings:#?}");
}

#[test]
fn the_app_and_screen_sources_carry_no_tin_like_run_ticket_value_or_memo_marker() {
    let mut paths = Vec::new();
    files(&repo().join("apps"), "tsx", &mut paths);
    files(&repo().join("crates/books-screens/src"), "rs", &mut paths);
    assert!(!paths.is_empty());
    let mut findings = Vec::new();
    for path in &paths {
        let text = std::fs::read_to_string(path).unwrap();
        for (i, line) in text.lines().enumerate() {
            let at = format!("{} line {}", path.display(), i + 1);
            if tin_like(&line.replace("0123456789", "")) {
                findings.push(format!("{at}: tin_like"));
            }
            if has_ticket_value(line) {
                findings.push(format!("{at}: ticket"));
            }
            if line.contains("MEMO-") {
                findings.push(format!("{at}: memo marker"));
            }
        }
    }
    assert!(findings.is_empty(), "{findings:#?}");
}

// ---- vectors, fixtures and proposals -------------------------------------------

fn json_files(dir: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    files(&repo().join(dir), "json", &mut paths);
    paths.sort();
    assert!(!paths.is_empty(), "{dir}");
    paths
}

#[test]
fn every_vector_file_is_clean() {
    for path in json_files("fixtures/vectors") {
        let found = scan(&read_json(&path), &MEMO_NEEDLES);
        assert!(found.is_empty(), "{}: {found:?}", path.display());
    }
}

#[test]
fn every_synthetic_fixture_is_clean_of_tin_like_runs_and_tickets() {
    for path in json_files("fixtures/synthetic") {
        // memos are allowed in the fixture source; the engine drops them
        let found: Vec<Finding> = scan(&read_json(&path), &[])
            .into_iter()
            .filter(|f| f.kind != Kind::Needle)
            .collect();
        assert!(found.is_empty(), "{}: {found:?}", path.display());
    }
}

/// `^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$`.
fn is_uuid(text: &str) -> bool {
    let parts: Vec<&str> = text.split('-').collect();
    parts.len() == 5
        && parts.iter().zip([8, 4, 4, 4, 12]).all(|(p, n)| {
            p.len() == n
                && p.bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        })
}

#[test]
fn fixture_profiles_carry_an_opaque_uuid_and_a_masked_display_only() {
    for path in json_files("fixtures/synthetic") {
        for profile in read_json(&path)["profiles"].as_array().unwrap() {
            assert_eq!(
                profile["display"],
                "***-***-***-00000",
                "{}",
                path.display()
            );
            let id = profile["tax_profile_id"].as_str().unwrap();
            assert!(is_uuid(id), "{}: {id}", path.display());
        }
    }
}

#[test]
fn every_proposal_in_every_case_is_clean() {
    let data = shared().unwrap();
    let mut cases = 0;
    for path in json_files("fixtures/vectors") {
        let v = read_json(&path);
        let Some(fixture) = v["fixture"].as_str() else {
            continue;
        };
        let raw = read_json(&repo().join(format!("fixtures/synthetic/{fixture}.json")));
        let memos: Vec<String> = raw["entries"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|e| e["memo"].as_str().map(str::to_string))
            .collect();
        let mut needles: Vec<&str> = MEMO_NEEDLES.to_vec();
        needles.extend(memos.iter().map(String::as_str));
        let input = normalize_fixture(&raw, &data.vocab).unwrap();
        let form = FormCode::parse(v["form"].as_str().unwrap()).unwrap();
        let ctx = form_context_from_value(&v["form_context"]).unwrap();
        let p = build_proposal(
            data,
            &input,
            form,
            v["period"].as_str().unwrap(),
            &ctx,
            None,
        )
        .unwrap();
        let found = scan(&p.to_value(), &needles);
        assert!(found.is_empty(), "{}: {found:?}", path.display());
        cases += 1;
    }
    assert_eq!(cases, 16);
}

#[test]
fn a_canary_memo_planted_in_a_source_fixture_is_absent_from_every_proposal() {
    let data = shared().unwrap();
    let c = canaries();
    let mut raw = read_json(&repo().join("fixtures/synthetic/a-sole-prop-2026.json"));
    raw["entries"][0]["memo"] = Value::String(format!("{} {}", c.memo, c.ticket));
    let input = normalize_fixture(&raw, &data.vocab).unwrap();
    let graduated_osd: FormContext = [
        ("/taxpayer/tax_option", "graduated"),
        ("/taxpayer/deduction", "osd"),
    ]
    .iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();
    for (form, period, ctx) in [
        (FormCode::F2551Q, "2026-Q3", FormContext::new()),
        (FormCode::F1601C, "2026-09", FormContext::new()),
        (FormCode::F1701MS, "2026", graduated_osd),
    ] {
        let p = build_proposal(data, &input, form, period, &ctx, None).unwrap();
        let value = p.to_value();
        assert!(scan(&value, &[&c.memo]).is_empty(), "{period}");
        assert!(!canonical(&value).unwrap().contains(&c.ticket), "{period}");
    }
}
