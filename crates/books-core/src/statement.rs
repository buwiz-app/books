//! Bank and card statements (M4): read a CSV statement, then match its
//! lines against the ledger. Pure: no store, no float.
//!
//! A line's amount is money into the account (positive) or out of it
//! (negative), in centavos. Descriptions keep no digit run of nine or more
//! (a TIN or an account number): they are masked before anything stores
//! them (invariants 3 and 4).

use crate::date::{Date, MONTHS};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub date: Date,
    pub description: String,
    pub amount: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    pub lines: Vec<Line>,
    /// The last running balance, when the statement has one.
    pub closing: Option<i64>,
}

/// One CSV record: commas, double quotes, `""` inside quotes.
fn record(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, quoted) {
            ('"', true) if chars.peek() == Some(&'"') => {
                cell.push('"');
                chars.next();
            }
            ('"', _) => quoted = !quoted,
            (',', false) => out.push(std::mem::take(&mut cell)),
            _ => cell.push(c),
        }
    }
    out.push(cell);
    out.into_iter().map(|c| c.trim().to_string()).collect()
}

/// Typed money to centavos: `1,234.50`, `-1234.5`, `(1,234.50)`,
/// `₱ 1,234`, `PHP 12.00`. Two decimals at most.
pub fn parse_amount(text: &str) -> Option<i64> {
    let mut s: String = text
        .chars()
        .filter(|c| !matches!(c, ',' | ' ' | '₱' | '\u{a0}'))
        .collect();
    if let Some(rest) = s.strip_prefix("PHP") {
        s = rest.to_string();
    }
    if s.is_empty() {
        return None;
    }
    let (negative, body) =
        if let Some(inner) = s.strip_prefix('(').and_then(|r| r.strip_suffix(')')) {
            (true, inner.to_string())
        } else if let Some(rest) = s.strip_prefix('-') {
            (true, rest.to_string())
        } else if let Some(rest) = s.strip_prefix('+') {
            (false, rest.to_string())
        } else {
            (false, s)
        };
    let (whole, cents) = match body.split_once('.') {
        Some((w, c)) => (w.to_string(), c.to_string()),
        None => (body, String::new()),
    };
    if whole.is_empty()
        || cents.len() > 2
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !cents.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let whole: i64 = whole.parse().ok()?;
    let cents: i64 = format!("{cents:0<2}").parse().ok()?;
    let value = whole.checked_mul(100)?.checked_add(cents)?;
    Some(if negative { -value } else { value })
}

/// `2026-10-03`, `10/03/2026` (month first), `Oct 03, 2026`, `03 Oct 2026`.
pub fn parse_date(text: &str) -> Option<Date> {
    if let Some(d) = Date::parse(text) {
        return Some(d);
    }
    let parts: Vec<&str> = text.split('/').collect();
    if parts.len() == 3 {
        let (m, d, y) = (
            parts[0].parse().ok()?,
            parts[1].parse().ok()?,
            parts[2].parse().ok()?,
        );
        return Date::new(y, m, d);
    }
    let words: Vec<String> = text
        .replace(',', " ")
        .split_whitespace()
        .map(str::to_string)
        .collect();
    if words.len() == 3 {
        let month = |w: &str| {
            MONTHS
                .iter()
                .position(|m| w.len() >= 3 && m.eq_ignore_ascii_case(&w[..3]))
                .map(|i| u8::try_from(i + 1).unwrap_or(1))
        };
        if let (Some(m), Ok(d), Ok(y)) = (month(&words[0]), words[1].parse(), words[2].parse()) {
            return Date::new(y, m, d);
        }
        if let (Ok(d), Some(m), Ok(y)) = (words[0].parse(), month(&words[1]), words[2].parse()) {
            return Date::new(y, m, d);
        }
    }
    None
}

/// Mask every run of nine or more digits (dashes and spaces inside count).
pub fn mask_digit_runs(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_digit() {
            let mut j = i;
            let mut digits = 0;
            while j < chars.len()
                && (chars[j].is_ascii_digit()
                    || ((chars[j] == '-' || chars[j] == ' ')
                        && j + 1 < chars.len()
                        && chars[j + 1].is_ascii_digit()))
            {
                if chars[j].is_ascii_digit() {
                    digits += 1;
                }
                j += 1;
            }
            if digits >= 9 {
                out.push_str("•••");
            } else {
                out.extend(&chars[i..j]);
            }
            i = j;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

fn find(header: &[String], names: &[&str]) -> Option<usize> {
    header.iter().position(|h| {
        let h = h.to_ascii_lowercase();
        names.iter().any(|n| h == *n)
    })
}

/// Read a CSV statement: a header row naming the date, the description and
/// either one signed amount or separate money-out and money-in columns,
/// and optionally a running balance.
pub fn parse_csv(text: &str) -> Result<Parsed, String> {
    let mut rows = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(record);
    let header = rows.next().ok_or("The statement is empty.")?;
    let date = find(
        &header,
        &[
            "date",
            "posting date",
            "transaction date",
            "value date",
            "txn date",
        ],
    )
    .ok_or("The statement has no Date column.")?;
    let desc = find(
        &header,
        &[
            "description",
            "details",
            "particulars",
            "narrative",
            "memo",
            "payee",
            "transaction",
        ],
    )
    .ok_or("The statement has no Description column.")?;
    let amount = find(&header, &["amount", "net amount"]);
    let out_col = find(
        &header,
        &[
            "debit",
            "withdrawal",
            "withdrawals",
            "money out",
            "debit amount",
        ],
    );
    let in_col = find(
        &header,
        &["credit", "deposit", "deposits", "money in", "credit amount"],
    );
    let balance = find(
        &header,
        &["balance", "running balance", "available balance"],
    );
    if amount.is_none() && (out_col.is_none() || in_col.is_none()) {
        return Err("The statement needs an Amount column, or Debit and Credit columns.".into());
    }
    let mut lines = Vec::new();
    let mut closing = None;
    for (n, row) in rows.enumerate() {
        let cell = |i: usize| row.get(i).map(String::as_str).unwrap_or("");
        let line_no = n + 2;
        let d = parse_date(cell(date))
            .ok_or_else(|| format!("Row {line_no}: the date is not a date."))?;
        let value = match amount {
            Some(a) => parse_amount(cell(a))
                .ok_or_else(|| format!("Row {line_no}: the amount is not an amount."))?,
            None => {
                let out = parse_amount(cell(out_col.unwrap_or(0))).unwrap_or(0);
                let inn = parse_amount(cell(in_col.unwrap_or(0))).unwrap_or(0);
                inn.abs() - out.abs()
            }
        };
        if value == 0 {
            continue;
        }
        if let Some(b) = balance.and_then(|b| parse_amount(cell(b))) {
            closing = Some(b);
        }
        let description = mask_digit_runs(cell(desc))
            .chars()
            .take(80)
            .collect::<String>();
        lines.push(Line {
            date: d,
            description: if description.is_empty() {
                "Bank line".into()
            } else {
                description
            },
            amount: value,
        });
        if lines.len() > 2000 {
            return Err("A statement has at most 2,000 lines.".into());
        }
    }
    if lines.is_empty() {
        return Err("The statement has no lines with an amount.".into());
    }
    Ok(Parsed { lines, closing })
}

/// A ledger movement on the account: an entry and its effect (debit minus
/// credit on the account, so money in is positive).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Candidate {
    pub entry_id: i64,
    pub date: Date,
    pub amount: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Match {
    /// Same amount, same day.
    Exact(i64),
    /// Same amount within five days: the user accepts it.
    Suggested(i64),
    /// Not in Books yet.
    Missing,
}

pub const SUGGEST_DAYS: i64 = 5;

/// Match statement lines to candidates. `kept[i]` is a line's stored match
/// (an accepted or added one), which stands and uses its entry. Each entry
/// matches one line at most.
pub fn match_lines(lines: &[Line], kept: &[Option<i64>], candidates: &[Candidate]) -> Vec<Match> {
    let mut used: Vec<bool> = candidates
        .iter()
        .map(|c| kept.contains(&Some(c.entry_id)))
        .collect();
    let mut out: Vec<Option<Match>> = kept.iter().map(|k| k.map(Match::Exact)).collect();
    out.resize(lines.len(), None);
    for (i, line) in lines.iter().enumerate() {
        if out[i].is_some() {
            continue;
        }
        if let Some(j) = (0..candidates.len()).find(|&j| {
            !used[j] && candidates[j].amount == line.amount && candidates[j].date == line.date
        }) {
            used[j] = true;
            out[i] = Some(Match::Exact(candidates[j].entry_id));
        }
    }
    for (i, line) in lines.iter().enumerate() {
        if out[i].is_some() {
            continue;
        }
        let best = (0..candidates.len())
            .filter(|&j| {
                !used[j]
                    && candidates[j].amount == line.amount
                    && (candidates[j].date.to_days() - line.date.to_days()).abs() <= SUGGEST_DAYS
            })
            .min_by_key(|&j| (candidates[j].date.to_days() - line.date.to_days()).abs());
        out[i] = Some(match best {
            Some(j) => {
                used[j] = true;
                Match::Suggested(candidates[j].entry_id)
            }
            None => Match::Missing,
        });
    }
    out.into_iter()
        .map(|m| m.unwrap_or(Match::Missing))
        .collect()
}

/// A kind for a line the bank describes plainly; `None` leaves it "not
/// sure yet" for the user (or the assistant) to pick.
pub fn suggest_kind(description: &str, amount: i64) -> Option<&'static str> {
    let d = description.to_ascii_uppercase();
    if amount > 0 && d.contains("INTEREST") {
        Some("interest_income")
    } else if amount < 0
        && (d.contains("SERVICE CHARGE") || d.contains("BANK FEE") || d.contains("BANK CHARGE"))
    {
        Some("bank_fees")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(t: &str) -> Date {
        Date::parse(t).unwrap()
    }

    #[test]
    fn amounts_and_dates_read_without_floats() {
        assert_eq!(parse_amount("1,234.50"), Some(123_450));
        assert_eq!(parse_amount("(1,234.5)"), Some(-123_450));
        assert_eq!(parse_amount("-42.18"), Some(-4_218));
        assert_eq!(parse_amount("₱ 80,000"), Some(8_000_000));
        assert_eq!(parse_amount("PHP 12.00"), Some(1_200));
        for bad in ["", "1.234", "1e3", "abc", "1.2.3"] {
            assert_eq!(parse_amount(bad), None, "{bad}");
        }
        assert_eq!(parse_date("10/03/2026"), Some(d("2026-10-03")));
        assert_eq!(parse_date("Oct 03, 2026"), Some(d("2026-10-03")));
        assert_eq!(parse_date("03 Oct 2026"), Some(d("2026-10-03")));
        assert_eq!(parse_date("2026-13-01"), None);
    }

    #[test]
    fn a_csv_with_debit_and_credit_columns_and_a_balance() {
        let ref_no = ["1234", "5678", "9012"].concat();
        let text = format!(
            "Date,Description,Debit,Credit,Balance\n2026-10-03,\"TRANSFER TO CARD, 7713\",\"10,000.00\",,166199.00\n10/05/2026,DEPOSIT ISLA CAFE ref {ref_no},,120000.00,286199.00\n"
        );
        let p = parse_csv(&text).unwrap();
        assert_eq!(p.lines.len(), 2);
        assert_eq!(p.lines[0].amount, -1_000_000);
        assert_eq!(p.lines[0].description, "TRANSFER TO CARD, 7713");
        assert_eq!(p.lines[1].amount, 12_000_000);
        assert_eq!(
            p.lines[1].description, "DEPOSIT ISLA CAFE ref •••",
            "a long digit run is masked"
        );
        assert_eq!(p.closing, Some(28_619_900));
        assert!(parse_csv("When,What\n").unwrap_err().contains("Date"));
    }

    #[test]
    fn matching_takes_exact_then_suggests_within_five_days() {
        let lines = vec![
            Line {
                date: d("2026-10-03"),
                description: "A".into(),
                amount: -1_000_000,
            },
            Line {
                date: d("2026-10-08"),
                description: "B".into(),
                amount: -15_000,
            },
            Line {
                date: d("2026-10-31"),
                description: "C".into(),
                amount: 4_218,
            },
            Line {
                date: d("2026-10-20"),
                description: "D".into(),
                amount: 9_700_000,
            },
        ];
        let cands = vec![
            Candidate {
                entry_id: 1,
                date: d("2026-10-03"),
                amount: -1_000_000,
            },
            Candidate {
                entry_id: 2,
                date: d("2026-10-06"),
                amount: -15_000,
            },
            Candidate {
                entry_id: 3,
                date: d("2026-10-20"),
                amount: 9_700_000,
            },
        ];
        let m = match_lines(&lines, &[None, None, None, None], &cands);
        assert_eq!(
            m,
            vec![
                Match::Exact(1),
                Match::Suggested(2),
                Match::Missing,
                Match::Exact(3)
            ]
        );
        let m = match_lines(&lines, &[None, Some(2), None, None], &cands);
        assert_eq!(m[1], Match::Exact(2), "a stored match stands");
        let far = vec![Candidate {
            entry_id: 9,
            date: d("2026-10-20"),
            amount: -15_000,
        }];
        assert_eq!(
            match_lines(&lines[1..2], &[None], &far),
            vec![Match::Missing]
        );
    }

    #[test]
    fn plain_bank_lines_get_a_kind() {
        assert_eq!(
            suggest_kind("INTEREST EARNED", 4_218),
            Some("interest_income")
        );
        assert_eq!(suggest_kind("SERVICE CHARGE", -15_000), Some("bank_fees"));
        assert_eq!(suggest_kind("LUMEN POWER CO", -249_900), None);
    }
}
