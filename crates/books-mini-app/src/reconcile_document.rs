//! `books_app_reconcile_document`: the static pages `buwiz.preview` shows for a
//! statement, the reconciliation report and one page of the statement's
//! snapshot with its lines boxed. Read-only and app only (the Bot runs a
//! preview as the app). The title carries no amount, number or name.

use books_core::date::{Date, MONTHS, Period, PeriodKind};
use books_core::money;
use books_core::statement::Match;
use books_store::Store;
use books_store::books::Book;
use books_store::months::{self, Statement, StatementLine};
use books_store::rusqlite::{Connection, OptionalExtension as _};
use std::collections::HashMap;

use serde_json::{Map, Value, json};

use crate::html::{self, escape};
use crate::service::{self, Refusal};

/// The preview document cap (Bot `permissions.preview.max_bytes`, 8 MiB),
/// kept under so the Bot never answers `result_too_large`.
pub const MAX_DOCUMENT: usize = 8_000_000;

/// A line's box on its page, in hundredths of a percent of the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineBox {
    pub page: i64,
    pub left: i64,
    pub top: i64,
    pub width: i64,
    pub height: i64,
}

fn has_column(conn: &Connection, table: &str, column: &str) -> Result<bool, Refusal> {
    let n: i64 = conn
        .query_row(
            "SELECT count(*) FROM pragma_table_info(?1) WHERE name = ?2",
            [table, column],
            |r| r.get(0),
        )
        .map_err(books_store::StoreError::from)?;
    Ok(n > 0)
}

/// The statement's snapshot evidence (reconcile-1's `statement.evidence_id`).
pub fn snapshot_of(conn: &Connection, statement: i64) -> Result<Option<i64>, Refusal> {
    if !has_column(conn, "statement", "evidence_id")? {
        return Ok(None);
    }
    Ok(conn
        .query_row(
            "SELECT evidence_id FROM statement WHERE statement_id = ?1",
            [statement],
            |r| r.get::<_, Option<i64>>(0),
        )
        .optional()
        .map_err(books_store::StoreError::from)?
        .flatten())
}

/// A line's page and box (reconcile-1's `statement_line` columns).
pub fn box_of(conn: &Connection, line: i64) -> Result<Option<LineBox>, Refusal> {
    if !has_column(conn, "statement_line", "page")? {
        return Ok(None);
    }
    let found = conn
        .query_row(
            "SELECT page, box_left, box_top, box_width, box_height FROM statement_line WHERE line_id = ?1",
            [line],
            |r| {
                Ok((
                    r.get::<_, Option<i64>>(0)?,
                    r.get::<_, Option<i64>>(1)?,
                    r.get::<_, Option<i64>>(2)?,
                    r.get::<_, Option<i64>>(3)?,
                    r.get::<_, Option<i64>>(4)?,
                ))
            },
        )
        .optional()
        .map_err(books_store::StoreError::from)?;
    Ok(match found {
        Some((Some(page), Some(left), Some(top), Some(width), Some(height))) if page >= 1 => {
            let c = |v: i64| v.clamp(0, 10_000);
            Some(LineBox {
                page,
                left: c(left),
                top: c(top),
                width: c(width),
                height: c(height),
            })
        }
        _ => None,
    })
}

/// Hundredths of a percent as CSS text: `1234` is `"12.34%"`.
pub fn percent(hundredths: i64) -> String {
    let v = hundredths.clamp(0, 10_000);
    format!("{}.{:02}%", v / 100, v % 100)
}

/// The `statement_view` hunk: whether the statement has a snapshot, and each
/// row's page and box (percent text), unless an earlier hunk set them.
pub fn snapshot_fields(
    store: &Store,
    s: &Statement,
    lines: &[StatementLine],
    statement: &mut Value,
) -> Result<(), Refusal> {
    let conn = store.conn();
    statement["snapshot"] = json!(snapshot_of(conn, s.id)?.is_some());
    let no = numbers(conn, s.id)?;
    let Some(rows) = statement["rows"].as_array_mut() else {
        return Ok(());
    };
    for (row, l) in rows.iter_mut().zip(lines) {
        row["no"] = json!(no.get(&l.id));
        if row.get("page").is_some() {
            continue;
        }
        if let Some(b) = box_of(conn, l.id)? {
            row["page"] = json!(b.page);
            row["box"] = json!({ "left": percent(b.left), "top": percent(b.top), "width": percent(b.width), "height": percent(b.height) });
        }
    }
    Ok(())
}

/// Each line's `line_no`, by line id.
fn numbers(conn: &Connection, statement: i64) -> Result<HashMap<i64, i64>, Refusal> {
    let mut stmt = conn
        .prepare("SELECT line_id, line_no FROM statement_line WHERE statement_id = ?1")
        .map_err(books_store::StoreError::from)?;
    let rows = stmt
        .query_map([statement], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(books_store::StoreError::from)?;
    Ok(rows
        .collect::<Result<HashMap<i64, i64>, _>>()
        .map_err(books_store::StoreError::from)?)
}

fn month_arg(args: &Map<String, Value>) -> Result<Period, Refusal> {
    Period::parse(service::required(args, "month")?)
        .filter(|p| p.kind == PeriodKind::Month)
        .ok_or_else(|| Refusal::invalid("`month` is YYYY-MM."))
}

fn whole(args: &Map<String, Value>, key: &str) -> Result<Option<i64>, Refusal> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_i64()
            .filter(|n| *n >= 1)
            .map(Some)
            .ok_or_else(|| Refusal::invalid(format!("`{key}` is a whole number from 1."))),
    }
}

fn month_title(month: &Period) -> String {
    format!(
        "{} {}",
        MONTHS[usize::from(month.start.month - 1)],
        month.start.year
    )
}

fn state_label(m: &Match) -> &'static str {
    match m {
        Match::Exact(_) => "Matched",
        Match::Suggested(_) => "Suggested",
        Match::Missing => "Not in Books",
    }
}

const STYLE: &str = "body{font:13px -apple-system,Helvetica,Arial,sans-serif;color:#1d1d1f;margin:24px}h1{font-size:20px;margin:0 0 4px}h2{font-size:15px;margin:20px 0 6px}p.sub{color:#6e6e73;margin:0 0 12px}table{border-collapse:collapse;width:100%}th,td{text-align:left;padding:4px 8px;border-bottom:1px solid #e5e5ea}td.n,th.n{text-align:right;font-variant-numeric:tabular-nums}tr.total td{font-weight:600;border-top:2px solid #1d1d1f}footer{margin-top:24px;color:#6e6e73;font-size:11px}.sheet{position:relative}.sheet img{width:100%;display:block}.box{position:absolute;border:1px solid #0a84ff;box-sizing:border-box}.box.hl{border:3px solid #ff9f0a;background:rgba(255,159,10,.15)}.side{display:flex;gap:16px}.side object{flex:1;height:900px}.side ol{width:280px}li.hl{font-weight:600}";

/// The preview result: one embedded HTML resource and a neutral title.
fn result(kind: &str, title: String, text: String) -> Value {
    json!({
        "content": [{ "type": "resource", "resource": { "uri": format!("books://reconcile/{kind}"), "mimeType": "text/html", "text": text } }],
        "structuredContent": { "title": title },
        "isError": false,
    })
}

/// `books_app_reconcile_document`.
pub fn document(store: &Store, book: Book, args: &Map<String, Value>) -> Result<Value, Refusal> {
    let account = service::account(store, &book, service::required(args, "account")?)?;
    let month = month_arg(args)?;
    let kind = service::required(args, "kind")?;
    if kind != "report" && kind != "page" {
        return Err(Refusal::invalid("`kind` is report or page."));
    }
    let page_no = whole(args, "page")?.unwrap_or(1);
    let highlight = whole(args, "line")?;
    let Some((s, lines)) = months::statement(store.conn(), book.id, account.id, &month.key)? else {
        return Err(Refusal::new(
            "not_found",
            "There is no statement for that month.",
        ));
    };
    let found = months::matches(store.conn(), book.id, &s, &lines)?;
    if kind == "report" {
        let (ours, diff) = months::difference(store.conn(), &book, &s)?;
        let finished: Option<String> = store
            .conn()
            .query_row(
                "SELECT finished_at FROM statement WHERE statement_id = ?1",
                [s.id],
                |r| r.get(0),
            )
            .map_err(books_store::StoreError::from)?;
        let title = format!("Bank reconciliation · {}", month_title(&month));
        let body = report_body(
            &account.name,
            &month,
            &s,
            finished.as_deref(),
            ours,
            diff,
            &lines,
            &found,
        )?;
        let text = html::page(&title, STYLE, &body);
        return Ok(result("report", title, text));
    }
    let Some(evidence) = snapshot_of(store.conn(), s.id)? else {
        return Err(Refusal::new(
            "no_snapshot",
            "This statement has no snapshot",
        ));
    };
    let (_owner, mime, len): (i64, String, i64) = store
        .conn()
        .query_row(
            "SELECT book_id, mime, byte_length FROM evidence WHERE id = ?1",
            [evidence],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .map_err(books_store::StoreError::from)?
        .filter(|e| e.0 == book.id)
        .ok_or_else(|| Refusal::new("no_snapshot", "This statement has no snapshot"))?;
    let len = usize::try_from(len).unwrap_or(usize::MAX);
    // the encoded bytes plus the page around them
    let room = MAX_DOCUMENT.saturating_sub(STYLE.len() + 1024 + lines.len() * 512);
    if html::base64_len(len) > room {
        return Err(Refusal::new(
            "too_large",
            "The snapshot is too large to show here",
        ));
    }
    let bytes: Vec<u8> = store
        .conn()
        .query_row(
            "SELECT bytes FROM evidence WHERE id = ?1",
            [evidence],
            |r| r.get(0),
        )
        .map_err(books_store::StoreError::from)?;
    let data = format!("data:{mime};base64,{}", html::base64(&bytes));
    let no = numbers(store.conn(), s.id)?;
    let lit = |l: &StatementLine| highlight.is_some() && highlight == no.get(&l.id).copied();
    let mut on_page = Vec::new();
    for l in &lines {
        if let Some(b) = box_of(store.conn(), l.id)?
            && b.page == page_no
        {
            on_page.push((l, b));
        }
    }
    let shown = |l: &StatementLine| {
        escape(&format!(
            "{} · {}",
            l.line.date.short(),
            money::display(l.line.amount)
        ))
    };
    let mut body = format!(
        "<h1>Statement page {page_no}</h1><p class=\"sub\">{} · {}</p>",
        escape(&account.name),
        escape(&month.label())
    );
    match mime.as_str() {
        "image/png" | "image/jpeg" | "image/webp" => {
            body.push_str(&format!("<div class=\"sheet\" style=\"position:relative\"><img src=\"{data}\" style=\"width:100%\" alt=\"Statement page {page_no}\">"));
            for (l, b) in &on_page {
                let hl = if lit(l) { " hl" } else { "" };
                body.push_str(&format!(
                    "<div class=\"box{hl}\" style=\"left:{};top:{};width:{};height:{}\" title=\"{}\"></div>",
                    percent(b.left),
                    percent(b.top),
                    percent(b.width),
                    percent(b.height),
                    shown(l)
                ));
            }
            body.push_str("</div>");
        }
        "application/pdf" => {
            body.push_str(&format!(
                "<div class=\"side\"><object data=\"{data}\" type=\"application/pdf\"></object><ol>"
            ));
            for (l, _) in &on_page {
                let hl = if lit(l) { " class=\"hl\"" } else { "" };
                body.push_str(&format!(
                    "<li{hl}>{} · {}</li>",
                    shown(l),
                    escape(&l.line.description)
                ));
            }
            body.push_str("</ol></div>");
        }
        _ => {
            return Err(Refusal::new(
                "not_supported",
                "This snapshot cannot be shown here",
            ));
        }
    }
    let title = format!("Statement page {page_no} · {}", month_title(&month));
    let text = html::page(&title, STYLE, &body);
    if text.len() > MAX_DOCUMENT {
        return Err(Refusal::new(
            "too_large",
            "The snapshot is too large to show here",
        ));
    }
    Ok(result("page", title, text))
}

#[allow(clippy::too_many_arguments)]
fn report_body(
    account: &str,
    month: &Period,
    s: &Statement,
    finished: Option<&str>,
    ours: i64,
    diff: i64,
    lines: &[StatementLine],
    found: &[Match],
) -> Result<String, Refusal> {
    let status = match (s.status.as_str(), finished) {
        ("finished", Some(at)) => format!("Finished on {}", at.get(..10).unwrap_or(at)),
        ("finished", None) => "Finished".to_string(),
        _ => "Open".to_string(),
    };
    let mut body = format!(
        "<h1>Bank reconciliation</h1><p class=\"sub\">{} · {} · {}</p>",
        escape(account),
        escape(&month.label()),
        escape(&status)
    );
    body.push_str(&format!(
        "<table><tbody><tr><td>Statement ending</td><td class=\"n\">{}</td></tr><tr><td>Books balance</td><td class=\"n\">{}</td></tr><tr><td>Difference</td><td class=\"n\">{}</td></tr></tbody></table>",
        money::display(s.closing),
        money::display(ours),
        money::display(diff)
    ));
    for (heading, money_in) in [("Money in", true), ("Money out", false)] {
        let mut total: i128 = 0;
        let mut rows = String::new();
        for (l, m) in lines.iter().zip(found) {
            if (l.line.amount > 0) != money_in || l.line.amount == 0 {
                continue;
            }
            let magnitude = i128::from(l.line.amount).abs();
            total += magnitude;
            rows.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td class=\"n\">{}</td><td>{}</td></tr>",
                escape(&l.line.date.short()),
                escape(&l.line.description),
                money::display(i64::try_from(magnitude).unwrap_or(i64::MAX)),
                state_label(m)
            ));
        }
        let total = i64::try_from(total)
            .map_err(|_| Refusal::new("overflow", "That total is too large to show."))?;
        body.push_str(&format!(
            "<h2>{heading}</h2><table><thead><tr><th>Date</th><th>Description</th><th class=\"n\">Amount</th><th>Match</th></tr></thead><tbody>{rows}<tr class=\"total\"><td colspan=\"2\">Total</td><td class=\"n\">{}</td><td></td></tr></tbody></table>",
            money::display(total)
        ));
    }
    body.push_str(&format!(
        "<footer>Prepared by Books on {}</footer>",
        Date::today()
    ));
    Ok(body)
}
