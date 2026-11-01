//! The payment history of one unpaid document (M10 parity, unpaid-1), as
//! `books_owed_list` `detail` when the call names `owed`.

use books_core::date::Date;
use books_core::money;
use books_store::Store;
use books_store::books::{self, Book};
use books_store::owed_payments;
use books_store::people;
use books_store::plumbing;
use serde_json::{Value, json};

use crate::records::owed_state;
use crate::service::{Origin, Refusal, ticket};

/// `{ticket, state, issued, due, amount_shown, paid_shown, balance_shown,
/// payments: [{date, amount_shown, account?, entry?, voided}]}`; account and
/// entry go to the app only, like the list's `who` and `doc`.
pub fn detail(store: &Store, book: &Book, owed: &str, origin: Origin) -> Result<Value, Refusal> {
    let id = match plumbing::subject(store.conn(), owed)? {
        Some((k, id)) if k == "owed" => id,
        _ => {
            return Err(Refusal::new(
                "not_found",
                "That document ticket is not one Books issued.",
            ));
        }
    };
    let u = people::unpaid(store.conn(), id)?
        .filter(|u| u.book_id == book.id)
        .ok_or_else(|| Refusal::new("not_found", "That document is not in this book."))?;
    let mut payments = Vec::new();
    for p in owed_payments::payments(store.conn(), id)? {
        let mut row = json!({
            "date": p.on.to_string(),
            "date_short": p.on.short(),
            "amount": money::format(p.amount),
            "amount_shown": money::display(p.amount),
            "voided": p.voided,
        });
        if origin.is_app() {
            row["account"] = json!(
                books::account(store.conn(), p.money_account_id)?
                    .map(|a| a.name)
                    .unwrap_or_default()
            );
            row["entry"] = json!(ticket(store, "entry", p.entry_id)?);
        }
        payments.push(row);
    }
    let state = match u.status.as_str() {
        "cancelled" => "cancelled",
        _ if u.balance() <= 0 => "paid",
        _ => owed_state(&u),
    };
    Ok(json!({
        "ticket": ticket(store, "owed", id)?,
        "side": u.side,
        "state": state,
        "issued": u.issued.to_string(),
        "due": u.due.map(|d: Date| d.to_string()),
        "amount_shown": money::display(u.amount),
        "paid_shown": money::display(u.paid),
        "balance_shown": money::display(u.balance()),
        "payments": payments,
    }))
}
