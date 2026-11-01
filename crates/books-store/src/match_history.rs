//! Match history: what the user's stored statement matches say about a
//! bank description. Read only; a user's accept or add is the learning
//! event, already stored immutably on `statement_line`.

use std::collections::HashMap;

use books_core::date::Period;
use books_core::match_assist::{Hint, normalize};
use books_core::statement::{Match, SUGGEST_DAYS};
use rusqlite::{Connection, OptionalExtension as _, params};

use crate::books::Book;
use crate::months;
use crate::{Result, error};

#[derive(Default)]
struct Tally {
    uses: i64,
    payees: HashMap<String, (i64, String)>,
    categories: HashMap<i64, (i64, String)>,
}

fn bump<K: Eq + std::hash::Hash>(map: &mut HashMap<K, (i64, String)>, key: K, period: &str) {
    let e = map.entry(key).or_insert((0, String::new()));
    e.0 += 1;
    if period > e.1.as_str() {
        e.1 = period.to_string();
    }
}

/// The most used key; ties go to the latest statement month, then the
/// smallest key so the answer is stable.
fn pick<K: Ord + Clone>(map: &HashMap<K, (i64, String)>) -> Option<K> {
    map.iter()
        .max_by(|a, b| {
            (a.1.0, &a.1.1)
                .cmp(&(b.1.0, &b.1.1))
                .then_with(|| b.0.cmp(a.0))
        })
        .map(|(k, _)| k.clone())
}

/// Hints for a money account's statements, keyed by normalized
/// description: matched posted entries' payee and their single category
/// line (an entry with more than one other line gives no category).
pub fn hints(conn: &Connection, book_id: i64, account_id: i64) -> Result<HashMap<String, Hint>> {
    let mut stmt = conn.prepare(
        "SELECT sl.description, e.payee, s.period_key,
                (SELECT CASE WHEN count(*) = 1 THEN min(l.account_id) END
                 FROM line l WHERE l.entry_id = e.entry_id AND l.account_id <> ?2)
         FROM statement_line sl
         JOIN statement s ON s.statement_id = sl.statement_id
         JOIN entry e ON e.entry_id = sl.matched_entry_id
         WHERE s.book_id = ?1 AND s.account_id = ?2 AND e.book_id = ?1
           AND e.status = 'posted' AND sl.match_kind IN ('exact', 'accepted', 'added')",
    )?;
    let rows = stmt.query_map(params![book_id, account_id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, Option<i64>>(3)?,
        ))
    })?;
    let mut tallies: HashMap<String, Tally> = HashMap::new();
    for row in rows {
        let (description, payee, period, category) = row?;
        let key = normalize(&description);
        if key.is_empty() {
            continue;
        }
        let t = tallies.entry(key).or_default();
        t.uses += 1;
        if let Some(p) = payee
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
        {
            bump(&mut t.payees, p, &period);
        }
        if let Some(c) = category {
            bump(&mut t.categories, c, &period);
        }
    }
    Ok(tallies
        .into_iter()
        .map(|(k, t)| {
            (
                k,
                Hint {
                    payee: pick(&t.payees),
                    category_account_id: pick(&t.categories),
                    uses: t.uses,
                },
            )
        })
        .collect())
}

/// The payee of each entry that has one.
pub fn payees(conn: &Connection, entry_ids: &[i64]) -> Result<HashMap<i64, String>> {
    let mut stmt = conn.prepare("SELECT payee FROM entry WHERE entry_id = ?1")?;
    let mut out = HashMap::new();
    for id in entry_ids {
        let payee: Option<Option<String>> = stmt
            .query_row(params![id], |r| r.get(0))
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                e => Err(e),
            })?;
        if let Some(p) = payee.flatten().filter(|p| !p.trim().is_empty()) {
            out.insert(*id, p);
        }
    }
    Ok(out)
}

/// Match one statement line (`line_no`, from 1) after the user's click:
/// its current suggestion, or `entry` when the user picked one. A picked
/// entry must be posted in this book, move `account_id` by exactly the
/// line's amount, be dated within the month ±[`SUGGEST_DAYS`] and not be
/// matched to any statement line yet. Returns the matched entry.
pub fn accept_line(
    conn: &Connection,
    book: &Book,
    account_id: i64,
    key: &str,
    line_no: i64,
    entry: Option<i64>,
) -> Result<i64> {
    let (s, lines) = months::open_statement(conn, book, account_id, key)?;
    let idx = usize::try_from(line_no - 1)
        .ok()
        .filter(|i| *i < lines.len())
        .ok_or_else(|| error("not_found", "That statement has no such line."))?;
    let line = &lines[idx];
    if line.matched_entry_id.is_some() {
        return Err(error("already_matched", "That line is already matched."));
    }
    let chosen = match entry {
        None => match months::matches(conn, book.id, &s, &lines)?[idx] {
            Match::Suggested(e) => e,
            _ => {
                return Err(error(
                    "no_suggestion",
                    "That line has no suggested match to accept.",
                ));
            }
        },
        Some(e) => {
            check_pick(conn, book, account_id, key, line.line.amount, e)?;
            e
        }
    };
    months::set_match(conn, line.id, chosen, "accepted")?;
    Ok(chosen)
}

fn check_pick(
    conn: &Connection,
    book: &Book,
    account_id: i64,
    key: &str,
    amount: i64,
    entry_id: i64,
) -> Result<()> {
    let refuse = |m: &str| error("entry_not_matchable", m.to_string());
    let row: Option<(String, String, i64)> = conn
        .query_row(
            "SELECT e.status, e.entry_date, coalesce(sum(l.debit), 0) - coalesce(sum(l.credit), 0)
             FROM entry e LEFT JOIN line l ON l.entry_id = e.entry_id AND l.account_id = ?3
             WHERE e.entry_id = ?1 AND e.book_id = ?2 GROUP BY e.entry_id",
            params![entry_id, book.id, account_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let Some((status, date, moved)) = row else {
        return Err(error("not_found", "That entry is not in this book."));
    };
    if status != "posted" {
        return Err(refuse("Only a posted entry can be matched."));
    }
    if moved != amount {
        return Err(refuse(
            "That entry's amount on this account differs from the line.",
        ));
    }
    let period = Period::parse(key).ok_or_else(|| error("invalid", "Bad statement month."))?;
    let day = books_core::date::Date::parse(&date)
        .map(|d| d.to_days())
        .ok_or_else(|| refuse("That entry has no valid date."))?;
    if day < period.start.to_days() - SUGGEST_DAYS || day > period.end.to_days() + SUGGEST_DAYS {
        return Err(refuse(
            "That entry is dated too far from the statement month.",
        ));
    }
    let used: i64 = conn.query_row(
        "SELECT count(*) FROM statement_line WHERE matched_entry_id = ?1",
        params![entry_id],
        |r| r.get(0),
    )?;
    if used > 0 {
        return Err(refuse("That entry is already matched to a statement line."));
    }
    Ok(())
}
