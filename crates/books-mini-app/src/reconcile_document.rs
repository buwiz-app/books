//! `books_app_reconcile_document`: the static pages `buwiz.preview` shows for a
//! statement, the reconciliation report and one page of the statement's
//! snapshot with its lines boxed. Read-only and app only (the Bot runs a
//! preview as the app). The title carries no amount, number or name.
//! `books_app_statement_page` gives the Books window the snapshot image
//! itself, in parts, to draw beside the match list.

use books_core::date::{Date, MONTHS, Period, PeriodKind};
use books_core::money;
use books_core::statement::Match;
use books_store::Store;
use books_store::books::Book;
use books_store::months::{self, Statement, StatementLine};
use books_store::rusqlite::{Connection, OptionalExtension as _};
use books_store::statement_drafts::{self as drafts, Extra};
use sha2::{Digest as _, Sha256};
use std::collections::HashMap;

use serde_json::{Map, Value, json};

use crate::html::{self, escape};
use crate::service::{self, Refusal};

/// The preview document cap (Bot `permissions.preview.max_bytes`, 8 MiB),
/// kept under so the Bot never answers `result_too_large`.
pub const MAX_DOCUMENT: usize = 8_000_000;

/// A line's box on its page, as the statement read stores it: page corners
/// `[x0, y0, x1, y1]` in thousandths of the page.
pub type Bbox = books_store::statement_drafts::Bbox;

/// The statement's snapshot evidence and each line's page and box, in line
/// order (the statement read's `statement.evidence_id` and `statement_line` columns).
fn snapshot_of(conn: &Connection, statement: i64) -> Result<(Option<i64>, Vec<Extra>), Refusal> {
    Ok(drafts::extras(conn, statement)?)
}

/// The evidence that pictures one page: the page's own picture when the
/// statement read named one, else, for page 1, the statement's document. No
/// other page has one: a statement keeps a single document, so a picture is
/// its first page, and a PDF's pages are inside a file Books does not draw.
fn page_evidence(conn: &Connection, statement: i64, page: i64) -> Result<Option<i64>, Refusal> {
    if let Some(own) = drafts::pages(conn, statement)?
        .into_iter()
        .find(|p| p.0 == page)
    {
        return Ok(Some(own.1));
    }
    Ok(if page == 1 {
        snapshot_of(conn, statement)?.0
    } else {
        None
    })
}

/// Thousandths of the page as CSS percent text: `1234` is `"123.4%"`.
pub fn percent(thousandths: i64) -> String {
    let v = thousandths.clamp(0, 1000);
    format!("{}.{}%", v / 10, v % 10)
}

/// CSS `left/top/width/height` for a corner box, from integer math.
pub fn box_css(b: &Bbox) -> String {
    format!(
        "left:{};top:{};width:{};height:{}",
        percent(b[0]),
        percent(b[1]),
        percent(b[2] - b[0]),
        percent(b[3] - b[1])
    )
}

/// The `statement_view` hunk: whether the statement has a snapshot, and each
/// row's `no`. Page and box come from the statement read's `view_extra`.
pub fn snapshot_fields(
    store: &Store,
    s: &Statement,
    lines: &[StatementLine],
    statement: &mut Value,
) -> Result<(), Refusal> {
    let conn = store.conn();
    let evidence = snapshot_of(conn, s.id)?.0;
    // a document, or a picture of any page: something See the page can open
    statement["snapshot"] = json!(evidence.is_some() || !drafts::pages(conn, s.id)?.is_empty());
    // what the Books window can do with it: draw the page itself (`image`),
    // or leave it to the preview (`pdf`, `large`, `other`)
    statement["snapshot_kind"] = match evidence {
        Some(id) => json!(snapshot_kind(conn, s.id, id)?),
        None => Value::Null,
    };
    let no = numbers(conn, s.id)?;
    // the pages the Books window can draw: each has a PNG or JPEG of its own
    // within the bound. A page that is not here shows a note, never bare boxes.
    let mut wanted: Vec<i64> = snapshot_of(conn, s.id)?
        .1
        .iter()
        .filter_map(|e| e.page)
        .chain(drafts::pages(conn, s.id)?.into_iter().map(|p| p.0))
        .chain(evidence.map(|_| 1))
        .collect();
    wanted.sort_unstable();
    wanted.dedup();
    let mut pictures = Vec::new();
    let mut stamp = Sha256::new();
    for page in wanted {
        if let Some(id) = page_evidence(conn, s.id, page)?
            && snapshot_kind(conn, s.id, id)? == "image"
        {
            pictures.push(page);
            let sha: String = conn
                .query_row("SELECT sha256 FROM evidence WHERE id = ?1", [id], |r| {
                    r.get(0)
                })
                .map_err(books_store::StoreError::from)?;
            stamp.update(format!("{page}:{sha};"));
        }
    }
    statement["snapshot_pictures"] = json!(pictures);
    // which pictures those are: it changes when any page's picture does, so
    // the window never keeps showing one it read for an earlier statement read
    statement["snapshot_stamp"] = json!(
        stamp.finalize()[..8]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    let Some(rows) = statement["rows"].as_array_mut() else {
        return Ok(());
    };
    for (row, l) in rows.iter_mut().zip(lines) {
        row["no"] = json!(no.get(&l.id));
    }
    Ok(())
}

/// The largest snapshot image the Books window draws itself. Larger ones
/// stay on the preview.
pub const MAX_PAGE_IMAGE: usize = 6_000_000;

/// The bytes one `books_app_statement_page` call carries: 400,000 characters
/// of base64. An answer holds them twice (its text and its structured
/// content), which stays under the 1 MiB a `buwiz.call` result may be. A
/// multiple of 3, so the parts' base64 joins into one.
pub const PAGE_PART: usize = 300_000;

/// The snapshot's mime and length, when it is this book's.
fn snapshot_meta(
    conn: &Connection,
    book: i64,
    evidence: i64,
) -> Result<Option<(String, usize)>, Refusal> {
    Ok(conn
        .query_row(
            "SELECT book_id, mime, byte_length FROM evidence WHERE id = ?1",
            [evidence],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            },
        )
        .optional()
        .map_err(books_store::StoreError::from)?
        .filter(|e| e.0 == book)
        .map(|e| (e.1, usize::try_from(e.2).unwrap_or(usize::MAX))))
}

/// What the Books window can do with a statement's snapshot.
fn snapshot_kind(
    conn: &Connection,
    statement: i64,
    evidence: i64,
) -> Result<&'static str, Refusal> {
    let book: i64 = conn
        .query_row(
            "SELECT book_id FROM statement WHERE statement_id = ?1",
            [statement],
            |r| r.get(0),
        )
        .map_err(books_store::StoreError::from)?;
    Ok(match snapshot_meta(conn, book, evidence)? {
        Some((mime, len)) if mime == "image/png" || mime == "image/jpeg" => {
            if len <= MAX_PAGE_IMAGE {
                "image"
            } else {
                "large"
            }
        }
        Some((mime, _)) if mime == "application/pdf" => "pdf",
        _ => "other",
    })
}

/// `books_app_statement_page`: one part of the statement's snapshot image,
/// as standard base64, for the Books window to draw beside the match list.
/// PNG and JPEG only, at most [`MAX_PAGE_IMAGE`] bytes, [`PAGE_PART`] bytes a
/// call. Read-only and app only; the answer carries no amount, name or ticket.
pub fn page_image(store: &Store, book: Book, args: &Map<String, Value>) -> Result<Value, Refusal> {
    let account = service::account(store, &book, service::required(args, "account")?)?;
    let month = month_arg(args)?;
    let part = match args.get("part") {
        None | Some(Value::Null) => 0,
        Some(v) => v
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| Refusal::invalid("`part` is a whole number from 0."))?,
    };
    let page = whole(args, "page")?.unwrap_or(1);
    let conn = store.conn();
    let Some((s, _)) = months::statement(conn, book.id, account.id, &month.key)? else {
        return Err(Refusal::new(
            "not_found",
            "There is no statement for that month.",
        ));
    };
    let no_snapshot = || Refusal::new("no_snapshot", "This page has no picture");
    let evidence = page_evidence(conn, s.id, page)?.ok_or_else(no_snapshot)?;
    let (mime, len) = snapshot_meta(conn, book.id, evidence)?.ok_or_else(no_snapshot)?;
    if mime != "image/png" && mime != "image/jpeg" {
        return Err(Refusal::new(
            "not_supported",
            "This snapshot cannot be shown here",
        ));
    }
    if len > MAX_PAGE_IMAGE {
        return Err(Refusal::new(
            "too_large",
            "The snapshot is too large to show here",
        ));
    }
    let parts = len.div_ceil(PAGE_PART).max(1);
    if part >= parts {
        return Err(Refusal::invalid("`part` is past the last part."));
    }
    let bytes: Vec<u8> = conn
        .query_row(
            "SELECT bytes FROM evidence WHERE id = ?1",
            [evidence],
            |r| r.get(0),
        )
        .optional()
        .map_err(books_store::StoreError::from)?
        .ok_or_else(no_snapshot)?;
    let from = (part * PAGE_PART).min(bytes.len());
    let to = (from + PAGE_PART).min(bytes.len());
    let size = image_size(&mime, &bytes);
    Ok(json!({
        "mime": mime,
        "page": page,
        "part": part,
        "parts": parts,
        "width": size.map(|s| s.0),
        "height": size.map(|s| s.1),
        "data_base64": html::base64(&bytes[from..to]),
    }))
}

/// A PNG's or JPEG's size in pixels, from its header, so the window can lay
/// the line boxes over the page at its own proportions.
fn image_size(mime: &str, b: &[u8]) -> Option<(u32, u32)> {
    let be16 = |at: usize| {
        Some(u32::from(u16::from_be_bytes([
            *b.get(at)?,
            *b.get(at + 1)?,
        ])))
    };
    let size = match mime {
        "image/png" => {
            if !b.starts_with(b"\x89PNG\r\n\x1a\n") || b.get(12..16) != Some(b"IHDR") {
                return None;
            }
            let be32 = |at: usize| Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?));
            (be32(16)?, be32(20)?)
        }
        "image/jpeg" => {
            if !b.starts_with(b"\xff\xd8") {
                return None;
            }
            let mut at = 2;
            loop {
                if *b.get(at)? != 0xff {
                    return None;
                }
                let marker = *b.get(at + 1)?;
                match marker {
                    // fill bytes and markers that carry no length
                    0xff => at += 1,
                    0x01 | 0xd0..=0xd8 => at += 2,
                    // a frame header: precision, height, width
                    0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf => {
                        break (be16(at + 7)?, be16(at + 5)?);
                    }
                    // the scan or the end, with no frame before it
                    0xd9 | 0xda => return None,
                    _ => at += 2 + usize::try_from(be16(at + 2)?).ok()?,
                }
            }
        }
        _ => return None,
    };
    (size.0 > 0 && size.1 > 0).then_some(size)
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
    let (document, extras) = snapshot_of(store.conn(), s.id)?;
    // the page's own picture when it has one, else the statement's document
    let pages = drafts::pages(store.conn(), s.id)?;
    let pictured = !pages.is_empty();
    let own = pages.into_iter().find(|p| p.0 == page_no).map(|p| p.1);
    let Some(evidence) = own.or(document) else {
        if !pictured {
            return Err(Refusal::new(
                "no_snapshot",
                "This statement has no snapshot",
            ));
        }
        // other pages have pictures and this one has none: its lines alone
        let no = numbers(store.conn(), s.id)?;
        let mut body = format!(
            "<h1>Statement page {page_no}</h1><p class=\"sub\">{} · {}</p><p class=\"sub\">No picture was kept for this page. Its lines:</p><ol>",
            escape(&account.name),
            escape(&month.label())
        );
        for (l, _) in lines
            .iter()
            .zip(&extras)
            .filter(|(_, e)| e.page == Some(page_no))
        {
            let hl = if highlight.is_some() && highlight == no.get(&l.id).copied() {
                " class=\"hl\""
            } else {
                ""
            };
            body.push_str(&format!(
                "<li{hl}>{} · {} · {}</li>",
                l.line.date.short(),
                escape(&money::display(l.line.amount)),
                escape(&l.line.description)
            ));
        }
        body.push_str("</ol>");
        let title = format!("Statement page {page_no} · {}", month_title(&month));
        let text = html::page(&title, STYLE, &body);
        return Ok(result("page", title, text));
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
        .optional()
        .map_err(books_store::StoreError::from)?
        .ok_or_else(|| Refusal::new("no_snapshot", "This statement has no snapshot"))?;
    let data = format!("data:{mime};base64,{}", html::base64(&bytes));
    let no = numbers(store.conn(), s.id)?;
    let lit = |l: &StatementLine| highlight.is_some() && highlight == no.get(&l.id).copied();
    let on_page: Vec<(&StatementLine, Option<Bbox>)> = lines
        .iter()
        .zip(&extras)
        .filter(|(_, e)| e.page == Some(page_no))
        .map(|(l, e)| (l, e.bbox))
        .collect();
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
        // a picture is one page: without its own, another page has none to show
        "image/png" | "image/jpeg" if own.is_none() && page_no != 1 => {
            body.push_str("<p class=\"sub\">No picture was kept for this page. Its lines:</p><ol>");
            for (l, _) in &on_page {
                let hl = if lit(l) { " class=\"hl\"" } else { "" };
                body.push_str(&format!(
                    "<li{hl}>{} · {}</li>",
                    shown(l),
                    escape(&l.line.description)
                ));
            }
            body.push_str("</ol>");
        }
        "image/png" | "image/jpeg" => {
            body.push_str(&format!("<div class=\"sheet\" style=\"position:relative\"><img src=\"{data}\" style=\"width:100%\" alt=\"Statement page {page_no}\">"));
            for (l, b) in on_page.iter().filter_map(|(l, b)| b.map(|b| (l, b))) {
                let hl = if lit(l) { " hl" } else { "" };
                body.push_str(&format!(
                    "<div class=\"box{hl}\" style=\"{}\" title=\"{}\"></div>",
                    box_css(&b),
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
    for (heading, sign) in [("Money in", 1), ("Money out", -1), ("No amount", 0)] {
        let mut total: i128 = 0;
        let mut rows = String::new();
        for (l, m) in lines.iter().zip(found) {
            if l.line.amount.signum() != sign {
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
        if sign == 0 && rows.is_empty() {
            continue;
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
