//! What the Bot's assistant reads of a screen is button labels and field
//! names, and no person's or business's data (the Bot's
//! `app/src/agent_hub/filter.rs`, owner decision O1). A button whose
//! accessibility label differs from its text goes out unnamed, so a button
//! that shows data carries fixed words as its label. These tests pin each
//! of those labels: a redesign that drops one brings the name back, and
//! nothing else would say so. Accounts and categories stay named on purpose
//! in pickers and in the entry drawer's chips: the assistant picks them to
//! fill a draft.

const KIT: &str = include_str!("../apps/books/kit.tsx");
const SHELL: &str = include_str!("../apps/cards/shell.tsx");
const ACTION: &str = include_str!("../apps/cards/action.tsx");
const UNPAID: &str = include_str!("../apps/books/unpaid.tsx");
const FORMS: &str = include_str!("../apps/books/forms.tsx");
const OLD_BOOKS: &str = include_str!("../apps/books/oldbooks.tsx");
const PAYROLL: &str = include_str!("../apps/books/payroll.tsx");
const CERTIFICATES: &str = include_str!("../apps/books/certificates.tsx");
const REPORTS: &str = include_str!("../apps/books/reports.tsx");

/// Every page with a picker, to find a list of people without its label.
const PAGES: [&str; 25] = [
    KIT,
    UNPAID,
    FORMS,
    OLD_BOOKS,
    PAYROLL,
    CERTIFICATES,
    REPORTS,
    include_str!("../apps/books/accounts.tsx"),
    include_str!("../apps/books/activity.tsx"),
    include_str!("../apps/books/entry.tsx"),
    include_str!("../apps/books/ledger_amend.tsx"),
    include_str!("../apps/books/ledger_bulk.tsx"),
    include_str!("../apps/books/ledger_findings.tsx"),
    include_str!("../apps/books/ledger_review.tsx"),
    include_str!("../apps/books/owed_print.tsx"),
    include_str!("../apps/books/owed_repeat.tsx"),
    include_str!("../apps/books/people.tsx"),
    include_str!("../apps/books/reconcile.tsx"),
    include_str!("../apps/books/reconcile_flags.tsx"),
    include_str!("../apps/books/settings.tsx"),
    include_str!("../apps/books/statement_read.tsx"),
    include_str!("../apps/books/statement_snapshot.tsx"),
    include_str!("../apps/books/tags.tsx"),
    include_str!("../apps/books/today.tsx"),
    include_str!("../apps/books/window.tsx"),
];

/// `first`, then `then` close after it: both on the one element.
fn on_one_element(source: &str, first: &str, then: &str) {
    let at = source
        .find(first)
        .unwrap_or_else(|| panic!("{first} is gone"));
    let rest: String = source[at..].chars().take(700).collect();
    assert!(rest.contains(then), "{then} no longer follows {first}");
}

#[test]
fn an_unpaid_row_says_show_payments_not_its_person() {
    on_one_element(UNPAID, r#"accessibilityLabel="Show payments""#, "{r().who}");
}

#[test]
fn a_filled_box_says_where_it_comes_from_not_its_amount() {
    on_one_element(
        FORMS,
        r#"accessibilityLabel="Show where this comes from""#,
        r#"{r().value_shown || ""}"#,
    );
}

#[test]
fn a_cards_chips_and_post_button_say_what_they_do_not_the_account() {
    on_one_element(
        ACTION,
        r#"said="Choose this category""#,
        "label={`${String(s().name)}",
    );
    on_one_element(
        ACTION,
        r#"said="Choose this account""#,
        "label={m().name || m().label}",
    );
    on_one_element(
        ACTION,
        "label={chosen() ? `Post as ${String(chosen().name)}`",
        r#"said={chosen() ? "Post with this category" : undefined}"#,
    );
    on_one_element(KIT, "function Chip(", "accessibilityLabel={p.said}");
    on_one_element(SHELL, "function Act(", "accessibilityLabel={p.said}");
}

#[test]
fn a_picked_file_is_plain_text_beside_a_fixed_button() {
    for (name, button) in [
        ("csvChart", r#"label={csvChart() ? "Change the chart…""#),
        (
            "csvJournal",
            r#"label={csvJournal() ? "Change the journal…""#,
        ),
    ] {
        assert!(OLD_BOOKS.contains(button), "{button} is gone");
        let label = format!("label={{{name}()");
        for line in OLD_BOOKS.lines().filter(|l| l.contains(&label)) {
            assert!(
                !line.contains(".name"),
                "a file name is back on the button: {line}"
            );
        }
    }
}

#[test]
fn pick_labels_every_option_when_asked() {
    let pick = KIT.find("function Pick(").expect("Pick is gone");
    on_one_element(
        &KIT[pick..],
        "accessibilityLabel={p.optionSaid}",
        "accessibilityValue={p.optionSaid ?",
    );
    let option = KIT[pick..]
        .find("accessibilityLabel={p.optionSaid}")
        .expect("the option's label is gone");
    let words = KIT[pick..]
        .find("{o().label || o().name}\n")
        .expect("the option's text is gone");
    assert!(option < words, "the label has to be on the option's row");
}

#[test]
fn each_list_of_people_or_tags_says_what_it_chooses() {
    for (page, label) in [
        (PAYROLL, "Choose this employee"),
        (CERTIFICATES, "Choose this customer"),
        (UNPAID, "Choose this person"),
        (REPORTS, "Choose this tag"),
    ] {
        assert!(
            page.contains(&format!(r#"optionSaid="{label}""#)),
            "{label} is gone"
        );
    }
}

/// Each `<Pick … />`.
fn picks(page: &str) -> impl Iterator<Item = &str> {
    page.match_indices("<Pick").map(|(at, _)| {
        let end = page[at..].find("/>").expect("a Pick ends");
        &page[at..at + end]
    })
}

#[test]
fn no_picker_of_people_or_tags_goes_without_a_fixed_label() {
    let mut found = 0;
    for page in PAGES {
        for pick in picks(page) {
            let names = ["people()", "employees", "customers", "vendors", "tags()"];
            if names.iter().any(|n| pick.contains(n)) {
                found += 1;
                assert!(pick.contains("optionSaid="), "no fixed label on: {pick}");
            }
        }
    }
    assert_eq!(found, 5, "a list of people came or went: pin it here");
}
