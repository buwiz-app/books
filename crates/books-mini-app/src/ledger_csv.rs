//! The ledger as CSV (`books_report_csv` with `kind: "ledger"`): one row
//! per line in date and entry order, amounts as two-place decimal text.
//! The assistant gets no payee, memo, reference or person columns.
//! Posted and draft entries only; voided originals with their reversals
//! and discarded drafts come in only with `filter: "all"`.

use books_core::date::Date;
use books_core::ledger::Status;
use books_core::money;
use books_store::Store;
use books_store::books::Book;
use books_store::entries::{self, Filter};
use books_store::people;
use serde_json::{Map, Value, json};

use crate::service::{self, Answer, Origin, Refusal};
use crate::views::{Chart, Summary};

/// The host's limit on one tool result: the text content plus the
/// structuredContent JSON, in bytes.
pub const RESULT_MAX: usize = 1024 * 1024;

/// Slack for any escaping difference between serde and the host's
/// `JSON.stringify`.
const SLACK: usize = 1024;

/// Bytes the host counts for `answer`: `jsonrpc::ok` sends it twice, as
/// the text content and as structuredContent.
pub fn result_bytes(answer: &Value) -> usize {
    2 * answer.to_string().len() + SLACK
}

fn too_large() -> Refusal {
    Refusal::new(
        "too_large",
        "The ledger CSV is over the 1 MB result limit. Pick a shorter date range.",
    )
}

/// RFC 4180 quoting, and a leading `'` on text that a spreadsheet would
/// read as a formula.
pub fn cell(text: &str) -> String {
    let guarded = if text.starts_with(['=', '+', '-', '@']) {
        format!("'{text}")
    } else {
        text.to_string()
    };
    if guarded.contains([',', '"', '\r', '\n']) {
        format!("\"{}\"", guarded.replace('"', "\"\""))
    } else {
        guarded
    }
}

fn date_arg(args: &Map<String, Value>, key: &str) -> Result<Option<Date>, Refusal> {
    match service::text(args, key) {
        None | Some("") => Ok(None),
        Some(t) => Date::parse(t)
            .map(Some)
            .ok_or_else(|| Refusal::invalid(format!("`{key}` is a date, YYYY-MM-DD."))),
    }
}

pub fn ledger_csv(store: &Store, book: Book, args: &Map<String, Value>, origin: Origin) -> Answer {
    let from = date_arg(args, "from")?;
    let to = date_arg(args, "to")?;
    let filter = service::text(args, "filter").unwrap_or("");
    if !["", "all", "review", "draft", "posted"].contains(&filter) {
        return Err(Refusal::invalid(
            "`filter` is all, review, draft or posted.",
        ));
    }
    let everything = filter == "all";
    let account = match service::text(args, "account") {
        Some(r) if !r.is_empty() && r != "all" => Some(service::account(store, &book, r)?),
        _ => None,
    };
    let chart = Chart::load(store, book)?;
    let c = store.conn();
    let mut list = entries::list(
        c,
        chart.book.id,
        &Filter {
            from,
            to,
            account_id: account.as_ref().map(|a| a.id),
            discarded: everything,
        },
    )?;
    list.sort_by_key(|e| (e.date, e.no));
    let full = origin.is_app();
    let mut head = vec!["date", "entry", "status", "form", "account", "category"];
    if full {
        head.extend(["person", "payee", "memo", "reference"]);
    }
    head.extend(["debit", "credit"]);
    let mut out = head.join(",");
    out.push_str("\r\n");
    let mut rows = 0usize;
    for e in list {
        let live = matches!(e.status, Status::Posted | Status::Draft) && e.reverses.is_none();
        if !everything && !live {
            continue;
        }
        let s = Summary::of(store, &chart, e)?;
        let keep = match filter {
            "review" => s.needs_review(),
            "draft" => s.entry.status == Status::Draft && !s.entry.needs_review,
            "posted" => s.entry.status == Status::Posted,
            _ => true,
        };
        if !keep {
            continue;
        }
        let person = if full {
            match people::links(c, s.entry.id)?.0 {
                Some(id) => people::person(c, id)?.map(|p| p.name).unwrap_or_default(),
                None => String::new(),
            }
        } else {
            String::new()
        };
        let category = s.category.as_ref().map_or("", |a| a.name.as_str());
        for l in &s.lines {
            let acct = chart
                .by_id
                .get(&l.account_id)
                .map(|a| format!("{} {}", a.number, a.name))
                .unwrap_or_default();
            let mut cells = vec![
                s.entry.date.to_string(),
                format!("{:04}", s.entry.no),
                s.entry.status.as_str().to_string(),
                cell(s.entry.form.as_str()),
                cell(&acct),
                cell(category),
            ];
            if full {
                cells.push(cell(&person));
                for t in [&s.entry.payee, &s.entry.memo, &s.entry.reference] {
                    cells.push(cell(t.as_deref().unwrap_or("")));
                }
            }
            cells.push(money::format(l.debit));
            cells.push(money::format(l.credit));
            out.push_str(&cells.join(","));
            out.push_str("\r\n");
            rows += 1;
            if 2 * out.len() > RESULT_MAX {
                return Err(too_large());
            }
        }
    }
    let span = match (from, to) {
        (Some(f), Some(t)) => format!("{f}-to-{t}"),
        (Some(f), None) => format!("from-{f}"),
        (None, Some(t)) => format!("to-{t}"),
        (None, None) => "all".into(),
    };
    let answer =
        json!({ "file_name": format!("books-ledger-{span}.csv"), "csv": out, "rows": rows + 1 });
    if result_bytes(&answer) > RESULT_MAX {
        return Err(too_large());
    }
    Ok(answer)
}
