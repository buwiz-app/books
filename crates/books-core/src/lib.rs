//! Buwiz Books domain rules, with no I/O: integer centavos ([`money`]),
//! Manila calendar days and period keys ([`date`]), the kind catalogue and
//! chart presets ([`catalog`]), the ledger's posting rules ([`ledger`]) and
//! the TIN refusal for typed text ([`privacy`]).
//!
//! The measures engine and the Forms bridge, ported from the PoC (`poc/src`)
//! byte for byte against `fixtures/vectors/`: deterministic JSON
//! ([`canonical`]), the compiled-in rule sets and crosswalks ([`rules`]),
//! the normalized input ([`input`]), posting for measurement ([`posting`]),
//! the accounting measures with lineage ([`measures`]), the mapping onto
//! Forms input boxes ([`bridge`]) and the proposal ([`proposal`]).
//!
//! Books holds no compliance logic (ADR 0007): no rate, election,
//! threshold, ATC choice, tax due, rounding, due date or penalty.

pub mod bridge;
pub mod canonical;
pub mod catalog;
pub mod date;
pub mod engine_error;
pub mod input;
pub mod ledger;
pub mod measures;
pub mod money;
pub mod posting;
pub mod privacy;
pub mod proposal;
pub mod rules;
pub mod statement;
