//! The book and its chart of accounts. A new book takes a chart preset
//! (the Philippines small-business chart for now): one account per kind,
//! numbered as the catalogue says.

use std::collections::HashMap;

use books_core::catalog::{AccountType, Catalog};
use books_core::date::Date;
use books_core::ledger::{Basis, KindRef};
use books_core::privacy;
use rusqlite::{Connection, OptionalExtension as _, Row, params};

use crate::{Result, error, now_iso};

#[derive(Debug, Clone)]
pub struct Book {
    pub id: i64,
    pub label: String,
    pub country: String,
    pub basis: Basis,
    pub fiscal_year_start: u8,
    pub coa_preset: String,
    pub coa_preset_version: i64,
    pub cloud_tax_profile_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct Account {
    pub id: i64,
    pub book_id: i64,
    pub number: String,
    pub name: String,
    pub account_type: AccountType,
    pub kind: KindRef,
    pub kind_confirmed: bool,
    pub active: bool,
}

impl Account {
    /// A cash or bank account: money moves through it.
    pub fn is_cash(&self) -> bool {
        self.kind == KindRef::Kind("bank_accounts".into())
    }

    /// Trade receivables and payables exist on accrual books only.
    pub fn accrual_only(&self) -> bool {
        self.kind.accrual_only()
    }

    /// The balance in the account's own sense from debits minus credits.
    pub fn natural(&self, debit_minus_credit: i64) -> i64 {
        if self.account_type.debit_normal() {
            debit_minus_credit
        } else {
            -debit_minus_credit
        }
    }
}

const BOOK_COLUMNS: &str = "book_id, label, country, basis, fiscal_year_start, coa_preset, coa_preset_version, cloud_tax_profile_id, created_at";

fn book_row(r: &Row<'_>) -> rusqlite::Result<Book> {
    let basis: String = r.get(3)?;
    let fy: i64 = r.get(4)?;
    Ok(Book {
        id: r.get(0)?,
        label: r.get(1)?,
        country: r.get(2)?,
        basis: Basis::parse(&basis).unwrap_or_default(),
        fiscal_year_start: u8::try_from(fy).unwrap_or(1),
        coa_preset: r.get(5)?,
        coa_preset_version: r.get(6)?,
        cloud_tax_profile_id: r.get(7)?,
        created_at: r.get(8)?,
    })
}

const ACCOUNT_COLUMNS: &str =
    "account_id, book_id, number, name, type, kind, kind_confirmed, active";

fn account_row(r: &Row<'_>) -> rusqlite::Result<Account> {
    let ty: String = r.get(4)?;
    let kind: String = r.get(5)?;
    Ok(Account {
        id: r.get(0)?,
        book_id: r.get(1)?,
        number: r.get(2)?,
        name: r.get(3)?,
        account_type: AccountType::parse(&ty).unwrap_or(AccountType::Asset),
        kind: KindRef::parse(&kind).unwrap_or(KindRef::Unsure),
        kind_confirmed: r.get::<_, i64>(6)? == 1,
        active: r.get::<_, i64>(7)? == 1,
    })
}

/// A book label: 1 to 60 characters, no TIN.
pub fn check_label(label: &str) -> Result<String> {
    let label = label.trim();
    if label.is_empty() || label.chars().count() > 60 {
        return Err(error("invalid_args", "A book name is 1 to 60 characters."));
    }
    privacy::refuse_tin("book name", label).map_err(|m| error("tin_refused", m))?;
    Ok(label.to_string())
}

/// Create a book on the Philippines preset. Returns its id.
pub fn create(conn: &Connection, label: &str, basis: Basis, fiscal_year_start: u8) -> Result<i64> {
    let label = check_label(label)?;
    if !(1..=12).contains(&fiscal_year_start) {
        return Err(error(
            "invalid_args",
            "The fiscal year starts in month 1 to 12.",
        ));
    }
    let catalog = Catalog::get();
    conn.execute(
        "INSERT INTO book (label, basis, fiscal_year_start, coa_preset, coa_preset_version, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            label,
            basis.as_str(),
            i64::from(fiscal_year_start),
            catalog.preset_id,
            catalog.preset_version,
            now_iso()
        ],
    )?;
    let book_id = conn.last_insert_rowid();
    let mut insert = conn.prepare(
        "INSERT INTO account (book_id, number, name, type, kind) VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    for a in &catalog.preset {
        insert.execute(params![
            book_id,
            a.number,
            a.name,
            a.account_type.as_str(),
            a.kind
        ])?;
    }
    Ok(book_id)
}

pub fn all(conn: &Connection) -> Result<Vec<Book>> {
    let mut stmt = conn.prepare(&format!("SELECT {BOOK_COLUMNS} FROM book ORDER BY book_id"))?;
    let rows = stmt.query_map([], book_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn get(conn: &Connection, id: i64) -> Result<Option<Book>> {
    Ok(conn
        .query_row(
            &format!("SELECT {BOOK_COLUMNS} FROM book WHERE book_id = ?1"),
            params![id],
            book_row,
        )
        .optional()?)
}

pub fn count(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("SELECT count(*) FROM book", [], |r| r.get(0))?)
}

pub fn accounts(conn: &Connection, book_id: i64) -> Result<Vec<Account>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {ACCOUNT_COLUMNS} FROM account WHERE book_id = ?1 ORDER BY number"
    ))?;
    let rows = stmt.query_map(params![book_id], account_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn account(conn: &Connection, id: i64) -> Result<Option<Account>> {
    Ok(conn
        .query_row(
            &format!("SELECT {ACCOUNT_COLUMNS} FROM account WHERE account_id = ?1"),
            params![id],
            account_row,
        )
        .optional()?)
}

pub fn account_by_number(conn: &Connection, book_id: i64, number: &str) -> Result<Option<Account>> {
    Ok(conn
        .query_row(
            &format!("SELECT {ACCOUNT_COLUMNS} FROM account WHERE book_id = ?1 AND number = ?2"),
            params![book_id, number],
            account_row,
        )
        .optional()?)
}

/// Debits minus credits per account over posted and voided entries (a void
/// and its reversal net to zero), dated within `[from, to]` when given.
pub fn movements(
    conn: &Connection,
    book_id: i64,
    from: Option<Date>,
    to: Option<Date>,
) -> Result<HashMap<i64, i64>> {
    let from = from.map_or_else(|| "0000-00-00".to_string(), |d| d.to_string());
    let to = to.map_or_else(|| "9999-99-99".to_string(), |d| d.to_string());
    let mut stmt = conn.prepare(
        "SELECT l.account_id, sum(l.debit) - sum(l.credit)
         FROM line l JOIN entry e ON e.entry_id = l.entry_id
         WHERE e.book_id = ?1 AND e.status IN ('posted', 'voided')
           AND e.entry_date >= ?2 AND e.entry_date <= ?3
         GROUP BY l.account_id",
    )?;
    let rows = stmt.query_map(params![book_id, from, to], |r| {
        Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Paths, Store, TempDir};

    #[test]
    fn a_new_book_gets_the_preset_chart_on_cash_basis() {
        let tmp = TempDir::new("book");
        let store = Store::open(&Paths::in_dir(tmp.0.join("d"))).unwrap();
        let id = create(store.conn(), "Buwiz Trading", Basis::Cash, 1).unwrap();
        let book = get(store.conn(), id).unwrap().unwrap();
        assert_eq!(book.basis, Basis::Cash);
        assert_eq!(book.coa_preset, "ph_small_business");
        let chart = accounts(store.conn(), id).unwrap();
        assert_eq!(chart.len(), 66);
        let bank = account_by_number(store.conn(), id, "11000")
            .unwrap()
            .unwrap();
        assert!(bank.is_cash());
        let ar = account_by_number(store.conn(), id, "12010")
            .unwrap()
            .unwrap();
        assert!(ar.accrual_only());
        assert_eq!(count(store.conn()).unwrap(), 1);
    }

    #[test]
    fn a_tin_shaped_or_empty_label_is_refused() {
        assert_eq!(check_label("  ").unwrap_err().code, "invalid_args");
        let tin = ["123", "456", "789"].join("-");
        assert_eq!(
            check_label(&format!("Shop {tin}")).unwrap_err().code,
            "tin_refused"
        );
        assert_eq!(check_label(" Buwiz Trading ").unwrap(), "Buwiz Trading");
    }
}
