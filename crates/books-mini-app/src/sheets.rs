//! Spreadsheets brought in where Books reads CSV: a chart of accounts, a
//! journal or a bank statement picked as an `.xlsx` (or a legacy `.xls`)
//! is read as the CSV Excel's "Save As CSV" would write, its cells as they
//! show (dates as dates, amounts with their format), then goes through the
//! same CSV readers. Values only: formulas count by the result the file
//! saved. Read by office_oxide; no number is computed here.

use crate::service::Refusal;

/// What reading a picked workbook may cost, set once for the process:
/// office_oxide's strict set (a part inflates to at most 64 MiB and 100
/// times its stored size, a package to 256 MiB, 128 MiB of working memory),
/// so a small file built to expand into gigabytes is refused, not read.
static LIMITS: std::sync::Once = std::sync::Once::new();

/// The CSV text of a picked workbook, or `None` when the bytes are not
/// one. The sheet read is the first whose name holds `key` (`chart`,
/// `journal`, `csv` for a statement), else the first sheet.
pub fn as_csv(bytes: &[u8], key: &str) -> Option<Result<String, Refusal>> {
    LIMITS.call_once(|| office_oxide::limits::Limits::strict().apply());
    let bad = || Refusal::new("bad_file", "That workbook could not be read.");
    if bytes.starts_with(b"PK\x03\x04") {
        let doc = match office_oxide::xlsx::XlsxDocument::from_reader(std::io::Cursor::new(bytes)) {
            Ok(d) => d,
            Err(_) => return Some(Err(bad())),
        };
        let names: Vec<&str> = doc.worksheets.iter().map(|w| w.name.as_str()).collect();
        let at = pick(&names, key);
        return Some(
            doc.sheet_to_csv(at)
                .ok_or_else(|| Refusal::invalid("That workbook has no sheets.")),
        );
    }
    if bytes.starts_with(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]) {
        let doc = match office_oxide::xls::XlsDocument::from_reader(std::io::Cursor::new(bytes)) {
            Ok(d) => d,
            Err(_) => return Some(Err(bad())),
        };
        let names: Vec<&str> = doc.sheets.iter().map(|s| s.name.as_str()).collect();
        let Some(sheet) = doc.sheets.get(pick(&names, key)) else {
            return Some(Err(Refusal::invalid("That workbook has no sheets.")));
        };
        let lines: Vec<String> = sheet
            .rows
            .iter()
            .enumerate()
            .map(|(r, row)| {
                (0..row.len())
                    .map(|c| field(&sheet.display_text(r, c).unwrap_or_default()))
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .collect();
        return Some(Ok(lines.join("\r\n")));
    }
    None
}

/// The sheet named for `key`, else the first.
fn pick(names: &[&str], key: &str) -> usize {
    let key = if key == "csv" { "statement" } else { key };
    names
        .iter()
        .position(|n| n.to_lowercase().contains(key))
        .unwrap_or(0)
}

/// A CSV field, quoted when it holds a comma, a quote or a line break.
fn field(text: &str) -> String {
    if text.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use office_oxide::xlsx::write::{CellData, XlsxWriter};

    /// A workbook of `sheets`, each rows of cells; built in memory.
    fn workbook(sheets: &[(&str, Vec<Vec<CellData>>)]) -> Vec<u8> {
        let mut w = XlsxWriter::new();
        for (name, rows) in sheets {
            let mut s = w.add_sheet(name);
            for row in rows {
                s.add_row(row.clone());
            }
        }
        let mut out = std::io::Cursor::new(Vec::new());
        w.write_to(&mut out).unwrap();
        out.into_inner()
    }

    fn t(s: &str) -> CellData {
        CellData::String(s.to_string())
    }

    #[test]
    fn a_workbook_reads_as_csv_from_the_named_sheet() {
        let bytes = workbook(&[
            ("Notes", vec![vec![t("not this one")]]),
            (
                "Chart of accounts",
                vec![
                    vec![t("Account"), t("Type")],
                    vec![t("Cash, petty"), t("Bank")],
                ],
            ),
        ]);
        let csv = as_csv(&bytes, "chart").unwrap().unwrap();
        assert_eq!(csv, "Account,Type\r\n\"Cash, petty\",Bank");
        // no sheet named for the key: the first
        let csv = as_csv(&bytes, "journal").unwrap().unwrap();
        assert_eq!(csv, "not this one");
    }

    #[test]
    fn text_is_not_a_workbook() {
        assert!(as_csv(b"Account,Type\n", "chart").is_none());
        let refusal = as_csv(b"PK\x03\x04broken", "chart").unwrap().unwrap_err();
        assert_eq!(refusal.message, "That workbook could not be read.");
    }

    #[test]
    fn a_workbook_is_read_under_strict_limits() {
        use office_oxide::limits::Limits;
        as_csv(b"PK\x03\x04broken", "chart");
        assert_eq!(Limits::current(), Limits::strict());
    }

    #[test]
    fn a_chart_and_a_journal_import_from_xlsx() {
        let chart = workbook(&[(
            "Chart",
            vec![
                vec![t("Account"), t("Type")],
                vec![t("Cash on hand"), t("Bank")],
                vec![t("Sales"), t("Income")],
            ],
        )]);
        let accounts =
            books_core::import::parse_csv_chart(&as_csv(&chart, "chart").unwrap().unwrap())
                .unwrap();
        assert_eq!(
            accounts.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(),
            ["Cash on hand", "Sales"]
        );
        let journal = workbook(&[(
            "Journal",
            vec![
                vec![t("Date"), t("Account"), t("Debit"), t("Credit")],
                vec![
                    t("2026-10-03"),
                    t("Cash on hand"),
                    t("1,250.50"),
                    CellData::Empty,
                ],
                vec![t("2026-10-03"), t("Sales"), CellData::Empty, t("1,250.50")],
            ],
        )]);
        let entries =
            books_core::import::parse_csv_journal(&as_csv(&journal, "journal").unwrap().unwrap())
                .unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].lines.len(), 2);
    }
}
