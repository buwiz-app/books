//! Slices of the ledger: account movements narrowed to the entries that
//! carry a tag (a department or a location on the entry).

use std::collections::HashMap;

use books_core::date::Date;
use rusqlite::{Connection, params};

use crate::Result;

/// Which entry column a tag lives in. Each maps to a fixed column name, so
/// no caller text ever reaches the SQL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagColumn {
    Department,
    Location,
}

impl TagColumn {
    /// The column for a tag kind (`department` or `location`).
    pub fn of_kind(kind: &str) -> Option<Self> {
        match kind {
            "department" => Some(Self::Department),
            "location" => Some(Self::Location),
            _ => None,
        }
    }

    fn sql(self) -> &'static str {
        match self {
            Self::Department => {
                "SELECT l.account_id, sum(l.debit) - sum(l.credit)
                 FROM line l JOIN entry e ON e.entry_id = l.entry_id
                 WHERE e.book_id = ?1 AND e.status IN ('posted', 'voided')
                   AND e.entry_date >= ?2 AND e.entry_date <= ?3 AND e.department_id = ?4
                 GROUP BY l.account_id"
            }
            Self::Location => {
                "SELECT l.account_id, sum(l.debit) - sum(l.credit)
                 FROM line l JOIN entry e ON e.entry_id = l.entry_id
                 WHERE e.book_id = ?1 AND e.status IN ('posted', 'voided')
                   AND e.entry_date >= ?2 AND e.entry_date <= ?3 AND e.location_id = ?4
                 GROUP BY l.account_id"
            }
        }
    }
}

/// [`crate::books::movements`] over the entries tagged `tag_id` only.
pub fn movements_tagged(
    conn: &Connection,
    book_id: i64,
    from: Option<Date>,
    to: Option<Date>,
    column: TagColumn,
    tag_id: i64,
) -> Result<HashMap<i64, i64>> {
    let from = from.map_or_else(|| "0000-00-00".to_string(), |d| d.to_string());
    let to = to.map_or_else(|| "9999-99-99".to_string(), |d| d.to_string());
    let mut stmt = conn.prepare(column.sql())?;
    let rows = stmt.query_map(params![book_id, from, to, tag_id], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::books;
    use crate::entries::{self, DraftInput, Form, LineInput};
    use crate::people;
    use crate::{Paths, Store, TempDir};
    use books_core::catalog::Facts;
    use books_core::ledger::Basis;

    #[test]
    fn movements_tagged_sums_only_posted_entries_with_the_tag() {
        let tmp = TempDir::new("slices");
        let store = Store::open(&Paths::in_dir(tmp.0.join("d"))).unwrap();
        let c = store.conn();
        let id = books::create(c, "Test Book", Basis::Cash, 1).unwrap();
        let book = books::get(c, id).unwrap().unwrap();
        let acct = |n: &str| books::account_by_number(c, book.id, n).unwrap().unwrap().id;
        let (bank, rent) = (acct("11000"), acct("66100"));
        let spend = |amount: i64| DraftInput {
            date: Date::new(2026, 10, 12).unwrap(),
            form: Form::MoneyOut,
            payee: Some("Synthetic Landlord".into()),
            memo: None,
            reference: None,
            needs_review: false,
            lines: vec![
                LineInput { account_id: rent, debit: amount, credit: 0, facts: Facts::default() },
                LineInput { account_id: bank, debit: 0, credit: amount, facts: Facts::default() },
            ],
        };
        let open: entries::Gate<'static> = &|_| Ok(());
        let dept = people::create_tag(c, &book, "department", "Kitchen").unwrap();
        let other = people::create_tag(c, &book, "department", "Front").unwrap();
        let tagged = |amount: i64, tag: i64, post: bool| {
            let e = entries::save_draft(c, &book, None, &spend(amount), "app").unwrap();
            people::set_entry_links(c, &book, e, None, Some(tag), None).unwrap();
            if post {
                entries::post(c, &book, e, open).unwrap();
            }
            e
        };
        tagged(10_000, dept, true);
        tagged(20_000, dept, false);
        let gone = tagged(40_000, dept, false);
        entries::discard(c, &book, gone).unwrap();
        tagged(80_000, other, true);
        let from = Date::new(2026, 10, 1);
        let to = Date::new(2026, 10, 31);
        let m = movements_tagged(c, book.id, from, to, TagColumn::Department, dept).unwrap();
        assert_eq!(m.get(&rent), Some(&10_000));
        assert_eq!(m.get(&bank), Some(&-10_000));
        assert!(
            movements_tagged(c, book.id, from, to, TagColumn::Location, dept)
                .unwrap()
                .is_empty(),
            "a department is not a location"
        );
        let all = books::movements(c, book.id, from, to).unwrap();
        assert_eq!(all.get(&rent), Some(&90_000));
    }
}
