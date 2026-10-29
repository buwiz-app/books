//! M5: applying an import [`Plan`] to a book, the owner's account matches
//! (Map 1), opening totals with proof for a mid-year start, and the
//! owner's OK of a return event.
//!
//! An import runs in the caller's one transaction: it all lands or none of
//! it does. Every imported account starts unconfirmed with the kind Map 1
//! suggests; its lines hold every box they could feed until the owner
//! confirms (owner decision: the owner confirms account matches).

use books_core::catalog::{AccountType, Catalog, Facts};
use books_core::date::Date;
use books_core::import::{AccountRef, Plan, Source, Suggestion};
use books_core::ledger::{Basis, KindRef, Status};
use rusqlite::{Connection, OptionalExtension as _, params};

use crate::books::{self, Account, Book};
use crate::entries::{self, DraftInput, Form, Gate, LineInput, check_text};
use crate::people;
use crate::{Result, error, now_iso};

/// What an import did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Summary {
    pub accounts_new: usize,
    pub accounts_matched: usize,
    pub people: usize,
    pub tags: usize,
    pub posted: usize,
    pub drafts: usize,
    pub skipped: Vec<(String, usize, String)>,
    pub batch_id: i64,
}

fn normalized(name: &str) -> String {
    let lower = name.to_lowercase().replace('&', " and ");
    let kept: String = lower
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    kept.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A free number for a new account of `ty`, in that type's block.
fn free_number(conn: &Connection, book: &Book, ty: AccountType) -> Result<String> {
    let base: i64 = match ty {
        AccountType::Asset => 18000,
        AccountType::Liability => 28000,
        AccountType::Equity => 38000,
        AccountType::Revenue => 48000,
        AccountType::CostOfRevenue => 58000,
        AccountType::Expense => 78000,
        AccountType::OtherIncome => 88000,
        AccountType::OtherExpense => 98000,
    };
    for n in base..base + 999 {
        if books::account_by_number(conn, book.id, &n.to_string())?.is_none() {
            return Ok(n.to_string());
        }
    }
    Err(error(
        "no_number",
        "No free account number is left for that type.",
    ))
}

/// Apply `plan` to `book`. `suggest` is Map 1 (`books_core::import::suggest_kind`).
pub fn apply(
    conn: &Connection,
    book: &Book,
    plan: &Plan,
    suggest: &dyn Fn(Option<&str>, &str, AccountType) -> Suggestion,
    file_name: Option<&str>,
    gate: Gate<'_>,
) -> Result<Summary> {
    let posted: i64 = conn.query_row(
        "SELECT count(*) FROM entry WHERE book_id = ?1 AND status IN ('posted','voided')",
        params![book.id],
        |r| r.get(0),
    )?;
    if posted > 0 {
        return Err(error(
            "book_not_empty",
            "Import old books into a book with nothing posted yet.",
        ));
    }
    if plan.entries.len() > 20_000 {
        return Err(error(
            "invalid_args",
            "An import has at most 20,000 entries.",
        ));
    }
    let file_name = check_text("file name", file_name, 120)?;
    let mut summary = Summary::default();
    let catalog = Catalog::get();

    // accounts: by old number and by old name, to the Books account
    let mut accrual_left_out = 0usize;
    let mut by_number: Vec<(String, i64)> = Vec::new();
    let mut by_name: Vec<(String, i64)> = Vec::new();
    for a in &plan.accounts {
        let name = check_text("account name", Some(&a.name), 60)?
            .ok_or_else(|| error("invalid_args", "An imported account has no name."))?;
        let s = suggest(a.number.as_deref(), &a.name, a.account_type);
        let kind = KindRef::parse(&s.kind).unwrap_or(KindRef::Unsure);
        if kind.accrual_only() && book.basis == Basis::Cash {
            // a receivable or payable cannot sit on a cash book; its lines
            // would be refused, so the import says so plainly
            let used = plan.entries.iter().any(|e| {
                e.lines.iter().any(|l| match &l.account {
                    AccountRef::Number(n) => a.number.as_deref() == Some(n.as_str()),
                    AccountRef::Name(n) => normalized(n) == normalized(&a.name),
                })
            });
            if !used {
                // never used: a cash book has none, so it stays out
                accrual_left_out += 1;
                continue;
            }
            return Err(error(
                "accrual_history",
                format!(
                    "The old books post to {name}, a receivable or payable account. Make this book accrual in Settings › Book first, or import into an accrual book."
                ),
            ));
        }
        // the preset account this one is, when the number and the kind agree
        let preset = match &a.number {
            Some(n) => books::account_by_number(conn, book.id, n)?
                .filter(|p| p.kind == kind && p.source_preset()),
            None => None,
        };
        let id = match preset {
            Some(p) => {
                conn.execute(
                    "UPDATE account SET kind_suggested = ?2, confidence = ?3, kind_confirmed = 0, imported_name = ?4 WHERE account_id = ?1",
                    params![p.id, s.kind, i64::from(s.confidence), if normalized(&p.name) == normalized(&name) { None } else { Some(name.clone()) }],
                )?;
                summary.accounts_matched += 1;
                p.id
            }
            None => {
                let number = match &a.number {
                    Some(n) if books::account_by_number(conn, book.id, n)?.is_none() => n.clone(),
                    _ => free_number(conn, book, a.account_type)?,
                };
                let ty = match &kind {
                    KindRef::Kind(k) => catalog.kind(k).map_or(a.account_type, |k| k.account_type),
                    _ => a.account_type,
                };
                conn.execute(
                    "INSERT INTO account (book_id, number, name, type, kind, kind_confirmed, active, source, kind_suggested, confidence)
                     VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, 'import', ?7, ?8)",
                    params![book.id, number, name, ty.as_str(), kind.as_str(), i64::from(a.active), s.kind, i64::from(s.confidence)],
                )?;
                summary.accounts_new += 1;
                conn.last_insert_rowid()
            }
        };
        if let Some(n) = &a.number {
            by_number.push((n.clone(), id));
        }
        by_name.push((normalized(&a.name), id));
    }
    let resolve = |r: &AccountRef| -> Result<i64> {
        let found = match r {
            AccountRef::Number(n) => by_number.iter().find(|(k, _)| k == n).map(|(_, id)| *id),
            AccountRef::Name(n) => by_name
                .iter()
                .find(|(k, _)| *k == normalized(n))
                .map(|(_, id)| *id),
        };
        match found {
            Some(id) => Ok(id),
            None => {
                // a line may name a Books account the plan did not list
                let fallback = match r {
                    AccountRef::Number(n) => books::account_by_number(conn, book.id, n)?,
                    AccountRef::Name(n) => books::accounts(conn, book.id)?
                        .into_iter()
                        .find(|a| normalized(&a.name) == normalized(n)),
                };
                fallback.map(|a| a.id).ok_or_else(|| {
                    error(
                        "unknown_account",
                        "A transaction names an account the import has no chart row for.",
                    )
                })
            }
        }
    };

    for p in &plan.people {
        people::person_for(conn, book, p.kind, &p.name)?;
        summary.people += 1;
    }
    for t in &plan.tags {
        if people::tags(conn, book.id, Some(t.kind))?
            .iter()
            .all(|x| normalized(&x.name) != normalized(&t.name))
        {
            people::create_tag(conn, book, t.kind, &t.name)?;
            summary.tags += 1;
        }
    }

    let source = match plan.source {
        Source::WebBooks { version } => format!("web Buwiz Books v{version}"),
        Source::Csv => "CSV".into(),
    };
    let skipped_text: String = plan
        .skipped
        .iter()
        .map(|s| format!("{} ({}): {}", s.what, s.count, s.why))
        .collect::<Vec<_>>()
        .join("; ");
    conn.execute(
        "INSERT INTO import_batch (book_id, source, file_name, accounts, entries, skipped, imported_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            book.id,
            source,
            file_name,
            i64::try_from(plan.accounts.len()).unwrap_or(0),
            i64::try_from(plan.entries.len()).unwrap_or(0),
            skipped_text,
            now_iso()
        ],
    )?;
    let batch = conn.last_insert_rowid();
    summary.batch_id = batch;

    let person_named = |name: &str| -> Result<Option<i64>> {
        Ok(conn
            .query_row(
                "SELECT person_id FROM person WHERE book_id = ?1 AND name = ?2 COLLATE NOCASE ORDER BY person_id LIMIT 1",
                params![book.id, name.trim()],
                |r| r.get(0),
            )
            .optional()?)
    };
    for e in &plan.entries {
        let mut lines = Vec::with_capacity(e.lines.len());
        for l in &e.lines {
            lines.push(LineInput {
                account_id: resolve(&l.account)?,
                debit: l.debit,
                credit: l.credit,
                facts: Facts::default(),
            });
        }
        let draft = DraftInput {
            date: e.date,
            form: Form::Journal,
            payee: e.party.clone(),
            memo: e.memo.clone(),
            reference: e.reference.clone(),
            needs_review: false,
            lines,
        };
        let id = entries::save_draft(conn, book, None, &draft, "import")?;
        conn.execute(
            "UPDATE entry SET import_batch_id = ?2 WHERE entry_id = ?1",
            params![id, batch],
        )?;
        let party = e
            .party
            .as_deref()
            .or_else(|| e.lines.iter().find_map(|l| l.party.as_deref()));
        if let Some(p) = party.map(person_named).transpose()?.flatten() {
            conn.execute(
                "UPDATE entry SET person_id = ?2 WHERE entry_id = ?1",
                params![id, p],
            )?;
        }
        let tag = |kind: &str, name: Option<&str>| -> Result<Option<i64>> {
            Ok(match name {
                Some(n) => people::tags(conn, book.id, Some(kind))?
                    .into_iter()
                    .find(|t| normalized(&t.name) == normalized(n))
                    .map(|t| t.id),
                None => None,
            })
        };
        let dept = tag(
            "department",
            e.lines.iter().find_map(|l| l.department.as_deref()),
        )?;
        let loc = tag(
            "location",
            e.lines.iter().find_map(|l| l.location.as_deref()),
        )?;
        if dept.is_some() || loc.is_some() {
            people::set_entry_links(conn, book, id, None, dept, loc)?;
        }
        if e.posted {
            match entries::post(conn, book, id, gate) {
                Ok(()) => {
                    summary.posted += 1;
                    continue;
                }
                Err(err) if err.code == "period_closed" || err.code == "accrual_account" => {
                    return Err(err);
                }
                Err(_) => {}
            }
        }
        // not posted in the old books, or refused by a post rule: a draft to review
        conn.execute(
            "UPDATE entry SET needs_review = 1 WHERE entry_id = ?1",
            params![id],
        )?;
        summary.drafts += 1;
    }
    summary.skipped = plan
        .skipped
        .iter()
        .map(|s| (s.what.clone(), s.count, s.why.clone()))
        .collect();
    if accrual_left_out > 0 {
        summary.skipped.push((
            "receivable and payable accounts".into(),
            accrual_left_out,
            "a cash book has none; unpaid invoices and bills go on the Unpaid page".into(),
        ));
    }
    Ok(summary)
}

impl Account {
    /// One of the chart preset's accounts (not imported or added).
    pub fn source_preset(&self) -> bool {
        self.source == "preset"
    }
}

/// An account waiting for, or holding, the owner's match.
#[derive(Debug, Clone)]
pub struct Match {
    pub account: Account,
    pub suggested: Option<String>,
    pub confidence: Option<i64>,
    pub imported_name: Option<String>,
}

/// `(account_id, kind_suggested, confidence, imported_name)`.
type MatchRow = (i64, Option<String>, Option<i64>, Option<String>);

/// Every account an import touched, and any not confirmed yet.
pub fn matches(conn: &Connection, book_id: i64) -> Result<Vec<Match>> {
    let mut stmt = conn.prepare(
        "SELECT account_id, kind_suggested, confidence, imported_name FROM account
         WHERE book_id = ?1 AND (source = 'import' OR kind_suggested IS NOT NULL OR kind_confirmed = 0) ORDER BY number",
    )?;
    let rows: Vec<MatchRow> = stmt
        .query_map(params![book_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })?
        .collect::<rusqlite::Result<_>>()?;
    let mut out = Vec::new();
    for (id, suggested, confidence, imported_name) in rows {
        if let Some(account) = books::account(conn, id)? {
            out.push(Match {
                account,
                suggested,
                confidence,
                imported_name,
            });
        }
    }
    Ok(out)
}

/// The owner's match: one kind, `mixed` (decided per line) or `unsure`
/// (the lines keep holding their boxes). A kind confirms the account.
pub fn confirm(conn: &Connection, book: &Book, account_id: i64, kind: &KindRef) -> Result<()> {
    let a = books::account(conn, account_id)?
        .filter(|a| a.book_id == book.id)
        .ok_or_else(|| error("not_found", "That account is not in this book."))?;
    if let KindRef::Kind(k) = kind {
        let def = Catalog::get()
            .kind(k)
            .ok_or_else(|| error("invalid_args", "No such kind."))?;
        let compatible = |t: AccountType| match t {
            AccountType::Revenue | AccountType::OtherIncome => 1,
            AccountType::CostOfRevenue | AccountType::Expense | AccountType::OtherExpense => 2,
            AccountType::Asset => 3,
            AccountType::Liability => 4,
            AccountType::Equity => 5,
        };
        if compatible(def.account_type) != compatible(a.account_type) {
            return Err(error(
                "kind_type",
                format!(
                    "{} is not a kind for a {} account.",
                    def.name,
                    a.account_type.group().trim_end_matches('s').to_lowercase()
                ),
            ));
        }
        if def.accrual_only && book.basis == Basis::Cash {
            return Err(error(
                "accrual_account",
                "A cash book has no receivable or payable accounts.",
            ));
        }
    }
    let confirmed = i64::from(!matches!(kind, KindRef::Unsure));
    conn.execute(
        "UPDATE account SET kind = ?2, kind_confirmed = ?3 WHERE account_id = ?1",
        params![a.id, kind.as_str(), confirmed],
    )?;
    Ok(())
}

/// Confirm every suggestion at or above `min` percent. Returns how many.
pub fn confirm_sure(conn: &Connection, book: &Book, min: i64) -> Result<usize> {
    let mut n = 0;
    for m in matches(conn, book.id)? {
        if m.account.kind_confirmed {
            continue;
        }
        if let (Some(s), Some(c)) = (&m.suggested, m.confidence)
            && c >= min
            && let Some(kind) =
                KindRef::parse(s).filter(|k| !matches!(k, KindRef::Unsure | KindRef::Mixed))
            && confirm(conn, book, m.account.id, &kind).is_ok()
        {
            n += 1;
        }
    }
    Ok(n)
}

// --- opening totals ---------------------------------------------------------

/// The measures an opening total may carry: the ones a mid-year start
/// needs from filed returns or old books, with the kind and side the
/// bridge reads them as.
pub const OPENING_MEASURES: [(&str, &str); 8] = [
    ("gross_sales", "Gross sales"),
    ("sales_returns", "Sales returns"),
    ("sales_allowances", "Sales allowances"),
    ("sales_discounts", "Sales discounts"),
    ("cost_of_sales", "Cost of sales"),
    ("other_non_operating_income", "Other non-operating income"),
    ("basic_pay", "Basic pay"),
    ("employee_mandatory_contributions", "Employee contributions"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opening {
    pub measure: String,
    pub from: Date,
    pub to: Date,
    pub centavos: i64,
    pub proof: String,
}

pub fn set_opening(conn: &Connection, book: &Book, o: &Opening) -> Result<()> {
    if !OPENING_MEASURES.iter().any(|(m, _)| *m == o.measure) {
        return Err(error(
            "invalid_args",
            "That total cannot be an opening figure.",
        ));
    }
    if o.from > o.to || o.from.year != o.to.year {
        return Err(error(
            "invalid_args",
            "An opening range runs forward within one year.",
        ));
    }
    if o.centavos < 0 {
        return Err(error(
            "invalid_args",
            "An opening total is not negative; returns have their own total.",
        ));
    }
    let proof = check_text("proof", Some(&o.proof), 200)?.ok_or_else(|| {
        error(
            "proof_needed",
            "Say what proves it, like the filed returns or bank statements.",
        )
    })?;
    // the opening must end before the book's own entries begin
    let first: Option<String> = conn.query_row(
        "SELECT min(entry_date) FROM entry WHERE book_id = ?1 AND status IN ('posted','voided')",
        params![book.id],
        |r| r.get(0),
    )?;
    if let Some(first) = first.and_then(|d| Date::parse(&d))
        && o.to >= first
    {
        return Err(error(
            "opening_overlaps",
            format!("The book's own entries start {first}; an opening total ends before that."),
        ));
    }
    conn.execute(
        "INSERT INTO opening_total (book_id, measure, period_from, period_to, centavos, proof, entered_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT (book_id, measure, period_from) DO UPDATE SET period_to = excluded.period_to, centavos = excluded.centavos, proof = excluded.proof, entered_at = excluded.entered_at",
        params![book.id, o.measure, o.from.to_string(), o.to.to_string(), o.centavos, proof, now_iso()],
    )?;
    Ok(())
}

pub fn openings(conn: &Connection, book_id: i64) -> Result<Vec<Opening>> {
    let mut stmt = conn.prepare(
        "SELECT measure, period_from, period_to, centavos, proof FROM opening_total WHERE book_id = ?1 ORDER BY period_from, measure",
    )?;
    let rows = stmt.query_map(params![book_id], |r| {
        let f: String = r.get(1)?;
        let t: String = r.get(2)?;
        Ok(Opening {
            measure: r.get(0)?,
            from: Date::parse(&f).unwrap_or(Date {
                year: 1970,
                month: 1,
                day: 1,
            }),
            to: Date::parse(&t).unwrap_or(Date {
                year: 1970,
                month: 1,
                day: 1,
            }),
            centavos: r.get(3)?,
            proof: r.get(4)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn delete_opening(conn: &Connection, book: &Book, measure: &str, from: Date) -> Result<()> {
    let n = conn.execute(
        "DELETE FROM opening_total WHERE book_id = ?1 AND measure = ?2 AND period_from = ?3",
        params![book.id, measure, from.to_string()],
    )?;
    if n == 0 {
        return Err(error("not_found", "There is no such opening total."));
    }
    Ok(())
}

// --- return events --------------------------------------------------------

/// The owner's OK of a return event (a posted credit note or a reversal).
pub fn acknowledge(conn: &Connection, book: &Book, entry_id: i64) -> Result<()> {
    let e = entries::get(conn, entry_id)?
        .filter(|e| e.book_id == book.id && matches!(e.status, Status::Posted | Status::Voided))
        .ok_or_else(|| error("not_found", "That posted entry is not in this book."))?;
    conn.execute(
        "INSERT INTO return_ack (book_id, entry_id, acknowledged_at) VALUES (?1, ?2, ?3) ON CONFLICT DO NOTHING",
        params![book.id, e.id, now_iso()],
    )?;
    Ok(())
}

pub fn acknowledged(conn: &Connection, book_id: i64) -> Result<Vec<i64>> {
    let mut stmt =
        conn.prepare("SELECT entry_id FROM return_ack WHERE book_id = ?1 ORDER BY entry_id")?;
    let rows = stmt.query_map(params![book_id], |r| r.get(0))?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}
