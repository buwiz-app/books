//! PR #37 regression checks on the Books window's bundled source (apps::find
//! "books" concatenates apps/books/*.tsx). Each test names the review finding
//! it covers; a fix changes the source the app serves, so these fail on the
//! un-fixed tree.

use books_mini_app::apps;
use books_mini_app::apps::AppSource;

fn books() -> &'static str {
    match &apps::find("books").unwrap().source {
        AppSource::Tsx(source) => source,
        AppSource::Native { .. } => panic!("the Books window is TSX"),
    }
}

/// Finding 1: a document that fails to link stays in the Documents library
/// unstaged, so the next form never shows another entry's attachment.
#[test]
fn f1_failed_links_leave_staged() {
    let s = books();
    assert!(
        s.contains("entryStaged.set([])"),
        "failed documents still survive a save as staged"
    );
    assert!(
        !s.contains("entryStaged.set(failed)"),
        "failed documents are kept staged for the next form"
    );
    assert!(
        s.contains("still in Documents"),
        "the toast never says where a failed document went"
    );
}

/// Finding 2: standalone SVG data URIs need the svg namespace to render.
#[test]
fn f2_side_icons_carry_the_svg_namespace() {
    let s = books();
    assert!(s.contains("xmlns="), "sideSvg emits no xmlns");
    assert!(
        !s.contains("http://"),
        "no URL literal may enter the bundle"
    );
}

/// Finding 3: a catalog kind fixes the account type, so the create call
/// omits `type` and the server derives it — whatever accounts exist.
#[test]
fn f3_catalog_kind_omits_type() {
    let s = books();
    assert!(
        s.contains("{ number, name, kind: ncKind() }") && s.contains("args.type = ncType()"),
        "the new-category form still sends a copyable type"
    );
    assert!(
        s.contains("kind: newCategoryKind() }") && s.contains("args.type = newCategoryType()"),
        "the suggestion create still sends a copyable type"
    );
}

/// Finding 4: the clipboard fallback must not claim delivery; a file-only
/// message must say the files stayed behind instead of "Sent to the Bot chat".
#[test]
fn f4_copy_path_reports_what_it_did() {
    let s = books();
    assert!(
        !s.contains("Sent to the Bot chat"),
        "the copy fallback still claims delivery"
    );
}

/// Finding 5: the remove control on a staged attachment is not hover-only.
#[test]
fn f5_remove_attachment_not_hover_only() {
    let s = books();
    // other rows keep their hover reveal; the staged-attachment remove control
    // must be visible without a pointer
    assert!(
        s.contains("opacity-60 group-hover:opacity-100"),
        "the remove control is hidden until hover"
    );
}

/// Finding 6: a second save edits the draft it already created, and the
/// remembered ticket clears on post, close and reset — never on plain Save.
#[test]
fn f6_second_save_edits_the_draft() {
    let s = books();
    assert!(s.contains("savedEntry"), "no remembered draft ticket");
    assert!(
        s.contains("base.entry = savedEntry()"),
        "save never sends the remembered ticket"
    );
    assert!(
        s.contains("savedEntry.set(post || close || fresh"),
        "the ticket survives post, close or a form reset"
    );
}

/// Finding 7: a picked payee sends its person ticket so allocations show on
/// the person's activity.
#[test]
fn f7_payee_sends_person_ticket() {
    let s = books();
    assert!(s.contains("base.person"), "the payee ticket is never sent");
}

/// Finding 8: a suggestion's kind is picked from a catalog id at the
/// source, and every built-in suggestion's kind parses — unsure is a
/// deliberate choice up front, never a caught refusal.
#[test]
fn f8_suggested_kinds_parse() {
    let s = books();
    assert!(
        s.contains("suggestion?.kind"),
        "suggestions never carry a catalog kind id"
    );
    assert!(
        !s.contains("/kind/i.test"),
        "the create still retries on an error-text match"
    );
    let suggestions: serde_json::Value =
        serde_json::from_str(include_str!("../apps/books/suggested-categories.json"))
            .expect("suggested-categories.json is not JSON");
    for suggestion in suggestions.as_array().expect("suggestions is not a list") {
        let kind = suggestion["kind"]
            .as_str()
            .expect("a suggestion has no kind");
        assert!(
            books_core::ledger::KindRef::parse(kind).is_some(),
            "suggestion kind {kind} is not a catalog kind id"
        );
    }
}

/// Finding 9: journal and transfer party pickers see every people kind.
#[test]
fn f9_journal_pickers_list_all_people() {
    let s = books();
    assert!(
        s.contains("\"employees\""),
        "people kinds other than vendors/customers are never fetched"
    );
}

/// Finding 10: the balance check gates Post, not Save — drafts may be
/// unbalanced.
#[test]
fn f10_unbalanced_journal_saves_as_draft() {
    let s = books();
    assert!(
        s.contains("args(post"),
        "args never learns whether it is saving or posting"
    );
    assert!(
        s.contains("balanced && post") || s.contains("post &&"),
        "the balance check is not gated on posting"
    );
}

/// Finding 11: the new-entry page offers a Cancel that closes it.
#[test]
fn f11_new_entry_offers_cancel() {
    let s = books();
    assert!(s.contains("Cancel"), "no Cancel action");
}

/// Cancel drops the documents staged for the entry, so the next entry's Save
/// never links them.
#[test]
fn cancel_drops_staged_documents() {
    let s = books();
    assert!(
        s.contains("entryStaged.set([]);\np.close();"),
        "Cancel leaves the staged documents for the next entry"
    );
}

/// A message with only files attached still goes to the assistant, files and all.
#[test]
fn file_only_messages_stay_in_the_side_history() {
    let s = books();
    assert!(
        s.contains(r#"(m.text || (m.role === "you" && m.chatFiles?.length))"#),
        "sideHistory drops a message that has files but no text"
    );
}

/// A category's row opens the ledger on the month the detail is showing.
#[test]
fn category_rows_open_the_ledger_on_their_month() {
    let s = books();
    assert!(
        s.contains("openLedger({ account: catDetailTicket(), month: month() ||"),
        "a category row opens the ledger without its month"
    );
}
