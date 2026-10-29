//! Buwiz Books domain rules, with no I/O: integer centavos ([`money`]),
//! Manila calendar days and period keys ([`date`]), the kind catalogue and
//! chart presets ([`catalog`]), the ledger's posting rules ([`ledger`]) and
//! the TIN refusal for typed text ([`privacy`]).
//!
//! Books holds no compliance logic (ADR 0007): no rate, election,
//! threshold, ATC choice, tax due, rounding, due date or penalty.

pub mod catalog;
pub mod date;
pub mod ledger;
pub mod money;
pub mod privacy;
