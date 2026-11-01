//! M5: old books in. A web Buwiz Books export, or a CSV chart and journal,
//! become one import plan (`books_core::import`), applied in one
//! transaction (`books_store::imports::apply`). Then the owner matches each
//! imported account to a kind (Map 1: suggested, never assumed), enters
//! opening totals with proof for a mid-year start, and OKs return events.
//!
//! Files arrive in parts: the Bot caps one call's arguments at 256 KiB, so the
//! app sends a picked file as base64url parts (`books_app_upload_part`) and
//! the import names the staged upload. Parts live in memory only, and an
//! import takes them.

use std::collections::BTreeMap;

use books_core::catalog::{AccountType, Catalog};
use books_core::date::{Date, MONTHS};
use books_core::import::{self, Plan, Source, Tag};
use books_core::ledger::{Basis, KindRef};
use books_core::money;
use books_store::Store;
use books_store::books::Book;
use books_store::imports::{self as store_imports, OPENING_MEASURES, Opening};
use books_store::rusqlite::{Connection, params};
use serde_json::{Map, Value, json};

use crate::forms::measure_label;
use crate::service::{Answer, Origin, Refusal, required, text, ticket, with_subject};
use crate::writes;

/// The largest file Books takes in parts (`import::MAX_EXPORT_BYTES`).
pub const MAX_UPLOAD: usize = import::MAX_EXPORT_BYTES;
/// At most this many uploads wait at once; the oldest goes first.
const MAX_WAITING: usize = 4;

/// Files waiting in parts, by the app's upload id.
#[derive(Default)]
pub struct Uploads {
    waiting: Vec<(String, Vec<Option<Vec<u8>>>)>,
}

fn upload_id(id: &str) -> Result<&str, Refusal> {
    let ok = (8..=64).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    if ok {
        Ok(id)
    } else {
        Err(Refusal::invalid(
            "`upload` is 8 to 64 letters, digits, - or _.",
        ))
    }
}

impl Uploads {
    /// Keep part `part` (from 0) of `parts`. Returns how many have arrived.
    pub fn stage(
        &mut self,
        id: &str,
        part: usize,
        parts: usize,
        bytes: Vec<u8>,
    ) -> Result<usize, Refusal> {
        let id = upload_id(id)?;
        if parts == 0 || parts > MAX_UPLOAD / 1024 || part >= parts {
            return Err(Refusal::invalid(
                "`part` counts from 0 and is less than `parts`.",
            ));
        }
        let pos = match self
            .waiting
            .iter()
            .position(|(k, p)| k == id && p.len() == parts)
        {
            Some(i) => i,
            None => {
                self.waiting.retain(|(k, _)| k != id);
                if self.waiting.len() >= MAX_WAITING {
                    self.waiting.remove(0);
                }
                self.waiting.push((id.to_string(), vec![None; parts]));
                self.waiting.len() - 1
            }
        };
        let slots = &mut self.waiting[pos].1;
        let size: usize = slots.iter().flatten().map(Vec::len).sum::<usize>()
            - slots[part].as_ref().map_or(0, Vec::len)
            + bytes.len();
        if size > MAX_UPLOAD {
            self.waiting.remove(pos);
            return Err(Refusal::new("too_large", "A file is at most 50 MB."));
        }
        slots[part] = Some(bytes);
        Ok(slots.iter().filter(|s| s.is_some()).count())
    }

    /// The whole file, when every part has arrived. It leaves the queue.
    pub fn take(&mut self, id: &str) -> Result<Vec<u8>, Refusal> {
        let id = upload_id(id)?;
        let pos = self
            .waiting
            .iter()
            .position(|(k, _)| k == id)
            .ok_or_else(|| {
                Refusal::new("not_found", "That file has not arrived. Pick it again.")
            })?;
        if self.waiting[pos].1.iter().any(Option::is_none) {
            return Err(Refusal::new(
                "incomplete",
                "Part of that file is missing. Pick it again.",
            ));
        }
        let (_, parts) = self.waiting.remove(pos);
        Ok(parts.into_iter().flatten().flatten().collect())
    }
}

/// `books_app_upload_part`: one base64url part of a picked file.
pub fn upload_part(uploads: &mut Uploads, args: &Map<String, Value>) -> Answer {
    let id = required(args, "upload")?;
    let count = |k: &str| -> Result<usize, Refusal> {
        args.get(k)
            .and_then(Value::as_u64)
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| Refusal::invalid(format!("`{k}` is a whole number.")))
    };
    let (part, parts) = (count("part")?, count("parts")?);
    let bytes = crate::b64::decode(required(args, "data_base64")?)
        .ok_or_else(|| Refusal::invalid("`data_base64` is not base64url."))?;
    let received = uploads.stage(id, part, parts, bytes)?;
    Ok(json!({ "upload": id, "received": received, "parts": parts }))
}

/// A text file a request names: inline as `<key>_text`, or staged as
/// `<key>_upload` (a staged `.xlsx` or `.xls` arrives as its CSV).
pub fn file_text(
    uploads: &mut Uploads,
    args: &Map<String, Value>,
    key: &str,
) -> Result<Option<String>, Refusal> {
    if let Some(t) = text(args, &format!("{key}_text")) {
        return Ok(Some(t.to_string()));
    }
    match text(args, &format!("{key}_upload")) {
        Some(id) => {
            let bytes = uploads.take(id)?;
            // a spreadsheet reads as the CSV it would save
            if let Some(csv) = crate::sheets::as_csv(&bytes, key) {
                return csv.map(Some);
            }
            let mut s = String::from_utf8(bytes)
                .map_err(|_| Refusal::invalid("That file is not UTF-8 text."))?;
            if s.starts_with('\u{feff}') {
                s.remove(0);
            }
            Ok(Some(s))
        }
        None => Ok(None),
    }
}

/// The files of an import: `web`, or `chart` and `journal`.
#[derive(Default)]
pub struct Files {
    pub web: Option<String>,
    pub chart: Option<String>,
    pub journal: Option<String>,
}

pub fn files(uploads: &mut Uploads, args: &Map<String, Value>) -> Result<Files, Refusal> {
    Ok(Files {
        web: file_text(uploads, args, "web")?,
        chart: file_text(uploads, args, "chart")?,
        journal: file_text(uploads, args, "journal")?,
    })
}

/// Departments and locations the journal names, once each.
fn tags_of(entries: &[import::Entry]) -> Vec<Tag> {
    let mut out: Vec<Tag> = Vec::new();
    for l in entries.iter().flat_map(|e| &e.lines) {
        for (kind, name) in [("department", &l.department), ("location", &l.location)] {
            if let Some(n) = name
                && !out
                    .iter()
                    .any(|t| t.kind == kind && t.name.eq_ignore_ascii_case(n))
            {
                out.push(Tag {
                    kind,
                    name: n.clone(),
                });
            }
        }
    }
    out
}

/// The import plan of a request.
pub fn plan(args: &Map<String, Value>, files: Files) -> Result<Plan, Refusal> {
    match required(args, "source")? {
        "web" => {
            let web = files
                .web
                .ok_or_else(|| Refusal::invalid("Send the web Books export as `web_upload`."))?;
            import::parse_web_export(&web).map_err(|e| Refusal::new("bad_file", e))
        }
        "csv" => {
            let (Some(chart), Some(journal)) = (files.chart, files.journal) else {
                return Err(Refusal::invalid(
                    "A CSV import takes a chart (`chart_upload`) and a journal (`journal_upload`).",
                ));
            };
            let accounts = import::parse_csv_chart(&chart)
                .map_err(|e| Refusal::new("bad_file", format!("The chart: {e}")))?;
            let entries = import::parse_csv_journal(&journal)
                .map_err(|e| Refusal::new("bad_file", format!("The journal: {e}")))?;
            Ok(Plan {
                source: Source::Csv,
                accounts,
                people: Vec::new(),
                tags: tags_of(&entries),
                entries,
                skipped: Vec::new(),
            })
        }
        _ => Err(Refusal::invalid("`source` is web or csv.")),
    }
}

/// `books_app_import`: apply a plan to a book with nothing posted yet.
pub fn run(conn: &Connection, args: &Map<String, Value>, plan: &Plan) -> Answer {
    let book = writes::book_in(conn, args)?;
    let g = writes::gate(conn, &book);
    let s = store_imports::apply(
        conn,
        &book,
        plan,
        &import::suggest_kind,
        text(args, "file_name"),
        &g,
    )?;
    Ok(with_subject(
        json!({
            "accounts_new": s.accounts_new,
            "accounts_matched": s.accounts_matched,
            "people": s.people,
            "tags": s.tags,
            "posted": s.posted,
            "drafts": s.drafts,
            "skipped": s.skipped.iter().map(|(what, count, why)| json!({ "what": what, "count": count, "why": why })).collect::<Vec<_>>(),
        }),
        "book",
        book.id,
    ))
}

/// The kinds an account of `ty` may take on `book`, as picker options.
fn side_of(ty: AccountType) -> &'static str {
    match ty {
        AccountType::Revenue | AccountType::OtherIncome => "income",
        AccountType::CostOfRevenue | AccountType::Expense | AccountType::OtherExpense => "expense",
        AccountType::Asset => "asset",
        AccountType::Liability => "liability",
        AccountType::Equity => "equity",
    }
}

/// What a kind feeds on the Forms page, in words.
pub fn feeds(kind: &KindRef) -> String {
    match kind {
        KindRef::Mixed => "per line".into(),
        KindRef::Unsure => "nothing yet".into(),
        KindRef::Kind(k) => match Catalog::get().kind(k) {
            Some(def) if !def.measures.is_empty() => {
                let mut out: Vec<String> = def
                    .measures
                    .iter()
                    .map(|m| measure_label(m).to_lowercase())
                    .collect();
                // the sales family also feeds net sales
                if def
                    .measures
                    .iter()
                    .any(|m| m.starts_with("gross_sales") || m.starts_with("sales_"))
                {
                    out.push("net sales".into());
                }
                out.join(", ")
            }
            Some(def) if def.account_type.is_pl() => "reports only".into(),
            _ => "—".into(),
        },
    }
}

fn kind_name(kind: &KindRef) -> String {
    match kind {
        KindRef::Mixed => "Mixed: decide per line".into(),
        KindRef::Unsure => "Not sure yet".into(),
        KindRef::Kind(k) => Catalog::get()
            .kind(k)
            .map_or_else(|| k.clone(), |d| d.name.clone()),
    }
}

fn kind_options(book: &Book) -> Value {
    let mut by_side: BTreeMap<&str, Vec<Value>> = BTreeMap::new();
    for k in &Catalog::get().kinds {
        if k.accrual_only && book.basis == Basis::Cash {
            continue;
        }
        let r = KindRef::Kind(k.id.clone());
        by_side
            .entry(side_of(k.account_type))
            .or_default()
            .push(json!({ "ticket": k.id, "label": k.name, "feeds": feeds(&r) }));
    }
    for list in by_side.values_mut() {
        for r in [KindRef::Mixed, KindRef::Unsure] {
            list.push(json!({ "ticket": r.as_str(), "label": kind_name(&r), "feeds": feeds(&r) }));
        }
    }
    json!(by_side)
}

fn range_label(from: Date, to: Date) -> String {
    let m = |d: Date| MONTHS[usize::from(d.month - 1)];
    if from.month == to.month {
        format!("{} {}", m(from), from.year)
    } else {
        format!("{}–{} {}", m(from), m(to), to.year)
    }
}

/// `books_import_get`: what came in, the matches waiting, opening totals.
pub fn status(store: &Store, book: Book, origin: Origin) -> Answer {
    let c = store.conn();
    let mut batches = Vec::new();
    let mut stmt = c.prepare(
        "SELECT source, file_name, accounts, entries, skipped, imported_at FROM import_batch WHERE book_id = ?1 ORDER BY batch_id DESC",
    ).map_err(books_store::StoreError::from)?;
    let rows = stmt.query_map(params![book.id], |r| {
        Ok(json!({ "source": r.get::<_, String>(0)?, "file_name": r.get::<_, Option<String>>(1)?, "accounts": r.get::<_, i64>(2)?, "entries": r.get::<_, i64>(3)?, "skipped": r.get::<_, String>(4)?, "imported_at": r.get::<_, String>(5)? }))
    }).map_err(books_store::StoreError::from)?;
    for row in rows {
        batches.push(row.map_err(books_store::StoreError::from)?);
    }
    let lines_of = |account: i64| -> Result<i64, Refusal> {
        Ok(c.query_row(
            "SELECT count(*) FROM line l JOIN entry e ON e.entry_id = l.entry_id WHERE l.account_id = ?1 AND e.status IN ('posted','voided')",
            params![account],
            |r| r.get(0),
        )
        .map_err(books_store::StoreError::from)?)
    };
    let (mut to_confirm, mut per_line, mut not_matched, mut sure, mut lines) = (0, 0, 0, 0, 0);
    let mut matches = Vec::new();
    for m in store_imports::matches(c, book.id)? {
        let a = &m.account;
        let status = match (&a.kind, a.kind_confirmed) {
            (KindRef::Unsure, _) => {
                not_matched += 1;
                "unsure"
            }
            (KindRef::Mixed, _) => {
                per_line += 1;
                "mixed"
            }
            (_, true) => "confirmed",
            (_, false) => {
                to_confirm += 1;
                if m.confidence.unwrap_or(0) >= 90 {
                    sure += 1;
                }
                "suggested"
            }
        };
        let n = lines_of(a.id)?;
        lines += n;
        matches.push(json!({
            "ticket": ticket(store, "account", a.id)?,
            "number": a.number,
            "name": a.name,
            "imported_name": m.imported_name,
            "type_label": a.account_type.group(),
            "side": side_of(a.account_type),
            "kind": a.kind.as_str(),
            "kind_name": kind_name(&a.kind),
            "feeds": feeds(&a.kind),
            "confidence": m.confidence,
            "suggested": m.suggested,
            "status": status,
            "lines": n,
        }));
    }
    let openings: Vec<Value> = store_imports::openings(c, book.id)?
        .into_iter()
        .map(|o| {
            let mut v = json!({
                "measure": o.measure,
                "label": measure_label(&o.measure),
                "from": o.from.to_string(),
                "to": o.to.to_string(),
                "range_label": range_label(o.from, o.to),
                "amount_shown": money::display(o.centavos),
            });
            if origin.is_app() {
                v["proof"] = json!(o.proof);
            }
            v
        })
        .collect();
    Ok(json!({
        "batches": batches,
        "matches": matches,
        "counts": { "to_confirm": to_confirm, "per_line": per_line, "not_matched": not_matched, "sure": sure, "accounts": matches.len(), "lines": lines },
        "held": not_matched + per_line + to_confirm > 0,
        "kinds": kind_options(&book),
        "openings": openings,
        "measures": OPENING_MEASURES.iter().map(|(id, label)| json!({ "ticket": id, "label": label })).collect::<Vec<_>>(),
        "can_import": c.query_row("SELECT count(*) FROM entry WHERE book_id = ?1 AND status IN ('posted','voided')", params![book.id], |r| r.get::<_, i64>(0)).map_err(books_store::StoreError::from)? == 0,
    }))
}

/// `books_app_account_match`: the owner's kind for one account.
pub fn account_match(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let book = writes::book_in(conn, args)?;
    let a = writes::account_in(conn, &book, required(args, "account")?)?;
    let kind =
        KindRef::parse(required(args, "kind")?).ok_or_else(|| Refusal::invalid("No such kind."))?;
    store_imports::confirm(conn, &book, a.id, &kind)?;
    Ok(with_subject(
        json!({ "kind": kind.as_str(), "confirmed": !matches!(kind, KindRef::Unsure) }),
        "account",
        a.id,
    ))
}

/// `books_app_accounts_confirm`: confirm every suggestion Map 1 is sure of.
pub fn confirm_sure(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let book = writes::book_in(conn, args)?;
    let min = args
        .get("min")
        .and_then(Value::as_i64)
        .unwrap_or(90)
        .clamp(50, 100);
    let n = store_imports::confirm_sure(conn, &book, min)?;
    Ok(with_subject(json!({ "confirmed": n }), "book", book.id))
}

fn date_arg(args: &Map<String, Value>, key: &str) -> Result<Date, Refusal> {
    Date::parse(required(args, key)?)
        .ok_or_else(|| Refusal::invalid(format!("`{key}` is YYYY-MM-DD.")))
}

/// `books_app_opening_set`: one opening total with its proof, or `delete`.
pub fn opening_set(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let book = writes::book_in(conn, args)?;
    let measure = required(args, "measure")?;
    let from = date_arg(args, "from")?;
    if args.get("delete") == Some(&Value::Bool(true)) {
        store_imports::delete_opening(conn, &book, measure, from)?;
        return Ok(with_subject(json!({ "deleted": true }), "book", book.id));
    }
    let to = date_arg(args, "to")?;
    let centavos = money::parse(required(args, "amount")?)
        .map_err(|e| Refusal::invalid(format!("`amount`: {e}")))?;
    let o = Opening {
        measure: measure.to_string(),
        from,
        to,
        centavos,
        proof: text(args, "proof").unwrap_or("").to_string(),
    };
    store_imports::set_opening(conn, &book, &o)?;
    Ok(with_subject(
        json!({ "measure": o.measure, "range_label": range_label(from, to), "amount_shown": money::display(centavos) }),
        "book",
        book.id,
    ))
}

/// `books_app_return_ok`: the owner's OK of a return event a form waits on.
pub fn return_ok(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let book = writes::book_in(conn, args)?;
    let id = writes::entry_in(conn, &book, required(args, "entry")?)?.id;
    store_imports::acknowledge(conn, &book, id)?;
    Ok(with_subject(json!({ "acknowledged": true }), "entry", id))
}
