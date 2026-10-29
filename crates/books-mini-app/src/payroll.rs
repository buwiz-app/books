//! M6: Payroll and Certificates, the two record pages the Forms page leans
//! on. Amounts only (docs/08): no employee TIN, no 2316, no tax table.
//!
//! - **A pay run** is one journal entry whose lines carry payroll facts
//!   (`document_kind` `payroll_register`, the pay component, the employee
//!   as an opaque `party_ref`). Tax withheld is what payroll recorded;
//!   Books never computes it. Saving makes a draft; the Ledger's Post
//!   button posts it.
//! - **A certificate** (2307, or the final-tax 2306) is a posted line with
//!   the certificate facts as printed on the paper. Its status changes
//!   later without touching the line (`books_store::certificates`). Which
//!   ones count is the bridge's rule: a received 2307 counts, a 2306 never.

use std::collections::BTreeSet;

use books_core::catalog::{Catalog, Facts};
use books_core::date::{Date, MONTHS, Period, PeriodKind};
use books_core::input::FormCode;
use books_core::ledger::{KindRef, Status};
use books_core::money;
use books_core::rules;
use books_store::Store;
use books_store::books::{self, Account, Book};
use books_store::certificates;
use books_store::entries::{self, DraftInput, Filter, Form, LineInput};
use books_store::months;
use books_store::people;
use books_store::rusqlite::Connection;
use serde_json::{Map, Value, json};

use crate::service::{Answer, Origin, Refusal, required, text, ticket, with_subject};
use crate::{views, writes};

/// The pay components that make up gross pay.
const GROSS: [&str; 5] = [
    "basic_pay",
    "statutory_minimum_wage",
    "holiday_ot_nsd_hazard",
    "thirteenth_month_and_benefits",
    "de_minimis",
];

/// The first active, confirmed account of `kind`.
fn account_of(conn: &Connection, book: &Book, kind: &str) -> Result<Account, Refusal> {
    books::accounts(conn, book.id)?
        .into_iter()
        .find(|a| a.active && a.kind_confirmed && a.kind == KindRef::Kind(kind.into()))
        .ok_or_else(|| {
            let name = Catalog::get().kind(kind).map_or(kind, |k| k.name.as_str());
            Refusal::new(
                "no_account",
                format!("Add an account of kind {name} first (Accounts)."),
            )
        })
}

fn range_label(from: Date, to: Date) -> String {
    let m = |d: Date| books_core::date::MONTH_NAMES[usize::from(d.month - 1)];
    if from.day == 1 && to == to.month_end() && from.month == to.month {
        format!("{} {}", m(from), from.year)
    } else if from.month == to.month {
        format!("{} {}–{}", m(from), from.day, to.day)
    } else {
        format!(
            "{} {} – {} {}",
            MONTHS[usize::from(from.month - 1)],
            from.day,
            MONTHS[usize::from(to.month - 1)],
            to.day
        )
    }
}

/// `books_wages_list`: the pay runs, newest first.
pub fn list(store: &Store, book: Book) -> Answer {
    let c = store.conn();
    let accounts: std::collections::HashMap<i64, Account> = books::accounts(c, book.id)?
        .into_iter()
        .map(|a| (a.id, a))
        .collect();
    let closed: BTreeSet<String> = months::months(c, book.id, Date::today())?
        .into_iter()
        .filter(|(_, closed)| *closed)
        .map(|(p, _)| p.key)
        .collect();
    let mut runs = Vec::new();
    for e in entries::list(c, book.id, &Filter::default())? {
        if e.reverses.is_some() || e.status == Status::Discarded {
            continue;
        }
        let mut people = BTreeSet::new();
        let (mut gross, mut is_run, mut span) = (0i64, false, None);
        for l in entries::lines(c, e.id)? {
            let Some(a) = accounts.get(&l.account_id) else {
                continue;
            };
            let f = a.kind.facts_for(&l.own);
            if f.document_kind.as_deref() != Some("payroll_register") {
                continue;
            }
            is_run = true;
            if let Some(p) = &f.party_ref {
                people.insert(p.clone());
            }
            if f.comp_component
                .as_deref()
                .is_some_and(|k| GROSS.contains(&k))
            {
                gross += l.debit - l.credit;
            }
            if span.is_none()
                && let (Some(s), Some(t)) = (
                    f.period_start.as_deref().and_then(Date::parse),
                    f.period_end.as_deref().and_then(Date::parse),
                )
            {
                span = Some((s, t));
            }
        }
        if !is_run {
            continue;
        }
        let month = Period::of(e.date, PeriodKind::Month);
        let short = MONTHS[usize::from(e.date.month - 1)];
        let (bir, bir_tone) = match e.status {
            Status::Draft => ("Waiting for this run".to_string(), "mute"),
            Status::Voided => ("Voided".to_string(), "mute"),
            _ if closed.contains(&month.key) => (format!("1601-C {short} ready"), "bir"),
            _ => (format!("Part of 1601-C {short}"), "mute"),
        };
        let (tone, label) = views::status_pill(&e);
        let n = people.len();
        runs.push(json!({
            "ticket": ticket(store, "entry", e.id)?,
            "label": span.map_or_else(|| e.memo.clone().unwrap_or_else(|| "Pay run".into()), |(s, t)| range_label(s, t)),
            "date": e.date.short(),
            "people": format!("{n} employee{}", if n == 1 { "" } else { "s" }),
            "gross_shown": money::display(gross),
            "status": e.status.as_str(),
            "tone": tone,
            "status_label": label,
            "bir": bir,
            "bir_tone": bir_tone,
        }));
    }
    let employees: Vec<Value> = people::people(c, book.id, Some("employee"))?
        .into_iter()
        .filter(|p| p.active)
        .map(|p| -> Result<Value, Refusal> {
            Ok(json!({ "ticket": ticket(store, "person", p.id)?, "label": p.name }))
        })
        .collect::<Result<_, _>>()?;
    let money_accounts: Vec<Value> = accounts
        .values()
        .filter(|a| a.active && a.is_cash())
        .map(|a| -> Result<Value, Refusal> { Ok(json!({ "ticket": ticket(store, "account", a.id)?, "label": format!("{} {}", a.number, a.name) })) })
        .collect::<Result<_, _>>()?;
    Ok(json!({ "runs": runs, "employees": employees, "money_accounts": money_accounts }))
}

fn cents(v: &Map<String, Value>, key: &str) -> Result<i64, Refusal> {
    match v.get(key).and_then(Value::as_str) {
        None | Some("") => Ok(0),
        Some(t) => {
            let c = money::parse(t).map_err(|e| Refusal::invalid(format!("`{key}`: {e}")))?;
            if c < 0 {
                return Err(Refusal::invalid(format!("`{key}` is not negative.")));
            }
            Ok(c)
        }
    }
}

/// `books_app_wage_run_save`: a pay run as a draft entry. Each employee's
/// amounts are what payroll recorded; net pay leaves the money account.
pub fn save(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let book = writes::book_in(conn, args)?;
    let date = |k: &str| -> Result<Date, Refusal> {
        Date::parse(required(args, k)?)
            .ok_or_else(|| Refusal::invalid(format!("`{k}` is YYYY-MM-DD.")))
    };
    let (from, to) = (date("period_from")?, date("period_to")?);
    if from > to || from.year != to.year {
        return Err(Refusal::invalid(
            "A pay period runs forward within one year.",
        ));
    }
    let paid = match text(args, "date") {
        Some(_) => date("date")?,
        None => to,
    };
    let money_account = writes::account_in(conn, &book, required(args, "account")?)?;
    if !money_account.is_cash() {
        return Err(Refusal::invalid("Net pay leaves a bank or cash account."));
    }
    let people_in = args
        .get("employees")
        .and_then(Value::as_array)
        .filter(|a| !a.is_empty() && a.len() <= 200)
        .ok_or_else(|| Refusal::invalid("`employees` lists 1 to 200 employees."))?;
    let salaries = account_of(conn, &book, "salaries")?;
    let benefits = account_of(conn, &book, "employee_benefits")?;
    let employer = account_of(conn, &book, "employer_payroll_taxes")?;
    let withheld = account_of(conn, &book, "ph_wtc_payable")?;
    let schemes = [
        (
            "sss",
            "sss_gsis",
            account_of(conn, &book, "ph_sss_payable")?,
        ),
        (
            "philhealth",
            "philhealth",
            account_of(conn, &book, "ph_philhealth_payable")?,
        ),
        (
            "pagibig",
            "pagibig",
            account_of(conn, &book, "ph_pagibig_payable")?,
        ),
    ];
    let mut lines = Vec::new();
    let mut seen = BTreeSet::new();
    let mut net_total = 0i64;
    for p in people_in {
        let p = p
            .as_object()
            .ok_or_else(|| Refusal::invalid("Each employee is an object."))?;
        let person_id = match crate::service::text(p, "person") {
            Some(t) => match books_store::plumbing::subject(conn, t)? {
                Some((k, id)) if k == "person" => id,
                _ => {
                    return Err(Refusal::new(
                        "not_found",
                        "That person ticket is not one Books issued.",
                    ));
                }
            },
            None => return Err(Refusal::invalid("Each employee names a `person`.")),
        };
        let person = people::person(conn, person_id)?
            .filter(|x| x.book_id == book.id && x.kind == "employee")
            .ok_or_else(|| Refusal::invalid("A pay run lists employees of this book."))?;
        if !seen.insert(person.id) {
            return Err(Refusal::invalid(format!(
                "{} is listed twice.",
                person.name
            )));
        }
        let fact = |component: &str, side: &str| Facts {
            measure_side: Some(side.into()),
            document_kind: Some("payroll_register".into()),
            person_scope: Some("employee".into()),
            party_ref: Some(format!("emp-{}", person.id)),
            comp_component: Some(component.into()),
            contribution_side: Some("none".into()),
            period_start: Some(from.to_string()),
            period_end: Some(to.to_string()),
            ..Facts::default()
        };
        let mut gross = 0i64;
        for (key, account, component) in [
            ("basic", &salaries, "basic_pay"),
            ("minimum_wage", &salaries, "statutory_minimum_wage"),
            ("overtime", &salaries, "holiday_ot_nsd_hazard"),
            ("thirteenth", &benefits, "thirteenth_month_and_benefits"),
            ("de_minimis", &benefits, "de_minimis"),
        ] {
            let c = cents(p, key)?;
            if c > 0 {
                gross += c;
                lines.push(LineInput {
                    account_id: account.id,
                    debit: c,
                    credit: 0,
                    facts: fact(component, "debit"),
                });
            }
        }
        if gross == 0 {
            return Err(Refusal::invalid(format!(
                "{} has no pay in this run.",
                person.name
            )));
        }
        let mut deducted = 0i64;
        for (key, scheme, payable) in &schemes {
            let ee = cents(p, &format!("ee_{key}"))?;
            let er = cents(p, &format!("er_{key}"))?;
            let contribution = |side: &str, ms: &str| Facts {
                contribution_side: Some(side.into()),
                contribution_scheme: Some((*scheme).into()),
                ..fact("contribution", ms)
            };
            if ee > 0 {
                deducted += ee;
                lines.push(LineInput {
                    account_id: payable.id,
                    debit: 0,
                    credit: ee,
                    facts: contribution("employee", "credit"),
                });
            }
            if er > 0 {
                lines.push(LineInput {
                    account_id: employer.id,
                    debit: er,
                    credit: 0,
                    facts: contribution("employer", "debit"),
                });
                lines.push(LineInput {
                    account_id: payable.id,
                    debit: 0,
                    credit: er,
                    facts: contribution("employer", "credit"),
                });
            }
        }
        let tax = cents(p, "tax_withheld")?;
        if tax > 0 {
            deducted += tax;
            lines.push(LineInput {
                account_id: withheld.id,
                debit: 0,
                credit: tax,
                facts: fact("tax_withheld", "credit"),
            });
        }
        if deducted > gross {
            return Err(Refusal::invalid(format!(
                "{}'s deductions are more than the pay.",
                person.name
            )));
        }
        net_total += gross - deducted;
    }
    if net_total > 0 {
        lines.push(LineInput {
            account_id: money_account.id,
            debit: 0,
            credit: net_total,
            facts: Facts::default(),
        });
    }
    let n = seen.len();
    let draft = DraftInput {
        date: paid,
        form: Form::Journal,
        payee: Some("Payroll".into()),
        memo: Some(format!(
            "Payroll · {} · {n} employee{}",
            range_label(from, to),
            if n == 1 { "" } else { "s" }
        )),
        reference: None,
        needs_review: false,
        lines,
    };
    let id = entries::save_draft(conn, &book, None, &draft, "app")?;
    Ok(with_subject(
        json!({ "status": "draft", "net_shown": money::display(net_total) }),
        "entry",
        id,
    ))
}

// --- certificates -----------------------------------------------------------

/// The box a certificate feeds, read from the rule sets: the mapping whose
/// measure reads received 2307s of that kind.
fn counts_toward(credit_kind: &str, period_end: Option<Date>) -> Option<String> {
    let data = rules::shared().ok()?;
    let (form, measure) = match credit_kind {
        "pt" => (FormCode::F2551Q, "pt_withheld_2307_received"),
        "it" => (FormCode::F1701MS, "it_withheld_2307_received"),
        _ => return None,
    };
    let q4 = period_end.is_some_and(|d| d.month >= 10);
    let m = data.rules(form).mappings.iter().find(|m| {
        m.measures.iter().any(|x| x == measure)
            && match m.sub_window.as_deref() {
                Some("Q4") => q4,
                Some(_) => !q4,
                None => true,
            }
    })?;
    // mapping ids read "2551Q.II.15" and "1701MS.V.3"
    Some(match form {
        FormCode::F2551Q => format!("2551Q item {}", m.id.rsplit('.').next().unwrap_or("")),
        _ => format!("1701-MS {}", m.id.trim_start_matches("1701MS.")),
    })
}

fn quarter_label(c: &certificates::Certificate) -> String {
    let d = c.period_end.unwrap_or(c.date);
    format!("Q{} {}", d.quarter(), d.year)
}

/// `books_certificates_list`: the certificates customers sent, and what
/// each counts toward.
pub fn list_certificates(
    store: &Store,
    book: Book,
    args: &Map<String, Value>,
    origin: Origin,
) -> Answer {
    let filter = text(args, "filter").unwrap_or("all");
    let mut rows = Vec::new();
    let mut counts = json!({ "all": 0, "received": 0, "pending": 0 });
    for c in certificates::list(store.conn(), book.id)? {
        if c.voided {
            continue;
        }
        counts["all"] = json!(counts["all"].as_i64().unwrap_or(0) + 1);
        if let Some(n) = counts.get(c.status.as_str()).and_then(Value::as_i64) {
            counts[c.status.as_str()] = json!(n + 1);
        }
        if filter != "all" && c.status != filter {
            continue;
        }
        let (toward, bir) = if c.form == "2306" {
            ("Never counted: final".to_string(), false)
        } else if c.status != "received" {
            ("Not counted until received".to_string(), false)
        } else {
            match counts_toward(&c.credit_kind, c.period_end) {
                Some(t) => (t, true),
                None => ("Not a 2551Q or 1701-MS credit".to_string(), false),
            }
        };
        let kind = match (c.form, c.credit_kind.as_str()) {
            ("2306", _) => "Final tax (2306)",
            (_, "pt") => "Percentage tax",
            (_, "it") => "Income tax",
            _ => "Other",
        };
        let mut row = json!({
            "ticket": ticket(store, "cert", c.line_id)?,
            "entry": ticket(store, "entry", c.entry_id)?,
            "from": c.from.clone().unwrap_or_else(|| "—".into()),
            "quarter": quarter_label(&c),
            "kind": kind,
            "atc": c.atc,
            "amount_shown": money::display(c.centavos),
            "status": c.status,
            "counts": toward,
            "counts_bir": bir,
        });
        if !origin.is_app() {
            row.as_object_mut().expect("an object").remove("from");
        }
        rows.push(row);
    }
    let c = store.conn();
    let customers: Vec<Value> = people::people(c, book.id, Some("customer"))?
        .into_iter()
        .filter(|p| p.active)
        .map(|p| -> Result<Value, Refusal> {
            Ok(json!({ "ticket": ticket(store, "person", p.id)?, "label": p.name }))
        })
        .collect::<Result<_, _>>()?;
    let revenue: Vec<Value> = books::accounts(c, book.id)?
        .into_iter()
        .filter(|a| a.active && a.account_type.is_pl() && a.account_type.group() == "Revenue")
        .map(|a| -> Result<Value, Refusal> { Ok(json!({ "ticket": ticket(store, "account", a.id)?, "label": format!("{} {}", a.number, a.name) })) })
        .collect::<Result<_, _>>()?;
    // a sale recorded in full as money in: the withheld part comes out of the bank
    let money: Vec<Value> = books::accounts(c, book.id)?
        .into_iter()
        .filter(|a| a.active && a.is_cash())
        .map(|a| -> Result<Value, Refusal> { Ok(json!({ "ticket": ticket(store, "account", a.id)?, "label": format!("{} {}", a.number, a.name) })) })
        .collect::<Result<_, _>>()?;
    let atc: Vec<Value> = rules::shared()
        .map(|d| {
            let mut v: Vec<&String> = d.vocab.certificate_atc.iter().collect();
            v.sort();
            v.into_iter()
                .map(|a| json!({ "ticket": a, "label": a }))
                .collect()
        })
        .unwrap_or_default();
    Ok(
        json!({ "rows": rows, "counts": counts, "customers": customers, "revenue_accounts": revenue, "money_accounts": money, "atc": atc }),
    )
}

/// `books_app_certificate_add`: record a certificate as it is printed and
/// post it against the sale it belongs to: a sale recorded net of the tax
/// withheld takes it as income (`against` a revenue account, the default);
/// a sale recorded in full as money in gives it back from the bank
/// (`against` that account). The ATC is copied from the paper, never
/// chosen by Books.
pub fn add_certificate(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let book = writes::book_in(conn, args)?;
    let form = match required(args, "form")? {
        "2307" => "certificate_2307",
        "2306" => "certificate_2306",
        _ => return Err(Refusal::invalid("`form` is 2307 or 2306.")),
    };
    let credit_kind = match required(args, "credit_kind")? {
        k @ ("pt" | "it") => k,
        _ => {
            return Err(Refusal::invalid(
                "`credit_kind` is pt (percentage tax) or it (income tax).",
            ));
        }
    };
    let status = match text(args, "status").unwrap_or("received") {
        s @ ("received" | "pending") => s,
        _ => {
            return Err(Refusal::invalid(
                "A new certificate is received or pending.",
            ));
        }
    };
    let atc = required(args, "atc")?;
    if !Catalog::get().allowed("certificate_atc", atc) {
        return Err(Refusal::invalid(
            "Copy the ATC as the paper prints it (WB080, WB082, WB084, WI010 or WC010).",
        ));
    }
    let quarter = Period::parse(required(args, "quarter")?)
        .filter(|p| p.kind == PeriodKind::Quarter)
        .ok_or_else(|| Refusal::invalid("`quarter` is like 2026-Q3."))?;
    let amount = writes::amount(args, "amount")?;
    let date = match text(args, "date") {
        Some(d) => Date::parse(d).ok_or_else(|| Refusal::invalid("`date` is YYYY-MM-DD."))?,
        None => Date::today(),
    };
    let receivable = account_of(
        conn,
        &book,
        if credit_kind == "pt" {
            "pct_tax_credit_receivable"
        } else {
            "ph_cwt_receivable"
        },
    )?;
    let against = match text(args, "against") {
        Some(a) => writes::account_in(conn, &book, a)?,
        None => account_of(conn, &book, "sales_revenue")?,
    };
    if against.id == receivable.id {
        return Err(Refusal::invalid(
            "Pick the sale's account, not the certificate's.",
        ));
    }
    let cert = Facts {
        measure_side: Some("debit".into()),
        document_kind: Some(form.into()),
        credit_kind: Some(credit_kind.into()),
        certificate_atc: Some(atc.into()),
        certificate_status: Some(status.into()),
        period_start: Some(quarter.start.to_string()),
        period_end: Some(quarter.end.to_string()),
        ..Facts::default()
    };
    // the sale side: an income line is a sale invoice (cash books never use payment_receipt)
    let mut sale = Facts::default();
    if against.account_type.is_pl() {
        sale.document_kind = Some("sale_invoice".into());
    }
    let mut draft = DraftInput {
        date,
        form: Form::Journal,
        payee: None,
        memo: Some(format!(
            "{} received",
            if form == "certificate_2307" {
                "2307"
            } else {
                "2306"
            }
        )),
        reference: text(args, "reference").map(str::to_string),
        needs_review: false,
        lines: vec![
            LineInput {
                account_id: receivable.id,
                debit: amount,
                credit: 0,
                facts: cert,
            },
            LineInput {
                account_id: against.id,
                debit: 0,
                credit: amount,
                facts: sale,
            },
        ],
    };
    if let Some(t) = text(args, "person") {
        let id = match books_store::plumbing::subject(conn, t)? {
            Some((k, id)) if k == "person" => id,
            _ => {
                return Err(Refusal::new(
                    "not_found",
                    "That person ticket is not one Books issued.",
                ));
            }
        };
        let p = people::person(conn, id)?
            .filter(|p| p.book_id == book.id)
            .ok_or_else(|| Refusal::new("not_found", "That person is not in this book."))?;
        draft.payee = Some(p.name.clone());
        let entry = entries::save_draft(conn, &book, None, &draft, "app")?;
        people::set_entry_links(conn, &book, entry, Some(p.id), None, None)?;
        let g = writes::gate(conn, &book);
        entries::post(conn, &book, entry, &g)?;
        return Ok(with_subject(json!({ "status": status }), "entry", entry));
    }
    Err(Refusal::invalid(
        "Name the customer who sent it (`person`).",
    ))
}

/// `books_app_certificate_set`: the paper arrived, or was lost or disputed.
pub fn set_certificate(conn: &Connection, args: &Map<String, Value>) -> Answer {
    let book = writes::book_in(conn, args)?;
    let line = match books_store::plumbing::subject(conn, required(args, "certificate")?)? {
        Some((k, id)) if k == "cert" => id,
        _ => {
            return Err(Refusal::new(
                "not_found",
                "That certificate ticket is not one Books issued.",
            ));
        }
    };
    let status = required(args, "status")?;
    certificates::set_status(conn, &book, line, status)?;
    Ok(with_subject(json!({ "status": status }), "cert", line))
}
