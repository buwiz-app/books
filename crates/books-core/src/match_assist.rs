//! Match assist: the matches a user already accepted or added teach which
//! entry a bank line is. Hints come from stored matches only, so there is
//! no AI and no new write; ranking changes only which same-amount
//! candidate within [`crate::statement::SUGGEST_DAYS`] is suggested.

use std::collections::HashMap;

use crate::statement::{Candidate, Line, Match, SUGGEST_DAYS};

/// The longest normalized description kept as a hint key.
pub const KEY_CHARS: usize = 40;

/// The character [`crate::statement::mask_digit_runs`] puts for a digit.
const MASK: char = '\u{2022}';

/// A description's hint key: ASCII case-folded, digits and the digit mask
/// dropped, other non-alphanumerics collapsed to single spaces, at most
/// [`KEY_CHARS`] characters. Empty means no hint.
pub fn normalize(description: &str) -> String {
    let mut out = String::new();
    let mut gap = false;
    for c in description.chars() {
        if c.is_ascii_digit() || c == MASK {
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
    let cut: String = out.chars().take(KEY_CHARS).collect();
    cut.trim_end().to_string()
}

/// What past matches say about a normalized description.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Hint {
    pub payee: Option<String>,
    pub category_account_id: Option<i64>,
    pub uses: i64,
}

/// The hint for a line, if its description has history.
pub fn hint_for<'a>(hints: &'a HashMap<String, Hint>, description: &str) -> Option<&'a Hint> {
    let key = normalize(description);
    if key.is_empty() {
        return None;
    }
    hints.get(&key)
}

fn same_payee(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

/// Whether a suggestion follows the line's hint payee.
pub fn follows_hint(
    line: &Line,
    entry_id: i64,
    cand_payee: &HashMap<i64, String>,
    hints: &HashMap<String, Hint>,
) -> bool {
    let want = hint_for(hints, &line.description).and_then(|h| h.payee.as_deref());
    match (want, cand_payee.get(&entry_id)) {
        (Some(w), Some(p)) => same_payee(w, p),
        _ => false,
    }
}

/// [`crate::statement::match_lines`] with hints: stored and exact matches, and
/// which lines get a suggestion, are the same. A suggestion prefers the
/// candidate whose payee equals the line's hint payee, then the nearest
/// date, then the original order. Each entry matches one line at most.
pub fn rank(
    lines: &[Line],
    kept: &[Option<i64>],
    candidates: &[Candidate],
    cand_payee: &HashMap<i64, String>,
    hints: &HashMap<String, Hint>,
) -> Vec<Match> {
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
        let gap = |j: usize| (candidates[j].date.to_days() - line.date.to_days()).abs();
        let best = (0..candidates.len())
            .filter(|&j| !used[j] && candidates[j].amount == line.amount && gap(j) <= SUGGEST_DAYS)
            .min_by_key(|&j| {
                (
                    !follows_hint(line, candidates[j].entry_id, cand_payee, hints),
                    gap(j),
                    j,
                )
            });
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::date::Date;
    use crate::statement;

    fn d(t: &str) -> Date {
        Date::parse(t).unwrap()
    }

    #[test]
    fn normalize_drops_digits_and_masks_and_folds_case() {
        assert_eq!(normalize("COFFEE SHOP 00123"), "coffee shop");
        assert_eq!(normalize("Coffee   Shop ref •••"), "coffee shop ref");
        assert_eq!(normalize("  Hardware-store / #12 "), "hardware store");
        assert_eq!(normalize("12345 678"), "");
        assert_eq!(normalize("•••"), "");
        assert_eq!(normalize(""), "");
        let long = "abcdefghij ".repeat(6);
        let n = normalize(&long);
        assert!(n.chars().count() <= KEY_CHARS, "{n}");
        assert!(!n.ends_with(' '));
    }

    fn line(date: &str, desc: &str, amount: i64) -> Line {
        Line {
            date: d(date),
            description: desc.into(),
            amount,
        }
    }

    fn cand(entry_id: i64, date: &str, amount: i64) -> Candidate {
        Candidate {
            entry_id,
            date: d(date),
            amount,
        }
    }

    #[test]
    fn a_hint_payee_beats_a_nearer_date() {
        let lines = vec![line("2026-02-10", "COFFEE SHOP 00456", -45_000)];
        let cands = vec![
            cand(1, "2026-02-09", -45_000),
            cand(2, "2026-02-13", -45_000),
        ];
        let payees = HashMap::from([(1, "Hardware store".into()), (2, "Coffee shop".into())]);
        let none = HashMap::new();
        assert_eq!(
            rank(&lines, &[None], &cands, &payees, &none),
            vec![Match::Suggested(1)]
        );
        let hints = HashMap::from([(
            "coffee shop".to_string(),
            Hint {
                payee: Some("Coffee shop".into()),
                category_account_id: None,
                uses: 1,
            },
        )]);
        assert_eq!(
            rank(&lines, &[None], &cands, &payees, &hints),
            vec![Match::Suggested(2)]
        );
    }

    /// A tiny integer generator, so the property needs no extra crate.
    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self, n: u64) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (self.0 >> 33) % n
        }
    }

    #[test]
    fn without_hints_rank_is_match_lines() {
        let amounts = [-45_000, -12_500, 30_000, 99_900];
        for seed in 0..50_u64 {
            let mut g = Lcg(seed + 1);
            let day = |g: &mut Lcg| d(&format!("2026-03-{:02}", 1 + g.next(28)));
            let nl = 1 + g.next(8);
            let nc = g.next(10);
            let lines: Vec<Line> = (0..nl)
                .map(|_| Line {
                    date: day(&mut g),
                    description: "Coffee shop".into(),
                    amount: amounts[usize::try_from(g.next(4)).unwrap()],
                })
                .collect();
            let cands: Vec<Candidate> = (0..nc)
                .map(|k| Candidate {
                    entry_id: i64::try_from(k).unwrap() + 1,
                    date: day(&mut g),
                    amount: amounts[usize::try_from(g.next(4)).unwrap()],
                })
                .collect();
            let kept: Vec<Option<i64>> = (0..nl)
                .map(|_| {
                    if nc > 0 && g.next(5) == 0 {
                        Some(i64::try_from(g.next(nc)).unwrap() + 1)
                    } else {
                        None
                    }
                })
                .collect();
            let payees: HashMap<i64, String> = cands
                .iter()
                .map(|c| (c.entry_id, "Hardware store".to_string()))
                .collect();
            assert_eq!(
                rank(&lines, &kept, &cands, &payees, &HashMap::new()),
                statement::match_lines(&lines, &kept, &cands),
                "seed {seed}"
            );
        }
    }
}
