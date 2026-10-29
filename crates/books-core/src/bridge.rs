//! The bridge: accounting measures to Forms input boxes, with lineage.
//! Port of `poc/src/bridge.ts` (`runBridge()`, `checkFormContext()`,
//! `nontaxableThirteenthCentavos()` and what they use).
//!
//! The bridge reads Forms-provided choices (`form_context`) to pick between
//! alternative boxes and never decides, infers or defaults them. It holds
//! no rate, election, threshold, applicability test, ATC choice, due date
//! or penalty (ADR 0007). It enforces the period lock, return-event
//! acknowledgement, the MVP spouse refusal and payroll completeness, and
//! fails closed on mappings that are open for tax review. The one labelled
//! input-level tax rule is the 1601-C item 17 cap (`1601C.II.17`).

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::canonical::js_cmp;
use crate::date::{Date, Period};
use crate::engine_error::EngineError;
use crate::input::{FormCode, FormContext, NormProfile};
use crate::measures::{
    MeasureResult, MeasureRow, Window, compute_measure, is_compensation_component,
};
use crate::money;
use crate::posting::{Ledger, in_range, money_text, to_centavos};
use crate::rules::{Aggregation, LoadedData, Mapping, RuleSet};

/// A blocking item or warning (`Item`). `ids` is empty when the PoC omits it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub code: String,
    pub detail: String,
    pub ids: Vec<String>,
}

/// One lineage row of a proposed value (`Lineage`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lineage {
    pub line_id: String,
    pub entry_id: String,
    pub amount: String,
}

/// A proposed value (`Val`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Val {
    pub key: String,
    pub value: String,
    pub mapping_id: String,
    /// "fill" or "reference".
    pub mode: String,
    pub measures: Vec<String>,
    pub lines: Vec<Lineage>,
    pub netted_pairs: i64,
    pub note: Option<String>,
}

/// What the bridge found (`Outcome`).
#[derive(Debug, Clone, Default)]
pub struct Outcome {
    pub blocking: Vec<Item>,
    pub warnings: Vec<Item>,
    pub values: Vec<Val>,
    pub measures: BTreeMap<String, MeasureResult>,
    pub excluded_reasons: HashMap<String, String>,
}

/// What the bridge reads (`BridgeCtx`). `period` must be of the form's
/// period kind.
pub struct BridgeCtx<'a> {
    pub data: &'a LoadedData,
    pub form: FormCode,
    pub profile: &'a NormProfile,
    pub ledger: &'a Ledger,
    pub period: &'a Period,
    pub acks: &'a HashSet<String>,
    pub form_context: &'a FormContext,
}

/// `COUNTED_ONCE` (REV.COUNT_ONCE): a line may feed only one value of its
/// group.
pub fn counted_once(form: FormCode) -> &'static [&'static str] {
    match form {
        FormCode::F2551Q => &["txtATCAmt1", "frm2551Qv2018:txt15"],
        FormCode::F1601C => &[
            "frm1601c:txtTax14",
            "frm1601c:txtTax19",
            "frm1601c:txtTax25",
        ],
        FormCode::F1701MS => &[
            "frm1701MS:txtSalesRevenueNo5a",
            "frm1701MS:txtSalesReturnsNo6a",
            "frm1701MS:txtCostofSalesNo8a",
            "frm1701MS:txtNonOperatingIncomeNo13a",
            "frm1701MS:txtSalesRevenuesFeesANo19a",
            "frm1701MS:txtOtherNonOperatingIncomeNo20a",
        ],
    }
}

/// `uniqueSorted()`.
pub(crate) fn unique_sorted<I: IntoIterator<Item = String>>(values: I) -> Vec<String> {
    let mut out: Vec<String> = values
        .into_iter()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    out.sort_by(|a, b| js_cmp(a, b));
    out
}

/// `push()`.
fn push(list: &mut Vec<Item>, code: &str, detail: String, ids: Vec<String>) {
    list.push(Item {
        code: code.to_string(),
        detail,
        ids: unique_sorted(ids),
    });
}

/// `lineage()`: rows as text, sorted by line id.
pub(crate) fn lineage(rows: &[MeasureRow]) -> Vec<Lineage> {
    let mut out: Vec<Lineage> = rows
        .iter()
        .map(|r| Lineage {
            line_id: r.line.line.line_id.clone(),
            entry_id: r.line.entry_id.clone(),
            amount: money::format(r.amount),
        })
        .collect();
    out.sort_by(|a, b| js_cmp(&a.line_id, &b.line_id));
    out
}

/// Port of `checkFormContext()`: the Forms context against the rule set's
/// declared inputs. Returns blocking items: an undeclared path, a missing
/// value that is required, or an unsupported value. Nothing is defaulted.
pub fn check_form_context(rs: &RuleSet, ctx: &FormContext) -> Vec<Item> {
    let mut items = Vec::new();
    let declared: HashSet<&str> = rs
        .form_context_inputs
        .iter()
        .map(|i| i.path.as_str())
        .collect();
    for key in ctx.keys() {
        if !declared.contains(key.as_str()) {
            items.push(Item {
                code: "form_context_undeclared".to_string(),
                detail: format!("{key} is not a Forms input this rule set reads"),
                ids: Vec::new(),
            });
        }
    }
    for input in &rs.form_context_inputs {
        let needed = input.required
            || input.required_when.as_ref().is_some_and(|when| {
                when.iter()
                    .all(|(path, value)| ctx.get(path).is_some_and(|v| v == value))
            });
        let Some(value) = ctx.get(&input.path) else {
            if needed {
                items.push(Item {
                    code: "form_context_missing".to_string(),
                    detail: format!(
                        "{} must be read from the Forms draft ({}); the bridge never decides or defaults it",
                        input.path, input.source
                    ),
                    ids: Vec::new(),
                });
            }
            continue;
        };
        if !input.supported_values.contains(value) {
            items.push(Item {
                code: "form_context_unsupported".to_string(),
                detail: format!(
                    "{} is {value}; this rule set maps {}",
                    input.path,
                    input.supported_values.join(", ")
                ),
                ids: Vec::new(),
            });
        }
    }
    items
}

/// `selected()`: every Forms-context condition of the mapping holds.
fn is_selected(m: &Mapping, ctx: &FormContext) -> bool {
    m.form_context
        .iter()
        .all(|c| ctx.get(&c.read).is_some_and(|v| *v == c.equals))
}

/// `windowFor()`.
fn window_for(m: &Mapping, period: &Period) -> Window {
    let year = period.start.year;
    match m.sub_window.as_deref() {
        Some("Q1-Q3") => Window {
            start: period.start.to_string(),
            end: format!("{year:04}-09-30"),
        },
        Some("Q4") => Window {
            start: format!("{year:04}-10-01"),
            end: period.end.to_string(),
        },
        _ => Window {
            start: period.start.to_string(),
            end: period.end.to_string(),
        },
    }
}

/// `measureKey()`.
fn measure_key(id: &str, m: &Mapping) -> String {
    match &m.sub_window {
        Some(w) if !w.is_empty() => format!("{id}[{w}]"),
        _ => id.to_string(),
    }
}

/// Port of `nontaxableThirteenthCentavos()`, 1601C.II.17 for one employee:
/// `min(paid in the month, max(0, cap - paid earlier in the year))`. The
/// one input-level tax rule in the bridge (see
/// data/rules/1601c-v2018.rules.json).
pub fn nontaxable_thirteenth_centavos(paid: i128, paid_earlier_in_year: i128, cap: i128) -> i128 {
    let room = cap - paid_earlier_in_year;
    let allowed = if room > 0 { room } else { 0 };
    if paid < allowed { paid } else { allowed }
}

/// `nontaxable13th()`: 1601C.II.17 summed over employees (by `party_ref`).
/// `None` when a line has no employee ref (then it blocks).
fn nontaxable_13th(
    m: &Mapping,
    month: &MeasureResult,
    ytd: &MeasureResult,
    o: &mut Outcome,
) -> Result<Option<i64>, EngineError> {
    let cap_text = m
        .aggregation_params
        .as_ref()
        .and_then(|p| p.get("annual_cap_text"))
        .ok_or_else(|| {
            EngineError::Money(format!(
                "{}.annual_cap_text: money must be decimal text, got a JS undefined",
                m.id
            ))
        })?;
    let cap = money::parse(cap_text)
        .map_err(|e| EngineError::Money(format!("{}.annual_cap_text: {e}", m.id)))?;
    let missing: Vec<String> = month
        .lines
        .iter()
        .chain(&ytd.lines)
        .filter(|r| r.line.line.party_ref.is_none())
        .map(|r| r.line.line.line_id.clone())
        .collect();
    if !missing.is_empty() {
        push(
            &mut o.blocking,
            "employee_ref_missing",
            format!(
                "{}: 13th-month lines need the employee's party_ref so the annual cap is applied per employee",
                m.id
            ),
            missing,
        );
        return Ok(None);
    }
    let sum_by = |rows: &[MeasureRow]| -> BTreeMap<String, i128> {
        let mut out = BTreeMap::new();
        for r in rows {
            if let Some(who) = &r.line.line.party_ref {
                *out.entry(who.clone()).or_insert(0) += i128::from(r.amount);
            }
        }
        out
    };
    let paid = sum_by(&month.lines);
    let before = sum_by(&ytd.lines);
    let mut total: i128 = 0;
    for (who, amount) in &paid {
        total += nontaxable_thirteenth_centavos(
            *amount,
            before.get(who).copied().unwrap_or(0),
            i128::from(cap),
        );
    }
    to_centavos(total, &m.id).map(Some)
}

fn param<'m>(m: &'m Mapping, name: &str) -> Result<&'m str, EngineError> {
    m.aggregation_params
        .as_ref()
        .and_then(|p| p.get(name))
        .map(String::as_str)
        .ok_or_else(|| EngineError::Data(format!("{}: aggregation_params.{name} is missing", m.id)))
}

/// Port of `runBridge()`.
pub fn run_bridge(ctx: &BridgeCtx<'_>) -> Result<Outcome, EngineError> {
    let BridgeCtx {
        data,
        form,
        profile,
        ledger,
        period,
        acks,
        form_context,
    } = *ctx;
    let rs = data.rules(form);
    let mut o = Outcome::default();
    let start = period.start.to_string();
    let end = period.end.to_string();

    // GATE.PERIOD_LOCK: measures are proposed from closed periods only.
    match profile.periods.get(&period.key) {
        None => push(
            &mut o.blocking,
            "period_unknown",
            format!("no status recorded for {}", period.key),
            Vec::new(),
        ),
        Some(status) if status != "closed" => push(
            &mut o.blocking,
            "GATE.PERIOD_LOCK",
            format!("{} is {status}; proposals need a closed period", period.key),
            Vec::new(),
        ),
        Some(_) => {}
    }

    // BRIDGE.FORM_CONTEXT_READ_ONLY: the Forms choices are inputs; missing
    // or unsupported values block.
    o.blocking.extend(check_form_context(rs, form_context));

    let fills: Vec<&Mapping> = rs
        .mappings
        .iter()
        .filter(|m| m.mode == "fill" || m.mode == "reference")
        .collect();
    let active: Vec<&Mapping> = fills
        .iter()
        .copied()
        .filter(|m| is_selected(m, form_context))
        .collect();
    for m in &fills {
        if active.iter().any(|a| std::ptr::eq(*a, *m)) {
            continue;
        }
        let why = m
            .form_context
            .iter()
            .map(|c| {
                format!(
                    "{} is {} (mapping needs {})",
                    c.read,
                    form_context.get(&c.read).map_or("unset", String::as_str),
                    c.equals
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        o.excluded_reasons.insert(
            m.target_key.clone(),
            format!("not selected by the Forms context: {why}"),
        );
    }

    let in_period: Vec<_> = ledger
        .lines
        .iter()
        .filter(|l| in_range(Some(&l.entry_date), &start, &end))
        .collect();

    // EXC.SPOUSE_MVP: spouse lines are measured but the bridge refuses to
    // map them in MVP.
    if active.iter().any(|m| m.person_scope == "taxpayer") {
        let spouse: Vec<String> = in_period
            .iter()
            .filter(|l| l.line.person_scope == "spouse" && l.measure != 0)
            .map(|l| l.line.line_id.clone())
            .collect();
        if !spouse.is_empty() {
            push(
                &mut o.blocking,
                "EXC.SPOUSE_MVP",
                "spouse-scoped lines are refused in MVP (phase P7)".to_string(),
                spouse,
            );
        }
    }

    // Payroll completeness: Books cannot tell an unrecorded payroll from
    // zero payroll, so it proposes nothing.
    if active.iter().any(|m| m.person_scope == "employee") {
        if !in_period.iter().any(|l| l.line.comp_component.is_some()) {
            push(
                &mut o.blocking,
                "no_payroll_lines",
                format!(
                    "no payroll lines are posted in {}; Books does not assert zero payroll",
                    period.key
                ),
                Vec::new(),
            );
        }
        let stray: Vec<String> = in_period
            .iter()
            .filter(|l| {
                is_compensation_component(l.line.comp_component.as_deref())
                    && l.line.person_scope != "employee"
            })
            .map(|l| l.line.line_id.clone())
            .collect();
        if !stray.is_empty() {
            push(
                &mut o.blocking,
                "comp_scope_not_employee",
                "compensation lines must carry person_scope employee; a stray line would be dropped silently".to_string(),
                stray,
            );
        }
    }

    let mut cache: HashMap<(String, Window), MeasureResult> = HashMap::new();
    let mut measure = |id: &str, window: &Window| -> Result<MeasureResult, EngineError> {
        let key = (id.to_string(), window.clone());
        if let Some(found) = cache.get(&key) {
            return Ok(found.clone());
        }
        let found = compute_measure(id, ledger, window)?;
        cache.insert(key, found.clone());
        Ok(found)
    };

    // Exclusion codes in first-seen order, each with its line ids.
    let mut excluded_by_code: Vec<(String, Vec<String>)> = Vec::new();
    for m in &active {
        let window = window_for(m, period);
        let results = m
            .measures
            .iter()
            .map(|id| measure(id, &window))
            .collect::<Result<Vec<_>, _>>()?;
        for (id, r) in m.measures.iter().zip(&results) {
            o.measures.insert(measure_key(id, m), r.clone());
        }

        for r in &results {
            for e in &r.excluded {
                match excluded_by_code
                    .iter_mut()
                    .find(|(code, _)| *code == e.code)
                {
                    Some((_, ids)) => ids.push(e.line_id.clone()),
                    None => excluded_by_code.push((e.code.clone(), vec![e.line_id.clone()])),
                }
            }
        }

        let unacked = unique_sorted(
            results
                .iter()
                .flat_map(|r| &r.return_events)
                .filter(|l| !acks.contains(&l.entry_id))
                .map(|l| l.entry_id.clone()),
        );
        if !unacked.is_empty() {
            push(
                &mut o.blocking,
                "GATE.RETURNS_ACK",
                format!(
                    "{}: return events (returns, allowances, discounts, credit notes, later-period reversals) need a person's acknowledgement",
                    m.id
                ),
                unacked,
            );
        }

        if m.review_status == "open" {
            push(
                &mut o.warnings,
                "MAPPING_REVIEW_OPEN",
                format!("{}: {}", m.id, m.review_note),
                Vec::new(),
            );
        }
        if let Some(rule) = m.input_level_tax_rule.as_deref().filter(|r| !r.is_empty()) {
            push(
                &mut o.warnings,
                "INPUT_LEVEL_TAX_RULE",
                format!("{}: {rule}", m.id),
                Vec::new(),
            );
        }

        let rows: Vec<MeasureRow> = results
            .iter()
            .flat_map(|r| r.lines.iter().cloned())
            .collect();
        if m.poc == "fail_closed" {
            let code = m.fail_closed_code.as_deref().unwrap_or("mapping_open");
            if rows.is_empty() {
                o.excluded_reasons.insert(
                    m.target_key.clone(),
                    format!("no source lines; mapping open for review ({code})"),
                );
            } else {
                push(
                    &mut o.blocking,
                    code,
                    format!("{}: {}", m.id, m.review_note),
                    rows.iter().map(|r| r.line.line.line_id.clone()).collect(),
                );
            }
            continue;
        }

        let sum: i128 = results.iter().map(|r| i128::from(r.value)).sum();
        let (value, note) = match m.aggregation {
            Aggregation::Sum => (money_text(sum), None),
            Aggregation::ChoiceYesIfNonzero => {
                let yes = param(m, "yes")?;
                let no = param(m, "no")?;
                (
                    if sum != 0 { yes } else { no }.to_string(),
                    Some(format!(
                        "{yes} when {} is not zero, else {no}",
                        m.measures.join(" + ")
                    )),
                )
            }
            Aggregation::Nontaxable13thMonthCap => {
                let ytd_measure = param(m, "ytd_measure")?;
                let day_before = Date::add_days(period.start, -1);
                let ytd_window = Window {
                    start: format!("{:04}-01-01", period.start.year),
                    end: day_before.to_string(),
                };
                let ytd = measure(ytd_measure, &ytd_window)?;
                o.measures
                    .insert(format!("{ytd_measure}[ytd_before_period]"), ytd.clone());
                let Some(split) = nontaxable_13th(m, &results[0], &ytd, &mut o)? else {
                    continue;
                };
                (
                    money::format(split),
                    Some(format!(
                        "non-taxable part after the annual cap of {} per employee; the one input-level tax rule in the bridge",
                        param(m, "annual_cap_text")?
                    )),
                )
            }
        };
        if m.non_negative && sum < 0 {
            push(
                &mut o.blocking,
                "negative_measure",
                format!(
                    "{}: {} is {}; the bridge does not propose a negative amount for this box",
                    m.id,
                    m.measures.join(" + "),
                    money_text(sum)
                ),
                Vec::new(),
            );
        }
        let netted_pairs = results.iter().map(|r| r.netted_pairs).sum();
        o.values.push(Val {
            key: m.target_key.clone(),
            value,
            mapping_id: m.id.clone(),
            mode: m.mode.clone(),
            measures: m.measures.clone(),
            lines: lineage(&rows),
            netted_pairs,
            note: note.filter(|n| !n.is_empty()),
        });
    }
    for (code, ids) in excluded_by_code {
        push(
            &mut o.warnings,
            &code,
            "excluded from the measure by rule; listed, never silently dropped".to_string(),
            ids,
        );
    }
    Ok(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_17_split() {
        let cap = 9_000_000;
        assert_eq!(nontaxable_thirteenth_centavos(9_500_000, 0, cap), 9_000_000);
        assert_eq!(
            nontaxable_thirteenth_centavos(8_000_000, 2_000_000, cap),
            7_000_000
        );
        assert_eq!(nontaxable_thirteenth_centavos(500_000, 9_500_000, cap), 0);
    }
}
