//! A report as an Excel workbook (`books_app_report_xlsx`): the P&L, the
//! balance sheet or the cash flow, with the rows and columns the CSV export
//! has (`reports::report_csv`), numbers as numbers and a money format. The
//! person's press hands the file to the Bot with `_meta["buwiz/save"]`, and
//! the Bot copies it to Downloads.
//!
//! Every amount is the report's own two-place text, read once as a number
//! for the cell: nothing is added, rounded or recomputed here, and a total
//! is the total the report computed (a value, never a formula). A cell
//! holds a double, so an amount of more than 15 digits is refused rather
//! than stored inexactly.

use std::io::Cursor;
use std::path::{Path, PathBuf};

use books_store::Store;
use books_store::books::Book;
use office_oxide::xlsx::write::{CellData, CellStyle, HAlign, XlsxWriter};
use serde_json::{Map, Value, json};

use crate::reports;
use crate::service::Refusal;

/// The money format of an amount cell: thousands separators, two places, a
/// plain minus.
pub const MONEY_FORMAT: &str = "#,##0.00;-#,##0.00";

/// The media type of the file.
pub const MEDIA_TYPE: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";

/// How long a handed-over file stays in its folder before the next export
/// sweeps it. The Bot has copied it long before.
const KEEP_SECS: u64 = 60 * 60;

/// One cell of a report table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cell {
    Text(String),
    /// Plain two-place decimal text, such as `-1234.50`.
    Amount(String),
    Blank,
}

/// A report as rows of cells: what the workbook shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// The sheet's name and the title on its first row.
    pub title: &'static str,
    /// The lines under the title: the period, the basis, a filter.
    pub notes: Vec<String>,
    /// `Section, Number, Account` and one amount column per period.
    pub header: Vec<String>,
    pub rows: Vec<Row>,
}

/// What a row is: the CSV has the items and the net line; a section's total
/// is the workbook's own addition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Item,
    SectionTotal,
    Net,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub cells: Vec<Cell>,
    pub role: Role,
}

fn title_of(kind: &str) -> Option<&'static str> {
    match kind {
        "pl" => Some("Profit and loss"),
        "bs" => Some("Balance sheet"),
        "cf" => Some("Cash flow"),
        _ => None,
    }
}

fn s(v: &Value) -> String {
    v.as_str().unwrap_or("").to_string()
}

fn amount(v: &Value) -> Cell {
    match reports::plain_amount(v) {
        t if t.is_empty() => Cell::Blank,
        t => Cell::Amount(t),
    }
}

/// The report `r` (`reports::report`'s answer) as a table: the CSV's rows,
/// with each section's total from the report beside them.
pub fn table(r: &Value) -> Result<Table, Refusal> {
    let title = r["kind"]
        .as_str()
        .and_then(title_of)
        .ok_or_else(|| Refusal::invalid("`kind` is pl, bs or cf for an Excel export."))?;
    let columns: Vec<String> = r["columns"]
        .as_array()
        .into_iter()
        .flatten()
        .map(s)
        .collect();
    // the CSV's own rule: a second amount column only with a compare period
    let two = !r["compare"].is_null() && columns.len() > 1;
    let mut header = vec!["Section".into(), "Number".into(), "Account".into()];
    header.push(columns.first().cloned().unwrap_or_default());
    if two {
        header.push(columns[1].clone());
    }
    let line = |section: &str, number: &str, name: &str, v: &Value, role: Role| {
        let mut cells = vec![
            Cell::Text(section.into()),
            Cell::Text(number.into()),
            Cell::Text(name.into()),
            amount(&v["shown"]),
        ];
        if two {
            cells.push(amount(&v["prev_shown"]));
        }
        Row { cells, role }
    };
    let mut rows = Vec::new();
    for sec in r["sections"].as_array().into_iter().flatten() {
        let section = s(&sec["title"]);
        for row in sec["rows"].as_array().into_iter().flatten() {
            rows.push(line(
                &section,
                &s(&row["number"]),
                &s(&row["name"]),
                row,
                Role::Item,
            ));
        }
        // the balance sheet's section total; the P&L's are in `totals`, and
        // the one that belongs to a section is labelled "Total <title>"
        let total = if sec["total"].is_object() {
            Some(&sec["total"])
        } else {
            let want = format!("total {}", section.to_lowercase());
            r["totals"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|t| s(&t["label"]).to_lowercase() == want)
        };
        if let Some(t) = total {
            rows.push(line("", "", &s(&t["label"]), t, Role::SectionTotal));
        }
    }
    let net = &r["net"];
    rows.push(line("", "", &s(&net["label"]), net, Role::Net));

    let mut notes = vec![
        format!("Period: {}", s(&r["period"]["label"])),
        s(&r["basis_label"]),
    ];
    if let Some(kind) = r["tag"]["kind"].as_str() {
        notes.push(format!("Only the {kind}: {}", s(&r["tag"]["name"])));
    }
    if let Some(note) = r["tag_note"].as_str() {
        notes.push(note.to_string());
    }
    Ok(Table {
        title,
        notes,
        header,
        rows,
    })
}

/// The workbook's bytes: one sheet named for the report.
pub fn workbook(t: &Table) -> Result<Vec<u8>, Refusal> {
    let bold = CellStyle::new().bold();
    let head = CellStyle::new().bold().background("F2F2F2");
    let head_num = CellStyle::new()
        .bold()
        .background("F2F2F2")
        .align(HAlign::Right);
    let money = CellStyle::new().number_format_code(MONEY_FORMAT);
    let money_bold = CellStyle::new().bold().number_format_code(MONEY_FORMAT);
    let mut w = XlsxWriter::new();
    {
        let mut sheet = w.add_sheet(t.title);
        let mut at = 0;
        sheet.set_cell_styled(
            at,
            0,
            CellData::String(t.title.into()),
            CellStyle::new().bold().font_size(14.0),
        );
        for note in t.notes.iter().filter(|n| !n.is_empty()) {
            at += 1;
            sheet.set_cell(at, 0, CellData::String(note.clone()));
        }
        at += 2;
        for (c, h) in t.header.iter().enumerate() {
            let style = if c < 3 {
                head.clone()
            } else {
                head_num.clone()
            };
            sheet.set_cell_styled(at, c, CellData::String(h.clone()), style);
        }
        for row in &t.rows {
            at += 1;
            for (c, cell) in row.cells.iter().enumerate() {
                match cell {
                    Cell::Text(x) if x.is_empty() => {}
                    Cell::Text(x) if (row.role != Role::Item) => {
                        sheet.set_cell_styled(at, c, CellData::String(x.clone()), bold.clone());
                    }
                    Cell::Text(x) => {
                        sheet.set_cell(at, c, CellData::String(x.clone()));
                    }
                    Cell::Amount(x) => {
                        let style = if row.role != Role::Item {
                            &money_bold
                        } else {
                            &money
                        };
                        sheet.set_cell_styled(
                            at,
                            c,
                            CellData::Number(crate::cell_amount::number(x)?),
                            style.clone(),
                        );
                    }
                    Cell::Blank => {}
                }
            }
        }
        sheet.set_column_width(0, 18.0);
        sheet.set_column_width(1, 10.0);
        sheet.set_column_width(2, 44.0);
        for c in 3..t.header.len() {
            sheet.set_column_width(c, 18.0);
        }
    }
    let mut out = Cursor::new(Vec::new());
    w.write_to(&mut out)
        .map_err(|_| Refusal::new("internal", "The workbook could not be made."))?;
    Ok(out.into_inner())
}

/// Where a file is handed to the Bot: `open-<hex>/1/<name>` in the plugin's
/// data folder, the only place the Bot takes one from. Older hand-offs there
/// are swept first. Returns the path.
pub fn hand_over(dir: &Path, name: &str, bytes: &[u8]) -> Result<PathBuf, Refusal> {
    use std::io::Write;
    let failed = || Refusal::new("io", "The file could not be saved.");
    sweep(dir);
    let mut raw = [0u8; 8];
    getrandom::fill(&mut raw).map_err(|_| failed())?;
    let hex: String = raw.iter().map(|b| format!("{b:02x}")).collect();
    let folder = dir.join(format!("open-{hex}")).join("1");
    books_store::private_dirs(&folder).map_err(|_| failed())?;
    let path = folder.join(name);
    let mut file = books_store::private_file(&path).map_err(|_| failed())?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| failed())?;
    Ok(path)
}

/// Removes the `open-<hex>` folders in `dir` that are over [`KEEP_SECS`]
/// old. A failure leaves them for the next sweep.
fn sweep(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let name = e.file_name();
        let Some(hex) = name.to_str().and_then(|n| n.strip_prefix("open-")) else {
            continue;
        };
        if hex.is_empty() || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            continue;
        }
        let old = e
            .metadata()
            .ok()
            .filter(|m| m.is_dir())
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age.as_secs() > KEEP_SECS);
        if old {
            let _ = std::fs::remove_dir_all(e.path());
        }
    }
}

/// A report made into a workbook, not yet handed over.
pub struct Prepared {
    pub name: String,
    bytes: Vec<u8>,
    rows: usize,
}

/// `books_app_report_xlsx`, first half: the report as a workbook's bytes.
pub fn prepare(store: &Store, book: Book, args: &Map<String, Value>) -> Result<Prepared, Refusal> {
    let r = reports::report(store, book, args)?;
    let t = table(&r)?;
    Ok(Prepared {
        bytes: workbook(&t)?,
        name: reports::export_name(&r, args, "xlsx"),
        // as the CSV tool counts: the header line and the CSV's own rows
        rows: 1 + t
            .rows
            .iter()
            .filter(|r| r.role != Role::SectionTotal)
            .count(),
    })
}

/// Second half: the file in `dir`. Returns the answer to store and show
/// (the file's name and row count) and what `buwiz/save` carries.
pub fn hand_over_prepared(dir: &Path, p: &Prepared) -> Result<(Value, Value), Refusal> {
    let path = hand_over(dir, &p.name, &p.bytes)?;
    Ok((
        json!({ "file_name": p.name, "rows": p.rows }),
        json!({ "path": path.to_string_lossy(), "name": p.name, "mime": MEDIA_TYPE }),
    ))
}
