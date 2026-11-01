//! M4: month locks (DD-16), bank statements and their matches, giving
//! "not sure yet" lines a category, the book's default categories and its
//! settings.
//!
//! A closed month refuses every post and void dated in it; reopening needs
//! a reason, and every close and reopen is a `period_event`. A statement
//! belongs to one account and month; it finishes only when every line is
//! matched and the difference is 0.00.

use books_core::catalog::Facts;
use books_core::date::{Date, Period, PeriodKind};
use books_core::ledger::{Basis, KindRef, Status};
use books_core::statement::{self, Candidate, Match};
use rusqlite::{Connection, OptionalExtension as _, params};

use crate::books::{self, Account, Book};
use crate::entries::{self, DraftInput, Form, Gate, LineInput, check_text};
use crate::{Result, error, now_iso};

fn month_key(d: Date) -> String {
    Period::of(d, PeriodKind::Month).key
}

pub fn is_closed(conn: &Connection, book_id: i64, key: &str) -> Result<bool> {
    Ok(conn
        .query_row(
            "SELECT status = 'closed' FROM period WHERE book_id = ?1 AND period_key = ?2",
            params![book_id, key],
            |r| r.get::<_, bool>(0),
        )
        .optional()?
        .unwrap_or(false))
}

/// Refuse a change dated in a closed month.
pub fn check_open(conn: &Connection, book_id: i64, d: Date) -> Result<()> {
    if is_closed(conn, book_id, &month_key(d))? {
        let p = Period::of(d, PeriodKind::Month);
        return Err(error(
            "period_closed",
            format!(
                "{} is closed. Reopen it in Settings › Months, with a reason, to change it.",
                p.label()
            ),
        ));
    }
    Ok(())
}

/// Close a month. Drafts dated in it must be posted or discarded first.
pub fn close(conn: &Connection, book: &Book, key: &str, reason: Option<&str>) -> Result<()> {
    let p = Period::parse(key)
        .filter(|p| p.kind == PeriodKind::Month)
        .ok_or_else(|| error("invalid_args", "A month is YYYY-MM."))?;
    if is_closed(conn, book.id, key)? {
        return Err(error(
            "already_closed",
            format!("{} is already closed.", p.label()),
        ));
    }
    let drafts: i64 = conn.query_row(
        "SELECT count(*) FROM entry WHERE book_id = ?1 AND status = 'draft' AND entry_date >= ?2 AND entry_date <= ?3",
        params![book.id, p.start.to_string(), p.end.to_string()],
        |r| r.get(0),
    )?;
    if drafts > 0 {
        return Err(error(
            "drafts_open",
            format!(
                "{} has {drafts} draft{} dated in it. Post or discard {} first.",
                p.label(),
                if drafts == 1 { "" } else { "s" },
                if drafts == 1 { "it" } else { "them" }
            ),
        ));
    }
    let reason = check_text("reason", reason, 200)?;
    let now = now_iso();
    conn.execute(
        "INSERT INTO period (book_id, period_key, status, closed_at) VALUES (?1, ?2, 'closed', ?3)
         ON CONFLICT (book_id, period_key) DO UPDATE SET status = 'closed', closed_at = excluded.closed_at",
        params![book.id, key, now],
    )?;
    conn.execute(
        "INSERT INTO period_event (book_id, period_key, event, reason, at) VALUES (?1, ?2, 'close', ?3, ?4)",
        params![book.id, key, reason, now],
    )?;
    Ok(())
}

/// Reopen a closed month: a reason is required (DD-16).
pub fn reopen(conn: &Connection, book: &Book, key: &str, reason: &str) -> Result<()> {
    if !is_closed(conn, book.id, key)? {
        return Err(error("not_closed", "That month is not closed."));
    }
    let reason = check_text("reason", Some(reason), 200)?
        .ok_or_else(|| error("reason_needed", "Reopening a month needs a reason."))?;
    let now = now_iso();
    conn.execute(
        "UPDATE period SET status = 'open' WHERE book_id = ?1 AND period_key = ?2",
        params![book.id, key],
    )?;
    conn.execute(
        "INSERT INTO period_event (book_id, period_key, event, reason, at) VALUES (?1, ?2, 'reopen', ?3, ?4)",
        params![book.id, key, reason, now],
    )?;
    Ok(())
}

#[derive(Debug, Clone)]
pub struct Event {
    pub period_key: String,
    pub event: String,
    pub reason: Option<String>,
    pub at: String,
}

pub fn events(conn: &Connection, book_id: i64) -> Result<Vec<Event>> {
    let mut stmt = conn.prepare(
        "SELECT period_key, event, reason, at FROM period_event WHERE book_id = ?1 ORDER BY event_id DESC",
    )?;
    let rows = stmt.query_map(params![book_id], |r| {
        Ok(Event {
            period_key: r.get(0)?,
            event: r.get(1)?,
            reason: r.get(2)?,
            at: r.get(3)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// The book's months, from its first entry (or January of `today`'s year)
/// to `today`'s month, oldest first, with whether each is closed.
pub fn months(conn: &Connection, book_id: i64, today: Date) -> Result<Vec<(Period, bool)>> {
    let first: Option<String> = conn.query_row(
        "SELECT min(entry_date) FROM entry WHERE book_id = ?1 AND status <> 'discarded'",
        params![book_id],
        |r| r.get(0),
    )?;
    let start = first
        .and_then(|d| Date::parse(&d))
        .unwrap_or_else(|| Date::new(today.year, 1, 1).unwrap_or(today))
        .min(today);
    let mut out = Vec::new();
    let mut p = Period::of(start, PeriodKind::Month);
    let last = Period::of(today, PeriodKind::Month);
    while p.start <= last.start && out.len() < 240 {
        let closed = is_closed(conn, book_id, &p.key)?;
        let next = Period::of(p.end.add_days(1), PeriodKind::Month);
        out.push((p, closed));
        p = next;
    }
    Ok(out)
}

// --- statements -------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Statement {
    pub id: i64,
    pub account_id: i64,
    pub period_key: String,
    pub closing: i64,
    pub source_name: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct StatementLine {
    pub id: i64,
    pub line: statement::Line,
    pub matched_entry_id: Option<i64>,
    pub match_kind: Option<String>,
}

/// Store a statement for `account` and the month of its lines, replacing
/// an unfinished one. `closing` is the ending balance (the last running
/// balance when the CSV has one).
pub fn import(
    conn: &Connection,
    book: &Book,
    account: &Account,
    parsed: &statement::Parsed,
    closing: Option<i64>,
    source_name: Option<&str>,
) -> Result<i64> {
    if account.book_id != book.id
        || !(account.is_cash() || account.kind == KindRef::Kind("loan_payable".into()))
    {
        return Err(error(
            "invalid_args",
            "A statement belongs to a bank, cash or card account.",
        ));
    }
    let first = parsed
        .lines
        .first()
        .ok_or_else(|| error("invalid_args", "The statement has no lines."))?;
    let key = month_key(first.date);
    if parsed.lines.iter().any(|l| month_key(l.date) != key) {
        return Err(error(
            "invalid_args",
            "A statement covers one month; this one spans more.",
        ));
    }
    let closing = closing
        .or(parsed.closing)
        .ok_or_else(|| error("closing_needed", "Type the statement's ending balance."))?;
    let source = check_text("file name", source_name, 120)?;
    let existing: Option<(i64, String)> = conn
        .query_row(
            "SELECT statement_id, status FROM statement WHERE book_id = ?1 AND account_id = ?2 AND period_key = ?3",
            params![book.id, account.id, key],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    if let Some((id, status)) = existing {
        if status == "finished" {
            return Err(error(
                "already_finished",
                "That month is already matched to the bank.",
            ));
        }
        conn.execute(
            "DELETE FROM statement_line WHERE statement_id = ?1",
            params![id],
        )?;
        conn.execute("DELETE FROM statement WHERE statement_id = ?1", params![id])?;
    }
    conn.execute(
        "INSERT INTO statement (book_id, account_id, period_key, closing, source_name, imported_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![book.id, account.id, key, closing, source, now_iso()],
    )?;
    let id = conn.last_insert_rowid();
    let mut insert = conn.prepare(
        "INSERT INTO statement_line (statement_id, line_no, line_date, description, amount) VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    for (i, l) in parsed.lines.iter().enumerate() {
        insert.execute(params![
            id,
            i64::try_from(i + 1).unwrap_or(0),
            l.date.to_string(),
            l.description,
            l.amount
        ])?;
    }
    Ok(id)
}

pub fn statement(
    conn: &Connection,
    book_id: i64,
    account_id: i64,
    key: &str,
) -> Result<Option<(Statement, Vec<StatementLine>)>> {
    let s: Option<Statement> = conn
        .query_row(
            "SELECT statement_id, account_id, period_key, closing, source_name, status FROM statement WHERE book_id = ?1 AND account_id = ?2 AND period_key = ?3",
            params![book_id, account_id, key],
            |r| {
                Ok(Statement {
                    id: r.get(0)?,
                    account_id: r.get(1)?,
                    period_key: r.get(2)?,
                    closing: r.get(3)?,
                    source_name: r.get(4)?,
                    status: r.get(5)?,
                })
            },
        )
        .optional()?;
    let Some(s) = s else { return Ok(None) };
    let mut stmt = conn.prepare(
        "SELECT line_id, line_date, description, amount, matched_entry_id, match_kind FROM statement_line WHERE statement_id = ?1 ORDER BY line_no",
    )?;
    let rows = stmt.query_map(params![s.id], |r| {
        let d: String = r.get(1)?;
        Ok(StatementLine {
            id: r.get(0)?,
            line: statement::Line {
                date: Date::parse(&d).unwrap_or(Date {
                    year: 1970,
                    month: 1,
                    day: 1,
                }),
                description: r.get(2)?,
                amount: r.get(3)?,
            },
            matched_entry_id: r.get(4)?,
            match_kind: r.get(5)?,
        })
    })?;
    let lines = rows.collect::<rusqlite::Result<_>>()?;
    Ok(Some((s, lines)))
}

pub fn statements(conn: &Connection, book_id: i64) -> Result<Vec<Statement>> {
    let mut stmt = conn.prepare(
        "SELECT statement_id, account_id, period_key, closing, source_name, status FROM statement WHERE book_id = ?1",
    )?;
    let rows = stmt.query_map(params![book_id], |r| {
        Ok(Statement {
            id: r.get(0)?,
            account_id: r.get(1)?,
            period_key: r.get(2)?,
            closing: r.get(3)?,
            source_name: r.get(4)?,
            status: r.get(5)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// The ledger's movements on `account` in `period`: each posted entry and
/// its effect on the account (a void and its reversal both stay, so they
/// can match a bank line and its correction).
pub fn candidates(
    conn: &Connection,
    book_id: i64,
    account_id: i64,
    period: &Period,
) -> Result<Vec<Candidate>> {
    let mut stmt = conn.prepare(
        "SELECT e.entry_id, e.entry_date, sum(l.debit) - sum(l.credit)
         FROM entry e JOIN line l ON l.entry_id = e.entry_id
         WHERE e.book_id = ?1 AND l.account_id = ?2 AND e.status IN ('posted', 'voided')
           AND e.entry_date >= ?3 AND e.entry_date <= ?4
         GROUP BY e.entry_id ORDER BY e.entry_date, e.entry_no",
    )?;
    let rows = stmt.query_map(
        params![
            book_id,
            account_id,
            period.start.to_string(),
            period.end.to_string()
        ],
        |r| {
            let d: String = r.get(1)?;
            Ok(Candidate {
                entry_id: r.get(0)?,
                date: Date::parse(&d).unwrap_or(Date {
                    year: 1970,
                    month: 1,
                    day: 1,
                }),
                amount: r.get(2)?,
            })
        },
    )?;
    Ok(rows
        .collect::<rusqlite::Result<Vec<_>>>()?
        .into_iter()
        .filter(|c| c.amount != 0)
        .collect())
}

/// The matches of a statement now: stored ones stand, the rest computed.
pub fn matches(
    conn: &Connection,
    book_id: i64,
    s: &Statement,
    lines: &[StatementLine],
) -> Result<Vec<Match>> {
    let period = Period::parse(&s.period_key)
        .ok_or_else(|| error("store", "A statement's month is not a month."))?;
    let cands = candidates(conn, book_id, s.account_id, &period)?;
    let plain: Vec<statement::Line> = lines.iter().map(|l| l.line.clone()).collect();
    let kept: Vec<Option<i64>> = lines.iter().map(|l| l.matched_entry_id).collect();
    let hints = crate::match_history::hints(conn, book_id, s.account_id)?;
    let ids: Vec<i64> = cands.iter().map(|c| c.entry_id).collect();
    let payees = crate::match_history::payees(conn, &ids)?;
    Ok(books_core::match_assist::rank(
        &plain, &kept, &cands, &payees, &hints,
    ))
}

pub(crate) fn set_match(conn: &Connection, line_id: i64, entry_id: i64, kind: &str) -> Result<()> {
    conn.execute(
        "UPDATE statement_line SET matched_entry_id = ?2, match_kind = ?3 WHERE line_id = ?1",
        params![line_id, entry_id, kind],
    )?;
    Ok(())
}

pub(crate) fn open_statement(
    conn: &Connection,
    book: &Book,
    account_id: i64,
    key: &str,
) -> Result<(Statement, Vec<StatementLine>)> {
    let (s, lines) = statement(conn, book.id, account_id, key)?.ok_or_else(|| {
        error(
            "not_found",
            "There is no statement for that account and month.",
        )
    })?;
    if s.status == "finished" {
        return Err(error(
            "already_finished",
            "That month is already matched to the bank.",
        ));
    }
    Ok((s, lines))
}

/// Accept every suggested match. Returns how many were accepted.
pub fn accept_suggestions(
    conn: &Connection,
    book: &Book,
    account_id: i64,
    key: &str,
) -> Result<usize> {
    let (s, lines) = open_statement(conn, book, account_id, key)?;
    let found = matches(conn, book.id, &s, &lines)?;
    let mut n = 0;
    for (l, m) in lines.iter().zip(found) {
        if let Match::Suggested(e) = m {
            set_match(conn, l.id, e, "accepted")?;
            n += 1;
        }
    }
    Ok(n)
}

/// The account a missing bank line goes to: the category past matches of
/// its description used, else the kind the bank's words name, else the
/// book's "not sure yet" default for its direction.
pub fn account_for_line(
    conn: &Connection,
    book: &Book,
    hint: Option<&books_core::match_assist::Hint>,
    line: &statement::Line,
) -> Result<Account> {
    let accounts = books::accounts(conn, book.id)?;
    if let Some(id) = hint.and_then(|h| h.category_account_id)
        && let Some(a) = accounts
            .iter()
            .find(|a| a.id == id && a.active && !(a.accrual_only() && book.basis == Basis::Cash))
    {
        return Ok(a.clone());
    }
    if let Some(kind) = statement::suggest_kind(&line.description, line.amount)
        && let Some(a) = accounts
            .iter()
            .find(|a| a.active && a.kind == KindRef::Kind(kind.into()))
    {
        return Ok(a.clone());
    }
    let slot = if line.amount > 0 {
        "unsure_in"
    } else {
        "unsure_out"
    };
    default_account(conn, book, slot)
}

/// Add a bank line that is not in Books: a posted entry on its date, then
/// matched. Returns the entry id.
pub fn add_line(
    conn: &Connection,
    book: &Book,
    account_id: i64,
    key: &str,
    line_id: i64,
    gate: Gate<'_>,
) -> Result<i64> {
    let (_s, lines) = open_statement(conn, book, account_id, key)?;
    let line = lines
        .iter()
        .find(|l| l.id == line_id)
        .ok_or_else(|| error("not_found", "That line is not on this statement."))?;
    if line.matched_entry_id.is_some() {
        return Err(error("already_matched", "That line is already in Books."));
    }
    let hints = crate::match_history::hints(conn, book.id, account_id)?;
    let hint = books_core::match_assist::hint_for(&hints, &line.line.description);
    let category = account_for_line(conn, book, hint, &line.line)?;
    let hinted_payee = hint.and_then(|h| h.payee.clone());
    let c = line.line.amount.abs();
    let (form, lines_in) = if line.line.amount > 0 {
        (
            Form::MoneyIn,
            vec![
                LineInput {
                    account_id,
                    debit: c,
                    credit: 0,
                    facts: Facts::default(),
                },
                LineInput {
                    account_id: category.id,
                    debit: 0,
                    credit: c,
                    facts: Facts::default(),
                },
            ],
        )
    } else {
        (
            Form::MoneyOut,
            vec![
                LineInput {
                    account_id: category.id,
                    debit: c,
                    credit: 0,
                    facts: Facts::default(),
                },
                LineInput {
                    account_id,
                    debit: 0,
                    credit: c,
                    facts: Facts::default(),
                },
            ],
        )
    };
    let draft = DraftInput {
        date: line.line.date,
        form,
        payee: hinted_payee,
        memo: Some(line.line.description.clone()),
        reference: Some("From bank statement".into()),
        needs_review: false,
        lines: lines_in,
    };
    let entry = entries::save_draft(conn, book, None, &draft, "app")?;
    entries::post(conn, book, entry, gate)?;
    set_match(conn, line.id, entry, "added")?;
    Ok(entry)
}

/// The statement's ending balance less Books' balance on the account at
/// the month's end (in the account's own sense).
pub fn difference(conn: &Connection, book: &Book, s: &Statement) -> Result<(i64, i64)> {
    let period = Period::parse(&s.period_key)
        .ok_or_else(|| error("store", "A statement's month is not a month."))?;
    let account = books::account(conn, s.account_id)?
        .ok_or_else(|| error("store", "The account is missing."))?;
    let m = books::movements(conn, book.id, None, Some(period.end))?;
    let ours = account.natural(*m.get(&account.id).unwrap_or(&0));
    Ok((ours, s.closing - ours))
}

/// Finish a month: every line matched and the difference 0.00.
pub fn finish(conn: &Connection, book: &Book, account_id: i64, key: &str) -> Result<()> {
    let (s, lines) = open_statement(conn, book, account_id, key)?;
    let found = matches(conn, book.id, &s, &lines)?;
    let open = found
        .iter()
        .filter(|m| !matches!(m, Match::Exact(_)))
        .count();
    if open > 0 {
        return Err(error(
            "lines_open",
            format!(
                "{open} line{} still need{} a match: accept the suggestions or add the missing lines.",
                if open == 1 { "" } else { "s" },
                if open == 1 { "s" } else { "" }
            ),
        ));
    }
    let (_, diff) = difference(conn, book, &s)?;
    if diff != 0 {
        return Err(error(
            "difference",
            format!(
                "The difference is {}, not 0.00.",
                books_core::money::display(diff)
            ),
        ));
    }
    for (l, m) in lines.iter().zip(found) {
        if let (None, Match::Exact(e)) = (l.matched_entry_id, m) {
            set_match(conn, l.id, e, "exact")?;
        }
    }
    conn.execute(
        "UPDATE statement SET status = 'finished', finished_at = ?2 WHERE statement_id = ?1",
        params![s.id, now_iso()],
    )?;
    Ok(())
}

// --- recategorize -----------------------------------------------------------

/// Give a posted entry's "not sure yet" lines a category: a reclassifying
/// entry on its date moves each such line's amount to `category`. The
/// original stays as it was (posted entries never change). Returns the
/// reclassifying entry's id.
pub fn recategorize(
    conn: &Connection,
    book: &Book,
    entry_id: i64,
    category: &Account,
    gate: Gate<'_>,
) -> Result<i64> {
    let e = entries::get(conn, entry_id)?
        .filter(|e| e.book_id == book.id)
        .ok_or_else(|| error("not_found", "That entry is not in this book."))?;
    if e.status != Status::Posted
        || e.form == Form::Reversal
        || e.recategorized_by.is_some()
        || e.recategorizes.is_some()
    {
        return Err(error(
            "not_reviewable",
            "Only a posted entry on a \"not sure yet\" account takes a category this way.",
        ));
    }
    if category.book_id != book.id
        || !category.active
        || category.kind == KindRef::Unsure
        || category.is_cash()
    {
        return Err(error("invalid_args", "Pick an income or expense category."));
    }
    let rows = entries::lines(conn, entry_id)?;
    let mut moves = Vec::new();
    for l in &rows {
        let a = books::account(conn, l.account_id)?
            .ok_or_else(|| error("store", "A line names a missing account."))?;
        if a.kind == KindRef::Unsure {
            moves.push(LineInput {
                account_id: category.id,
                debit: l.debit,
                credit: l.credit,
                facts: Facts::default(),
            });
            moves.push(LineInput {
                account_id: a.id,
                debit: l.credit,
                credit: l.debit,
                facts: Facts::default(),
            });
        }
    }
    if moves.is_empty() {
        return Err(error(
            "not_reviewable",
            "That entry has no \"not sure yet\" line.",
        ));
    }
    let draft = DraftInput {
        date: e.date,
        form: Form::Journal,
        payee: e.payee.clone(),
        memo: Some(format!("Category for entry {:04}", e.no)),
        reference: e.reference.clone(),
        needs_review: false,
        lines: moves,
    };
    let id = entries::save_draft(conn, book, None, &draft, "app")?;
    entries::post(conn, book, id, gate)?;
    conn.execute(
        "UPDATE entry SET recategorizes_entry_id = ?2 WHERE entry_id = ?1",
        params![id, entry_id],
    )?;
    conn.execute(
        "UPDATE entry SET recategorized_by_entry_id = ?2 WHERE entry_id = ?1",
        params![entry_id, id],
    )?;
    conn.execute(
        "UPDATE entry SET (person_id, department_id, location_id) = (SELECT person_id, department_id, location_id FROM entry WHERE entry_id = ?1) WHERE entry_id = ?2",
        params![entry_id, id],
    )?;
    Ok(id)
}

// --- defaults and settings --------------------------------------------------

/// The Defaults tab: `(slot, group, label, preset account number, accrual only)`.
pub const SLOTS: [(&str, &str, &str, &str, bool); 10] = [
    (
        "checking",
        "Banks & cards",
        "Checking account",
        "11000",
        false,
    ),
    (
        "credit_card",
        "Banks & cards",
        "Credit card",
        "22000",
        false,
    ),
    ("cash_box", "Banks & cards", "Cash box", "11100", false),
    ("unsure_out", "Money out", "Not sure yet", "69999", false),
    ("rent", "Money out", "Rent and office", "66100", false),
    ("software", "Money out", "Software", "65000", false),
    ("unpaid_bills", "Money out", "Unpaid bills", "21000", true),
    ("sales", "Money in", "Sales", "41000", false),
    ("unsure_in", "Money in", "Not sure yet", "40999", false),
    (
        "unpaid_invoices",
        "Money in",
        "Unpaid invoices",
        "12010",
        true,
    ),
];

pub fn default_account(conn: &Connection, book: &Book, slot: &str) -> Result<Account> {
    let set: Option<i64> = conn
        .query_row(
            "SELECT account_id FROM category_default WHERE book_id = ?1 AND slot = ?2",
            params![book.id, slot],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(a) = set.and_then(|id| books::account(conn, id).ok().flatten()) {
        return Ok(a);
    }
    let number = SLOTS
        .iter()
        .find(|s| s.0 == slot)
        .map(|s| s.3)
        .ok_or_else(|| error("invalid_args", "No such default."))?;
    books::account_by_number(conn, book.id, number)?
        .ok_or_else(|| error("no_account", "The book has no account for that default."))
}

pub fn set_default(conn: &Connection, book: &Book, slot: &str, account: &Account) -> Result<()> {
    let s = SLOTS
        .iter()
        .find(|s| s.0 == slot)
        .ok_or_else(|| error("invalid_args", "No such default."))?;
    if s.4 && book.basis == Basis::Cash {
        return Err(error(
            "accrual_account",
            "That default is for accrual books only.",
        ));
    }
    if account.book_id != book.id || !account.active {
        return Err(error("invalid_args", "That account is not in this book."));
    }
    conn.execute(
        "INSERT INTO category_default (book_id, slot, account_id) VALUES (?1, ?2, ?3)
         ON CONFLICT (book_id, slot) DO UPDATE SET account_id = excluded.account_id",
        params![book.id, slot, account.id],
    )?;
    Ok(())
}

/// Rename the book, move its fiscal year, or change its basis. A basis
/// changes only from the first day of a year with nothing posted yet, and
/// a switch to cash needs zero trade receivable and payable balances then
/// (ADR 0008, "Changing the basis").
pub fn update_book(
    conn: &Connection,
    book: &Book,
    label: Option<&str>,
    fiscal_year_start: Option<u8>,
    basis: Option<Basis>,
    today: Date,
) -> Result<()> {
    if let Some(label) = label {
        let label = books::check_label(label)?;
        conn.execute(
            "UPDATE book SET label = ?2 WHERE book_id = ?1",
            params![book.id, label],
        )?;
    }
    if let Some(m) = fiscal_year_start {
        if !(1..=12).contains(&m) {
            return Err(error(
                "invalid_args",
                "The fiscal year starts in month 1 to 12.",
            ));
        }
        conn.execute(
            "UPDATE book SET fiscal_year_start = ?2 WHERE book_id = ?1",
            params![book.id, i64::from(m)],
        )?;
    }
    if let Some(b) = basis.filter(|b| *b != book.basis) {
        let year_start = Date::new(today.year, 1, 1).unwrap_or(today);
        let posted_this_year: i64 = conn.query_row(
            "SELECT count(*) FROM entry WHERE book_id = ?1 AND status IN ('posted','voided') AND entry_date >= ?2",
            params![book.id, year_start.to_string()],
            |r| r.get(0),
        )?;
        if posted_this_year > 0 {
            return Err(error(
                "basis_locked",
                format!(
                    "The basis changes only from the start of a year with nothing posted; {} already has entries.",
                    today.year
                ),
            ));
        }
        if b == Basis::Cash {
            let m = books::movements(conn, book.id, None, Some(year_start.add_days(-1)))?;
            let trade = books::accounts(conn, book.id)?
                .into_iter()
                .filter(|a| a.accrual_only())
                .any(|a| *m.get(&a.id).unwrap_or(&0) != 0);
            if trade {
                return Err(error(
                    "basis_trade_balances",
                    "Switching to cash needs zero receivable and payable balances at the start of the year.",
                ));
            }
        }
        conn.execute(
            "UPDATE book SET basis = ?2 WHERE book_id = ?1",
            params![book.id, b.as_str()],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Paths, Store, TempDir};

    fn setup() -> (TempDir, Store, Book) {
        let tmp = TempDir::new("months");
        let store = Store::open(&Paths::in_dir(tmp.0.join("d"))).unwrap();
        let id = books::create(store.conn(), "Months Test", Basis::Cash, 1).unwrap();
        let book = books::get(store.conn(), id).unwrap().unwrap();
        (tmp, store, book)
    }

    #[test]
    fn a_closed_month_refuses_changes_and_reopening_needs_a_reason() {
        let (_t, store, book) = setup();
        let c = store.conn();
        let d = Date::new(2026, 9, 15).unwrap();
        close(c, &book, "2026-09", None).unwrap();
        assert_eq!(check_open(c, book.id, d).unwrap_err().code, "period_closed");
        assert!(check_open(c, book.id, Date::new(2026, 10, 1).unwrap()).is_ok());
        assert_eq!(
            reopen(c, &book, "2026-09", "  ").unwrap_err().code,
            "reason_needed"
        );
        reopen(c, &book, "2026-09", "A late bank fee").unwrap();
        assert!(check_open(c, book.id, d).is_ok());
        let ev = events(c, book.id).unwrap();
        assert_eq!(ev.len(), 2);
        assert_eq!(
            (ev[0].event.as_str(), ev[0].reason.as_deref()),
            ("reopen", Some("A late bank fee"))
        );
    }

    #[test]
    fn a_basis_change_waits_for_an_empty_year() {
        let (_t, store, book) = setup();
        let c = store.conn();
        let today = Date::new(2026, 1, 1).unwrap();
        update_book(c, &book, None, None, Some(Basis::Accrual), today).unwrap();
        assert_eq!(
            books::get(c, book.id).unwrap().unwrap().basis,
            Basis::Accrual
        );
    }
}
