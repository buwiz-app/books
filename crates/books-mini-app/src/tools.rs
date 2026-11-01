//! Books' own tools. Every name starts with `books_` (spec 5.3; RK-5: a
//! tool that does not is skipped silently by the Bot's agent surface).
//!
//! Who may call what (docs/01 §5.5, DD-20):
//! - reads: the app and the assistant (the assistant gets totals and
//!   tickets, never memos or lines; D11);
//! - `books_entry_draft`: the app and the assistant (D16), and a routine
//!   that declares it;
//! - `books_app_*`: the app only, behind a person's click. They carry
//!   `_meta["buwiz/audience"] = ["app"]`, so the model never sees them.

use serde_json::{Map, Value, json};

#[path = "document_review.rs"]
pub mod document_review;
#[path = "evidence_tools.rs"]
pub(crate) mod evidence_tools;
#[path = "owed_proposals.rs"]
pub(crate) mod owed_proposals;

use crate::jsonrpc::{fail, ok};
use crate::manifest;
use crate::service::{self, Books, Origin, Refusal, answer, write};
use crate::{
    cards, checks, forms, handoff, oldbooks, owed_repeats, payroll, reconcile, reconcile_document,
    records, reports, statement_read, views, writes,
};

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
pub const RECONCILE_GRID: &str = "books_reconcile_grid";
pub const STATEMENT_GET: &str = "books_statement_get";
pub const SETTINGS_GET: &str = "books_settings_get";
pub const STATEMENT_IMPORT: &str = "books_app_statement_import";
pub const STATEMENT_ACCEPT: &str = "books_app_statement_accept";
pub const STATEMENT_ADD: &str = "books_app_statement_add_line";
pub const STATEMENT_FINISH: &str = "books_app_statement_finish";
pub const RECATEGORIZE: &str = "books_app_recategorize";
pub const MONTH_CLOSE: &str = "books_app_month_close";
pub const MONTH_REOPEN: &str = "books_app_month_reopen";
pub const DEFAULT_SET: &str = "books_app_default_set";
pub const BOOK_UPDATE: &str = "books_app_book_update";
pub const UPLOAD_PART: &str = "books_app_upload_part";
pub const IMPORT: &str = "books_app_import";
pub const IMPORT_GET: &str = "books_import_get";
pub const ACCOUNT_MATCH: &str = "books_app_account_match";
pub const ACCOUNTS_CONFIRM: &str = "books_app_accounts_confirm";
pub const OPENING_SET: &str = "books_app_opening_set";
pub const RETURN_OK: &str = "books_app_return_ok";
pub const FORMS: &str = "books_forms";
pub const FORM_PROPOSAL: &str = "books_form_proposal";
pub const FORM_LINEAGE: &str = "books_app_form_lineage";
pub const CERTIFICATES: &str = "books_certificates_list";
pub const CERTIFICATE_ADD: &str = "books_app_certificate_add";
pub const CERTIFICATE_SET: &str = "books_app_certificate_set";
pub const WAGES: &str = "books_wages_list";
pub const WAGE_RUN_SAVE: &str = "books_app_wage_run_save";
pub const HANDOFF_CREATE: &str = "books_app_handoff_create";
pub const HANDOFF_GET: &str = "books_handoff_get";
pub const HANDOFF_CONFIRM: &str = "books_app_handoff_confirm";
pub const CROSS_CHECKS: &str = "books_cross_checks";
pub const SHOW: &str = "books_show";
pub const NUDGE: &str = "books_nudge";
pub const SIGNIN: &str = "books_app_signin";
pub const TAX_PROFILES: &str = "books_app_tax_list";
pub const PROFILE_LINK: &str = "books_app_tax_link";
pub const EVIDENCE_ATTACH: &str = "books_app_evidence_attach";
pub const EVIDENCE_DETACH: &str = "books_app_evidence_detach";
pub const EVIDENCE_LIST: &str = "books_evidence_list";
pub const OWED_DRAFT: &str = "books_owed_draft";
pub const EVIDENCE_CLASSIFY: &str = "books_evidence_classify";
pub const EVIDENCE_KIND: &str = "books_app_evidence_kind";
pub const EVIDENCE_PREVIEW: &str = "books_app_evidence_preview";
pub const EVIDENCE_REVIEW: &str = "books_app_evidence_review";
pub const STATEMENT_DRAFT_LINES: &str = "books_statement_draft_lines";
pub const RECONCILE_DOCUMENT: &str = "books_app_reconcile_document";
pub const CHECKS: &str = "books_checks";
pub const OWED_DOCUMENT: &str = "books_owed_document";
pub const OWED_REPEAT: &str = "books_app_owed_repeat";

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
            "filter": { "type": "string", "enum": ["all", "review", "draft", "posted", "checks"] }
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
            "location": { "type": "string" },
            "extracted": { "type": "object", "description": "What the assistant read, per field: {\"fields\": {\"category\": {\"confidence\": 55}}}. Fields: date, amount, account, to_account, category, payee, memo, reference, lines; confidence is a whole percent." }
        }
    })
}

fn post_schema() -> Value {
    let mut schema = draft_schema();
    schema["properties"]["keep_both"] = json!({ "type": "boolean", "description": "Post although the draft looks like another entry; records that the user keeps both." });
    schema
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
    json!({ "type": "object", "properties": { "book": { "type": "string" }, "side": { "type": "string", "enum": ["collect", "pay"] }, "owed": { "type": "string", "description": "A document ticket: adds its payment history as detail." } } })
}

fn report_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "book": { "type": "string" },
            "kind": { "type": "string", "enum": ["pl", "bs", "cf", "tb", "ag", "ac"] },
            "period_kind": { "type": "string", "enum": ["month", "quarter", "year"] },
            "period": { "type": "string", "description": "2026-10, 2026-Q3 or 2026; this period when absent." },
            "side": { "type": "string", "enum": ["collect", "pay"], "description": "Aging only." },
                "compare_to": { "type": "string", "enum": ["previous", "last_year", "none"], "description": "P&L, balance sheet and cash flow: the period shown beside this one. previous by default on the P&L, none otherwise." },
                "tag": { "type": "string", "description": "P&L and cash flow only: a department or location ticket from books_tags_list." },
            "account": { "type": "string", "description": "Account activity (ac) only: an account ticket or number." }
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
        "required": ["client_request_id"],
        "properties": {
            "client_request_id": { "type": "string" }, "book": { "type": "string" },
            "side": { "type": "string", "enum": ["collect", "pay"], "description": "Required without a proposal, as are amount and category." },
            "proposal": { "type": "string", "description": "A proposal ticket from books_owed_list; with decision." },
            "decision": { "type": "string", "enum": ["accept", "discard"] },
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

fn owed_settle_schema() -> Value {
    json!({
        "type": "object",
        "required": ["client_request_id", "owed", "account"],
        "properties": {
            "client_request_id": { "type": "string" }, "book": { "type": "string" },
            "owed": { "type": "string" }, "account": { "type": "string" },
            "amount": { "type": "string", "description": "Two-place decimal text; the whole balance when absent." },
            "on": { "type": "string", "description": "YYYY-MM-DD; today when absent." },
            "date": { "type": "string", "description": "Same as on." }
        }
    })
}

fn statement_ref() -> Value {
    json!({ "type": "object", "required": ["account", "month"], "properties": { "book": { "type": "string" }, "account": { "type": "string" }, "month": { "type": "string", "description": "YYYY-MM" } } })
}

fn reconcile_document_schema() -> Value {
    json!({ "type": "object", "required": ["account", "month", "kind"], "properties": { "book": { "type": "string" }, "account": { "type": "string" }, "month": { "type": "string", "description": "YYYY-MM" }, "kind": { "type": "string", "enum": ["report", "page"] }, "page": { "type": "integer", "minimum": 1 }, "line": { "type": "integer", "minimum": 1 } } })
}

fn statement_write() -> Value {
    json!({
        "type": "object",
        "required": ["client_request_id", "account"],
        "properties": {
            "client_request_id": { "type": "string" }, "book": { "type": "string" },
            "account": { "type": "string" }, "month": { "type": "string" },
            "line": { "type": ["string", "integer"], "description": "A statement line ticket; for accept, a row's `no` to match that one line." },
            "csv_base64": { "type": "string", "description": "The CSV file's bytes, base64url." },
            "csv_text": { "type": "string" }, "closing": { "type": "string", "description": "The ending balance, two-place text." },
            "file_name": { "type": "string" },
            "entry": { "type": "string", "description": "With accept and a row `no`: the entry ticket to match that line to." }
        }
    })
}

fn recategorize_schema() -> Value {
    json!({ "type": "object", "required": ["client_request_id", "entry", "category"], "properties": { "client_request_id": { "type": "string" }, "book": { "type": "string" }, "entry": { "type": "string" }, "category": { "type": "string" } } })
}

fn month_write() -> Value {
    json!({ "type": "object", "required": ["client_request_id", "month"], "properties": { "client_request_id": { "type": "string" }, "book": { "type": "string" }, "month": { "type": "string" }, "reason": { "type": "string", "description": "Required to reopen." } } })
}

fn default_schema() -> Value {
    json!({ "type": "object", "required": ["client_request_id", "slot", "account"], "properties": { "client_request_id": { "type": "string" }, "book": { "type": "string" }, "slot": { "type": "string" }, "account": { "type": "string" } } })
}

fn book_update_schema() -> Value {
    json!({ "type": "object", "required": ["client_request_id"], "properties": { "client_request_id": { "type": "string" }, "book": { "type": "string" }, "label": { "type": "string" }, "fiscal_year_start": { "type": "integer" }, "basis": { "type": "string", "enum": ["cash", "accrual"] } } })
}

fn upload_part_schema() -> Value {
    json!({ "type": "object", "required": ["upload", "part", "parts", "data_base64"], "properties": {
        "upload": { "type": "string", "description": "The app's id for one picked file." },
        "part": { "type": "integer", "minimum": 0 }, "parts": { "type": "integer", "minimum": 1 },
        "data_base64": { "type": "string", "description": "This part's bytes, base64url." } } })
}

fn import_schema() -> Value {
    json!({ "type": "object", "required": ["client_request_id", "source"], "properties": {
        "client_request_id": { "type": "string" }, "book": { "type": "string" },
        "source": { "type": "string", "enum": ["web", "csv"] },
        "web_upload": { "type": "string" }, "chart_upload": { "type": "string" }, "journal_upload": { "type": "string" },
        "web_text": { "type": "string" }, "chart_text": { "type": "string" }, "journal_text": { "type": "string" },
        "file_name": { "type": "string" } } })
}

fn account_match_schema() -> Value {
    json!({ "type": "object", "required": ["client_request_id", "account", "kind"], "properties": {
        "client_request_id": { "type": "string" }, "book": { "type": "string" },
        "account": { "type": "string" }, "kind": { "type": "string", "description": "A kind id, mixed or unsure." } } })
}

fn confirm_schema() -> Value {
    json!({ "type": "object", "required": ["client_request_id"], "properties": {
        "client_request_id": { "type": "string" }, "book": { "type": "string" },
        "min": { "type": "integer", "minimum": 50, "maximum": 100, "description": "Confirm suggestions at least this sure (90)." } } })
}

fn opening_schema() -> Value {
    json!({ "type": "object", "required": ["client_request_id", "measure", "from"], "properties": {
        "client_request_id": { "type": "string" }, "book": { "type": "string" },
        "measure": { "type": "string" }, "from": { "type": "string" }, "to": { "type": "string" },
        "amount": { "type": "string" }, "proof": { "type": "string", "description": "What proves it: filed returns, bank statements." },
        "delete": { "type": "boolean" } } })
}

fn form_schema() -> Value {
    json!({ "type": "object", "properties": {
        "book": { "type": "string" },
        "form": { "type": "string", "enum": ["2551Q", "1601C", "1701MS"] },
        "period": { "type": "string", "description": "2026-Q3, 2026-09 or 2026; the last full period when absent." },
        "form_context": { "type": "object", "description": "The BIR choices read from the BIR draft, such as {\"/taxpayer/tax_option\": \"graduated\"}. Books never sets them." },
        "key": { "type": "string" } } })
}

fn certificates_schema() -> Value {
    json!({ "type": "object", "properties": { "book": { "type": "string" }, "filter": { "type": "string", "enum": ["all", "received", "pending"] } } })
}

fn certificate_add_schema() -> Value {
    json!({ "type": "object", "required": ["client_request_id", "form", "credit_kind", "atc", "quarter", "amount", "person"], "properties": {
        "client_request_id": { "type": "string" }, "book": { "type": "string" },
        "form": { "type": "string", "enum": ["2307", "2306"] }, "credit_kind": { "type": "string", "enum": ["pt", "it"] },
        "atc": { "type": "string", "description": "As printed on the paper." }, "quarter": { "type": "string" },
        "amount": { "type": "string" }, "status": { "type": "string", "enum": ["received", "pending"] },
        "person": { "type": "string" }, "against": { "type": "string" }, "date": { "type": "string" }, "reference": { "type": "string" } } })
}

fn certificate_set_schema() -> Value {
    json!({ "type": "object", "required": ["client_request_id", "certificate", "status"], "properties": {
        "client_request_id": { "type": "string" }, "book": { "type": "string" }, "certificate": { "type": "string" },
        "status": { "type": "string", "enum": ["received", "pending", "lost", "disputed"] } } })
}

fn wage_run_schema() -> Value {
    json!({ "type": "object", "required": ["client_request_id", "period_from", "period_to", "account", "employees"], "properties": {
        "client_request_id": { "type": "string" }, "book": { "type": "string" },
        "period_from": { "type": "string" }, "period_to": { "type": "string" }, "date": { "type": "string" },
        "account": { "type": "string", "description": "The bank or cash account net pay leaves." },
        "employees": { "type": "array", "items": { "type": "object", "required": ["person"], "properties": {
            "person": { "type": "string" }, "basic": { "type": "string" }, "minimum_wage": { "type": "string" },
            "overtime": { "type": "string" }, "thirteenth": { "type": "string" }, "de_minimis": { "type": "string" },
            "ee_sss": { "type": "string" }, "ee_philhealth": { "type": "string" }, "ee_pagibig": { "type": "string" },
            "er_sss": { "type": "string" }, "er_philhealth": { "type": "string" }, "er_pagibig": { "type": "string" },
            "tax_withheld": { "type": "string", "description": "As payroll recorded it; Books does not compute it." } } } } } })
}

fn handoff_ref() -> Value {
    json!({ "type": "object", "required": ["handoff"], "properties": { "book": { "type": "string" }, "handoff": { "type": "string", "description": "A hand-off ticket (the proposal the owner attached)." }, "client_request_id": { "type": "string" } } })
}

fn handoff_create_schema() -> Value {
    json!({ "type": "object", "required": ["client_request_id", "form"], "properties": {
        "client_request_id": { "type": "string" }, "book": { "type": "string" },
        "form": { "type": "string", "enum": ["2551Q", "1601C", "1701MS"] }, "period": { "type": "string" },
        "form_context": { "type": "object" } } })
}

fn cross_checks_schema() -> Value {
    json!({ "type": "object", "properties": {
        "book": { "type": "string" }, "year": { "type": "string" },
        "filed": { "type": "array", "description": "Values BIR shows as filed, read from eBIRForms: [{form, period, key, value}].",
            "items": { "type": "object", "properties": { "form": { "type": "string" }, "period": { "type": "string" }, "key": { "type": "string" }, "value": { "type": "string" } } } } } })
}

fn show_schema() -> Value {
    json!({ "type": "object", "required": ["card"], "properties": {
        "book": { "type": "string" },
        "card": { "type": "string", "enum": cards::CARDS.map(|c| c.0) },
        "person": { "type": "string", "description": "A person ticket (person card)." },
        "account": { "type": "string" }, "entry": { "type": "string" }, "owed": { "type": "string" },
        "certificate": { "type": "string" }, "month": { "type": "string" }, "period": { "type": "string" },
        "form": { "type": "string", "enum": ["2551Q", "1601C", "1701MS"] }, "side": { "type": "string", "enum": ["collect", "pay"] },
        "tax_option": { "type": "string", "enum": ["graduated", "eight_percent"] }, "deduction": { "type": "string", "enum": ["osd", "itemized"] } } })
}

fn statement_draft_schema() -> Value {
    json!({ "type": "object", "required": ["client_request_id", "account", "month", "lines"], "properties": {
        "client_request_id": { "type": "string" }, "book": { "type": "string" },
        "account": { "type": "string", "description": "The bank, cash or card account's ticket." },
        "month": { "type": "string", "description": "YYYY-MM" },
        "evidence": { "type": "string", "description": "The attached statement's evidence ticket." },
        "closing": { "type": "string", "description": "The ending balance, two-place text." },
        "lines": { "type": "array", "minItems": 1, "maxItems": 2000, "items": { "type": "object", "required": ["date", "amount"], "properties": {
            "date": { "type": "string", "description": "YYYY-MM-DD" },
            "description": { "type": "string", "maxLength": 80 },
            "amount": { "type": "string", "description": "Two-place text; money into the account is positive." },
            "balance": { "type": "string", "description": "The running balance after the line, two-place text." },
            "page": { "type": "integer", "minimum": 1, "maximum": 500 },
            "bbox": { "type": "array", "items": { "type": "integer", "minimum": 0, "maximum": 1000 }, "minItems": 4, "maxItems": 4, "description": "left, top, right, bottom in thousandths of the page." },
            "confidence": { "type": "integer", "minimum": 0, "maximum": 100 } } } } } })
}

fn nudge_schema() -> Value {
    json!({ "type": "object", "properties": { "book": { "type": "string" }, "kind": { "type": "string", "enum": ["tidy", "month_end", "bir_pair", "review"] }, "month": { "type": "string" }, "any_day": { "type": "boolean", "description": "Answer on any day, not only on the routine's day." } } })
}

fn signin_schema() -> Value {
    json!({ "type": "object", "properties": { "step": { "type": "string", "enum": ["start", "poll", "out"] } } })
}

fn profile_link_schema() -> Value {
    json!({ "type": "object", "required": ["client_request_id"], "properties": {
        "client_request_id": { "type": "string" }, "book": { "type": "string" },
        "profile": { "type": "string", "description": "The tax profile's opaque id from the Buwiz list." },
        "unlink": { "type": "boolean" } } })
}

fn write_ref() -> Value {
    json!({
        "type": "object",
        "required": ["client_request_id", "entry"],
        "properties": { "client_request_id": { "type": "string" }, "book": { "type": "string" }, "entry": { "type": "string" } }
    })
}

fn void_schema() -> Value {
    let mut s = write_ref();
    s["properties"]["amend"] = json!({ "type": "boolean", "description": "Also save a replacement draft with the same lines, linked to the voided entry." });
    s["properties"]["reason"] = json!({ "type": "string", "description": "Why the entry is amended, 3 to 200 characters; needed with amend." });
    s["properties"]["date"] = json!({ "type": "string", "description": "YYYY-MM-DD for the reversal and the replacement (amend only; default the entry's date)." });
    s
}

fn owed_document_schema() -> Value {
    json!({
        "type": "object",
        "required": ["owed"],
        "properties": { "book": { "type": "string" }, "owed": { "type": "string", "description": "An owed document ticket." } },
        "additionalProperties": false
    })
}

fn book_create_schema() -> Value {
    json!({
        "type": "object",
        "required": ["client_request_id", "label"],
        "properties": {
            "client_request_id": { "type": "string" },
            "label": { "type": "string", "description": "The name shown in the Buwiz Bot; never a TIN." },
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

fn checks_schema() -> Value {
    json!({ "type": "object", "properties": { "book": { "type": "string" }, "check": { "type": "string", "enum": crate::findings::CHECKS } } })
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
        description: "Resolve the /books command for the Buwiz Bot.",
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
        schema: post_schema,
        description: "Post a draft (or save and post a new entry) after the user's click. Refuses an unbalanced entry, and receivable or payable lines on a cash book.",
    },
    Tool {
        name: VOID_ENTRY,
        reach: Reach::App,
        write: true,
        schema: void_schema,
        description: "Void a posted entry after the user's click: it shows Voided and a reversal is posted on its date. With amend, a replacement draft with the same lines is saved too.",
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
        description: "People of one kind (banks and cards, vendors, customers, employees, owners, lenders, government) with counts per kind. No TINs: eBIRForms keeps them.",
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
        description: "A report on the book's basis: P&L with the previous period, balance sheet, cash flow, trial balance, or aging (accrual books only). compare_to sets the period beside it (previous, last_year or none); tag narrows the P&L or cash flow to one department or location.",
    },
    Tool {
        name: REPORT_CSV,
        reach: Reach::Everyone,
        write: false,
        schema: report_schema,
        description: "The same report as CSV text, amounts as two-place decimal text; takes the same compare_to and tag.",
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
        schema: owed_settle_schema,
        description: "Settle an invoice or bill after the user's click, in full or by an amount: posts the money on that day.",
    },
    Tool {
        name: OWED_CANCEL,
        reach: Reach::App,
        write: true,
        schema: owed_ref,
        description: "Cancel an open invoice or bill after the user's click.",
    },
    Tool {
        name: RECONCILE_GRID,
        reach: Reach::Everyone,
        write: false,
        schema: book_only,
        description: "Bank, cash and card accounts by month: matched, draft or to do, with counts.",
    },
    Tool {
        name: STATEMENT_GET,
        reach: Reach::Everyone,
        write: false,
        schema: statement_ref,
        description: "One imported statement beside the ledger: each line's match (same, suggested, missing) and the difference.",
    },
    Tool {
        name: SETTINGS_GET,
        reach: Reach::Everyone,
        write: false,
        schema: book_only,
        description: "The book's settings: name, basis, fiscal year, months open or closed with their history, default categories, and what is linked.",
    },
    Tool {
        name: STATEMENT_IMPORT,
        reach: Reach::App,
        write: true,
        schema: statement_write,
        description: "Import a CSV bank or card statement for one account and month after the user picks the file.",
    },
    Tool {
        name: STATEMENT_ACCEPT,
        reach: Reach::App,
        write: true,
        schema: statement_write,
        description: "Accept every suggested match of a statement after the user's click.",
    },
    Tool {
        name: STATEMENT_ADD,
        reach: Reach::App,
        write: true,
        schema: statement_write,
        description: "Post a statement line that is not in Books, on its date, after the user's click.",
    },
    Tool {
        name: STATEMENT_FINISH,
        reach: Reach::App,
        write: true,
        schema: crate::statement_flags::finish_schema,
        description: "Finish a month when every line matches and the difference is 0.00, or reopen a finished one (`reopen`) with the user's reason.",
    },
    Tool {
        name: RECATEGORIZE,
        reach: Reach::App,
        write: true,
        schema: recategorize_schema,
        description: "Give a posted line on a not-sure-yet account its category, by a reclassifying entry on its date.",
    },
    Tool {
        name: MONTH_CLOSE,
        reach: Reach::App,
        write: true,
        schema: month_write,
        description: "Close a month after the user's click. A closed month refuses every post and void dated in it.",
    },
    Tool {
        name: MONTH_REOPEN,
        reach: Reach::App,
        write: true,
        schema: month_write,
        description: "Reopen a closed month with the user's reason.",
    },
    Tool {
        name: DEFAULT_SET,
        reach: Reach::App,
        write: true,
        schema: default_schema,
        description: "Set the account a default category uses.",
    },
    Tool {
        name: BOOK_UPDATE,
        reach: Reach::App,
        write: true,
        schema: book_update_schema,
        description: "Rename the book, move its fiscal year, or change its basis (only from the start of a year with nothing posted).",
    },
    Tool {
        name: SIGNIN,
        reach: Reach::App,
        write: true,
        schema: signin_schema,
        description: "Sign in to Buwiz with a device code (optional), check it, or sign out.",
    },
    Tool {
        name: TAX_PROFILES,
        reach: Reach::App,
        write: false,
        schema: book_only,
        description: "The approved Buwiz tax profiles you own: id, masked TIN and registered name only.",
    },
    Tool {
        name: PROFILE_LINK,
        reach: Reach::App,
        write: true,
        schema: profile_link_schema,
        description: "Link the book to one of your tax profiles by its id, or unlink it.",
    },
    Tool {
        name: SHOW,
        reach: Reach::Everyone,
        write: false,
        schema: show_schema,
        description: "Offer a Books card in the chat: answers (cash, month, person, unpaid, pl) show amounts because the user asked; actions end in the user's one click; quiet cards show counts only.",
    },
    Tool {
        name: NUDGE,
        reach: Reach::Everyone,
        write: false,
        schema: nudge_schema,
        description: "What a Books routine looks for: bank lines to categorize, last month left open, BIR hand-offs out of date. Counts only.",
    },
    Tool {
        name: HANDOFF_CREATE,
        reach: Reach::App,
        write: true,
        schema: handoff_create_schema,
        description: "Hand a ready form's input boxes to the BIR draft: records them for the owner to attach to the chat.",
    },
    Tool {
        name: HANDOFF_GET,
        reach: Reach::Everyone,
        write: false,
        schema: handoff_ref,
        description: "The input boxes of an attached Books proposal, to fill once into the BIR draft with source document. Refuses one the books no longer match.",
    },
    Tool {
        name: HANDOFF_CONFIRM,
        reach: Reach::App,
        write: true,
        schema: handoff_ref,
        description: "The owner's OK that the BIR draft shows the handed values.",
    },
    Tool {
        name: CROSS_CHECKS,
        reach: Reach::Everyone,
        write: false,
        schema: cross_checks_schema,
        description: "Cross-checks: quarters against the year, P&L sales against gross sales, pay runs against 1601-C, 2307s against 1701-MS, and filed returns you read from BIR against Books.",
    },
    Tool {
        name: UPLOAD_PART,
        reach: Reach::App,
        write: true,
        schema: upload_part_schema,
        description: "Send one part of a picked file (old books or a statement) for an import.",
    },
    Tool {
        name: IMPORT,
        reach: Reach::App,
        write: true,
        schema: import_schema,
        description: "Bring old books into a book with nothing posted: a web Buwiz Books export, or a CSV chart and journal.",
    },
    Tool {
        name: IMPORT_GET,
        reach: Reach::Everyone,
        write: false,
        schema: book_only,
        description: "What came in from old books, the account matches waiting for the owner, and opening totals.",
    },
    Tool {
        name: ACCOUNT_MATCH,
        reach: Reach::App,
        write: true,
        schema: account_match_schema,
        description: "The owner's match of one imported account to a kind, mixed or not sure yet.",
    },
    Tool {
        name: ACCOUNTS_CONFIRM,
        reach: Reach::App,
        write: true,
        schema: confirm_schema,
        description: "Confirm every suggested account match Books is sure of.",
    },
    Tool {
        name: OPENING_SET,
        reach: Reach::App,
        write: true,
        schema: opening_schema,
        description: "Enter (or delete) an opening total with its proof, for a book that starts mid-year.",
    },
    Tool {
        name: RETURN_OK,
        reach: Reach::App,
        write: true,
        schema: write_ref,
        description: "The owner's OK of a return event a form is waiting on.",
    },
    Tool {
        name: FORMS,
        reach: Reach::Everyone,
        write: false,
        schema: form_schema,
        description: "The three BIR forms Books feeds, with their period and status.",
    },
    Tool {
        name: FORM_PROPOSAL,
        reach: Reach::Everyone,
        write: false,
        schema: form_schema,
        description: "One form's Books totals beside its BIR input boxes. Pass the BIR choices you read as form_context.",
    },
    Tool {
        name: FORM_LINEAGE,
        reach: Reach::App,
        write: false,
        schema: form_schema,
        description: "The ledger lines behind one BIR box.",
    },
    Tool {
        name: CERTIFICATES,
        reach: Reach::Everyone,
        write: false,
        schema: certificates_schema,
        description: "The 2307s (and 2306s) customers sent, with status and what each counts toward.",
    },
    Tool {
        name: CERTIFICATE_ADD,
        reach: Reach::App,
        write: true,
        schema: certificate_add_schema,
        description: "Record a certificate as printed and post it against its sale.",
    },
    Tool {
        name: CERTIFICATE_SET,
        reach: Reach::App,
        write: true,
        schema: certificate_set_schema,
        description: "A certificate's paper arrived, or was lost or disputed.",
    },
    Tool {
        name: WAGES,
        reach: Reach::Everyone,
        write: false,
        schema: book_only,
        description: "Pay runs with gross pay and status. Amounts only.",
    },
    Tool {
        name: WAGE_RUN_SAVE,
        reach: Reach::App,
        write: true,
        schema: wage_run_schema,
        description: "Save a pay run as a draft entry; the Ledger posts it.",
    },
    Tool {
        name: EVIDENCE_ATTACH,
        reach: Reach::App,
        write: true,
        schema: evidence_tools::attach_schema,
        description: "Stage, retain or link evidence. App only.",
    },
    Tool {
        name: EVIDENCE_DETACH,
        reach: Reach::App,
        write: true,
        schema: evidence_tools::detach_schema,
        description: "Detach an evidence link; retain its contents. App only.",
    },
    Tool {
        name: EVIDENCE_LIST,
        reach: Reach::Everyone,
        write: false,
        schema: evidence_tools::list_schema,
        description: "Read evidence metadata and link counts, never contents.",
    },
    Tool {
        name: OWED_DRAFT,
        reach: Reach::Everyone,
        write: true,
        schema: owed_proposals::draft_schema,
        description: "Save the assistant's reviewable bill or invoice proposal from imported evidence. Creates no unpaid item, person or entry; the user accepts it in Unpaid.",
    },
    Tool {
        name: EVIDENCE_CLASSIFY,
        reach: Reach::Everyone,
        write: true,
        schema: document_review::classify_schema,
        description: "Propose what an evidence document is (statement, bill, invoice, receipt, payslip, contract, tax_form or other) with a whole-number confidence 0..100. A DRAFT proposal: the evidence kind changes only when the user accepts it in the app. Assistant only; takes a client_request_id.",
    },
    Tool {
        name: EVIDENCE_KIND,
        reach: Reach::App,
        write: true,
        schema: document_review::kind_schema,
        description: "Accept or discard a document-kind proposal. App only.",
    },
    Tool {
        name: EVIDENCE_PREVIEW,
        reach: Reach::App,
        write: false,
        schema: document_review::preview_schema,
        description: "A static print preview of one evidence document, optionally with its linked entry. App only.",
    },
    Tool {
        name: EVIDENCE_REVIEW,
        reach: Reach::App,
        write: false,
        schema: document_review::review_schema,
        description: "Document-kind proposals: generic kind, confidence and status. App only.",
    },
    Tool {
        name: STATEMENT_DRAFT_LINES,
        reach: Reach::Everyone,
        write: true,
        schema: statement_draft_schema,
        description: "Hand Books the lines you read from a statement the user attached, with page, box and confidence. Saved as a draft the user accepts in the app; it changes no statement.",
    },
    Tool {
        name: RECONCILE_DOCUMENT,
        reach: Reach::App,
        write: false,
        schema: reconcile_document_schema,
        description: "A static page for the preview: the reconciliation report or one statement page with its lines boxed. App only.",
    },
    Tool {
        name: CHECKS,
        reach: Reach::Everyone,
        write: false,
        schema: checks_schema,
        description: "Review findings to look at twice: uncategorized lines, stale drafts, months not matched to a statement, weekend dates, round amounts, money with no person. Counts, entry numbers and months only; never amounts, names or memos.",
    },
    Tool {
        name: OWED_DOCUMENT,
        reach: Reach::App,
        write: false,
        schema: owed_document_schema,
        description: "A print page (Statement of account or Bill record) for one owed document, shown by the Books app's print preview.",
    },
    Tool {
        name: OWED_REPEAT,
        reach: Reach::App,
        write: true,
        schema: owed_repeat_schema,
        description: "Start or stop a monthly repeat on an invoice or bill, or add this month's copy after the user's click.",
    },
];

fn owed_repeat_schema() -> Value {
    json!({
        "type": "object",
        "required": ["client_request_id", "owed", "action"],
        "properties": {
            "client_request_id": { "type": "string" },
            "book": { "type": "string" },
            "owed": { "type": "string" },
            "action": { "type": "string", "enum": ["start", "stop", "add"] },
            "month": { "type": "string", "description": "YYYY-MM, add only; this month when absent." }
        }
    })
}

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
                entry["_meta"] = json!({ "buwiz/audience": ["app"] });
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
    // a scope token for a period or form, passed as the assistant got it
    let unticked;
    let arguments = if books.exists()
        && ["period", "month", "form"].iter().any(|k| {
            arguments
                .get(*k)
                .and_then(Value::as_str)
                .is_some_and(books_store::plumbing::is_ticket)
        }) {
        unticked = match books.store() {
            Ok(store) => cards::untick(store.conn(), arguments),
            Err(_) => arguments.clone(),
        };
        &unticked
    } else {
        arguments
    };
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
                   OR (e.status = 'posted' AND e.form <> 'reversal'
                       AND e.recategorized_by_entry_id IS NULL AND e.recategorizes_entry_id IS NULL AND EXISTS (
                     SELECT 1 FROM line l JOIN account a ON a.account_id = l.account_id
                     WHERE l.entry_id = e.entry_id AND a.kind = 'unsure')))"
            )?);
            out["unpaid"] = json!(count(&format!(
                "SELECT count(*) FROM unpaid WHERE book_id = ?1 AND {}",
                books_store::people::STILL_OWED_SQL
            ))?);
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
            let list = records::owed_list(store, book.clone(), args, origin)?;
            let list = owed_proposals::decorate(store, &book, origin, list)?;
            owed_repeats::decorate(store, book, origin, list)
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
        OWED_CREATE => owed_proposals::owed_create(books, name, origin, arguments),
        OWED_SETTLE => write(books, name, origin, arguments, |tx| {
            records::owed_settle(tx, arguments)
        }),
        OWED_CANCEL => write(books, name, origin, arguments, |tx| {
            records::owed_cancel(tx, arguments)
        }),
        RECONCILE_GRID => read(books, arguments, |store, book, _| {
            reconcile::grid(store, book)
        }),
        STATEMENT_GET => read(books, arguments, |store, book, args| {
            reconcile::statement_view(store, book, args, origin)
        }),
        SETTINGS_GET => read(books, arguments, |store, book, _| {
            reconcile::settings(store, book)
        }),
        STATEMENT_IMPORT => {
            // a statement over one call's 256 KiB arrives in parts
            let mut args = arguments.clone();
            if service::text(arguments, "csv_upload").is_some() {
                match oldbooks::file_text(&mut books.uploads, arguments, "csv") {
                    Ok(Some(t)) => {
                        args.remove("csv_upload");
                        args.insert("csv_text".into(), json!(t));
                    }
                    Ok(None) => {}
                    Err(r) => return Some(r.into_result()),
                }
            }
            write(books, name, origin, &args, |tx| {
                reconcile::statement_import(tx, &args)
            })
        }
        STATEMENT_ACCEPT => write(books, name, origin, arguments, |tx| {
            if arguments.contains_key("draft") {
                return statement_read::accept(tx, arguments);
            }
            if crate::match_assist::is_one_line(arguments) {
                return crate::match_assist::accept_one(tx, arguments);
            }
            reconcile::statement_accept(tx, arguments)
        }),
        STATEMENT_ADD => write(books, name, origin, arguments, |tx| {
            reconcile::statement_add(tx, arguments)
        }),
        STATEMENT_FINISH => write(books, name, origin, arguments, |tx| {
            crate::statement_flags::finish_or_reopen(tx, arguments)
        }),
        RECATEGORIZE => write(books, name, origin, arguments, |tx| {
            reconcile::recategorize(tx, arguments)
        }),
        MONTH_CLOSE => write(books, name, origin, arguments, |tx| {
            reconcile::month_close(tx, arguments)
        }),
        MONTH_REOPEN => write(books, name, origin, arguments, |tx| {
            reconcile::month_reopen(tx, arguments)
        }),
        DEFAULT_SET => write(books, name, origin, arguments, |tx| {
            reconcile::default_set(tx, arguments)
        }),
        BOOK_UPDATE => write(books, name, origin, arguments, |tx| {
            reconcile::book_update(tx, arguments)
        }),
        UPLOAD_PART => answer(oldbooks::upload_part(&mut books.uploads, arguments)),
        IMPORT => {
            let plan = match oldbooks::files(&mut books.uploads, arguments)
                .and_then(|f| oldbooks::plan(arguments, f))
            {
                Ok(p) => p,
                Err(r) => return Some(r.into_result()),
            };
            write(books, name, origin, arguments, |tx| {
                oldbooks::run(tx, arguments, &plan)
            })
        }
        IMPORT_GET => read(books, arguments, |store, book, _| {
            oldbooks::status(store, book, origin)
        }),
        ACCOUNT_MATCH => write(books, name, origin, arguments, |tx| {
            oldbooks::account_match(tx, arguments)
        }),
        ACCOUNTS_CONFIRM => write(books, name, origin, arguments, |tx| {
            oldbooks::confirm_sure(tx, arguments)
        }),
        OPENING_SET => write(books, name, origin, arguments, |tx| {
            oldbooks::opening_set(tx, arguments)
        }),
        RETURN_OK => write(books, name, origin, arguments, |tx| {
            oldbooks::return_ok(tx, arguments)
        }),
        FORMS => read(books, arguments, |store, book, args| {
            forms::overview(store, book, args)
        }),
        FORM_PROPOSAL => read(books, arguments, |store, book, args| {
            forms::proposal(store, book, args, origin)
        }),
        FORM_LINEAGE => read(books, arguments, |store, book, args| {
            forms::lineage(store, book, args)
        }),
        CERTIFICATES => read(books, arguments, |store, book, args| {
            payroll::list_certificates(store, book, args, origin)
        }),
        CERTIFICATE_ADD => write(books, name, origin, arguments, |tx| {
            payroll::add_certificate(tx, arguments)
        }),
        CERTIFICATE_SET => write(books, name, origin, arguments, |tx| {
            payroll::set_certificate(tx, arguments)
        }),
        WAGES => read(books, arguments, |store, book, _| {
            payroll::list(store, book)
        }),
        WAGE_RUN_SAVE => write(books, name, origin, arguments, |tx| {
            payroll::save(tx, arguments)
        }),
        SIGNIN => answer((|| {
            let (store, session, transport) = books.cloud_parts()?;
            crate::cloud::signin(transport, store.conn(), session, arguments)
        })()),
        TAX_PROFILES => answer((|| {
            let (store, session, transport) = books.cloud_parts()?;
            let book = service::book(store, arguments)?;
            if !crate::cloud::signed_in(store.conn())? || transport.is_none() {
                return Ok(
                    json!({ "signed_in": false, "linked": book.cloud_tax_profile_id.is_some(), "profiles": [] }),
                );
            }
            let transport = transport.expect("checked");
            let mut list = crate::cloud::profiles(transport, store.conn(), session)?;
            for p in &mut list {
                p["linked"] = json!(book.cloud_tax_profile_id.as_deref() == p["id"].as_str());
            }
            Ok(
                json!({ "signed_in": true, "linked": book.cloud_tax_profile_id.is_some(), "profiles": list }),
            )
        })()),
        PROFILE_LINK => {
            let unlink = arguments.get("unlink") == Some(&Value::Bool(true));
            // a link names a profile the cloud lists for this user now
            let chosen = if unlink {
                None
            } else {
                let checked = (|| -> Result<String, Refusal> {
                    let id = service::required(arguments, "profile")?.to_lowercase();
                    let (store, session, transport) = books.cloud_parts()?;
                    let transport = transport.ok_or_else(|| {
                        Refusal::new("cloud_off", "Sign-in is not set up in this Books.")
                    })?;
                    let list = crate::cloud::profiles(transport, store.conn(), session)?;
                    if list.iter().any(|p| p["id"] == id.as_str()) {
                        Ok(id)
                    } else {
                        Err(Refusal::new(
                            "not_found",
                            "That tax profile is not one of your approved profiles.",
                        ))
                    }
                })();
                match checked {
                    Ok(id) => Some(id),
                    Err(r) => return Some(r.into_result()),
                }
            };
            write(books, name, origin, arguments, |tx| {
                let book = writes::book_in(tx, arguments)?;
                books_store::link::link_profile(tx, &book, chosen.as_deref())?;
                Ok(service::with_subject(
                    json!({ "linked": chosen.is_some() }),
                    "book",
                    book.id,
                ))
            })
        }
        SHOW => {
            let mut out = read(books, arguments, |store, book, args| {
                cards::show(store, book, args)
            });
            // the card offer rides on the result's _meta (spec 9.6), never in the text
            if let Some(open) = out["structuredContent"]
                .as_object_mut()
                .and_then(|o| o.remove("_open"))
            {
                out["content"] =
                    json!([{ "type": "text", "text": out["structuredContent"].to_string() }]);
                out["_meta"] = json!({ "buwiz/open": open });
            }
            out
        }
        NUDGE => read(books, arguments, |store, book, args| {
            cards::nudge(store, book, args)
        }),
        HANDOFF_CREATE => write(books, name, origin, arguments, |tx| {
            handoff::create(tx, arguments)
        }),
        HANDOFF_GET => read(books, arguments, |store, book, args| {
            handoff::get(store, book, args, origin)
        }),
        HANDOFF_CONFIRM => write(books, name, origin, arguments, |tx| {
            handoff::confirm(tx, arguments)
        }),
        CROSS_CHECKS => read(books, arguments, |store, book, args| {
            checks::cross_checks(store, book, args)
        }),
        EVIDENCE_ATTACH => evidence_tools::attach(books, name, origin, arguments),
        EVIDENCE_DETACH => evidence_tools::detach(books, name, origin, arguments),
        EVIDENCE_LIST => read(books, arguments, |store, _book, args| {
            evidence_tools::list(store.conn(), args)
        }),
        OWED_DRAFT => owed_proposals::draft_tool(books, name, origin, arguments),
        EVIDENCE_CLASSIFY => document_review::classify(books, name, origin, arguments),
        EVIDENCE_KIND => document_review::decide(books, name, origin, arguments),
        EVIDENCE_PREVIEW => document_review::preview(books, origin, arguments),
        EVIDENCE_REVIEW => read(books, arguments, |store, _book, args| {
            document_review::review(store.conn(), args)
        }),
        STATEMENT_DRAFT_LINES => write(books, name, origin, arguments, |tx| {
            statement_read::draft_lines(tx, arguments, origin)
        }),
        RECONCILE_DOCUMENT => {
            if !origin.is_app() {
                return Some(fail("forbidden", "Only the app may open this page."));
            }
            if !books.exists() {
                return Some(fail(
                    "not_set_up",
                    "There is no book yet. Open Books to make one.",
                ));
            }
            (|| {
                let store = books.store()?;
                let book = service::book(store, arguments)?;
                reconcile_document::document(store, book, arguments)
            })()
            .unwrap_or_else(Refusal::into_result)
        }
        CHECKS => read(books, arguments, |store, book, args| {
            crate::findings::checks(store, book, args)
        }),
        OWED_DOCUMENT => crate::owed_doc::tool(books, arguments, origin),
        OWED_REPEAT => write(books, name, origin, arguments, |tx| {
            owed_repeats::repeat(tx, arguments)
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

/// The `proposal` scope: a hand-off ticket resolves to its token; no value
/// lists the current hand-offs.
fn proposal_scope(arguments: &Map<String, Value>, books: &mut Books) -> Value {
    if !books.exists() {
        return json!({ "result": "error", "message": "There is no book yet." });
    }
    let Ok(store) = books.store() else {
        return json!({ "result": "error", "message": "Books could not open right now." });
    };
    let value = arguments.get("value").and_then(Value::as_str).unwrap_or("");
    if !value.is_empty() {
        return match handoff::ticket_title(store, value) {
            Some(title) => json!({ "result": "token", "ticket": value, "title": title }),
            None => json!({ "result": "error", "message": "That is not a Books hand-off." }),
        };
    }
    match handoff::choices(store) {
        Ok(c) if !c.is_empty() => json!({ "result": "choices", "choices": c }),
        Ok(_) => {
            json!({ "result": "error", "message": "No hand-off yet. Open Forms in Books and click Hand to BIR draft." })
        }
        Err(_) => {
            json!({ "result": "error", "message": "Books could not list hand-offs right now." })
        }
    }
}

/// Spec 9.2: `{kind, name, value, complete, context}` in, one `result` out.
fn resolve(arguments: &Map<String, Value>, books: &mut Books) -> Value {
    let kind = arguments.get("kind").and_then(Value::as_str).unwrap_or("");
    let name = arguments.get("name").and_then(Value::as_str).unwrap_or("");
    let value = arguments.get("value").and_then(Value::as_str).unwrap_or("");
    let complete = arguments.get("complete") == Some(&Value::Bool(true));
    let home = |books: &mut Books, props: Value| match books.home_ticket() {
        Ok(ticket) => ok(json!({
            "result": "open",
            "app": "books",
            "props": props,
            "ticket": ticket,
            "title": service::HOME_TITLE,
            "placement": "full",
        })),
        Err(_) => fail("internal", "Books could not open right now."),
    };
    match (kind, name) {
        ("command", "books") => {
            // a page chosen from the choices comes back as its ticket
            if let Some(page) = plumbing_page(books, value) {
                return home(books, json!({ "page": page }));
            }
            if complete
                && books.exists()
                && let Ok(store) = books.store()
                && let Ok(choices) = cards::page_choices(store, value)
                && !choices.is_empty()
            {
                return ok(json!({ "result": "choices", "choices": choices }));
            }
            match cards::command_open(name, value) {
                Some(open) if open["app"] == "books" => home(books, open["props"].clone()),
                Some(open) => ok(card_result(open)),
                None => home(books, json!({ "page": "today" })),
            }
        }
        (
            "command",
            "books-receipt" | "books-tidy" | "books-close" | "books-quarter" | "books-results",
        ) => {
            if !books.exists() {
                return ok(
                    json!({ "result": "error", "message": "There is no book yet. Type /books to make one." }),
                );
            }
            match cards::command_open(name, value) {
                Some(open) => ok(card_result(open)),
                None => ok(
                    json!({ "result": "error", "message": "Books does not know that template." }),
                ),
            }
        }
        ("scope", "proposal") => ok(proposal_scope(arguments, books)),
        ("scope", "person" | "account" | "period" | "entry" | "form" | "tag" | "evidence") => {
            if !books.exists() {
                return ok(json!({ "result": "error", "message": "There is no book yet." }));
            }
            match books.store() {
                Ok(store) => ok(cards::scope(store, name, value)),
                Err(_) => {
                    ok(json!({ "result": "error", "message": "Books could not open right now." }))
                }
            }
        }
        _ => ok(
            json!({ "result": "error", "message": "Books does not know that command or scope." }),
        ),
    }
}

/// A card offer as a resolve `open` (spec 9.2).
fn card_result(open: Value) -> Value {
    let mut out = json!({ "result": "open", "app": open["app"], "props": open["props"], "title": open["title"], "placement": "inline" });
    if let Some(t) = open.get("ticket") {
        out["ticket"] = t.clone();
    }
    out
}

/// The page a `/books` choice's ticket names, if it is one.
fn plumbing_page(books: &mut Books, value: &str) -> Option<&'static str> {
    if !books_store::plumbing::is_ticket(value) || !books.exists() {
        return None;
    }
    let store = books.store().ok()?;
    match books_store::plumbing::subject(store.conn(), value).ok()?? {
        (k, id) if k == "page" => cards::page_of(id),
        _ => None,
    }
}
