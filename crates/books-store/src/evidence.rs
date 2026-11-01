//! Immutable evidence blobs and independently detachable links.

use books_core::date::Date;
use rusqlite::{Connection, OptionalExtension as _, params};
use sha2::{Digest as _, Sha256};

use crate::{Result, error, months, now_iso};

#[derive(Debug, Clone)]
pub struct Metadata {
    pub id: i64,
    pub book_id: i64,
    pub kind: String,
    pub mime: String,
    pub page_count: Option<i64>,
    pub source: String,
}

#[derive(Debug, Clone, Copy)]
pub enum Target {
    Entry(i64),
    Owed(i64),
}

impl Target {
    fn ids(self) -> (Option<i64>, Option<i64>) {
        match self {
            Self::Entry(id) => (Some(id), None),
            Self::Owed(id) => (None, Some(id)),
        }
    }
}

pub struct Input<'a> {
    pub bytes: &'a [u8],
    pub mime: &'a str,
    pub page_count: Option<i64>,
    pub source: &'a str,
    pub kind: &'a str,
}

pub fn insert(conn: &Connection, book: i64, input: &Input<'_>) -> Result<(i64, bool)> {
    if input.bytes.len() > 50 * 1024 * 1024 {
        return Err(error("too_large", "Evidence is at most 50 MiB."));
    }
    let hash = crate::hex(&Sha256::digest(input.bytes));
    let changed = conn.execute(
        "INSERT INTO evidence(book_id,sha256,mime,byte_length,bytes,page_count,source,kind,created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT(book_id,sha256) DO NOTHING",
        params![book, hash, input.mime, input.bytes.len() as i64, input.bytes, input.page_count, input.source, input.kind, now_iso()],
    )?;
    let id = conn.query_row(
        "SELECT id FROM evidence WHERE book_id=?1 AND sha256=?2",
        params![book, hash],
        |r| r.get(0),
    )?;
    Ok((id, changed == 0))
}

fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Metadata> {
    Ok(Metadata {
        id: r.get(0)?,
        book_id: r.get(1)?,
        kind: r.get(2)?,
        mime: r.get(3)?,
        page_count: r.get(4)?,
        source: r.get(5)?,
    })
}

pub fn get(conn: &Connection, id: i64) -> Result<Option<Metadata>> {
    Ok(conn
        .query_row(
            "SELECT id,book_id,kind,mime,page_count,source FROM evidence WHERE id=?1",
            [id],
            row,
        )
        .optional()?)
}

pub fn list(
    conn: &Connection,
    book: i64,
    evidence: Option<i64>,
    target: Option<Target>,
    after: i64,
    limit: usize,
) -> Result<Vec<Metadata>> {
    let (entry, owed) = target.map(Target::ids).unwrap_or_default();
    let mut stmt = conn.prepare("SELECT e.id,e.book_id,e.kind,e.mime,e.page_count,e.source FROM evidence e
        WHERE e.book_id=?1 AND e.id>?2 AND (?3 IS NULL OR e.id=?3)
        AND ((?4 IS NULL AND ?5 IS NULL) OR EXISTS(SELECT 1 FROM evidence_link l WHERE l.evidence_id=e.id AND (l.entry_id=?4 OR l.owed_id=?5)))
        ORDER BY e.id LIMIT ?6")?;
    Ok(stmt
        .query_map(
            params![book, after, evidence, entry, owed, limit.min(201) as i64],
            row,
        )?
        .collect::<rusqlite::Result<_>>()?)
}

pub fn validate_target(conn: &Connection, book: i64, target: Target, open: bool) -> Result<()> {
    let (sql, id) = match target {
        Target::Entry(id) => ("SELECT book_id,entry_date FROM entry WHERE entry_id=?1", id),
        Target::Owed(id) => ("SELECT book_id,issued FROM unpaid WHERE unpaid_id=?1", id),
    };
    let found: Option<(i64, String)> = conn
        .query_row(sql, [id], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()?;
    let (_, date) = found
        .filter(|r| r.0 == book)
        .ok_or_else(|| error("not_found", "That target is not in this book."))?;
    if open {
        months::check_open(
            conn,
            book,
            Date::parse(&date).ok_or_else(|| error("store", "Invalid target date."))?,
        )?;
    }
    Ok(())
}

fn validate(conn: &Connection, book: i64, id: i64, target: Target) -> Result<()> {
    get(conn, id)?
        .filter(|e| e.book_id == book)
        .ok_or_else(|| error("not_found", "That evidence is not in this book."))?;
    validate_target(conn, book, target, true)
}

pub fn link(conn: &Connection, book: i64, id: i64, target: Target) -> Result<()> {
    validate(conn, book, id, target)?;
    let (entry, owed) = target.ids();
    conn.execute("INSERT INTO evidence_link(evidence_id,entry_id,owed_id,created_at) VALUES(?1,?2,?3,?4) ON CONFLICT DO NOTHING", params![id, entry, owed, now_iso()])?;
    Ok(())
}

pub fn detach(conn: &Connection, book: i64, id: i64, target: Target) -> Result<bool> {
    validate(conn, book, id, target)?;
    let (entry, owed) = target.ids();
    let removed = conn.execute(
        "DELETE FROM evidence_link WHERE evidence_id=?1 AND (entry_id=?2 OR owed_id=?3)",
        params![id, entry, owed],
    )?;
    Ok(removed > 0)
}

pub fn linked_count(conn: &Connection, id: i64) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT count(*) FROM evidence_link WHERE evidence_id=?1",
        [id],
        |r| r.get(0),
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Paths, Store, TempDir, books};
    use books_core::ledger::Basis;

    #[test]
    fn dedup_and_empty_list() {
        let tmp = TempDir::new("evidence-store");
        let store = Store::open(&Paths::in_dir(tmp.0.join("db"))).unwrap();
        let c = store.conn();
        let book = books::create(c, "Synthetic", Basis::Cash, 1).unwrap();
        assert!(list(c, book, None, None, 0, 50).unwrap().is_empty());
        let input = Input {
            bytes: b"%PDF-synthetic",
            mime: "application/pdf",
            page_count: Some(1),
            source: "user",
            kind: "receipt",
        };
        let (id, duplicate) = insert(c, book, &input).unwrap();
        assert!(!duplicate);
        assert_eq!(insert(c, book, &input).unwrap(), (id, true));
        assert_eq!(linked_count(c, id).unwrap(), 0);
        assert_eq!(get(c, id).unwrap().unwrap().kind, "receipt");
        assert!(link(c, book, id, Target::Entry(-1)).is_err());
        assert_eq!(linked_count(c, id).unwrap(), 0);
    }

    #[test]
    fn migration_upgrades_version_six() {
        let tmp = TempDir::new("evidence-migration");
        let paths = Paths::in_dir(tmp.0.join("db"));
        {
            let store = Store::open(&paths).unwrap();
            store.conn().execute_batch("DROP TABLE evidence_link; DROP TABLE evidence; UPDATE meta SET value='6' WHERE key='schema_version';").unwrap();
        }
        let store = Store::open(&paths).unwrap();
        assert_eq!(store.meta("schema_version").unwrap().as_deref(), Some("7"));
        assert!(
            list(store.conn(), -1, None, None, 0, 50)
                .unwrap()
                .is_empty()
        );
    }
}
