//! Deterministic JSON, the form every hash and byte comparison is taken
//! over. Port of `poc/src/canonical.ts`: `canonical()` sorts object keys,
//! writes no whitespace and takes safe integers only; `pretty()` is the
//! file form (`prettyJson()`: two-space indent, sorted keys, trailing
//! newline). Strings are escaped exactly as JavaScript's `JSON.stringify`
//! escapes them, so Rust and the PoC hash the same bytes.
//!
//! Numbers are read as integers only. A JSON number with a fraction or an
//! exponent is refused, as the PoC refuses a number that is not a safe
//! integer.

use std::cmp::Ordering;
use std::fmt;

use serde_json::{Map, Number, Value};
use sha2::{Digest, Sha256};

/// `Number.MAX_SAFE_INTEGER`.
const MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalError(pub String);

impl fmt::Display for CanonicalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CanonicalError {}

/// JavaScript string order: by UTF-16 code unit, as `Array.prototype.sort`
/// and `<` compare strings.
pub fn js_cmp(a: &str, b: &str) -> Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

/// Port of `canonical()`.
pub fn canonical(value: &Value) -> Result<String, CanonicalError> {
    let mut out = String::new();
    write_canonical(value, &mut out)?;
    Ok(out)
}

fn write_canonical(value: &Value, out: &mut String) -> Result<(), CanonicalError> {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&integer_text(n)?),
        Value::String(s) => write_string(s, out),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(item, out)?;
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort_by(|a, b| js_cmp(a, b));
            out.push('{');
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(key, out);
                out.push(':');
                write_canonical(&map[key.as_str()], out)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

fn integer_text(n: &Number) -> Result<String, CanonicalError> {
    let safe = n
        .as_i64()
        .filter(|v| (-MAX_SAFE_INTEGER..=MAX_SAFE_INTEGER).contains(v));
    safe.map(|v| v.to_string())
        .ok_or_else(|| CanonicalError(format!("canonical JSON takes safe integers only, got {n}")))
}

/// `JSON.stringify` of a string: `"` and `\` escaped, the short escapes
/// for backspace, form feed, newline, carriage return and tab, every other
/// control character as `\u00xx` (lower-case hex), everything else raw.
fn write_string(s: &str, out: &mut String) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => out.push_str(&format!("\\u{:04x}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// A JavaScript array index ("0", or a digit run with no leading zero,
/// below 2^32 - 1). `JSON.stringify` writes such keys first, in numeric
/// order, whatever order they were inserted in.
fn array_index(key: &str) -> Option<u32> {
    if key.is_empty() || !key.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if key.len() > 1 && key.starts_with('0') {
        return None;
    }
    key.parse::<u32>().ok().filter(|v| *v != u32::MAX)
}

/// Key order of `JSON.stringify(sortKeysDeep(v))`: the object is rebuilt
/// with sorted keys, and JavaScript then lists array-index keys first.
fn pretty_key_order(map: &Map<String, Value>) -> Vec<&String> {
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort_by(|a, b| match (array_index(a), array_index(b)) {
        (Some(x), Some(y)) => x.cmp(&y),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => js_cmp(a, b),
    });
    keys
}

/// Port of `prettyJson()`: `JSON.stringify(sortKeysDeep(v), null, 2) + "\n"`.
pub fn pretty(value: &Value) -> Result<String, CanonicalError> {
    let mut out = String::new();
    write_pretty(value, 0, &mut out)?;
    out.push('\n');
    Ok(out)
}

fn indent(depth: usize, out: &mut String) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn write_pretty(value: &Value, depth: usize, out: &mut String) -> Result<(), CanonicalError> {
    match value {
        Value::Array(items) if !items.is_empty() => {
            out.push_str("[\n");
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(",\n");
                }
                indent(depth + 1, out);
                write_pretty(item, depth + 1, out)?;
            }
            out.push('\n');
            indent(depth, out);
            out.push(']');
        }
        Value::Object(map) if !map.is_empty() => {
            out.push_str("{\n");
            for (i, key) in pretty_key_order(map).into_iter().enumerate() {
                if i > 0 {
                    out.push_str(",\n");
                }
                indent(depth + 1, out);
                write_string(key, out);
                out.push_str(": ");
                write_pretty(&map[key.as_str()], depth + 1, out)?;
            }
            out.push('\n');
            indent(depth, out);
            out.push('}');
        }
        Value::Array(_) => out.push_str("[]"),
        Value::Object(_) => out.push_str("{}"),
        other => write_canonical(other, out)?,
    }
    Ok(())
}

/// Port of `sha256Hex()`: lower-case hex SHA-256 of the UTF-8 bytes.
pub fn sha256_hex(text: &str) -> String {
    let digest = Sha256::digest(text.as_bytes());
    let mut hex = String::with_capacity(64);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

/// Port of `sha256Of()`: the SHA-256 of the canonical form.
pub fn sha256_of(value: &Value) -> Result<String, CanonicalError> {
    Ok(sha256_hex(&canonical(value)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sorts_keys_and_writes_no_whitespace() {
        let v = json!({"b": [1, true, null], "a": {"d": "x", "c": -2}});
        assert_eq!(
            canonical(&v).unwrap(),
            r#"{"a":{"c":-2,"d":"x"},"b":[1,true,null]}"#
        );
    }

    #[test]
    fn escapes_like_json_stringify() {
        let v = json!("q\"b\\\u{08}\u{0c}\n\r\t\u{01}\u{1f}\u{7f}é");
        assert_eq!(
            canonical(&v).unwrap(),
            "\"q\\\"b\\\\\\b\\f\\n\\r\\t\\u0001\\u001f\u{7f}é\""
        );
    }

    #[test]
    fn refuses_unsafe_and_fractional_numbers() {
        let big: Value = serde_json::from_str("9007199254740992").unwrap();
        assert!(canonical(&big).is_err());
        let frac: Value = serde_json::from_str("1.5").unwrap();
        assert!(canonical(&frac).is_err());
        let max: Value = serde_json::from_str("-9007199254740991").unwrap();
        assert_eq!(canonical(&max).unwrap(), "-9007199254740991");
    }

    #[test]
    fn pretty_lists_index_keys_first_like_javascript() {
        let v = json!({"b": 1, "10": [], "2": {}, "2026-Q3": "open", "a": [1, {"z": null}]});
        assert_eq!(
            pretty(&v).unwrap(),
            "{\n  \"2\": {},\n  \"10\": [],\n  \"2026-Q3\": \"open\",\n  \"a\": [\n    1,\n    {\n      \"z\": null\n    }\n  ],\n  \"b\": 1\n}\n"
        );
    }

    #[test]
    fn hashes_utf8_bytes() {
        assert_eq!(
            sha256_hex(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
