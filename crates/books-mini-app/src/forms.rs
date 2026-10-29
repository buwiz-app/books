//! M5/M6: the bridge over the store. The book becomes the PoC's input
//! (`buwiz-poc-fixture/3`), normalized by the ported loader (closed
//! vocabularies, the cash-basis rule), then the ported engine builds the
//! proposal (`books_core::proposal::build_proposal`), the same code the 17
//! vectors prove.
//!
//! - **Holds (plan 2.2).** A line on an account that is "not sure yet", or
//!   imported and not confirmed, or mixed without its own facts, goes in
//!   with neutral facts and holds every box its account type could feed:
//!   those boxes are not proposed, and the hold says why.
//! - **Opening totals (plan 2.3).** Each comes in as one opening entry on
//!   its last day, so a measure counts it and its lineage shows it.
//! - **Periods.** A quarter or a year is closed when every month in it is.
//!
//! Books computes no tax (ADR 0007): the proposal holds measures and input
//! boxes only; Forms computes the rest.

use std::collections::{BTreeMap, BTreeSet};

use books_core::catalog::{AccountType, Catalog, Facts};
use books_core::date::{Date, MONTHS, Period, PeriodKind};
use books_core::input::{FormCode, form_context_from_value, normalize_fixture};
use books_core::ledger::{KindRef, Status};
use books_core::money;
use books_core::proposal::{Proposal, build_proposal};
use books_core::rules;
use books_store::Store;
use books_store::books::{self, Account, Book};
use books_store::certificates;
use books_store::entries::{self, Filter};
use books_store::imports;
use books_store::months;
use serde_json::{Map, Value, json};

use crate::service::{Origin, Refusal, text};

/// A line the bridge cannot read yet, and the boxes it holds.
#[derive(Debug, Clone)]
pub struct Held {
    pub account: String,
    pub why: &'static str,
    pub entry_id: i64,
    pub date: Date,
    pub measures: BTreeSet<String>,
}

fn same_side(a: AccountType, b: AccountType) -> bool {
    let side = |t: AccountType| match t {
        AccountType::Revenue | AccountType::OtherIncome => 1,
        AccountType::CostOfRevenue | AccountType::Expense | AccountType::OtherExpense => 2,
        AccountType::Asset => 3,
        AccountType::Liability => 4,
        AccountType::Equity => 5,
    };
    side(a) == side(b)
}

/// The measures a line on an account of `ty` could feed (`data/measures.v1.json`):
/// those drawn from a kind on the same side, compensation measures (picked by
/// facts alone) for expense accounts, and the measures derived from those.
fn measures_of_type(ty: AccountType) -> BTreeSet<String> {
    let catalog = Catalog::get();
    let Ok(data) = rules::shared() else {
        return BTreeSet::new();
    };
    let list = data.measures["measures"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut out = BTreeSet::new();
    for m in &list {
        let from = &m["derived_from"];
        let cats: Vec<&str> = from["categories"]
            .as_array()
            .map(|a| a.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        let hit = if cats.is_empty() {
            from["measures"].is_null() && same_side(ty, AccountType::Expense)
        } else {
            cats.iter().any(|c| {
                catalog
                    .kind(c)
                    .is_some_and(|k| same_side(k.account_type, ty))
            })
        };
        if hit && let Some(id) = m["id"].as_str() {
            out.insert(id.to_string());
        }
    }
    for m in &list {
        let inputs = m["derived_from"]["measures"].as_array();
        if inputs.is_some_and(|a| a.iter().filter_map(Value::as_str).any(|x| out.contains(x)))
            && let Some(id) = m["id"].as_str()
        {
            out.insert(id.to_string());
        }
    }
    out
}

/// A stable opaque id for the book's profile in the bridge input: the
/// linked tax profile's id, else one derived from the store (never a TIN).
fn profile_id(store: &Store, book: &Book) -> String {
    if let Some(id) = &book.cloud_tax_profile_id {
        return id.clone();
    }
    let seed = store.meta("store_id").ok().flatten().unwrap_or_default();
    let digest = crate::jcs::sha256_hex(format!("{seed}:{}", book.id).as_bytes());
    format!(
        "{}-{}-4{}-8{}-{}",
        &digest[0..8],
        &digest[8..12],
        &digest[13..16],
        &digest[17..20],
        &digest[20..32]
    )
}

fn line_json(line_id: &str, account: &str, debit: i64, credit: i64, f: &Facts) -> Value {
    let mut l = json!({ "line_id": line_id, "account": account, "debit": money::format(debit), "credit": money::format(credit) });
    let m = l.as_object_mut().expect("an object");
    for (k, v) in f.to_json().as_object().into_iter().flatten() {
        m.insert(k.clone(), v.clone());
    }
    l
}

/// The kind and side an opening total goes in as, with the kind's own
/// line facts (and the contra kind a returns figure needs).
fn opening_shape(measure: &str) -> Option<(&'static str, bool, Facts)> {
    let (kind, debit, contra) = match measure {
        "gross_sales" => ("sales_revenue", false, None),
        "sales_returns" => ("refunds_and_discounts", true, Some("return")),
        "sales_allowances" => ("refunds_and_discounts", true, Some("allowance")),
        "sales_discounts" => ("refunds_and_discounts", true, Some("discount")),
        "cost_of_sales" => ("cost_of_goods_sold", true, None),
        "other_non_operating_income" => ("other_non_operating_income", false, None),
        "basic_pay" => ("salaries", true, None),
        "employee_mandatory_contributions" => ("ph_sss_payable", false, None),
        _ => return None,
    };
    let own = Facts {
        contra_kind: contra.map(str::to_string),
        ..Facts::default()
    };
    Some((
        kind,
        debit,
        KindRef::Kind(kind.into()).facts_for(&own).normalized(),
    ))
}

/// The book as the bridge's input, and the lines it holds.
pub fn fixture(store: &Store, book: &Book) -> Result<(Value, Vec<Held>), Refusal> {
    let c = store.conn();
    let pid = profile_id(store, book);
    let accounts: BTreeMap<i64, Account> = books::accounts(c, book.id)?
        .into_iter()
        .map(|a| (a.id, a))
        .collect();
    let overlay = certificates::statuses(c, book.id)?;
    let mut entries_json = Vec::new();
    let mut held = Vec::new();
    for e in entries::list(c, book.id, &Filter::default())?
        .into_iter()
        .filter(|e| matches!(e.status, Status::Posted | Status::Voided))
    {
        let mut lines = Vec::new();
        for l in entries::lines(c, e.id)? {
            let a = accounts
                .get(&l.account_id)
                .ok_or_else(|| Refusal::new("store", "A line names a missing account."))?;
            let classified = l.own.income_class.is_some()
                || l.own.comp_component.is_some()
                || l.own
                    .document_kind
                    .as_deref()
                    .is_some_and(|d| d.starts_with("certificate_"));
            let why = match &a.kind {
                KindRef::Unsure => Some("not sure yet"),
                KindRef::Mixed if !classified => {
                    Some("mixed, and this line has no facts of its own")
                }
                KindRef::Kind(_) if !a.kind_confirmed => Some("imported and not confirmed"),
                _ => None,
            };
            let id = format!("e{}.{}", e.id, l.no);
            match (why, &a.kind) {
                (Some(why), _) => {
                    // a void pair nets to nothing, and a recategorized line
                    // has moved to its real account
                    let settled = e.voided_by.is_some()
                        || e.reverses.is_some()
                        || e.recategorizes.is_some()
                        || (e.recategorized_by.is_some() && a.kind == KindRef::Unsure);
                    if !settled {
                        held.push(Held {
                            account: a.name.clone(),
                            why,
                            entry_id: e.id,
                            date: e.date,
                            measures: measures_of_type(a.account_type),
                        });
                    }
                    lines.push(line_json(
                        &id,
                        "bank_accounts",
                        l.debit,
                        l.credit,
                        &Facts::default().normalized(),
                    ));
                }
                (None, KindRef::Kind(k)) => {
                    let mut f = a.kind.facts_for(&l.own);
                    // a certificate's status as it stands now (the paper may have come later)
                    if f.document_kind
                        .as_deref()
                        .is_some_and(|d| d.starts_with("certificate_"))
                        && let Some(s) = overlay.get(&(e.reverses.unwrap_or(e.id), l.no))
                    {
                        f.certificate_status = Some(s.clone());
                    }
                    lines.push(line_json(&id, k, l.debit, l.credit, &f));
                }
                (None, _) => {
                    // a classified line of a mixed account: its own facts decide
                    lines.push(line_json(
                        &id,
                        "bank_accounts",
                        l.debit,
                        l.credit,
                        &l.own.normalized(),
                    ));
                }
            }
        }
        entries_json.push(json!({
            "entry_id": format!("e{}", e.id),
            "tax_profile_id": pid,
            "entry_date": e.date.to_string(),
            "reverses_entry_id": e.reverses.map(|r| format!("e{r}")),
            "lines": lines,
        }));
    }
    for o in imports::openings(c, book.id)? {
        let Some((kind, debit, f)) = opening_shape(&o.measure) else {
            continue;
        };
        let id = format!("open-{}-{}", o.measure.replace('_', "-"), o.from);
        let (d, cr) = if debit {
            (o.centavos, 0)
        } else {
            (0, o.centavos)
        };
        entries_json.push(json!({
            "entry_id": id.chars().take(40).collect::<String>(),
            "tax_profile_id": pid,
            "entry_date": o.to.to_string(),
            "reverses_entry_id": null,
            "lines": [
                line_json(&format!("{id}.1"), kind, d, cr, &f.normalized()),
                line_json(&format!("{id}.2"), "owners_equity", cr, d, &Facts::default().normalized()),
            ],
        }));
    }
    // periods: a month by its lock; a quarter or year when all its months are closed
    let today = Date::today();
    let mut periods = Map::new();
    let months_list = months::months(c, book.id, today)?;
    let mut by_quarter: BTreeMap<String, Vec<bool>> = BTreeMap::new();
    let mut by_year: BTreeMap<String, Vec<bool>> = BTreeMap::new();
    // months an opening total covers are kept outside Books, fixed by their
    // proof: closed, unless the book keeps that month itself
    let mut all: BTreeMap<String, (Date, bool)> = BTreeMap::new();
    for o in imports::openings(c, book.id)? {
        let mut m = o.from.month_start();
        while m <= o.to {
            all.insert(Period::of(m, PeriodKind::Month).key, (m, true));
            m = m.month_end().add_days(1);
        }
    }
    // a month before the book's first entry that the owner closed
    let mut stmt = c
        .prepare("SELECT period_key, status FROM period WHERE book_id = ?1")
        .map_err(books_store::StoreError::from)?;
    let locked = stmt
        .query_map([book.id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(books_store::StoreError::from)?;
    for row in locked {
        let (key, status) = row.map_err(books_store::StoreError::from)?;
        if let Some(p) = Period::parse(&key).filter(|p| p.kind == PeriodKind::Month) {
            all.insert(key, (p.start, status == "closed"));
        }
    }
    for (p, closed) in &months_list {
        all.insert(p.key.clone(), (p.start, *closed));
    }
    for (key, (start, closed)) in &all {
        periods.insert(key.clone(), json!(if *closed { "closed" } else { "open" }));
        by_quarter
            .entry(Period::of(*start, PeriodKind::Quarter).key)
            .or_default()
            .push(*closed);
        by_year
            .entry(Period::of(*start, PeriodKind::Year).key)
            .or_default()
            .push(*closed);
    }
    for (k, v) in by_quarter {
        periods.insert(
            k,
            json!(if v.len() == 3 && v.iter().all(|c| *c) {
                "closed"
            } else {
                "open"
            }),
        );
    }
    for (k, v) in by_year {
        periods.insert(
            k,
            json!(if v.len() == 12 && v.iter().all(|c| *c) {
                "closed"
            } else {
                "open"
            }),
        );
    }
    let acks: Vec<String> = imports::acknowledged(c, book.id)?
        .into_iter()
        .map(|e| format!("e{e}"))
        .collect();
    let fx = json!({
        "fixture_format": "buwiz-poc-fixture/3",
        "as_of": today.to_string(),
        "profiles": [{ "tax_profile_id": pid, "display": "***-***-***-00000", "accounting_basis": book.basis.as_str(), "periods": periods }],
        "acknowledged_entry_ids": acks,
        "entries": entries_json,
    });
    Ok((fx, held))
}

/// The proposal for `form` and `period`, with holds applied.
pub struct Built {
    pub proposal: Proposal,
    pub held: Vec<(String, String)>,
    pub held_lines: Vec<Held>,
    /// Totals that explain a box (sales beside returns, pay by component),
    /// over the form's period: display only, never sent.
    pub parts: BTreeMap<&'static str, i64>,
    /// While the period is still open: each summed box's total so far, to
    /// preview (display only; nothing is proposed or sent).
    pub preview: BTreeMap<String, i64>,
}

/// The totals the Forms rows show under a box.
const PARTS: [&str; 9] = [
    "gross_sales",
    "sales_returns",
    "sales_allowances",
    "sales_discounts",
    "basic_pay",
    "mwe_basic_pay",
    "holiday_ot_nsd_hazard_pay",
    "de_minimis_paid",
    "employer_mandatory_contributions",
];

pub fn build(
    store: &Store,
    book: &Book,
    form: FormCode,
    period: &str,
    ctx: &Value,
) -> Result<Built, Refusal> {
    let data = rules::shared().map_err(|e| Refusal::new("engine", e.to_string()))?;
    let (fx, held_lines) = fixture(store, book)?;
    let vocab = &data.vocab;
    let input =
        normalize_fixture(&fx, vocab).map_err(|e| Refusal::new("bridge_input", e.to_string()))?;
    let context = form_context_from_value(ctx).map_err(|e| Refusal::invalid(e.to_string()))?;
    let mut proposal = build_proposal(data, &input, form, period, &context, None)
        .map_err(|e| Refusal::new("bridge", e.to_string()))?;
    let p = Period::parse(period).ok_or_else(|| Refusal::invalid("No such period."))?;
    let in_window: Vec<&Held> = held_lines.iter().filter(|h| p.contains(h.date)).collect();
    let mut held = Vec::new();
    for m in &data.rules(form).mappings {
        let hit: Vec<&&Held> = in_window
            .iter()
            .filter(|h| m.measures.iter().any(|x| h.measures.contains(x)))
            .collect();
        if hit.is_empty() {
            continue;
        }
        let accounts: BTreeSet<&str> = hit.iter().map(|h| h.account.as_str()).collect();
        let why = format!(
            "{} line{} on {} {} not matched yet",
            hit.len(),
            if hit.len() == 1 { "" } else { "s" },
            accounts.into_iter().collect::<Vec<_>>().join(", "),
            if hit.len() == 1 { "is" } else { "are" }
        );
        proposal.fill_args.remove(&m.target_key);
        proposal.reference_values.remove(&m.target_key);
        held.push((m.target_key.clone(), why));
    }
    let mut parts = BTreeMap::new();
    if let Ok(ledger) = books_core::posting::post_ledger(&input.entries) {
        let window = books_core::measures::Window {
            start: p.start.to_string(),
            end: p.end.to_string(),
        };
        for id in PARTS {
            if let Ok(r) = books_core::measures::compute_measure(id, &ledger, &window) {
                parts.insert(id, r.value);
            }
        }
    }
    let mut preview = BTreeMap::new();
    let open_only = proposal.status == "blocked"
        && proposal
            .blocking
            .iter()
            .all(|b| b.code == "GATE.PERIOD_LOCK" || b.code.starts_with("form_context"));
    if open_only && let Ok(ledger) = books_core::posting::post_ledger(&input.entries) {
        let window = books_core::measures::Window {
            start: p.start.to_string(),
            end: p.end.to_string(),
        };
        for m in data.rules(form).mappings.iter().filter(|m| {
            m.aggregation == rules::Aggregation::Sum
                && !held.iter().any(|(k, _)| *k == m.target_key)
                // a box for a BIR choice not read, or not made, has no preview
                && m.form_context.iter().all(|c| context.get(&c.read) == Some(&c.equals))
        }) {
            // V.3 reads Q1 to Q3, V.4 reads Q4
            let window = match m.sub_window.as_deref() {
                Some("Q1-Q3") => books_core::measures::Window {
                    start: p.start.to_string(),
                    end: format!("{}-09-30", p.start.year),
                },
                Some("Q4") => books_core::measures::Window {
                    start: format!("{}-10-01", p.start.year),
                    end: p.end.to_string(),
                },
                _ => window.clone(),
            };
            let total: i64 = m
                .measures
                .iter()
                .filter_map(|id| books_core::measures::compute_measure(id, &ledger, &window).ok())
                .map(|r| r.value)
                .sum();
            preview.insert(m.target_key.clone(), total);
        }
    }
    Ok(Built {
        proposal,
        held,
        held_lines,
        parts,
        preview,
    })
}

fn form_arg(args: &Map<String, Value>) -> Result<FormCode, Refusal> {
    FormCode::parse(text(args, "form").unwrap_or("2551Q"))
        .ok_or_else(|| Refusal::invalid("`form` is 2551Q, 1601C or 1701MS."))
}

/// The default period of a form: the last full quarter or month, or this year.
pub fn default_period(form: FormCode) -> String {
    let today = Date::today();
    match form.period_kind() {
        PeriodKind::Quarter => Period::of(today, PeriodKind::Quarter).previous().key,
        PeriodKind::Month => Period::of(today, PeriodKind::Month).previous().key,
        PeriodKind::Year => Period::of(today, PeriodKind::Year).key,
    }
}

const RULES_2551Q: &str = include_str!("../../../data/rules/2551q-v2018.rules.json");
const RULES_1601C: &str = include_str!("../../../data/rules/1601c-v2018.rules.json");
const RULES_1701MS: &str = include_str!("../../../data/rules/1701ms-v2024.rules.json");

/// Who fills the form's boxes: Books, BIR, the profile, past returns, the user.
fn owners(form: FormCode) -> Value {
    let raw = match form {
        FormCode::F2551Q => RULES_2551Q,
        FormCode::F1601C => RULES_1601C,
        FormCode::F1701MS => RULES_1701MS,
    };
    let v: Value = serde_json::from_str(raw).unwrap_or(Value::Null);
    let count = |owners: &[&str]| {
        v["not_mapped"].as_array().map_or(0, |a| {
            a.iter()
                .filter(|x| owners.contains(&x["owner"].as_str().unwrap_or("")))
                .count()
        })
    };
    // the boxes Books fills, and the ones it shows BIR for reference
    let books = v["mappings"].as_array().map_or(0, Vec::len);
    json!({ "books": books, "bir": count(&["forms"]), "profile": count(&["profile", "identity"]), "past": count(&["prior_filing"]), "you": count(&["user"]) })
}

/// What a measure is called on the Forms page.
pub fn measure_label(id: &str) -> &'static str {
    match id {
        "gross_sales" => "Gross sales",
        "sales_returns" => "Sales returns",
        "sales_allowances" => "Sales allowances",
        "sales_discounts" => "Sales discounts",
        "net_sales" => "Net sales",
        "cost_of_sales" => "Cost of sales",
        "other_non_operating_income" => "Other non-operating income",
        "gross_compensation" => "Gross compensation",
        "basic_pay" => "Basic pay",
        "mwe_basic_pay" => "Minimum wage pay",
        "holiday_ot_nsd_hazard_pay" => "Holiday, overtime, night shift",
        "thirteenth_month_and_benefits_paid" => "13th month and other benefits",
        "de_minimis_paid" => "De minimis benefits",
        "employee_mandatory_contributions" => "Employee share of SSS, PhilHealth, Pag-IBIG",
        "employer_mandatory_contributions" => "Employer share",
        "tax_withheld_from_employees" => "Tax withheld, as payroll recorded it",
        "pt_withheld_2307_received" => "Percentage tax withheld, 2307s received",
        "it_withheld_2307_received" => "Income tax withheld, 2307s received",
        "certificates_2316_received" => "2316s received",
        "operating_expenses_by_category" => "Expenses (reports only)",
        _ => "A Books total",
    }
}

fn form_title(form: FormCode) -> (&'static str, &'static str) {
    match form {
        FormCode::F2551Q => ("2551Q", "Percentage tax"),
        FormCode::F1601C => ("1601-C", "Withholding on compensation"),
        FormCode::F1701MS => ("1701-MS", "Annual income tax"),
    }
}

fn period_label(form: FormCode, key: &str) -> String {
    let Some(p) = Period::parse(key) else {
        return key.to_string();
    };
    match form.period_kind() {
        PeriodKind::Quarter => format!(
            "{} · {}–{}",
            p.label(),
            MONTHS[usize::from(p.start.month - 1)],
            MONTHS[usize::from(p.end.month - 1)]
        ),
        PeriodKind::Month => p.label(),
        PeriodKind::Year => format!("Year {}", p.start.year),
    }
}

/// The line under a Forms row: what the total is made of.
fn row_sub(
    form: FormCode,
    measure: &str,
    parts: &BTreeMap<&'static str, i64>,
    certs: &[certificates::Certificate],
) -> String {
    let get = |k: &str| parts.get(k).copied().unwrap_or(0);
    let joined = |items: &[(&str, &str)], sep: &str| -> String {
        items
            .iter()
            .filter(|(k, _)| get(k) != 0)
            .map(|(k, label)| format!("{label} {}", money::display(get(k))))
            .collect::<Vec<_>>()
            .join(sep)
    };
    match measure {
        "net_sales" => {
            let less = joined(
                &[
                    ("sales_returns", "returns"),
                    ("sales_allowances", "allowances"),
                    ("sales_discounts", "discounts"),
                ],
                " − ",
            );
            let sales = format!("Sales {}", money::display(get("gross_sales")));
            if less.is_empty() {
                sales
            } else {
                format!("{sales} − {less}")
            }
        }
        "gross_sales" => "Before returns, allowances and discounts".into(),
        "gross_compensation" => joined(
            &[
                ("basic_pay", "Basic"),
                ("mwe_basic_pay", "Minimum wage"),
                ("holiday_ot_nsd_hazard_pay", "Overtime"),
                ("de_minimis_paid", "de minimis"),
            ],
            " + ",
        ),
        "employee_mandatory_contributions" if get("employer_mandatory_contributions") != 0 => {
            format!(
                "Employer share {} kept out",
                money::display(get("employer_mandatory_contributions"))
            )
        }
        "tax_withheld_from_employees" if form == FormCode::F1601C => {
            "Books records it; it does not compute it".into()
        }
        "pt_withheld_2307_received" | "it_withheld_2307_received" => {
            let kind = if measure.starts_with("pt") {
                "pt"
            } else {
                "it"
            };
            let mine: Vec<&certificates::Certificate> =
                certs.iter().filter(|c| c.credit_kind == kind).collect();
            let received = mine
                .iter()
                .filter(|c| c.form == "2307" && c.status == "received")
                .count();
            let waiting = mine
                .iter()
                .filter(|c| c.form == "2307" && c.status != "received")
                .count();
            let finals = mine.iter().filter(|c| c.form == "2306").count();
            let mut out = format!("{received} received");
            let left: Vec<String> = [(waiting, "pending"), (finals, "final (2306)")]
                .iter()
                .filter(|(n, _)| *n > 0)
                .map(|(n, what)| format!("{n} {what}"))
                .collect();
            if !left.is_empty() {
                out.push_str(&format!(" · {} left out", left.join(" and ")));
            }
            out
        }
        _ => String::new(),
    }
}

/// A blocked proposal in words: the card's label and why its boxes wait.
fn blocked(code: &str, detail: &str) -> (&'static str, String) {
    match code {
        "GATE.PERIOD_LOCK" => (
            "Preview",
            "the period is still open; close its months first".into(),
        ),
        "GATE.RETURNS_ACK" => (
            "Needs your OK",
            "waits for your OK of the return below".into(),
        ),
        "no_payroll_lines" => (
            "No pay run",
            "no pay run is posted for this month; Books does not report zero pay".into(),
        ),
        c if c.starts_with("form_context") => (
            "Needs the BIR option",
            "waits for the tax option from the BIR draft".into(),
        ),
        _ => ("Needs a look", detail.to_string()),
    }
}

/// The value text a box shows: `"1"` on a choice box reads Yes.
fn shown(key: &str, value: &str) -> String {
    if key.ends_with("TaxWithheld") {
        return if value == "1" {
            "Yes".into()
        } else {
            "No".into()
        };
    }
    money::parse(value).map_or_else(|_| value.to_string(), money::display)
}

/// One form's Books totals beside its BIR boxes.
pub fn form_view(
    store: &Store,
    book: &Book,
    form: FormCode,
    period: &str,
    ctx: &Value,
    origin: Origin,
) -> Result<Value, Refusal> {
    let data = rules::shared().map_err(|e| Refusal::new("engine", e.to_string()))?;
    let built = build(store, book, form, period, ctx)?;
    let p = &built.proposal;
    let context = form_context_from_value(ctx).map_err(|e| Refusal::invalid(e.to_string()))?;
    let pp = Period::parse(period).ok_or_else(|| Refusal::invalid("No such period."))?;
    let certs: Vec<certificates::Certificate> = certificates::list(store.conn(), book.id)?
        .into_iter()
        .filter(|c| !c.voided && c.period_end.is_some_and(|d| pp.contains(d)))
        .collect();
    let mut rows = Vec::new();
    for m in &data.rules(form).mappings {
        // a box for another tax option is not this form's row; a box for
        // another BIR choice (OSD, not itemized) stays, not sent
        let other =
            |c: &&rules::FormContextCondition| context.get(&c.read).is_some_and(|v| *v != c.equals);
        if m.form_context
            .iter()
            .filter(other)
            .any(|c| c.read == "/taxpayer/tax_option")
        {
            continue;
        }
        let not_chosen = m.form_context.iter().find(other).map(|c| {
            let said = context.get(&c.read).map_or("", String::as_str);
            let said = if said == "osd" { "OSD" } else { said };
            format!(
                "not sent: {} is {said}",
                c.read.rsplit('/').next().unwrap_or("the choice")
            )
        });
        let value = p
            .fill_args
            .get(&m.target_key)
            .or_else(|| p.reference_values.get(&m.target_key));
        let held = built
            .held
            .iter()
            .find(|(k, _)| *k == m.target_key)
            .map(|(_, why)| why.clone());
        let unchosen = not_chosen.is_some();
        let excluded = not_chosen.or_else(|| {
            p.excluded_keys
                .iter()
                .chain(&p.refused_keys)
                .find(|k| k.key == m.target_key)
                .map(
                    |k| match (k.reason.starts_with("proposal blocked"), p.blocking.first()) {
                        (true, Some(b)) => blocked(&b.code, &b.detail).1,
                        _ if k.reason.contains("open for review") => {
                            "not sent until a tax review settles it".to_string()
                        }
                        _ => k.reason.clone(),
                    },
                )
        });
        let measure = m.measures.first().map(String::as_str).unwrap_or("");
        let sub = row_sub(form, measure, &built.parts, &certs);
        let lines = p.lineage.get(&m.target_key).map_or(0, |l| l.lines.len());
        // a yes/no box reads as a question; the 13th-month box names its cap
        let (label, sub) = match m.aggregation {
            rules::Aggregation::ChoiceYesIfNonzero => (
                "Any tax withheld?",
                "Payroll recorded tax withheld".to_string(),
            ),
            rules::Aggregation::Nontaxable13thMonthCap => (
                measure_label(measure),
                match m
                    .aggregation_params
                    .as_ref()
                    .and_then(|a| a.get("annual_cap_text"))
                    .and_then(|c| money::parse(c).ok())
                {
                    Some(cap) => format!(
                        "Non-taxable part, {} yearly cap per employee (the rule file's)",
                        money::display(cap)
                    ),
                    None => sub,
                },
            ),
            _ => (measure_label(measure), sub),
        };
        let mut row = json!({
            "mapping": m.id,
            "measure": label,
            "sub": if m.mode == "reference" { "Shown for BIR to check; not typed in".to_string() } else { sub },
            "item": m.printed_item,
            "key": m.target_key,
            "mode": m.mode,
            "value": value,
            "value_shown": value.map(|v| shown(&m.target_key, v)),
            "preview_shown": built.preview.get(&m.target_key).map(|c| money::display(*c)),
            "held": held.is_some() || (value.is_none() && excluded.is_some() && !built.preview.contains_key(&m.target_key)),
            // a box BIR's choice leaves out says so before any hold
            "why": if unchosen { excluded } else { held.or(excluded) },
            "review_open": m.review_status == "open",
            "flag": if m.input_level_tax_rule.is_some() {
                Some("The one tax rule in the bridge".to_string())
            } else if m.review_status == "open" {
                Some("Review open: a mapping rule waiting for tax review".to_string())
            } else {
                None
            },
            "lines": lines,
        });
        if !origin.is_app() {
            row.as_object_mut().expect("an object").remove("why");
        }
        rows.push(row);
    }
    // the return events waiting for the owner's OK (GATE.RETURNS_ACK)
    let mut returns = Vec::new();
    for item in p.blocking.iter().filter(|i| i.code == "GATE.RETURNS_ACK") {
        for id in item
            .ids
            .iter()
            .filter_map(|i| i.strip_prefix('e'))
            .filter_map(|n| n.split('.').next()?.parse::<i64>().ok())
        {
            if let Some(e) = entries::get(store.conn(), id)? {
                let mut r = json!({ "ticket": crate::service::ticket(store, "entry", e.id)?, "date": e.date.short() });
                if origin.is_app() {
                    r["what"] = json!(
                        e.payee
                            .clone()
                            .or(e.memo.clone())
                            .unwrap_or_else(|| format!("Entry {:04}", e.no))
                    );
                }
                returns.push(r);
            }
        }
    }
    let (code, title) = form_title(form);
    let blocking: Vec<Value> = p
        .blocking
        .iter()
        .map(|i| json!({ "code": i.code, "detail": i.detail }))
        .collect();
    let warnings: Vec<Value> = p
        .warnings
        .iter()
        .map(|i| json!({ "code": i.code, "detail": i.detail }))
        .collect();
    // boxes this form leaves out: Books holds, and boxes that fail closed
    let held_rows = rows.iter().filter(|r| r["held"] == true).count();
    let status = if held_rows > 0 && p.status == "ready" {
        "ready_held"
    } else {
        p.status.as_str()
    };
    Ok(json!({
        "form": form.as_str(), "code": code, "title": title,
        "period": period, "period_label": period_label(form, period),
        "status": status,
        "status_label": match (status, p.blocking.first()) {
            ("ready", _) => "Ready".to_string(),
            ("ready_held", _) => format!("Ready · {held_rows} held"),
            (_, Some(b)) => blocked(&b.code, &b.detail).0.to_string(),
            _ => "Needs a look".to_string(),
        },
        "form_context": ctx,
        "rows": rows,
        "blocking": blocking,
        "warnings": warnings,
        "returns": returns,
        "owners": owners(form),
        "basis": p.accounting_basis,
        "fill_args": if p.status == "ready" { json!(p.fill_args) } else { json!({}) },
        "bir_note": match form {
            FormCode::F2551Q => "BIR picks the ATC, applies the rate, computes the tax due and keeps the due date.",
            FormCode::F1601C => "BIR adds up non-taxable and taxable pay, applies last month's adjustments, penalties and the due date.",
            FormCode::F1701MS => "BIR keeps the yearly 8% or graduated choice, OSD or itemized, computes taxable income, the tax and the payable.",
        },
    }))
}

/// `books_form_proposal`: one form's proposal. The assistant passes the
/// BIR choices it read as `form_context` (owner decision); Books never
/// sets them.
pub fn proposal(
    store: &Store,
    book: Book,
    args: &Map<String, Value>,
    origin: Origin,
) -> Result<Value, Refusal> {
    let form = form_arg(args)?;
    let period = text(args, "period").map_or_else(|| default_period(form), str::to_string);
    let ctx = args
        .get("form_context")
        .cloned()
        .unwrap_or_else(|| json!({}));
    form_view(store, &book, form, &period, &ctx, origin)
}

/// The three form cards of the Forms page.
pub fn overview(store: &Store, book: Book, args: &Map<String, Value>) -> Result<Value, Refusal> {
    let ctx = args
        .get("form_context")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let mut cards = Vec::new();
    for form in [FormCode::F2551Q, FormCode::F1601C, FormCode::F1701MS] {
        let period = default_period(form);
        let ctx = if form == FormCode::F1701MS {
            ctx.clone()
        } else {
            json!({})
        };
        let v = form_view(store, &book, form, &period, &ctx, Origin::App)?;
        cards.push(json!({ "form": v["form"], "code": v["code"], "title": v["title"], "period": period, "period_label": v["period_label"], "status": v["status"], "status_label": v["status_label"] }));
    }
    Ok(json!({ "cards": cards }))
}

/// `books_app_form_lineage`: the ledger lines behind one box (app only, D11).
pub fn lineage(store: &Store, book: Book, args: &Map<String, Value>) -> Result<Value, Refusal> {
    let form = form_arg(args)?;
    let period = text(args, "period").map_or_else(|| default_period(form), str::to_string);
    let ctx = args
        .get("form_context")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let key = crate::service::required(args, "key")?;
    let built = build(store, &book, form, &period, &ctx)?;
    let lin = built
        .proposal
        .lineage
        .get(key)
        .ok_or_else(|| Refusal::new("not_found", "That box has no lineage."))?;
    let mut rows = Vec::new();
    let mut total = 0i64;
    for l in &lin.lines {
        let amount = money::parse(&l.amount).unwrap_or(0);
        total += amount;
        let (date, what) = if let Some(id) = l
            .entry_id
            .strip_prefix('e')
            .and_then(|n| n.parse::<i64>().ok())
        {
            match entries::get(store.conn(), id)? {
                Some(e) => (
                    e.date.short(),
                    format!(
                        "{}{}",
                        e.payee
                            .clone()
                            .map(|p| format!("{p} · "))
                            .unwrap_or_default(),
                        e.reference
                            .clone()
                            .or(e.memo.clone())
                            .unwrap_or_else(|| format!("Entry {:04}", e.no))
                    ),
                ),
                None => (String::new(), l.entry_id.clone()),
            }
        } else {
            ("Opening".into(), "Opening figure (from its proof)".into())
        };
        rows.push(
            json!({ "date": date, "what": what, "amount_shown": crate::views::signed(amount) }),
        );
    }
    Ok(
        json!({ "key": key, "measures": lin.measures.iter().map(|m| measure_label(m)).collect::<Vec<_>>(), "rows": rows, "total_shown": money::display(total), "note": lin.note }),
    )
}
