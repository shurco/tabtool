//! Undo and redo behavior of the document: one step per action, targeted cell
//! undo, snapshot restore for structural changes, and redo invalidation.

use std::collections::BTreeSet;

use tabtool_core::CsvConfiguration;
use tabtool_document::{Document, GridSelection, UndoAction};

fn loaded(csv: &str) -> Document {
    let configuration = CsvConfiguration::default();
    let mut document = Document::new(configuration.clone());
    document
        .load_from_bytes(csv.as_bytes(), None, Some(configuration), false)
        .expect("load the fixture");
    document
}

fn loaded_with_header(csv: &str) -> Document {
    let configuration = CsvConfiguration {
        first_row_as_header: true,
        ..CsvConfiguration::default()
    };
    let mut document = Document::new(configuration.clone());
    document
        .load_from_bytes(csv.as_bytes(), None, Some(configuration), false)
        .expect("load the fixture");
    document
}

fn select_rows(document: &mut Document, rows: impl IntoIterator<Item = usize>) {
    *document.selection_mut() = GridSelection {
        rows: rows.into_iter().collect(),
        columns: BTreeSet::new(),
    };
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn column(document: &Document, index: usize) -> Vec<String> {
    document.table().expect("table").columns[index]
        .cells
        .clone()
}

#[test]
fn undo_without_history_returns_false() {
    let mut document = loaded("a\n");

    assert!(!document.can_undo());
    assert!(!document.can_redo());
    assert!(!document.undo());
    assert!(!document.redo());
}

#[test]
fn starting_a_new_document_clears_the_history() {
    let mut document = loaded("a\n");
    document.set_cell(0, 0, "x");
    assert!(document.can_undo());

    document
        .new_document(CsvConfiguration::default())
        .expect("replace with a new document");

    assert!(!document.can_undo());
    assert!(!document.can_redo());
}

#[test]
fn cell_undo_and_redo_restore_the_text() {
    let mut document = loaded("a,b\n1,2\n");
    document.set_cell(1, 1, "42");

    assert!(document.undo());
    assert_eq!(document.table().expect("table").cell(1, 1), "2");
    assert!(!document.can_undo());
    assert!(document.can_redo());

    assert!(document.redo());
    assert_eq!(document.table().expect("table").cell(1, 1), "42");
    assert!(document.can_undo());
    assert!(!document.can_redo());
}

#[test]
fn a_snapshot_taken_after_undo_sees_the_previous_text() {
    let mut document = loaded("a,b\n1,2\n");

    document.set_cell(1, 1, "42");
    assert!(document.undo());

    assert_eq!(document.snapshot().expect("snapshot").table.cell(1, 1), "2");
}

#[test]
fn undo_and_redo_bump_the_revision_and_keep_the_document_dirty() {
    let mut document = loaded("a\n");
    let clean = document.revision();

    document.set_cell(0, 0, "x");
    assert_eq!(document.revision(), clean + 1);

    assert!(document.undo());
    assert_eq!(document.revision(), clean + 2);
    assert!(document.is_dirty());

    assert!(document.redo());
    assert_eq!(document.revision(), clean + 3);
    assert!(document.is_dirty());
}

#[test]
fn a_new_action_clears_redo() {
    let mut document = loaded("a\n");
    document.set_cell(0, 0, "x");
    assert!(document.undo());
    assert!(document.can_redo());

    document.set_cell(0, 0, "y");

    assert!(!document.can_redo());
    assert!(!document.redo());
}

#[test]
fn a_no_op_does_not_clear_redo() {
    let mut document = loaded("a\n");
    document.set_cell(0, 0, "x");
    assert!(document.undo());
    let revision = document.revision();

    document.set_cell(0, 0, "a");

    assert_eq!(document.revision(), revision);
    assert!(document.can_redo());
    assert!(document.redo());
    assert_eq!(document.table().expect("table").cell(0, 0), "x");
}

#[test]
fn a_selection_change_is_not_an_undo_step_and_keeps_redo() {
    let mut document = loaded("a\nb\n");
    document.set_cell(0, 0, "x");
    assert!(document.undo());
    let revision = document.revision();

    select_rows(&mut document, [1]);

    assert_eq!(document.revision(), revision);
    assert!(!document.can_undo());
    assert!(document.can_redo());
    assert_eq!(document.selection().rows, BTreeSet::from([1]));
}

#[test]
fn undo_restores_the_selection_of_an_insert() {
    let mut document = loaded("a\nb\n");

    document.add_row_below_selection();
    assert_eq!(document.table().expect("table").row_count(), 3);
    assert_eq!(document.selection().rows, BTreeSet::from([2]));

    assert!(document.undo());
    assert_eq!(document.table().expect("table").row_count(), 2);
    assert_eq!(document.selection(), &GridSelection::default());

    assert!(document.redo());
    assert_eq!(document.table().expect("table").row_count(), 3);
    assert_eq!(document.selection().rows, BTreeSet::from([2]));
}

#[test]
fn undo_of_a_delete_restores_the_table_and_the_selection() {
    let mut document = loaded("a\nb\n");
    select_rows(&mut document, [0]);

    document.delete_selection();
    assert_eq!(column(&document, 0), strings(&["b"]));
    assert_eq!(document.selection(), &GridSelection::default());

    assert!(document.undo());
    assert_eq!(column(&document, 0), strings(&["a", "b"]));
    assert_eq!(document.selection().rows, BTreeSet::from([0]));

    assert!(document.redo());
    assert_eq!(column(&document, 0), strings(&["b"]));
    assert_eq!(document.selection(), &GridSelection::default());
}

#[test]
fn undo_of_a_column_move_restores_the_column_order() {
    let mut document = loaded_with_header("A,B,C\n1,2,3\n");
    *document.selection_mut() = GridSelection {
        rows: BTreeSet::new(),
        columns: BTreeSet::from([0]),
    };

    document.move_selected_columns(1);
    assert_eq!(
        document.table().expect("table").headers(),
        Some(strings(&["B", "A", "C"]))
    );

    assert!(document.undo());
    assert_eq!(
        document.table().expect("table").headers(),
        Some(strings(&["A", "B", "C"]))
    );

    assert!(document.redo());
    assert_eq!(
        document.table().expect("table").headers(),
        Some(strings(&["B", "A", "C"]))
    );
}

#[test]
fn multiple_actions_undo_in_reverse_order() {
    let mut document = loaded("a\nb\n");
    document.set_cell(0, 0, "x");
    document.add_row_below_selection();
    assert_eq!(document.table().expect("table").row_count(), 3);

    assert!(document.undo());
    assert_eq!(document.table().expect("table").row_count(), 2);
    assert_eq!(document.table().expect("table").cell(0, 0), "x");
    assert_eq!(document.selection(), &GridSelection::default());

    assert!(document.undo());
    assert_eq!(document.table().expect("table").cell(0, 0), "a");
    assert!(!document.can_undo());

    assert!(document.redo());
    assert_eq!(document.table().expect("table").cell(0, 0), "x");
    assert!(document.redo());
    assert_eq!(document.table().expect("table").row_count(), 3);
}

#[test]
fn undoing_a_cell_edit_keeps_the_selection() {
    let mut document = loaded("a\nb\n");
    select_rows(&mut document, [1]);

    document.set_cell(1, 0, "x");
    assert!(document.undo());

    assert_eq!(document.selection().rows, BTreeSet::from([1]));
}

#[test]
fn undo_action_keys_are_stable_english_identifiers() {
    assert_eq!(UndoAction::EditCell.key(), "editCell");
    assert_eq!(UndoAction::AddRow.key(), "addRow");
    assert_eq!(UndoAction::AddColumn.key(), "addColumn");
    assert_eq!(UndoAction::DeleteRows.key(), "deleteRows");
    assert_eq!(UndoAction::DeleteColumns.key(), "deleteColumns");
    assert_eq!(UndoAction::MoveRows.key(), "moveRows");
    assert_eq!(UndoAction::MoveColumn.key(), "moveColumn");
    assert_eq!(UndoAction::PasteData.key(), "pasteData");
    assert_eq!(UndoAction::ChangeFormat.key(), "changeFormat");
    assert_eq!(UndoAction::ToggleHeader.key(), "toggleHeader");
    assert_eq!(UndoAction::RenameColumn.key(), "renameColumn");
    assert_eq!(UndoAction::SortRows.key(), "sortRows");
    assert_eq!(UndoAction::RemoveSort.key(), "removeSort");
    assert_eq!(UndoAction::RevertToSaved.key(), "revertToSaved");
}
