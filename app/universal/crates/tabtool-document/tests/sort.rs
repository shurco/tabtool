//! Port of `CSVDocumentSortingTests.swift` plus the document-level locale and
//! column-move cases.
//!
//! The Swift field-editor session (`sortCommitsTheFieldEditorFirst`) and the
//! `canSortSelection` toolbar predicate are UI state and have no counterpart
//! here: the Rust document has no field editor, and sorting takes an explicit
//! column index, so a valid index replaces the one-column selection gate.

use std::collections::BTreeSet;

use tabtool_core::{CsvConfiguration, Locale, SortDirection};
use tabtool_document::{Document, GridSelection, UndoAction};

fn locale(id: &str) -> Locale {
    id.parse().expect("test locale is valid")
}

fn loaded(csv: &str) -> Document {
    let configuration = CsvConfiguration::default();
    let mut document = Document::new(configuration.clone());
    document
        .load_from_bytes(csv.as_bytes(), None, Some(configuration), false)
        .expect("load the fixture");
    document
}

fn column(document: &Document, index: usize) -> Vec<String> {
    document.table().expect("table").columns[index]
        .cells
        .clone()
}

fn select_rows(document: &mut Document, rows: impl IntoIterator<Item = usize>) {
    *document.selection_mut() = GridSelection {
        rows: rows.into_iter().collect(),
        columns: BTreeSet::new(),
    };
}

#[test]
fn sort_then_undo_and_redo_restore_rows_and_sort_state() {
    let mut document = loaded("b\na\nc\n");

    document.sort_rows(0, SortDirection::Ascending, &locale("en-US"));
    assert_eq!(column(&document, 0), vec!["a", "b", "c"]);
    assert_eq!(document.sort_direction(0), Some(SortDirection::Ascending));

    assert!(document.undo());
    assert_eq!(column(&document, 0), vec!["b", "a", "c"]);
    assert_eq!(document.active_sort(), None);

    assert!(document.redo());
    assert_eq!(column(&document, 0), vec!["a", "b", "c"]);
    assert_eq!(document.sort_direction(0), Some(SortDirection::Ascending));
}

#[test]
fn ascending_descending_remove_restores_source_order() {
    let mut document = loaded("b\na\nc\n");

    document.sort_rows(0, SortDirection::Ascending, &locale("en-US"));
    document.sort_rows(0, SortDirection::Descending, &locale("en-US"));
    assert_eq!(column(&document, 0), vec!["c", "b", "a"]);

    document.remove_sort();
    assert_eq!(column(&document, 0), vec!["b", "a", "c"]);
    assert_eq!(document.active_sort(), None);
}

#[test]
fn cell_edit_keeps_sort_and_remove_sort_keeps_the_edited_row() {
    let mut document = loaded("b,1\na,2\n");

    document.sort_rows(0, SortDirection::Ascending, &locale("en-US"));
    document.set_cell(0, 1, "z");
    assert!(document.active_sort().is_some());

    document.remove_sort();
    assert_eq!(column(&document, 0), vec!["b", "a"]);
    assert_eq!(column(&document, 1), vec!["1", "z"]);
}

#[test]
fn remove_sort_clears_state_when_rows_look_identical() {
    let mut document = loaded("b\na\n");

    document.sort_rows(0, SortDirection::Ascending, &locale("en-US"));
    document.set_cell(0, 0, "b");
    document.remove_sort();

    assert_eq!(document.active_sort(), None);
    assert_eq!(document.undo_action(), Some(UndoAction::EditCell));
}

#[test]
fn resorting_same_direction_after_edit_reorders_and_keeps_source() {
    let mut document = loaded("b\na\nc\n");

    document.sort_rows(0, SortDirection::Ascending, &locale("en-US"));
    document.set_cell(0, 0, "z");
    document.sort_rows(0, SortDirection::Ascending, &locale("en-US"));
    assert_eq!(column(&document, 0), vec!["b", "c", "z"]);

    document.remove_sort();
    assert_eq!(column(&document, 0), vec!["b", "z", "c"]);
}

#[test]
fn sorting_another_column_composes_and_remove_sort_restores_original() {
    let mut document = loaded("b,2\na,3\nc,1\n");

    document.sort_rows(0, SortDirection::Ascending, &locale("en-US"));
    document.sort_rows(1, SortDirection::Ascending, &locale("en-US"));
    assert_eq!(column(&document, 1), vec!["1", "2", "3"]);

    document.remove_sort();
    assert_eq!(column(&document, 0), vec!["b", "a", "c"]);
    assert_eq!(document.active_sort(), None);
}

#[test]
fn remove_sort_undo_and_redo() {
    let mut document = loaded("b\na\nc\n");

    document.sort_rows(0, SortDirection::Ascending, &locale("en-US"));
    document.remove_sort();
    assert_eq!(document.undo_action(), Some(UndoAction::SortRows));

    assert!(document.undo());
    assert_eq!(column(&document, 0), vec!["a", "b", "c"]);
    assert_eq!(document.sort_direction(0), Some(SortDirection::Ascending));

    assert!(document.redo());
    assert_eq!(column(&document, 0), vec!["b", "a", "c"]);
    assert_eq!(document.active_sort(), None);
}

#[test]
fn structural_change_clears_sort() {
    let mut document = loaded("b\na\n");
    document.sort_rows(0, SortDirection::Ascending, &locale("en-US"));
    document.add_row_above_selection();
    assert_eq!(document.active_sort(), None);

    let mut other = loaded("b\na\n");
    other.sort_rows(0, SortDirection::Ascending, &locale("en-US"));
    other.set_header_enabled(true);
    assert_eq!(other.active_sort(), None);
}

#[test]
fn already_sorted_column_sets_sort_without_undo() {
    let mut document = loaded("a\nb\n");
    let revision = document.revision();

    document.sort_rows(0, SortDirection::Ascending, &locale("en-US"));

    assert!(document.active_sort().is_some());
    assert_eq!(document.sort_direction(0), Some(SortDirection::Ascending));
    assert!(!document.can_undo());
    assert_eq!(document.revision(), revision);
}

#[test]
fn changing_direction_without_reordering_still_updates_the_arrow() {
    let mut document = loaded("a\na\n");

    document.sort_rows(0, SortDirection::Ascending, &locale("en-US"));
    assert_eq!(document.sort_direction(0), Some(SortDirection::Ascending));
    assert!(!document.can_undo());

    document.sort_rows(0, SortDirection::Descending, &locale("en-US"));
    assert_eq!(document.sort_direction(0), Some(SortDirection::Descending));
    assert!(!document.can_undo());

    document.remove_sort();
    assert_eq!(document.active_sort(), None);
    assert!(!document.can_undo());
}

#[test]
fn cycle_sort_goes_ascending_descending_then_removes() {
    let mut document = loaded("b\na\nc\n");

    document.cycle_sort(0, &locale("en-US"));
    assert_eq!(document.sort_direction(0), Some(SortDirection::Ascending));
    assert_eq!(column(&document, 0), vec!["a", "b", "c"]);

    document.cycle_sort(0, &locale("en-US"));
    assert_eq!(document.sort_direction(0), Some(SortDirection::Descending));
    assert_eq!(column(&document, 0), vec!["c", "b", "a"]);

    document.cycle_sort(0, &locale("en-US"));
    assert_eq!(document.active_sort(), None);
    assert_eq!(column(&document, 0), vec!["b", "a", "c"]);
}

#[test]
fn cycle_sort_on_another_column_starts_ascending() {
    let mut document = loaded("b,1\na,2\n");

    document.sort_rows(0, SortDirection::Descending, &locale("en-US"));
    document.cycle_sort(1, &locale("en-US"));

    assert_eq!(document.sort_direction(1), Some(SortDirection::Ascending));
    assert_eq!(document.sort_direction(0), None);
}

#[test]
fn sorting_uses_the_requested_locale() {
    let english = loaded("z\nä\n");
    let swedish = loaded("z\nä\n");
    let en = locale("en-US");
    let sv = locale("sv-SE");

    let mut english = english;
    english.sort_rows(0, SortDirection::Ascending, &en);
    let mut swedish = swedish;
    swedish.sort_rows(0, SortDirection::Ascending, &sv);

    assert_eq!(column(&english, 0), vec!["ä", "z"]);
    assert_eq!(column(&swedish, 0), vec!["z", "ä"]);
}

#[test]
fn sorting_requires_a_valid_column() {
    let mut document = loaded("b\na\n");
    let revision = document.revision();

    document.sort_rows(9, SortDirection::Ascending, &locale("en-US"));
    document.cycle_sort(9, &locale("en-US"));

    assert_eq!(document.revision(), revision);
    assert_eq!(document.active_sort(), None);
    assert!(!document.can_undo());
    assert_eq!(column(&document, 0), vec!["b", "a"]);
}

#[test]
fn sorting_without_a_table_is_a_no_op() {
    let mut document = Document::new(CsvConfiguration::default());
    let revision = document.revision();

    document.sort_rows(0, SortDirection::Ascending, &locale("en-US"));
    document.remove_sort();
    document.cycle_sort(0, &locale("en-US"));

    assert_eq!(document.revision(), revision);
    assert_eq!(document.active_sort(), None);
    assert!(!document.can_undo());
}

#[test]
fn sorting_selects_the_sorted_column() {
    let mut document = loaded("a,b\n1,2\n");
    select_rows(&mut document, [0]);

    document.sort_rows(1, SortDirection::Ascending, &locale("en-US"));

    assert_eq!(
        document.selection(),
        &GridSelection {
            rows: BTreeSet::new(),
            columns: BTreeSet::from([1]),
        }
    );
}

#[test]
fn column_moves_keep_the_sort_and_remove_sort_restores_row_order() {
    let mut document = loaded("b,2\na,1\n");

    document.sort_rows(0, SortDirection::Ascending, &locale("en-US"));
    document.move_columns(&BTreeSet::from([0]), 2);

    assert!(document.active_sort().is_some());
    assert_eq!(document.sort_direction(1), Some(SortDirection::Ascending));
    assert_eq!(document.sort_direction(0), None);

    document.remove_sort();
    assert_eq!(column(&document, 0), vec!["2", "1"]);
    assert_eq!(column(&document, 1), vec!["b", "a"]);
}
