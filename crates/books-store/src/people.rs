//! People, tags and unpaid documents (M2).
//!
//! A person is a name and a kind (vendor, customer, employee, owner,
//! lender, government); banks and cards are the book's cash and card
//! accounts. There is no TIN anywhere: the BIR app keeps them. Tags are
//! departments and locations, set on an entry.
//!
//! An unpaid document waits for money. On a cash book (the default, ADR
//! 0008) it is a reminder and posts nothing; marking it paid posts the
//! money on the day it moves. On an accrual book saving it posts the
//! receivable or payable, and marking it paid clears it.

use books_core::catalog::Facts;
use books_core::date::Date;
use books_core::ledger::{Basis, KindRef, Status};
use rusqlite::{Connection, OptionalExtension as _, Row, params};

use crate::books::{self, Book};
use crate::entries::{self, DraftInput, Form, Gate, LineInput, check_text};
use crate::{Result, error, now_iso};

pub const PERSON_KINDS: [&str; 6] = [
    "vendor",
    "customer",
    "employee",
    "owner",
    "lender",
    "government",
];
pub const TAG_KINDS: [&str; 2] = ["department", "location"];

#[derive(Debug, Clone)]
pub struct Person {
    pub id: i64,
    pub book_id: i64,
    pub kind: String,
    pub name: String,
    pub note: Option<String>,
    pub active: bool,
}

fn person_row(r: &Row<'_>) -> rusqlite::Result<Person> {
    Ok(Person {
        id: r.get(0)?,
        book_id: r.get(1)?,
        kind: r.get(2)?,
        name: r.get(3)?,
        note: r.get(4)?,
        active: r.get::<_, i64>(5)? == 1,
    })
}

const PERSON_COLUMNS: &str = "person_id, book_id, kind, name, note, active";

fn check_name(what: &str, name: &str) -> Result<String> {
    let name = check_text(what, Some(name), 80)?
        .ok_or_else(|| error("invalid_args", format!("A {what} is needed.")))?;
    Ok(name)
}

/// A person by kind and name, made when it does not exist yet.
pub fn person_for(conn: &Connection, book: &Book, kind: &str, name: &str) -> Result<i64> {
    if !PERSON_KINDS.contains(&kind) {
        return Err(error(
            "invalid_args",
            "A person is a vendor, customer, employee, owner, lender or government.",
        ));
    }
    let name = check_name("name", name)?;
    if let Some(id) = conn
        .query_row(
            "SELECT person_id FROM person WHERE book_id = ?1 AND kind = ?2 AND name = ?3 COLLATE NOCASE",
            params![book.id, kind, name],
            |r| r.get::<_, i64>(0),
        )
        .optional()?
    {
        return Ok(id);
    }
    conn.execute(
        "INSERT INTO person (book_id, kind, name, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![book.id, kind, name, now_iso()],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn create_person(
    conn: &Connection,
    book: &Book,
    kind: &str,
    name: &str,
    note: Option<&str>,
) -> Result<i64> {
    let note = check_text("note", note, 200)?;
    let exists = conn
        .query_row(
            "SELECT 1 FROM person WHERE book_id = ?1 AND kind = ?2 AND name = ?3 COLLATE NOCASE",
            params![book.id, kind, name.trim()],
            |_| Ok(()),
        )
        .optional()?;
    if exists.is_some() {
        return Err(error("exists", "That person is already in the book."));
    }
    let id = person_for(conn, book, kind, name)?;
    conn.execute(
        "UPDATE person SET note = ?2 WHERE person_id = ?1",
        params![id, note],
    )?;
    Ok(id)
}

pub fn person(conn: &Connection, id: i64) -> Result<Option<Person>> {
    Ok(conn
        .query_row(
            &format!("SELECT {PERSON_COLUMNS} FROM person WHERE person_id = ?1"),
            params![id],
            person_row,
        )
        .optional()?)
}

pub fn people(conn: &Connection, book_id: i64, kind: Option<&str>) -> Result<Vec<Person>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {PERSON_COLUMNS} FROM person WHERE book_id = ?1 AND (?2 IS NULL OR kind = ?2) AND active = 1 ORDER BY name COLLATE NOCASE"
    ))?;
    let rows = stmt.query_map(params![book_id, kind], person_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[derive(Debug, Clone)]
pub struct Tag {
    pub id: i64,
    pub book_id: i64,
    pub kind: String,
    pub name: String,
}

pub fn create_tag(conn: &Connection, book: &Book, kind: &str, name: &str) -> Result<i64> {
    if !TAG_KINDS.contains(&kind) {
        return Err(error(
            "invalid_args",
            "A tag is a department or a location.",
        ));
    }
    let name = check_name("tag name", name)?;
    let exists = conn
        .query_row(
            "SELECT 1 FROM tag WHERE book_id = ?1 AND kind = ?2 AND name = ?3 COLLATE NOCASE",
            params![book.id, kind, name],
            |_| Ok(()),
        )
        .optional()?;
    if exists.is_some() {
        return Err(error(
            "exists",
            format!("That {kind} is already in the book."),
        ));
    }
    conn.execute(
        "INSERT INTO tag (book_id, kind, name, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![book.id, kind, name, now_iso()],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn tags(conn: &Connection, book_id: i64, kind: Option<&str>) -> Result<Vec<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT tag_id, book_id, kind, name FROM tag WHERE book_id = ?1 AND (?2 IS NULL OR kind = ?2) AND active = 1 ORDER BY name COLLATE NOCASE",
    )?;
    let rows = stmt.query_map(params![book_id, kind], |r| {
        Ok(Tag {
            id: r.get(0)?,
            book_id: r.get(1)?,
            kind: r.get(2)?,
            name: r.get(3)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn tag(conn: &Connection, id: i64) -> Result<Option<Tag>> {
    Ok(conn
        .query_row(
            "SELECT tag_id, book_id, kind, name FROM tag WHERE tag_id = ?1",
            params![id],
            |r| {
                Ok(Tag {
                    id: r.get(0)?,
                    book_id: r.get(1)?,
                    kind: r.get(2)?,
                    name: r.get(3)?,
                })
            },
        )
        .optional()?)
}

/// Set an entry's person and tags. Only a draft changes, except that
/// tags ride on a posted entry too: they change no amount or fact.
pub fn set_entry_links(
    conn: &Connection,
    book: &Book,
    entry_id: i64,
    person_id: Option<i64>,
    department_id: Option<i64>,
    location_id: Option<i64>,
) -> Result<()> {
    let entry = entries::get(conn, entry_id)?
        .filter(|e| e.book_id == book.id)
        .ok_or_else(|| error("not_found", "That entry is not in this book."))?;
    if let Some(p) = person_id {
        person(conn, p)?
            .filter(|x| x.book_id == book.id)
            .ok_or_else(|| error("not_found", "That person is not in this book."))?;
        if entry.status != Status::Draft && links(conn, entry_id)?.0 != Some(p) {
            return Err(error(
                "not_draft",
                "The person of a posted entry does not change.",
            ));
        }
    }
    for (t, kind) in [(department_id, "department"), (location_id, "location")] {
        if let Some(t) = t {
            tag(conn, t)?
                .filter(|x| x.book_id == book.id && x.kind == kind)
                .ok_or_else(|| error("not_found", format!("That {kind} is not in this book.")))?;
        }
    }
    conn.execute(
        "UPDATE entry SET person_id = coalesce(?2, person_id), department_id = ?3, location_id = ?4 WHERE entry_id = ?1",
        params![entry_id, person_id, department_id, location_id],
    )?;
    Ok(())
}

/// An entry's `(person, department, location)`.
pub fn links(conn: &Connection, entry_id: i64) -> Result<(Option<i64>, Option<i64>, Option<i64>)> {
    Ok(conn.query_row(
        "SELECT person_id, department_id, location_id FROM entry WHERE entry_id = ?1",
        params![entry_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?)
}

#[derive(Debug, Clone)]
pub struct Unpaid {
    pub id: i64,
    pub book_id: i64,
    pub side: String,
    pub person_id: Option<i64>,
    pub reference: Option<String>,
    pub memo: Option<String>,
    pub issued: Date,
    pub due: Option<Date>,
    pub amount: i64,
    pub category_account_id: i64,
    pub status: String,
    pub accrual_entry_id: Option<i64>,
    pub paid_entry_id: Option<i64>,
    pub paid_on: Option<Date>,
}

const UNPAID_COLUMNS: &str = "unpaid_id, book_id, side, person_id, reference, memo, issued, due, amount, category_account_id, status, accrual_entry_id, paid_entry_id, paid_on";

fn unpaid_row(r: &Row<'_>) -> rusqlite::Result<Unpaid> {
    let date = |s: Option<String>| s.and_then(|s| Date::parse(&s));
    Ok(Unpaid {
        id: r.get(0)?,
        book_id: r.get(1)?,
        side: r.get(2)?,
        person_id: r.get(3)?,
        reference: r.get(4)?,
        memo: r.get(5)?,
        issued: date(r.get(6)?).unwrap_or(Date {
            year: 1970,
            month: 1,
            day: 1,
        }),
        due: date(r.get(7)?),
        amount: r.get(8)?,
        category_account_id: r.get(9)?,
        status: r.get(10)?,
        accrual_entry_id: r.get(11)?,
        paid_entry_id: r.get(12)?,
        paid_on: date(r.get(13)?),
    })
}

pub fn unpaid(conn: &Connection, id: i64) -> Result<Option<Unpaid>> {
    Ok(conn
        .query_row(
            &format!("SELECT {UNPAID_COLUMNS} FROM unpaid WHERE unpaid_id = ?1"),
            params![id],
            unpaid_row,
        )
        .optional()?)
}

/// Open documents on one side, by due date (no due date last).
pub fn open_unpaid(conn: &Connection, book_id: i64, side: &str) -> Result<Vec<Unpaid>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {UNPAID_COLUMNS} FROM unpaid WHERE book_id = ?1 AND side = ?2 AND status = 'open'
         ORDER BY due IS NULL, due, unpaid_id"
    ))?;
    let rows = stmt.query_map(params![book_id, side], unpaid_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// The trade account an accrual book uses for a side.
fn trade_account(conn: &Connection, book: &Book, side: &str) -> Result<i64> {
    let kind = if side == "collect" {
        "accounts_receivable_gross"
    } else {
        "accounts_payable"
    };
    books::accounts(conn, book.id)?
        .into_iter()
        .find(|a| a.active && a.kind == KindRef::Kind(kind.into()))
        .map(|a| a.id)
        .ok_or_else(|| {
            error(
                "no_account",
                "The book has no receivable or payable account.",
            )
        })
}

pub struct UnpaidInput<'a> {
    pub side: &'a str,
    pub person_id: Option<i64>,
    pub reference: Option<&'a str>,
    pub memo: Option<&'a str>,
    pub issued: Date,
    pub due: Option<Date>,
    pub amount: i64,
    pub category_account_id: i64,
}

/// Save an invoice to collect or a bill to pay. On an accrual book this
/// posts the receivable or payable now (the gate applies to its date).
pub fn create_unpaid(
    conn: &Connection,
    book: &Book,
    input: &UnpaidInput<'_>,
    gate: Gate<'_>,
) -> Result<i64> {
    if input.side != "collect" && input.side != "pay" {
        return Err(error(
            "invalid_args",
            "An unpaid document is to collect or to pay.",
        ));
    }
    if input.amount <= 0 {
        return Err(error("invalid_args", "The amount must be more than 0.00."));
    }
    let reference = check_text("reference", input.reference, 60)?;
    let memo = check_text("memo", input.memo, 500)?;
    let category = books::account(conn, input.category_account_id)?
        .filter(|a| a.book_id == book.id && a.active)
        .ok_or_else(|| error("invalid_args", "That category is not in this book."))?;
    if category.is_cash() || category.accrual_only() {
        return Err(error("invalid_args", "Pick an income or expense category."));
    }
    if let Some(p) = input.person_id {
        person(conn, p)?
            .filter(|x| x.book_id == book.id)
            .ok_or_else(|| error("not_found", "That person is not in this book."))?;
    }
    if let Some(due) = input.due
        && due < input.issued
    {
        return Err(error(
            "invalid_args",
            "The due date is before the document's date.",
        ));
    }
    conn.execute(
        "INSERT INTO unpaid (book_id, side, person_id, reference, memo, issued, due, amount, category_account_id, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            book.id,
            input.side,
            input.person_id,
            reference,
            memo,
            input.issued.to_string(),
            input.due.map(|d| d.to_string()),
            input.amount,
            category.id,
            now_iso()
        ],
    )?;
    let id = conn.last_insert_rowid();
    if book.basis == Basis::Accrual {
        let trade = trade_account(conn, book, input.side)?;
        let (dr, cr) = if input.side == "collect" {
            (trade, category.id)
        } else {
            (category.id, trade)
        };
        let facts = sales_facts(&category.kind, input.side);
        let draft = DraftInput {
            date: input.issued,
            form: Form::Journal,
            payee: None,
            memo: memo.clone(),
            reference: reference.clone(),
            needs_review: false,
            lines: vec![
                LineInput {
                    account_id: dr,
                    debit: input.amount,
                    credit: 0,
                    facts: if dr == category.id {
                        facts.clone()
                    } else {
                        Facts::default()
                    },
                },
                LineInput {
                    account_id: cr,
                    debit: 0,
                    credit: input.amount,
                    facts: if cr == category.id {
                        facts
                    } else {
                        Facts::default()
                    },
                },
            ],
        };
        let entry = entries::save_draft(conn, book, None, &draft, "app")?;
        conn.execute(
            "UPDATE entry SET person_id = ?2 WHERE entry_id = ?1",
            params![entry, input.person_id],
        )?;
        entries::post(conn, book, entry, gate)?;
        conn.execute(
            "UPDATE unpaid SET accrual_entry_id = ?2 WHERE unpaid_id = ?1",
            params![id, entry],
        )?;
    }
    Ok(id)
}

/// A sales line settles an invoice (`sale_invoice`), so the sales
/// measures count it and the cash rule accepts it (ADR 0008).
fn sales_facts(kind: &KindRef, side: &str) -> Facts {
    let sales = matches!(kind, KindRef::Kind(k) if books_core::catalog::Catalog::get()
        .kind(k)
        .is_some_and(|k| k.defaults.income_class.as_deref() == Some("business_sales")));
    if side == "collect" && sales {
        Facts {
            document_kind: Some("sale_invoice".into()),
            ..Facts::default()
        }
    } else if side == "pay" {
        Facts {
            document_kind: Some("expense_document".into()),
            ..Facts::default()
        }
    } else {
        Facts::default()
    }
}

/// Mark a document paid on `on` from `money_account`: posts the money
/// entry that day and closes the document. Returns the entry id.
pub fn mark_paid(
    conn: &Connection,
    book: &Book,
    id: i64,
    money_account: i64,
    on: Date,
    gate: Gate<'_>,
) -> Result<i64> {
    let u = unpaid(conn, id)?
        .filter(|u| u.book_id == book.id)
        .ok_or_else(|| error("not_found", "That document is not in this book."))?;
    if u.status != "open" {
        return Err(error("already_paid", "That document is already paid."));
    }
    if on < u.issued {
        return Err(error("invalid_args", "It cannot be paid before its date."));
    }
    let money = books::account(conn, money_account)?
        .filter(|a| a.book_id == book.id && a.active)
        .ok_or_else(|| error("invalid_args", "That account is not in this book."))?;
    if !money.is_cash() && money.kind != KindRef::Kind("loan_payable".into()) {
        return Err(error(
            "invalid_args",
            format!("{} is not a bank, cash or card account.", money.name),
        ));
    }
    let category = books::account(conn, u.category_account_id)?
        .ok_or_else(|| error("store", "The document's category is missing."))?;
    // cash: the income or expense lands now; accrual: the trade balance clears
    let other = if book.basis == Basis::Accrual {
        trade_account(conn, book, &u.side)?
    } else {
        category.id
    };
    let facts = if other == category.id {
        sales_facts(&category.kind, &u.side)
    } else {
        Facts::default()
    };
    let (form, lines) = if u.side == "collect" {
        (
            Form::MoneyIn,
            vec![
                LineInput {
                    account_id: money.id,
                    debit: u.amount,
                    credit: 0,
                    facts: Facts::default(),
                },
                LineInput {
                    account_id: other,
                    debit: 0,
                    credit: u.amount,
                    facts,
                },
            ],
        )
    } else {
        (
            Form::MoneyOut,
            vec![
                LineInput {
                    account_id: other,
                    debit: u.amount,
                    credit: 0,
                    facts,
                },
                LineInput {
                    account_id: money.id,
                    debit: 0,
                    credit: u.amount,
                    facts: Facts::default(),
                },
            ],
        )
    };
    let payee = match u.person_id {
        Some(p) => person(conn, p)?.map(|p| p.name),
        None => None,
    };
    let draft = DraftInput {
        date: on,
        form,
        payee,
        memo: u.memo.clone(),
        reference: u.reference.clone(),
        needs_review: false,
        lines,
    };
    let entry = entries::save_draft(conn, book, None, &draft, "app")?;
    conn.execute(
        "UPDATE entry SET person_id = ?2 WHERE entry_id = ?1",
        params![entry, u.person_id],
    )?;
    entries::post(conn, book, entry, gate)?;
    conn.execute(
        "UPDATE unpaid SET status = 'paid', paid_entry_id = ?2, paid_on = ?3 WHERE unpaid_id = ?1",
        params![id, entry, on.to_string()],
    )?;
    Ok(entry)
}

/// Cancel an open document. An accrual book voids its receivable entry.
pub fn cancel(conn: &Connection, book: &Book, id: i64, gate: Gate<'_>) -> Result<()> {
    let u = unpaid(conn, id)?
        .filter(|u| u.book_id == book.id)
        .ok_or_else(|| error("not_found", "That document is not in this book."))?;
    if u.status != "open" {
        return Err(error("not_open", "Only an open document can be cancelled."));
    }
    if let Some(e) = u.accrual_entry_id {
        let entry = entries::get(conn, e)?
            .ok_or_else(|| error("store", "The document's entry is missing."))?;
        entries::void(conn, book, e, entry.date, gate)?;
    }
    conn.execute(
        "UPDATE unpaid SET status = 'cancelled' WHERE unpaid_id = ?1",
        params![id],
    )?;
    Ok(())
}
