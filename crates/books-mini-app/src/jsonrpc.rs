//! MCP over stdio: JSON-RPC 2.0, one message per line (spec 14.3).
//!
//! Methods: `initialize`, `ping`, `tools/list`, `tools/call`,
//! `resources/list`, `resources/templates/list`, `resources/read`.
//! Notifications get no answer. Resources (the manifest and the app
//! sources) are readable before `buwiz.provision`; tools are not (P7).
//!
//! Nothing is logged: no argument, no subject and never the key (P4).

use std::io::{BufRead, Write};

use serde_json::{Map, Value, json};

use crate::manifest::{self, Source};
use crate::reserved::{self, Host};
use crate::service::{Books, Origin};
use crate::{apps, tools};

pub const SERVER_NAME: &str = "books-mini-app";

pub const PARSE_ERROR: i64 = -32700;
pub const INVALID_REQUEST: i64 = -32600;
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INVALID_PARAMS: i64 = -32602;

/// MCP protocol revisions this server speaks; the first is the latest.
const SUPPORTED_PROTOCOLS: [&str; 4] = ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

const INSTRUCTIONS: &str = "Buwiz Books keeps a small business's books on this computer, on \
cash basis by default. The assistant can read totals and draft entries (books_entry_draft); only \
the user posts, voids or closes, in the Books app. Books never computes tax, never stores a TIN, \
and never sends, files or pays anything.";

// Result shapes are the SDK's: `structuredContent` for `ok`, a
// `code`/`message` refusal for `fail` (spec 14.1).
pub use buwiz_plugin::{fail, ok};

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

struct Caller {
    origin: Origin,
    app: Option<String>,
    open_id: Option<String>,
    /// The routine whose turn this is (spec 5.9), if any.
    routine: Option<String>,
}

impl Caller {
    fn of(params: &Value) -> Self {
        let caller = params
            .get("_meta")
            .and_then(|meta| meta.get("buwiz/caller"));
        let text = |key: &str| {
            caller
                .and_then(|c| c.get(key))
                .and_then(Value::as_str)
                .map(str::to_string)
        };
        let origin = match text("origin").as_deref() {
            Some("app") => Origin::App,
            Some("agent") => Origin::Agent,
            Some("routine") => Origin::Routine,
            Some("resolve") => Origin::Resolve,
            _ => Origin::Unknown,
        };
        Self {
            origin,
            app: text("app"),
            open_id: text("open_id"),
            routine: text("routine"),
        }
    }
}

/// The server's state: the reserved tools' key and opens, and the books.
pub struct Server {
    source: Source,
    host: Host,
    books: Books,
}

impl Server {
    pub fn new(source: Source, books: Books) -> Self {
        Self {
            source,
            host: Host::default(),
            books,
        }
    }

    /// One line in; the line to answer, if any.
    pub fn handle_line(&mut self, line: &str) -> Option<String> {
        if line.trim().is_empty() {
            return None;
        }
        let reply = match serde_json::from_str::<Value>(line) {
            Ok(message) => self.handle(&message),
            Err(_) => Some(error_response(Value::Null, PARSE_ERROR, "parse error")),
        };
        reply.map(|value| value.to_string())
    }

    /// One JSON-RPC message; `None` for a notification.
    pub fn handle(&mut self, message: &Value) -> Option<Value> {
        let Some(object) = message.as_object() else {
            return Some(error_response(
                Value::Null,
                INVALID_REQUEST,
                "expected a single JSON-RPC object",
            ));
        };
        let id = object.get("id").cloned();
        let Some(method) = object.get("method").and_then(Value::as_str) else {
            return id.map(|id| error_response(id, INVALID_REQUEST, "missing method"));
        };
        let id = id?;
        let params = object.get("params").cloned().unwrap_or(Value::Null);
        let outcome = match method {
            "initialize" => Ok(initialize(&params)),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(tool_list()),
            "tools/call" => self.tools_call(&params),
            "resources/list" => Ok(resource_list()),
            "resources/templates/list" => Ok(json!({ "resourceTemplates": [] })),
            "resources/read" => self.resource_read(&params),
            other => {
                let shown: String = other.chars().take(60).collect();
                Err((METHOD_NOT_FOUND, format!("method not found: {shown}")))
            }
        };
        Some(match outcome {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Err((code, message)) => error_response(id, code, &message),
        })
    }

    fn tools_call(&mut self, params: &Value) -> Result<Value, (i64, String)> {
        let name = params.get("name").and_then(Value::as_str).ok_or((
            INVALID_PARAMS,
            "tools/call requires params.name".to_string(),
        ))?;
        let empty = Map::new();
        let arguments = match params.get("arguments") {
            None | Some(Value::Null) => &empty,
            Some(Value::Object(arguments)) => arguments,
            Some(_) => return Ok(fail("invalid_args", "arguments must be a JSON object")),
        };
        // reserved tools first; an unknown buwiz.* is method-not-found (7.9)
        if name.starts_with("buwiz.") {
            let now = now_ms();
            return match self
                .host
                .call(&self.source, &mut self.books, name, arguments, now)
            {
                None => Err((
                    METHOD_NOT_FOUND,
                    "not a reserved tool of this plugin".into(),
                )),
                Some(Err(message)) => Err((INVALID_REQUEST, message)),
                Some(Ok(result)) => Ok(result),
            };
        }
        // P7: nothing before a successful provision
        if !self.host.provisioned() {
            return Err((INVALID_REQUEST, "buwiz.provision must come first".into()));
        }
        if tools::find(name).is_none() {
            return Err((INVALID_PARAMS, "no such tool".into()));
        }
        let caller = Caller::of(params);
        if let Err(message) = self.authorize(name, &caller) {
            return Ok(fail("permission_denied", &message));
        }
        Ok(tools::call(name, arguments, caller.origin, &mut self.books).expect("a listed tool"))
    }

    /// The origin checks Books owns (docs/01 §4.4), as BIR makes them.
    fn authorize(&self, name: &str, caller: &Caller) -> Result<(), String> {
        let tool = tools::find(name).ok_or("no such tool")?;
        // DD-20: post, void, close, reopen, confirm and hand-off are the
        // user's clicks in the Books app
        if tool.reach == tools::Reach::App && caller.origin != Origin::App {
            return Err(format!(
                "{name} needs the user's click in the Books app; the assistant can save a draft instead"
            ));
        }
        // P6: resolve and routine calls only read
        if matches!(caller.origin, Origin::Resolve | Origin::Routine) && !tools::read_only(name) {
            return Err(format!(
                "{name} changes Books data, so it does not run here"
            ));
        }
        // a routine's turn writes only what the routine declares, and Books
        // lets a routine declare drafts only (P6, M8)
        if let Some(routine) = &caller.routine
            && !tools::read_only(name)
            && !crate::cards::routine_writes(routine)
                .iter()
                .any(|w| w == name)
        {
            return Err(format!("{name} is not a write this routine declares"));
        }
        if caller.origin != Origin::App {
            return Ok(());
        }
        let app = caller.app.as_deref().ok_or("an app call names no app")?;
        let open = caller.open_id.as_deref().unwrap_or("");
        if self.host.session_app(open) != Some(app) {
            return Err("this app open was not certified by Books".into());
        }
        let allowed = self
            .source
            .app(app)?
            .and_then(|entry| entry["permissions"]["tools"].as_array().cloned())
            .unwrap_or_default();
        if allowed.iter().any(|tool| tool == name) {
            Ok(())
        } else {
            Err(format!("{name} is not in this app's permissions"))
        }
    }

    fn resource_read(&self, params: &Value) -> Result<Value, (i64, String)> {
        let uri = params.get("uri").and_then(Value::as_str).ok_or((
            INVALID_PARAMS,
            "resources/read requires params.uri".to_string(),
        ))?;
        if uri == manifest::MANIFEST_URI {
            let text = self
                .source
                .text()
                .map_err(|error| (INVALID_PARAMS, error))?;
            return Ok(
                json!({ "contents": [{ "uri": uri, "mimeType": manifest::MANIFEST_MIME, "text": text }] }),
            );
        }
        let app = apps::by_uri(uri).ok_or((INVALID_PARAMS, "no such resource".to_string()))?;
        Ok(json!({ "contents": [{ "uri": uri, "mimeType": apps::MIME, "text": app.source }] }))
    }
}

fn initialize(params: &Value) -> Value {
    let requested = params.get("protocolVersion").and_then(Value::as_str);
    let version = requested
        .filter(|version| SUPPORTED_PROTOCOLS.contains(version))
        .unwrap_or(SUPPORTED_PROTOCOLS[0]);
    json!({
        "protocolVersion": version,
        "capabilities": {
            "tools": { "listChanged": false },
            "resources": { "listChanged": false, "subscribe": false },
        },
        "serverInfo": { "name": SERVER_NAME, "version": env!("CARGO_PKG_VERSION") },
        "instructions": INSTRUCTIONS,
    })
}

fn tool_list() -> Value {
    let mut list: Vec<Value> = reserved::RESERVED
        .iter()
        .map(|name| json!({ "name": name, "description": "Reserved for the Buwiz Bot.", "inputSchema": { "type": "object" } }))
        .collect();
    list.extend(tools::list());
    json!({ "tools": list })
}

fn resource_list() -> Value {
    let mut resources = vec![json!({
        "uri": manifest::MANIFEST_URI,
        "name": "manifest",
        "description": "The Mini App Protocol 2.0 manifest.",
        "mimeType": manifest::MANIFEST_MIME,
    })];
    for app in apps::APPS {
        resources.push(json!({
            "uri": format!("{}{}", apps::URI_PREFIX, app.id),
            "name": app.id,
            "mimeType": apps::MIME,
        }));
    }
    json!({ "resources": resources })
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_millis()).ok())
        .unwrap_or(0)
}

/// Serve stdin to stdout until stdin closes.
pub fn serve_stdio(server: &mut Server) -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        if let Some(reply) = server.handle_line(&line?) {
            stdout.write_all(reply.as_bytes())?;
            stdout.write_all(b"\n")?;
            stdout.flush()?;
        }
    }
    Ok(())
}
