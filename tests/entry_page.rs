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

/// Finding 1: a document that failed to link keeps the entry ticket it was
/// staged for instead of landing on the next saved entry.
#[test]
fn f1_failed_links_keep_their_entry() {
    let s = books();
    assert!(
        s.contains("d.target || r.entry"),
        "the link target ignores a document's kept ticket"
    );
    assert!(
        s.contains("target: d.target || r.entry"),
        "failed documents no longer record their entry ticket"
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

/// Finding 3: picking a kind in the new-category form syncs the account type,
/// so an expense kind cannot be saved as an asset.
#[test]
fn f3_kind_pick_syncs_account_type() {
    let s = books();
    assert!(
        s.contains("ncType.set(String(named.type))"),
        "choosing a kind never sets the type"
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

/// Finding 6: a second save edits the draft it already created.
#[test]
fn f6_second_save_edits_the_draft() {
    let s = books();
    assert!(s.contains("savedEntry"), "no remembered draft ticket");
    assert!(
        s.contains("base.entry = savedEntry()"),
        "save never sends the remembered ticket"
    );
}

/// Finding 7: a picked payee sends its person ticket so allocations show on
/// the person's activity.
#[test]
fn f7_payee_sends_person_ticket() {
    let s = books();
    assert!(s.contains("base.person"), "the payee ticket is never sent");
}

/// Finding 8: a suggested subtype that is not a catalog kind still creates
/// the account (the create retried as unsure instead of failing dead).
#[test]
fn f8_legacy_subtype_falls_back_to_unsure() {
    let s = books();
    assert!(
        s.contains("\"unsure\")"),
        "no unsure fallback on kind refusal"
    );
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
