//! The 2.0 manifest (spec 5) for `app.buwiz.books`.
//!
//! [`unsigned`] builds it from this build's code: every app's fingerprint is
//! the SHA-256 of the source bytes `resources/read` answers ([`crate::apps`]),
//! and `permissions.tools` names the tools each app's source calls. The
//! `manifest` subcommand prints it as JCS text; the owner signs it at release
//! (`sign-manifest.mjs --keychain gc-2026-01`), and a test run signs it with a
//! throwaway key under a test key id (`tools/bot/conformance.sh`).
//!
//! The signed file is what this process serves (`BOOKS_V1_MANIFEST`), byte
//! for byte, read again on every request. Without it the unsigned manifest
//! is served, and the Bot refuses it: only a signed Books runs in the Bot.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};

use crate::{apps, jcs, tools};

pub const PROTOCOL: &str = "2.0.0";
pub const PLUGIN_ID: &str = "app.buwiz.books";
pub const SHORT: &str = "books";
pub const VERSION: &str = "0.2.0";
pub const PUBLISHER_ID: &str = "app.buwiz";
/// The release key (owner decision D9). Tests sign under a test key id.
pub const RELEASE_KEY_ID: &str = "gc-2026-01";
pub const MANIFEST_URI: &str = "buwiz://manifest";
pub const MANIFEST_MIME: &str = "application/json";
/// The signed manifest file this process serves.
pub const MANIFEST_ENV: &str = "BOOKS_V1_MANIFEST";
/// The props key an app takes its subject from.
pub const TICKET_PROP: &str = "ticket";

/// `sha256:<hex>` of an app source as served (spec 6.2).
pub fn fingerprint(source: &str) -> String {
    format!("sha256:{}", jcs::sha256_hex(source.as_bytes()))
}

/// What each app may reach (spec 5.6). `tools` is exactly the Books tools
/// its source names (checked in `tests/manifest_shape.rs`).
fn permissions(app: &apps::App) -> Value {
    let mut p = json!({ "tools": apps::named_tools(app.source) });
    // the window copies a report's CSV to the clipboard (M3 export)
    if app.source.contains("buwiz.copy(") {
        p["copy"] = json!(true);
    }
    // Reconcile picks a CSV bank statement (M4); Import picks old books, a
    // web Books export or CSV files (M5). Sources split `buwiz` and the
    // method across lines, so the method's own name is the signal.
    if app.source.contains(".pickFile(") {
        p["pick_file"] = json!({ "extensions": [".csv", ".json"], "max_bytes": 52_428_800 });
    }
    // Forms attaches a hand-off to the chat (M7)
    if app.source.contains(".attach(") {
        p["attach"] = json!(["proposal"]);
    }
    // a card opens the Books window at a page (M8)
    if app.id != "books" && app.source.contains(".open({") {
        p["open"] = json!({ "apps": ["books"] });
    }
    p
}

fn app_entry(app: &apps::App) -> Value {
    match app.id {
        "books" => json!({
            "id": "books",
            // neutral: no business name, amount or TIN (D1, RK-10)
            "title": "Books",
            "description": "Your books: today, the ledger, unpaid invoices and bills, reports, people, accounts and tags. Only you post, void or close here.",
            "entry": format!("{}{}", apps::URI_PREFIX, app.id),
            "mime": apps::MIME,
            "sdk": "1",
            "fingerprint": fingerprint(app.source),
            "props": {
                "type": "object",
                "properties": {
                    TICKET_PROP: { "type": "string", "maxLength": 30 },
                    "page": { "type": "string", "enum": ["today", "ledger", "unpaid", "reconcile", "reports", "forms", "people", "accounts", "tags", "payroll", "certificates", "settings", "import"] },
                    // the BIR choices the assistant read from the BIR draft (owner decision): shown, never set
                    "form": { "type": "string", "enum": ["2551Q", "1601C", "1701MS"] },
                    "tax_option": { "type": "string", "enum": ["graduated", "eight_percent"] },
                    "deduction": { "type": "string", "enum": ["osd", "itemized"] },
                    "account": { "type": "string", "maxLength": 30 },
                    "month": { "type": "string", "maxLength": 7 },
                    "filter": { "type": "string", "enum": ["all", "review", "draft", "posted"] }
                }
            },
            "placement": ["full"],
            "permissions": permissions(app),
        }),
        "home" => json!({
            "id": "home",
            // neutral: no business name, amount or TIN (D1, RK-10)
            "title": "Books",
            "description": "Books home: setup status and counts only.",
            "entry": format!("{}{}", apps::URI_PREFIX, app.id),
            "mime": apps::MIME,
            "sdk": "1",
            "fingerprint": fingerprint(app.source),
            "props": {
                "type": "object",
                "properties": { TICKET_PROP: { "type": "string", "maxLength": 30 } }
            },
            "placement": ["full", "inline"],
            "inline": { "width": 360, "height": 120, "min_height": 96, "max_height": 240, "resizable": false, "wait_ready": true },
            "permissions": permissions(app),
        }),
        id => {
            let (title, description, height, max_height) = card_kind(id);
            json!({
                "id": id,
                // neutral: no business name, amount or TIN (D1, RK-10)
                "title": title,
                "description": description,
                "entry": format!("{}{}", apps::URI_PREFIX, app.id),
                "mime": apps::MIME,
                "sdk": "1",
                "fingerprint": fingerprint(app.source),
                "props": {
                    "type": "object",
                    "properties": {
                        TICKET_PROP: { "type": "string", "maxLength": 30 },
                        "card": { "type": "string", "enum": crate::cards::cards_of(id) },
                        "person": { "type": "string", "maxLength": 30 },
                        "account": { "type": "string", "maxLength": 30 },
                        "entry": { "type": "string", "maxLength": 30 },
                        "owed": { "type": "string", "maxLength": 30 },
                        "certificate": { "type": "string", "maxLength": 30 },
                        "month": { "type": "string", "maxLength": 7 },
                        "period": { "type": "string", "maxLength": 7 },
                        "form": { "type": "string", "enum": ["2551Q", "1601C", "1701MS"] },
                        "side": { "type": "string", "enum": ["collect", "pay"] },
                        "tax_option": { "type": "string", "enum": ["graduated", "eight_percent"] },
                        "deduction": { "type": "string", "enum": ["osd", "itemized"] }
                    }
                },
                "placement": ["inline"],
                // The Bot holds its placeholder until the card's reads are in (`buwiz.ready`)
                "inline": { "width": CARD_WIDTH, "height": height, "min_height": 96, "max_height": max_height, "resizable": false, "wait_ready": true },
                "permissions": permissions(app),
            })
        }
    }
}

/// Every inline card's width: the Bot's agent column, the width of its own cards
/// (buwiz app/src/cards/mod.rs `shell`), so the chat's cards line up.
const CARD_WIDTH: u32 = 560;

/// A card app's title, description, first height and max height
/// (docs/design/inline-cards.md). The card reports its natural height with
/// buwiz.fit; its parts are bounded (a list scrolls inside 240 points), so the
/// height stays under the max and the footer is never cut off.
fn card_kind(id: &str) -> (&'static str, &'static str, u32, u32) {
    match id {
        "answer" => (
            "Books answer",
            "A Books answer in the chat: cash, this month, a person, unpaid. Amounts only because you asked.",
            180,
            440,
        ),
        "report" => (
            "Books report",
            "A Books report card in the chat: profit and loss, this month against last.",
            300,
            460,
        ),
        "action" => (
            "Books action",
            "One click to finish in the chat: post a draft, categorize bank lines, mark paid, match the bank, close a month, a 2307 received.",
            220,
            460,
        ),
        "pair" => (
            "Books for BIR",
            "A form's Books totals for eBIRForms: a return to confirm, ready, hand to the BIR draft.",
            280,
            500,
        ),
        "quiet" => (
            "Books note",
            "A quiet Books card from a routine: counts and status only.",
            120,
            200,
        ),
        other => unreachable!("no manifest entry for app {other}"),
    }
}

/// The manifest without its signature, its publisher key id `key_id`
/// ([`RELEASE_KEY_ID`] for release; a test id for local runs).
pub fn unsigned(key_id: &str) -> Value {
    json!({
        "protocol": PROTOCOL,
        "id": PLUGIN_ID,
        "short": SHORT,
        "version": VERSION,
        "title": "Buwiz Books",
        "description": "Bookkeeping on this computer, cash basis by default. Books records what happened; eBIRForms computes and files tax.",
        "publisher": { "id": PUBLISHER_ID, "key_id": key_id, "name": "Buwiz" },
        "commands": [
            { "name": "books", "description": "Open your books: a page (ledger, unpaid, reconcile, forms…) or a card (cash, month, unpaid, pl)", "args": "[page or card]", "resolve": tools::RESOLVE },
            // templates (TMP): starter flows, each a command that opens its card (M8)
            { "name": "books-receipt", "description": "Record a receipt: attach the photo or PDF, the assistant drafts it, you post", "resolve": tools::RESOLVE },
            { "name": "books-tidy", "description": "Tidy up the bank lines: pick a category for each", "resolve": tools::RESOLVE },
            { "name": "books-close", "description": "Close last month: the checks, then close", "resolve": tools::RESOLVE },
            { "name": "books-quarter", "description": "Get the quarter ready for BIR: returns to confirm, then hand the totals over", "resolve": tools::RESOLVE },
            { "name": "books-results", "description": "How did we do? This month against last", "resolve": tools::RESOLVE }
        ],
        "scopes": crate::cards::scopes(),
        "routines": crate::cards::routines(),
        "apps": apps::APPS.iter().map(app_entry).collect::<Vec<_>>(),
    })
}

/// [`unsigned`] as JCS text, the generator's output.
pub fn unsigned_jcs(key_id: &str) -> String {
    jcs::canonical(&unsigned(key_id)).expect("the manifest carries integers only")
}

/// Where the served manifest comes from.
#[derive(Debug, Clone, Default)]
pub struct Source {
    /// The signed file; `None` serves [`unsigned`] under the release key id.
    pub signed: Option<PathBuf>,
}

impl Source {
    /// `BOOKS_V1_MANIFEST`, when set and not empty.
    pub fn from_env() -> Self {
        Self {
            signed: std::env::var_os(MANIFEST_ENV)
                .filter(|path| !path.is_empty())
                .map(PathBuf::from),
        }
    }

    /// The manifest text this process serves.
    pub fn text(&self) -> Result<String, String> {
        match &self.signed {
            Some(path) => read(path),
            None => Ok(unsigned_jcs(RELEASE_KEY_ID)),
        }
    }

    /// The served manifest, parsed as I-JSON.
    pub fn value(&self) -> Result<Value, String> {
        let value = jcs::parse(&self.text()?).map_err(|error| format!("the manifest: {error}"))?;
        if value.is_object() {
            Ok(value)
        } else {
            Err("the manifest is not a JSON object".into())
        }
    }

    /// `manifest_sha256` (spec 5.2 step 7): SHA-256 of the served manifest's JCS bytes.
    pub fn sha256(&self) -> Result<String, String> {
        jcs::sha256_jcs(&self.value()?).map_err(|error| error.to_string())
    }

    /// One app of the served manifest.
    pub fn app(&self, id: &str) -> Result<Option<Value>, String> {
        Ok(self.value()?["apps"]
            .as_array()
            .and_then(|apps| apps.iter().find(|app| app["id"] == id))
            .cloned())
    }
}

fn read(path: &Path) -> Result<String, String> {
    // the path is the operator's; the error names the variable, not the file
    std::fs::read_to_string(path).map_err(|error| format!("{MANIFEST_ENV}: {error}"))
}

/// `sha256(JCS(permissions))`, an absent object being `{}` (spec 4.4).
pub fn permissions_digest(app: &Value) -> Result<String, String> {
    let empty = Value::Object(Map::new());
    jcs::sha256_jcs(app.get("permissions").unwrap_or(&empty)).map_err(|error| error.to_string())
}
