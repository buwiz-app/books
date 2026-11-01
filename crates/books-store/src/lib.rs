//! The Books store: one SQLCipher file per Mac, its key in a 0600 key file
//! (ADR 0002, D2 option a). It refuses to open with a wrong key or when
//! SQLCipher is not linked (an unlinked SQLite would ignore `PRAGMA key`
//! and write plain text: risk RK-7), and records the cipher version it was
//! created with.
//!
//! - [`Paths`]: where the file and the key live (`BOOKS_DATA_DIR`,
//!   `BOOKS_DB_KEY_FILE`);
//! - [`Store`]: open, migrate, and one transaction per tool call;
//! - [`books`], [`entries`]: the book, its chart and the ledger;
//! - [`people`]: people, tags and unpaid documents;
//! - [`months`]: month locks, statements, defaults and book settings;
//! - [`imports`]: old books coming in, account matches, opening totals;
//! - [`plumbing`]: idempotency, audit rows and tickets.

use std::fmt;
use std::fs;
use std::io::Write as _;
use std::os::unix::fs::{DirBuilderExt as _, OpenOptionsExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension as _, params};

pub mod books;
pub mod certificates;
pub mod entries;
pub mod handoffs;
pub mod imports;
pub mod link;
pub mod months;
pub mod people;
pub mod plumbing;
pub mod slices;

pub use rusqlite;

/// Migrations, applied in order; `meta.schema_version` is the last applied.
const MIGRATIONS: &[(i64, &str)] = &[
    (1, include_str!("../migrations/0001_init.sql")),
    (2, include_str!("../migrations/0002_people_tags_unpaid.sql")),
    (3, include_str!("../migrations/0003_months_statements.sql")),
    (4, include_str!("../migrations/0004_imports_openings.sql")),
    (5, include_str!("../migrations/0005_certificates.sql")),
    (6, include_str!("../migrations/0006_handoffs.sql")),
];

pub const DATA_DIR_ENV: &str = "BOOKS_DATA_DIR";
pub const KEY_FILE_ENV: &str = "BOOKS_DB_KEY_FILE";
pub const DB_FILE: &str = "books.db";
pub const KEY_FILE: &str = "books.key";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreError {
    pub code: &'static str,
    pub message: String,
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for StoreError {}

pub fn error(code: &'static str, message: impl Into<String>) -> StoreError {
    StoreError {
        code,
        message: message.into(),
    }
}

impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        // SQLite messages name tables and columns, never values
        error("store", format!("the store failed: {e}"))
    }
}

impl From<books_core::ledger::LedgerError> for StoreError {
    fn from(e: books_core::ledger::LedgerError) -> Self {
        StoreError {
            code: e.code,
            message: e.message,
        }
    }
}

pub type Result<T> = std::result::Result<T, StoreError>;

/// Where the store lives.
#[derive(Debug, Clone)]
pub struct Paths {
    pub dir: PathBuf,
    pub key_file: PathBuf,
}

impl Paths {
    pub fn in_dir(dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        Self {
            key_file: dir.join(KEY_FILE),
            dir,
        }
    }

    /// `BOOKS_DATA_DIR` (default `~/Library/Application Support/app.buwiz.books`)
    /// and `BOOKS_DB_KEY_FILE` (default `<dir>/books.key`).
    pub fn from_env() -> Self {
        let dir = std::env::var_os(DATA_DIR_ENV)
            .filter(|d| !d.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let home = std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_default();
                home.join("Library/Application Support/app.buwiz.books")
            });
        let key_file = std::env::var_os(KEY_FILE_ENV)
            .filter(|k| !k.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| dir.join(KEY_FILE));
        Self { dir, key_file }
    }

    pub fn db(&self) -> PathBuf {
        self.dir.join(DB_FILE)
    }
}

/// `2026-10-29T04:05:06Z` for a unix time in milliseconds.
pub fn iso_instant(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let day = books_core::date::Date::from_days(secs.div_euclid(86_400));
    let s = secs.rem_euclid(86_400);
    format!("{day}T{:02}:{:02}:{:02}Z", s / 3600, s / 60 % 60, s % 60)
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_millis()).ok())
        .unwrap_or(0)
}

pub fn now_iso() -> String {
    iso_instant(now_ms())
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Read the key file, or create it (0600, 32 random bytes as hex) when it
/// does not exist. A key file others can read is refused.
fn key(path: &Path) -> Result<String> {
    match fs::metadata(path) {
        Ok(meta) => {
            if meta.permissions().mode() & 0o077 != 0 {
                return Err(error(
                    "key_file_mode",
                    "The Books key file can be read by other users; it must be mode 0600.",
                ));
            }
            let text = fs::read_to_string(path)
                .map_err(|_| error("key_file", "The Books key file cannot be read."))?;
            let text = text.trim();
            if text.len() != 64 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(error(
                    "key_file",
                    "The Books key file is not a 32-byte hex key.",
                ));
            }
            Ok(text.to_ascii_lowercase())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if let Some(parent) = path.parent() {
                fs::DirBuilder::new()
                    .recursive(true)
                    .mode(0o700)
                    .create(parent)
                    .map_err(|_| error("key_file", "The Books key folder cannot be made."))?;
            }
            let mut bytes = [0u8; 32];
            getrandom::fill(&mut bytes)
                .map_err(|_| error("key_file", "No randomness for a new Books key."))?;
            let text = hex(&bytes);
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(path)
                .map_err(|_| error("key_file", "The Books key file cannot be written."))?;
            file.write_all(text.as_bytes())
                .and_then(|()| file.sync_all())
                .map_err(|_| error("key_file", "The Books key file cannot be written."))?;
            Ok(text)
        }
        Err(_) => Err(error("key_file", "The Books key file cannot be read.")),
    }
}

/// The SQLCipher major version, e.g. `4` from `4.6.1 community`.
fn cipher_major(version: &str) -> &str {
    version.split('.').next().unwrap_or("")
}

pub struct Store {
    conn: Connection,
}

impl Store {
    /// Open (creating when absent) the store at `paths`, migrated to the
    /// latest schema.
    pub fn open(paths: &Paths) -> Result<Self> {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&paths.dir)
            .map_err(|_| error("data_dir", "The Books data folder cannot be made."))?;
        let db = paths.db();
        let fresh = !db.exists();
        let key = key(&paths.key_file)?;
        let conn = Connection::open(&db)?;
        // SQLCipher reads a raw key as x'<64 hex>' (no key derivation)
        conn.execute_batch(&format!("PRAGMA key = \"x'{key}'\";"))?;
        let cipher: Option<String> = conn
            .query_row("PRAGMA cipher_version", [], |r| r.get(0))
            .optional()?;
        let Some(cipher) = cipher.filter(|c| !c.is_empty()) else {
            return Err(error(
                "no_cipher",
                "This build has no SQLCipher, so Books will not open a store it cannot encrypt.",
            ));
        };
        if conn
            .query_row("SELECT count(*) FROM sqlite_master", [], |r| {
                r.get::<_, i64>(0)
            })
            .is_err()
        {
            return Err(error(
                "wrong_key",
                "The Books store does not open with this key file.",
            ));
        }
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = DELETE;")?;
        let mut store = Self { conn };
        store.migrate(fresh, &cipher)?;
        Ok(store)
    }

    fn migrate(&mut self, fresh: bool, cipher: &str) -> Result<()> {
        let has_meta: bool = self.conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'meta'",
            [],
            |r| r.get::<_, i64>(0).map(|n| n > 0),
        )?;
        let current: i64 = if has_meta {
            self.conn
                .query_row(
                    "SELECT value FROM meta WHERE key = 'schema_version'",
                    [],
                    |r| r.get::<_, String>(0),
                )
                .optional()?
                .and_then(|v| v.parse().ok())
                .unwrap_or(0)
        } else {
            0
        };
        if has_meta {
            let created: Option<String> = self
                .conn
                .query_row(
                    "SELECT value FROM meta WHERE key = 'cipher_version'",
                    [],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(created) = created
                && cipher_major(&created) != cipher_major(cipher)
            {
                return Err(error(
                    "cipher_mismatch",
                    "The Books store was made with another SQLCipher version; Books will not open it.",
                ));
            }
        } else if !fresh {
            let tables: i64 =
                self.conn
                    .query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get(0))?;
            if tables > 0 {
                return Err(error("not_books", "That file is not a Books store."));
            }
        }
        let latest = MIGRATIONS.last().map_or(0, |m| m.0);
        if current > latest {
            return Err(error(
                "schema_newer",
                "The Books store is from a newer Books; update Books to open it.",
            ));
        }
        for (version, sql) in MIGRATIONS {
            if *version <= current {
                continue;
            }
            let tx = self.conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.execute(
                "INSERT INTO meta (key, value) VALUES ('schema_version', ?1)
                 ON CONFLICT (key) DO UPDATE SET value = excluded.value",
                params![version.to_string()],
            )?;
            if *version == 1 {
                let mut id = [0u8; 16];
                getrandom::fill(&mut id)
                    .map_err(|_| error("store", "No randomness for a store id."))?;
                tx.execute(
                    "INSERT INTO meta (key, value) VALUES ('cipher_version', ?1), ('store_id', ?2), ('created_at', ?3)",
                    params![cipher, hex(&id), now_iso()],
                )?;
            }
            tx.commit()?;
        }
        Ok(())
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// One transaction: everything in `f` commits, or nothing does.
    pub fn tx<T>(&mut self, f: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T>) -> Result<T> {
        let tx = self.conn.transaction()?;
        let out = f(&tx)?;
        tx.commit()?;
        Ok(out)
    }

    /// A transaction the caller commits; dropping it rolls back.
    pub fn transaction(&mut self) -> Result<rusqlite::Transaction<'_>> {
        Ok(self.conn.transaction()?)
    }

    pub fn meta(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM meta WHERE key = ?1", params![key], |r| {
                r.get(0)
            })
            .optional()?)
    }
}

/// A fresh temp folder outside the repo, removed on drop (tests).
pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new(tag: &str) -> Self {
        let mut r = [0u8; 8];
        getrandom::fill(&mut r).expect("randomness");
        let dir = std::env::temp_dir().join(format!("books-{tag}-{}", hex(&r)));
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&dir)
            .expect("a temp folder");
        Self(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CANARY: &str = "MEMO-SYNTH-STRINGS-CANARY-7f3a";

    #[test]
    fn rk7_the_file_holds_no_plain_text_and_opens_only_with_its_key() {
        let tmp = TempDir::new("strings");
        let paths = Paths::in_dir(tmp.0.join("data"));
        {
            let store = Store::open(&paths).unwrap();
            store
                .conn()
                .execute(
                    "INSERT INTO meta (key, value) VALUES ('canary', ?1)",
                    params![CANARY],
                )
                .unwrap();
            assert!(
                store
                    .meta("cipher_version")
                    .unwrap()
                    .unwrap()
                    .starts_with('4')
            );
        }
        let bytes = fs::read(paths.db()).unwrap();
        assert!(bytes.len() > 1024);
        let find = |needle: &[u8]| bytes.windows(needle.len()).any(|w| w == needle);
        assert!(
            !find(CANARY.as_bytes()),
            "the canary is in the file as plain text"
        );
        assert!(
            !find(b"SQLite format 3"),
            "the file has a plain SQLite header"
        );
        assert!(!find(b"CREATE TABLE"), "the schema is readable");
        let mode = fs::metadata(&paths.key_file).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        let mode = fs::metadata(&paths.dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o700);
        let store = Store::open(&paths).unwrap();
        assert_eq!(store.meta("canary").unwrap().as_deref(), Some(CANARY));
        assert_eq!(store.meta("schema_version").unwrap().as_deref(), Some("6"));
        drop(store);
        let other = Paths {
            dir: paths.dir.clone(),
            key_file: tmp.0.join("other.key"),
        };
        let e = Store::open(&other).err().unwrap();
        assert_eq!(e.code, "wrong_key");
        assert_eq!(
            fs::read(paths.db()).unwrap(),
            bytes,
            "a refused open leaves the file as it was"
        );
    }

    #[test]
    fn a_key_file_others_can_read_is_refused() {
        let tmp = TempDir::new("mode");
        let paths = Paths::in_dir(tmp.0.join("data"));
        drop(Store::open(&paths).unwrap());
        fs::set_permissions(&paths.key_file, fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(Store::open(&paths).err().unwrap().code, "key_file_mode");
    }

    #[test]
    fn a_plain_sqlite_file_is_not_opened_as_a_store() {
        let tmp = TempDir::new("plain");
        let paths = Paths::in_dir(tmp.0.join("data"));
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&paths.dir)
            .unwrap();
        {
            let plain = Connection::open(paths.db()).unwrap();
            plain.execute_batch("CREATE TABLE t (x INTEGER);").unwrap();
        }
        assert_eq!(Store::open(&paths).err().unwrap().code, "wrong_key");
    }

    #[test]
    fn instants_are_utc_text() {
        assert_eq!(iso_instant(0), "1970-01-01T00:00:00Z");
        assert_eq!(
            iso_instant(1_793_232_000_000 + 3_723_000),
            "2026-10-29T01:02:03Z"
        );
    }
}
