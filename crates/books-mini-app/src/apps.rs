//! The app sources this binary serves as `rox-app://<id>` (spec 6.1). They
//! are compiled in, so the bytes `resources/read` answers are the bytes the
//! manifest fingerprints (6.2) and nothing on disk can drift from them.

/// `text/x-nsh-tsx` (spec 5.5).
pub const MIME: &str = "text/x-nsh-tsx";
/// Every app's entry is `rox-app://<id>`.
pub const URI_PREFIX: &str = "rox-app://";

/// One app source.
pub struct App {
    pub id: &'static str,
    pub source: &'static str,
}

/// Every app, in manifest order.
pub const APPS: &[App] = &[App {
    id: "home",
    source: include_str!("../../../apps/home.tsx"),
}];

/// The app with this id.
pub fn find(id: &str) -> Option<&'static App> {
    APPS.iter().find(|app| app.id == id)
}

/// The app an entry URI names.
pub fn by_uri(uri: &str) -> Option<&'static App> {
    uri.strip_prefix(URI_PREFIX).and_then(find)
}
