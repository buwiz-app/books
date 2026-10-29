//! Posting the normalized entries for the measures engine: balance and
//! one-side checks, the reversal mirror and orphan rules, and each line's
//! signed amount (`measure_side`). Port of `poc/src/ledger.ts`
//! (`postLedger()`, `sameWindowReversalEntries()`, `sumMeasure()`).
//!
//! This is the engine's read of a set of entries, separate from
//! [`crate::ledger`], which holds the rules a post in the store enforces.

use std::collections::{HashMap, HashSet};

use crate::engine_error::EngineError;
use crate::input::{NormEntry, NormLine};
use crate::money;

/// A posted line (`PostedLine`). `measure` is the line's signed amount for
/// the measures that select it: credit minus debit when `measure_side` is
/// credit, debit minus credit when it is debit, zero when it is unset. A
/// reversal carries the mirrored sides, so its measure is the exact
/// negative of the original.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostedLine {
    pub line: NormLine,
    pub entry_id: String,
    pub entry_date: String,
    pub reverses_entry_id: Option<String>,
    pub original_entry_date: Option<String>,
    pub dr: i64,
    pub cr: i64,
    pub measure: i64,
}

/// The posted ledger (`Ledger`). Lines keep entry order, then line order.
#[derive(Debug, Clone)]
pub struct Ledger {
    pub lines: Vec<PostedLine>,
    pub entries: Vec<NormEntry>,
    /// Original entry id to the id of the entry that reverses it.
    pub reversed_by: HashMap<String, String>,
}

/// `inRange()`: a date (text) inside a window, both ends included. ISO
/// days compare as text.
pub fn in_range(date: Option<&str>, start: &str, end: &str) -> bool {
    date.is_some_and(|d| d >= start && d <= end)
}

fn parse_money(text: &str, label: &str) -> Result<i64, EngineError> {
    money::parse(text).map_err(|e| EngineError::Money(format!("{label}: {e}")))
}

/// `mirrors()`: the reversal line carries the same facts with the sides
/// swapped.
fn mirrors(original: &NormLine, reversal: &NormLine) -> Result<bool, EngineError> {
    let same_facts = original.account == reversal.account
        && original.measure_side == reversal.measure_side
        && original.income_class == reversal.income_class
        && original.contra_kind == reversal.contra_kind
        && original.document_kind == reversal.document_kind
        && original.person_scope == reversal.person_scope
        && original.party_ref == reversal.party_ref
        && original.comp_component == reversal.comp_component
        && original.contribution_side == reversal.contribution_side
        && original.contribution_scheme == reversal.contribution_scheme
        && original.credit_kind == reversal.credit_kind
        && original.certificate_atc == reversal.certificate_atc
        && original.certificate_status == reversal.certificate_status
        && original.period_start == reversal.period_start
        && original.period_end == reversal.period_end
        && original.final_tax_withheld == reversal.final_tax_withheld;
    if !same_facts {
        return Ok(false);
    }
    Ok(
        parse_money(&original.debit, "debit")? == parse_money(&reversal.credit, "credit")?
            && parse_money(&original.credit, "credit")? == parse_money(&reversal.debit, "debit")?,
    )
}

/// Port of `postLedger()`. Fails with a ledger error code: unbalanced,
/// line_sides, duplicate_id, orphan_reversal, reversed_twice,
/// reversal_not_mirror, reversal_before_original.
pub fn post_ledger(entries: &[NormEntry]) -> Result<Ledger, EngineError> {
    let mut by_id: HashMap<&str, &NormEntry> = HashMap::new();
    let mut line_ids: HashSet<&str> = HashSet::new();
    for entry in entries {
        if by_id.insert(entry.entry_id.as_str(), entry).is_some() {
            return Err(EngineError::ledger("duplicate_id", entry.entry_id.clone()));
        }
        let mut debits: i128 = 0;
        let mut credits: i128 = 0;
        for line in &entry.lines {
            if !line_ids.insert(line.line_id.as_str()) {
                return Err(EngineError::ledger("duplicate_id", line.line_id.clone()));
            }
            let dr = parse_money(&line.debit, &format!("{}.debit", line.line_id))?;
            let cr = parse_money(&line.credit, &format!("{}.credit", line.line_id))?;
            if dr < 0 || cr < 0 {
                return Err(EngineError::ledger(
                    "line_sides",
                    format!("{}: negative side", line.line_id),
                ));
            }
            if (dr > 0) == (cr > 0) {
                return Err(EngineError::ledger(
                    "line_sides",
                    format!(
                        "{}: exactly one of debit or credit must be non-zero",
                        line.line_id
                    ),
                ));
            }
            debits += i128::from(dr);
            credits += i128::from(cr);
        }
        if debits != credits {
            return Err(EngineError::ledger(
                "unbalanced",
                format!(
                    "{}: debits {} do not equal credits {}",
                    entry.entry_id,
                    money_text(debits),
                    money_text(credits)
                ),
            ));
        }
    }

    let mut reversed_by: HashMap<String, String> = HashMap::new();
    for entry in entries {
        let Some(target) = &entry.reverses_entry_id else {
            continue;
        };
        let Some(original) = by_id.get(target.as_str()) else {
            return Err(EngineError::ledger(
                "orphan_reversal",
                format!(
                    "{} reverses {target}, which is not in scope",
                    entry.entry_id
                ),
            ));
        };
        if let Some(first) = reversed_by.get(&original.entry_id) {
            return Err(EngineError::ledger(
                "reversed_twice",
                format!(
                    "{} is reversed by {first} and {}",
                    original.entry_id, entry.entry_id
                ),
            ));
        }
        let mut mirrored = original.lines.len() == entry.lines.len();
        if mirrored {
            for (a, b) in original.lines.iter().zip(&entry.lines) {
                if !mirrors(a, b)? {
                    mirrored = false;
                    break;
                }
            }
        }
        if !mirrored {
            return Err(EngineError::ledger(
                "reversal_not_mirror",
                format!(
                    "{} does not mirror {} line by line",
                    entry.entry_id, original.entry_id
                ),
            ));
        }
        if entry.entry_date < original.entry_date {
            return Err(EngineError::ledger(
                "reversal_before_original",
                format!("{} is dated before {}", entry.entry_id, original.entry_id),
            ));
        }
        reversed_by.insert(original.entry_id.clone(), entry.entry_id.clone());
    }

    let mut lines = Vec::new();
    for entry in entries {
        let original_entry_date = entry
            .reverses_entry_id
            .as_deref()
            .and_then(|id| by_id.get(id))
            .map(|e| e.entry_date.clone());
        for line in &entry.lines {
            let dr = parse_money(&line.debit, "debit")?;
            let cr = parse_money(&line.credit, "credit")?;
            // Both sides are non-negative i64, so either difference fits.
            let measure = match line.measure_side.as_deref() {
                None => 0,
                Some("credit") => cr - dr,
                Some(_) => dr - cr,
            };
            lines.push(PostedLine {
                line: line.clone(),
                entry_id: entry.entry_id.clone(),
                entry_date: entry.entry_date.clone(),
                reverses_entry_id: entry.reverses_entry_id.clone(),
                original_entry_date: original_entry_date.clone(),
                dr,
                cr,
                measure,
            });
        }
    }
    Ok(Ledger {
        lines,
        entries: entries.to_vec(),
        reversed_by,
    })
}

/// Port of `sameWindowReversalEntries()`: entry ids whose lines net to zero
/// inside a window, that is a reversal dated in the window whose original
/// is also dated in the window. Both sides are left out of lineage, and the
/// sum is unchanged.
pub fn same_window_reversal_entries<'a, I>(lines: I, start: &str, end: &str) -> HashSet<String>
where
    I: IntoIterator<Item = &'a PostedLine>,
{
    let mut netted = HashSet::new();
    for line in lines {
        if let Some(original) = &line.reverses_entry_id
            && in_range(Some(&line.entry_date), start, end)
            && in_range(line.original_entry_date.as_deref(), start, end)
        {
            netted.insert(line.entry_id.clone());
            netted.insert(original.clone());
        }
    }
    netted
}

/// Port of `sumMeasure()`, in `i128`.
pub fn sum_measure(lines: &[PostedLine]) -> i128 {
    lines.iter().map(|l| i128::from(l.measure)).sum()
}

/// `formatMoney()` for an `i128` sum: two-place decimal text, exactly as
/// [`money::format`] writes an `i64`.
pub fn money_text(centavos: i128) -> String {
    let magnitude = centavos.unsigned_abs();
    let sign = if centavos < 0 { "-" } else { "" };
    format!("{sign}{}.{:02}", magnitude / 100, magnitude % 100)
}

/// An `i128` sum narrowed to `i64` centavos, or a `money_overflow` error.
pub fn to_centavos(sum: i128, what: &str) -> Result<i64, EngineError> {
    i64::try_from(sum).map_err(|_| {
        EngineError::ledger(
            "money_overflow",
            format!("{what}: the sum is outside the i64 centavo range"),
        )
    })
}
