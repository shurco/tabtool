//! Port of `CSVDocumentRenamingTests.swift`.
//!
//! Column renaming delegates to [`CsvTable::rename_header`] and is one named
//! undo step. Without headers, renaming turns header mode on with generated
//! names (A, B, C…) and keeps every row; renaming to the generated name is a
//! no-op. Sort and column identities survive the rename.
//!
//! Two Swift cases are waived: `selectedColumnRenameNeedsExactlyOneColumn` and
//! `renameSelectedColumnAsksTheGridToOpenTheEditor` are UI predicates and the
//! grid's inline editor. The Rust document takes an explicit column index and
//! has no field editor (the Task 10 parity waiver).

use std::collections::BTreeSet;
use std::sync::Arc;

use tabtool_core::{CsvConfiguration, CsvTable, Locale, SortDirection};
use tabtool_document::{Document, GridSelection, UndoAction};

fn locale(id: &str) -> Locale {
    id.parse().expect("test locale is valid")
}

fn loaded(csv: &str, header: bool) -> Document {
    let configuration = CsvConfiguration {
        first_row_as_header: header,
        ..CsvConfiguration::default()
    };
    let mut document = Document::new(configuration.clone());
    document
        .load_from_bytes(csv.as_bytes(), None, Some(configuration), false)
        .expect("load the fixture");
    document
}

fn headers(document: &Document) -> Option<Vec<String>> {
    document.table().expect("table").headers()
}

fn column_ids(document: &Document) -> Vec<u64> {
    document
        .table()
        .expect("table")
        .columns
        .iter()
        .map(|column| column.id)
        .collect()
}

fn column(document: &Document, index: usize) -> Vec<String> {
    document.table().expect("table").columns[index]
        .cells
        .clone()
}

fn table(document: &Document) -> Option<Arc<CsvTable>> {
    document.table().cloned()
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn rename_is_one_named_undoable_step() {
    let mut document = loaded("a,b\n1,2\n", true);
    document.rename_column(1, "x");
    assert_eq!(headers(&document), Some(strings(&["a", "x"])));
    assert_eq!(document.undo_action(), Some(UndoAction::RenameColumn));

    assert!(document.undo());
    assert_eq!(headers(&document), Some(strings(&["a", "b"])));
    assert!(document.redo());
    assert_eq!(headers(&document), Some(strings(&["a", "x"])));
}

#[test]
fn rename_keeps_the_sort_and_column_identity() {
    let mut document = loaded("k,v\nb,1\na,2\n", true);
    document.sort_rows(0, SortDirection::Ascending, &locale("en-US"));
    let sort = document.active_sort().cloned();
    let ids = column_ids(&document);

    document.rename_column(0, "key");
    assert_eq!(document.active_sort().cloned(), sort);
    assert_eq!(column_ids(&document), ids);
    assert_eq!(column(&document, 0), strings(&["a", "b"]));

    assert!(document.undo());
    assert_eq!(document.active_sort().cloned(), sort);
}

#[test]
fn rename_to_the_same_name_records_nothing() {
    let mut document = loaded("a,b\n1,2\n", true);
    document.rename_column(0, "a");
    assert!(!document.can_undo());
}

#[test]
fn empty_name_is_allowed() {
    let mut document = loaded("a,b\n1,2\n", true);
    document.rename_column(0, "");
    assert_eq!(headers(&document), Some(strings(&["", "b"])));
}

#[test]
fn rename_without_header_turns_header_on_and_keeps_rows() {
    let mut document = loaded("a,b\n1,2\n", false);
    document.selection_mut().columns = BTreeSet::from([0]);
    assert!(document.can_rename_column(1));

    document.rename_column(1, "x");
    assert_eq!(headers(&document), Some(strings(&["A", "x"])));
    assert_eq!(document.table().expect("table").row_count(), 2);
    assert!(document.configuration().first_row_as_header);
    assert_eq!(document.undo_action(), Some(UndoAction::RenameColumn));

    assert!(document.undo());
    assert_eq!(headers(&document), None);
    assert_eq!(document.table().expect("table").row_count(), 2);
    assert!(!document.configuration().first_row_as_header);
}

#[test]
fn rename_without_header_exports_header_line_first() {
    let mut document = loaded("a,b\n1,2\n", false);
    document.rename_column(1, "x");

    assert_eq!(
        document.encoded_data(),
        Ok(b"\"A\",\"x\"\n\"a\",\"b\"\n\"1\",\"2\"\n".to_vec())
    );
}

#[test]
fn rename_without_header_keeps_the_sort() {
    let mut document = loaded("b,1\na,2\n", false);
    document.sort_rows(0, SortDirection::Ascending, &locale("en-US"));
    let sort = document.active_sort().cloned();

    document.rename_column(0, "key");
    assert_eq!(document.active_sort().cloned(), sort);
    assert_eq!(column(&document, 0), strings(&["a", "b"]));
}

#[test]
fn rename_without_header_to_generated_name_records_nothing() {
    let mut document = loaded("a,b\n1,2\n", false);
    document.rename_column(0, "A");
    assert!(!document.can_undo());
    assert_eq!(headers(&document), None);
}

#[test]
fn undo_and_redo_restore_header_mode_rows_and_generated_names() {
    let mut document = loaded("a,b\n1,2\n", false);
    let original = table(&document);
    document.rename_column(1, "x");
    let renamed = table(&document);

    assert!(document.undo());
    assert_eq!(table(&document), original);
    assert_eq!(headers(&document), None);
    assert!(!document.configuration().first_row_as_header);

    assert!(document.redo());
    assert_eq!(table(&document), renamed);
    assert_eq!(headers(&document), Some(strings(&["A", "x"])));
    assert!(document.configuration().first_row_as_header);
}

#[test]
fn undo_restores_sort_and_inserted_rows_and_columns() {
    let mut document = loaded("b,1\na,2\n", false);
    let original = table(&document);
    document.sort_rows(0, SortDirection::Ascending, &locale("en-US"));

    assert!(document.undo());
    assert_eq!(table(&document), original);
    assert_eq!(document.active_sort(), None);

    assert!(document.redo());
    assert!(document.active_sort().is_some());

    document.add_row_below_selection();
    document.add_column_right_of_selection();
    assert_eq!(document.table().expect("table").columns.len(), 3);

    assert!(document.undo());
    assert_eq!(document.table().expect("table").columns.len(), 2);
    assert!(document.undo());
    assert_eq!(document.table().expect("table").row_count(), 2);
}

#[test]
fn renaming_keeps_the_selection() {
    let mut document = loaded("a,b\n1,2\n", true);
    document.selection_mut().columns = BTreeSet::from([1]);
    document.rename_column(1, "x");
    assert_eq!(document.selection().columns, BTreeSet::from([1]));
}

#[test]
fn an_invalid_column_is_a_no_op() {
    let mut document = loaded("a,b\n1,2\n", true);
    assert!(!document.can_rename_column(2));
    document.rename_column(2, "x");
    assert!(!document.can_undo());
    assert_eq!(headers(&document), Some(strings(&["a", "b"])));
}

#[test]
fn renaming_is_refused_without_a_loaded_table() {
    let mut document = Document::new(CsvConfiguration::default());
    assert!(!document.can_rename_column(0));
    document.rename_column(0, "x");
    assert_eq!(document.table().expect("table").headers(), None);
}

#[test]
fn a_rename_can_be_undone_after_a_column_move() {
    let mut document = loaded("a,b,c\n1,2,3\n", true);
    document.rename_column(2, "z");
    document.selection_mut().columns = BTreeSet::from([2]);
    document.move_selected_columns(-1);

    assert_eq!(headers(&document), Some(strings(&["a", "z", "b"])));
    assert!(document.undo());
    assert_eq!(headers(&document), Some(strings(&["a", "b", "z"])));
    assert!(document.undo());
    assert_eq!(headers(&document), Some(strings(&["a", "b", "c"])));
}

#[test]
fn a_header_off_rename_row_is_selected_as_a_whole_row() {
    let mut document = loaded("a,b\n1,2\n", false);
    document.selection_mut().rows = BTreeSet::from([1]);
    document.rename_column(0, "key");
    assert_eq!(
        document.selection().rows,
        BTreeSet::from([1]),
        "renaming keeps a row selection"
    );
    assert_eq!(
        document.selection(),
        &GridSelection {
            rows: BTreeSet::from([1]),
            columns: BTreeSet::new(),
        }
    );
}
