//! Books' own tools. Every name starts with `books_` (spec 5.3; RK-5: a
//! tool that does not is skipped silently by Rox's agent surface).
//!
//! Who may call what (docs/01 §5.5, DD-20):
//! - reads: the app and the assistant (the assistant gets totals and
//!   tickets, never memos or lines; D11);
//! - `books_entry_draft`: the app and the assistant (D16), and a routine
//!   that declares it;
//! - `books_app_*`: the app only, behind a person's click. They carry
//!   `_meta["rox/audience"] = ["app"]`, so the model never sees them.

use serde_json::{Map, Value, json};

use crate::jsonrpc::{fail, ok};
use crate::manifest;
use crate::service::{self, Books, Origin, Refusal, answer, write};
use crate::{records, reports, views, writes};

pub const STATUS: &str = "books_status";
pub const RESOLVE: &str = "books_resolve";
pub const BOOK_GET: &str = "books_book_get";
pub const TODAY: &str = "books_today";
pub const LEDGER_LIST: &str = "books_ledger_list";
pub const ENTRY_GET: &str = "books_entry_get";
pub const ENTRY_OPTIONS: &str = "books_entry_options";
pub const ACCOUNTS_LIST: &str = "books_accounts_list";
pub const ENTRY_DRAFT: &str = "books_entry_draft";
pub const POST_ENTRY: &str = "books_app_post_entry";
pub const VOID_ENTRY: &str = "books_app_void_entry";
pub const DISCARD_ENTRY: &str = "books_app_discard_entry";
pub const BOOK_CREATE: &str = "books_app_book_create";
pub const ACCOUNT_CREATE: &str = "books_app_account_create";
pub const PEOPLE_LIST: &str = "books_people_list";
pub const PERSON_GET: &str = "books_person_get";
pub const TAGS_LIST: &str = "books_tags_list";
pub const OWED_LIST: &str = "books_owed_list";
pub const OWED_OPTIONS: &str = "books_owed_options";
pub const REPORT: &str = "books_report";
pub const REPORT_CSV: &str = "books_report_csv";
pub const PERSON_CREATE: &str = "books_app_person_create";
pub const TAG_CREATE: &str = "books_app_tag_create";
pub const OWED_CREATE: &str = "books_app_owed_create";
pub const OWED_SETTLE: &str = "books_app_owed_settle";
pub const OWED_CANCEL: &str = "books_app_owed_cancel";

/// Who a tool is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// The app and the assistant.
    Everyone,
    /// The app only, after a click.
    App,
}

pub struct Tool {
    pub name: &'static str,
    pub reach: Reach,
    pub write: bool,
    pub description: &'static str,
    schema: fn() -> Value,
}

fn none() -> Value {
    json!({ "type": "object", "properties": {}, "additionalProperties": false })
}

fn book_only() -> Value {
    json!({ "type": "object", "properties": { "book": { "type": "string", "description": "A book ticket; the only book when absent." } } })
}

fn resolve_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "kind": { "type": "string", "enum": ["command", "scope"] },
            "name": { "type": "string" },
            "value": { "type": "string" },
            "complete": { "type": "boolean" },
            "context": { "type": "object" }
        }
    })
}

fn ledger_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "book": { "type": "string" },
            "month": { "type": "string", "description": "YYYY-MM; this month when absent." },
            "account": { "type": "string", "description": "An account ticket or chart number; every account when absent." },
            "filter": { "type": "string", "enum": ["all", "review", "draft", "posted"] }
        }
    })
}

fn entry_ref() -> Value {
    json!({
        "type": "object",
        "required": ["entry"],
        "properties": { "book": { "type": "string" }, "entry": { "type": "string", "description": "An entry ticket." } }
    })
}

fn accounts_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "book": { "type": "string" },
            "group": { "type": "string", "enum": ["all", "Assets", "Liabilities", "Equity", "Revenue", "Expenses"] }
        }
    })
}

fn draft_schema() -> Value {
    json!({
        "type": "object",
        "required": ["client_request_id"],
        "properties": {
            "client_request_id": { "type": "string", "description": "8 to 64 characters; a repeat returns the stored result." },
            "book": { "type": "string" },
            "entry": { "type": "string", "description": "A draft's ticket, to replace its content." },
            "copy_of": { "type": "string", "description": "A posted entry's ticket, to start a new draft from it." },
            "form": { "type": "string", "enum": ["money_in", "money_out", "transfer", "journal"] },
            "date": { "type": "string", "description": "YYYY-MM-DD. On a cash book, the day the money moved." },
            "amount": { "type": "string", "description": "Decimal text with two places, e.g. \"1350.00\"." },
            "account": { "type": "string", "description": "The bank, cash or card account (ticket or number); for a transfer, where the money leaves." },
            "to_account": { "type": "string", "description": "Transfer only: where the money goes." },
            "category": { "type": "string", "description": "The income or expense account (ticket or number)." },
            "payee": { "type": "string" },
            "memo": { "type": "string", "description": "What it was for. Never a TIN." },
            "reference": { "type": "string", "description": "An invoice or receipt number." },
            "facts": { "type": "object", "description": "Line facts for the category line, e.g. {\"document_kind\": \"sale_invoice\"}." },
            "lines": { "type": "array", "description": "Journal only: [{account, debit, credit, facts}]." },
            "person": { "type": "string", "description": "A person ticket." },
            "person_kind": { "type": "string", "description": "With payee: the kind of person to link or make (vendors, customers, employees, owners, lenders, government)." },
            "department": { "type": "string" },
            "location": { "type": "string" }
        }
    })
}

fn people_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "book": { "type": "string" },
            "kind": { "type": "string", "enum": ["banks", "vendors", "customers", "employees", "owners", "lenders", "government"] }
        }
    })
}

fn person_ref() -> Value {
    json!({ "type": "object", "required": ["person"], "properties": { "book": { "type": "string" }, "person": { "type": "string", "description": "A person or bank account ticket." } } })
}

fn tags_schema() -> Value {
    json!({ "type": "object", "properties": { "book": { "type": "string" }, "kind": { "type": "string", "enum": ["department", "location"] }, "month": { "type": "string" } } })
}

fn owed_schema() -> Value {
    json!({ "type": "object", "properties": { "book": { "type": "string" }, "side": { "type": "string", "enum": ["collect", "pay"] } } })
}

fn report_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "book": { "type": "string" },
            "kind": { "type": "string", "enum": ["pl", "bs", "cf", "tb", "ag"] },
            "period_kind": { "type": "string", "enum": ["month", "quarter", "year"] },
            "period": { "type": "string", "description": "2026-10, 2026-Q3 or 2026; this period when absent." },
            "side": { "type": "string", "enum": ["collect", "pay"], "description": "Aging only." }
        }
    })
}

fn person_create_schema() -> Value {
    json!({
        "type": "object",
        "required": ["client_request_id", "kind", "name"],
        "properties": { "client_request_id": { "type": "string" }, "book": { "type": "string" }, "kind": { "type": "string" }, "name": { "type": "string", "description": "A name, never a TIN." }, "note": { "type": "string" } }
    })
}

fn tag_create_schema() -> Value {
    json!({
        "type": "object",
        "required": ["client_request_id", "kind", "name"],
        "properties": { "client_request_id": { "type": "string" }, "book": { "type": "string" }, "kind": { "type": "string", "enum": ["department", "location"] }, "name": { "type": "string" } }
    })
}

fn owed_create_schema() -> Value {
    json!({
        "type": "object",
        "required": ["client_request_id", "side", "amount", "category"],
        "properties": {
            "client_request_id": { "type": "string" }, "book": { "type": "string" },
            "side": { "type": "string", "enum": ["collect", "pay"] },
            "person": { "type": "string" }, "person_name": { "type": "string" },
            "amount": { "type": "string" }, "category": { "type": "string" },
            "issued": { "type": "string" }, "due": { "type": "string" },
            "reference": { "type": "string" }, "memo": { "type": "string" }
        }
    })
}

fn owed_ref() -> Value {
    json!({
        "type": "object",
        "required": ["client_request_id", "owed"],
        "properties": { "client_request_id": { "type": "string" }, "book": { "type": "string" }, "owed": { "type": "string" }, "account": { "type": "string" }, "date": { "type": "string" } }
    })
}

fn write_ref() -> Value {
    json!({
        "type": "object",
        "required": ["client_request_id", "entry"],
        "properties": { "client_request_id": { "type": "string" }, "book": { "type": "string" }, "entry": { "type": "string" } }
    })
}

fn book_create_schema() -> Value {
    json!({
        "type": "object",
        "required": ["client_request_id", "label"],
        "properties": {
            "client_request_id": { "type": "string" },
            "label": { "type": "string", "description": "The name shown in Rox; never a TIN." },
            "basis": { "type": "string", "enum": ["cash", "accrual"], "description": "Cash unless chosen (ADR 0008)." },
            "fiscal_year_start": { "type": "integer", "minimum": 1, "maximum": 12 }
        }
    })
}

fn account_create_schema() -> Value {
    json!({
        "type": "object",
        "required": ["client_request_id", "number", "name", "kind"],
        "properties": {
            "client_request_id": { "type": "string" },
            "book": { "type": "string" },
            "number": { "type": "string" },
            "name": { "type": "string" },
            "kind": { "type": "string", "description": "A kind id of the catalogue, mixed or unsure." },
            "type": { "type": "string" }
        }
    })
}

/// Every Books tool, in `tools/list` order.
pub const TOOLS: &[Tool] = &[
    Tool {
        name: STATUS,
        reach: Reach::Everyone,
        write: false,
        schema: none,
        description: "Report Buwiz Books' setup status on this computer: the plugin version, whether a book exists, and its accounting basis. Counts and status only; never amounts, names or TINs.",
    },
    Tool {
        name: RESOLVE,
        reach: Reach::Everyone,
        write: false,
        schema: resolve_schema,
        description: "Resolve the /books command for Rox.",
    },
    Tool {
        name: BOOK_GET,
        reach: Reach::Everyone,
        write: false,
        schema: book_only,
        description: "The book's neutral label, accounting basis (cash by default) and counts of entries, drafts and lines to review.",
    },
    Tool {
        name: TODAY,
        reach: Reach::Everyone,
        write: false,
        schema: book_only,
        description: "Today in Books: cash now by account, this month's money in and out, unpaid totals, and what needs the user. Amounts are decimal text.",
    },
    Tool {
        name: LEDGER_LIST,
        reach: Reach::Everyone,
        write: false,
        schema: ledger_schema,
        description: "A month of ledger entries, for one account or all: date, status, category and amount, each with an entry ticket. The assistant sees no payees or memos.",
    },
    Tool {
        name: ENTRY_GET,
        reach: Reach::Everyone,
        write: false,
        schema: entry_ref,
        description: "One entry by ticket: status, date, category and amount. Lines, payee and memo are shown in the Books app only.",
    },
    Tool {
        name: ENTRY_OPTIONS,
        reach: Reach::Everyone,
        write: false,
        schema: book_only,
        description: "The accounts a new entry can use: bank, cash and card accounts, income and expense categories, and the whole chart for a journal.",
    },
    Tool {
        name: ACCOUNTS_LIST,
        reach: Reach::Everyone,
        write: false,
        schema: accounts_schema,
        description: "The chart of accounts with balances, by group. On a cash book, receivable and payable accounts are marked accrual-only.",
    },
    Tool {
        name: ENTRY_DRAFT,
        reach: Reach::Everyone,
        write: true,
        schema: draft_schema,
        description: "Save a DRAFT entry: money in, money out, a transfer or a journal. A draft is in no total until the user posts it in the Books app; the assistant cannot post. Takes a client_request_id; a repeat returns the stored result.",
    },
    Tool {
        name: POST_ENTRY,
        reach: Reach::App,
        write: true,
        schema: draft_schema,
        description: "Post a draft (or save and post a new entry) after the user's click. Refuses an unbalanced entry, and receivable or payable lines on a cash book.",
    },
    Tool {
        name: VOID_ENTRY,
        reach: Reach::App,
        write: true,
        schema: write_ref,
        description: "Void a posted entry after the user's click: it shows Voided and a reversal is posted on its date.",
    },
    Tool {
        name: DISCARD_ENTRY,
        reach: Reach::App,
        write: true,
        schema: write_ref,
        description: "Discard a draft after the user's click.",
    },
    Tool {
        name: BOOK_CREATE,
        reach: Reach::App,
        write: true,
        schema: book_create_schema,
        description: "Create a book on the Philippines small-business chart, cash basis unless the user picks accrual.",
    },
    Tool {
        name: ACCOUNT_CREATE,
        reach: Reach::App,
        write: true,
        schema: account_create_schema,
        description: "Add an account to the chart under one kind, or as not sure yet.",
    },
    Tool {
        name: PEOPLE_LIST,
        reach: Reach::Everyone,
        write: false,
        schema: people_schema,
        description: "People of one kind (banks and cards, vendors, customers, employees, owners, lenders, government) with counts per kind. No TINs: the BIR app keeps them.",
    },
    Tool {
        name: PERSON_GET,
        reach: Reach::Everyone,
        write: false,
        schema: person_ref,
        description: "One person or bank account: what was paid or received this year, ten months of totals and the last entries.",
    },
    Tool {
        name: TAGS_LIST,
        reach: Reach::Everyone,
        write: false,
        schema: tags_schema,
        description: "Departments or locations, with the month's money out and entry counts.",
    },
    Tool {
        name: OWED_LIST,
        reach: Reach::Everyone,
        write: false,
        schema: owed_schema,
        description: "Unpaid invoices to collect or bills to pay, by due date, with totals. On a cash book they are reminders, not entries.",
    },
    Tool {
        name: OWED_OPTIONS,
        reach: Reach::Everyone,
        write: false,
        schema: book_only,
        description: "What a new invoice or bill can use: customers, vendors, categories and money accounts.",
    },
    Tool {
        name: REPORT,
        reach: Reach::Everyone,
        write: false,
        schema: report_schema,
        description: "A report on the book's basis: P&L with the previous period, balance sheet, cash flow, trial balance, or aging (accrual books only).",
    },
    Tool {
        name: REPORT_CSV,
        reach: Reach::Everyone,
        write: false,
        schema: report_schema,
        description: "The same report as CSV text, amounts as two-place decimal text.",
    },
    Tool {
        name: PERSON_CREATE,
        reach: Reach::App,
        write: true,
        schema: person_create_schema,
        description: "Add a person of one kind by name. Never a TIN.",
    },
    Tool {
        name: TAG_CREATE,
        reach: Reach::App,
        write: true,
        schema: tag_create_schema,
        description: "Add a department or a location.",
    },
    Tool {
        name: OWED_CREATE,
        reach: Reach::App,
        write: true,
        schema: owed_create_schema,
        description: "Save an invoice to collect or a bill to pay. On a cash book it is a reminder; on an accrual book it posts the receivable or payable.",
    },
    Tool {
        name: OWED_SETTLE,
        reach: Reach::App,
        write: true,
        schema: owed_ref,
        description: "Mark an invoice or bill paid after the user's click: posts the money on that day.",
    },
    Tool {
        name: OWED_CANCEL,
        reach: Reach::App,
        write: true,
        schema: owed_ref,
        description: "Cancel an open invoice or bill after the user's click.",
    },
];

pub fn find(name: &str) -> Option<&'static Tool> {
    TOOLS.iter().find(|t| t.name == name)
}

pub fn names() -> Vec<&'static str> {
    TOOLS.iter().map(|t| t.name).collect()
}

/// Whether `name` changes nothing (`readOnlyHint`, P8).
pub fn read_only(name: &str) -> bool {
    find(name).is_some_and(|t| !t.write)
}

/// The `tools/list` entries.
pub fn list() -> Vec<Value> {
    TOOLS
        .iter()
        .map(|t| {
            let mut entry = json!({
                "name": t.name,
                "description": t.description,
                "inputSchema": (t.schema)(),
                "annotations": {
                    "readOnlyHint": !t.write,
                    "destructiveHint": false,
                    "idempotentHint": true,
                    "openWorldHint": false
                },
            });
            if t.reach == Reach::App {
                entry["_meta"] = json!({ "rox/audience": ["app"] });
            }
            entry
        })
        .collect()
}

/// Run a Books tool. `None`: no such tool.
pub fn call(
    name: &str,
    arguments: &Map<String, Value>,
    origin: Origin,
    books: &mut Books,
) -> Option<Value> {
    find(name)?;
    Some(match name {
        STATUS => status(books),
        RESOLVE => resolve(arguments, books),
        BOOK_GET => read(books, arguments, |store, book, _| {
            let c = store.conn();
            let count = |sql: &str| -> Result<i64, Refusal> {
                Ok(c.query_row(sql, [book.id], |r| r.get(0))
                    .map_err(books_store::StoreError::from)?)
            };
            let mut out = views::book_json(store, &book)?;
            out["entries"] = json!(count(
                "SELECT count(*) FROM entry WHERE book_id = ?1 AND status IN ('posted','voided')"
            )?);
            out["drafts"] = json!(count(
                "SELECT count(*) FROM entry WHERE book_id = ?1 AND status = 'draft' AND needs_review = 0"
            )?);
            // drafts from a statement, and posted lines on "not sure yet" accounts
            out["to_review"] = json!(count(
                "SELECT count(*) FROM entry e WHERE e.book_id = ?1 AND (
                   (e.status = 'draft' AND e.needs_review = 1)
                   OR (e.status = 'posted' AND e.form <> 'reversal' AND EXISTS (
                     SELECT 1 FROM line l JOIN account a ON a.account_id = l.account_id
                     WHERE l.entry_id = e.entry_id AND a.kind = 'unsure')))"
            )?);
            out["unpaid"] = json!(count(
                "SELECT count(*) FROM unpaid WHERE book_id = ?1 AND status = 'open'"
            )?);
            Ok(out)
        }),
        TODAY => read(books, arguments, |store, book, _| {
            views::today(store, book, origin)
        }),
        LEDGER_LIST => read(books, arguments, |store, book, args| {
            views::ledger(store, book, args, origin)
        }),
        ENTRY_GET => read(books, arguments, |store, book, args| {
            views::entry(store, book, service::required(args, "entry")?, origin)
        }),
        ENTRY_OPTIONS => read(books, arguments, |store, book, _| {
            views::entry_options(store, book)
        }),
        ACCOUNTS_LIST => read(books, arguments, |store, book, args| {
            views::accounts(store, book, args)
        }),
        ENTRY_DRAFT => write(books, name, origin, arguments, |tx| {
            writes::draft(tx, arguments, origin)
        }),
        POST_ENTRY => write(books, name, origin, arguments, |tx| {
            writes::post(tx, arguments)
        }),
        VOID_ENTRY => write(books, name, origin, arguments, |tx| {
            writes::void(tx, arguments)
        }),
        DISCARD_ENTRY => write(books, name, origin, arguments, |tx| {
            writes::discard(tx, arguments)
        }),
        BOOK_CREATE => write(books, name, origin, arguments, |tx| {
            writes::book_create(tx, arguments)
        }),
        ACCOUNT_CREATE => write(books, name, origin, arguments, |tx| {
            writes::account_create(tx, arguments)
        }),
        PEOPLE_LIST => read(books, arguments, |store, book, args| {
            records::people_list(store, book, args)
        }),
        PERSON_GET => read(books, arguments, |store, book, args| {
            records::person_get(store, book, args, origin)
        }),
        TAGS_LIST => read(books, arguments, |store, book, args| {
            records::tags_list(store, book, args)
        }),
        OWED_LIST => read(books, arguments, |store, book, args| {
            records::owed_list(store, book, args, origin)
        }),
        OWED_OPTIONS => read(books, arguments, |store, book, _| {
            records::owed_options(store, book)
        }),
        REPORT => read(books, arguments, |store, book, args| {
            reports::report(store, book, args)
        }),
        REPORT_CSV => read(books, arguments, |store, book, args| {
            reports::report_csv(store, book, args)
        }),
        PERSON_CREATE => write(books, name, origin, arguments, |tx| {
            records::person_create(tx, arguments)
        }),
        TAG_CREATE => write(books, name, origin, arguments, |tx| {
            records::tag_create(tx, arguments)
        }),
        OWED_CREATE => write(books, name, origin, arguments, |tx| {
            records::owed_create(tx, arguments)
        }),
        OWED_SETTLE => write(books, name, origin, arguments, |tx| {
            records::owed_settle(tx, arguments)
        }),
        OWED_CANCEL => write(books, name, origin, arguments, |tx| {
            records::owed_cancel(tx, arguments)
        }),
        _ => fail("internal", "Books has no handler for that tool."),
    })
}

/// A read about one book.
fn read(
    books: &mut Books,
    args: &Map<String, Value>,
    f: impl FnOnce(
        &books_store::Store,
        books_store::books::Book,
        &Map<String, Value>,
    ) -> service::Answer,
) -> Value {
    if !books.exists() {
        return fail(
            "not_set_up",
            "There is no book yet. Open Books to make one.",
        );
    }
    answer((|| {
        let store = books.store()?;
        let book = service::book(store, args)?;
        f(store, book, args)
    })())
}

/// Setup status, without making a store when none exists.
fn status(books: &mut Books) -> Value {
    let mut out = json!({ "plugin": manifest::PLUGIN_ID, "version": manifest::VERSION });
    if !books.exists() {
        out["status"] = json!("not_set_up");
        out["books"] = json!(0);
        return ok(out);
    }
    answer((|| {
        let store = books.store()?;
        let all = books_store::books::all(store.conn())?;
        out["status"] = json!(if all.is_empty() {
            "not_set_up"
        } else {
            "ready"
        });
        out["books"] = json!(all.len());
        if let Some(b) = all.first() {
            out["basis"] = json!(b.basis.as_str());
        }
        Ok(out)
    })())
}

/// Spec 9.2: `{kind, name, value, complete, context}` in, one `result` out.
fn resolve(arguments: &Map<String, Value>, books: &mut Books) -> Value {
    let kind = arguments.get("kind").and_then(Value::as_str).unwrap_or("");
    let name = arguments.get("name").and_then(Value::as_str).unwrap_or("");
    match (kind, name) {
        ("command", "books") => match books.home_ticket() {
            Ok(ticket) => ok(json!({
                "result": "open",
                "app": "books",
                "props": { "page": "today" },
                "ticket": ticket,
                "title": service::HOME_TITLE,
                "placement": "full",
            })),
            Err(_) => fail("internal", "Books could not open right now."),
        },
        _ => ok(
            json!({ "result": "error", "message": "Books does not know that command or scope." }),
        ),
    }
}
