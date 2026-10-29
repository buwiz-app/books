//! The errors of the measures engine and the bridge. The PoC throws one
//! class per concern (`InputError`, `LedgerError`, `CrosswalkViolation`,
//! `ProposalError`, `MoneyError`); here they are one enum so a caller can
//! match on the concern and, where the PoC has one, on the code.

use std::fmt;

use crate::canonical::CanonicalError;
use crate::money::MoneyError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    /// `InputError` (poc/src/load.ts): the input breaks a closed vocabulary,
    /// carries a compliance attribute or breaks the cash-basis rule.
    Input(String),
    /// `LedgerError` (poc/src/ledger.ts), with its code: unbalanced,
    /// line_sides, duplicate_id, orphan_reversal, reversed_twice,
    /// reversal_not_mirror, reversal_before_original, measure_not_implemented,
    /// netting_mismatch, counted_twice, and money_overflow (Rust only: a sum
    /// beyond `i64`, which the PoC's bigint never meets).
    Ledger { code: String, detail: String },
    /// `CrosswalkViolation` (poc/src/proposal.ts): a value the bridge must
    /// never send.
    Crosswalk(String),
    /// `ProposalError` (poc/src/proposal.ts), with its code: no_profile,
    /// profile_required, period_kind, bad_period, not_ready, stale.
    Proposal { code: String, message: String },
    /// `MoneyError` (poc/src/money.ts).
    Money(String),
    /// A value outside canonical JSON (poc/src/canonical.ts).
    Canonical(String),
    /// A data file that does not load or whose pins disagree (loadData()).
    Data(String),
}

impl EngineError {
    pub fn ledger(code: &str, detail: impl Into<String>) -> Self {
        Self::Ledger {
            code: code.to_string(),
            detail: detail.into(),
        }
    }

    pub fn proposal(code: &str, message: impl Into<String>) -> Self {
        Self::Proposal {
            code: code.to_string(),
            message: message.into(),
        }
    }

    pub fn input(message: impl Into<String>) -> Self {
        Self::Input(message.into())
    }

    /// The PoC error code, where the PoC class has one.
    pub fn code(&self) -> Option<&str> {
        match self {
            Self::Ledger { code, .. } | Self::Proposal { code, .. } => Some(code),
            _ => None,
        }
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ledger { code, detail } => write!(f, "{code}: {detail}"),
            Self::Proposal { code, message } => write!(f, "{code}: {message}"),
            Self::Input(m)
            | Self::Crosswalk(m)
            | Self::Money(m)
            | Self::Canonical(m)
            | Self::Data(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for EngineError {}

impl From<CanonicalError> for EngineError {
    fn from(e: CanonicalError) -> Self {
        Self::Canonical(e.0)
    }
}

impl From<MoneyError> for EngineError {
    fn from(e: MoneyError) -> Self {
        Self::Money(e.0)
    }
}
