//! Tickets (spec 9.5): `tkt_` plus 26 Crockford base32 characters, 130 bits
//! from the OS CSPRNG. A ticket stands for a subject everywhere outside this
//! process; the subject map stays here, in memory, until `rox.uninstall`.
//!
//! P0 has one subject, the Books home. P1 moves the map into the store.

use std::collections::HashMap;

const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// What a ticket stands for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Subject {
    /// The Books home view (no book, no amount, no name).
    Home,
}

impl Subject {
    /// The neutral title `rox.ticket.resolve` shows (P3, D1).
    pub fn title(self) -> &'static str {
        match self {
            Subject::Home => "Books",
        }
    }
}

/// Whether `text` has the ticket grammar `^tkt_[0-9A-HJKMNP-TV-Z]{26}$`.
pub fn is_ticket(text: &str) -> bool {
    text.strip_prefix("tkt_")
        .is_some_and(|rest| rest.len() == 26 && rest.bytes().all(|byte| CROCKFORD.contains(&byte)))
}

/// A fresh ticket: 130 random bits, 5 per character.
pub fn new_ticket() -> Result<String, String> {
    let mut bytes = [0u8; 17];
    getrandom::fill(&mut bytes).map_err(|error| format!("no randomness for a ticket: {error}"))?;
    let mut out = String::from("tkt_");
    for i in 0..26 {
        let bit = i * 5;
        let pair = (u16::from(bytes[bit / 8]) << 8) | u16::from(bytes[bit / 8 + 1]);
        let symbol = (pair >> (11 - bit % 8)) & 31;
        out.push(char::from(CROCKFORD[usize::from(symbol)]));
    }
    Ok(out)
}

/// Subject to ticket and back, in this process only.
#[derive(Default)]
pub struct Tickets {
    by_ticket: HashMap<String, Subject>,
    by_subject: HashMap<Subject, String>,
}

impl Tickets {
    /// The subject's ticket: the one it has, else a new one.
    pub fn issue(&mut self, subject: Subject) -> Result<String, String> {
        if let Some(ticket) = self.by_subject.get(&subject) {
            return Ok(ticket.clone());
        }
        let ticket = new_ticket()?;
        self.by_subject.insert(subject, ticket.clone());
        self.by_ticket.insert(ticket.clone(), subject);
        Ok(ticket)
    }

    /// The subject of a ticket this process issued.
    pub fn subject(&self, ticket: &str) -> Option<Subject> {
        self.by_ticket.get(ticket).copied()
    }

    /// Forget every ticket (`rox.uninstall`).
    pub fn clear(&mut self) {
        self.by_ticket.clear();
        self.by_subject.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tickets_follow_the_grammar_and_are_stable_per_subject() {
        let mut tickets = Tickets::default();
        let ticket = tickets.issue(Subject::Home).unwrap();
        assert!(is_ticket(&ticket), "{ticket}");
        assert_eq!(tickets.issue(Subject::Home).unwrap(), ticket);
        assert_eq!(tickets.subject(&ticket), Some(Subject::Home));
        assert_ne!(new_ticket().unwrap(), new_ticket().unwrap());
        tickets.clear();
        assert_eq!(tickets.subject(&ticket), None);
        assert!(!is_ticket("tkt_0000000000000000000000000I"));
        assert!(!is_ticket("tkt_short"));
    }
}
