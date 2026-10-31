//! Bringing old books in (M5): a web Buwiz Books export, or CSV files (a
//! chart and a journal), read into one [`Plan`] the store then applies.
//! Pure: no store, no float. And Map 1: a suggested kind, with a
//! confidence, for each imported account; the owner confirms it.
//!
//! Books reads only the fields it keeps. Nothing that holds a TIN is read:
//! the web export's Philippine tax tables (`ph*`) are skipped whole, and
//! any run of nine or more digits in a name or memo is masked.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use crate::catalog::{AccountType, Catalog};
use crate::date::Date;
use crate::money;
use crate::statement::{mask_digit_runs, parse_amount, parse_date, record};

/// Where a plan came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// A web Buwiz Books export (`meta.version` 1 to 4).
    WebBooks { version: i64 },
    /// CSV files.
    Csv,
}

/// An account of the old chart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    /// The old number, when it had one (digits only, at most 12).
    pub number: Option<String>,
    pub name: String,
    pub account_type: AccountType,
    /// The old parent's name, for display only.
    pub parent: Option<String>,
    pub active: bool,
}

/// How a line names its account in the old books.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountRef {
    Number(String),
    Name(String),
}

/// A person of the old books: a name and a kind (`vendor`, `customer`,
/// `employee`, `owner`, `lender`, `government`). Never a TIN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Person {
    pub kind: &'static str,
    pub name: String,
}

/// A department or a location (`department`, `location`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub kind: &'static str,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub account: AccountRef,
    /// Centavos, one side only (the store's post rules check).
    pub debit: i64,
    pub credit: i64,
    pub description: Option<String>,
    pub party: Option<String>,
    pub department: Option<String>,
    pub location: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The old transaction or reference number.
    pub reference: Option<String>,
    pub date: Date,
    pub memo: Option<String>,
    /// The person the old books named on the transaction.
    pub party: Option<String>,
    /// Posted in the old books; anything else comes in as a draft.
    pub posted: bool,
    pub lines: Vec<Line>,
}

/// Something the plan leaves out on purpose, with why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    pub what: String,
    pub count: usize,
    pub why: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub source: Source,
    pub accounts: Vec<Account>,
    pub people: Vec<Person>,
    pub tags: Vec<Tag>,
    pub entries: Vec<Entry>,
    pub skipped: Vec<Skipped>,
}

/// What an imported account most likely is (plan 2.2, "Map 1").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    /// A kind id of `data/categories.v1.json`, `mixed` or `unsure`.
    pub kind: String,
    /// 0 to 100.
    pub confidence: u8,
}

/// The largest web export Books reads.
pub const MAX_EXPORT_BYTES: usize = 50 * 1024 * 1024;

/// The export versions Books reads (`meta.version`; a legacy file is 1).
pub const WEB_VERSIONS: std::ops::RangeInclusive<i64> = 1..=4;

/// Trimmed, every long digit run masked, `None` when nothing is left.
fn clean(text: &str) -> Option<String> {
    let masked = mask_digit_runs(text.trim());
    let t = masked.trim();
    (!t.is_empty()).then(|| t.to_string())
}

/// A text field of a JSON row, cleaned.
fn field(row: &Value, key: &str) -> Option<String> {
    row.get(key).and_then(Value::as_str).and_then(clean)
}

/// An old account number Books keeps: 1 to 12 digits.
fn account_number(text: &str) -> Option<String> {
    let t = text.trim();
    (!t.is_empty() && t.len() <= 12 && t.bytes().all(|b| b.is_ascii_digit())).then(|| t.to_string())
}

/// Lowercase words, separators made spaces, runs of spaces collapsed.
fn words(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars().flat_map(char::to_lowercase) {
        match c {
            '&' => out.push_str(" and "),
            '\'' | '\u{2019}' | '.' => {}
            c if c.is_alphanumeric() => out.push(c),
            _ => out.push(' '),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// An account type as web Books, QuickBooks, Xero or a plain sheet names it.
fn account_type_from_text(text: &str) -> Option<AccountType> {
    if let Some(t) = AccountType::parse(text.trim()) {
        return Some(t);
    }
    Some(match words(text).as_str() {
        "asset"
        | "assets"
        | "bank"
        | "cash"
        | "accounts receivable"
        | "accounts receivable a r"
        | "a r"
        | "receivable"
        | "other current asset"
        | "other current assets"
        | "current asset"
        | "current assets"
        | "fixed asset"
        | "fixed assets"
        | "fixed"
        | "other asset"
        | "other assets"
        | "non current asset"
        | "noncurrent asset"
        | "inventory"
        | "prepayment" => AccountType::Asset,
        "liability"
        | "liabilities"
        | "accounts payable"
        | "accounts payable a p"
        | "a p"
        | "payable"
        | "credit card"
        | "other current liability"
        | "other current liabilities"
        | "current liability"
        | "current liabilities"
        | "long term liability"
        | "long term liabilities"
        | "non current liability"
        | "term liability" => AccountType::Liability,
        "equity" | "owners equity" => AccountType::Equity,
        "revenue" | "income" | "sales" | "sales revenue" => AccountType::Revenue,
        "other income" | "other revenue" => AccountType::OtherIncome,
        "cost of goods sold" | "cost of goods" | "cost of sales" | "cost of revenue" | "cogs"
        | "direct costs" | "direct cost" => AccountType::CostOfRevenue,
        "expense" | "expenses" | "overhead" | "overheads" | "depreciation" => AccountType::Expense,
        "other expense" | "other expenses" => AccountType::OtherExpense,
        _ => return None,
    })
}

/// Whether `needle` starts a word of the normalized `text`.
fn has(text: &str, needle: &str) -> bool {
    format!(" {text}").contains(&format!(" {needle}"))
}

/// A type for a chart row that has none, read from its name.
fn infer_type(name: &str) -> Option<AccountType> {
    let n = words(name);
    let any = |needles: &[&str]| needles.iter().any(|w| has(&n, w));
    Some(
        if any(&["cost of goods", "cogs", "cost of sales", "cost of revenue"]) {
            AccountType::CostOfRevenue
        } else if any(&["payable", "liabilit", "loan", "credit card", "accrued"]) {
            AccountType::Liability
        } else if any(&["other income", "interest income"]) {
            AccountType::OtherIncome
        } else if any(&["income", "revenue", "sales"]) {
            AccountType::Revenue
        } else if any(&[
            "expense",
            "fee",
            "charge",
            "rent",
            "utilit",
            "salar",
            "wage",
            "supplies",
            "advertis",
            "insurance",
        ]) {
            AccountType::Expense
        } else if any(&[
            "receivable",
            "cash",
            "bank",
            "checking",
            "savings",
            "inventory",
            "prepaid",
            "equipment",
            "asset",
        ]) {
            AccountType::Asset
        } else if any(&["equity", "capital", "drawing", "retained", "owner"]) {
            AccountType::Equity
        } else {
            return None;
        },
    )
}

/// `1234.56000000` (up to eight places, as web Books stores `decimal(20,8)`)
/// to centavos, exactly. Places past the second must be zeros.
fn decimal_to_centavos(text: &str) -> Result<i64, &'static str> {
    const NOT: &str = "is not decimal text";
    let (negative, body) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (whole, frac) = match body.split_once('.') {
        Some((w, f)) if !f.is_empty() => (w, f),
        Some(_) => return Err(NOT),
        None => (body, ""),
    };
    if whole.is_empty()
        || frac.len() > 8
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !frac.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(NOT);
    }
    if frac.len() > 2 && !frac.bytes().skip(2).all(|b| b == b'0') {
        return Err("has digits past the centavo");
    }
    let cents: String = frac.chars().take(2).collect();
    let whole: i128 = whole.parse().map_err(|_| NOT)?;
    let cents: i128 = format!("{cents:0<2}").parse().map_err(|_| NOT)?;
    let magnitude = whole
        .checked_mul(100)
        .and_then(|w| w.checked_add(cents))
        .ok_or("is too large")?;
    i64::try_from(if negative { -magnitude } else { magnitude }).map_err(|_| "is too large")
}

/// A web amount (`debit` or `credit`): text, a whole number, or empty.
fn web_amount(value: Option<&Value>, what: &str) -> Result<i64, String> {
    let text = match value {
        None | Some(Value::Null) => return Ok(0),
        Some(Value::String(s)) => s.trim().to_string(),
        Some(Value::Number(n)) if n.is_i64() => n.to_string(),
        Some(_) => return Err(format!("{what}: the amount is not decimal text.")),
    };
    if text.is_empty() {
        return Ok(0);
    }
    let value = decimal_to_centavos(&text)
        .map_err(|why| format!("{what}: \"{text}\" {why}; Books keeps whole centavos."))?;
    if value < 0 {
        return Err(format!("{what}: \"{text}\" is negative."));
    }
    Ok(value)
}

fn rows<'a>(data: &'a Map<String, Value>, key: &str) -> &'a [Value] {
    data.get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

/// How many records a part of the export holds.
fn count(value: &Value) -> usize {
    match value {
        Value::Array(a) => a.len(),
        Value::Object(o) if o.is_empty() => 0,
        Value::Object(o) => match o.get("headers") {
            Some(Value::Array(h)) => h.len(),
            _ => 1,
        },
        Value::Null => 0,
        _ => 1,
    }
}

/// Why Books leaves a part of the web export out.
fn skip_why(key: &str) -> &'static str {
    match key {
        k if k.starts_with("ph") => "eBIRForms keeps tax profiles, TINs and returns",
        "bills" | "invoices" => "open bills and invoices: add them on the Unpaid page",
        "products" => "Books keeps no product list; sales come in through their accounts",
        "numberSequences" => "Books numbers its own entries",
        "orgSettings" => "Books keeps its own settings for the business",
        "banks" => "their ledger accounts come in as categories",
        _ => "Books does not read this part of the export",
    }
}

const WEB_PEOPLE: [(&str, &str); 6] = [
    ("vendors", "vendor"),
    ("customers", "customer"),
    ("employees", "employee"),
    ("shareholders", "owner"),
    ("lenders", "lender"),
    ("government", "government"),
];

const WEB_TAGS: [(&str, &str); 2] = [("departments", "department"), ("locations", "location")];

/// Read a web Buwiz Books export (`meta.version` 1 to 4) into a plan.
///
/// Only these fields are read: categories' `accountNumber`, `name`,
/// `accountType`, `parentName`, `isActive`; the `name` of departments,
/// locations and people; transaction headers' `transactionNumber`,
/// `referenceNumber`, `transactionDate`, `memo`, `partyName`, `status`; and
/// lines' `transactionNumber`, `accountNumber`, `accountName`, `debit`,
/// `credit`, `lineDescription`, `partyName`, `departmentName`,
/// `locationName`. Every other part is listed in [`Plan::skipped`].
pub fn parse_web_export(json: &str) -> Result<Plan, String> {
    if json.len() > MAX_EXPORT_BYTES {
        return Err(
            "The file is over 50 MB. Books reads a web Buwiz Books export of 50 MB at most.".into(),
        );
    }
    let root: Value = serde_json::from_str(json.trim_start_matches('\u{feff}')).map_err(
        |_| "This file is not JSON. Books reads the .json file web Buwiz Books exports.",
    )?;
    let not_export = "This is not a web Buwiz Books export: it has neither a meta block nor the old entities block.";
    let (version, data) = match root.get("meta") {
        Some(meta) if meta.is_object() => {
            let version = meta
                .get("version")
                .and_then(Value::as_i64)
                .ok_or("This export has no version number, so Books cannot read it.")?;
            (version, root.get("data"))
        }
        Some(_) => return Err(not_export.into()),
        None if root.get("exportedAt").is_some_and(Value::is_string)
            && root.get("entities").is_some_and(Value::is_object) =>
        {
            (1, root.get("entities"))
        }
        None => return Err(not_export.into()),
    };
    if !WEB_VERSIONS.contains(&version) {
        return Err(format!(
            "This export is version {version}. Books reads web Buwiz Books exports of versions 1 to 4."
        ));
    }
    let data = data
        .and_then(Value::as_object)
        .ok_or("This export has no data block.")?;

    let mut accounts = Vec::new();
    for (i, row) in rows(data, "categories").iter().enumerate() {
        let Some(name) = field(row, "name") else {
            continue;
        };
        let type_text = row.get("accountType").and_then(Value::as_str).unwrap_or("");
        let account_type = account_type_from_text(type_text).ok_or_else(|| {
            format!(
                "Category {} (\"{name}\"): the account type \"{}\" is not one Books knows.",
                i + 1,
                mask_digit_runs(type_text)
            )
        })?;
        accounts.push(Account {
            number: row
                .get("accountNumber")
                .and_then(Value::as_str)
                .and_then(account_number),
            name,
            account_type,
            parent: field(row, "parentName"),
            active: row.get("isActive").and_then(Value::as_bool).unwrap_or(true),
        });
    }

    let mut seen = BTreeSet::new();
    let mut people = Vec::new();
    for (key, kind) in WEB_PEOPLE {
        for row in rows(data, key) {
            if let Some(name) = field(row, "name")
                && seen.insert((kind, name.clone()))
            {
                people.push(Person { kind, name });
            }
        }
    }
    let mut tags = Vec::new();
    for (key, kind) in WEB_TAGS {
        for row in rows(data, key) {
            if let Some(name) = field(row, "name")
                && seen.insert((kind, name.clone()))
            {
                tags.push(Tag { kind, name });
            }
        }
    }

    let read: BTreeSet<&str> = ["categories", "transactions"]
        .into_iter()
        .chain(WEB_PEOPLE.iter().map(|(k, _)| *k))
        .chain(WEB_TAGS.iter().map(|(k, _)| *k))
        .collect();
    let mut skipped: Vec<Skipped> = data
        .iter()
        .filter(|(key, _)| !read.contains(key.as_str()))
        .map(|(key, value)| Skipped {
            what: key.clone(),
            count: count(value),
            why: skip_why(key).to_string(),
        })
        .filter(|s| s.count > 0)
        .collect();

    let entries = match data.get("transactions") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Object(tx)) => web_entries(tx, &mut skipped)?,
        Some(_) => {
            return Err(
                "The transactions part is not in the shape Books reads (headers and lines).".into(),
            );
        }
    };

    Ok(Plan {
        source: Source::WebBooks { version },
        accounts,
        people,
        tags,
        entries,
        skipped,
    })
}

/// Headers and lines joined by `transactionNumber`, oldest first.
fn web_entries(tx: &Map<String, Value>, skipped: &mut Vec<Skipped>) -> Result<Vec<Entry>, String> {
    let number_of = |v: &Value| {
        v.get("transactionNumber")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(str::to_string)
    };
    let headers = rows(tx, "headers");
    let mut orphan_lines = 0;
    let mut lines_by_number: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
    for line in rows(tx, "lines") {
        match number_of(line) {
            Some(n) => lines_by_number.entry(n).or_default().push(line),
            None => orphan_lines += 1,
        }
    }
    let mut uses: BTreeMap<String, usize> = BTreeMap::new();
    for h in headers {
        if let Some(n) = number_of(h) {
            *uses.entry(n).or_default() += 1;
        }
    }

    let (mut voided, mut unjoined) = (0, 0);
    let mut entries = Vec::new();
    for h in headers {
        let Some(number) = number_of(h).filter(|n| uses.get(n) == Some(&1)) else {
            unjoined += 1;
            continue;
        };
        let raw = lines_by_number.remove(&number).unwrap_or_default();
        let status = h.get("status").and_then(Value::as_str).unwrap_or("draft");
        if status == "voided" {
            voided += 1;
            continue;
        }
        let label = format!("Transaction {}", mask_digit_runs(&number));
        let date_text: String = h
            .get("transactionDate")
            .and_then(Value::as_str)
            .unwrap_or("")
            .chars()
            .take(10)
            .collect();
        let date = Date::parse(&date_text)
            .ok_or_else(|| format!("{label}: the date is not a YYYY-MM-DD date."))?;
        let mut lines = Vec::new();
        for (j, l) in raw.iter().enumerate() {
            let what = format!("{label}, line {}", j + 1);
            let debit = web_amount(l.get("debit"), &what)?;
            let credit = web_amount(l.get("credit"), &what)?;
            if debit == 0 && credit == 0 {
                continue;
            }
            let account = match l
                .get("accountNumber")
                .and_then(Value::as_str)
                .and_then(account_number)
            {
                Some(n) => AccountRef::Number(n),
                None => AccountRef::Name(
                    field(l, "accountName").ok_or_else(|| format!("{what} names no account."))?,
                ),
            };
            lines.push(Line {
                account,
                debit,
                credit,
                description: field(l, "lineDescription"),
                party: field(l, "partyName"),
                department: field(l, "departmentName"),
                location: field(l, "locationName"),
            });
        }
        if lines.is_empty() {
            unjoined += 1;
            continue;
        }
        let posted = status == "posted";
        if posted {
            check_balance(&lines, &label)?;
        }
        entries.push(Entry {
            reference: clean(&number).or_else(|| field(h, "referenceNumber")),
            date,
            memo: field(h, "memo"),
            party: field(h, "partyName"),
            posted,
            lines,
        });
    }
    orphan_lines += lines_by_number.values().map(Vec::len).sum::<usize>();
    entries.sort_by_key(|e| e.date);

    for (what, n, why) in [
        (
            "voided transactions",
            voided,
            "they were voided in the old books and never counted",
        ),
        (
            "transactions without lines",
            unjoined,
            "the export ties no lines to them (no transaction number, a shared one, or no amounts)",
        ),
        (
            "transaction lines without a transaction",
            orphan_lines,
            "their transaction number matches no transaction Books brings in",
        ),
    ] {
        if n > 0 {
            skipped.push(Skipped {
                what: what.into(),
                count: n,
                why: why.into(),
            });
        }
    }
    Ok(entries)
}

/// Debits equal credits, to the centavo.
fn check_balance(lines: &[Line], label: &str) -> Result<(), String> {
    let debits = money::sum(lines.iter().map(|l| l.debit));
    let credits = money::sum(lines.iter().map(|l| l.credit));
    match (debits, credits) {
        (Some(d), Some(c)) if d == c => Ok(()),
        (Some(d), Some(c)) => Err(format!(
            "{label}: debits {} and credits {} do not balance.",
            money::display(d),
            money::display(c)
        )),
        _ => Err(format!("{label}: the amounts are too large to add up.")),
    }
}

/// The CSV rows of a file, by file line (a byte-order mark dropped).
fn csv_rows(text: &str) -> Vec<Vec<String>> {
    text.trim_start_matches('\u{feff}')
        .lines()
        .map(record)
        .collect()
}

/// A header cell as compared: lowercase, Xero's `*` dropped, spaces collapsed.
fn header_key(cell: &str) -> String {
    cell.trim()
        .trim_start_matches('*')
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The first header cell among `names`, in their order of preference.
fn column(header: &[String], names: &[&str]) -> Option<usize> {
    let keys: Vec<String> = header.iter().map(|h| header_key(h)).collect();
    names.iter().find_map(|n| keys.iter().position(|k| k == n))
}

/// The header row: the first of the first 20 rows with every group of
/// names. Reports put a title and a date range above it.
fn header_row(rows: &[Vec<String>], groups: &[&[&str]]) -> Option<usize> {
    rows.iter()
        .take(20)
        .position(|r| groups.iter().all(|names| column(r, names).is_some()))
}

fn is_blank(row: &[String]) -> bool {
    row.iter().all(|c| c.trim().is_empty())
}

/// A report's total line: its first cell with text starts with "Total".
fn is_total(row: &[String]) -> bool {
    row.iter().find(|c| !c.trim().is_empty()).is_some_and(|c| {
        let c = c.trim().to_ascii_lowercase();
        c == "total" || c.starts_with("total ") || c.starts_with("total:")
    })
}

const CHART_NAME: &[&str] = &["account name", "full name", "account", "name"];
const CHART_NUMBER: &[&str] = &[
    "account number",
    "account no.",
    "account no",
    "account #",
    "account code",
    "number",
    "no.",
    "no",
    "code",
];
const CHART_TYPE: &[&str] = &["type", "account type"];
const CHART_DETAIL: &[&str] = &["detail type"];

/// Read a CSV chart of accounts: a header row with an account name column
/// and optional number and type columns (Books, QuickBooks or Xero types).
/// A row without a type takes one from its name, else it is an error.
pub fn parse_csv_chart(text: &str) -> Result<Vec<Account>, String> {
    let rows = csv_rows(text);
    let h = header_row(&rows, &[CHART_NAME]).ok_or(
        "The chart has no account name column (Account, Name, Account name or Full name).",
    )?;
    let header = &rows[h];
    let name_col = column(header, CHART_NAME);
    let number_col = column(header, CHART_NUMBER);
    let type_col = column(header, CHART_TYPE);
    let detail_col = column(header, CHART_DETAIL);
    let mut accounts = Vec::new();
    for (idx, row) in rows.iter().enumerate().skip(h + 1) {
        if is_blank(row) || is_total(row) {
            continue;
        }
        let n = idx + 1;
        let cell = |c: Option<usize>| c.and_then(|c| row.get(c)).map_or("", |s| s.trim());
        let Some(name) = clean(cell(name_col)) else {
            continue;
        };
        let type_text = match cell(type_col) {
            "" => cell(detail_col),
            t => t,
        };
        let account_type = if type_text.is_empty() {
            infer_type(&name).ok_or_else(|| {
                format!(
                    "Row {n} (\"{name}\"): it has no type and its name does not say one. Add a Type column."
                )
            })?
        } else {
            account_type_from_text(type_text).ok_or_else(|| {
                format!(
                    "Row {n} (\"{name}\"): the type \"{}\" is not one Books knows.",
                    mask_digit_runs(type_text)
                )
            })?
        };
        let parent = name.rsplit_once(':').and_then(|(p, _)| clean(p));
        accounts.push(Account {
            number: account_number(cell(number_col)),
            name,
            account_type,
            parent,
            active: true,
        });
    }
    if accounts.is_empty() {
        return Err("The chart has no accounts.".into());
    }
    Ok(accounts)
}

const J_DATE: &[&str] = &[
    "date",
    "transaction date",
    "txn date",
    "posting date",
    "entry date",
];
const J_ACCOUNT: &[&str] = &["account", "account name", "full name"];
const J_NUMBER: &[&str] = &[
    "account number",
    "account no.",
    "account no",
    "account #",
    "account code",
];
const J_ACCOUNT_ANY: &[&str] = &[
    "account",
    "account name",
    "full name",
    "account number",
    "account no.",
    "account no",
    "account #",
    "account code",
];
const J_DEBIT: &[&str] = &["debit", "debits", "debit amount", "dr"];
const J_CREDIT: &[&str] = &["credit", "credits", "credit amount", "cr"];
const J_AMOUNT: &[&str] = &["amount", "signed amount", "net amount"];
const J_REF: &[&str] = &[
    "num",
    "no.",
    "no",
    "reference",
    "ref",
    "ref no.",
    "ref no",
    "transaction",
    "transaction no.",
    "transaction number",
    "entry no.",
    "journal no.",
];
const J_PARTY: &[&str] = &["name", "payee", "party"];
const J_MEMO: &[&str] = &[
    "memo/description",
    "memo",
    "description",
    "line description",
];
const J_DEPARTMENT: &[&str] = &["department", "class"];
const J_LOCATION: &[&str] = &["location"];

struct JRow {
    n: usize,
    date: Date,
    reference: Option<String>,
    party: Option<String>,
    line: Line,
}

/// Read a CSV journal, one row per line: Date, Account (name) or Account
/// number, Debit and Credit or one signed Amount (positive is a debit), and
/// optionally a reference, a name, a memo, a department and a location.
/// Rows group into an entry by (date, reference) when there is a reference,
/// else consecutive rows until debits equal credits. A QuickBooks Journal
/// report works as exported: continuation rows with no date take the
/// transaction's date and number, and total rows are passed over. Every
/// entry comes in posted.
pub fn parse_csv_journal(text: &str) -> Result<Vec<Entry>, String> {
    let rows = csv_rows(text);
    let h = header_row(&rows, &[J_DATE, J_ACCOUNT_ANY])
        .ok_or("The journal needs a Date column and an Account or Account number column.")?;
    let header = &rows[h];
    let date_col = column(header, J_DATE);
    let account_col = column(header, J_ACCOUNT);
    let number_col = column(header, J_NUMBER);
    let amount_col = column(header, J_AMOUNT);
    let debit_col = column(header, J_DEBIT);
    let credit_col = column(header, J_CREDIT);
    let ref_col = column(header, J_REF);
    let party_col = column(header, J_PARTY);
    let memo_col = column(header, J_MEMO);
    let dept_col = column(header, J_DEPARTMENT);
    let loc_col = column(header, J_LOCATION);
    if amount_col.is_none() && debit_col.is_none() && credit_col.is_none() {
        return Err("The journal needs Debit and Credit columns, or one Amount column.".into());
    }

    let mut parsed = Vec::new();
    let mut last: Option<(Date, Option<String>, Option<String>)> = None;
    for (idx, row) in rows.iter().enumerate().skip(h + 1) {
        if is_blank(row) || is_total(row) {
            continue;
        }
        let n = idx + 1;
        let cell = |c: Option<usize>| c.and_then(|c| row.get(c)).map_or("", |s| s.trim());
        let money = |c: Option<usize>, what: &str| -> Result<i64, String> {
            match cell(c) {
                "" => Ok(0),
                t => parse_amount(t).ok_or_else(|| {
                    format!(
                        "Row {n}: the {what} \"{}\" is not an amount.",
                        mask_digit_runs(t)
                    )
                }),
            }
        };
        let net = match amount_col {
            Some(_) if debit_col.is_none() && credit_col.is_none() => money(amount_col, "amount")?,
            _ => {
                let d = money(debit_col, "debit")?;
                let c = money(credit_col, "credit")?;
                d.checked_sub(c)
                    .ok_or_else(|| format!("Row {n}: the amounts are too large."))?
            }
        };
        let date_text = cell(date_col);
        let account_text = cell(account_col);
        let number = account_number(cell(number_col));
        if date_text.is_empty() && account_text.is_empty() && number.is_none() {
            // a report subtotal
            continue;
        }
        if net == 0 {
            continue;
        }
        let (date, reference, party) = if date_text.is_empty() {
            let (d, r, p) = last
                .clone()
                .ok_or_else(|| format!("Row {n}: it has no date."))?;
            (d, r, clean(cell(party_col)).or(p))
        } else {
            let d = parse_date(date_text).ok_or_else(|| {
                format!(
                    "Row {n}: the date \"{}\" is not a date.",
                    mask_digit_runs(date_text)
                )
            })?;
            let r = clean(cell(ref_col));
            let p = clean(cell(party_col));
            last = Some((d, r.clone(), p.clone()));
            (d, r, p)
        };
        let account = match number {
            Some(num) => AccountRef::Number(num),
            None => AccountRef::Name(
                clean(account_text).ok_or_else(|| format!("Row {n}: it names no account."))?,
            ),
        };
        parsed.push(JRow {
            n,
            date,
            reference,
            party: party.clone(),
            line: Line {
                account,
                debit: net.max(0),
                credit: net.checked_neg().unwrap_or(0).max(0),
                description: clean(cell(memo_col)),
                party,
                department: clean(cell(dept_col)),
                location: clean(cell(loc_col)),
            },
        });
    }

    // Groups, in the order their first row appears.
    let mut groups: Vec<Vec<JRow>> = Vec::new();
    let mut keyed: BTreeMap<(Date, String), usize> = BTreeMap::new();
    let mut open: Option<usize> = None;
    for row in parsed {
        if let Some(r) = row.reference.clone() {
            let next = groups.len();
            let g = *keyed.entry((row.date, r)).or_insert(next);
            if g == next {
                groups.push(Vec::new());
            }
            groups[g].push(row);
            continue;
        }
        let g = match open {
            Some(g) if groups[g][0].date == row.date => g,
            Some(g) => {
                return Err(format!(
                    "Row {}: the entry that starts on row {} does not balance before the date changes.",
                    row.n, groups[g][0].n
                ));
            }
            None => {
                groups.push(Vec::new());
                groups.len() - 1
            }
        };
        groups[g].push(row);
        let lines: Vec<Line> = groups[g].iter().map(|r| r.line.clone()).collect();
        open = check_balance(&lines, "").is_err().then_some(g);
    }

    let mut entries = Vec::new();
    for group in groups {
        let first = group[0].n;
        let last_row = group[group.len() - 1].n;
        let label = if first == last_row {
            format!("Row {first}")
        } else {
            format!("Rows {first} to {last_row}")
        };
        let lines: Vec<Line> = group.iter().map(|r| r.line.clone()).collect();
        check_balance(&lines, &label)?;
        entries.push(Entry {
            reference: group[0].reference.clone(),
            date: group[0].date,
            memo: lines.iter().find_map(|l| l.description.clone()),
            party: group.iter().find_map(|r| r.party.clone()),
            posted: true,
            lines,
        });
    }
    if entries.is_empty() {
        return Err("The journal has no rows with an amount.".into());
    }
    Ok(entries)
}

/// Map 1 keyword rules, most specific first: (words that start a word of
/// the name, kind, confidence). A rule applies only when the kind's type
/// fits the account's.
const KEYWORD_RULES: &[(&[&str], &str, u8)] = &[
    (
        &["interest income", "interest earned"],
        "interest_income",
        84,
    ),
    (
        &["interest expense", "interest paid", "finance charge"],
        "interest_expense",
        84,
    ),
    (&["interest"], "interest_income", 70),
    (&["interest"], "interest_expense", 70),
    (
        &["bank fee", "bank charge", "bank service", "service charge"],
        "bank_fees",
        82,
    ),
    (
        &["cost of goods", "cogs", "cost of sales"],
        "cost_of_goods_sold",
        82,
    ),
    (
        &["direct cost", "cost of service"],
        "direct_service_cost",
        72,
    ),
    (
        &["refund", "return", "discount", "allowance"],
        "refunds_and_discounts",
        82,
    ),
    (
        &["other income", "miscellaneous income"],
        "other_non_operating_income",
        70,
    ),
    (
        &["subscription revenue", "subscription income"],
        "subscription_revenue",
        80,
    ),
    (&["subscription"], "subscription_revenue", 70),
    (
        &["sales", "revenue", "service", "income", "fees earned"],
        "sales_revenue",
        82,
    ),
    (
        &["payment processing", "merchant fee", "card fee"],
        "payment_processing_fees",
        76,
    ),
    (
        &["payroll processing", "payroll fee", "payroll service"],
        "payroll_processing_fees",
        76,
    ),
    (&["rent"], "rent", 80),
    (
        &[
            "utilit",
            "electric",
            "water",
            "internet",
            "power",
            "telephone",
            "phone",
        ],
        "utilities",
        80,
    ),
    (&["salar", "wage"], "salaries", 80),
    (
        &["employee benefit", "benefits", "health insurance", "hmo"],
        "employee_benefits",
        72,
    ),
    (&["retirement", "pension"], "retirement_contributions", 72),
    (
        &["software", "subscription", "saas", "apps"],
        "business_applications_and_software",
        78,
    ),
    (&["hosting", "cloud"], "hosting_fees", 74),
    (&["legal", "attorney", "lawyer"], "legal", 76),
    (
        &["accounting", "bookkeeping", "tax and accounting", "audit"],
        "tax_and_accounting",
        76,
    ),
    (
        &["consult", "contractor", "professional", "outside service"],
        "consultants_and_contractors",
        74,
    ),
    (&["office supplies", "supplies"], "office_supplies", 78),
    (&["advertis", "marketing", "promotion"], "advertising", 80),
    (&["repair", "maintenance"], "repairs_and_maintenance", 78),
    (&["airfare", "flight", "travel"], "airfare", 66),
    (&["meal", "food", "entertainment"], "meals", 66),
    (&["depreciation"], "depreciation_expense", 80),
    (&["receivable"], "accounts_receivable_gross", 80),
    (&["inventory", "merchandise", "stock"], "inventory", 78),
    (&["prepaid", "prepayment"], "prepaid_expenses", 78),
    (&["computer", "equipment", "laptop"], "computer_assets", 64),
    (
        &[
            "loan",
            "note payable",
            "notes payable",
            "mortgage",
            "credit card",
        ],
        "loan_payable",
        76,
    ),
    (&["payable"], "accounts_payable", 78),
    (
        &["cash", "bank", "checking", "savings", "petty"],
        "bank_accounts",
        80,
    ),
    (&["drawing", "withdrawal", "draw"], "owner_drawings", 80),
    (&["capital", "equity", "owner"], "owners_equity", 76),
    (&["retained"], "owners_equity", 66),
];

/// Names that hold more than one kind of thing: each line picks its own.
const MIXED: &[&str] = &[
    "payroll liabilit",
    "payroll payable",
    "payroll tax",
    "taxes and licenses",
    "taxes payable",
    "withholding taxes",
    "accrued liabilit",
    "accrued expense",
    "government contribution",
    "statutory",
    "opening balance",
];

/// Names that say nothing about what the money was.
const UNSURE: &[&str] = &[
    "ask my accountant",
    "uncategori",
    "suspense",
    "miscellaneous",
    "misc",
];

/// The account name as compared: a QuickBooks `Parent:` prefix dropped,
/// lowercase, `&` as "and", punctuation gone, spaces collapsed.
fn normalize_name(name: &str) -> String {
    words(name.rsplit(':').next().unwrap_or(name))
}

fn compatible(a: AccountType, b: AccountType) -> bool {
    use AccountType::*;
    a == b
        || matches!((a, b), (Revenue | OtherIncome, Revenue | OtherIncome))
        || matches!(
            (a, b),
            (
                CostOfRevenue | Expense | OtherExpense,
                CostOfRevenue | Expense | OtherExpense
            )
        )
}

/// Map 1: the kind an imported account most likely is, with a confidence.
/// Only kinds of a fitting type are suggested (revenue and other income fit
/// each other, as do cost of revenue, expense and other expense).
///
/// An exact number and name is 96, the number alone 90, the name alone 88;
/// keyword rules give 60 to 84; a name that mixes things (Payroll
/// Liabilities) is `mixed` at 64; anything else is `unsure` at 30.
pub fn suggest_kind(number: Option<&str>, name: &str, account_type: AccountType) -> Suggestion {
    let suggest = |kind: &str, confidence: u8| Suggestion {
        kind: kind.to_string(),
        confidence,
    };
    // "41000 Sales Revenue": a number written into the name.
    let (number, name) = match (number.and_then(account_number), name.trim().split_once(' ')) {
        (Some(n), _) => (Some(n), name.trim()),
        (None, Some((head, rest))) if account_number(head).is_some() => {
            (account_number(head), rest.trim())
        }
        (None, _) => (None, name.trim()),
    };
    let norm = normalize_name(name);
    let catalog = Catalog::get();
    let kinds = || {
        catalog
            .kinds
            .iter()
            .filter(|k| compatible(k.account_type, account_type))
    };
    let same_number = |k: &&crate::catalog::Kind| number.is_some() && k.number == number;
    let same_name = |k: &&crate::catalog::Kind| !norm.is_empty() && normalize_name(&k.name) == norm;

    if let Some(k) = kinds().find(|k| same_number(k) && same_name(k)) {
        return suggest(&k.id, 96);
    }
    if let Some(k) = kinds().find(same_number) {
        return suggest(&k.id, 90);
    }
    if let Some(k) = kinds().find(same_name) {
        return suggest(&k.id, 88);
    }
    if norm == "other" || norm == "others" || UNSURE.iter().any(|w| has(&norm, w)) {
        return suggest("unsure", 30);
    }
    if MIXED.iter().any(|w| has(&norm, w)) {
        return suggest("mixed", 64);
    }
    for (needles, kind, confidence) in KEYWORD_RULES {
        if needles.iter().any(|w| has(&norm, w))
            && catalog
                .kind(kind)
                .is_some_and(|k| compatible(k.account_type, account_type))
        {
            return suggest(kind, *confidence);
        }
    }
    suggest("unsure", 30)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn web(data: &str) -> Result<Plan, String> {
        parse_web_export(&format!(
            r#"{{"meta":{{"version":4,"exportedAt":"2026-10-28T00:00:00Z","organizationName":"Demo"}},"data":{data}}}"#
        ))
    }

    fn one_entry(debit: &str, credit: &str) -> String {
        format!(
            r#"{{"transactions":{{"headers":[{{"transactionNumber":"JE-1","transactionDate":"2026-07-01","status":"posted"}}],
            "lines":[{{"transactionNumber":"JE-1","accountNumber":"11000","debit":"{debit}","credit":"0.00000000"}},
                     {{"transactionNumber":"JE-1","accountNumber":"41000","debit":"0.00000000","credit":"{credit}"}}]}}}}"#
        )
    }

    #[test]
    fn eight_place_decimals_are_exact_centavos() {
        assert_eq!(decimal_to_centavos("1234.56000000"), Ok(123_456));
        assert_eq!(decimal_to_centavos("0.00000000"), Ok(0));
        assert_eq!(decimal_to_centavos("120000.00000000"), Ok(12_000_000));
        assert_eq!(decimal_to_centavos("7"), Ok(700));
        assert_eq!(decimal_to_centavos("7.5"), Ok(750));
        assert!(decimal_to_centavos("1.005").is_err());
        assert!(decimal_to_centavos("1.00000001").is_err());
        for bad in ["", ".5", "1.", "1,000.00", "1e3", "1.000000000", "abc"] {
            assert!(decimal_to_centavos(bad).is_err(), "{bad}");
        }
        let plan = web(&one_entry("1234.56000000", "1234.56000000")).unwrap();
        assert_eq!(plan.entries[0].lines[0].debit, 123_456);
        assert_eq!(plan.entries[0].lines[1].credit, 123_456);
        let err = web(&one_entry("1.005", "1.005")).unwrap_err();
        assert!(err.contains("Transaction JE-1, line 1"), "{err}");
        assert!(err.contains("past the centavo"), "{err}");
        let err = web(&one_entry("10.00", "9.00")).unwrap_err();
        assert!(err.contains("do not balance"), "{err}");
    }

    #[test]
    fn versions_size_and_shape_are_checked() {
        let legacy = r#"{"exportedAt":"2026-01-01T00:00:00Z","entities":{"customers":[{"name":" Bayani Foods "},{"name":""}]}}"#;
        let plan = parse_web_export(legacy).unwrap();
        assert_eq!(plan.source, Source::WebBooks { version: 1 });
        assert_eq!(
            plan.people,
            vec![Person {
                kind: "customer",
                name: "Bayani Foods".into()
            }]
        );
        for v in [0, 5] {
            let err = parse_web_export(&format!(r#"{{"meta":{{"version":{v}}},"data":{{}}}}"#))
                .unwrap_err();
            assert!(err.contains("versions 1 to 4"), "{err}");
        }
        assert!(parse_web_export("{}").unwrap_err().contains("not a web"));
        assert!(parse_web_export("nope").unwrap_err().contains("not JSON"));
        let big = " ".repeat(MAX_EXPORT_BYTES + 1);
        assert!(parse_web_export(&big).unwrap_err().contains("50 MB"));
    }

    #[test]
    fn web_types_people_and_masking() {
        let run = ["1234", "5678", "9"].concat();
        let data = format!(
            r#"{{"categories":[{{"accountNumber":"51000","name":"Cost of Goods Sold","accountType":"cost_of_revenue","isActive":false}},
                {{"accountNumber":"A-1","name":"Petty cash {run}","accountType":"asset","parentName":"Cash"}}ODD],
              "shareholders":[{{"name":"Ana Reyes","taxId":"canary-value"}}],
              "departments":[{{"name":"Store"}}],"locations":[{{"name":"Cebu"}}]}}"#
        );
        let err =
            web(&data.replace("ODD", r#",{"name":"Odd","accountType":"weird"}"#)).unwrap_err();
        assert!(err.contains("Category 3") && err.contains("weird"), "{err}");
        let plan = web(&data.replace("ODD", "")).unwrap();
        assert_eq!(plan.accounts[0].account_type, AccountType::CostOfRevenue);
        assert!(!plan.accounts[0].active);
        assert_eq!(plan.accounts[1].number, None, "not 1-12 digits");
        assert_eq!(plan.accounts[1].name, "Petty cash •••");
        assert_eq!(plan.people[0].kind, "owner");
        assert_eq!(plan.tags.len(), 2);
        assert!(!format!("{plan:?}").contains("canary"));
    }

    #[test]
    fn web_drafts_voids_and_unjoined_lines() {
        let data = r#"{"transactions":{"headers":[
            {"transactionNumber":"JE-1","transactionDate":"2026-07-02T00:00:00","status":"draft","memo":"later"},
            {"transactionNumber":"JE-2","transactionDate":"2026-07-01","status":"voided"},
            {"transactionNumber":null,"transactionDate":"2026-07-01","status":"posted"}],
          "lines":[
            {"transactionNumber":"JE-1","accountName":"Sales","debit":"0","credit":"5.00"},
            {"transactionNumber":"JE-2","accountNumber":"11000","debit":"5.00","credit":"0"},
            {"transactionNumber":"JE-9","accountNumber":"11000","debit":"5.00","credit":"0"}]},
          "bills":[{"billNumber":"B-1"}],"phOrgTaxProfile":[{"registeredName":"Demo"}],"products":[]}"#;
        let plan = web(data).unwrap();
        assert_eq!(plan.entries.len(), 1);
        let e = &plan.entries[0];
        assert!(!e.posted, "a draft may be unbalanced and stays a draft");
        assert_eq!(e.date, Date::parse("2026-07-02").unwrap());
        assert_eq!(e.lines[0].account, AccountRef::Name("Sales".into()));
        let what: Vec<&str> = plan.skipped.iter().map(|s| s.what.as_str()).collect();
        assert_eq!(
            what,
            [
                "bills",
                "phOrgTaxProfile",
                "voided transactions",
                "transactions without lines",
                "transaction lines without a transaction"
            ]
        );
        assert!(plan.skipped[1].why.contains("eBIRForms"));
    }

    #[test]
    fn chart_types_from_quickbooks_xero_and_names() {
        let csv = "*Code,*Name,*Type\n200,Sales,Revenue\n310,Cost of Goods,Direct Costs\n090,Business Bank,Bank\n800,Accounts Payable,Current Liability\n";
        let chart = parse_csv_chart(csv).unwrap();
        assert_eq!(chart[0].number.as_deref(), Some("200"));
        assert_eq!(chart[1].account_type, AccountType::CostOfRevenue);
        assert_eq!(chart[2].account_type, AccountType::Asset);
        assert_eq!(chart[3].account_type, AccountType::Liability);
        let qb = "Account,Type\nAccounts Receivable (A/R),Accounts receivable (A/R)\nVisa,Credit Card\nIncome:Consulting,\nTOTAL,\n";
        let chart = parse_csv_chart(qb).unwrap();
        assert_eq!(chart.len(), 3);
        assert_eq!(chart[0].account_type, AccountType::Asset);
        assert_eq!(chart[1].account_type, AccountType::Liability);
        assert_eq!(chart[2].account_type, AccountType::Revenue, "from the name");
        assert_eq!(chart[2].parent.as_deref(), Some("Income"));
        let err = parse_csv_chart("Name\nThingamajig\n").unwrap_err();
        assert!(
            err.contains("Row 2") && err.contains("Thingamajig"),
            "{err}"
        );
        assert!(
            parse_csv_chart("Foo,Bar\n1,2\n")
                .unwrap_err()
                .contains("account name")
        );
    }

    #[test]
    fn journal_groups_by_reference_or_by_balance() {
        let csv = "Date,Num,Account,Debit,Credit\n2026-07-01,7,Cash,100.00,\n2026-07-01,8,Cash,50.00,\n2026-07-01,7,Sales,,100.00\n2026-07-01,8,Sales,,50.00\n";
        let e = parse_csv_journal(csv).unwrap();
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].reference.as_deref(), Some("7"));
        assert_eq!(e[0].lines.len(), 2);
        assert!(e.iter().all(|e| e.posted));
        let signed = "Date,Account number,Amount\n07/01/2026,11000,12.50\n07/01/2026,41000,-12.50\n07/02/2026,11000,(3.00)\n07/02/2026,41000,3.00\n";
        let e = parse_csv_journal(signed).unwrap();
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].lines[0].account, AccountRef::Number("11000".into()));
        assert_eq!(e[0].lines[1].credit, 1_250);
        assert_eq!(e[1].lines[0].credit, 300);
        let err = parse_csv_journal(
            "Date,Account,Amount\n2026-07-01,Cash,1.00\n2026-07-02,Sales,-1.00\n",
        )
        .unwrap_err();
        assert!(err.contains("Row 3"), "{err}");
        let err = parse_csv_journal("Date,Account,Amount\n2026-07-01,Cash,1.00\n").unwrap_err();
        assert!(err.contains("do not balance"), "{err}");
        let err = parse_csv_journal("Date,Account,Amount\n2026-07-01,Cash,1.005\n").unwrap_err();
        assert!(err.contains("Row 2"), "{err}");
    }

    #[test]
    fn the_drawing_examples() {
        let s = suggest_kind(None, "Income:Services", AccountType::Revenue);
        assert_eq!(s.kind, "sales_revenue");
        assert!(s.confidence >= 80);
        assert_eq!(
            suggest_kind(None, "Payroll Liabilities", AccountType::Liability),
            Suggestion {
                kind: "mixed".into(),
                confidence: 64
            }
        );
        assert_eq!(
            suggest_kind(None, "Ask My Accountant", AccountType::Expense),
            Suggestion {
                kind: "unsure".into(),
                confidence: 30
            }
        );
        let s = suggest_kind(Some("41000"), "Sales Revenue", AccountType::Revenue);
        assert_eq!((s.kind.as_str(), s.confidence), ("sales_revenue", 96));
        let s = suggest_kind(None, "41000 Sales Revenue", AccountType::Revenue);
        assert_eq!((s.kind.as_str(), s.confidence), ("sales_revenue", 96));
    }

    #[test]
    fn suggestions_follow_the_order_and_the_type() {
        let s = |n: Option<&str>, name: &str, t| {
            let s = suggest_kind(n, name, t);
            (s.kind, s.confidence)
        };
        use AccountType::*;
        assert_eq!(s(Some("66100"), "Shop lease", Expense), ("rent".into(), 90));
        assert_eq!(
            s(Some("66100"), "Shop lease", Asset).1,
            30,
            "a number of the wrong type"
        );
        assert_eq!(
            s(None, "Refunds & Discounts", Revenue),
            ("refunds_and_discounts".into(), 88)
        );
        assert_eq!(
            s(None, "Interest Income", Revenue),
            ("interest_income".into(), 88)
        );
        assert_eq!(s(None, "Bank Service Charges", Expense).0, "bank_fees");
        assert_eq!(s(None, "Checking", Asset).0, "bank_accounts");
        assert_eq!(s(None, "Sales Returns", Revenue).0, "refunds_and_discounts");
        assert_eq!(s(None, "Visa", Liability).1, 30);
        assert_eq!(
            s(None, "Company Visa credit card", Liability).0,
            "loan_payable"
        );
        assert_eq!(
            s(None, "Other Current Asset", Asset).0,
            "unsure",
            "rent is not current"
        );
        assert_eq!(s(None, "Uncategorized Income", Revenue).0, "unsure");
        assert_eq!(s(None, "Owner's Draw", Equity).0, "owner_drawings");
        assert_eq!(
            s(None, "Cost of Goods Sold", CostOfRevenue),
            ("cost_of_goods_sold".into(), 88)
        );
    }

    #[test]
    fn every_rule_names_a_catalogue_kind_in_range() {
        let c = Catalog::get();
        for (_, kind, confidence) in KEYWORD_RULES {
            assert!(c.kind(kind).is_some(), "{kind}");
            assert!((60..=84).contains(confidence), "{kind}");
        }
    }
}
