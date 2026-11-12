//! Port of the structural cases in `CSVDocumentEditing.swift`,
//! `CSVDocumentMoving.swift`, `CSVDocumentMovingTests.swift` and the
//! delete-selection cases of `GridInteractionTests.swift`.

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

fn select_columns(document: &mut Document, columns: impl IntoIterator<Item = usize>) {
    *document.selection_mut() = GridSelection {
        rows: BTreeSet::new(),
        columns: columns.into_iter().collect(),
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

fn headers(document: &Document) -> Option<Vec<String>> {
    document.table().expect("table").headers()
}

#[test]
fn deleting_selected_rows_removes_them_and_clears_the_selection() {
    let mut document = loaded("a\nb\nc\n");
    select_rows(&mut document, [0, 2]);

    document.delete_selection();

    assert_eq!(column(&document, 0), strings(&["b"]));
    assert_eq!(document.selection(), &GridSelection::default());
    assert!(document.can_undo());
}

#[test]
fn deleting_selected_columns_removes_them_and_clears_the_selection() {
    let mut document = loaded("a,b,c\n1,2,3\n");
    select_columns(&mut document, [1]);

    document.delete_selection();

    let table = document.table().expect("table");
    assert_eq!(table.columns.len(), 2);
    assert_eq!(column(&document, 0), strings(&["a", "1"]));
    assert_eq!(column(&document, 1), strings(&["c", "3"]));
    assert_eq!(document.selection(), &GridSelection::default());
}

#[test]
fn delete_selection_prefers_rows_over_columns() {
    let mut document = loaded("a,b\nc,d\n");
    *document.selection_mut() = GridSelection {
        rows: BTreeSet::from([0]),
        columns: BTreeSet::from([1]),
    };

    document.delete_selection();

    assert_eq!(document.table().expect("table").row_count(), 1);
    assert_eq!(document.table().expect("table").columns.len(), 2);
    assert_eq!(document.selection(), &GridSelection::default());

    assert!(document.undo());
    assert_eq!(document.table().expect("table").row_count(), 2);
}

#[test]
fn deleting_every_selected_column_keeps_the_leftmost_one() {
    let mut document = loaded_with_header("h1,h2\na,b\n");

    document.delete_columns(&BTreeSet::from([0, 1]));

    let table = document.table().expect("table");
    assert_eq!(table.columns.len(), 1);
    assert_eq!(table.row_count(), 1);
    assert_eq!(headers(&document), Some(strings(&["h1"])));
    assert_eq!(table.cell(0, 0), "a");

    assert!(document.undo());
    assert_eq!(document.table().expect("table").columns.len(), 2);
    assert_eq!(headers(&document), Some(strings(&["h1", "h2"])));
}

#[test]
fn deleting_the_only_column_is_a_no_op() {
    let mut document = loaded("a\nb\n");
    let revision = document.revision();

    document.delete_columns(&BTreeSet::from([0]));

    assert_eq!(document.table().expect("table").columns.len(), 1);
    assert_eq!(column(&document, 0), strings(&["a", "b"]));
    assert_eq!(document.revision(), revision);
    assert!(!document.can_undo());
}

#[test]
fn deleting_columns_ignores_out_of_range_indexes() {
    let mut document = loaded("a,b\n1,2\n");
    let revision = document.revision();

    document.delete_columns(&BTreeSet::from([9]));

    assert_eq!(document.revision(), revision);
    assert!(!document.can_undo());

    document.delete_columns(&BTreeSet::from([0, 9]));

    assert_eq!(document.table().expect("table").columns.len(), 1);
    assert_eq!(column(&document, 0), strings(&["b", "2"]));
}

#[test]
fn deleting_rows_ignores_out_of_range_indexes() {
    let mut document = loaded("a\nb\nc\n");
    let revision = document.revision();

    document.delete_rows(&BTreeSet::from([9]));

    assert_eq!(document.revision(), revision);
    assert!(!document.can_undo());

    document.delete_rows(&BTreeSet::from([0, 9]));

    assert_eq!(column(&document, 0), strings(&["b", "c"]));
}

#[test]
fn a_structural_change_recomputes_the_write_issue() {
    let configuration = CsvConfiguration {
        quote_character: String::new(),
        ..CsvConfiguration::default()
    };
    let mut document = Document::new(configuration.clone());
    document
        .load_from_bytes(b"a\n\n", None, Some(configuration), false)
        .expect("load the fixture");
    assert_eq!(document.write_issue(), Some(CsvWriteIssue::AmbiguousOutput));

    document.delete_rows(&BTreeSet::from([1]));

    assert_eq!(document.write_issue(), None);
    assert_eq!(document.phase(), &DocumentPhase::Ready);

    assert!(document.undo());
    assert_eq!(document.write_issue(), Some(CsvWriteIssue::AmbiguousOutput));
}

#[test]
fn moving_rows_clamps_the_destination_to_the_end() {
    let mut document = loaded("a\nb\nc\n");

    document.move_rows(&BTreeSet::from([0]), 99);

    assert_eq!(column(&document, 0), strings(&["b", "c", "a"]));
    assert!(document.undo());
    assert_eq!(column(&document, 0), strings(&["a", "b", "c"]));
}

#[test]
fn moving_columns_clamps_the_destination_to_the_right_edge() {
    let mut document = loaded_with_header("A,B,C\n1,2,3\n");

    document.move_columns(&BTreeSet::from([0]), 99);

    assert_eq!(headers(&document), Some(strings(&["B", "C", "A"])));
    assert_eq!(column(&document, 0), strings(&["2"]));
    assert_eq!(column(&document, 2), strings(&["1"]));
}

#[test]
fn moving_with_an_empty_selection_records_nothing() {
    let mut document = loaded("a,b\n1,2\n");
    let revision = document.revision();

    document.move_rows(&BTreeSet::new(), 1);
    document.move_columns(&BTreeSet::new(), 1);
    document.move_selected_rows(1);
    document.move_selected_columns(-1);

    assert_eq!(document.revision(), revision);
    assert!(!document.can_undo());
}

#[test]
fn move_rows_down_keeps_gaps_and_selection() {
    let mut document = loaded("0\n1\n2\n3\n4\n");
    select_rows(&mut document, [1, 3]);

    document.move_selected_rows(1);

    assert_eq!(column(&document, 0), strings(&["0", "2", "1", "4", "3"]));
    assert_eq!(document.selection().rows, BTreeSet::from([2, 4]));
    assert!(document.selection().columns.is_empty());

    assert!(document.undo());
    assert_eq!(column(&document, 0), strings(&["0", "1", "2", "3", "4"]));
    assert_eq!(document.selection().rows, BTreeSet::from([1, 3]));
}

#[test]
fn move_columns_left_moves_headers_and_selection() {
    let mut document = loaded_with_header("A,B,C\n1,2,3\n");
    select_columns(&mut document, [1, 2]);

    document.move_selected_columns(-1);

    assert_eq!(headers(&document), Some(strings(&["B", "C", "A"])));
    assert_eq!(document.selection().columns, BTreeSet::from([0, 1]));
    assert!(document.selection().rows.is_empty());
}

#[test]
fn one_step_moves_are_disabled_at_the_edges() {
    let mut document = loaded("a,b,c\n1,2,3\n4,5,6\n");
    let revision = document.revision();

    document.move_selected_rows(1);
    document.move_selected_columns(-1);
    assert_eq!(document.revision(), revision);

    select_rows(&mut document, [0, 2]);
    document.move_selected_rows(-1);
    document.move_selected_rows(1);
    assert_eq!(document.revision(), revision);

    select_columns(&mut document, [0]);
    document.move_selected_columns(-1);
    assert_eq!(document.revision(), revision);
    assert!(!document.can_undo());

    document.move_selected_columns(1);
    assert_eq!(column(&document, 0), strings(&["b", "2", "5"]));
    assert!(document.can_undo());
}

#[test]
fn a_direct_row_move_does_not_follow_the_selection() {
    let mut document = loaded("a\nb\nc\nd\n");
    select_rows(&mut document, [0, 3]);

    document.move_rows(&BTreeSet::from([3]), 0);

    assert_eq!(column(&document, 0), strings(&["d", "a", "b", "c"]));
    assert_eq!(document.selection().rows, BTreeSet::from([0, 3]));
}
