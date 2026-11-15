//! Ports of the Apple `CSVReorderTests` suite, plus the Review Focus cases for
//! moving rows above, inside and below the selection.

use std::collections::BTreeSet;

use tabtool_core::{CsvTable, is_permutation, shifted};

fn rows(values: &[&[&str]]) -> Vec<Vec<String>> {
    values
        .iter()
        .map(|row| row.iter().map(|cell| (*cell).to_owned()).collect())
        .collect()
}

fn indexes(values: &[usize]) -> BTreeSet<usize> {
    values.iter().copied().collect()
}

fn single_column(values: &[&str]) -> CsvTable {
    let rows = values
        .iter()
        .map(|value| vec![(*value).to_owned()])
        .collect();
    CsvTable::from_rows(rows, None, 1)
}

#[test]
fn shifted_moves_each_selected_index_and_keeps_gaps() {
    assert_eq!(shifted(5, &indexes(&[1, 3]), -1), Some(vec![1, 0, 3, 2, 4]));
    assert_eq!(shifted(5, &indexes(&[1, 2]), 1), Some(vec![0, 3, 1, 2, 4]));
}

#[test]
fn shifted_rejects_edges_empty_and_out_of_range() {
    assert_eq!(shifted(3, &indexes(&[0, 2]), -1), None);
    assert_eq!(shifted(3, &indexes(&[2]), 1), None);
    assert_eq!(shifted(3, &indexes(&[]), 1), None);
    assert_eq!(shifted(3, &indexes(&[5]), -1), None);
    assert_eq!(shifted(3, &indexes(&[5]), -3), None);
}

#[test]
fn reorder_rows_applies_a_permutation_to_every_column() {
    let mut table = CsvTable::from_rows(
        rows(&[&["0a", "0b"], &["1a", "1b"], &["2a", "2b"]]),
        Some(vec!["A".to_owned(), "B".to_owned()]),
        1,
    );
    table.reorder_rows(&[2, 0, 1]);
    assert_eq!(table.columns[0].cells, vec!["2a", "0a", "1a"]);
    assert_eq!(table.columns[1].cells, vec!["2b", "0b", "1b"]);
    assert_eq!(table.headers(), Some(vec!["A".to_owned(), "B".to_owned()]));
}

#[test]
fn reorder_ignores_non_permutations() {
    let mut table = single_column(&["0", "1"]);
    table.reorder_rows(&[0]);
    table.reorder_rows(&[1, 1]);
    table.reorder_rows(&[0, 2]);
    assert_eq!(table.columns[0].cells, vec!["0", "1"]);

    let mut columns = CsvTable::from_rows(rows(&[&["a", "b", "c"]]), None, 1);
    let ids: Vec<_> = columns.columns.iter().map(|column| column.id).collect();
    columns.reorder_columns(&[2, 0, 1]);
    assert_eq!(
        columns
            .columns
            .iter()
            .map(|column| column.id)
            .collect::<Vec<_>>(),
        vec![ids[2], ids[0], ids[1]]
    );
    columns.reorder_columns(&[0, 0, 1]);
    assert_eq!(
        columns
            .columns
            .iter()
            .map(|column| column.id)
            .collect::<Vec<_>>(),
        vec![ids[2], ids[0], ids[1]]
    );
}

#[test]
fn move_rows_places_the_selection_before_a_destination_above_it() {
    let mut table = single_column(&["0", "1", "2", "3", "4"]);
    table.move_rows(&indexes(&[3, 4]), 0);
    assert_eq!(table.columns[0].cells, vec!["3", "4", "0", "1", "2"]);
}

#[test]
fn move_rows_places_the_selection_before_a_destination_inside_it() {
    let mut table = single_column(&["0", "1", "2", "3", "4"]);
    table.move_rows(&indexes(&[1, 3]), 3);
    assert_eq!(table.columns[0].cells, vec!["0", "2", "1", "3", "4"]);
}

#[test]
fn move_rows_places_the_selection_before_a_destination_below_it() {
    let mut table = single_column(&["0", "1", "2", "3", "4"]);
    table.move_rows(&indexes(&[0, 2]), 5);
    assert_eq!(table.columns[0].cells, vec!["1", "3", "4", "0", "2"]);
}

#[test]
fn move_rows_clamps_the_destination_and_ignores_empty_selections() {
    let mut table = single_column(&["0", "1", "2"]);
    table.move_rows(&indexes(&[0]), 99);
    assert_eq!(table.columns[0].cells, vec!["1", "2", "0"]);

    let before = table.clone();
    table.move_rows(&indexes(&[]), 2);
    table.move_rows(&indexes(&[7]), 0);
    assert_eq!(table, before);
}

#[test]
fn permutations_require_each_index_exactly_once() {
    assert!(is_permutation(&[], 0));
    assert!(is_permutation(&[0, 1, 2], 3));
    assert!(!is_permutation(&[0, 1], 3));
    assert!(!is_permutation(&[0, 1, 1], 3));
    assert!(!is_permutation(&[0, 1, 3], 3));
}
