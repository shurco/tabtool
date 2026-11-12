//! Port of `CSVDocumentInsertTests.swift` and the cell-edit behavior of
//! `CSVDocumentEditing.swift`.
//!
//! The capability predicates of the Swift toolbar (`canAddRowAboveSelection`
//! and friends) are UI state and are not part of the Rust document API; the
//! commands themselves follow the same selection rules.

use std::collections::BTreeSet;

use tabtool_core::{CsvConfiguration, CsvWriteIssue};
use tabtool_document::{Document, DocumentPhase, GridSelection};

fn loaded(csv: &str) -> Document {
    let configuration = CsvConfiguration::default();
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

fn select_columns(document: &mut Document, columns: impl IntoIterator<Item = usize>) {
    *document.selection_mut() = GridSelection {
        rows: BTreeSet::new(),
        columns: columns.into_iter().collect(),
    };
}

fn column(document: &Document, index: usize) -> Vec<String> {
    document.table().expect("table").columns[index]
        .cells
        .clone()
}

#[test]
fn inserting_before_the_selection_places_the_new_row_and_column_first() {
    let mut document = loaded("a,1\nb,2\n");

    select_rows(&mut document, [1]);
    document.add_row_above_selection();
    let table = document.table().expect("table");
    assert_eq!(table.cell(1, 0), "");
    assert_eq!(table.cell(2, 0), "b");
    assert_eq!(document.selection().rows, BTreeSet::from([1]));
    assert!(document.selection().columns.is_empty());

    select_columns(&mut document, [1]);
    document.add_column_left_of_selection();
    let table = document.table().expect("table");
    assert_eq!(table.cell(0, 1), "");
    assert_eq!(table.cell(0, 2), "1");
    assert_eq!(document.selection().columns, BTreeSet::from([1]));
    assert!(document.selection().rows.is_empty());
}

#[test]
fn inserting_after_without_a_selection_appends_at_the_end() {
    let mut document = loaded("a,1\nb,2\n");

    document.add_row_below_selection();
    let table = document.table().expect("table");
    assert_eq!(table.row_count(), 3);
    assert_eq!(table.cell(2, 0), "");
    assert_eq!(table.cell(1, 0), "b");
    assert_eq!(document.selection().rows, BTreeSet::from([2]));
    assert!(document.selection().columns.is_empty());

    *document.selection_mut() = GridSelection::default();
    document.add_column_right_of_selection();
    let table = document.table().expect("table");
    assert_eq!(table.columns.len(), 3);
    assert_eq!(table.cell(0, 2), "");
    assert_eq!(table.cell(0, 1), "1");
    assert_eq!(document.selection().columns, BTreeSet::from([2]));
    assert!(document.selection().rows.is_empty());
}

#[test]
fn inserting_above_without_a_selection_inserts_at_the_top() {
    let mut document = loaded("a\nb\n");

    document.add_row_above_selection();

    assert_eq!(column(&document, 0), vec!["", "a", "b"]);
    assert_eq!(document.selection().rows, BTreeSet::from([0]));
}

#[test]
fn inserting_below_the_last_selected_row() {
    let mut document = loaded("a\nb\nc\n");

    select_rows(&mut document, [0]);
    document.add_row_below_selection();

    assert_eq!(column(&document, 0), vec!["a", "", "b", "c"]);
    assert_eq!(document.selection().rows, BTreeSet::from([1]));
}

#[test]
fn inserting_right_of_the_last_selected_column() {
    let mut document = loaded("a,b,c\n1,2,3\n");

    select_columns(&mut document, [0]);
    document.add_column_right_of_selection();

    let table = document.table().expect("table");
    assert_eq!(table.columns.len(), 4);
    assert_eq!(table.cell(0, 1), "");
    assert_eq!(table.cell(0, 2), "b");
    assert_eq!(document.selection().columns, BTreeSet::from([1]));
}

#[test]
fn a_new_row_replaces_the_selection_on_the_row_axis() {
    let mut document = loaded("a\nb\n");

    select_columns(&mut document, [0]);
    document.add_row_above_selection();

    assert_eq!(document.selection().rows, BTreeSet::from([0]));
    assert!(document.selection().columns.is_empty());
}

#[test]
fn a_new_column_replaces_the_selection_on_the_column_axis() {
    let mut document = loaded("a,b\n1,2\n");

    select_rows(&mut document, [0]);
    document.add_column_left_of_selection();

    assert_eq!(document.selection().columns, BTreeSet::from([0]));
    assert!(document.selection().rows.is_empty());
}

#[test]
fn inserting_a_column_in_header_mode_gives_it_an_empty_header() {
    let configuration = CsvConfiguration {
        first_row_as_header: true,
        ..CsvConfiguration::default()
    };
    let mut document = Document::new(configuration.clone());
    document
        .load_from_bytes(b"A,B\n1,2\n", None, Some(configuration), false)
        .expect("load the fixture");

    document.add_column_right_of_selection();

    let table = document.table().expect("table");
    assert_eq!(
        table.headers(),
        Some(vec!["A".to_string(), "B".to_string(), String::new()])
    );
    assert_eq!(table.row_count(), 1);
    assert_eq!(table.cell(0, 2), "");
}

#[test]
fn set_cell_replaces_the_text_and_bumps_the_revision() {
    let mut document = loaded("a,b\n1,2\n");
    let revision = document.revision();
    assert!(!document.is_dirty());

    document.set_cell(1, 1, "42");

    assert_eq!(document.table().expect("table").cell(1, 1), "42");
    assert_eq!(document.revision(), revision + 1);
    assert!(document.is_dirty());
    assert!(document.can_undo());
}

#[test]
fn a_snapshot_taken_after_a_cell_edit_sees_the_change() {
    let mut document = loaded("a,b\n1,2\n");

    document.set_cell(1, 1, "42");

    assert_eq!(
        document.snapshot().expect("snapshot").table.cell(1, 1),
        "42"
    );
}

#[test]
fn setting_the_same_text_records_no_undo_step() {
    let mut document = loaded("a,b\n1,2\n");
    let revision = document.revision();

    document.set_cell(1, 1, "2");

    assert_eq!(document.revision(), revision);
    assert!(!document.can_undo());
}

#[test]
fn setting_a_cell_outside_the_table_records_no_undo_step() {
    let mut document = loaded("a,b\n1,2\n");
    let revision = document.revision();

    document.set_cell(9, 0, "x");
    document.set_cell(0, 9, "x");

    assert_eq!(document.revision(), revision);
    assert!(!document.can_undo());
}

#[test]
fn a_cell_edit_recomputes_the_write_issue() {
    let configuration = CsvConfiguration {
        quote_character: String::new(),
        ..CsvConfiguration::default()
    };
    let mut document = Document::new(configuration.clone());
    document
        .load_from_bytes(b"a\nb\n", None, Some(configuration), false)
        .expect("load the fixture");
    assert_eq!(document.phase(), &DocumentPhase::Ready);

    document.set_cell(0, 0, "x,y");

    assert_eq!(document.write_issue(), Some(CsvWriteIssue::AmbiguousOutput));
    assert_eq!(
        document.phase(),
        &DocumentPhase::NeedsAttention(CsvWriteIssue::AmbiguousOutput)
    );

    assert!(document.undo());
    assert_eq!(document.write_issue(), None);
    assert_eq!(document.phase(), &DocumentPhase::Ready);
    assert_eq!(document.table().expect("table").cell(0, 0), "a");
}

#[test]
fn editing_a_document_without_a_table_is_a_no_op() {
    let mut document = Document::new(CsvConfiguration::default());
    assert!(
        document
            .load_from_bytes(
                &[0xFF, 0xFD],
                None,
                Some(CsvConfiguration::default()),
                false
            )
            .is_err()
    );
    assert!(document.table().is_none());
    let revision = document.revision();

    document.set_cell(0, 0, "x");
    document.add_row_above_selection();
    document.add_row_below_selection();
    document.add_column_left_of_selection();
    document.add_column_right_of_selection();
    document.delete_selection();
    document.delete_rows(&BTreeSet::from([0]));
    document.delete_columns(&BTreeSet::from([0]));
    document.move_rows(&BTreeSet::from([0]), 1);
    document.move_columns(&BTreeSet::from([0]), 1);
    document.move_selected_rows(1);
    document.move_selected_columns(1);

    assert_eq!(document.revision(), revision);
    assert!(!document.can_undo());
    assert!(!document.undo());
    assert!(!document.redo());
    assert!(document.table().is_none());
}
