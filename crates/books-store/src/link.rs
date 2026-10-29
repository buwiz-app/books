//! M9: the link to buwiz-cloud. The book links to one approved tax profile
//! by its opaque id only (never a TIN, docs/01 5.7), and the rotating
//! refresh token lives in the encrypted store, written before the access
//! token it came with is used (P-C4: reusing an old refresh token revokes
//! the whole grant).

use rusqlite::{Connection, OptionalExtension as _, params};

use crate::books::Book;
use crate::{Result, error};

const REFRESH_KEY: &str = "cloud_refresh_token";

/// `^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$`.
pub fn is_profile_id(id: &str) -> bool {
    let groups: Vec<&str> = id.split('-').collect();
    groups.len() == 5
        && groups
            .iter()
            .zip([8, 4, 4, 4, 12])
            .all(|(g, n)| g.len() == n && g.bytes().all(|b| b.is_ascii_hexdigit()))
}

/// Link `book` to the tax profile `id`, or unlink it (`None`).
pub fn link_profile(conn: &Connection, book: &Book, id: Option<&str>) -> Result<()> {
    // the UUID shape is the check: a TIN never has it
    if let Some(id) = id
        && !is_profile_id(id)
    {
        return Err(error(
            "invalid_args",
            "A tax profile links by its opaque id only.",
        ));
    }
    conn.execute(
        "UPDATE book SET cloud_tax_profile_id = ?2 WHERE book_id = ?1",
        params![book.id, id.map(str::to_lowercase)],
    )?;
    Ok(())
}

/// The saved refresh token, if signed in.
pub fn refresh_token(conn: &Connection) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            params![REFRESH_KEY],
            |r| r.get(0),
        )
        .optional()?)
}

/// Save a new refresh token (it replaces the old one at once), or forget it.
pub fn save_refresh_token(conn: &Connection, token: Option<&str>) -> Result<()> {
    match token {
        Some(t) => conn.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2) ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![REFRESH_KEY, t],
        )?,
        None => conn.execute("DELETE FROM meta WHERE key = ?1", params![REFRESH_KEY])?,
    };
    Ok(())
}
