//! The writes. Each runs inside [`service::write`]: one transaction, the
//! `client_request_id` replay, an audit row. Posting, voiding and
//! discarding are app-only (DD-20): the assistant and routines may only
//! draft.

use books_core::catalog::{AccountType, Facts};
use books_core::date::Date;
use books_core::ledger::{Basis, KindRef};
use books_core::money;
use books_store::books::{self, Account, Book};
use books_store::entries::{self, DraftInput, Form, LineInput};
use books_store::plumbing;
use books_store::rusqlite::{Connection, params};
use serde_json::{Map, Value, json};

use crate::service::{self, Answer, Origin, Refusal, text, with_subject};

fn tickets(conn: &Connection, kind: &str, id: i64) -> Result<String, Refusal> {
    Ok(plumbing::ticket_for(conn, kind, id)?)
}

fn book_in(conn: &Connection, args: &Map<String, Value>) -> Result<Book, Refusal> {
    if let Some(t) = text(args, "book") {
        let id = match plumbing::subject(conn, t)? {
            Some((k, id)) if k == "book" => id,
            _ => {
                return Err(Refusal::new(
                    "not_found",
                    "That book ticket is not one Books issued.",
                ));
            }
        };
        return books::get(conn, id)?.ok_or_else(|| Refusal::new("not_found", "No such book."));
    }
    books::all(conn)?.into_iter().next().ok_or_else(|| {
        Refusal::new(
            "not_set_up",
            "There is no book yet. Open Books to make one.",
        )
    })
}

fn account_in(conn: &Connection, book: &Book, reference: &str) -> Result<Account, Refusal> {
    let found = if plumbing::is_ticket(reference) {
        match plumbing::subject(conn, reference)? {
            Some((k, id)) if k == "account" => books::account(conn, id)?,
            _ => None,
        }
    } else {
        books::account_by_number(conn, book.id, reference)?
    };
    found
        .filter(|a| a.book_id == book.id)
        .ok_or_else(|| Refusal::new("not_found", "That account is not in this book."))
}

fn entry_in(conn: &Connection, book: &Book, reference: &str) -> Result<entries::Entry, Refusal> {
    let id = match plumbing::subject(conn, reference)? {
        Some((k, id)) if k == "entry" => id,
        _ => {
            return Err(Refusal::new(
                "not_found",
                "That entry ticket is not one Books issued.",
            ));
        }
    };
    entries::get(conn, id)?
        .filter(|e| e.book_id == book.id)
        .ok_or_else(|| Refusal::new("not_found", "That entry is not in this book."))
}

fn amount(args: &Map<String, Value>, key: &str) -> Result<i64, Refusal> {
    let t = service::required(args, key)?;
    let c = money::parse(t).map_err(|e| Refusal::invalid(format!("`{key}`: {e}")))?;
    if c <= 0 {
        return Err(Refusal::invalid(format!("`{key}` must be more than 0.00.")));
    }
    Ok(c)
}

fn date(args: &Map<String, Value>) -> Result<Date, Refusal> {
    match text(args, "date") {
        Some(d) => Date::parse(d).ok_or_else(|| Refusal::invalid("`date` is YYYY-MM-DD.")),
        None => Ok(Date::today()),
    }
}

fn facts(args: &Map<String, Value>, key: &str) -> Result<Facts, Refusal> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(Facts::default()),
        Some(v) => Facts::from_json(v).map_err(Refusal::invalid),
    }
}

fn money_side(a: &Account) -> bool {
    a.is_cash() || a.kind == KindRef::Kind("loan_payable".into())
}

/// The draft a request describes. `money_in`: the money account takes the
/// debit, the category the credit; `money_out` the other way; `transfer`
/// moves between two money accounts; `journal` takes its lines as given.
fn draft_input(
    conn: &Connection,
    book: &Book,
    args: &Map<String, Value>,
) -> Result<DraftInput, Refusal> {
    let form = Form::parse(service::required(args, "form")?)
        .filter(|f| *f != Form::Reversal)
        .ok_or_else(|| Refusal::invalid("`form` is money_in, money_out, transfer or journal."))?;
    let lines = match form {
        Form::MoneyIn | Form::MoneyOut => {
            let c = amount(args, "amount")?;
            let money = account_in(conn, book, service::required(args, "account")?)?;
            if !money_side(&money) {
                return Err(Refusal::invalid(format!(
                    "{} is not a bank, cash or card account.",
                    money.name
                )));
            }
            let category = account_in(conn, book, service::required(args, "category")?)?;
            if category.id == money.id {
                return Err(Refusal::invalid(
                    "The category and the money account are the same.",
                ));
            }
            let own = facts(args, "facts")?;
            let (m, k) = if form == Form::MoneyIn {
                ((c, 0), (0, c))
            } else {
                ((0, c), (c, 0))
            };
            let money_line = LineInput {
                account_id: money.id,
                debit: m.0,
                credit: m.1,
                facts: Facts::default(),
            };
            let category_line = LineInput {
                account_id: category.id,
                debit: k.0,
                credit: k.1,
                facts: own,
            };
            if form == Form::MoneyIn {
                vec![money_line, category_line]
            } else {
                vec![category_line, money_line]
            }
        }
        Form::Transfer => {
            let c = amount(args, "amount")?;
            let from = account_in(conn, book, service::required(args, "account")?)?;
            let to = account_in(conn, book, service::required(args, "to_account")?)?;
            if !money_side(&from) || !money_side(&to) || from.id == to.id {
                return Err(Refusal::invalid(
                    "A transfer moves money between two bank, cash or card accounts.",
                ));
            }
            vec![
                LineInput {
                    account_id: to.id,
                    debit: c,
                    credit: 0,
                    facts: Facts::default(),
                },
                LineInput {
                    account_id: from.id,
                    debit: 0,
                    credit: c,
                    facts: Facts::default(),
                },
            ]
        }
        Form::Journal => {
            let raw = args
                .get("lines")
                .and_then(Value::as_array)
                .ok_or_else(|| Refusal::invalid("A journal takes `lines`."))?;
            let mut out = Vec::new();
            for (i, l) in raw.iter().enumerate() {
                let l = l.as_object().ok_or_else(|| {
                    Refusal::invalid(format!("Line {} must be an object.", i + 1))
                })?;
                let side = |k: &str| -> Result<i64, Refusal> {
                    match text(l, k) {
                        None => Ok(0),
                        Some(t) => money::parse(t)
                            .map_err(|e| Refusal::invalid(format!("Line {}: {e}", i + 1))),
                    }
                };
                let account = account_in(conn, book, service::required(l, "account")?)?;
                out.push(LineInput {
                    account_id: account.id,
                    debit: side("debit")?,
                    credit: side("credit")?,
                    facts: facts(l, "facts")?,
                });
            }
            out
        }
        Form::Reversal => unreachable!("refused above"),
    };
    Ok(DraftInput {
        date: date(args)?,
        form,
        payee: text(args, "payee").map(str::to_string),
        memo: text(args, "memo").map(str::to_string),
        reference: text(args, "reference").map(str::to_string),
        needs_review: false,
        lines,
    })
}

/// The month gate: always open until months can close (M4).
pub fn gate(_conn: &Connection, _book: &Book) -> impl Fn(Date) -> books_store::Result<()> {
    |_| Ok(())
}

fn entry_result(conn: &Connection, id: i64) -> Result<Value, Refusal> {
    let e = entries::get(conn, id)?.ok_or_else(|| Refusal::new("store", "The entry vanished."))?;
    Ok(with_subject(
        json!({ "entry": tickets(conn, "entry", id)?, "no": format!("{:04}", e.no), "status": e.status.as_str() }),
        "entry",
        id,
    ))
}

/// `books_entry_draft`: save a new draft, edit one (`entry`), or copy a
/// posted one (`copy_of`).
pub fn draft(conn: &Connection, args: &Map<String, Value>, origin: Origin) -> Answer {
    let book = book_in(conn, args)?;
    let existing = match text(args, "entry") {
        Some(t) => Some(entry_in(conn, &book, t)?.id),
        None => None,
    };
    let input = match text(args, "copy_of") {
        Some(t) => {
            let source = entry_in(conn, &book, t)?;
            if source.form == Form::Reversal {
                return Err(Refusal::invalid("A reversal cannot be copied."));
            }
            let rows = entries::lines(conn, source.id)?;
            DraftInput {
                date: date(args)?,
                form: source.form,
                payee: source.payee.clone(),
                memo: source.memo.clone(),
                reference: None,
                needs_review: false,
                lines: rows
                    .into_iter()
                    .map(|l| LineInput {
                        account_id: l.account_id,
                        debit: l.debit,
                        credit: l.credit,
                        facts: l.own,
                    })
                    .collect(),
            }
        }
        None => draft_input(conn, &book, args)?,
    };
    let id = entries::save_draft(conn, &book, existing, &input, origin.as_str())?;
    entry_result(conn, id)
}

/// `books_app_post_entry`: post a draft (`entry`), or save and post a new
/// one in the same transaction.
pub fn post(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let book = book_in(conn, args)?;
    let id = match text(args, "entry") {
        Some(t) => entry_in(conn, &book, t)?.id,
        None => {
            let input = draft_input(conn, &book, args)?;
            entries::save_draft(conn, &book, None, &input, "app")?
        }
    };
    let g = gate(conn, &book);
    entries::post(conn, &book, id, &g)?;
    entry_result(conn, id)
}

/// `books_app_void_entry`: mark the entry voided and post its reversal,
/// dated like the original (DD-15).
pub fn void(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let book = book_in(conn, args)?;
    let e = entry_in(conn, &book, service::required(args, "entry")?)?;
    let g = gate(conn, &book);
    let reversal = entries::void(conn, &book, e.id, e.date, &g)?;
    let mut out = entry_result(conn, e.id)?;
    out["reversal"] = json!(tickets(conn, "entry", reversal)?);
    Ok(out)
}

/// `books_app_discard_entry`.
pub fn discard(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let book = book_in(conn, args)?;
    let e = entry_in(conn, &book, service::required(args, "entry")?)?;
    entries::discard(conn, &book, e.id)?;
    entry_result(conn, e.id)
}

/// `books_app_book_create`: a book on the Philippines preset; cash basis
/// unless the owner picks accrual (ADR 0008).
pub fn book_create(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let label = service::required(args, "label")?;
    let basis = match text(args, "basis") {
        None => Basis::Cash,
        Some(b) => {
            Basis::parse(b).ok_or_else(|| Refusal::invalid("`basis` is cash or accrual."))?
        }
    };
    let fy = match args.get("fiscal_year_start") {
        None => 1,
        Some(v) => v
            .as_i64()
            .and_then(|m| u8::try_from(m).ok())
            .ok_or_else(|| Refusal::invalid("`fiscal_year_start` is a month, 1 to 12."))?,
    };
    let id = books::create(conn, label, basis, fy)?;
    Ok(with_subject(
        json!({ "book": tickets(conn, "book", id)?, "basis": basis.as_str() }),
        "book",
        id,
    ))
}

/// `books_app_account_create`: a new account under a kind (or "Not sure yet").
pub fn account_create(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let book = book_in(conn, args)?;
    let number = service::required(args, "number")?.trim();
    if number.is_empty() || number.len() > 12 || !number.bytes().all(|b| b.is_ascii_digit()) {
        return Err(Refusal::invalid("An account number is 1 to 12 digits."));
    }
    let name = books::check_label(service::required(args, "name")?).map_err(|e| {
        Refusal::new(
            e.code,
            e.message
                .replace("book name", "account name")
                .replace("A book name", "An account name"),
        )
    })?;
    let kind = KindRef::parse(service::required(args, "kind")?)
        .ok_or_else(|| Refusal::invalid("`kind` is a kind id, mixed or unsure."))?;
    let ty = match &kind {
        KindRef::Kind(k) => books_core::catalog::Catalog::get()
            .kind(k)
            .map(|k| k.account_type),
        _ => None,
    };
    let ty = match text(args, "type") {
        Some(t) => {
            AccountType::parse(t).ok_or_else(|| Refusal::invalid("Unknown account type."))?
        }
        None => ty.ok_or_else(|| {
            Refusal::invalid("`type` is needed for an account that is not one kind.")
        })?,
    };
    if kind.accrual_only() && book.basis == Basis::Cash {
        return Err(Refusal::new(
            "accrual_account",
            "A cash book has no receivable or payable accounts.",
        ));
    }
    if books::account_by_number(conn, book.id, number)?.is_some() {
        return Err(Refusal::new("exists", "That account number is taken."));
    }
    conn.execute(
        "INSERT INTO account (book_id, number, name, type, kind) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![book.id, number, name, ty.as_str(), kind.as_str()],
    )
    .map_err(books_store::StoreError::from)?;
    let id = conn.last_insert_rowid();
    Ok(with_subject(
        json!({ "account": tickets(conn, "account", id)? }),
        "account",
        id,
    ))
}
