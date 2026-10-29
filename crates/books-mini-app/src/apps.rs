//! The app sources this binary serves as `rox-app://<id>` (spec 6.1). They
//! are compiled in, so the bytes `resources/read` answers are the bytes the
//! manifest fingerprints (6.2) and nothing on disk can drift from them.
//!
//! The Books window (`books`) is one app built from several files: the kit
//! every page shares, one file per page, and the window that holds them.
//! An app may import only `nsh`, `nsh/base` and `rox` (6.3), so the files
//! are joined here rather than imported.

/// `text/x-nsh-tsx` (spec 5.5).
pub const MIME: &str = "text/x-nsh-tsx";
/// Every app's entry is `rox-app://<id>`.
pub const URI_PREFIX: &str = "rox-app://";

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
    include_str!("../../../apps/books/window.tsx"),
);

/// Every app, in manifest order.
pub const APPS: &[App] = &[
    App {
        id: "books",
        source: BOOKS,
    },
    App {
        id: "home",
        source: include_str!("../../../apps/home.tsx"),
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
/// An app reaches a tool only through `rox.call`, and names it only so, so
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
