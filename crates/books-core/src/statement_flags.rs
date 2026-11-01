//! Display-only anomaly flags on a bank statement (M10 parity): a repeated
//! date and amount, an amount far from the payee's usual, and a month whose
//! lines do not carry last month's ending to this one's. Integers only; a
//! flag never blocks Finish.

use std::collections::{HashMap, HashSet};

use crate::date::Date;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagKind {
    Duplicate,
    UnusualAmount,
    ChainBreak,
}

impl FlagKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Duplicate => "duplicate",
            Self::UnusualAmount => "unusual_amount",
            Self::ChainBreak => "chain_break",
        }
    }

    pub fn note(self) -> &'static str {
        match self {
            Self::Duplicate => "Same date and amount as another line",
            Self::UnusualAmount => "Much larger or smaller than usual for this payee",
            Self::ChainBreak => {
                "Last month's ending plus this month's lines does not equal this ending"
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flag {
    /// The statement line number; `None` for a flag on the whole statement.
    pub line_no: Option<i64>,
    pub kind: FlagKind,
}

/// Earlier amounts a payee needs before an amount counts as unusual.
pub const MIN_HISTORY: usize = 4;

/// The flags of one statement. `lines` are (line number, date, amount,
/// payee key); `history` holds each payee key's earlier amounts.
pub fn flags(
    lines: &[(i64, Date, i64, &str)],
    history: &HashMap<String, Vec<i64>>,
    prev_closing: Option<i64>,
    closing: i64,
) -> Vec<Flag> {
    let mut out = Vec::new();
    let mut seen: HashSet<(Date, i64)> = HashSet::new();
    for &(no, date, amount, key) in lines {
        if !seen.insert((date, amount)) {
            out.push(Flag {
                line_no: Some(no),
                kind: FlagKind::Duplicate,
            });
            continue;
        }
        if let Some(m) = history.get(key).and_then(|h| lower_median(h))
            && unusual(amount, m)
        {
            out.push(Flag {
                line_no: Some(no),
                kind: FlagKind::UnusualAmount,
            });
        }
    }
    if let Some(p) = prev_closing {
        let moved: i128 = lines.iter().map(|l| i128::from(l.2)).sum();
        if i128::from(p) + moved != i128::from(closing) {
            out.push(Flag {
                line_no: None,
                kind: FlagKind::ChainBreak,
            });
        }
    }
    out
}

/// The lower median of the absolute amounts, when there are enough.
fn lower_median(history: &[i64]) -> Option<i128> {
    if history.len() < MIN_HISTORY {
        return None;
    }
    let mut abs: Vec<i128> = history.iter().map(|a| i128::from(*a).abs()).collect();
    abs.sort_unstable();
    abs.get((abs.len() - 1) / 2).copied()
}

fn unusual(amount: i64, m: i128) -> bool {
    let a = i128::from(amount).abs();
    a > 3 * m || 3 * a < m
}

/// The payee a description names: lower case, no digits or mask stars,
/// runs of other characters as one space, at most 40 characters.
pub fn payee_key(description: &str) -> String {
    let mut out = String::new();
    let mut gap = false;
    for c in description.chars() {
        if c.is_ascii_digit() || c == '*' {
            continue;
        }
        if c.is_alphanumeric() {
            if gap && !out.is_empty() {
                out.push(' ');
            }
            gap = false;
            out.extend(c.to_lowercase());
        } else {
            gap = true;
        }
    }
    out.chars().take(40).collect::<String>().trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(day: u8) -> Date {
        Date::new(2026, 3, day).unwrap()
    }

    fn hist(key: &str, amounts: &[i64]) -> HashMap<String, Vec<i64>> {
        HashMap::from([(key.to_string(), amounts.to_vec())])
    }

    #[test]
    fn a_duplicate_pair_flags_only_the_second() {
        let lines = [
            (1, d(3), -45_000, "coffee shop"),
            (2, d(3), -45_000, "coffee shop"),
            (3, d(4), -45_000, "coffee shop"),
        ];
        let f = flags(&lines, &HashMap::new(), None, 0);
        assert_eq!(
            f,
            [Flag {
                line_no: Some(2),
                kind: FlagKind::Duplicate
            }]
        );
    }

    #[test]
    fn an_unusual_amount_needs_four_earlier_samples() {
        let lines = [(1, d(5), -500_000, "coffee shop")];
        let three = hist("coffee shop", &[-40_000, -45_000, -50_000]);
        assert!(flags(&lines, &three, None, 0).is_empty());
        let four = hist("coffee shop", &[-40_000, -45_000, -50_000, -42_000]);
        assert_eq!(
            flags(&lines, &four, None, 0),
            [Flag {
                line_no: Some(1),
                kind: FlagKind::UnusualAmount
            }]
        );
        // a third of the median is still usual; less is not
        let small = [(1, d(5), -14_000, "coffee shop")];
        assert_eq!(flags(&small, &four, None, 0).len(), 0);
        let tiny = [(1, d(5), -13_000, "coffee shop")];
        assert_eq!(flags(&tiny, &four, None, 0).len(), 1);
        // the lower median of 40, 42, 45, 50 is 42: three times is 126
        let edge = [(1, d(5), -126_000, "coffee shop")];
        assert!(flags(&edge, &four, None, 0).is_empty());
    }

    #[test]
    fn a_chain_break_needs_last_months_ending() {
        let lines = [(1, d(2), 10_000, "deposit"), (2, d(9), -2_500, "fee")];
        assert!(flags(&lines, &HashMap::new(), Some(100_000), 107_500).is_empty());
        assert_eq!(
            flags(&lines, &HashMap::new(), Some(100_000), 107_400),
            [Flag {
                line_no: None,
                kind: FlagKind::ChainBreak
            }]
        );
        assert!(flags(&lines, &HashMap::new(), None, 1).is_empty());
    }

    #[test]
    fn huge_amounts_do_not_overflow() {
        let big = i64::MAX / 4;
        let lines = [
            (1, d(1), big, "deposit"),
            (2, d(2), big, "deposit"),
            (3, d(3), big, "deposit"),
            (4, d(4), big, "deposit"),
        ];
        let h = hist("deposit", &[big, big, big, big]);
        // four quarters of i64::MAX overflow i64, not i128
        assert_eq!(
            flags(&lines, &h, Some(big), big),
            [Flag {
                line_no: None,
                kind: FlagKind::ChainBreak
            }]
        );
        let tiny = hist("deposit", &[1, 1, 1, 1]);
        assert_eq!(flags(&lines[..1], &tiny, None, big).len(), 1);
    }

    #[test]
    fn payee_keys_drop_digits_masks_and_punctuation() {
        assert_eq!(payee_key("  Coffee-Shop #0042 ****1234 "), "coffee shop");
        assert_eq!(payee_key("POS  Coffee   shop"), "pos coffee shop");
        assert_eq!(payee_key(&"a".repeat(60)).len(), 40);
        assert_eq!(payee_key("1234"), "");
    }
}
