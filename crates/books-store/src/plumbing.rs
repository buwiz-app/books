//! Idempotency (DD-17: every write takes a `client_request_id`; a repeat
//! returns the stored result), audit rows (no memo, TIN or amount) and
//! tickets (spec 9.5: `tkt_` plus 26 Crockford base32 characters, 130 bits
//! from the OS CSPRNG), kept in the store so they survive a restart.

use rusqlite::{Connection, OptionalExtension as _, params};
use serde_json::Value;
use sha2::{Digest as _, Sha256};

use crate::{Result, error, now_iso};

/// `^[A-Za-z0-9._:-]{8,64}$`.
pub fn check_request_id(id: &str) -> Result<()> {
    let ok = (8..=64).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-'));
    if ok {
        Ok(())
    } else {
        Err(error(
            "invalid_args",
            "client_request_id must be 8 to 64 letters, digits or . _ : -",
        ))
    }
}

/// SHA-256 of the request's canonical text.
pub fn request_digest(tool: &str, canonical: &str) -> String {
    let mut h = Sha256::new();
    h.update(tool.as_bytes());
    h.update([0]);
    h.update(canonical.as_bytes());
    crate::hex(&h.finalize())
}

/// The stored result of an earlier call with this id. The same id with a
/// different request is refused.
pub fn replay(conn: &Connection, id: &str, tool: &str, digest: &str) -> Result<Option<Value>> {
    let row: Option<(String, String, String)> = conn
        .query_row(
            "SELECT tool, request_sha256, result FROM idempotency WHERE client_request_id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    match row {
        None => Ok(None),
        Some((t, d, result)) if t == tool && d == digest => Ok(serde_json::from_str(&result).ok()),
        Some(_) => Err(error(
            "request_id_reused",
            "That client_request_id was used for a different request.",
        )),
    }
}

pub fn remember(
    conn: &Connection,
    id: &str,
    tool: &str,
    digest: &str,
    result: &Value,
) -> Result<()> {
    conn.execute(
        "INSERT INTO idempotency (client_request_id, tool, request_sha256, result, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, tool, digest, result.to_string(), now_iso()],
    )?;
    Ok(())
}

/// One audit row. `outcome` is a short code such as `ok` or a refusal code.
pub fn audit(
    conn: &Connection,
    origin: &str,
    tool: &str,
    subject: Option<(&str, i64)>,
    outcome: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO audit_event (at, origin, tool, subject_kind, subject_id, outcome) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![now_iso(), origin, tool, subject.map(|s| s.0), subject.map(|s| s.1), outcome],
    )?;
    Ok(())
}

pub fn audit_count(conn: &Connection, tool: &str) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT count(*) FROM audit_event WHERE tool = ?1",
        params![tool],
        |r| r.get(0),
    )?)
}

const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Whether `text` has the ticket grammar `^tkt_[0-9A-HJKMNP-TV-Z]{26}$`.
pub fn is_ticket(text: &str) -> bool {
    text.strip_prefix("tkt_")
        .is_some_and(|rest| rest.len() == 26 && rest.bytes().all(|b| CROCKFORD.contains(&b)))
}

/// A fresh ticket: 130 random bits, 5 per character.
pub fn new_ticket() -> Result<String> {
    let mut bytes = [0u8; 17];
    getrandom::fill(&mut bytes).map_err(|_| error("internal", "No randomness for a ticket."))?;
    let mut out = String::from("tkt_");
    for i in 0..26 {
        let bit = i * 5;
        let pair = (u16::from(bytes[bit / 8]) << 8) | u16::from(bytes[bit / 8 + 1]);
        let symbol = (pair >> (11 - bit % 8)) & 31;
        out.push(char::from(CROCKFORD[usize::from(symbol)]));
    }
    Ok(out)
}

/// The subject's ticket: the one it has, else a new one.
pub fn ticket_for(conn: &Connection, kind: &str, id: i64) -> Result<String> {
    if let Some(t) = conn
        .query_row(
            "SELECT ticket FROM ticket WHERE subject_kind = ?1 AND subject_id = ?2 AND revoked_at IS NULL",
            params![kind, id],
            |r| r.get::<_, String>(0),
        )
        .optional()?
    {
        return Ok(t);
    }
    let t = new_ticket()?;
    conn.execute(
        "INSERT INTO ticket (ticket, subject_kind, subject_id, created_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (subject_kind, subject_id) DO UPDATE SET ticket = excluded.ticket, created_at = excluded.created_at, revoked_at = NULL",
        params![t, kind, id, now_iso()],
    )?;
    Ok(t)
}

/// The subject of a live ticket.
pub fn subject(conn: &Connection, ticket: &str) -> Result<Option<(String, i64)>> {
    if !is_ticket(ticket) {
        return Ok(None);
    }
    Ok(conn
        .query_row(
            "SELECT subject_kind, subject_id FROM ticket WHERE ticket = ?1 AND revoked_at IS NULL",
            params![ticket],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?)
}

/// Revoke every ticket (`buwiz.uninstall`). The book itself is kept (D10).
pub fn revoke_all(conn: &Connection) -> Result<()> {
    conn.execute(
        "UPDATE ticket SET revoked_at = ?1 WHERE revoked_at IS NULL",
        params![now_iso()],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Paths, Store, TempDir};

    #[test]
    fn a_repeat_returns_the_stored_result_and_a_reused_id_is_refused() {
        let tmp = TempDir::new("idem");
        let store = Store::open(&Paths::in_dir(tmp.0.join("d"))).unwrap();
        let c = store.conn();
        let d1 = request_digest("books_entry_draft", "{\"a\":1}");
        assert_eq!(
            replay(c, "req-00000001", "books_entry_draft", &d1).unwrap(),
            None
        );
        remember(
            c,
            "req-00000001",
            "books_entry_draft",
            &d1,
            &serde_json::json!({"ok": true}),
        )
        .unwrap();
        assert_eq!(
            replay(c, "req-00000001", "books_entry_draft", &d1).unwrap(),
            Some(serde_json::json!({"ok": true}))
        );
        let d2 = request_digest("books_entry_draft", "{\"a\":2}");
        assert_eq!(
            replay(c, "req-00000001", "books_entry_draft", &d2)
                .unwrap_err()
                .code,
            "request_id_reused"
        );
        assert!(check_request_id("short").is_err());
        assert!(check_request_id("ok-request-0001").is_ok());
    }

    #[test]
    fn tickets_are_stable_per_subject_and_revocable() {
        let tmp = TempDir::new("tkt");
        let store = Store::open(&Paths::in_dir(tmp.0.join("d"))).unwrap();
        let c = store.conn();
        let t = ticket_for(c, "entry", 7).unwrap();
        assert!(is_ticket(&t), "{t}");
        assert_eq!(ticket_for(c, "entry", 7).unwrap(), t);
        assert_eq!(subject(c, &t).unwrap(), Some(("entry".into(), 7)));
        assert_ne!(ticket_for(c, "entry", 8).unwrap(), t);
        revoke_all(c).unwrap();
        assert_eq!(subject(c, &t).unwrap(), None);
        assert_ne!(
            ticket_for(c, "entry", 7).unwrap(),
            t,
            "a revoked ticket is not reissued"
        );
        assert!(!is_ticket("tkt_0000000000000000000000000I"));
    }

    #[test]
    fn audit_rows_carry_codes_only() {
        let tmp = TempDir::new("audit");
        let store = Store::open(&Paths::in_dir(tmp.0.join("d"))).unwrap();
        audit(
            store.conn(),
            "app",
            "books_app_post_entry",
            Some(("entry", 1)),
            "ok",
        )
        .unwrap();
        assert_eq!(
            audit_count(store.conn(), "books_app_post_entry").unwrap(),
            1
        );
    }
}
