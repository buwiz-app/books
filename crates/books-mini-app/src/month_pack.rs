//! The month pack: one static HTML document for `buwiz.preview` (Bot
//! bridge.md, "preview"): the month's P&L, balance sheet, cash flow, trial
//! balance and reconcile status. Every figure is read from the reports'
//! own JSON (`*_shown` text); nothing is summed here. The page loads
//! nothing: one inline style, no script, no link, no URL.

use books_core::date::{Date, Period};
use books_store::Store;
use books_store::books::Book;
use books_store::rusqlite::params;
use serde_json::{Map, Value, json};

use crate::service::{Answer, Refusal};
use crate::{reconcile, reports, views};

/// Headroom under the Bot's 8 MiB preview cap.
pub const MAX_BYTES: usize = 4 * 1024 * 1024;

/// The pack's resource URI for a month key.
pub fn uri(month_key: &str) -> String {
    format!("books://print/month-pack-{month_key}.html")
}

/// HTML text escaping for every string that comes from the store.
pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
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

fn s(v: &Value) -> String {
    esc(v.as_str().unwrap_or(""))
}

const STYLE: &str = "@page{size:A4;margin:16mm}\
body{font:11pt -apple-system,'Helvetica Neue',Arial,sans-serif;color:#111;margin:0}\
h1{font-size:20pt;margin:0 0 4mm}h2{font-size:14pt;margin:0 0 3mm}\
section{page-break-before:always;break-before:page}\
table{width:100%;border-collapse:collapse;margin-bottom:4mm}\
th,td{padding:1.2mm 2mm;border-bottom:1px solid #ddd;text-align:left;vertical-align:top}\
th{font-weight:600;border-bottom:1px solid #999}\
td.n,th.n{text-align:right;font-family:ui-monospace,Menlo,Consolas,monospace;white-space:nowrap}\
tr.t td{font-weight:600;border-top:1px solid #999}\
.cover dt{color:#555}.cover dd{margin:0 0 2mm}\
footer{margin-top:8mm;color:#555;font-size:9pt}\
@media screen{body{margin:24px}}";

fn report(
    store: &Store,
    book: &Book,
    args: &Map<String, Value>,
    kind: &str,
    month: &Period,
) -> Answer {
    let mut a = Map::new();
    if let Some(b) = args.get("book") {
        a.insert("book".into(), b.clone());
    }
    a.insert("kind".into(), json!(kind));
    a.insert("period_kind".into(), json!("month"));
    a.insert("period".into(), json!(month.key));
    reports::report(store, book.clone(), &a)
}

fn count(store: &Store, sql: &str, book: &Book, month: &Period) -> Result<i64, Refusal> {
    Ok(store
        .conn()
        .query_row(
            sql,
            params![book.id, month.start.to_string(), month.end.to_string()],
            |r| r.get(0),
        )
        .map_err(books_store::StoreError::from)?)
}

fn rows(report: &Value) -> impl Iterator<Item = &Value> {
    report["sections"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|sec| sec["rows"].as_array().into_iter().flatten())
}

fn pl_html(pl: &Value) -> String {
    let cols = &pl["columns"];
    let mut h = format!(
        "<section><h2>Profit and loss</h2><table><tr><th>Account</th><th class=n>{}</th><th class=n>{}</th></tr>",
        s(&cols[0]),
        s(&cols[1])
    );
    for sec in pl["sections"].as_array().into_iter().flatten() {
        h.push_str(&format!(
            "<tr class=t><td colspan=3>{}</td></tr>",
            s(&sec["title"])
        ));
        for r in sec["rows"].as_array().into_iter().flatten() {
            h.push_str(&format!(
                "<tr><td>{}</td><td class=n>{}</td><td class=n>{}</td></tr>",
                s(&r["name"]),
                s(&r["shown"]),
                s(&r["prev_shown"])
            ));
        }
    }
    for t in pl["totals"]
        .as_array()
        .into_iter()
        .flatten()
        .chain([&pl["net"]])
    {
        h.push_str(&format!(
            "<tr class=t><td>{}</td><td class=n>{}</td><td class=n>{}</td></tr>",
            s(&t["label"]),
            s(&t["shown"]),
            s(&t["prev_shown"])
        ));
    }
    h.push_str("</table></section>");
    h
}

fn bs_html(bs: &Value) -> String {
    let mut h = format!(
        "<section><h2>Balance sheet</h2><table><tr><th>Account</th><th class=n>{}</th></tr>",
        s(&bs["columns"][0])
    );
    for sec in bs["sections"].as_array().into_iter().flatten() {
        h.push_str(&format!(
            "<tr class=t><td colspan=2>{}</td></tr>",
            s(&sec["title"])
        ));
        for r in sec["rows"].as_array().into_iter().flatten() {
            h.push_str(&format!(
                "<tr><td>{}</td><td class=n>{}</td></tr>",
                s(&r["name"]),
                s(&r["shown"])
            ));
        }
        h.push_str(&format!(
            "<tr class=t><td>{}</td><td class=n>{}</td></tr>",
            s(&sec["total"]["label"]),
            s(&sec["total"]["shown"])
        ));
    }
    h.push_str(&format!(
        "<tr class=t><td>{}</td><td class=n>{}</td></tr><tr><td colspan=2>{}</td></tr></table></section>",
        s(&bs["net"]["label"]),
        s(&bs["net"]["shown"]),
        s(&bs["check"])
    ));
    h
}

fn cf_html(cf: &Value) -> String {
    let mut h = format!(
        "<section><h2>Cash flow</h2><table><tr><th>{}</th><th class=n></th></tr>",
        s(&cf["columns"][0])
    );
    for r in rows(cf) {
        h.push_str(&format!(
            "<tr><td>{}</td><td class=n>{}</td></tr>",
            s(&r["name"]),
            s(&r["shown"])
        ));
    }
    h.push_str(&format!(
        "<tr class=t><td>{}</td><td class=n>{}</td></tr></table></section>",
        s(&cf["net"]["label"]),
        s(&cf["net"]["shown"])
    ));
    h
}

fn tb_html(tb: &Value) -> String {
    let cols = &tb["columns"];
    let mut h = format!(
        "<section><h2>Trial balance</h2><table><tr><th>{}</th><th class=n>{}</th><th class=n>{}</th></tr>",
        s(&cols[0]),
        s(&cols[1]),
        s(&cols[2])
    );
    for r in rows(tb) {
        h.push_str(&format!(
            "<tr><td>{} {}</td><td class=n>{}</td><td class=n>{}</td></tr>",
            s(&r["number"]),
            s(&r["name"]),
            s(&r["debit_shown"]),
            s(&r["credit_shown"])
        ));
    }
    h.push_str(&format!(
        "<tr class=t><td>{}</td><td class=n>{}</td><td class=n>{}</td></tr></table></section>",
        s(&tb["net"]["label"]),
        s(&tb["net"]["debit_shown"]),
        s(&tb["net"]["credit_shown"])
    ));
    h
}

/// Each reconcile row's state label for `month` ("—" when the grid has no
/// cell for it).
fn reconcile_html(grid: &Value, month: &Period) -> String {
    let mut h = String::from(
        "<section><h2>Reconcile status</h2><table><tr><th>Account</th><th>State</th></tr>",
    );
    for r in grid["rows"].as_array().into_iter().flatten() {
        let label = r["cells"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|c| c["month"] == month.key.as_str())
            .map_or_else(|| "—".to_string(), |c| s(&c["label"]));
        h.push_str(&format!(
            "<tr><td>{}</td><td>{label}</td></tr>",
            s(&r["name"])
        ));
    }
    h.push_str("</table></section>");
    h
}

/// `books_app_month_pack`: `{title, month: {key, label}, html, bytes}`. The
/// title is neutral (no book label, amount, name or TIN).
pub fn pack(store: &Store, book: Book, args: &Map<String, Value>) -> Answer {
    let month = views::month_arg(args)?;
    let pl = report(store, &book, args, "pl", &month)?;
    let bs = report(store, &book, args, "bs", &month)?;
    let cf = report(store, &book, args, "cf", &month)?;
    let tb = report(store, &book, args, "tb", &month)?;
    let grid = reconcile::grid(store, book.clone())?;
    let drafts = count(
        store,
        "SELECT count(*) FROM entry WHERE book_id = ?1 AND status = 'draft' AND needs_review = 0
           AND entry_date >= ?2 AND entry_date <= ?3",
        &book,
        &month,
    )?;
    // the same lines as books_book_get's to_review, in the month
    let to_review = count(
        store,
        "SELECT count(*) FROM entry e WHERE e.book_id = ?1 AND e.entry_date >= ?2 AND e.entry_date <= ?3 AND (
           (e.status = 'draft' AND e.needs_review = 1)
           OR (e.status = 'posted' AND e.form <> 'reversal'
               AND e.recategorized_by_entry_id IS NULL AND e.recategorizes_entry_id IS NULL AND EXISTS (
             SELECT 1 FROM line l JOIN account a ON a.account_id = l.account_id
             WHERE l.entry_id = e.entry_id AND a.kind = 'unsure')))",
        &book,
        &month,
    )?;
    let label = month.label();
    let title = format!("Month pack · {label}");
    let mut html = format!(
        "<!DOCTYPE html><html lang=en><head><meta charset=utf-8><title>{t}</title><style>{STYLE}{dark}</style></head><body>\
         <div class=cover><h1>{t}</h1><dl>\
         <dt>Book</dt><dd>{book}</dd><dt>Basis</dt><dd>{basis}</dd><dt>Month</dt><dd>{m}</dd>\
         <dt>Generated</dt><dd>{today}</dd>\
         <dt>Drafts this month</dt><dd>{drafts}</dd><dt>Lines to review</dt><dd>{to_review}</dd></dl></div>",
        t = esc(&title),
        dark = crate::html::SCREEN_DARK,
        book = esc(&book.label),
        basis = s(&pl["basis_label"]),
        m = esc(&label),
        today = Date::today(),
    );
    html.push_str(&pl_html(&pl));
    html.push_str(&bs_html(&bs));
    html.push_str(&cf_html(&cf));
    html.push_str(&tb_html(&tb));
    html.push_str(&reconcile_html(&grid, &month));
    html.push_str(
        "<footer>Drafts are not included. Prepared by Books; not a tax return.</footer></body></html>",
    );
    if html.len() > MAX_BYTES {
        return Err(Refusal::invalid("The month pack is too large to preview."));
    }
    Ok(json!({
        "title": title,
        "month": { "key": month.key, "label": label },
        "bytes": html.len(),
        "html": html,
    }))
}
