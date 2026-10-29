//! Entries and their lines. A draft can be saved half done; posting runs
//! every ledger rule ([`books_core::ledger::check`]) on the book's basis. A
//! posted entry never changes: a void marks it `voided` and posts a
//! mirrored reversal dated like the original (DD-15), so both stay in the
//! ledger and the totals skip the pair.

use books_core::catalog::Facts;
use books_core::date::Date;
use books_core::ledger::{self, Status};
use books_core::privacy;
use rusqlite::{Connection, OptionalExtension as _, Row, params};

use crate::books::{self, Book};
use crate::{Result, StoreError, error, now_iso};

/// How the entry was made (the new-entry sheet's modes, and reversals).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    MoneyIn,
    MoneyOut,
    Transfer,
    Journal,
    Reversal,
}

impl Form {
    pub fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "money_in" => Self::MoneyIn,
            "money_out" => Self::MoneyOut,
            "transfer" => Self::Transfer,
            "journal" => Self::Journal,
            "reversal" => Self::Reversal,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::MoneyIn => "money_in",
            Self::MoneyOut => "money_out",
            Self::Transfer => "transfer",
            Self::Journal => "journal",
            Self::Reversal => "reversal",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub id: i64,
    pub book_id: i64,
    pub no: i64,
    pub date: Date,
    pub status: Status,
    pub form: Form,
    pub payee: Option<String>,
    pub memo: Option<String>,
    pub reference: Option<String>,
    pub reverses: Option<i64>,
    pub voided_by: Option<i64>,
    pub origin: String,
    pub needs_review: bool,
    pub created_at: String,
    pub posted_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LineRow {
    pub id: i64,
    pub no: i64,
    pub account_id: i64,
    pub debit: i64,
    pub credit: i64,
    /// The line's own facts; the account's kind supplies the rest.
    pub own: Facts,
}

#[derive(Debug, Clone)]
pub struct LineInput {
    pub account_id: i64,
    pub debit: i64,
    pub credit: i64,
    pub facts: Facts,
}

#[derive(Debug, Clone)]
pub struct DraftInput {
    pub date: Date,
    pub form: Form,
    pub payee: Option<String>,
    pub memo: Option<String>,
    pub reference: Option<String>,
    pub needs_review: bool,
    pub lines: Vec<LineInput>,
}

const ENTRY_COLUMNS: &str = "entry_id, book_id, entry_no, entry_date, status, form, payee, memo, reference, reverses_entry_id, voided_by_entry_id, origin, needs_review, created_at, posted_at";

fn entry_row(r: &Row<'_>) -> rusqlite::Result<Entry> {
    let date: String = r.get(3)?;
    let status: String = r.get(4)?;
    let form: String = r.get(5)?;
    Ok(Entry {
        id: r.get(0)?,
        book_id: r.get(1)?,
        no: r.get(2)?,
        date: Date::parse(&date).unwrap_or(Date {
            year: 1970,
            month: 1,
            day: 1,
        }),
        status: Status::parse(&status).unwrap_or(Status::Draft),
        form: Form::parse(&form).unwrap_or(Form::Journal),
        payee: r.get(6)?,
        memo: r.get(7)?,
        reference: r.get(8)?,
        reverses: r.get(9)?,
        voided_by: r.get(10)?,
        origin: r.get(11)?,
        needs_review: r.get::<_, i64>(12)? == 1,
        created_at: r.get(13)?,
        posted_at: r.get(14)?,
    })
}

/// Free text a person types: at most `max` characters, no TIN.
pub fn check_text(field: &str, text: Option<&str>, max: usize) -> Result<Option<String>> {
    let Some(text) = text.map(str::trim).filter(|t| !t.is_empty()) else {
        return Ok(None);
    };
    if text.chars().count() > max {
        return Err(error(
            "invalid_args",
            format!("The {field} is at most {max} characters."),
        ));
    }
    privacy::refuse_tin(field, text).map_err(|m| error("tin_refused", m))?;
    Ok(Some(text.to_string()))
}

pub fn get(conn: &Connection, id: i64) -> Result<Option<Entry>> {
    Ok(conn
        .query_row(
            &format!("SELECT {ENTRY_COLUMNS} FROM entry WHERE entry_id = ?1"),
            params![id],
            entry_row,
        )
        .optional()?)
}

pub fn lines(conn: &Connection, entry_id: i64) -> Result<Vec<LineRow>> {
    let mut stmt = conn.prepare(
        "SELECT line_id, line_no, account_id, debit, credit, facts FROM line WHERE entry_id = ?1 ORDER BY line_no",
    )?;
    let rows = stmt.query_map(params![entry_id], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, i64>(2)?,
            r.get::<_, i64>(3)?,
            r.get::<_, i64>(4)?,
            r.get::<_, String>(5)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (id, no, account_id, debit, credit, facts) = row?;
        let own = serde_json::from_str(&facts)
            .ok()
            .and_then(|v| Facts::from_json(&v).ok())
            .unwrap_or_default();
        out.push(LineRow {
            id,
            no,
            account_id,
            debit,
            credit,
            own,
        });
    }
    Ok(out)
}

/// The entry, which must belong to `book` (the tenancy check of one Mac).
fn owned(conn: &Connection, book: &Book, id: i64) -> Result<Entry> {
    get(conn, id)?
        .filter(|e| e.book_id == book.id)
        .ok_or_else(|| error("not_found", "That entry is not in this book."))
}

fn check_lines_input(conn: &Connection, book: &Book, lines: &[LineInput]) -> Result<()> {
    if lines.len() > 200 {
        return Err(error("invalid_args", "An entry has at most 200 lines."));
    }
    for (i, l) in lines.iter().enumerate() {
        let account = books::account(conn, l.account_id)?
            .filter(|a| a.book_id == book.id)
            .ok_or_else(|| {
                error(
                    "invalid_args",
                    format!("Line {}: that account is not in this book.", i + 1),
                )
            })?;
        if !account.active {
            return Err(error(
                "invalid_args",
                format!("Line {}: {} is closed.", i + 1, account.name),
            ));
        }
        if l.debit < 0 || l.credit < 0 {
            return Err(error(
                "line_sides",
                format!("Line {} has a negative amount.", i + 1),
            ));
        }
    }
    Ok(())
}

fn write_lines(conn: &Connection, entry_id: i64, lines: &[LineInput]) -> Result<()> {
    conn.execute("DELETE FROM line WHERE entry_id = ?1", params![entry_id])?;
    let mut insert = conn.prepare(
        "INSERT INTO line (entry_id, line_no, account_id, debit, credit, facts) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    for (i, l) in lines.iter().enumerate() {
        let no = i64::try_from(i + 1).unwrap_or(i64::MAX);
        insert.execute(params![
            entry_id,
            no,
            l.account_id,
            l.debit,
            l.credit,
            l.facts.to_json().to_string()
        ])?;
    }
    Ok(())
}

/// Save a new draft, or replace a draft's content. Returns the entry id.
pub fn save_draft(
    conn: &Connection,
    book: &Book,
    existing: Option<i64>,
    input: &DraftInput,
    origin: &str,
) -> Result<i64> {
    let payee = check_text("payee", input.payee.as_deref(), 80)?;
    let memo = check_text("memo", input.memo.as_deref(), 500)?;
    let reference = check_text("reference", input.reference.as_deref(), 60)?;
    if input.form == Form::Reversal {
        return Err(error(
            "invalid_args",
            "A reversal is made by voiding an entry.",
        ));
    }
    check_lines_input(conn, book, &input.lines)?;
    let id = match existing {
        Some(id) => {
            let entry = owned(conn, book, id)?;
            if entry.status != Status::Draft {
                return Err(error(
                    "not_draft",
                    "Only a draft can be edited. A posted entry changes by a void.",
                ));
            }
            conn.execute(
                "UPDATE entry SET entry_date = ?2, form = ?3, payee = ?4, memo = ?5, reference = ?6, needs_review = ?7 WHERE entry_id = ?1",
                params![
                    id,
                    input.date.to_string(),
                    input.form.as_str(),
                    payee,
                    memo,
                    reference,
                    i64::from(input.needs_review)
                ],
            )?;
            id
        }
        None => {
            let no: i64 = conn.query_row(
                "SELECT coalesce(max(entry_no), 0) + 1 FROM entry WHERE book_id = ?1",
                params![book.id],
                |r| r.get(0),
            )?;
            conn.execute(
                "INSERT INTO entry (book_id, entry_no, entry_date, status, form, payee, memo, reference, origin, needs_review, created_at)
                 VALUES (?1, ?2, ?3, 'draft', ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    book.id,
                    no,
                    input.date.to_string(),
                    input.form.as_str(),
                    payee,
                    memo,
                    reference,
                    origin,
                    i64::from(input.needs_review),
                    now_iso()
                ],
            )?;
            conn.last_insert_rowid()
        }
    };
    write_lines(conn, id, &input.lines)?;
    Ok(id)
}

/// The lines as the ledger rules see them: each with its account's kind
/// and its effective facts.
pub fn core_lines(conn: &Connection, rows: &[LineRow]) -> Result<Vec<ledger::Line>> {
    rows.iter()
        .map(|l| {
            let account = books::account(conn, l.account_id)?
                .ok_or_else(|| error("store", "A line names a missing account."))?;
            Ok(ledger::Line {
                account_id: l.account_id,
                facts: account.kind.facts_for(&l.own),
                kind: account.kind,
                debit: l.debit,
                credit: l.credit,
            })
        })
        .collect()
}

/// A period gate the caller supplies (closed months refuse changes; M4).
pub type Gate<'a> = &'a dyn Fn(Date) -> Result<()>;

/// Post a draft: every ledger rule on the book's basis, then `posted`.
pub fn post(conn: &Connection, book: &Book, id: i64, gate: Gate<'_>) -> Result<()> {
    let entry = owned(conn, book, id)?;
    match entry.status {
        Status::Draft => {}
        Status::Posted | Status::Voided => {
            return Err(error("already_posted", "That entry is already posted."));
        }
        Status::Discarded => return Err(error("discarded", "That draft was discarded.")),
    }
    gate(entry.date)?;
    let rows = lines(conn, id)?;
    let core = core_lines(conn, &rows)?;
    ledger::check(&core, book.basis).map_err(StoreError::from)?;
    conn.execute(
        "UPDATE entry SET status = 'posted', needs_review = 0, posted_at = ?2 WHERE entry_id = ?1",
        params![id, now_iso()],
    )?;
    Ok(())
}

/// Void a posted entry: mark it `voided` and post its mirrored reversal on
/// `date` (the original's date while its month is open). Returns the
/// reversal's id.
pub fn void(conn: &Connection, book: &Book, id: i64, date: Date, gate: Gate<'_>) -> Result<i64> {
    let entry = owned(conn, book, id)?;
    match entry.status {
        Status::Posted if entry.form != Form::Reversal => {}
        Status::Posted => {
            return Err(error("is_reversal", "A reversal cannot be voided."));
        }
        Status::Voided => return Err(error("already_voided", "That entry is already voided.")),
        Status::Draft | Status::Discarded => {
            return Err(error(
                "not_posted",
                "Only a posted entry can be voided. Discard a draft instead.",
            ));
        }
    }
    if date < entry.date {
        return Err(error(
            "invalid_args",
            "A reversal cannot be dated before the entry it voids.",
        ));
    }
    gate(date)?;
    let rows = lines(conn, id)?;
    let core = core_lines(conn, &rows)?;
    let mirrored = ledger::mirror(&core);
    ledger::check(&mirrored, book.basis).map_err(StoreError::from)?;
    let no: i64 = conn.query_row(
        "SELECT coalesce(max(entry_no), 0) + 1 FROM entry WHERE book_id = ?1",
        params![book.id],
        |r| r.get(0),
    )?;
    let now = now_iso();
    conn.execute(
        "INSERT INTO entry (book_id, entry_no, entry_date, status, form, payee, memo, reference, reverses_entry_id, origin, created_at, posted_at)
         VALUES (?1, ?2, ?3, 'posted', 'reversal', ?4, NULL, ?5, ?6, 'app', ?7, ?7)",
        params![
            book.id,
            no,
            date.to_string(),
            entry.payee,
            entry.reference,
            id,
            now
        ],
    )?;
    let reversal = conn.last_insert_rowid();
    let inputs: Vec<LineInput> = rows
        .iter()
        .map(|l| LineInput {
            account_id: l.account_id,
            debit: l.credit,
            credit: l.debit,
            facts: l.own.clone(),
        })
        .collect();
    write_lines(conn, reversal, &inputs)?;
    conn.execute(
        "UPDATE entry SET status = 'voided', voided_by_entry_id = ?2 WHERE entry_id = ?1",
        params![id, reversal],
    )?;
    // the reversal belongs to the same person and tags
    conn.execute(
        "UPDATE entry SET (person_id, department_id, location_id) =
           (SELECT person_id, department_id, location_id FROM entry WHERE entry_id = ?1)
         WHERE entry_id = ?2",
        params![id, reversal],
    )?;
    Ok(reversal)
}

/// Discard a draft. It leaves every list and total.
pub fn discard(conn: &Connection, book: &Book, id: i64) -> Result<()> {
    let entry = owned(conn, book, id)?;
    if entry.status != Status::Draft {
        return Err(error(
            "not_draft",
            "Only a draft can be discarded. Void a posted entry instead.",
        ));
    }
    conn.execute(
        "UPDATE entry SET status = 'discarded' WHERE entry_id = ?1",
        params![id],
    )?;
    Ok(())
}

/// What a list asks for.
#[derive(Debug, Clone, Default)]
pub struct Filter {
    pub from: Option<Date>,
    pub to: Option<Date>,
    pub account_id: Option<i64>,
    /// Include discarded drafts (never by default).
    pub discarded: bool,
}

/// Entries of a book, newest first.
pub fn list(conn: &Connection, book_id: i64, filter: &Filter) -> Result<Vec<Entry>> {
    let from = filter
        .from
        .map_or_else(|| "0000-00-00".into(), |d| d.to_string());
    let to = filter
        .to
        .map_or_else(|| "9999-99-99".into(), |d| d.to_string());
    let account = filter.account_id.unwrap_or(0);
    let sql = format!(
        "SELECT {ENTRY_COLUMNS} FROM entry e WHERE book_id = ?1 AND entry_date >= ?2 AND entry_date <= ?3
           AND (?4 = 0 OR EXISTS (SELECT 1 FROM line l WHERE l.entry_id = e.entry_id AND l.account_id = ?4))
           AND (?5 = 1 OR status <> 'discarded')
         ORDER BY entry_date DESC, entry_no DESC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(
        params![book_id, from, to, account, i64::from(filter.discarded)],
        entry_row,
    )?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Paths, Store, TempDir};
    use books_core::ledger::Basis;

    fn open(tag: &str) -> (TempDir, Store, Book) {
        let tmp = TempDir::new(tag);
        let store = Store::open(&Paths::in_dir(tmp.0.join("d"))).unwrap();
        let id = books::create(store.conn(), "Test Book", Basis::Cash, 1).unwrap();
        let book = books::get(store.conn(), id).unwrap().unwrap();
        (tmp, store, book)
    }

    fn acct(store: &Store, book: &Book, number: &str) -> i64 {
        books::account_by_number(store.conn(), book.id, number)
            .unwrap()
            .unwrap()
            .id
    }

    fn sale(store: &Store, book: &Book, amount: i64) -> DraftInput {
        DraftInput {
            date: Date::new(2026, 10, 28).unwrap(),
            form: Form::MoneyIn,
            payee: Some("Bayani Foods".into()),
            memo: Some("MEMO-SYNTH-sale".into()),
            reference: Some("Invoice 0142".into()),
            needs_review: false,
            lines: vec![
                LineInput {
                    account_id: acct(store, book, "11000"),
                    debit: amount,
                    credit: 0,
                    facts: Facts::default(),
                },
                LineInput {
                    account_id: acct(store, book, "41000"),
                    debit: 0,
                    credit: amount,
                    facts: Facts::default(),
                },
            ],
        }
    }

    const OPEN: Gate<'static> = &|_| Ok(());

    #[test]
    fn draft_post_void_keeps_both_entries_and_nets_the_balances() {
        let (_t, store, book) = open("post");
        let c = store.conn();
        let id = save_draft(c, &book, None, &sale(&store, &book, 8_000_000), "app").unwrap();
        assert_eq!(get(c, id).unwrap().unwrap().status, Status::Draft);
        let bank = acct(&store, &book, "11000");
        assert!(
            !books::movements(c, book.id, None, None)
                .unwrap()
                .contains_key(&bank),
            "a draft is in no total"
        );
        post(c, &book, id, OPEN).unwrap();
        assert_eq!(
            books::movements(c, book.id, None, None).unwrap()[&bank],
            8_000_000
        );
        assert_eq!(post(c, &book, id, OPEN).unwrap_err().code, "already_posted");
        let rev = void(c, &book, id, Date::new(2026, 10, 28).unwrap(), OPEN).unwrap();
        let original = get(c, id).unwrap().unwrap();
        assert_eq!(original.status, Status::Voided);
        assert_eq!(original.voided_by, Some(rev));
        let reversal = get(c, rev).unwrap().unwrap();
        assert_eq!(
            (reversal.status, reversal.form, reversal.reverses),
            (Status::Posted, Form::Reversal, Some(id))
        );
        assert_eq!(
            reversal.date, original.date,
            "dated like the original (DD-15)"
        );
        assert_eq!(books::movements(c, book.id, None, None).unwrap()[&bank], 0);
        assert_eq!(
            void(c, &book, id, original.date, OPEN).unwrap_err().code,
            "already_voided"
        );
        assert_eq!(
            void(c, &book, rev, original.date, OPEN).unwrap_err().code,
            "is_reversal"
        );
        assert_eq!(list(c, book.id, &Filter::default()).unwrap().len(), 2);
    }

    #[test]
    fn posting_refuses_an_unbalanced_draft_and_a_cash_receivable() {
        let (_t, store, book) = open("refuse");
        let c = store.conn();
        let mut bad = sale(&store, &book, 10_000);
        bad.lines[1].credit = 9_999;
        let id = save_draft(c, &book, None, &bad, "agent").unwrap();
        assert_eq!(post(c, &book, id, OPEN).unwrap_err().code, "unbalanced");
        assert_eq!(get(c, id).unwrap().unwrap().status, Status::Draft);
        let mut ar = sale(&store, &book, 10_000);
        ar.lines[0].account_id = acct(&store, &book, "12010");
        let id = save_draft(c, &book, None, &ar, "app").unwrap();
        assert_eq!(
            post(c, &book, id, OPEN).unwrap_err().code,
            "accrual_account"
        );
    }

    #[test]
    fn drafts_edit_and_discard_but_posted_entries_do_not() {
        let (_t, store, book) = open("edit");
        let c = store.conn();
        let id = save_draft(c, &book, None, &sale(&store, &book, 100), "agent").unwrap();
        save_draft(c, &book, Some(id), &sale(&store, &book, 200), "agent").unwrap();
        assert_eq!(lines(c, id).unwrap()[0].debit, 200);
        discard(c, &book, id).unwrap();
        assert!(list(c, book.id, &Filter::default()).unwrap().is_empty());
        let id = save_draft(c, &book, None, &sale(&store, &book, 100), "app").unwrap();
        post(c, &book, id, OPEN).unwrap();
        assert_eq!(
            save_draft(c, &book, Some(id), &sale(&store, &book, 1), "app")
                .unwrap_err()
                .code,
            "not_draft"
        );
        assert_eq!(discard(c, &book, id).unwrap_err().code, "not_draft");
    }

    #[test]
    fn a_tin_in_a_memo_is_refused_and_a_closed_month_gate_holds() {
        let (_t, store, book) = open("tin");
        let c = store.conn();
        let mut s = sale(&store, &book, 100);
        s.memo = Some(format!("payee tin {}-000", ["123", "456", "789"].join("-")));
        assert_eq!(
            save_draft(c, &book, None, &s, "app").unwrap_err().code,
            "tin_refused"
        );
        let id = save_draft(c, &book, None, &sale(&store, &book, 100), "app").unwrap();
        let closed: Gate<'_> = &|_| Err(error("period_closed", "October is closed."));
        assert_eq!(
            post(c, &book, id, closed).unwrap_err().code,
            "period_closed"
        );
    }
}
