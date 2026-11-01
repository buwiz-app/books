//! `books_owed_document`: one owed document as a static HTML print page for
//! `buwiz.preview` (bridge.md, "Print preview"). A collect document is a
//! "Statement of account" and a pay document a "Bill record": Books issues
//! no BIR documents (ADR 0007), so never an invoice or an official receipt.
//!
//! The page has an inline `<style>` only: no script, link, image, URL or
//! network reference. It never carries the memo, a ticket or an entry id;
//! every text goes through [`esc`].

use books_core::money;
use books_store::books::{self as store_books, Book};
use books_store::people::{self, Unpaid};
use books_store::plumbing;
use books_store::rusqlite::Connection;
use serde_json::{Map, Value, json};

use crate::jsonrpc::fail;
use crate::service::{self, Books, Origin, Refusal};

/// The resource URI; it names no subject.
pub const URI: &str = "books://print/owed.html";
pub const MIME: &str = "text/html";
/// A rendered page over this many bytes is refused (`too_large`).
pub const MAX_BYTES: usize = 262_144;
pub const FOOTER: &str = "Prepared in Buwiz Books. This is not an official receipt or invoice.";

/// The heading and title of a document on `side`.
pub fn title(side: &str) -> &'static str {
    if side == "collect" {
        "Statement of account"
    } else {
        "Bill record"
    }
}

/// HTML text escaping for element content and attribute values.
pub fn esc(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

const STYLE: &str = "body{font-family:-apple-system,Helvetica,Arial,sans-serif;color:#1d1d1f;margin:32px;font-size:13px}\
h1{font-size:20px;margin:4px 0 18px}.book{font-size:12px;color:#6e6e73;text-transform:uppercase;letter-spacing:.06em}\
table{border-collapse:collapse;width:100%;max-width:560px}th{text-align:left;font-weight:600;color:#6e6e73;width:160px;padding:6px 12px 6px 0;vertical-align:top}\
td{padding:6px 0;border-bottom:1px solid #e5e5ea}.amount{font-family:Menlo,monospace;font-size:15px;font-weight:600}\
footer{margin-top:28px;font-size:11px;color:#6e6e73}@media print{body{margin:0}}";

/// The page for one owed document of `book`.
pub fn render(conn: &Connection, book: &Book, u: &Unpaid) -> Result<String, Refusal> {
    let heading = title(&u.side);
    let who = match u.person_id {
        Some(p) => people::person(conn, p)?.map(|p| p.name),
        None => None,
    }
    .unwrap_or_else(|| "No person".to_string());
    let category = store_books::account(conn, u.category_account_id)?
        .map(|a| a.name)
        .unwrap_or_default();
    // part paid: money came in (or went out) but some is still due; the
    // page then shows what was settled and what is left, so a reader is
    // never asked for the whole amount again (`paid` counts posted
    // payments only, so a voided one is back in the balance)
    let part_paid = u.status != "cancelled" && u.paid > 0 && u.balance() > 0;
    let status = match u.status.as_str() {
        "cancelled" => "Cancelled".to_string(),
        _ if part_paid => "Part paid".to_string(),
        _ if u.balance() > 0 => "Open".to_string(),
        _ => match u.paid_on {
            Some(d) => format!("Settled on {d}"),
            None => "Settled".to_string(),
        },
    };
    let mut rows: Vec<(&str, String)> = vec![
        (
            if u.side == "collect" {
                "Customer"
            } else {
                "Vendor"
            },
            who,
        ),
        ("Reference", u.reference.clone().unwrap_or_default()),
        ("Issued", u.issued.to_string()),
        (
            "Due",
            u.due
                .map_or_else(|| "No due date".to_string(), |d| d.to_string()),
        ),
        ("Category", category),
        ("Amount", format!("PHP {}", money::display(u.amount))),
    ];
    if part_paid {
        rows.push((
            if u.side == "collect" {
                "Received to date"
            } else {
                "Paid to date"
            },
            format!("PHP {}", money::display(u.paid)),
        ));
        rows.push((
            "Balance due",
            format!("PHP {}", money::display(u.balance())),
        ));
    }
    rows.push(("Status", status));
    let mut html = String::with_capacity(2048);
    html.push_str("<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>");
    html.push_str(&esc(heading));
    html.push_str("</title><style>");
    html.push_str(STYLE);
    html.push_str("</style></head><body><div class=\"book\">");
    html.push_str(&esc(&book.label));
    html.push_str("</div><h1>");
    html.push_str(&esc(heading));
    html.push_str("</h1><table>");
    for (label, value) in &rows {
        let class = if *label == "Amount" || *label == "Balance due" {
            " class=\"amount\""
        } else {
            ""
        };
        html.push_str(&format!(
            "<tr><th>{}</th><td{class}>{}</td></tr>",
            esc(label),
            esc(value)
        ));
    }
    html.push_str("</table><footer>");
    html.push_str(&esc(FOOTER));
    html.push_str("</footer></body></html>");
    capped(html)
}

fn capped(html: String) -> Result<String, Refusal> {
    if html.len() > MAX_BYTES {
        return Err(Refusal::new(
            "too_large",
            "That document is too large to preview.",
        ));
    }
    Ok(html)
}

/// The preview result: an embedded `text/html` resource, not `jsonrpc::ok`.
pub fn result(html: String, title: &str) -> Value {
    json!({
        "content": [{ "type": "resource", "resource": { "uri": URI, "mimeType": MIME, "text": html } }],
        "structuredContent": { "title": title },
        "isError": false,
    })
}

/// The tool: app origin only, read-only, makes no store and no ticket.
pub fn tool(books: &mut Books, args: &Map<String, Value>, origin: Origin) -> Value {
    if origin != Origin::App {
        return fail(
            "permission_denied",
            "books_owed_document needs the user's click in the Books app.",
        );
    }
    if !books.exists() {
        return fail(
            "not_set_up",
            "There is no book yet. Open Books to make one.",
        );
    }
    let page = (|| -> Result<(String, &'static str), Refusal> {
        let store = books.store()?;
        let conn = store.conn();
        let book = service::book(store, args)?;
        let not_found =
            || Refusal::new("not_found", "That document ticket is not one Books issued.");
        let id = match plumbing::subject(conn, service::required(args, "owed")?)? {
            Some((kind, id)) if kind == "owed" => id,
            _ => return Err(not_found()),
        };
        let u = people::unpaid(conn, id)?
            .filter(|u| u.book_id == book.id)
            .ok_or_else(not_found)?;
        Ok((render(conn, &book, &u)?, title(&u.side)))
    })();
    match page {
        Ok((html, t)) => result(html, t),
        Err(r) => r.into_result(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_over_the_cap_is_refused_and_one_at_the_cap_is_kept() {
        let at_cap = "a".repeat(MAX_BYTES);
        assert_eq!(capped(at_cap.clone()).ok(), Some(at_cap));
        let over = capped("a".repeat(MAX_BYTES + 1)).err().map(|r| r.code);
        assert_eq!(over.as_deref(), Some("too_large"));
    }
}
