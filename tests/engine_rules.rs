//! The measures engine and the Forms bridge (`books_core`), held to the
//! rules the byte-for-byte vectors in `fixtures/vectors/` do not show on
//! their own: refusals (ledger, reversal, accounting basis, normalization),
//! the crosswalk gate, the read-only Forms context, profile isolation,
//! idempotency, staleness, payroll and certificate rules, negative
//! measures, and the discipline of the compiled-in data (rule sets,
//! crosswalks, measures catalogue). Every figure is synthetic and read from
//! `fixtures/synthetic/` or built here.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use books_core::bridge::Item;
use books_core::canonical::{canonical, pretty};
use books_core::date::Date;
use books_core::engine_error::EngineError;
use books_core::input::{
    FormCode, FormContext, NormEntry, NormInput, NormLine, form_context_from_value,
    normalize_entry, normalize_fixture,
};
use books_core::measures::{IMPLEMENTED_MEASURES, Window, compute_measure};
use books_core::money;
use books_core::posting::post_ledger;
use books_core::proposal::{
    Freshness, Proposal, assert_fill_key, assert_fillable, build_proposal, freshness,
};
use books_core::rules::{LoadedData, shared};
use serde_json::{Value, json};

const PROFILE_A: &str = "6b1d2f3a-4c5e-4f60-8a71-92b3c4d5e6f7";
const PROFILE_B: &str = "c0a8e3f1-2b7d-4e9a-9f10-5d6c7b8a9e0f";
const FILL_POLICIES: [&str; 3] = ["fill_mvp", "fill_p6", "fill_p7"];

// ---- shared helpers ------------------------------------------------------

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_json(path: &Path) -> Value {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn data() -> &'static LoadedData {
    shared().unwrap()
}

fn synthetic_names() -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(repo().join("fixtures/synthetic"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.ends_with(".json"))
        .map(|n| n.trim_end_matches(".json").to_string())
        .collect();
    names.sort();
    names
}

fn raw_fixture(name: &str) -> Value {
    read_json(&repo().join(format!("fixtures/synthetic/{name}.json")))
}

fn norm(raw: &Value) -> Result<NormInput, EngineError> {
    normalize_fixture(raw, &data().vocab)
}

fn fixture(name: &str) -> NormInput {
    norm(&raw_fixture(name)).unwrap()
}

fn ctx(pairs: &[(&str, &str)]) -> FormContext {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn graduated_osd() -> FormContext {
    ctx(&[
        ("/taxpayer/tax_option", "graduated"),
        ("/taxpayer/deduction", "osd"),
    ])
}

fn graduated_itemized() -> FormContext {
    ctx(&[
        ("/taxpayer/tax_option", "graduated"),
        ("/taxpayer/deduction", "itemized"),
    ])
}

fn eight_percent() -> FormContext {
    ctx(&[("/taxpayer/tax_option", "eight_percent")])
}

/// None for 2551Q and 1601-C, graduated + OSD for 1701-MS.
fn default_ctx(form: FormCode) -> FormContext {
    if form == FormCode::F1701MS {
        graduated_osd()
    } else {
        FormContext::new()
    }
}

fn proposal_ctx(input: &NormInput, form: FormCode, period: &str, c: &FormContext) -> Proposal {
    build_proposal(data(), input, form, period, c, None).unwrap()
}

fn proposal(input: &NormInput, form: FormCode, period: &str) -> Proposal {
    proposal_ctx(input, form, period, &default_ctx(form))
}

fn codes(items: &[Item]) -> Vec<&str> {
    items.iter().map(|i| i.code.as_str()).collect()
}

fn item<'a>(items: &'a [Item], code: &str) -> &'a Item {
    items
        .iter()
        .find(|i| i.code == code)
        .unwrap_or_else(|| panic!("no {code} in {:?}", codes(items)))
}

fn fill<'a>(p: &'a Proposal, key: &str) -> &'a str {
    p.fill_args
        .get(key)
        .unwrap_or_else(|| panic!("{key} not in fill_args {:?}", p.fill_args))
}

fn measure<'a>(p: &'a Proposal, id: &str) -> &'a str {
    &p.measures
        .get(id)
        .unwrap_or_else(|| panic!("measure {id} missing"))
        .value
}

fn sum_text(values: &[&str]) -> String {
    money::format(values.iter().map(|v| money::parse(v).unwrap()).sum())
}

fn refusal<T: std::fmt::Debug>(r: Result<T, EngineError>) -> EngineError {
    match r {
        Err(e) => e,
        Ok(v) => panic!("expected a refusal, got {v:?}"),
    }
}

fn refusal_text<T: std::fmt::Debug>(r: Result<T, EngineError>) -> String {
    refusal(r).to_string()
}

fn line_mut<'a>(input: &'a mut NormInput, entry: &str, line: &str) -> &'a mut NormLine {
    input
        .entries
        .iter_mut()
        .find(|e| e.entry_id == entry)
        .unwrap_or_else(|| panic!("no entry {entry}"))
        .lines
        .iter_mut()
        .find(|l| l.line_id == line)
        .unwrap_or_else(|| panic!("no line {line}"))
}

/// The proposal cases, as the vector files name them: id, fixture, form,
/// period and Forms context.
struct Case {
    id: String,
    fixture: String,
    form: FormCode,
    period: String,
    form_context: FormContext,
}

fn cases() -> Vec<Case> {
    let dir = repo().join("fixtures/vectors");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.ends_with(".json") && n != "money-and-mapping.json")
        .collect();
    names.sort();
    assert_eq!(names.len(), 16);
    names
        .iter()
        .map(|n| {
            let v = read_json(&dir.join(n));
            Case {
                id: n.trim_end_matches(".json").to_string(),
                fixture: v["fixture"].as_str().unwrap().to_string(),
                form: FormCode::parse(v["form"].as_str().unwrap()).unwrap(),
                period: v["period"].as_str().unwrap().to_string(),
                form_context: form_context_from_value(&v["form_context"]).unwrap(),
            }
        })
        .collect()
}

fn all_proposals() -> Vec<(String, FormCode, Proposal)> {
    cases()
        .into_iter()
        .map(|c| {
            let p = proposal_ctx(&fixture(&c.fixture), c.form, &c.period, &c.form_context);
            (c.id, c.form, p)
        })
        .collect()
}

/// The raw rules file of a form (for fields the engine does not keep, such
/// as `not_mapped` and `test_vectors`).
fn raw_rules(form: FormCode) -> Value {
    let name = match form {
        FormCode::F2551Q => "2551q-v2018",
        FormCode::F1601C => "1601c-v2018",
        FormCode::F1701MS => "1701ms-v2024",
    };
    read_json(&repo().join(format!("data/rules/{name}.rules.json")))
}

/// The raw crosswalk file of a form (for `draft_paths`, `key_pattern` and
/// `typed_alias_status`).
fn raw_crosswalk(form: FormCode) -> Value {
    let name = match form {
        FormCode::F2551Q => "2551q-v2018",
        FormCode::F1601C => "1601c-v2018",
        FormCode::F1701MS => "1701ms-v2024",
    };
    read_json(&repo().join(format!("data/crosswalk/{name}.json")))
}

fn texts(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

// ---- in-memory inputs for scenarios not worth a fixture file -------------

/// A raw input with one profile (PROFILE_A), no basis stated (so cash), the
/// usual closed periods and no acknowledgements.
fn memory(entries: Vec<Value>) -> Value {
    json!({
        "fixture_format": "buwiz-poc-fixture/3",
        "fixture_id": "in-memory",
        "as_of": "2026-10-29",
        "profiles": [{
            "tax_profile_id": PROFILE_A,
            "display": "***-***-***-00000",
            "periods": {"2026-Q3": "closed", "2026-09": "closed", "2026-12": "closed", "2026": "closed"},
        }],
        "acknowledged_entry_ids": [],
        "entries": entries,
    })
}

fn accrual(mut raw: Value) -> Value {
    raw["profiles"][0]["accounting_basis"] = json!("accrual");
    raw
}

fn acked(mut raw: Value, ids: &[&str]) -> Value {
    raw["acknowledged_entry_ids"] = json!(ids);
    raw
}

fn with_periods(mut raw: Value, periods: Value) -> Value {
    raw["profiles"][0]["periods"] = periods;
    raw
}

/// A sale paid on the day: bank debit, sales credit.
fn sale_entry(id: &str, date: &str, amount: &str) -> Value {
    json!({
        "entry_id": id, "tax_profile_id": PROFILE_A, "entry_date": date,
        "memo": format!("in-memory sale {id}"),
        "lines": [
            {"line_id": format!("{id}.1"), "account": "bank_accounts", "debit": amount, "credit": "0.00"},
            {"line_id": format!("{id}.2"), "account": "sales_revenue", "debit": "0.00", "credit": amount,
             "measure_side": "credit", "income_class": "business_sales", "document_kind": "sale_invoice"},
        ],
    })
}

/// A contra-sales debit (return, allowance or discount) refunded from the bank.
fn credit_note_entry(id: &str, date: &str, amount: &str, contra_kind: &str) -> Value {
    json!({
        "entry_id": id, "tax_profile_id": PROFILE_A, "entry_date": date,
        "memo": format!("in-memory credit note {id}"),
        "lines": [
            {"line_id": format!("{id}.1"), "account": "refunds_and_discounts", "debit": amount, "credit": "0.00",
             "measure_side": "credit", "income_class": "contra_sales", "contra_kind": contra_kind,
             "document_kind": "credit_note"},
            {"line_id": format!("{id}.2"), "account": "bank_accounts", "debit": "0.00", "credit": amount},
        ],
    })
}

/// A certificate posted against the customer's receivable (accrual books).
fn cert_entry(
    pid: &str,
    id: &str,
    date: &str,
    amount: &str,
    credit_kind: &str,
    status: &str,
) -> Value {
    let account = if credit_kind == "pt" {
        "pct_tax_credit_receivable"
    } else {
        "ph_cwt_receivable"
    };
    json!({
        "entry_id": id, "tax_profile_id": pid, "entry_date": date,
        "memo": format!("in-memory certificate {id}"),
        "lines": [
            {"line_id": format!("{id}.1"), "account": account, "debit": amount, "credit": "0.00",
             "measure_side": "debit", "certificate_atc": "WB082", "credit_kind": credit_kind,
             "certificate_status": status, "period_start": "2026-07-01", "period_end": "2026-09-30",
             "document_kind": "certificate_2307"},
            {"line_id": format!("{id}.2"), "account": "accounts_receivable_gross", "debit": "0.00", "credit": amount},
        ],
    })
}

/// One employee's 13th-month payment, balanced against net pay.
fn thirteenth_entry(id: &str, date: &str, amount: &str, employee: &str) -> Value {
    json!({
        "entry_id": id, "tax_profile_id": PROFILE_A, "entry_date": date,
        "lines": [
            {"line_id": format!("{id}.1"), "account": "employee_benefits", "debit": amount, "credit": "0.00",
             "measure_side": "debit", "comp_component": "thirteenth_month_and_benefits",
             "person_scope": "employee", "party_ref": employee, "document_kind": "payroll_register"},
            {"line_id": format!("{id}.2"), "account": "ph_net_pay_payable", "debit": "0.00", "credit": amount},
        ],
    })
}

/// Reverses an entry: mirrored sides, same attributes, dated as given.
fn reversal_entry(id: &str, original: &Value, date: &str) -> Value {
    let lines: Vec<Value> = original["lines"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let mut l = line.clone();
            l["line_id"] = json!(format!("{id}.{}", i + 1));
            l["debit"] = line["credit"].clone();
            l["credit"] = line["debit"].clone();
            l
        })
        .collect();
    json!({
        "entry_id": id, "tax_profile_id": original["tax_profile_id"], "entry_date": date,
        "memo": format!("in-memory void of {}", original["entry_id"].as_str().unwrap()),
        "reverses_entry_id": original["entry_id"], "lines": lines,
    })
}

fn normalized(pid: &str, raw: &Value) -> NormEntry {
    let ids = std::iter::once(pid.to_string()).collect();
    normalize_entry(raw, &ids, &data().vocab).unwrap()
}

fn two_line(id: &str, date: &str, first: Value, second: Value) -> Value {
    json!({"entry_id": id, "tax_profile_id": PROFILE_A, "entry_date": date, "lines": [first, second]})
}

fn sales_line(line_id: &str, amount: &str, document_kind: &str) -> Value {
    json!({"line_id": line_id, "account": "sales_revenue", "debit": "0.00", "credit": amount,
        "measure_side": "credit", "income_class": "business_sales", "document_kind": document_kind})
}

/// Invoice on the books: receivable debit, sales credit (accrual only).
fn invoiced(id: &str, date: &str, amount: &str) -> Value {
    two_line(
        id,
        date,
        json!({"line_id": format!("{id}.1"), "account": "accounts_receivable_gross", "debit": amount, "credit": "0.00"}),
        sales_line(&format!("{id}.2"), amount, "sale_invoice"),
    )
}

/// Collection of that invoice: bank debit, receivable credit (accrual only).
fn collected(id: &str, date: &str, amount: &str) -> Value {
    two_line(
        id,
        date,
        json!({"line_id": format!("{id}.1"), "account": "bank_accounts", "debit": amount, "credit": "0.00"}),
        json!({"line_id": format!("{id}.2"), "account": "accounts_receivable_gross", "debit": "0.00", "credit": amount}),
    )
}

/// Sale posted when the money arrives: bank debit, sales credit.
fn received(id: &str, date: &str, amount: &str, document_kind: &str) -> Value {
    two_line(
        id,
        date,
        json!({"line_id": format!("{id}.1"), "account": "bank_accounts", "debit": amount, "credit": "0.00"}),
        sales_line(&format!("{id}.2"), amount, document_kind),
    )
}

/// Supplier bill on the books: expense debit, payable credit (accrual only).
fn billed(id: &str, date: &str, amount: &str) -> Value {
    two_line(
        id,
        date,
        json!({"line_id": format!("{id}.1"), "account": "rent", "debit": amount, "credit": "0.00",
            "measure_side": "debit", "income_class": "operating_expense", "document_kind": "expense_document"}),
        json!({"line_id": format!("{id}.2"), "account": "accounts_payable", "debit": "0.00", "credit": amount}),
    )
}

fn window(start: &str, end: &str) -> Window {
    Window {
        start: start.into(),
        end: end.into(),
    }
}

// ---- the accounting basis (ADR 0008) ---------------------------------------

mod accounting_basis {
    use super::*;

    #[test]
    fn a_profile_that_does_not_state_a_basis_is_a_cash_basis_book() {
        let mut raw = raw_fixture("f-slice-a-2026");
        raw["profiles"][0]
            .as_object_mut()
            .unwrap()
            .remove("accounting_basis");
        assert_eq!(norm(&raw).unwrap().profiles[0].accounting_basis, "cash");
    }

    #[test]
    fn only_cash_or_accrual_is_accepted() {
        let mut raw = raw_fixture("f-slice-a-2026");
        raw["profiles"][0]["accounting_basis"] = json!("modified_cash");
        assert!(matches!(refusal(norm(&raw)), EngineError::Input(_)));
    }

    #[test]
    fn every_synthetic_fixture_states_its_basis() {
        for name in synthetic_names() {
            for profile in raw_fixture(&name)["profiles"].as_array().unwrap() {
                let basis = profile["accounting_basis"].as_str();
                assert!(matches!(basis, Some("cash" | "accrual")), "{name}");
            }
        }
    }

    #[test]
    fn a_cash_book_refuses_a_trade_receivable_line() {
        let raw = memory(vec![invoiced("I-1", "2026-09-25", "50000.00")]);
        assert!(refusal_text(norm(&raw)).contains("accrual account"));
    }

    #[test]
    fn a_cash_book_refuses_a_trade_payable_line() {
        let raw = memory(vec![billed("B-1", "2026-09-25", "8000.00")]);
        assert!(refusal_text(norm(&raw)).contains("accrual account"));
    }

    #[test]
    fn a_cash_book_refuses_a_sales_line_tagged_payment_receipt() {
        let raw = memory(vec![received(
            "R-1",
            "2026-10-05",
            "50000.00",
            "payment_receipt",
        )]);
        assert!(refusal_text(norm(&raw)).contains("sale_invoice"));
    }

    #[test]
    fn an_accrual_book_accepts_receivable_and_payable_lines() {
        let raw = accrual(memory(vec![
            invoiced("I-1", "2026-09-25", "50000.00"),
            billed("B-1", "2026-09-25", "8000.00"),
        ]));
        assert_eq!(norm(&raw).unwrap().profiles[0].accounting_basis, "accrual");
    }

    #[test]
    fn invoiced_in_q3_and_paid_in_q4_counts_in_q3_on_accrual_and_q4_on_cash() {
        let acc = norm(&accrual(memory(vec![
            invoiced("I-1", "2026-09-25", "50000.00"),
            collected("I-2", "2026-10-05", "50000.00"),
        ])))
        .unwrap();
        let cash = norm(&memory(vec![received(
            "R-1",
            "2026-10-05",
            "50000.00",
            "sale_invoice",
        )]))
        .unwrap();
        let acc = post_ledger(&acc.entries).unwrap();
        let cash = post_ledger(&cash.entries).unwrap();
        let q3 = window("2026-07-01", "2026-09-30");
        let q4 = window("2026-10-01", "2026-12-31");
        let gross = |l, w| compute_measure("gross_sales", l, w).unwrap();
        assert_eq!(gross(&acc, &q3).value, 5_000_000);
        assert_eq!(gross(&acc, &q4).value, 0);
        assert_eq!(gross(&cash, &q3).value, 0);
        assert_eq!(gross(&cash, &q4).value, 5_000_000);
        let ids: Vec<String> = gross(&cash, &q4)
            .lines
            .iter()
            .map(|r| r.line.line.line_id.clone())
            .collect();
        assert_eq!(ids, ["R-1.2"]);
    }

    #[test]
    fn changing_the_books_basis_makes_a_stored_proposal_stale() {
        let input = fixture("f-slice-a-2026");
        let stored = proposal(&input, FormCode::F2551Q, "2026-Q3");
        assert_eq!(
            freshness(&stored, data(), &input, None, None).unwrap(),
            Freshness::Fresh
        );
        let mut switched = input.clone();
        switched.profiles[0].accounting_basis = "accrual".into();
        assert_eq!(
            freshness(&stored, data(), &switched, None, None).unwrap(),
            Freshness::Stale
        );
    }
}

// ---- the crosswalk gate and the data discipline (BRIDGE.INPUT_ONLY) --------

mod crosswalk {
    use super::*;

    #[test]
    fn every_fill_key_in_every_case_is_an_input_key_with_a_fill_policy() {
        for (id, form, p) in all_proposals() {
            for key in p.fill_args.keys() {
                let field = data()
                    .crosswalk(form)
                    .by_fill_key(key)
                    .unwrap_or_else(|| panic!("{id}: {key}"));
                assert_eq!(field.fill_class, "input", "{id}: {key}");
                assert!(
                    FILL_POLICIES.contains(&field.books_policy.as_str()),
                    "{id}: {key}"
                );
            }
        }
    }

    #[test]
    fn no_computed_locked_or_forms_owned_key_is_ever_proposed_or_referenced() {
        for (id, form, p) in all_proposals() {
            let cw = data().crosswalk(form);
            for key in p.fill_args.keys().chain(p.reference_values.keys()) {
                let field = cw
                    .by_fill_key(key)
                    .or_else(|| cw.by_key(key))
                    .unwrap_or_else(|| panic!("{id}: {key}"));
                assert_eq!(field.fill_class, "input", "{id}: {key}");
                assert_ne!(field.books_policy, "forms_computes", "{id}: {key}");
                assert_ne!(field.books_source, "forms", "{id}: {key}");
            }
        }
    }

    #[test]
    fn every_mapping_targets_an_input_key_and_fill_mappings_a_fill_policy_key() {
        for form in FormCode::ALL {
            let cw = data().crosswalk(form);
            for m in &data().rules(form).mappings {
                let field = cw
                    .by_fill_key(&m.target_key)
                    .or_else(|| cw.by_key(&m.target_key))
                    .unwrap_or_else(|| panic!("{}", m.id));
                assert_eq!(field.fill_class, "input", "{}", m.id);
                if m.mode == "fill" {
                    assert!(
                        FILL_POLICIES.contains(&field.books_policy.as_str()),
                        "{}",
                        m.id
                    );
                } else {
                    assert_eq!(field.books_policy, "never_fill", "{}", m.id);
                }
                assert!(field.rule_ids.contains(&m.id), "{}", m.id);
            }
        }
    }

    #[test]
    fn every_fill_policy_key_has_one_fill_mapping_and_every_rule_id_resolves() {
        let common: BTreeSet<String> = data().common["rules"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["id"].as_str().unwrap().to_string())
            .collect();
        for form in FormCode::ALL {
            let cw = data().crosswalk(form);
            let rs = data().rules(form);
            let mut fill_targets: Vec<&str> = rs
                .mappings
                .iter()
                .filter(|m| m.mode == "fill")
                .map(|m| m.target_key.as_str())
                .collect();
            fill_targets.sort_unstable();
            let mut sendable: Vec<&str> = cw
                .fields
                .iter()
                .filter(|f| FILL_POLICIES.contains(&f.books_policy.as_str()))
                .filter_map(|f| f.fill_key.as_deref())
                .collect();
            sendable.sort_unstable();
            assert_eq!(fill_targets, sendable, "{}", form.as_str());
            let mut ids: BTreeSet<String> = rs.mappings.iter().map(|m| m.id.clone()).collect();
            for n in raw_rules(form)["not_mapped"].as_array().unwrap() {
                ids.insert(n["id"].as_str().unwrap().to_string());
            }
            ids.extend(common.iter().cloned());
            for field in &cw.fields {
                for id in &field.rule_ids {
                    assert!(ids.contains(id), "{}: {id}", form.as_str());
                }
            }
        }
    }

    #[test]
    fn forms_owned_boxes_have_no_books_rule_only_a_pointer_to_forms() {
        for form in FormCode::ALL {
            let mapped: BTreeSet<&str> = data()
                .rules(form)
                .mappings
                .iter()
                .map(|m| m.id.as_str())
                .collect();
            let raw = raw_rules(form);
            let owner: BTreeMap<&str, &str> = raw["not_mapped"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| (n["id"].as_str().unwrap(), n["owner"].as_str().unwrap_or("")))
                .collect();
            for field in data()
                .crosswalk(form)
                .fields
                .iter()
                .filter(|f| f.books_policy == "forms_computes")
            {
                for id in &field.rule_ids {
                    assert!(!mapped.contains(id.as_str()), "{}: {id}", form.as_str());
                    assert_eq!(
                        owner.get(id.as_str()),
                        Some(&"forms"),
                        "{}: {id}",
                        form.as_str()
                    );
                }
            }
        }
    }

    #[test]
    fn forged_fills_of_a_computed_election_or_atc_key_are_refused() {
        let cw = data().crosswalk(FormCode::F2551Q);
        assert!(matches!(
            refusal(assert_fill_key(
                cw,
                "frm2551Qv2018:txt14",
                "8700.00",
                "2551Q.II.14"
            )),
            EngineError::Crosswalk(_)
        ));
        assert!(matches!(
            refusal(assert_fill_key(
                cw,
                "frm2551Qv2018:taxRate",
                "3",
                "2551Q.I.13"
            )),
            EngineError::Crosswalk(_)
        ));
        assert!(
            refusal_text(assert_fill_key(
                cw,
                "schedule_1.0.atc",
                "PT010",
                "2551Q.S1.1.ATC"
            ))
            .contains("not sendable")
        );
        assert!(
            refusal_text(assert_fill_key(
                data().crosswalk(FormCode::F1601C),
                "frm1601c:txtATC",
                "WW010",
                "1601C.H.5"
            ))
            .contains("not sendable")
        );
    }

    #[test]
    fn a_fill_with_the_wrong_mapping_or_a_value_of_the_wrong_type_is_refused() {
        let cw = data().crosswalk(FormCode::F2551Q);
        assert!(
            refusal_text(assert_fill_key(
                cw,
                "txtATCAmt1",
                "290000.00",
                "2551Q.II.15"
            ))
            .contains("governed by")
        );
        assert!(
            refusal_text(assert_fill_key(
                cw,
                "txtATCAmt1",
                "290000",
                "2551Q.S1.1.base"
            ))
            .contains("two-place decimal")
        );
        let field = assert_fill_key(cw, "txtATCAmt1", "290000.00", "2551Q.S1.1.base").unwrap();
        assert_eq!(field.fill_class, "input");
        assert!(
            refusal_text(assert_fill_key(
                data().crosswalk(FormCode::F1601C),
                "frm1601c:TaxWithheld",
                "yes",
                "1601C.H.3"
            ))
            .contains("printed option")
        );
    }

    /// `/No\d+b$|Spouse|NoB/i`: a spouse (B-side) key.
    fn spouse_like(key: &str) -> bool {
        let l = key.to_ascii_lowercase();
        if l.contains("spouse") || l.contains("nob") {
            return true;
        }
        let Some(rest) = l.strip_suffix('b') else {
            return false;
        };
        let head = rest.trim_end_matches(|c: char| c.is_ascii_digit());
        head.len() < rest.len() && head.ends_with("no")
    }

    #[test]
    fn election_inputs_and_spouse_keys_are_never_filled_and_appear_only_as_refused() {
        for (id, _, p) in all_proposals() {
            for key in [
                "frm1701MS:txtTaxpayerNo12b",
                "frm1701MS:txtTaxpayerNo12",
                "frm1701MS:txtTaxpayerNo17",
                "taxCodeDropdown",
                "frm2551Qv2018:taxRate",
            ] {
                assert!(!p.fill_args.contains_key(key), "{id}: {key}");
            }
            for key in p.fill_args.keys() {
                assert!(!spouse_like(key), "{id}: {key}");
            }
        }
        let refused =
            proposal(&fixture("a-sole-prop-2026"), FormCode::F1701MS, "2026").refused_keys;
        assert!(refused.iter().any(|r| r.reason.contains("spouse")));
        assert!(
            refused
                .iter()
                .any(|r| r.key == "frm1701MS:txtTaxpayerNo17" && r.reason.contains("form_context"))
        );
    }

    #[test]
    fn every_forms_context_condition_reads_a_declared_input_whose_value_is_a_draft_path_option() {
        for form in FormCode::ALL {
            let rs = data().rules(form);
            let declared: BTreeMap<&str, &Vec<String>> = rs
                .form_context_inputs
                .iter()
                .map(|i| (i.path.as_str(), &i.supported_values))
                .collect();
            let draft_paths: BTreeSet<String> = raw_crosswalk(form)["fields"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|f| texts(&f["draft_paths"]))
                .collect();
            for input in &rs.form_context_inputs {
                assert!(draft_paths.contains(&input.path), "{}", input.path);
            }
            for m in &rs.mappings {
                for c in &m.form_context {
                    let values = declared
                        .get(c.read.as_str())
                        .unwrap_or_else(|| panic!("{}: {}", m.id, c.read));
                    assert!(values.contains(&c.equals), "{}: {}", m.id, c.read);
                }
            }
        }
    }

    #[test]
    fn every_measure_a_mapping_reads_is_in_the_catalogue_and_implemented_in_the_engine() {
        for form in FormCode::ALL {
            for m in &data().rules(form).mappings {
                for id in &m.measures {
                    assert!(data().measure_status(id).is_some(), "{}: {id}", m.id);
                    assert!(
                        IMPLEMENTED_MEASURES.contains(&id.as_str()),
                        "{}: {id}",
                        m.id
                    );
                }
            }
        }
    }

    #[test]
    fn the_catalogue_and_the_engine_agree_on_what_is_implemented() {
        let mut in_catalogue: Vec<&str> = data().measures["measures"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|m| m["poc"] == "implemented")
            .map(|m| m["id"].as_str().unwrap())
            .collect();
        in_catalogue.sort_unstable();
        let mut engine = IMPLEMENTED_MEASURES.to_vec();
        engine.sort_unstable();
        assert_eq!(engine, in_catalogue);
    }

    #[test]
    fn every_measure_has_a_unit_sign_convention_period_basis_person_scope_and_lineage() {
        for m in data().measures["measures"].as_array().unwrap() {
            let id = m["id"].as_str().unwrap();
            assert_eq!(m["unit"], "centavos", "{id}");
            assert!(
                ["credit_positive", "debit_positive", "reduction_positive"]
                    .contains(&m["sign_convention"].as_str().unwrap_or("")),
                "{id}"
            );
            assert!(!texts(&m["period_basis"]).is_empty(), "{id}");
            assert!(!texts(&m["person_scope"]).is_empty(), "{id}");
            assert!(
                m["lineage"].as_str().unwrap_or("").starts_with("required"),
                "{id}"
            );
        }
    }

    #[test]
    fn the_coverage_matrix_lists_exactly_the_mappings_by_input_key() {
        let coverage = read_json(&repo().join("data/coverage-matrix.v1.json"));
        for form in FormCode::ALL {
            let entry = coverage["forms"]
                .as_array()
                .unwrap()
                .iter()
                .find(|f| f["form"] == form.as_str())
                .unwrap_or_else(|| panic!("{}", form.as_str()));
            let mut listed: Vec<&str> = entry["fed_by_measures"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r["input_key"].as_str().unwrap())
                .collect();
            listed.sort_unstable();
            let mut mapped: Vec<&str> = data()
                .rules(form)
                .mappings
                .iter()
                .map(|m| m.target_key.as_str())
                .collect();
            mapped.sort_unstable();
            assert_eq!(listed, mapped, "{}", form.as_str());
        }
    }

    #[test]
    fn each_proposal_value_cites_its_mapping_and_the_measures_it_read() {
        for (id, _, p) in all_proposals() {
            for (key, l) in &p.lineage {
                assert!(!l.mapping_id.is_empty(), "{id}: {key}");
                assert!(!l.measures.is_empty(), "{id}: {key}");
            }
        }
    }

    #[test]
    fn the_rules_file_mapping_vectors_agree_with_the_worked_example() {
        let p = proposal(&fixture("a-sole-prop-2026"), FormCode::F1601C, "2026-09");
        let raw = raw_rules(FormCode::F1601C);
        let ii14 = raw["mappings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["id"] == "1601C.II.14")
            .unwrap();
        assert_eq!(
            fill(&p, "frm1601c:txtTax14"),
            ii14["test_vectors"][0]["expected_text"].as_str().unwrap()
        );
    }

    #[test]
    fn the_1701ms_sales_boxes_write_the_draft_paths_the_forms_option_selects() {
        let raw = raw_crosswalk(FormCode::F1701MS);
        let paths = |fill_key: &str| -> Vec<String> {
            let f = raw["fields"]
                .as_array()
                .unwrap()
                .iter()
                .find(|f| f["fill_key"] == fill_key)
                .unwrap_or_else(|| panic!("{fill_key}"));
            texts(&f["draft_paths"])
        };
        assert_eq!(paths("frm1701MS:txtSalesRevenueNo5a"), ["/taxpayer/sales"]);
        assert_eq!(
            paths("frm1701MS:txtSalesRevenuesFeesANo19a"),
            ["/taxpayer/eight_sales"]
        );
    }
}

// ---- an account is not meaning ---------------------------------------------

mod account_is_not_meaning {
    use super::*;

    #[test]
    fn renaming_the_sales_account_on_every_sale_line_leaves_the_proposal_unchanged() {
        let base = fixture("a-sole-prop-2026");
        let mut renamed = base.clone();
        for entry in &mut renamed.entries {
            for line in &mut entry.lines {
                if line.account == "sales_revenue" {
                    line.account = "subscription_revenue".into();
                }
            }
        }
        let before = proposal(&base, FormCode::F2551Q, "2026-Q3");
        let after = proposal(&renamed, FormCode::F2551Q, "2026-Q3");
        assert_eq!(after.fill_args, before.fill_args);
        assert_eq!(after.blocking, before.blocking);
        assert_eq!(after.warnings, before.warnings);
        assert_eq!(
            after.lineage["txtATCAmt1"].lines,
            before.lineage["txtATCAmt1"].lines
        );
    }

    #[test]
    fn rewriting_every_category_default_leaves_the_proposal_unchanged() {
        let mut mutated = data().clone();
        for category in mutated.categories["categories"].as_array_mut().unwrap() {
            category["default_line_attributes"] =
                json!({"income_class": "none", "measure_side": null, "person_scope": "self"});
        }
        let input = fixture("a-sole-prop-2026");
        let a = build_proposal(
            &mutated,
            &input,
            FormCode::F2551Q,
            "2026-Q3",
            &FormContext::new(),
            None,
        )
        .unwrap();
        let b = proposal(&input, FormCode::F2551Q, "2026-Q3");
        assert_eq!(a.output_sha256, b.output_sha256);
    }

    #[test]
    fn category_defaults_carry_accounting_facts_only() {
        for category in data().categories["categories"].as_array().unwrap() {
            let id = category["id"].as_str().unwrap();
            assert!(category.get("default_tax_attributes").is_none(), "{id}");
            let attrs = category["default_line_attributes"].as_object().unwrap();
            for banned in ["tax_type", "atc", "treatment", "regime", "vat_status"] {
                assert!(!attrs.contains_key(banned), "{id}.{banned}");
            }
        }
    }

    #[test]
    fn every_account_key_in_the_fixtures_is_in_the_category_catalogue() {
        let ids: BTreeSet<&str> = data().categories["categories"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["id"].as_str().unwrap())
            .collect();
        for name in synthetic_names() {
            for entry in raw_fixture(&name)["entries"].as_array().unwrap() {
                for line in entry["lines"].as_array().unwrap() {
                    let account = line["account"].as_str().unwrap();
                    assert!(ids.contains(account), "{name}: {account}");
                }
            }
        }
    }

    #[test]
    fn a_quarter_key_for_the_annual_form_is_refused() {
        let e = refusal(build_proposal(
            data(),
            &fixture("a-sole-prop-2026"),
            FormCode::F1701MS,
            "2026-Q3",
            &FormContext::new(),
            Some(PROFILE_A),
        ));
        assert_eq!(e.code(), Some("period_kind"));
    }
}

// ---- profile isolation -------------------------------------------------------

mod isolation {
    use super::*;

    /// Profiles A and B in one input, as a multi-profile store hands them over.
    fn merged() -> NormInput {
        let a = fixture("a-sole-prop-2026");
        let e = fixture("e-second-profile-2026");
        NormInput {
            input_format: a.input_format.clone(),
            as_of: a.as_of.clone(),
            profiles: [a.profiles, e.profiles].concat(),
            acknowledged_entry_ids: [a.acknowledged_entry_ids, e.acknowledged_entry_ids].concat(),
            entries: [a.entries, e.entries].concat(),
        }
    }

    fn for_profile(input: &NormInput, form: FormCode, period: &str, pid: &str) -> Proposal {
        build_proposal(data(), input, form, period, &default_ctx(form), Some(pid)).unwrap()
    }

    #[test]
    fn a_proposal_for_a_is_byte_identical_whether_or_not_b_is_in_the_input() {
        let both = merged();
        let alone = fixture("a-sole-prop-2026");
        for (form, period) in [
            (FormCode::F2551Q, "2026-Q3"),
            (FormCode::F1601C, "2026-09"),
            (FormCode::F1701MS, "2026"),
        ] {
            let a = proposal(&alone, form, period);
            let with_b = for_profile(&both, form, period, PROFILE_A);
            assert_eq!(with_b.output_sha256, a.output_sha256, "{period}");
            assert_eq!(
                pretty(&with_b.to_value()).unwrap(),
                pretty(&a.to_value()).unwrap()
            );
        }
    }

    #[test]
    fn bs_2551q_is_computed_from_bs_lines_alone() {
        let with_a = for_profile(&merged(), FormCode::F2551Q, "2026-Q3", PROFILE_B);
        let alone = proposal(
            &fixture("e-second-profile-2026"),
            FormCode::F2551Q,
            "2026-Q3",
        );
        assert_eq!(with_a.output_sha256, alone.output_sha256);
        assert_eq!(fill(&with_a, "txtATCAmt1"), "999000.00");
        assert_eq!(measure(&with_a, "net_sales"), "999000.00");
    }

    #[test]
    fn as_lineage_names_only_as_lines_and_carries_no_b_identifier() {
        let p = for_profile(&merged(), FormCode::F2551Q, "2026-Q3", PROFILE_A);
        let b_lines: BTreeSet<String> = fixture("e-second-profile-2026")
            .entries
            .iter()
            .flat_map(|e| e.lines.iter().map(|l| l.line_id.clone()))
            .collect();
        for l in &p.lineage["txtATCAmt1"].lines {
            assert!(!b_lines.contains(&l.line_id), "{}", l.line_id);
        }
        let text = canonical(&p.to_value()).unwrap();
        for needle in [PROFILE_B, "E-001", "E-002"] {
            assert!(!text.contains(needle), "{needle}");
        }
    }

    #[test]
    fn a_proposal_for_a_profile_not_in_the_input_is_refused() {
        let e = refusal(build_proposal(
            data(),
            &fixture("a-sole-prop-2026"),
            FormCode::F2551Q,
            "2026-Q3",
            &FormContext::new(),
            Some(PROFILE_B),
        ));
        assert_eq!(e.code(), Some("no_profile"));
    }

    #[test]
    fn a_reversal_of_another_profiles_entry_is_an_orphan() {
        let mut mixed = merged();
        let b_entry = mixed
            .entries
            .iter()
            .find(|e| e.entry_id == "E-001")
            .unwrap()
            .clone();
        let lines = b_entry
            .lines
            .iter()
            .map(|l| {
                let mut r = l.clone();
                r.line_id = format!("X-1.{}", &l.line_id[l.line_id.len() - 1..]);
                r.debit = l.credit.clone();
                r.credit = l.debit.clone();
                r
            })
            .collect();
        mixed.entries.push(NormEntry {
            entry_id: "X-1".into(),
            tax_profile_id: PROFILE_A.into(),
            entry_date: b_entry.entry_date.clone(),
            reverses_entry_id: Some("E-001".into()),
            lines,
        });
        let e = refusal(build_proposal(
            data(),
            &mixed,
            FormCode::F2551Q,
            "2026-Q3",
            &FormContext::new(),
            Some(PROFILE_A),
        ));
        assert_eq!(e.code(), Some("orphan_reversal"));
    }
}

// ---- idempotency -------------------------------------------------------------

mod idempotency {
    use super::*;

    #[test]
    fn building_the_same_proposal_three_times_gives_the_same_canonical_json() {
        let input = fixture("a-sole-prop-2026");
        let build =
            || canonical(&proposal(&input, FormCode::F2551Q, "2026-Q3").to_value()).unwrap();
        let first = build();
        assert_eq!(build(), first);
        assert_eq!(build(), first);
    }

    #[test]
    fn the_pretty_file_form_is_stable_too() {
        let input = fixture("f-slice-a-2026");
        let build = || pretty(&proposal(&input, FormCode::F2551Q, "2026-Q3").to_value()).unwrap();
        assert_eq!(build(), build());
    }

    #[test]
    fn a_memo_change_changes_neither_the_input_nor_the_proposal() {
        let raw = raw_fixture("f-slice-a-2026");
        let mut edited = raw.clone();
        edited["entries"][0]["memo"] = json!("MEMO-SYNTH-EDITED a different free-text memo");
        let before = norm(&raw).unwrap();
        let after = norm(&edited).unwrap();
        assert_eq!(
            canonical(&after.to_value()).unwrap(),
            canonical(&before.to_value()).unwrap()
        );
        assert_eq!(
            proposal(&after, FormCode::F2551Q, "2026-Q3").output_sha256,
            proposal(&before, FormCode::F2551Q, "2026-Q3").output_sha256
        );
    }
}

// ---- staleness: a changed input marks the stored proposal stale ------------

mod staleness {
    use super::*;

    fn base() -> NormInput {
        fixture("a-sole-prop-2026")
    }

    fn stored() -> Proposal {
        proposal(&base(), FormCode::F2551Q, "2026-Q3")
    }

    fn set_sale(input: &mut NormInput, entry: &str, amount: &str) {
        line_mut(input, entry, &format!("{entry}.2")).credit = amount.into();
        line_mut(input, entry, &format!("{entry}.1")).debit = amount.into();
    }

    #[test]
    fn the_stored_proposal_is_fresh_against_the_same_inputs_and_fill_is_allowed() {
        let (base, stored) = (base(), stored());
        assert_eq!(
            freshness(&stored, data(), &base, None, None).unwrap(),
            Freshness::Fresh
        );
        assert_eq!(
            assert_fillable(&stored, data(), &base, None, None).unwrap(),
            &stored.fill_args
        );
    }

    #[test]
    fn a_changed_q3_line_makes_the_proposal_stale_and_fill_is_refused() {
        let stored = stored();
        let mut changed = base();
        set_sale(&mut changed, "A-010", "100500.00");
        assert_eq!(
            freshness(&stored, data(), &changed, None, None).unwrap(),
            Freshness::Stale
        );
        let e = refusal(assert_fillable(&stored, data(), &changed, None, None));
        assert_eq!(e.code(), Some("stale"));
    }

    #[test]
    fn a_q4_change_does_not_stale_the_q3_proposal_but_changes_the_input_hash() {
        let stored = stored();
        let mut changed = base();
        set_sale(&mut changed, "A-017", "100500.00");
        assert_eq!(
            freshness(&stored, data(), &changed, None, None).unwrap(),
            Freshness::Fresh
        );
        let after = proposal(&changed, FormCode::F2551Q, "2026-Q3");
        assert_ne!(after.fixture_sha256, stored.fixture_sha256);
        assert_ne!(after.output_sha256, stored.output_sha256);
        assert_eq!(after.fill_args, stored.fill_args);
    }

    #[test]
    fn withdrawing_an_acknowledgement_makes_the_q3_proposal_stale_and_blocked() {
        let stored = stored();
        let mut changed = base();
        changed.acknowledged_entry_ids.clear();
        assert_eq!(
            freshness(&stored, data(), &changed, None, None).unwrap(),
            Freshness::Stale
        );
        let now = proposal(&changed, FormCode::F2551Q, "2026-Q3");
        assert!(codes(&now.blocking).contains(&"GATE.RETURNS_ACK"));
    }

    #[test]
    fn a_change_in_the_forms_tax_option_makes_the_1701ms_proposal_stale() {
        let base = base();
        let annual = proposal_ctx(&base, FormCode::F1701MS, "2026", &graduated_osd());
        assert_eq!(
            freshness(&annual, data(), &base, None, None).unwrap(),
            Freshness::Fresh
        );
        let eight = eight_percent();
        assert_eq!(
            freshness(&annual, data(), &base, Some(&eight), None).unwrap(),
            Freshness::Stale
        );
        assert!(
            refusal_text(assert_fillable(&annual, data(), &base, Some(&eight), None))
                .contains("stale")
        );
    }

    #[test]
    fn a_measures_catalogue_change_makes_the_proposal_stale() {
        let stored = stored();
        let mut changed = data().clone();
        changed.measures["version"] = json!(2);
        assert_eq!(
            freshness(&stored, &changed, &base(), None, None).unwrap(),
            Freshness::Stale
        );
    }

    #[test]
    fn a_blocked_proposal_is_refused_for_fill_even_when_fresh() {
        let c = fixture("c-reversal-credit-note-2026");
        let blocked = proposal(&c, FormCode::F2551Q, "2026-Q3");
        assert_eq!(blocked.status, "blocked");
        let e = refusal(assert_fillable(&blocked, data(), &c, None, None));
        assert_eq!(e.code(), Some("not_ready"));
    }
}

// ---- the ledger: balance, sides and reversals ---------------------------------

mod ledger {
    use super::*;

    fn entry(raw: Value) -> NormEntry {
        normalized(PROFILE_A, &raw)
    }

    #[test]
    fn an_entry_with_debit_100_and_credit_99_99_is_refused_as_unbalanced() {
        let bad = entry(json!({
            "entry_id": "N1", "tax_profile_id": PROFILE_A, "entry_date": "2026-07-20",
            "lines": [
                {"line_id": "N1.1", "account": "bank_accounts", "debit": "100.00", "credit": "0.00"},
                {"line_id": "N1.2", "account": "sales_revenue", "debit": "0.00", "credit": "99.99",
                 "measure_side": "credit", "income_class": "business_sales"},
            ],
        }));
        assert_eq!(refusal(post_ledger(&[bad])).code(), Some("unbalanced"));
    }

    #[test]
    fn a_line_with_both_debit_and_credit_is_refused() {
        let bad = entry(json!({
            "entry_id": "N2", "tax_profile_id": PROFILE_A, "entry_date": "2026-07-20",
            "lines": [
                {"line_id": "N2.1", "account": "bank_accounts", "debit": "10.00", "credit": "10.00"},
                {"line_id": "N2.2", "account": "sales_revenue", "debit": "0.00", "credit": "0.00"},
            ],
        }));
        assert_eq!(refusal(post_ledger(&[bad])).code(), Some("line_sides"));
    }

    #[test]
    fn voiding_the_100000_sale_posts_a_reversal_on_the_original_date_and_q3_gross_is_200000() {
        let f = fixture("f-slice-a-2026");
        let ledger = post_ledger(&f.entries).unwrap();
        assert_eq!(
            ledger.reversed_by.get("F-002").map(String::as_str),
            Some("F-005")
        );
        let reversal = f.entries.iter().find(|e| e.entry_id == "F-005").unwrap();
        assert_eq!(reversal.entry_date, "2026-07-15");
        let gross: i64 = ledger
            .lines
            .iter()
            .filter(|l| {
                l.line.income_class == "business_sales"
                    && l.entry_date.as_str() >= "2026-07-01"
                    && l.entry_date.as_str() <= "2026-09-30"
            })
            .map(|l| l.measure)
            .sum();
        assert_eq!(gross, 20_000_000);
    }

    fn original() -> Value {
        sale_entry("R-1", "2026-07-10", "50000.00")
    }

    #[test]
    fn a_reversal_must_mirror_its_original_line_by_line() {
        let mut wrong = reversal_entry("R-2", &original(), "2026-07-10");
        wrong["lines"][1]["debit"] = json!("49999.00");
        let input = norm(&memory(vec![original(), wrong])).unwrap();
        let code = refusal(post_ledger(&input.entries))
            .code()
            .map(str::to_string);
        assert!(
            matches!(code.as_deref(), Some("reversal_not_mirror" | "unbalanced")),
            "{code:?}"
        );
    }

    #[test]
    fn an_orphan_reversal_is_refused() {
        let orphan = reversal_entry("R-3", &original(), "2026-07-10");
        let input = norm(&memory(vec![orphan])).unwrap();
        assert_eq!(
            refusal(post_ledger(&input.entries)).code(),
            Some("orphan_reversal")
        );
    }

    #[test]
    fn a_second_reversal_of_the_same_entry_is_refused() {
        let first = reversal_entry("R-4", &original(), "2026-07-10");
        let second = reversal_entry("R-5", &original(), "2026-07-11");
        let input = norm(&memory(vec![original(), first, second])).unwrap();
        assert_eq!(
            refusal(post_ledger(&input.entries)).code(),
            Some("reversed_twice")
        );
    }

    #[test]
    fn a_reversal_dated_before_its_original_is_refused() {
        let early = reversal_entry("R-6", &original(), "2026-07-01");
        let input = norm(&memory(vec![original(), early])).unwrap();
        assert_eq!(
            refusal(post_ledger(&input.entries)).code(),
            Some("reversal_before_original")
        );
    }

    #[test]
    fn returns_larger_than_sales_block_with_negative_measure_and_are_never_clamped() {
        let raw = acked(
            memory(vec![
                sale_entry("S-1", "2026-07-10", "1000.00"),
                credit_note_entry("CN-1", "2026-07-20", "2000.00", "return"),
            ]),
            &["CN-1"],
        );
        let p = proposal(&norm(&raw).unwrap(), FormCode::F2551Q, "2026-Q3");
        assert_eq!(p.status, "blocked");
        assert!(codes(&p.blocking).contains(&"negative_measure"));
        assert!(p.fill_args.is_empty());
    }
}

// ---- normalization refusals ----------------------------------------------------

mod normalization {
    use super::*;

    #[test]
    fn a_compliance_attribute_on_a_line_is_refused() {
        for banned in [
            json!({"atc": "PT010"}),
            json!({"treatment": "sale_pt"}),
            json!({"tax_types": ["percentage_tax"]}),
            json!({"regime": "graduated"}),
        ] {
            let mut sale = json!({"line_id": "X.2", "account": "sales_revenue", "debit": "0.00", "credit": "1.00",
                "measure_side": "credit", "income_class": "business_sales"});
            for (k, v) in banned.as_object().unwrap() {
                sale[k] = v.clone();
            }
            let raw = json!({"entry_id": "X", "tax_profile_id": PROFILE_A, "entry_date": "2026-07-01",
                "lines": [{"line_id": "X.1", "account": "bank_accounts", "debit": "1.00", "credit": "0.00"}, sale]});
            let ids = std::iter::once(PROFILE_A.to_string()).collect();
            let text = refusal_text(normalize_entry(&raw, &ids, &data().vocab));
            assert!(text.contains("compliance attribute"), "{banned}: {text}");
        }
    }

    #[test]
    fn a_profile_carrying_elections_or_tax_facts_is_refused() {
        let raw = json!({
            "fixture_format": "buwiz-poc-fixture/3", "as_of": "2026-10-29",
            "profiles": [{"tax_profile_id": PROFILE_A, "display": "***-***-***-00000",
                "facts": {"2026": {"regime": "eight_percent"}}}],
            "entries": [],
        });
        assert!(refusal_text(norm(&raw)).contains("Forms"));
    }

    #[test]
    fn a_contra_sales_line_must_say_return_allowance_or_discount() {
        let mut entry = credit_note_entry("CK-1", "2026-07-20", "10.00", "return");
        entry["lines"][0]
            .as_object_mut()
            .unwrap()
            .remove("contra_kind");
        let ids = std::iter::once(PROFILE_A.to_string()).collect();
        assert!(refusal_text(normalize_entry(&entry, &ids, &data().vocab)).contains("contra_kind"));
    }
}

// ---- payroll (1601-C) ------------------------------------------------------------

mod payroll {
    use super::*;

    fn a() -> NormInput {
        fixture("a-sole-prop-2026")
    }

    #[test]
    fn a_larger_employer_share_changes_neither_gross_compensation_nor_item_19() {
        let mut input = a();
        line_mut(&mut input, "A-015", "A-015.3").debit = "9000.00".into();
        line_mut(&mut input, "A-015", "A-015.5").credit = "9000.00".into();
        let p = proposal(&input, FormCode::F1601C, "2026-09");
        assert_eq!(p.status, "ready");
        assert_eq!(fill(&p, "frm1601c:txtTax14"), "42000.00");
        assert_eq!(fill(&p, "frm1601c:txtTax19"), "2000.00");
    }

    #[test]
    fn contribution_side_decides_whose_share_it_is_not_the_scheme() {
        let mut input = a();
        line_mut(&mut input, "A-015", "A-015.5").contribution_scheme = Some("philhealth".into());
        let p = proposal(&input, FormCode::F1601C, "2026-09");
        assert_eq!(fill(&p, "frm1601c:txtTax19"), "2000.00");
        assert!(
            item(&p.warnings, "EXC.EMPLOYER_SHARE_NOT_19")
                .ids
                .contains(&"A-015.5".to_string())
        );
    }

    #[test]
    fn tax_withheld_is_the_amount_payroll_recorded_not_a_computation() {
        let mut input = a();
        line_mut(&mut input, "A-015", "A-015.6").credit = "2808.41".into();
        line_mut(&mut input, "A-015", "A-015.7").credit = "37191.59".into();
        let p = proposal(&input, FormCode::F1601C, "2026-09");
        assert_eq!(fill(&p, "frm1601c:txtTax25"), "2808.41");
    }

    #[test]
    fn item_17_gives_90000_non_taxable_of_95000_paid_and_the_gross_stays_in_item_14() {
        let raw = memory(vec![thirteenth_entry(
            "T-1",
            "2026-12-15",
            "95000.00",
            "EMP-1",
        )]);
        let p = proposal(&norm(&raw).unwrap(), FormCode::F1601C, "2026-12");
        assert_eq!(p.status, "ready");
        assert_eq!(fill(&p, "frm1601c:txtTax17"), "90000.00");
        assert_eq!(fill(&p, "frm1601c:txtTax14"), "95000.00");
        assert_eq!(
            measure(&p, "thirteenth_month_and_benefits_paid"),
            "95000.00"
        );
        assert!(codes(&p.warnings).contains(&"INPUT_LEVEL_TAX_RULE"));
    }

    #[test]
    fn benefits_paid_earlier_in_the_year_use_up_the_item_17_cap() {
        let raw = with_periods(
            memory(vec![
                thirteenth_entry("T-1", "2026-06-15", "20000.00", "EMP-1"),
                thirteenth_entry("T-2", "2026-12-15", "80000.00", "EMP-1"),
            ]),
            json!({"2026-12": "closed"}),
        );
        let p = proposal(&norm(&raw).unwrap(), FormCode::F1601C, "2026-12");
        assert_eq!(fill(&p, "frm1601c:txtTax17"), "70000.00");
        assert_eq!(
            measure(&p, "thirteenth_month_and_benefits_paid[ytd_before_period]"),
            "20000.00"
        );
    }

    #[test]
    fn the_item_17_cap_is_per_employee() {
        let raw = with_periods(
            memory(vec![
                thirteenth_entry("T-1", "2026-06-15", "90000.00", "EMP-1"),
                thirteenth_entry("T-2", "2026-12-15", "10000.00", "EMP-1"),
                thirteenth_entry("T-3", "2026-12-15", "10000.00", "EMP-2"),
            ]),
            json!({"2026-12": "closed"}),
        );
        let p = proposal(&norm(&raw).unwrap(), FormCode::F1601C, "2026-12");
        assert_eq!(fill(&p, "frm1601c:txtTax17"), "10000.00");
    }

    #[test]
    fn a_13th_month_line_without_an_employee_reference_blocks() {
        let mut entry = thirteenth_entry("T-1", "2026-12-15", "1000.00", "EMP-1");
        entry["lines"][0]
            .as_object_mut()
            .unwrap()
            .remove("party_ref");
        let p = proposal(
            &norm(&memory(vec![entry])).unwrap(),
            FormCode::F1601C,
            "2026-12",
        );
        assert!(codes(&p.blocking).contains(&"employee_ref_missing"));
    }

    #[test]
    fn the_rules_file_labels_item_17_and_carries_the_cap() {
        let m = data()
            .rules(FormCode::F1601C)
            .mappings
            .iter()
            .find(|m| m.id == "1601C.II.17")
            .unwrap();
        assert!(
            m.input_level_tax_rule
                .as_deref()
                .unwrap()
                .contains("THE ONE INPUT-LEVEL TAX RULE IN THE BRIDGE")
        );
        assert_eq!(
            m.aggregation_params.as_ref().unwrap()["annual_cap_text"],
            "90000.00"
        );
        let raw = raw_rules(FormCode::F1601C);
        let raw_m = raw["mappings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["id"] == "1601C.II.17")
            .unwrap();
        assert_eq!(raw_m["test_vectors"][0]["expected_text"], "90000.00");
    }

    #[test]
    fn only_one_mapping_carries_an_input_level_tax_rule() {
        let labelled: Vec<&str> = FormCode::ALL
            .iter()
            .flat_map(|f| data().rules(*f).mappings.iter())
            .filter(|m| m.input_level_tax_rule.is_some())
            .map(|m| m.id.as_str())
            .collect();
        assert_eq!(labelled, ["1601C.II.17"]);
    }

    #[test]
    fn a_month_without_payroll_blocks_rather_than_asserting_zero() {
        let raw = memory(vec![sale_entry("S-9", "2026-09-10", "1000.00")]);
        let p = proposal(&norm(&raw).unwrap(), FormCode::F1601C, "2026-09");
        assert!(codes(&p.blocking).contains(&"no_payroll_lines"));
    }

    #[test]
    fn a_compensation_line_not_tagged_to_an_employee_blocks_instead_of_being_dropped() {
        let mut input = a();
        line_mut(&mut input, "A-015", "A-015.1").person_scope = "self".into();
        let p = proposal(&input, FormCode::F1601C, "2026-09");
        assert!(codes(&p.blocking).contains(&"comp_scope_not_employee"));
        assert!(p.fill_args.is_empty());
    }

    #[test]
    fn holiday_and_overtime_pay_blocks_until_the_mwe_split_exists() {
        let mut input = a();
        line_mut(&mut input, "A-015", "A-015.2").comp_component =
            Some("holiday_ot_nsd_hazard".into());
        let p = proposal(&input, FormCode::F1601C, "2026-09");
        assert!(codes(&p.blocking).contains(&"mwe_split_not_implemented"));
    }
}

// ---- certificates (EXC.PT_CWT_NOT_IT, EXC.IT_CWT_NOT_PT) -----------------------

mod certificates {
    use super::*;

    #[test]
    fn a_received_pt_2307_feeds_2551q_item_15_and_never_1701ms_v3_or_v4() {
        let mut input = fixture("a-sole-prop-2026");
        input.entries.push(normalized(
            PROFILE_A,
            &cert_entry(PROFILE_A, "PT-1", "2026-09-30", "3000.00", "pt", "received"),
        ));
        let q3 = proposal(&input, FormCode::F2551Q, "2026-Q3");
        assert_eq!(fill(&q3, "frm2551Qv2018:txt15"), "3000.00");
        let annual = proposal(&input, FormCode::F1701MS, "2026");
        assert_eq!(
            annual.reference_values["frm1701MS:txtCreditableTaxNo3a"],
            "8900.00"
        );
        assert_eq!(
            annual.reference_values["frm1701MS:txtCreditableFourthQuarterNo4a"],
            "3000.00"
        );
        assert_eq!(item(&annual.warnings, "EXC.PT_CWT_NOT_IT").ids, ["PT-1.1"]);
    }

    #[test]
    fn lost_and_disputed_certificates_do_not_count() {
        let base = fixture("g-pt-withholding-2026");
        let pid = base.profiles[0].tax_profile_id.clone();
        let mut input = base.clone();
        input.entries.push(normalized(
            &pid,
            &cert_entry(&pid, "G-L1", "2026-09-30", "1200.00", "pt", "lost"),
        ));
        input.entries.push(normalized(
            &pid,
            &cert_entry(&pid, "G-D1", "2026-09-30", "1300.00", "pt", "disputed"),
        ));
        let p = proposal(&input, FormCode::F2551Q, "2026-Q3");
        assert_eq!(fill(&p, "frm2551Qv2018:txt15"), "3000.00");
    }

    #[test]
    fn certificates_are_windowed_by_their_own_period_end_not_the_posting_date() {
        let raw = accrual(memory(vec![
            sale_entry("S-1", "2026-07-10", "1000.00"),
            cert_entry(PROFILE_A, "C-LATE", "2026-10-05", "30.00", "pt", "received"),
        ]));
        let p = proposal(&norm(&raw).unwrap(), FormCode::F2551Q, "2026-Q3");
        assert_eq!(fill(&p, "frm2551Qv2018:txt15"), "30.00");
    }
}

// ---- returns and the Forms context ------------------------------------------------

mod returns_and_context {
    use super::*;

    fn a() -> NormInput {
        fixture("a-sole-prop-2026")
    }

    #[test]
    fn a_void_posted_after_a_closed_quarter_does_not_change_that_quarter() {
        let original = sale_entry("L-1", "2026-06-10", "70000.00");
        let periods = json!({"2026-Q2": "closed", "2026-Q3": "closed"});
        let before = norm(&with_periods(
            memory(vec![original.clone()]),
            periods.clone(),
        ))
        .unwrap();
        let after = norm(&acked(
            with_periods(
                memory(vec![
                    original.clone(),
                    reversal_entry("L-2", &original, "2026-07-20"),
                ]),
                periods,
            ),
            &["L-2"],
        ))
        .unwrap();
        assert_eq!(
            proposal(&after, FormCode::F2551Q, "2026-Q2").fill_args,
            proposal(&before, FormCode::F2551Q, "2026-Q2").fill_args
        );
        assert!(
            codes(&proposal(&after, FormCode::F2551Q, "2026-Q3").blocking)
                .contains(&"negative_measure")
        );
    }

    #[test]
    fn returns_allowances_and_discounts_are_separate_and_1701ms_item_6_sums_them_once() {
        let raw = acked(
            memory(vec![
                sale_entry("S-1", "2026-07-02", "10000.00"),
                credit_note_entry("R-1", "2026-07-09", "100.00", "return"),
                credit_note_entry("R-2", "2026-07-10", "200.00", "allowance"),
                credit_note_entry("R-3", "2026-07-11", "300.00", "discount"),
            ]),
            &["R-1", "R-2", "R-3"],
        );
        let input = norm(&raw).unwrap();
        let p = proposal(&input, FormCode::F1701MS, "2026");
        assert_eq!(measure(&p, "sales_returns"), "100.00");
        assert_eq!(measure(&p, "sales_allowances"), "200.00");
        assert_eq!(measure(&p, "sales_discounts"), "300.00");
        assert_eq!(fill(&p, "frm1701MS:txtSalesRevenueNo5a"), "10000.00");
        assert_eq!(fill(&p, "frm1701MS:txtSalesReturnsNo6a"), "600.00");
        assert_eq!(
            sum_text(&[fill(&p, "frm1701MS:txtSalesRevenueNo5a"), "-600.00"]),
            "9400.00"
        );
        let q3 = proposal(&input, FormCode::F2551Q, "2026-Q3");
        assert_eq!(fill(&q3, "txtATCAmt1"), "9400.00");
    }

    #[test]
    fn the_1701ms_without_the_forms_tax_option_blocks_and_is_never_defaulted() {
        let p = proposal_ctx(&a(), FormCode::F1701MS, "2026", &FormContext::new());
        assert_eq!(p.status, "blocked");
        assert!(
            item(&p.blocking, "form_context_missing")
                .detail
                .contains("/taxpayer/tax_option")
        );
        assert!(p.fill_args.is_empty());
    }

    #[test]
    fn graduated_without_the_deduction_method_blocks_and_eight_percent_does_not_need_it() {
        let p = proposal_ctx(
            &a(),
            FormCode::F1701MS,
            "2026",
            &ctx(&[("/taxpayer/tax_option", "graduated")]),
        );
        assert!(codes(&p.blocking).contains(&"form_context_missing"));
        let eight = proposal_ctx(&a(), FormCode::F1701MS, "2026", &eight_percent());
        assert_eq!(eight.status, "ready");
    }

    #[test]
    fn an_unmapped_option_or_an_undeclared_context_key_blocks() {
        let p = proposal_ctx(
            &a(),
            FormCode::F1701MS,
            "2026",
            &ctx(&[("/taxpayer/tax_option", "unanswered")]),
        );
        assert!(codes(&p.blocking).contains(&"form_context_unsupported"));
        let q = proposal_ctx(
            &a(),
            FormCode::F2551Q,
            "2026-Q3",
            &ctx(&[("/taxpayer/tax_option", "graduated")]),
        );
        assert!(codes(&q.blocking).contains(&"form_context_undeclared"));
    }

    #[test]
    fn under_itemized_read_from_forms_cost_of_sales_is_proposed() {
        let mut input = a();
        let template = input.entries[0].lines[0].clone();
        let mut cost = template.clone();
        cost.line_id = "CS-1.1".into();
        cost.account = "cost_of_goods_sold".into();
        cost.debit = "5000.00".into();
        cost.credit = "0.00".into();
        cost.measure_side = Some("debit".into());
        cost.income_class = "cost_of_sales".into();
        let mut bank = template;
        bank.line_id = "CS-1.2".into();
        bank.account = "bank_accounts".into();
        bank.debit = "0.00".into();
        bank.credit = "5000.00".into();
        input.entries.push(NormEntry {
            entry_id: "CS-1".into(),
            tax_profile_id: PROFILE_A.into(),
            entry_date: "2026-05-05".into(),
            reverses_entry_id: None,
            lines: vec![cost, bank],
        });
        let p = proposal_ctx(&input, FormCode::F1701MS, "2026", &graduated_itemized());
        assert_eq!(fill(&p, "frm1701MS:txtCostofSalesNo8a"), "5000.00");
    }

    #[test]
    fn the_eight_percent_context_changes_only_the_1701ms_mapping() {
        let b = fixture("b-eight-percent-2026");
        for (form, period) in [(FormCode::F2551Q, "2026-Q3"), (FormCode::F1601C, "2026-09")] {
            assert_eq!(
                proposal(&b, form, period).fill_args,
                proposal(&a(), form, period).fill_args,
                "{period}"
            );
        }
    }
}

// ---- Books holds no compliance logic (ADR 0007) -------------------------------------

mod no_compliance_logic {
    use super::*;

    const COMPLIANCE: [&str; 13] = [
        "rate_bps",
        "pt_rate",
        "vat_threshold",
        "eight_percent_year",
        "GATE.8PCT",
        "GATE.VAT_THRESHOLD",
        "GATE.OVERDUE",
        "graduatedTax",
        "percentOf",
        "dueDate",
        "penalty_computed",
        "penalties_computed",
        "GATE.MIXED_INCOME",
    ];

    fn files(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let path = e.unwrap().path();
            if path.is_dir() {
                files(&path, ext, out);
            } else if path.extension().and_then(|x| x.to_str()) == Some(ext) {
                out.push(path);
            }
        }
    }

    #[test]
    fn no_rust_or_app_source_holds_a_rate_threshold_due_date_or_penalty_helper() {
        let mut paths = Vec::new();
        files(&repo().join("crates"), "rs", &mut paths);
        files(&repo().join("apps"), "tsx", &mut paths);
        files(&repo().join("apps"), "ts", &mut paths);
        assert!(paths.len() > 10, "{} files", paths.len());
        for path in paths {
            let text = std::fs::read_to_string(&path).unwrap();
            for needle in COMPLIANCE {
                assert!(!text.contains(needle), "{}: {needle}", path.display());
            }
        }
    }

    #[test]
    fn the_data_directory_holds_no_rate_file_and_the_rules_hold_no_compliance_gate() {
        let mut names: Vec<String> = std::fs::read_dir(repo().join("data/rules"))
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        assert_eq!(
            names,
            [
                "1601c-v2018.rules.json",
                "1701ms-v2024.rules.json",
                "2551q-v2018.rules.json",
                "common.rules.json",
            ]
        );
        for name in names {
            let text = std::fs::read_to_string(repo().join("data/rules").join(&name)).unwrap();
            for needle in [
                "GATE.VAT_THRESHOLD",
                "GATE.8PCT",
                "GATE.MIXED_INCOME",
                "GATE.OVERDUE",
                "\"ELIG.",
                "rate_bps",
                "\"brackets\"",
            ] {
                assert!(!text.contains(needle), "{name}: {needle}");
            }
        }
    }
}

// ---- vectors: what the byte-for-byte check does not say ------------------------------

mod vector_discipline {
    use super::*;

    fn vector(id: &str) -> Value {
        read_json(&repo().join(format!("fixtures/vectors/{id}.json")))
    }

    #[test]
    fn each_vector_input_is_memo_free() {
        for c in cases() {
            let text = canonical(&vector(&c.id)["input"]).unwrap();
            assert!(!text.contains("memo"), "{}", c.id);
            assert!(!text.contains("MEMO-"), "{}", c.id);
        }
    }

    #[test]
    fn expected_forms_computed_is_documentation_only_and_never_a_books_output_key() {
        let mut documented = 0;
        for c in cases() {
            let v = vector(&c.id);
            let Some(doc) = v.get("expected_forms_computed") else {
                continue;
            };
            documented += 1;
            assert_eq!(doc["asserted_against_books"], false, "{}", c.id);
            for key in doc["values"].as_object().unwrap().keys() {
                assert!(
                    v["expected"]["fill_args"].get(key).is_none(),
                    "{}: {key}",
                    c.id
                );
                assert!(
                    v["expected"]["reference_values"].get(key).is_none(),
                    "{}: {key}",
                    c.id
                );
            }
        }
        assert!(documented > 0);
    }

    #[test]
    fn forms_tax_for_the_2551q_worked_example_is_not_in_the_books_proposal() {
        let v = vector("a-2551q-2026-Q3");
        let tax = v["expected_forms_computed"]["values"]["frm2551Qv2018:txt14"]
            .as_str()
            .unwrap()
            .to_string();
        let p = proposal(&fixture("a-sole-prop-2026"), FormCode::F2551Q, "2026-Q3");
        assert!(!canonical(&p.to_value()).unwrap().contains(&tax));
    }
}

// ---- dates ------------------------------------------------------------------------

mod dates {
    use super::*;

    #[test]
    fn an_instant_with_a_manila_offset_is_taken_on_its_manila_day() {
        assert_eq!(
            Date::manila_from_iso_instant("2026-09-30T23:30:00+08:00"),
            Date::new(2026, 9, 30)
        );
        assert_eq!(
            Date::manila_from_iso_instant("2026-10-01T00:30:00+08:00"),
            Date::new(2026, 10, 1)
        );
    }
}

// ---- the real agent-forms schemas (skipped when absent) --------------------------------

mod forms_schema {
    use super::*;

    /// `BUWIZ_FORMS_SCHEMAS`: a folder holding the pinned agent-forms schemas
    /// (buwiz-oss `crates/bir-core/data/agent-forms`). Unset or absent, every
    /// test here passes with a note.
    fn schema_dir() -> Option<PathBuf> {
        let dir = std::env::var_os("BUWIZ_FORMS_SCHEMAS").map(PathBuf::from);
        match dir {
            Some(dir) if dir.is_dir() => Some(dir),
            _ => {
                eprintln!("SKIPPED forms-schema checks: set BUWIZ_FORMS_SCHEMAS to run them.");
                None
            }
        }
    }

    fn schema_text(dir: &Path, form: FormCode) -> String {
        let file = raw_crosswalk(form)["schema_file"]
            .as_str()
            .unwrap()
            .rsplit('/')
            .next()
            .unwrap()
            .to_string();
        std::fs::read_to_string(dir.join(file)).unwrap()
    }

    /// One form's schema problems, as text.
    fn problems(dir: &Path, form: FormCode) -> Vec<String> {
        let cw = raw_crosswalk(form);
        let text = schema_text(dir, form);
        let mut out = Vec::new();
        let digest = books_core::canonical::sha256_hex(&text);
        if digest != cw["schema_sha256"].as_str().unwrap() {
            out.push(format!("schema file sha256 {digest} differs from the pin"));
        }
        let schema: Value = serde_json::from_str(&text).unwrap();
        let empty = serde_json::Map::new();
        let bindings = schema["bindings"].as_object().unwrap_or(&empty);
        let unmapped = schema["unmapped"].as_object().unwrap_or(&empty);
        let boxes: BTreeSet<&str> = schema["boxes"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|b| b["key"].as_str())
            .collect();
        let fields = cw["fields"].as_array().unwrap();
        for field in fields {
            let fill_key = field["fill_key"].as_str();
            let Some(key) = field["key"].as_str() else {
                if fill_key.is_some() {
                    out.push(format!(
                        "{}: row family marked sendable",
                        field["key_pattern"]
                    ));
                }
                continue;
            };
            if !bindings.contains_key(key) && !boxes.contains(key) && !unmapped.contains_key(key) {
                out.push(format!("{key}: absent from the schema"));
            }
            let lock_of = |k: &str| -> String {
                bindings
                    .get(k)
                    .and_then(|b| b["lock"].as_str())
                    .unwrap_or("none")
                    .to_string()
            };
            if let Some(fk) = fill_key {
                if fk == key {
                    if !bindings.contains_key(key) {
                        out.push(format!("{key}: sendable but has no binding"));
                    } else if lock_of(key) != "none" {
                        out.push(format!("{key}: sendable but locked"));
                    }
                    if unmapped.contains_key(key) {
                        out.push(format!("{key}: sendable but unmapped"));
                    }
                } else {
                    if !boxes.contains(key) {
                        out.push(format!("{key}: typed alias {fk} needs an official box"));
                    }
                    if !field["typed_alias_status"]
                        .as_str()
                        .unwrap_or("")
                        .starts_with("CONFIRMED")
                    {
                        out.push(format!("{key}: typed alias {fk} is not verified"));
                    }
                }
            }
            let class = field["fill_class"].as_str().unwrap();
            if let Some(wanted) = class.strip_prefix("locked_")
                && lock_of(key) != wanted
            {
                out.push(format!("{key}: crosswalk {class} but schema lock differs"));
            }
            if class == "computed" && bindings.contains_key(key) {
                out.push(format!(
                    "{key}: crosswalk says computed but the schema binds it"
                ));
            }
        }
        let rw = data().crosswalk(form);
        for m in &data().rules(form).mappings {
            let Some(target) = rw
                .by_fill_key(&m.target_key)
                .or_else(|| rw.by_key(&m.target_key))
            else {
                out.push(format!("{}: target is not a crosswalk key", m.id));
                continue;
            };
            if !target.rule_ids.contains(&m.id) {
                out.push(format!("{}: target does not cite the mapping", m.id));
            }
            if target.fill_class != "input" {
                out.push(format!("{}: target is {}", m.id, target.fill_class));
            }
            if m.mode == "fill"
                && (target.fill_key.is_none() || !target.books_policy.starts_with("fill_"))
            {
                out.push(format!(
                    "{}: fill mapping with a target that is not sendable",
                    m.id
                ));
            }
        }
        let mut sets: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for binding in bindings.values() {
            for option in binding["options"].as_array().into_iter().flatten() {
                for (path, value) in option["set"].as_object().into_iter().flatten() {
                    if let Some(v) = value.as_str() {
                        sets.entry(path.clone()).or_default().insert(v.to_string());
                    }
                }
            }
        }
        for input in &data().rules(form).form_context_inputs {
            let Some(values) = sets.get(&input.path) else {
                out.push(format!(
                    "form context {}: no schema binding sets it",
                    input.path
                ));
                continue;
            };
            for v in &input.supported_values {
                if !values.contains(v) {
                    out.push(format!(
                        "form context {}: no schema option sets {v}",
                        input.path
                    ));
                }
            }
        }
        out
    }

    #[test]
    fn each_schema_matches_its_pin_and_every_crosswalk_key_mapping_and_context_path_checks_out() {
        let Some(dir) = schema_dir() else { return };
        for form in FormCode::ALL {
            assert_eq!(
                problems(&dir, form),
                Vec::<String>::new(),
                "{}",
                form.as_str()
            );
        }
    }

    #[test]
    fn the_crosswalk_and_rules_pin_the_same_schema_digest_for_each_form() {
        let Some(dir) = schema_dir() else { return };
        for form in FormCode::ALL {
            let digest = books_core::canonical::sha256_hex(&schema_text(&dir, form));
            assert_eq!(digest, data().crosswalk(form).schema_sha256);
            assert_eq!(data().rules(form).schema_sha256, digest);
        }
    }

    #[test]
    fn every_sendable_key_in_every_case_is_a_bound_unlocked_key_or_a_verified_typed_alias() {
        let Some(dir) = schema_dir() else { return };
        let schemas: BTreeMap<&str, Value> = FormCode::ALL
            .iter()
            .map(|f| {
                (
                    f.as_str(),
                    serde_json::from_str(&schema_text(&dir, *f)).unwrap(),
                )
            })
            .collect();
        for (id, form, p) in all_proposals() {
            let schema = &schemas[form.as_str()];
            let raw = raw_crosswalk(form);
            for key in p.fill_args.keys() {
                let field = raw["fields"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|f| f["fill_key"] == key.as_str())
                    .unwrap();
                if field["key"] == key.as_str() {
                    let binding = &schema["bindings"][key];
                    assert!(binding.is_object(), "{id}: {key} must be bound");
                    assert!(binding["lock"].is_null(), "{id}: {key} must not be locked");
                } else {
                    assert!(
                        field["typed_alias_status"]
                            .as_str()
                            .unwrap_or("")
                            .starts_with("CONFIRMED"),
                        "{id}: {key} alias"
                    );
                    assert!(
                        schema["boxes"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|b| b["key"] == field["key"]),
                        "{id}: {key} official box"
                    );
                }
            }
        }
    }
}
