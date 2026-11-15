//! Navigation order, sheet stacking and the missing-document notice.

use std::path::Path;

use tabtool_ui::mobile::{
    BackOutcome, DocumentId, MISSING_DOCUMENT_NOTICE, NavStack, Screen, SheetKind,
};

fn document(name: &str) -> DocumentId {
    DocumentId::new(format!("/documents/{name}"))
}

#[test]
fn a_new_stack_starts_at_the_library_and_exits_on_back() {
    let mut nav = NavStack::new();

    assert_eq!(nav.screen(), &Screen::Library);
    assert!(nav.sheets().is_empty());
    assert_eq!(nav.back(), BackOutcome::Exit);
    assert_eq!(nav.screen(), &Screen::Library);
}

#[test]
fn opening_a_document_pushes_the_editor_once() {
    let mut nav = NavStack::new();

    assert!(nav.open_document(document("Untitled.csv")));
    assert!(!nav.open_document(document("Untitled.csv")));
    assert_eq!(nav.screen(), &Screen::Editor(document("Untitled.csv")));

    assert_eq!(nav.back(), BackOutcome::ClosedDocument);
    assert_eq!(nav.screen(), &Screen::Library);
}

#[test]
fn opening_another_document_replaces_the_editor() {
    let mut nav = NavStack::new();
    nav.open_document(document("first.csv"));
    nav.open_document(document("second.csv"));

    assert_eq!(nav.screen(), &Screen::Editor(document("second.csv")));

    assert_eq!(nav.back(), BackOutcome::ClosedDocument);
    assert_eq!(nav.screen(), &Screen::Library);
}

#[test]
fn back_pops_one_sheet_at_a_time() {
    let mut nav = NavStack::new();
    nav.open_document(document("Untitled.csv"));
    nav.open_sheet(SheetKind::Settings);
    nav.open_sheet(SheetKind::About);

    assert_eq!(nav.sheet(), Some(SheetKind::About));
    assert_eq!(nav.sheets(), [SheetKind::Settings, SheetKind::About]);

    assert_eq!(nav.back(), BackOutcome::ClosedSheet);
    assert_eq!(nav.sheet(), Some(SheetKind::Settings));

    assert_eq!(nav.back(), BackOutcome::ClosedSheet);
    assert_eq!(nav.sheet(), None);

    assert_eq!(nav.back(), BackOutcome::ClosedDocument);
    assert_eq!(nav.screen(), &Screen::Library);
}

#[test]
fn opening_a_document_closes_sheets_on_top_of_the_library() {
    let mut nav = NavStack::new();
    nav.open_sheet(SheetKind::Settings);
    nav.open_sheet(SheetKind::About);

    nav.open_document(document("Untitled.csv"));

    assert!(nav.sheets().is_empty());
    assert_eq!(nav.screen(), &Screen::Editor(document("Untitled.csv")));
}

#[test]
fn closing_a_sheet_keeps_the_editor() {
    let mut nav = NavStack::new();
    nav.open_document(document("Untitled.csv"));
    nav.open_sheet(SheetKind::Format);

    assert!(nav.close_sheet());
    assert!(!nav.close_sheet());
    assert_eq!(nav.screen(), &Screen::Editor(document("Untitled.csv")));
    assert_eq!(nav.sheet(), None);
}

#[test]
fn a_missing_document_returns_to_the_library_with_a_notice() {
    let mut nav = NavStack::new();
    nav.open_document(document("gone.csv"));

    nav.document_missing();

    assert_eq!(nav.screen(), &Screen::Library);
    assert!(nav.sheets().is_empty());
    assert_eq!(nav.take_notice(), Some(MISSING_DOCUMENT_NOTICE));
    assert_eq!(nav.take_notice(), None);
}

#[test]
fn a_document_id_is_its_path() {
    let id = DocumentId::new("/documents/notes.csv");

    assert_eq!(id.path(), Path::new("/documents/notes.csv"));
    assert_eq!(id, DocumentId::from(Path::new("/documents/notes.csv")));
}
