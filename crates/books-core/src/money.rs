//! Money is integer centavos (`i64`; `i128` for sums). JSON carries decimal
//! text with exactly two places, for example `"200000.00"`. There is no
//! float, no rounding and no rate here: measures are exact sums, and tax
//! rounding is Forms' (ADR 0007).

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoneyError(pub String);

impl fmt::Display for MoneyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for MoneyError {}

/// `^-?(0|[1-9][0-9]*)\.[0-9]{2}$`, never `-0.00`, within `i64`.
pub fn parse(text: &str) -> Result<i64, MoneyError> {
    let bad = || {
        MoneyError(format!(
            "\"{text}\" is not decimal text with exactly two places"
        ))
    };
    let (negative, body) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (whole, cents) = body.split_once('.').ok_or_else(bad)?;
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(whole) || cents.len() != 2 || !digits(cents) {
        return Err(bad());
    }
    if whole.len() > 1 && whole.starts_with('0') {
        return Err(bad());
    }
    let whole: i128 = whole.parse().map_err(|_| bad())?;
    let cents: i128 = cents.parse().map_err(|_| bad())?;
    let magnitude = whole
        .checked_mul(100)
        .and_then(|w| w.checked_add(cents))
        .ok_or_else(|| MoneyError(format!("\"{text}\" is outside the i64 range")))?;
    if negative && magnitude == 0 {
        return Err(bad());
    }
    let value = if negative { -magnitude } else { magnitude };
    i64::try_from(value)
        .ok()
        .filter(|v| *v != i64::MIN)
        .ok_or_else(|| MoneyError(format!("\"{text}\" is outside the i64 range")))
}

/// An amount as a BIR form shows it: [`parse`]'s text, with thousands
/// separators in groups of three (`"1,200,000.00"`) and whole pesos
/// (`"1,200,000"`) allowed. eBIRForms shows amounts this way.
pub fn parse_shown(text: &str) -> Result<i64, MoneyError> {
    let bad = || MoneyError(format!("\"{text}\" is not an amount"));
    let text = text.trim();
    let (sign, body) = match text.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", text),
    };
    let (whole, cents) = match body.split_once('.') {
        Some((whole, cents)) => (whole, cents),
        None => (body, "00"),
    };
    let mut groups = whole.split(',');
    let first = groups.next().unwrap_or_default();
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let mut plain = String::from(first);
    let mut grouped = false;
    for group in groups {
        if group.len() != 3 || !digits(group) {
            return Err(bad());
        }
        grouped = true;
        plain.push_str(group);
    }
    if !digits(first) || (grouped && (first.len() > 3 || first.starts_with('0'))) {
        return Err(bad());
    }
    parse(&format!("{sign}{plain}.{cents}")).map_err(|_| bad())
}

/// Two-place decimal text: `-5` is `"-0.05"`.
pub fn format(centavos: i64) -> String {
    let magnitude = i128::from(centavos).unsigned_abs();
    let sign = if centavos < 0 { "-" } else { "" };
    format!("{sign}{}.{:02}", magnitude / 100, magnitude % 100)
}

/// For people: thousands separators and a real minus sign, `−1,350.00`.
pub fn display(centavos: i64) -> String {
    let magnitude = i128::from(centavos).unsigned_abs();
    let whole = (magnitude / 100).to_string();
    let mut grouped = String::with_capacity(whole.len() + whole.len() / 3);
    for (i, ch) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let sign = if centavos < 0 { "\u{2212}" } else { "" };
    format!("{sign}{grouped}.{:02}", magnitude % 100)
}

/// A checked sum. `None` on overflow of `i64` (sums run in `i128`).
pub fn sum<I: IntoIterator<Item = i64>>(values: I) -> Option<i64> {
    let total = values
        .into_iter()
        .try_fold(0i128, |acc, v| acc.checked_add(i128::from(v)))?;
    i64::try_from(total).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_amount_as_a_form_shows_it() {
        for (shown, centavos) in [
            ("290000.00", 29_000_000),
            ("290,000.00", 29_000_000),
            ("1,200,000", 120_000_000),
            ("0", 0),
            ("0.00", 0),
            ("-2,808.40", -280_840),
            (" 10,000 ", 1_000_000),
        ] {
            assert_eq!(parse_shown(shown), Ok(centavos), "{shown}");
        }
        for bad in [
            "1",
            "true",
            "01/15/2026",
            "1,20,000",
            "12,0000.00",
            "0,100",
            "1.5",
            "",
            "-0",
            ",100",
        ] {
            if bad == "1" {
                // a whole peso is an amount; a radio's "1" is told apart by the box, not here
                assert_eq!(parse_shown(bad), Ok(100));
                continue;
            }
            assert!(parse_shown(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn parses_two_place_text_only() {
        assert_eq!(parse("200000.00"), Ok(20_000_000));
        assert_eq!(parse("-1.50"), Ok(-150));
        assert_eq!(parse("0.00"), Ok(0));
        assert_eq!(parse("0.05"), Ok(5));
        for bad in [
            "1.5", "1.499", "1,000.00", "1e3", "-0.00", " 1.00", "1.00 ", "01.00", ".50", "1.", "",
            "+1.00", "-",
        ] {
            assert!(parse(bad).is_err(), "{bad:?}");
        }
        assert!(parse("92233720368547758.07").is_ok());
        assert!(parse("92233720368547758.08").is_err());
    }

    #[test]
    fn formats_back_to_text() {
        assert_eq!(format(20_000_000), "200000.00");
        assert_eq!(format(-5), "-0.05");
        assert_eq!(format(0), "0.00");
        assert_eq!(format(i64::MIN), "-92233720368547758.08");
        for text in ["1.00", "-1234567.89", "0.10"] {
            assert_eq!(format(parse(text).unwrap()), text);
        }
    }

    #[test]
    fn displays_for_people() {
        assert_eq!(display(135_000), "1,350.00");
        assert_eq!(display(-135_000), "\u{2212}1,350.00");
        assert_eq!(display(41_735_000), "417,350.00");
        assert_eq!(display(122_454_840), "1,224,548.40");
        assert_eq!(display(99), "0.99");
    }

    #[test]
    fn sums_are_checked() {
        assert_eq!(sum([1, 2, -3]), Some(0));
        assert_eq!(sum([i64::MAX, 1]), None);
        assert_eq!(sum([i64::MAX, 1, -1]), Some(i64::MAX));
    }
}
