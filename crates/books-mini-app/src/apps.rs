//! The app sources this binary serves as `buwiz-app://<id>` (spec 6.1). They
//! are compiled in, so the bytes `resources/read` answers are the bytes the
//! manifest fingerprints (6.2) and nothing on disk can drift from them.
//!
//! The Books window (`books`) is one app built from several files: the kit
//! every page shares, one file per page, and the window that holds them.
//! An app may import only `nsh`, `nsh/base` and `buwiz` (6.3), so the files
//! are joined here rather than imported.

/// `text/x-nsh-tsx` (spec 5.5).
pub const MIME: &str = "text/x-nsh-tsx";
/// Every app's entry is `buwiz-app://<id>`.
pub const URI_PREFIX: &str = "buwiz-app://";

/// One app source.
pub struct App {
    pub id: &'static str,
    pub source: &'static str,
}

/// The Books window: kit, pages, window, in that order.
const BOOKS: &str = concat!(
    include_str!("../../../apps/books/kit.tsx"),
    "\n",
    include_str!("../../../apps/books/today.tsx"),
    "\n",
    include_str!("../../../apps/books/ledger.tsx"),
    "\n",
    include_str!("../../../apps/books/accounts.tsx"),
    "\n",
    include_str!("../../../apps/books/unpaid.tsx"),
    "\n",
    include_str!("../../../apps/books/people.tsx"),
    "\n",
    include_str!("../../../apps/books/tags.tsx"),
    "\n",
    include_str!("../../../apps/books/reports.tsx"),
    "\n",
    include_str!("../../../apps/books/reconcile.tsx"),
    "\n",
    include_str!("../../../apps/books/settings.tsx"),
    "\n",
    include_str!("../../../apps/books/oldbooks.tsx"),
    "\n",
    include_str!("../../../apps/books/forms.tsx"),
    "\n",
    include_str!("../../../apps/books/payroll.tsx"),
    "\n",
    include_str!("../../../apps/books/certificates.tsx"),
    "\n",
    include_str!("../../../apps/books/window.tsx"),
);

/// Every app, in manifest order.
/// An inline card app: the kit, the card shell, then its own cards (M8).
macro_rules! card_app {
    ($file:literal) => {
        concat!(
            include_str!("../../../apps/books/kit.tsx"),
            "\n",
            include_str!("../../../apps/cards/shell.tsx"),
            "\n",
            include_str!(concat!("../../../apps/cards/", $file)),
        )
    };
}

pub const APPS: &[App] = &[
    App {
        id: "books",
        source: BOOKS,
    },
    App {
        id: "home",
        source: include_str!("../../../apps/home.tsx"),
    },
    // inline cards in the chat (drawing: Books Inline Cards)
    App {
        id: "answer",
        source: card_app!("answer.tsx"),
    },
    App {
        id: "report",
        source: card_app!("report.tsx"),
    },
    App {
        id: "action",
        source: card_app!("action.tsx"),
    },
    App {
        id: "pair",
        source: card_app!("pair.tsx"),
    },
    App {
        id: "quiet",
        source: card_app!("quiet.tsx"),
    },
    // the document library and its "What is this document?" card (evidence-4)
    App {
        id: "evidence",
        source: concat!(
            include_str!("../../../apps/books/kit.tsx"),
            "\n",
            include_str!("../../../apps/evidence.tsx"),
        ),
    },
    App {
        id: "document",
        source: card_app!("evidence.tsx"),
    },
];

/// The app with this id.
pub fn find(id: &str) -> Option<&'static App> {
    APPS.iter().find(|app| app.id == id)
}

/// The app an entry URI names.
pub fn by_uri(uri: &str) -> Option<&'static App> {
    uri.strip_prefix(URI_PREFIX).and_then(find)
}

/// The Books tools an app source names: every `"books_…"` string literal.
/// An app reaches a tool only through `buwiz.call`, and names it only so, so
/// this is exactly what its `permissions.tools` must grant (P8).
pub fn named_tools(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = source;
    while let Some(at) = rest.find("\"books_") {
        rest = &rest[at + 1..];
        let end = rest
            .find(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'))
            .unwrap_or(rest.len());
        if rest[end..].starts_with('"') {
            found.push(rest[..end].to_string());
        }
        rest = &rest[end..];
    }
    found.sort();
    found.dedup();
    found
}
