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

fn within(line: &Line, c: &Candidate) -> bool {
    c.amount == line.amount && (c.date.to_days() - line.date.to_days()).abs() <= SUGGEST_DAYS
}

/// [`crate::statement::match_lines`] with hints. Every line keeps the state
/// match_lines gives it (exact, suggested or missing) and every exact match
/// stands; a hint only moves a suggestion to an entry with the hint payee,
/// nearest date first, then the original order. It takes that entry when
/// it is free (the old one is freed), or swaps with the line that holds it
/// when that line does not follow its own hint and the swapped entry is
/// within its window too. Each entry matches one line at most.
pub fn rank(
    lines: &[Line],
    kept: &[Option<i64>],
    candidates: &[Candidate],
    cand_payee: &HashMap<i64, String>,
    hints: &HashMap<String, Hint>,
) -> Vec<Match> {
    let mut out = crate::statement::match_lines(lines, kept, candidates);
    if hints.is_empty() {
        return out;
    }
    let pos = |e: i64| candidates.iter().position(|c| c.entry_id == e);
    // holder[j]: the line suggested candidate j; exact and kept ones are never moved.
    let mut holder: Vec<Option<usize>> = vec![None; candidates.len()];
    let mut fixed: Vec<bool> = candidates
        .iter()
        .map(|c| kept.contains(&Some(c.entry_id)))
        .collect();
    for (i, m) in out.iter().enumerate() {
        match *m {
            Match::Exact(e) => {
                if let Some(j) = pos(e) {
                    fixed[j] = true;
                }
            }
            Match::Suggested(e) => {
                if let Some(j) = pos(e) {
                    holder[j] = Some(i);
                }
            }
            Match::Missing => {}
        }
    }
    for (i, line) in lines.iter().enumerate() {
        let Match::Suggested(now) = out[i] else {
            continue;
        };
        if follows_hint(line, now, cand_payee, hints) {
            continue;
        }
        let Some(cur) = pos(now) else {
            continue;
        };
        let gap = |j: usize| (candidates[j].date.to_days() - line.date.to_days()).abs();
        let mut wanted: Vec<usize> = (0..candidates.len())
            .filter(|&j| {
                !fixed[j]
                    && within(line, &candidates[j])
                    && follows_hint(line, candidates[j].entry_id, cand_payee, hints)
            })
            .collect();
        wanted.sort_by_key(|&j| (gap(j), j));
        for j in wanted {
            match holder[j] {
                None => {
                    holder[cur] = None;
                    holder[j] = Some(i);
                    out[i] = Match::Suggested(candidates[j].entry_id);
                    break;
                }
                Some(k) => {
                    let other = &lines[k];
                    let keeps_own = follows_hint(other, candidates[j].entry_id, cand_payee, hints);
                    if !keeps_own && within(other, &candidates[cur]) {
                        holder[cur] = Some(k);
                        holder[j] = Some(i);
                        out[k] = Match::Suggested(candidates[cur].entry_id);
                        out[i] = Match::Suggested(candidates[j].entry_id);
                        break;
                    }
                }
            }
        }
    }
    out
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

    fn coffee_hint() -> HashMap<String, Hint> {
        HashMap::from([(
            "coffee shop".to_string(),
            Hint {
                payee: Some("Coffee shop".into()),
                category_account_id: None,
                uses: 1,
            },
        )])
    }

    #[test]
    fn a_hint_never_takes_another_lines_only_candidate() {
        let lines = vec![
            line("2026-03-10", "COFFEE SHOP 1", -10_000),
            line("2026-03-17", "BAKERY", -10_000),
        ];
        let cands = vec![
            cand(1, "2026-03-07", -10_000),
            cand(2, "2026-03-14", -10_000),
        ];
        let payees = HashMap::from([(1, "Hardware store".into()), (2, "Coffee shop".into())]);
        let base = statement::match_lines(&lines, &[None, None], &cands);
        assert_eq!(base, vec![Match::Suggested(1), Match::Suggested(2)]);
        assert_eq!(
            rank(&lines, &[None, None], &cands, &payees, &coffee_hint()),
            base
        );
    }

    #[test]
    fn a_hint_swaps_when_both_lines_stay_in_window() {
        let lines = vec![
            line("2026-03-10", "COFFEE SHOP 1", -10_000),
            line("2026-03-12", "BAKERY", -10_000),
        ];
        let cands = vec![
            cand(1, "2026-03-09", -10_000),
            cand(2, "2026-03-13", -10_000),
        ];
        let payees = HashMap::from([(1, "Hardware store".into()), (2, "Coffee shop".into())]);
        assert_eq!(
            statement::match_lines(&lines, &[None, None], &cands),
            vec![Match::Suggested(1), Match::Suggested(2)]
        );
        assert_eq!(
            rank(&lines, &[None, None], &cands, &payees, &coffee_hint()),
            vec![Match::Suggested(2), Match::Suggested(1)]
        );
    }

    fn state(m: &Match) -> u8 {
        match m {
            Match::Exact(_) => 0,
            Match::Suggested(_) => 1,
            Match::Missing => 2,
        }
    }

    #[test]
    fn with_hints_rank_keeps_every_lines_state_and_uses_an_entry_once() {
        let amounts = [-45_000, -12_500, 30_000];
        let descs = ["COFFEE SHOP 1", "Hardware store 22", "BAKERY", "Fuel 7"];
        let names = ["Coffee shop", "Hardware store", "Bakery", "Fuel"];
        let mut moved = 0;
        for seed in 0..500_u64 {
            let mut g = Lcg(seed + 7);
            let pick = |g: &mut Lcg, n: usize| usize::try_from(g.next(n as u64)).unwrap();
            let day = |g: &mut Lcg| d(&format!("2026-03-{:02}", 1 + g.next(28)));
            let nl = 1 + g.next(8);
            let nc = g.next(10);
            let lines: Vec<Line> = (0..nl)
                .map(|_| Line {
                    date: day(&mut g),
                    description: descs[pick(&mut g, 4)].into(),
                    amount: amounts[pick(&mut g, 3)],
                })
                .collect();
            let cands: Vec<Candidate> = (0..nc)
                .map(|k| Candidate {
                    entry_id: i64::try_from(k).unwrap() + 1,
                    date: day(&mut g),
                    amount: amounts[pick(&mut g, 3)],
                })
                .collect();
            // a stored match holds its entry, so kept ids are distinct
            let mut kept: Vec<Option<i64>> = Vec::new();
            for _ in 0..nl {
                let k = (nc > 0 && g.next(6) == 0).then(|| i64::try_from(g.next(nc)).unwrap() + 1);
                kept.push(k.filter(|e| !kept.contains(&Some(*e))));
            }
            let payees: HashMap<i64, String> = cands
                .iter()
                .map(|c| (c.entry_id, names[pick(&mut g, 4)].to_string()))
                .collect();
            let mut hints: HashMap<String, Hint> = HashMap::new();
            for desc in descs {
                if g.next(3) != 0 {
                    let payee = names[pick(&mut g, 4)].to_string();
                    hints.insert(
                        normalize(desc),
                        Hint {
                            payee: Some(payee),
                            category_account_id: None,
                            uses: 1,
                        },
                    );
                }
            }
            let base = statement::match_lines(&lines, &kept, &cands);
            let got = rank(&lines, &kept, &cands, &payees, &hints);
            assert_eq!(
                got.iter().map(state).collect::<Vec<_>>(),
                base.iter().map(state).collect::<Vec<_>>(),
                "seed {seed}"
            );
            for (b, r) in base.iter().zip(&got) {
                if let Match::Exact(_) = b {
                    assert_eq!(b, r, "seed {seed}");
                }
            }
            let mut ids: Vec<i64> = got
                .iter()
                .filter_map(|m| match m {
                    Match::Exact(e) | Match::Suggested(e) => Some(*e),
                    Match::Missing => None,
                })
                .collect();
            let n = ids.len();
            ids.sort_unstable();
            ids.dedup();
            assert_eq!(ids.len(), n, "seed {seed}");
            for (l, m) in lines.iter().zip(&got) {
                if let Match::Suggested(e) = m {
                    let c = cands.iter().find(|c| c.entry_id == *e).unwrap();
                    assert!(within(l, c), "seed {seed}");
                }
            }
            if got != base {
                moved += 1;
            }
        }
        assert!(moved > 0, "the generator never exercised a hint");
    }
}
