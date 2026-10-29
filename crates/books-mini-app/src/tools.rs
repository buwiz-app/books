//! Books' own tools. Every name starts with `books_` (spec 5.3; RK-5: a
//! tool that does not is skipped silently by Rox's agent surface).
//!
//! P0 has two, both read-only (P8):
//! - `books_status`: setup status and counts, never an amount, TIN or name;
//! - `books_resolve`: the `/books` command (spec 9.2).

use serde_json::{Map, Value, json};

use crate::jsonrpc::{fail, ok};
use crate::manifest;
use crate::tickets::{Subject, Tickets};

pub const STATUS: &str = "books_status";
pub const RESOLVE: &str = "books_resolve";

/// Every Books tool, in `tools/list` order.
pub const TOOLS: [&str; 2] = [STATUS, RESOLVE];

/// Whether `name` changes nothing (`readOnlyHint`, P8).
pub fn read_only(name: &str) -> bool {
    TOOLS.contains(&name)
}

/// The `tools/list` entries.
pub fn list() -> Vec<Value> {
    vec![
        json!({
            "name": STATUS,
            "description": "Report Buwiz Books' setup status on this computer: the plugin version and how many books exist. Counts and status only; never amounts, names or TINs.",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false },
            "annotations": { "readOnlyHint": true },
        }),
        json!({
            "name": RESOLVE,
            "description": "Resolve the /books command for Rox.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "kind": { "type": "string", "enum": ["command", "scope"] },
                    "name": { "type": "string" },
                    "value": { "type": "string" },
                    "complete": { "type": "boolean" },
                    "context": { "type": "object" }
                }
            },
            "annotations": { "readOnlyHint": true },
        }),
    ]
}

/// Run a Books tool. `None`: no such tool.
pub fn call(name: &str, arguments: &Map<String, Value>, tickets: &mut Tickets) -> Option<Value> {
    match name {
        STATUS => Some(status()),
        RESOLVE => Some(resolve(arguments, tickets)),
        _ => None,
    }
}

/// P0 has no ledger yet: no book exists.
fn status() -> Value {
    ok(json!({
        "plugin": manifest::PLUGIN_ID,
        "version": manifest::VERSION,
        "status": "not_set_up",
        "books": 0,
    }))
}

/// Spec 9.2: `{kind, name, value, complete, context}` in, one `result` out.
fn resolve(arguments: &Map<String, Value>, tickets: &mut Tickets) -> Value {
    let kind = arguments.get("kind").and_then(Value::as_str).unwrap_or("");
    let name = arguments.get("name").and_then(Value::as_str).unwrap_or("");
    match (kind, name) {
        ("command", "books") => match tickets.issue(Subject::Home) {
            Ok(ticket) => ok(json!({
                "result": "open",
                "app": "home",
                "props": {},
                "ticket": ticket,
                "title": Subject::Home.title(),
                "placement": "full",
            })),
            Err(_) => fail("internal", "Books could not open right now."),
        },
        _ => ok(
            json!({ "result": "error", "message": "Books does not know that command or scope." }),
        ),
    }
}
