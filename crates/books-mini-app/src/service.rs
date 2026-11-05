//! What every Books tool shares: the store (opened on first use), the book
//! a call is about, tickets in place of identifiers, and writes that are
//! idempotent (`client_request_id`), audited and one transaction each.

use books_store::books::{self, Account, Book};
use books_store::plumbing;
use books_store::rusqlite::Transaction;
use books_store::{Paths, Store, StoreError};
use serde_json::{Map, Value, json};

use crate::jcs;
use crate::jsonrpc::{fail, ok};

/// Who a `tools/call` comes from (`_meta["buwiz/caller"]`, protocol 1.0 §16.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    App,
    Agent,
    Routine,
    Resolve,
    Unknown,
}

impl Origin {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Agent => "agent",
            Self::Routine => "routine",
            Self::Resolve => "resolve",
            Self::Unknown => "unknown",
        }
    }

    /// The app sees names, memos and lines; the model sees totals and
    /// tickets.
    pub fn is_app(self) -> bool {
        self == Self::App
    }
}

/// A refusal: a spec error code and a user-facing sentence (no subject data).
#[derive(Debug, Clone)]
pub struct Refusal {
    pub code: String,
    pub message: String,
    /// What the refusal is about, for a batch: an entry's number, never its
    /// content. Sent as `refused` beside the code and the message.
    pub refused: Option<String>,
}

impl Refusal {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            refused: None,
        }
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new("invalid_args", message)
    }

    /// The same refusal, naming what it is about.
    #[must_use]
    pub fn about(mut self, subject: impl Into<String>) -> Self {
        self.refused = Some(subject.into());
        self
    }

    pub fn into_result(self) -> Value {
        let mut out = fail(&self.code, &self.message);
        if let Some(subject) = &self.refused {
            out["structuredContent"]["refused"] = Value::String(subject.clone());
        }
        out
    }
}

impl From<StoreError> for Refusal {
    fn from(e: StoreError) -> Self {
        Self {
            code: e.code.to_string(),
            message: e.message,
            refused: None,
        }
    }
}

pub type Answer = Result<Value, Refusal>;

pub fn answer(a: Answer) -> Value {
    match a {
        Ok(v) => ok(v),
        Err(r) => r.into_result(),
    }
}

pub struct Books {
    paths: Paths,
    store: Option<Store>,
    /// The ticket of the Books home, which needs no store.
    home_ticket: Option<String>,
    /// Picked files arriving in parts.
    pub uploads: crate::oldbooks::Uploads,
    /// The cloud sign-in: memory only; the refresh token is in the store.
    pub cloud: crate::cloud::Session,
    /// How Books reaches buwiz-cloud; `None` when no cloud is set.
    pub transport: Option<Box<dyn crate::cloud::Transport>>,
    /// The person's yes this one call runs under, if it does.
    pub approved: Option<crate::press::Approved>,
}

pub const HOME_TITLE: &str = "Books";

impl Books {
    pub fn new(paths: Paths) -> Self {
        Self {
            paths,
            store: None,
            home_ticket: None,
            uploads: crate::oldbooks::Uploads::default(),
            cloud: crate::cloud::Session::default(),
            transport: None,
            approved: None,
        }
    }

    pub fn from_env() -> Self {
        let mut books = Self::new(Paths::from_env());
        books.transport =
            crate::cloud::Http::from_env().map(|t| Box::new(t) as Box<dyn crate::cloud::Transport>);
        books
    }

    /// Whether a store file exists (without making one).
    pub fn exists(&self) -> bool {
        self.store.is_some() || self.paths.db().exists()
    }

    pub fn store(&mut self) -> Result<&mut Store, Refusal> {
        if self.store.is_none() {
            self.store = Some(Store::open(&self.paths)?);
        }
        Ok(self.store.as_mut().expect("just opened"))
    }

    /// The store, the cloud session and the transport at once.
    pub fn cloud_parts(
        &mut self,
    ) -> Result<
        (
            &Store,
            &mut crate::cloud::Session,
            Option<&dyn crate::cloud::Transport>,
        ),
        Refusal,
    > {
        if self.store.is_none() {
            self.store = Some(Store::open(&self.paths)?);
        }
        let store = self.store.as_ref().expect("just opened");
        Ok((store, &mut self.cloud, self.transport.as_deref()))
    }

    /// The home ticket (no store needed).
    pub fn home_ticket(&mut self) -> Result<String, Refusal> {
        if let Some(t) = &self.home_ticket {
            return Ok(t.clone());
        }
        let t = plumbing::new_ticket()?;
        self.home_ticket = Some(t.clone());
        Ok(t)
    }

    /// `buwiz.ticket.resolve`: a neutral title, never the subject.
    pub fn ticket_title(&mut self, ticket: &str) -> Option<String> {
        if self.home_ticket.as_deref() == Some(ticket) {
            return Some(HOME_TITLE.into());
        }
        if !self.exists() {
            return None;
        }
        let store = self.store().ok()?;
        let (kind, id) = plumbing::subject(store.conn(), ticket).ok()??;
        title(store, &kind, id)
    }

    /// `buwiz.uninstall`: forget every ticket; the book stays.
    pub fn forget_tickets(&mut self) {
        self.home_ticket = None;
        if self.exists()
            && let Ok(store) = self.store()
        {
            let _ = plumbing::revoke_all(store.conn());
        }
    }
}

/// The neutral title of a subject: no amount, TIN or legal name.
/// A ticket's neutral title, for a scope token.
pub fn title_of(store: &Store, ticket: &str) -> Option<String> {
    let (kind, id) = plumbing::subject(store.conn(), ticket).ok()??;
    title(store, &kind, id)
}

fn title(store: &Store, kind: &str, id: i64) -> Option<String> {
    let c = store.conn();
    match kind {
        "book" => Some(HOME_TITLE.into()),
        "account" => books::account(c, id)
            .ok()?
            .map(|a| format!("Account · {}", a.name)),
        "entry" => books_store::entries::get(c, id)
            .ok()?
            .map(|e| format!("Entry {:04}", e.no)),
        // a person's name may be a legal name: the title stays neutral
        "person" => books_store::people::person(c, id).ok()?.map(|p| {
            let kind = match p.kind.as_str() {
                "vendor" => "Vendor",
                "customer" => "Customer",
                "employee" => "Employee",
                "owner" => "Owner",
                "lender" => "Lender",
                _ => "Government",
            };
            kind.to_string()
        }),
        "tag" => books_store::people::tag(c, id).ok()?.map(|t| {
            format!(
                "{} · {}",
                if t.kind == "department" {
                    "Department"
                } else {
                    "Location"
                },
                t.name
            )
        }),
        "owed" => books_store::people::unpaid(c, id).ok()?.map(|u| {
            if u.side == "collect" {
                "Invoice to collect".into()
            } else {
                "Bill to pay".into()
            }
        }),
        "sline" => Some("Statement line".into()),
        "cert" => Some("Certificate".into()),
        "evidence" => books_store::evidence::get(c, id)
            .ok()?
            .map(|e| crate::tools::evidence_tools::label(&e)),
        "period" | "form" => crate::cards::title(kind, id),
        "page" => crate::cards::page_of(id)
            .map(|p| format!("Books · {}{}", p[..1].to_uppercase(), &p[1..])),
        "handoff" => books_store::handoffs::get(c, id)
            .ok()?
            .map(|h| crate::handoff::title(&h.form, &h.period_key)),
        _ => None,
    }
}

pub fn ticket(store: &Store, kind: &str, id: i64) -> Result<String, Refusal> {
    Ok(plumbing::ticket_for(store.conn(), kind, id)?)
}

/// The subject id of a ticket of `kind`.
pub fn resolve(store: &Store, kind: &str, ticket: &str) -> Result<i64, Refusal> {
    match plumbing::subject(store.conn(), ticket)? {
        Some((k, id)) if k == kind => Ok(id),
        _ => Err(Refusal::new(
            "not_found",
            format!("That {kind} ticket is not one Books issued."),
        )),
    }
}

/// The book a call is about: its `book` ticket, else the only book.
pub fn book(store: &Store, args: &Map<String, Value>) -> Result<Book, Refusal> {
    if let Some(t) = args.get("book").and_then(Value::as_str) {
        let id = resolve(store, "book", t)?;
        return books::get(store.conn(), id)?
            .ok_or_else(|| Refusal::new("not_found", "No such book."));
    }
    books::all(store.conn())?.into_iter().next().ok_or_else(|| {
        Refusal::new(
            "not_set_up",
            "There is no book yet. Open Books to make one.",
        )
    })
}

/// An account by ticket or chart number, in `book`.
pub fn account(store: &Store, book: &Book, reference: &str) -> Result<Account, Refusal> {
    let found = if plumbing::is_ticket(reference) {
        let id = resolve(store, "account", reference)?;
        books::account(store.conn(), id)?
    } else {
        books::account_by_number(store.conn(), book.id, reference)?
    };
    found
        .filter(|a| a.book_id == book.id)
        .ok_or_else(|| Refusal::new("not_found", "That account is not in this book."))
}

pub fn text<'a>(args: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    args.get(key).and_then(Value::as_str)
}

pub fn required<'a>(args: &'a Map<String, Value>, key: &str) -> Result<&'a str, Refusal> {
    text(args, key)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| Refusal::invalid(format!("`{key}` is required.")))
}

/// What spending a `once` came to: a second use, and a store that couldn't
/// say, are different things to be told.
fn spent_or_refused(spent: Result<bool, StoreError>) -> Result<(), Refusal> {
    match spent {
        Ok(true) => Ok(()),
        Ok(false) => Err(Refusal::new(
            "press_reused",
            "That approval was used already, so nothing changed.",
        )),
        Err(_) => Err(Refusal::new(
            "press_unchecked",
            "Books couldn't check that approval, so nothing changed.",
        )),
    }
}

/// A Books write: the `client_request_id` replays the stored result; else
/// `f` runs in one transaction, its result is stored, and an audit row is
/// written either way (with the refusal code on failure).
pub fn write(
    books: &mut Books,
    tool: &str,
    origin: Origin,
    args: &Map<String, Value>,
    f: impl FnOnce(&Transaction<'_>) -> Answer,
) -> Value {
    let books_approved = books.approved.clone();
    let store = match books.store() {
        Ok(s) => s,
        Err(r) => return r.into_result(),
    };
    let crid = match text(args, "client_request_id") {
        Some(id) => id.to_string(),
        None => {
            return Refusal::invalid("Every Books write takes a client_request_id.").into_result();
        }
    };
    if let Err(e) = plumbing::check_request_id(&crid) {
        return Refusal::from(e).into_result();
    }
    let canonical = jcs::canonical(&Value::Object(args.clone())).unwrap_or_default();
    let digest = plumbing::request_digest(tool, &canonical);
    let approved = books_approved.as_ref().map(|a| plumbing::AuditPress {
        who: &a.who,
        approved_by: &a.approved_by,
        at: a.at,
        once: &a.once,
        args_hash: &a.args_hash,
    });
    let audit = |tx: &books_store::rusqlite::Connection,
                 subject: Option<(&str, i64)>,
                 outcome: &str| match &approved {
        Some(press) => plumbing::audit_pressed(tx, origin.as_str(), tool, subject, outcome, press),
        None => plumbing::audit(tx, origin.as_str(), tool, subject, outcome),
    };
    // spent before anything runs, and for good: a yes works once, go or no go
    if let Some(a) = &books_approved {
        let spent = store.tx(|tx| plumbing::spend_once(tx, &a.once, tool, a.at));
        if let Err(refusal) = spent_or_refused(spent) {
            let _ = store.tx(|tx| audit(tx, None, &refusal.code));
            return refusal.into_result();
        }
    }
    let outcome: Result<Value, Refusal> = (|| {
        let tx = store.transaction()?;
        if let Some(stored) = plumbing::replay(&tx, &crid, tool, &digest)? {
            return Ok(stored);
        }
        // a refusal returns before the commit: the transaction rolls back
        let result = f(&tx)?;
        plumbing::remember(&tx, &crid, tool, &digest, &result)?;
        let subject = subject_of(&result);
        audit(&tx, subject.as_ref().map(|(k, i)| (k.as_str(), *i)), "ok")?;
        tx.commit().map_err(books_store::StoreError::from)?;
        Ok(result)
    })();
    match outcome {
        Ok(result) => ok(strip_subject(result)),
        Err(refusal) => {
            // nothing of the work stays; the refusal is still audited
            let code = refusal.code.clone();
            let _ = store.tx(|tx| audit(tx, None, &code));
            refusal.into_result()
        }
    }
}

/// Results may carry `_subject: [kind, id]` for the audit row; it never
/// leaves the process.
pub fn with_subject(mut result: Value, kind: &str, id: i64) -> Value {
    if let Some(map) = result.as_object_mut() {
        map.insert("_subject".into(), json!([kind, id]));
    }
    result
}

fn subject_of(result: &Value) -> Option<(String, i64)> {
    let s = result.get("_subject")?.as_array()?;
    Some((s.first()?.as_str()?.to_string(), s.get(1)?.as_i64()?))
}

fn strip_subject(mut result: Value) -> Value {
    if let Some(map) = result.as_object_mut() {
        map.remove("_subject");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_store_error_on_a_once_isnt_called_a_used_approval() {
        assert!(spent_or_refused(Ok(true)).is_ok());
        assert_eq!(
            spent_or_refused(Ok(false)).unwrap_err().code,
            "press_reused"
        );
        let e = spent_or_refused(Err(StoreError {
            code: "store",
            message: "disk".into(),
        }))
        .unwrap_err();
        assert_eq!(e.code, "press_unchecked");
        assert!(!e.message.contains("used"));
    }
}
