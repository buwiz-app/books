//! The TSX app sources this binary serves as `buwiz-app://<id>` (spec 6.1).
//! They are compiled in, so the bytes `resources/read` answers are the bytes
//! the manifest fingerprints (6.2) and nothing on disk can drift from them.
//!
//! The Books window (`books`) is one app built from several files: the kit
//! every page shares, one file per page, and the window that holds them.
//! An app may import only `nsh`, `nsh/base` and `buwiz` (6.3), so the files
//! are joined here rather than imported.

/// `text/x-nsh-tsx` (spec 5.5).
pub const MIME: &str = "text/x-nsh-tsx";
/// Every app's entry is `buwiz-app://<id>`.
pub const URI_PREFIX: &str = "buwiz-app://";

/// One app's implementation.
#[derive(Clone, Copy)]
pub enum AppSource {
    Tsx(&'static str),
    Native {
        screen: &'static str,
        abi: u32,
        tools: &'static [&'static str],
        open_apps: &'static [&'static str],
    },
}

/// One app.
pub struct App {
    pub id: &'static str,
    pub source: AppSource,
}

/// The Books window: kit, pages, window, in that order.
const BOOKS: &str = concat!(
    include_str!("../../../apps/books/kit.tsx"),
    "\n",
    include_str!("../../../apps/books/today.tsx"),
    "\n",
    include_str!("../../../apps/books/ledger_review.tsx"),
    "\n",
    include_str!("../../../apps/books/ledger_findings.tsx"),
    "\n",
    include_str!("../../../apps/books/ledger_amend.tsx"),
    "\n",
    include_str!("../../../apps/books/entry.tsx"),
    "\n",
    include_str!("../../../apps/books/accounts.tsx"),
    "\n",
    include_str!("../../../apps/books/unpaid.tsx"),
    "\n",
    include_str!("../../../apps/books/owed_print.tsx"),
    "\n",
    include_str!("../../../apps/books/owed_repeat.tsx"),
    "\n",
    include_str!("../../../apps/books/people.tsx"),
    "\n",
    include_str!("../../../apps/books/tags.tsx"),
    "\n",
    include_str!("../../../apps/books/reports.tsx"),
    "\n",
    include_str!("../../../apps/books/reconcile.tsx"),
    "\n",
    include_str!("../../../apps/books/statement_read.tsx"),
    "\n",
    include_str!("../../../apps/books/statement_snapshot.tsx"),
    "\n",
    include_str!("../../../apps/books/reconcile_flags.tsx"),
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
    include_str!("../../../apps/books/activity.tsx"),
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
        source: AppSource::Tsx(BOOKS),
    },
    App {
        id: "home",
        source: AppSource::Tsx(include_str!("../../../apps/home.tsx")),
    },
    // inline cards in the chat (drawing: Books Inline Cards)
    App {
        id: "answer",
        source: AppSource::Tsx(card_app!("answer.tsx")),
    },
    App {
        id: "report",
        source: AppSource::Tsx(card_app!("report.tsx")),
    },
    App {
        id: "action",
        source: AppSource::Tsx(card_app!("action.tsx")),
    },
    App {
        id: "pair",
        source: AppSource::Tsx(card_app!("pair.tsx")),
    },
    App {
        id: "ledger",
        source: AppSource::Native {
            screen: "books.ledger",
            abi: 1,
            tools: &[
                crate::tools::LEDGER_LIST,
                crate::tools::ENTRY_GET,
                crate::tools::ENTRY_OPTIONS,
                crate::tools::ENTRY_DRAFT,
                crate::tools::POST_ENTRY,
                crate::tools::VOID_ENTRY,
                crate::tools::DISCARD_ENTRY,
                crate::tools::RECATEGORIZE,
            ],
            open_apps: &[],
        },
    },
    App {
        id: "quiet",
        source: AppSource::Native {
            screen: "books.quiet",
            abi: 1,
            tools: &[crate::tools::NUDGE],
            open_apps: &["books"],
        },
    },
    // the document library and its "What is this document?" card (evidence-4)
    App {
        id: "evidence",
        source: AppSource::Tsx(concat!(
            include_str!("../../../apps/books/kit.tsx"),
            "\n",
            include_str!("../../../apps/evidence.tsx"),
        )),
    },
    App {
        id: "document",
        source: AppSource::Tsx(card_app!("evidence.tsx")),
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
