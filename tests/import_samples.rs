//! The M5 import samples (`fixtures/import/`): a web Buwiz Books export, a
//! QuickBooks chart and journal, and a plain spreadsheet chart and journal.
//! Each parses; every entry balances; Q3 2026 sales are 300,000.00 and
//! returns 10,000.00 in all three; Map 1 finds the sales, returns and bank
//! accounts; and no name or memo keeps a run of nine digits.

use books_core::catalog::AccountType;
use books_core::date::Date;
use books_core::import::{
    Account, AccountRef, Entry, Plan, Source, parse_csv_chart, parse_csv_journal, parse_web_export,
    suggest_kind,
};

fn sample(name: &str) -> String {
    let path = format!(
        "{}/../../fixtures/import/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn web() -> Plan {
    parse_web_export(&sample("web-books-sample.json")).unwrap()
}

fn csv(prefix: &str) -> (Vec<Account>, Vec<Entry>) {
    (
        parse_csv_chart(&sample(&format!("{prefix}-chart.csv"))).unwrap(),
        parse_csv_journal(&sample(&format!("{prefix}-journal.csv"))).unwrap(),
    )
}

fn q3(date: Date) -> bool {
    Date::parse("2026-07-01").unwrap() <= date && date <= Date::parse("2026-09-30").unwrap()
}

/// Credits minus debits on `account`, over posted Q3 2026 entries.
fn q3_credit_net(entries: &[Entry], account: &AccountRef) -> i64 {
    entries
        .iter()
        .filter(|e| e.posted && q3(e.date))
        .flat_map(|e| &e.lines)
        .filter(|l| &l.account == account)
        .map(|l| l.credit - l.debit)
        .sum()
}

fn balanced(entries: &[Entry]) {
    assert!(!entries.is_empty());
    for e in entries {
        let d: i64 = e.lines.iter().map(|l| l.debit).sum();
        let c: i64 = e.lines.iter().map(|l| l.credit).sum();
        assert_eq!(d, c, "{:?} on {}", e.reference, e.date);
        assert!(d > 0);
        for l in &e.lines {
            assert!(l.debit >= 0 && l.credit >= 0 && (l.debit == 0) != (l.credit == 0));
        }
    }
}

fn has_nine_digit_run(text: &str) -> bool {
    let mut run = 0;
    for c in text.chars() {
        run = if c.is_ascii_digit() { run + 1 } else { 0 };
        if run >= 9 {
            return true;
        }
    }
    false
}

fn texts_clean(accounts: &[Account], entries: &[Entry], more: &[&str]) {
    let mut all: Vec<&str> = more.to_vec();
    for a in accounts {
        all.push(&a.name);
        all.extend(a.parent.as_deref());
    }
    for e in entries {
        all.extend(e.memo.as_deref());
        all.extend(e.party.as_deref());
        all.extend(e.reference.as_deref());
        for l in &e.lines {
            if let AccountRef::Name(n) = &l.account {
                all.push(n);
            }
            all.extend(l.description.as_deref());
            all.extend(l.party.as_deref());
            all.extend(l.department.as_deref());
            all.extend(l.location.as_deref());
        }
    }
    for t in all {
        assert!(!has_nine_digit_run(t), "{t}");
    }
}

fn suggested(accounts: &[Account], name: &str) -> (String, u8) {
    let a = accounts
        .iter()
        .find(|a| a.name == name)
        .unwrap_or_else(|| panic!("{name}"));
    let s = suggest_kind(a.number.as_deref(), &a.name, a.account_type);
    (s.kind, s.confidence)
}

fn assert_map(accounts: &[Account], sales: &str, returns: &str, bank: &str) {
    let (k, c) = suggested(accounts, sales);
    assert!(k == "sales_revenue" && c >= 80, "{sales}: {k} {c}");
    let (k, c) = suggested(accounts, returns);
    assert!(
        k == "refunds_and_discounts" && c >= 80,
        "{returns}: {k} {c}"
    );
    assert_eq!(suggested(accounts, bank).0, "bank_accounts", "{bank}");
}

#[test]
fn the_web_sample() {
    let plan = web();
    assert_eq!(plan.source, Source::WebBooks { version: 4 });
    assert_eq!(plan.accounts.len(), 11);
    let sales = plan
        .accounts
        .iter()
        .find(|a| a.name == "Sales Revenue")
        .unwrap();
    assert_eq!(sales.number.as_deref(), Some("41000"));
    assert_eq!(sales.parent.as_deref(), Some("Revenue"));
    assert!(
        !plan
            .accounts
            .iter()
            .find(|a| a.name == "Interest Income")
            .unwrap()
            .active
    );
    let customers: Vec<&str> = plan
        .people
        .iter()
        .filter(|p| p.kind == "customer")
        .map(|p| p.name.as_str())
        .collect();
    assert_eq!(customers, ["Bayani Foods", "Luntian Cafe"]);
    assert!(plan.people.iter().any(|p| p.kind == "owner"));
    assert_eq!(plan.tags.len(), 2);

    // Six transactions: five posted, one draft; the voided one is skipped.
    assert_eq!(plan.entries.len(), 6);
    assert_eq!(plan.entries.iter().filter(|e| e.posted).count(), 5);
    assert!(plan.entries.windows(2).all(|w| w[0].date <= w[1].date));
    balanced(&plan.entries);
    let first = &plan.entries[0];
    assert_eq!(first.reference.as_deref(), Some("JE-0001"));
    assert_eq!(first.lines[0].debit, 12_000_000);
    assert_eq!(first.lines[1].department.as_deref(), Some("Store"));

    let n = |s: &str| AccountRef::Number(s.into());
    assert_eq!(q3_credit_net(&plan.entries, &n("41000")), 30_000_000);
    assert_eq!(-q3_credit_net(&plan.entries, &n("49000")), 1_000_000);

    let skipped: Vec<&str> = plan.skipped.iter().map(|s| s.what.as_str()).collect();
    for what in [
        "bills",
        "phOrgTaxProfile",
        "phTaxCertificates",
        "banks",
        "voided transactions",
    ] {
        assert!(skipped.contains(&what), "{what} in {skipped:?}");
    }
    for s in &plan.skipped {
        if s.what.starts_with("ph") {
            assert!(s.why.contains("eBIRForms"));
        }
    }
    let text = format!("{plan:?}");
    for never in [
        "SYNTH-NOT-A-TIN",
        "tinMasked",
        "XXX-XXX",
        "XXXX-",
        "orders@",
        "October shop rent",
    ] {
        assert!(!text.contains(never), "{never}");
    }
    let names: Vec<&str> = plan
        .people
        .iter()
        .map(|p| p.name.as_str())
        .chain(plan.tags.iter().map(|t| t.name.as_str()))
        .collect();
    texts_clean(&plan.accounts, &plan.entries, &names);
    assert_map(
        &plan.accounts,
        "Sales Revenue",
        "Refunds & Discounts",
        "Bank Accounts",
    );
    assert_eq!(
        suggested(&plan.accounts, "Sales Revenue"),
        ("sales_revenue".into(), 96)
    );
    assert_eq!(
        suggested(&plan.accounts, "Uncategorized Expense"),
        ("unsure".into(), 30)
    );
}

#[test]
fn the_quickbooks_sample() {
    let (chart, entries) = csv("quickbooks");
    assert_eq!(chart.len(), 13);
    let services = chart.iter().find(|a| a.name == "Income:Services").unwrap();
    assert_eq!(services.account_type, AccountType::Revenue);
    assert_eq!(services.parent.as_deref(), Some("Income"));
    assert_eq!(
        chart
            .iter()
            .find(|a| a.name == "Payroll Liabilities")
            .unwrap()
            .account_type,
        AccountType::Liability
    );
    assert_eq!(entries.len(), 5);
    assert!(entries.iter().all(|e| e.posted));
    balanced(&entries);
    let rent = entries.iter().find(|e| e.reference.is_none()).unwrap();
    assert_eq!(rent.party.as_deref(), Some("Lumen Property Leasing"));
    assert_eq!(
        rent.lines.len(),
        2,
        "a continuation row joins its transaction"
    );
    let name = |s: &str| AccountRef::Name(s.into());
    assert_eq!(
        q3_credit_net(&entries, &name("Income:Services")),
        30_000_000
    );
    assert_eq!(-q3_credit_net(&entries, &name("Sales Returns")), 1_000_000);
    texts_clean(&chart, &entries, &[]);
    assert_map(&chart, "Income:Services", "Sales Returns", "Checking");
    assert_eq!(
        suggested(&chart, "Payroll Liabilities"),
        ("mixed".into(), 64)
    );
    assert_eq!(
        suggested(&chart, "Ask My Accountant"),
        ("unsure".into(), 30)
    );
}

#[test]
fn the_spreadsheet_sample() {
    let (chart, entries) = csv("spreadsheet");
    assert!(chart.iter().all(|a| a.number.is_none()));
    assert_eq!(
        chart
            .iter()
            .find(|a| a.name == "Supplies")
            .unwrap()
            .account_type,
        AccountType::Expense,
        "a row with no type takes one from its name"
    );
    assert_eq!(entries.len(), 6);
    balanced(&entries);
    let supplies = entries
        .iter()
        .find(|e| e.memo.as_deref() == Some("Paper and ink"))
        .unwrap();
    assert_eq!(supplies.lines[0].debit, 125_050);
    let name = |s: &str| AccountRef::Name(s.into());
    assert_eq!(q3_credit_net(&entries, &name("Sales")), 30_000_000);
    assert_eq!(-q3_credit_net(&entries, &name("Returns")), 1_000_000);
    texts_clean(&chart, &entries, &[]);
    assert_map(&chart, "Sales", "Returns", "Cash in bank");
}

#[test]
fn every_line_account_is_in_its_chart() {
    let plan = web();
    for e in &plan.entries {
        for l in &e.lines {
            let AccountRef::Number(n) = &l.account else {
                panic!("by number")
            };
            assert!(
                plan.accounts.iter().any(|a| a.number.as_ref() == Some(n)),
                "{n}"
            );
        }
    }
    for prefix in ["quickbooks", "spreadsheet"] {
        let (chart, entries) = csv(prefix);
        for l in entries.iter().flat_map(|e| &e.lines) {
            let AccountRef::Name(n) = &l.account else {
                panic!("by name")
            };
            assert!(chart.iter().any(|a| &a.name == n), "{prefix}: {n}");
        }
    }
}
