//! Entries and the rules a post enforces, ported from the PoC
//! (`poc/src/ledger.ts` `postLedger`, `poc/src/load.ts` `normalizeLine`
//! and `checkCashBasis`):
//!
//! - at least two lines; each line is a debit or a credit, never both,
//!   never negative;
//! - debits equal credits to the centavo;
//! - every fact is in its closed vocabulary; a measured line names its
//!   measure side; a `contra_sales` line names its `contra_kind`;
//! - a cash book (the default, ADR 0008) refuses trade receivable and
//!   payable lines and a sales line tagged `payment_receipt`.
//!
//! A posted entry never changes. A void marks it `voided` and posts a
//! mirrored reversal (owner decision: the original shows Voided; totals
//! skip both).

use std::fmt;

use crate::catalog::{Catalog, Facts};
use crate::date::Date;

/// The accounting basis of a book (ADR 0008). Cash is the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Basis {
    #[default]
    Cash,
    Accrual,
}

impl Basis {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "cash" => Some(Self::Cash),
            "accrual" => Some(Self::Accrual),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cash => "cash",
            Self::Accrual => "accrual",
        }
    }
}

/// An entry's state (plan machine 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Draft,
    Posted,
    Voided,
    Discarded,
}

impl Status {
    pub fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "draft" => Self::Draft,
            "posted" => Self::Posted,
            "voided" => Self::Voided,
            "discarded" => Self::Discarded,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Posted => "posted",
            Self::Voided => "voided",
            Self::Discarded => "discarded",
        }
    }
}

/// What an account means to the bridge (plan 2.2, "Map 1").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KindRef {
    /// One kind: its default facts apply to every line.
    Kind(String),
    /// Decided per line.
    Mixed,
    /// Not matched yet: its lines hold every box they could feed.
    Unsure,
}

impl KindRef {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "mixed" => Some(Self::Mixed),
            "unsure" => Some(Self::Unsure),
            id => Catalog::get().kind(id).map(|_| Self::Kind(id.to_string())),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Kind(id) => id,
            Self::Mixed => "mixed",
            Self::Unsure => "unsure",
        }
    }

    pub fn accrual_only(&self) -> bool {
        match self {
            Self::Kind(id) => Catalog::get().kind(id).is_some_and(|k| k.accrual_only),
            _ => false,
        }
    }

    /// A line's facts: the kind's defaults under the line's own facts,
    /// then the PoC's defaults.
    pub fn facts_for(&self, own: &Facts) -> Facts {
        match self {
            Self::Kind(id) => match Catalog::get().kind(id) {
                Some(k) => k.defaults.overlay(own).normalized(),
                None => own.normalized(),
            },
            Self::Mixed | Self::Unsure => own.normalized(),
        }
    }
}

/// One line as checked: amounts in centavos, the account's kind and the
/// line's effective facts.
#[derive(Debug, Clone)]
pub struct Line {
    pub account_id: i64,
    pub kind: KindRef,
    pub debit: i64,
    pub credit: i64,
    pub facts: Facts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerError {
    pub code: &'static str,
    pub message: String,
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for LedgerError {}

fn err(code: &'static str, message: impl Into<String>) -> LedgerError {
    LedgerError {
        code,
        message: message.into(),
    }
}

/// `^[A-Za-z0-9_-]{1,40}$`: an opaque party id, never a TIN or a name.
pub fn is_party_ref(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 40
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// The fact rules of `normalizeLine`, on effective facts. `n` is the
/// line's 1-based number, for the message.
pub fn check_facts(n: usize, f: &Facts) -> Result<(), LedgerError> {
    let c = Catalog::get();
    for name in [
        "measure_side",
        "income_class",
        "contra_kind",
        "document_kind",
        "person_scope",
        "comp_component",
        "contribution_side",
        "contribution_scheme",
        "credit_kind",
        "certificate_atc",
        "certificate_status",
        "final_tax_withheld",
    ] {
        if let Some(v) = f.get(name)
            && !c.allowed(name, v)
        {
            return Err(err(
                "bad_fact",
                format!("Line {n}: \"{v}\" is not a known {name}."),
            ));
        }
    }
    if let Some(p) = &f.party_ref
        && !is_party_ref(p)
    {
        return Err(err(
            "bad_fact",
            format!("Line {n}: party_ref must be an opaque id."),
        ));
    }
    for (name, v) in [
        ("period_start", &f.period_start),
        ("period_end", &f.period_end),
    ] {
        if let Some(v) = v
            && Date::parse(v).is_none()
        {
            return Err(err("bad_fact", format!("Line {n}: {name} is not a date.")));
        }
    }
    let income = f.income_class.as_deref().unwrap_or("none");
    let certificate = f
        .document_kind
        .as_deref()
        .is_some_and(|d| d.starts_with("certificate_"));
    if (income != "none" || f.comp_component.is_some() || certificate) && f.measure_side.is_none() {
        return Err(err(
            "bad_fact",
            format!("Line {n}: a measured line needs measure_side."),
        ));
    }
    if income == "contra_sales" && f.contra_kind.is_none() {
        return Err(err(
            "bad_fact",
            format!("Line {n}: a return, allowance or discount line needs contra_kind."),
        ));
    }
    if income != "contra_sales" && f.contra_kind.is_some() {
        return Err(err(
            "bad_fact",
            format!("Line {n}: contra_kind is only for contra_sales lines."),
        ));
    }
    Ok(())
}

/// Every rule a post enforces. Drafts may be saved unbalanced; posting runs
/// this first.
pub fn check(lines: &[Line], basis: Basis) -> Result<(), LedgerError> {
    if lines.len() < 2 {
        return Err(err("too_few_lines", "An entry needs at least two lines."));
    }
    let (mut debits, mut credits) = (0i128, 0i128);
    for (i, line) in lines.iter().enumerate() {
        let n = i + 1;
        if line.debit < 0 || line.credit < 0 {
            return Err(err(
                "line_sides",
                format!("Line {n} has a negative amount."),
            ));
        }
        if (line.debit > 0) == (line.credit > 0) {
            return Err(err(
                "line_sides",
                format!("Line {n} must have exactly one of a debit or a credit."),
            ));
        }
        debits += i128::from(line.debit);
        credits += i128::from(line.credit);
        check_facts(n, &line.facts)?;
        if basis == Basis::Cash {
            if line.kind.accrual_only() {
                return Err(err(
                    "accrual_account",
                    format!(
                        "Line {n} uses a receivable or payable account. A cash book records a sale when the money comes in and an expense when it is paid."
                    ),
                ));
            }
            if line.facts.income_class.as_deref() == Some("business_sales")
                && line.facts.document_kind.as_deref() == Some("payment_receipt")
            {
                return Err(err(
                    "cash_sales_receipt",
                    format!(
                        "Line {n}: on a cash book the sales line carries the invoice it settles, not a payment receipt."
                    ),
                ));
            }
        }
    }
    if debits != credits {
        return Err(err(
            "unbalanced",
            format!(
                "Debits {} do not equal credits {}.",
                crate::money::format(i64::try_from(debits).unwrap_or(i64::MAX)),
                crate::money::format(i64::try_from(credits).unwrap_or(i64::MAX)),
            ),
        ));
    }
    if i64::try_from(debits).is_err() {
        return Err(err("overflow", "The entry is too large."));
    }
    Ok(())
}

/// The reversal of posted lines: each line mirrored (debit and credit
/// swapped), every fact kept, so a measure of the reversal is the exact
/// negative of the original (`mirrors` in the PoC).
pub fn mirror(lines: &[Line]) -> Vec<Line> {
    lines
        .iter()
        .map(|l| Line {
            debit: l.credit,
            credit: l.debit,
            ..l.clone()
        })
        .collect()
}

/// A line's signed amount for measures that select it: credit − debit on
/// the credit side, debit − credit on the debit side, zero without a side.
pub fn measure(line: &Line) -> i64 {
    match line.facts.measure_side.as_deref() {
        Some("credit") => line.credit - line.debit,
        Some("debit") => line.debit - line.credit,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(kind: &str, debit: i64, credit: i64) -> Line {
        let kind = KindRef::parse(kind).unwrap();
        let facts = kind.facts_for(&Facts::default());
        Line {
            account_id: 1,
            kind,
            debit,
            credit,
            facts,
        }
    }

    #[test]
    fn a3_unbalanced_and_two_sided_lines_are_refused() {
        let e = check(
            &[
                line("bank_accounts", 10_000, 0),
                line("sales_revenue", 0, 9_999),
            ],
            Basis::Cash,
        )
        .unwrap_err();
        assert_eq!(e.code, "unbalanced");
        assert!(e.message.contains("100.00") && e.message.contains("99.99"));
        let e = check(
            &[
                line("bank_accounts", 1_000, 1_000),
                line("sales_revenue", 0, 0),
            ],
            Basis::Cash,
        )
        .unwrap_err();
        assert_eq!(e.code, "line_sides");
        assert_eq!(
            check(&[line("bank_accounts", 1, 0)], Basis::Cash)
                .unwrap_err()
                .code,
            "too_few_lines"
        );
        assert_eq!(
            check(
                &[line("bank_accounts", -1, 0), line("sales_revenue", 0, -1)],
                Basis::Cash
            )
            .unwrap_err()
            .code,
            "line_sides"
        );
    }

    #[test]
    fn a_balanced_cash_sale_posts() {
        let lines = [
            line("bank_accounts", 8_000_000, 0),
            line("sales_revenue", 0, 8_000_000),
        ];
        assert_eq!(check(&lines, Basis::Cash), Ok(()));
        assert_eq!(measure(&lines[1]), 8_000_000);
        assert_eq!(measure(&lines[0]), 0);
    }

    #[test]
    fn a_cash_book_refuses_receivables_payables_and_receipt_tagged_sales() {
        let ar = [
            line("accounts_receivable_gross", 100, 0),
            line("sales_revenue", 0, 100),
        ];
        assert_eq!(check(&ar, Basis::Cash).unwrap_err().code, "accrual_account");
        assert_eq!(check(&ar, Basis::Accrual), Ok(()));
        let ap = [
            line("office_supplies", 100, 0),
            line("accounts_payable", 0, 100),
        ];
        assert_eq!(check(&ap, Basis::Cash).unwrap_err().code, "accrual_account");
        let mut sale = line("sales_revenue", 0, 100);
        sale.facts.document_kind = Some("payment_receipt".into());
        let e = check(&[line("bank_accounts", 100, 0), sale.clone()], Basis::Cash).unwrap_err();
        assert_eq!(e.code, "cash_sales_receipt");
        assert_eq!(
            check(&[line("bank_accounts", 100, 0), sale], Basis::Accrual),
            Ok(())
        );
    }

    #[test]
    fn fact_rules_follow_normalize_line() {
        let mut refund = line("refunds_and_discounts", 100, 0);
        assert_eq!(refund.facts.contra_kind.as_deref(), Some("return"));
        refund.facts.contra_kind = None;
        let e = check(&[refund, line("bank_accounts", 0, 100)], Basis::Cash).unwrap_err();
        assert!(e.message.contains("contra_kind"));
        let mut odd = line("bank_accounts", 100, 0);
        odd.facts.contra_kind = Some("return".into());
        assert_eq!(
            check(&[odd, line("sales_revenue", 0, 100)], Basis::Cash)
                .unwrap_err()
                .code,
            "bad_fact"
        );
        let mut bad = line("bank_accounts", 100, 0);
        bad.facts.party_ref = Some("Juan Dela Cruz".into());
        assert_eq!(
            check(&[bad, line("sales_revenue", 0, 100)], Basis::Cash)
                .unwrap_err()
                .code,
            "bad_fact"
        );
        let mut unmeasured = line("sales_revenue", 0, 100);
        unmeasured.facts.measure_side = None;
        assert!(
            check(&[line("bank_accounts", 100, 0), unmeasured], Basis::Cash)
                .unwrap_err()
                .message
                .contains("measure_side")
        );
    }

    #[test]
    fn a4_a_reversal_mirrors_and_nets_to_zero() {
        let lines = vec![
            line("bank_accounts", 10_000_000, 0),
            line("sales_revenue", 0, 10_000_000),
        ];
        let rev = mirror(&lines);
        assert_eq!(check(&rev, Basis::Cash), Ok(()));
        let net: i64 = lines.iter().chain(&rev).map(measure).sum();
        assert_eq!(net, 0);
        assert_eq!(rev[1].facts, lines[1].facts);
    }

    #[test]
    fn unsure_and_mixed_accounts_carry_no_defaults() {
        let unsure = KindRef::parse("unsure").unwrap();
        let f = unsure.facts_for(&Facts::default());
        assert_eq!(f.income_class.as_deref(), Some("none"));
        assert_eq!(f.measure_side, None);
        assert!(KindRef::parse("not_a_kind").is_none());
    }
}
