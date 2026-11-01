//! The review notes on a draft (migration 15): which fields the assistant
//! or a routine set, with the confidence the assistant reported, and the
//! likely duplicates of an entry. Integer rules only, no scores: same book,
//! a live draft or posted entry that is not a reversal or a copy of the
//! other, the same amount (the debit total), dates at most three days
//! apart, and the same payee (case and spaces aside) or the same reference.

use books_core::date::Date;
use rusqlite::{Connection, params};

use crate::{Result, now_iso};

/// The days either side of an entry's date a duplicate may fall on.
pub const WINDOW_DAYS: i64 = 3;
/// At most this many likely duplicates are named.
pub const MAX_DUPLICATES: usize = 5;

/// A draft field the review notes can name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Field {
    Date,
    Amount,
    Account,
    ToAccount,
    Category,
    Payee,
    Memo,
    Reference,
    Lines,
}

impl Field {
    pub const ALL: [Field; 9] = [
        Self::Date,
        Self::Amount,
        Self::Account,
        Self::ToAccount,
        Self::Category,
        Self::Payee,
        Self::Memo,
        Self::Reference,
        Self::Lines,
    ];

    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.as_str() == text)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Date => "date",
            Self::Amount => "amount",
            Self::Account => "account",
            Self::ToAccount => "to_account",
            Self::Category => "category",
            Self::Payee => "payee",
            Self::Memo => "memo",
            Self::Reference => "reference",
            Self::Lines => "lines",
        }
    }
}

/// One field's note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldNote {
    pub field: Field,
    pub confidence: Option<u8>,
    pub changed_by: Option<String>,
}

/// Note fields: each given confidence replaces the stored one, and
/// `changed_by` (when given) marks the fields as set by that origin;
/// `None` keeps what is stored.
pub fn set_fields(
    conn: &Connection,
    entry_id: i64,
    fields: &[(Field, Option<u8>)],
    changed_by: Option<&str>,
) -> Result<()> {
    let at = changed_by.map(|_| now_iso());
    let mut stmt = conn.prepare(
        "INSERT INTO draft_field (entry_id, field, confidence, changed_by, changed_at) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (entry_id, field) DO UPDATE SET
           confidence = coalesce(excluded.confidence, draft_field.confidence),
           changed_by = coalesce(excluded.changed_by, draft_field.changed_by),
           changed_at = coalesce(excluded.changed_at, draft_field.changed_at)",
    )?;
    for (field, confidence) in fields {
        stmt.execute(params![
            entry_id,
            field.as_str(),
            confidence.map(i64::from),
            changed_by,
            at
        ])?;
    }
    Ok(())
}

/// Forget the notes on these fields (the user saved them).
pub fn clear_fields(conn: &Connection, entry_id: i64, fields: &[Field]) -> Result<()> {
    let mut stmt = conn.prepare("DELETE FROM draft_field WHERE entry_id = ?1 AND field = ?2")?;
    for field in fields {
        stmt.execute(params![entry_id, field.as_str()])?;
    }
    Ok(())
}

/// The notes on an entry, in field order.
pub fn fields(conn: &Connection, entry_id: i64) -> Result<Vec<FieldNote>> {
    let mut stmt =
        conn.prepare("SELECT field, confidence, changed_by FROM draft_field WHERE entry_id = ?1")?;
    let rows = stmt.query_map(params![entry_id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, Option<i64>>(1)?,
            r.get::<_, Option<String>>(2)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (field, confidence, changed_by) = row?;
        if let Some(field) = Field::parse(&field) {
            out.push(FieldNote {
                field,
                confidence: confidence.and_then(|c| u8::try_from(c).ok()),
                changed_by,
            });
        }
    }
    out.sort_by_key(|n| n.field);
    Ok(out)
}

/// Record that `entry_id` was started from `source_id`.
pub fn set_copy(conn: &Connection, entry_id: i64, source_id: i64) -> Result<()> {
    conn.execute(
        "INSERT INTO entry_copy (entry_id, source_entry_id) VALUES (?1, ?2)
         ON CONFLICT (entry_id) DO UPDATE SET source_entry_id = excluded.source_entry_id",
        params![entry_id, source_id],
    )?;
    Ok(())
}

fn payee_key(text: Option<String>) -> Option<String> {
    text.map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty())
}

fn reference_key(text: Option<String>) -> Option<String> {
    text.map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
}

struct Shape {
    book_id: i64,
    live: bool,
    date: Date,
    amount: i64,
    payee: Option<String>,
    reference: Option<String>,
}

fn shape(conn: &Connection, entry_id: i64) -> Result<Option<Shape>> {
    let mut stmt = conn.prepare(
        "SELECT e.book_id, e.status IN ('draft', 'posted') AND e.form <> 'reversal', e.entry_date, e.payee, e.reference,
                coalesce((SELECT sum(l.debit) FROM line l WHERE l.entry_id = e.entry_id), 0)
         FROM entry e WHERE e.entry_id = ?1",
    )?;
    let mut rows = stmt.query(params![entry_id])?;
    let Some(r) = rows.next()? else {
        return Ok(None);
    };
    let Some(date) = Date::parse(&r.get::<_, String>(2)?) else {
        return Ok(None);
    };
    Ok(Some(Shape {
        book_id: r.get(0)?,
        live: r.get::<_, i64>(1)? == 1,
        date,
        payee: payee_key(r.get(3)?),
        reference: reference_key(r.get(4)?),
        amount: r.get(5)?,
    }))
}

/// Days between two dates when at most [`WINDOW_DAYS`].
fn distance(a: Date, b: Date) -> Option<i64> {
    (0..=WINDOW_DAYS).find(|d| a.add_days(*d) == b || a.add_days(-*d) == b)
}

/// The likely duplicates of an entry of `book_id`, nearest date first then
/// by entry number, at most [`MAX_DUPLICATES`].
pub fn likely_duplicates(conn: &Connection, book_id: i64, entry_id: i64) -> Result<Vec<i64>> {
    let Some(me) = shape(conn, entry_id)? else {
        return Ok(Vec::new());
    };
    if me.book_id != book_id
        || !me.live
        || me.amount <= 0
        || (me.payee.is_none() && me.reference.is_none())
    {
        return Ok(Vec::new());
    }
    let mut stmt = conn.prepare(
        "SELECT e.entry_id, e.entry_no, e.entry_date, e.payee, e.reference
         FROM entry e
         WHERE e.book_id = ?1 AND e.entry_id <> ?2 AND e.status IN ('draft', 'posted') AND e.form <> 'reversal'
           AND e.entry_date >= ?3 AND e.entry_date <= ?4
           AND coalesce((SELECT sum(l.debit) FROM line l WHERE l.entry_id = e.entry_id), 0) = ?5
           AND NOT EXISTS (SELECT 1 FROM entry_copy c
                           WHERE (c.entry_id = ?2 AND c.source_entry_id = e.entry_id)
                              OR (c.entry_id = e.entry_id AND c.source_entry_id = ?2))",
    )?;
    let rows = stmt.query_map(
        params![
            book_id,
            entry_id,
            me.date.add_days(-WINDOW_DAYS).to_string(),
            me.date.add_days(WINDOW_DAYS).to_string(),
            me.amount
        ],
        |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<String>>(4)?,
            ))
        },
    )?;
    let mut found = Vec::new();
    for row in rows {
        let (id, no, date, payee, reference) = row?;
        let Some(days) = Date::parse(&date).and_then(|d| distance(me.date, d)) else {
            continue;
        };
        let same_payee = me.payee.is_some() && payee_key(payee) == me.payee;
        let same_reference = me.reference.is_some() && reference_key(reference) == me.reference;
        if same_payee || same_reference {
            found.push((days, no, id));
        }
    }
    found.sort_unstable();
    Ok(found
        .into_iter()
        .take(MAX_DUPLICATES)
        .map(|(_, _, id)| id)
        .collect())
}

/// The user keeps `entry_id` alongside each of `others`.
pub fn ack(conn: &Connection, entry_id: i64, others: &[i64]) -> Result<()> {
    let at = now_iso();
    let mut stmt = conn.prepare(
        "INSERT INTO duplicate_ack (entry_id, other_entry_id, acked_at) VALUES (?1, ?2, ?3) ON CONFLICT DO NOTHING",
    )?;
    for other in others {
        stmt.execute(params![entry_id, other, at])?;
    }
    Ok(())
}

/// The entries the user kept `entry_id` alongside, either way round.
pub fn acked(conn: &Connection, entry_id: i64) -> Result<Vec<i64>> {
    let mut stmt = conn.prepare(
        "SELECT other_entry_id FROM duplicate_ack WHERE entry_id = ?1
         UNION SELECT entry_id FROM duplicate_ack WHERE other_entry_id = ?1 ORDER BY 1",
    )?;
    let rows = stmt.query_map(params![entry_id], |r| r.get::<_, i64>(0))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// The likely duplicates the user has not kept the entry alongside.
pub fn unacked_duplicates(conn: &Connection, book_id: i64, entry_id: i64) -> Result<Vec<i64>> {
    let kept = acked(conn, entry_id)?;
    Ok(likely_duplicates(conn, book_id, entry_id)?
        .into_iter()
        .filter(|id| !kept.contains(id))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books::{self, Book};
    use crate::entries::{self, DraftInput, Form, LineInput};
    use crate::{Paths, Store, TempDir};
    use books_core::catalog::Facts;
    use books_core::ledger::Basis;

    fn open(tag: &str) -> (TempDir, Store, Book) {
        let tmp = TempDir::new(tag);
        let store = Store::open(&Paths::in_dir(tmp.0.join("d"))).unwrap();
        let id = books::create(store.conn(), "Test Book", Basis::Cash, 1).unwrap();
        let book = books::get(store.conn(), id).unwrap().unwrap();
        (tmp, store, book)
    }

    fn draft(store: &Store, book: &Book, day: u8, amount: i64, payee: &str) -> i64 {
        let acct = |n: &str| {
            books::account_by_number(store.conn(), book.id, n)
                .unwrap()
                .unwrap()
                .id
        };
        let input = DraftInput {
            date: Date::new(2026, 10, day).unwrap(),
            form: Form::MoneyOut,
            payee: Some(payee.into()),
            memo: None,
            reference: None,
            needs_review: false,
            lines: vec![
                LineInput {
                    account_id: acct("67200"),
                    debit: amount,
                    credit: 0,
                    facts: Facts::default(),
                },
                LineInput {
                    account_id: acct("11000"),
                    debit: 0,
                    credit: amount,
                    facts: Facts::default(),
                },
            ],
        };
        entries::save_draft(store.conn(), book, None, &input, "agent").unwrap()
    }

    #[test]
    fn notes_keep_confidence_and_clear_per_field() {
        let (_t, store, book) = open("review-notes");
        let c = store.conn();
        let id = draft(&store, &book, 18, 100, "Synthetic Co");
        set_fields(
            c,
            id,
            &[(Field::Category, Some(55)), (Field::Payee, None)],
            Some("agent"),
        )
        .unwrap();
        set_fields(c, id, &[(Field::Payee, Some(90))], None).unwrap();
        let notes = fields(c, id).unwrap();
        assert_eq!(notes.len(), 2);
        let payee = notes.iter().find(|n| n.field == Field::Payee).unwrap();
        assert_eq!(
            (payee.confidence, payee.changed_by.as_deref()),
            (Some(90), Some("agent"))
        );
        clear_fields(c, id, &[Field::Payee]).unwrap();
        assert_eq!(fields(c, id).unwrap().len(), 1);
        assert_eq!(Field::parse("to_account"), Some(Field::ToAccount));
        assert_eq!(Field::parse("tin"), None);
    }

    #[test]
    fn duplicates_follow_the_rules_and_acks_hide_them() {
        let (_t, store, book) = open("review-dupes");
        let c = store.conn();
        let a = draft(&store, &book, 16, 135_000, "Synthetic Co");
        let b = draft(&store, &book, 18, 135_000, " synthetic co ");
        let far = draft(&store, &book, 20, 135_000, "Synthetic Co");
        let other = draft(&store, &book, 17, 135_100, "Synthetic Co");
        assert_eq!(likely_duplicates(c, book.id, a).unwrap(), vec![b]);
        assert_eq!(likely_duplicates(c, book.id, b).unwrap(), vec![a, far]);
        assert!(likely_duplicates(c, book.id, other).unwrap().is_empty());
        set_copy(c, far, b).unwrap();
        assert_eq!(likely_duplicates(c, book.id, b).unwrap(), vec![a]);
        ack(c, b, &[a]).unwrap();
        ack(c, b, &[a]).unwrap();
        assert_eq!(acked(c, b).unwrap(), vec![a]);
        assert!(unacked_duplicates(c, book.id, b).unwrap().is_empty());
        assert!(unacked_duplicates(c, book.id, a).unwrap().is_empty());
    }
}
