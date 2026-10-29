//! The accounting measures of `data/measures.v1.json`. Port of
//! `poc/src/measures.ts` (`computeMeasure()` and its measure table).
//!
//! Each measure selects posted lines by their accounting facts, never by
//! account, and sums their signed centavos exactly. No rate, election,
//! threshold or rounding is applied here (ADR 0007). Every value carries
//! its lineage (the line ids and signed amounts that sum to it), same-
//! window voids are netted out of the lineage, and lines in the window that
//! a measure deliberately leaves out are listed with the rule that excludes
//! them.
//!
//! The catalogue lists 20 measures. The PoC implements 19 of them;
//! `certificates_2316_received` is `planned` there and fails closed here
//! with `measure_not_implemented`, as it does in the PoC.

use std::collections::{BTreeMap, HashSet};

use crate::engine_error::EngineError;
use crate::posting::{Ledger, PostedLine, in_range, same_window_reversal_entries, to_centavos};

/// A measurement window, both ends included (ISO days).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Window {
    pub start: String,
    pub end: String,
}

/// A line a measure leaves out on purpose, and the rule code that does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Excluded {
    pub code: String,
    pub line_id: String,
}

/// One lineage row: a line and its signed contribution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureRow {
    pub line: PostedLine,
    pub amount: i64,
}

/// `MeasureResult`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasureResult {
    pub id: String,
    pub window: Window,
    pub value: i64,
    /// Lines that make up the value, with their signed contribution.
    /// Same-window void pairs are netted out.
    pub lines: Vec<MeasureRow>,
    pub netted_pairs: i64,
    pub breakdown: Option<BTreeMap<String, i64>>,
    /// Lines in the window the measure leaves out, with the excluding rule.
    pub excluded: Vec<Excluded>,
    /// Sales-family lines that are return events (GATE.RETURNS_ACK).
    pub return_events: Vec<PostedLine>,
}

/// `COMPENSATION_COMPONENTS`.
pub const COMPENSATION_COMPONENTS: [&str; 5] = [
    "basic_pay",
    "statutory_minimum_wage",
    "holiday_ot_nsd_hazard",
    "thirteenth_month_and_benefits",
    "de_minimis",
];

pub fn is_compensation_component(component: Option<&str>) -> bool {
    component.is_some_and(|c| COMPENSATION_COMPONENTS.contains(&c))
}

/// `IMPLEMENTED_MEASURES`, in the PoC's order.
pub const IMPLEMENTED_MEASURES: [&str; 19] = [
    "gross_sales",
    "sales_returns",
    "sales_allowances",
    "sales_discounts",
    "net_sales",
    "cost_of_sales",
    "operating_expenses_by_category",
    "other_non_operating_income",
    "gross_compensation",
    "basic_pay",
    "mwe_basic_pay",
    "holiday_ot_nsd_hazard_pay",
    "thirteenth_month_and_benefits_paid",
    "de_minimis_paid",
    "employee_mandatory_contributions",
    "employer_mandatory_contributions",
    "tax_withheld_from_employees",
    "pt_withheld_2307_received",
    "it_withheld_2307_received",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Basis {
    /// The entry date is in the window.
    Entry,
    /// The certificate's `period_end` is in the window.
    Certificate,
}

/// `Spec`: how one measure selects and signs its lines.
struct Spec {
    basis: Basis,
    select: fn(&PostedLine) -> bool,
    /// The contribution is the negated line measure (contra-sales measures
    /// are reported as positive amounts).
    negate: bool,
    breakdown_by: Option<fn(&PostedLine) -> String>,
    exclude: Option<fn(&PostedLine) -> Option<&'static str>>,
    sales_family: bool,
}

fn own(l: &PostedLine) -> bool {
    l.line.person_scope == "self"
}

fn employee(l: &PostedLine) -> bool {
    l.line.person_scope == "employee"
}

fn is_sale(l: &PostedLine) -> bool {
    l.line.income_class == "business_sales" && l.line.document_kind != "payment_receipt" && own(l)
}

fn is_contra(l: &PostedLine, kind: &str) -> bool {
    l.line.income_class == "contra_sales" && l.line.contra_kind.as_deref() == Some(kind) && own(l)
}

fn is_any_contra(l: &PostedLine) -> bool {
    l.line.income_class == "contra_sales" && own(l)
}

fn component(l: &PostedLine, name: &str) -> bool {
    l.line.comp_component.as_deref() == Some(name) && employee(l)
}

fn is_contribution(l: &PostedLine, side: &str) -> bool {
    l.line.comp_component.as_deref() == Some("contribution") && l.line.contribution_side == side
}

fn scheme(l: &PostedLine) -> String {
    l.line
        .contribution_scheme
        .clone()
        .unwrap_or_else(|| "unspecified".to_string())
}

/// `certificateExclusion()`.
fn certificate_exclusion(l: &PostedLine, kind: &str) -> Option<&'static str> {
    let doc = l.line.document_kind.as_str();
    if !doc.starts_with("certificate_") {
        return None;
    }
    if doc == "certificate_2306" {
        return Some("EXC.WB084_FINAL");
    }
    if doc != "certificate_2307" {
        return None;
    }
    if l.line.credit_kind != kind {
        return Some(if kind == "pt" {
            "EXC.IT_CWT_NOT_PT"
        } else {
            "EXC.PT_CWT_NOT_IT"
        });
    }
    if l.line.certificate_status.as_deref() != Some("received") {
        return Some("EXC.UNRECEIVED_CERT");
    }
    None
}

/// `certificate()`.
fn certificate(l: &PostedLine, kind: &str) -> bool {
    l.line.document_kind == "certificate_2307"
        && l.line.credit_kind == kind
        && l.line.certificate_status.as_deref() == Some("received")
}

const fn entry(select: fn(&PostedLine) -> bool) -> Spec {
    Spec {
        basis: Basis::Entry,
        select,
        negate: false,
        breakdown_by: None,
        exclude: None,
        sales_family: false,
    }
}

/// The measure table (`SPECS`).
fn spec(id: &str) -> Option<Spec> {
    let sales = |select: fn(&PostedLine) -> bool, negate: bool| Spec {
        negate,
        sales_family: true,
        ..entry(select)
    };
    Some(match id {
        "gross_sales" => sales(is_sale, false),
        "sales_returns" => sales(|l| is_contra(l, "return"), true),
        "sales_allowances" => sales(|l| is_contra(l, "allowance"), true),
        "sales_discounts" => sales(|l| is_contra(l, "discount"), true),
        "net_sales" => sales(|l| is_sale(l) || is_any_contra(l), false),
        "cost_of_sales" => entry(|l| l.line.income_class == "cost_of_sales" && own(l)),
        "operating_expenses_by_category" => Spec {
            breakdown_by: Some(|l| l.line.account.clone()),
            ..entry(|l| l.line.income_class == "operating_expense" && own(l))
        },
        "other_non_operating_income" => {
            entry(|l| l.line.income_class == "non_operating_income" && own(l))
        }
        "gross_compensation" => {
            entry(|l| employee(l) && is_compensation_component(l.line.comp_component.as_deref()))
        }
        "basic_pay" => entry(|l| component(l, "basic_pay")),
        "mwe_basic_pay" => entry(|l| component(l, "statutory_minimum_wage")),
        "holiday_ot_nsd_hazard_pay" => entry(|l| component(l, "holiday_ot_nsd_hazard")),
        "thirteenth_month_and_benefits_paid" => {
            entry(|l| component(l, "thirteenth_month_and_benefits"))
        }
        "de_minimis_paid" => entry(|l| component(l, "de_minimis")),
        "employee_mandatory_contributions" => Spec {
            breakdown_by: Some(scheme),
            exclude: Some(|l| {
                is_contribution(l, "employer").then_some("EXC.EMPLOYER_SHARE_NOT_19")
            }),
            ..entry(|l| is_contribution(l, "employee"))
        },
        "employer_mandatory_contributions" => Spec {
            breakdown_by: Some(scheme),
            ..entry(|l| {
                is_contribution(l, "employer") && l.line.measure_side.as_deref() == Some("debit")
            })
        },
        "tax_withheld_from_employees" => {
            entry(|l| l.line.comp_component.as_deref() == Some("tax_withheld"))
        }
        "pt_withheld_2307_received" => Spec {
            basis: Basis::Certificate,
            exclude: Some(|l| certificate_exclusion(l, "pt")),
            ..entry(|l| certificate(l, "pt"))
        },
        "it_withheld_2307_received" => Spec {
            basis: Basis::Certificate,
            exclude: Some(|l| certificate_exclusion(l, "it")),
            ..entry(|l| certificate(l, "it"))
        },
        _ => return None,
    })
}

/// `returnEventReason()`: why a sales-family line is a return event.
fn return_event_reason(l: &PostedLine, window: &Window) -> Option<&'static str> {
    if l.line.income_class == "contra_sales" {
        return Some("contra_sales");
    }
    if l.line.document_kind == "credit_note" {
        return Some("credit_note");
    }
    if l.reverses_entry_id.is_some()
        && !in_range(l.original_entry_date.as_deref(), &window.start, &window.end)
    {
        return Some("later_period_reversal");
    }
    None
}

/// Port of `computeMeasure()`: one measure over one window, with lineage.
/// Fails with `measure_not_implemented` for a measure the PoC does not
/// implement (fail closed).
pub fn compute_measure(
    id: &str,
    ledger: &Ledger,
    window: &Window,
) -> Result<MeasureResult, EngineError> {
    let spec = spec(id).ok_or_else(|| {
        EngineError::ledger(
            "measure_not_implemented",
            format!("{id} is not implemented in the PoC"),
        )
    })?;
    let date_of = |l: &PostedLine| -> Option<String> {
        match spec.basis {
            Basis::Entry => Some(l.entry_date.clone()),
            Basis::Certificate => l.line.period_end.clone(),
        }
    };
    let in_window: Vec<&PostedLine> = ledger
        .lines
        .iter()
        .filter(|l| in_range(date_of(l).as_deref(), &window.start, &window.end))
        .collect();
    let selected: Vec<&PostedLine> = in_window
        .iter()
        .copied()
        .filter(|l| (spec.select)(l))
        .collect();
    let amount_of = |l: &PostedLine| -> i64 { if spec.negate { -l.measure } else { l.measure } };
    let value: i128 = selected.iter().map(|l| i128::from(amount_of(l))).sum();

    // Same-window voids net to zero: both sides are left out of lineage,
    // and the value is unchanged.
    let netted = if spec.basis == Basis::Entry {
        same_window_reversal_entries(selected.iter().copied(), &window.start, &window.end)
    } else {
        HashSet::new()
    };
    let kept: Vec<&PostedLine> = selected
        .iter()
        .copied()
        .filter(|l| !netted.contains(&l.entry_id))
        .collect();
    let kept_value: i128 = kept.iter().map(|l| i128::from(amount_of(l))).sum();
    if kept_value != value {
        return Err(EngineError::ledger(
            "netting_mismatch",
            format!("{id}: netted lines do not sum to zero; a reversal is not an exact mirror"),
        ));
    }
    let netted_pairs = selected
        .iter()
        .filter(|l| l.reverses_entry_id.is_some() && netted.contains(&l.entry_id))
        .map(|l| l.entry_id.as_str())
        .collect::<HashSet<_>>()
        .len();

    let breakdown = match spec.breakdown_by {
        None => None,
        Some(key_of) => {
            let mut sums: BTreeMap<String, i128> = BTreeMap::new();
            for l in &selected {
                *sums.entry(key_of(l)).or_insert(0) += i128::from(amount_of(l));
            }
            let mut out = BTreeMap::new();
            for (key, sum) in sums {
                let centavos = to_centavos(sum, id)?;
                out.insert(key, centavos);
            }
            Some(out)
        }
    };

    let mut excluded = Vec::new();
    if let Some(exclude) = spec.exclude {
        for l in &in_window {
            if selected.iter().any(|s| std::ptr::eq(*s, *l)) {
                continue;
            }
            if let Some(code) = exclude(l) {
                excluded.push(Excluded {
                    code: code.to_string(),
                    line_id: l.line.line_id.clone(),
                });
            }
        }
    }

    let return_events = if spec.sales_family {
        selected
            .iter()
            .filter(|l| return_event_reason(l, window).is_some())
            .map(|l| (*l).clone())
            .collect()
    } else {
        Vec::new()
    };

    Ok(MeasureResult {
        id: id.to_string(),
        window: window.clone(),
        value: to_centavos(value, id)?,
        lines: kept
            .iter()
            .map(|l| MeasureRow {
                line: (*l).clone(),
                amount: amount_of(l),
            })
            .collect(),
        netted_pairs: i64::try_from(netted_pairs).unwrap_or(i64::MAX),
        breakdown,
        excluded,
        return_events,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{NormEntry, NormLine};
    use crate::posting::post_ledger;

    fn line(id: &str, account: &str, debit: &str, credit: &str, sale: bool) -> NormLine {
        NormLine {
            line_id: id.into(),
            account: account.into(),
            debit: debit.into(),
            credit: credit.into(),
            measure_side: sale.then(|| "credit".into()),
            income_class: if sale { "business_sales" } else { "none" }.into(),
            contra_kind: None,
            document_kind: if sale { "sale_invoice" } else { "other" }.into(),
            person_scope: "self".into(),
            party_ref: None,
            comp_component: None,
            contribution_side: "none".into(),
            contribution_scheme: None,
            credit_kind: "none".into(),
            certificate_atc: None,
            certificate_status: None,
            period_start: None,
            period_end: None,
            final_tax_withheld: "unknown".into(),
        }
    }

    fn entry(id: &str, date: &str, reverses: Option<&str>, amount: &str, void: bool) -> NormEntry {
        let (d, c) = if void {
            ("0.00", amount)
        } else {
            (amount, "0.00")
        };
        NormEntry {
            entry_id: id.into(),
            tax_profile_id: "p".into(),
            entry_date: date.into(),
            reverses_entry_id: reverses.map(Into::into),
            lines: vec![
                line(&format!("{id}.1"), "bank_accounts", d, c, false),
                line(&format!("{id}.2"), "sales_revenue", c, d, true),
            ],
        }
    }

    #[test]
    fn sums_with_lineage_and_nets_same_window_voids() {
        let ledger = post_ledger(&[
            entry("S-1", "2026-07-05", None, "100.00", false),
            entry("S-2", "2026-07-06", None, "40.00", false),
            entry("S-3", "2026-07-09", Some("S-2"), "40.00", true),
        ])
        .unwrap();
        let q3 = Window {
            start: "2026-07-01".into(),
            end: "2026-09-30".into(),
        };
        let gross = compute_measure("gross_sales", &ledger, &q3).unwrap();
        assert_eq!(gross.value, 10_000);
        assert_eq!(gross.netted_pairs, 1);
        assert_eq!(gross.lines.len(), 1);
        assert_eq!(
            (
                gross.lines[0].line.line.line_id.as_str(),
                gross.lines[0].amount
            ),
            ("S-1.2", 10_000)
        );
        assert!(gross.return_events.is_empty());

        let err = compute_measure("certificates_2316_received", &ledger, &q3).unwrap_err();
        assert_eq!(err.code(), Some("measure_not_implemented"));
    }
}
