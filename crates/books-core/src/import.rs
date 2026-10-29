//! Bringing old books in (M5): a web Buwiz Books export, or CSV files (a
//! chart and a journal), read into one [`Plan`] the store then applies.
//! Pure: no store, no float. And Map 1: a suggested kind, with a
//! confidence, for each imported account; the owner confirms it.
//!
//! Books reads only the fields it keeps. Nothing that holds a TIN is read:
//! the web export's Philippine tax tables (`ph*`) are skipped whole, and
//! any run of nine or more digits in a name or memo is masked.

use crate::catalog::AccountType;
use crate::date::Date;

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
