//! Ports of the Apple `CSVTableTests`, `ColumnNameGeneratorTests` and
//! `RowSourceTests` suites, plus the Review Focus cases.
//!
//! Apple's tests use `UUID` column identities; here deterministic ids are
//! passed explicitly wherever identity is asserted and generated ids are used
//! elsewhere. `LargeFile.thresholdBytes` belongs to the large-file task.

use std::collections::BTreeSet;

use tabtool_core::{ColumnId, CsvColumn, CsvTable, RowSource, column_name, column_name_checked};

fn rows(values: &[&[&str]]) -> Vec<Vec<String>> {
    values
        .iter()
        .map(|row| row.iter().map(|cell| (*cell).to_owned()).collect())
        .collect()
}

fn headers(values: &[&str]) -> Option<Vec<String>> {
    Some(values.iter().map(|name| (*name).to_owned()).collect())
}

fn indexes(values: &[usize]) -> BTreeSet<usize> {
    values.iter().copied().collect()
}

fn column(id: ColumnId, header: Option<&str>, cells: &[&str]) -> CsvColumn {
    CsvColumn {
        id,
        header: header.map(str::to_owned),
        cells: cells.iter().map(|cell| (*cell).to_owned()).collect(),
    }
}

fn cells(table: &CsvTable) -> Vec<Vec<String>> {
    table
        .columns
        .iter()
        .map(|column| column.cells.clone())
        .collect()
}

#[test]
fn ragged_rows_are_padded_not_truncated() {
    let table = CsvTable::from_rows(
        rows(&[&["a", "b", "c"], &["d"]]),
        headers(&["h1", "h2", "h3"]),
        1,
    );
    assert_eq!(table.row_count(), 2);
    assert_eq!(table.columns.len(), 3);
    assert_eq!(table.cell(1, 2), "");
    assert_eq!(table.headers(), headers(&["h1", "h2", "h3"]));
}

#[test]
fn new_document_is_three_columns_by_one_row() {
    let table = CsvTable::new_document(1, 3);
    assert_eq!(table.row_count(), 1);
    assert_eq!(table.columns.len(), 3);
    assert_eq!(table.headers(), None);
}

#[test]
fn new_document_clamps_to_one_by_one() {
    let table = CsvTable::new_document(0, 0);
    assert_eq!(table.row_count(), 1);
    assert_eq!(table.columns.len(), 1);
    assert_eq!(table.cell(0, 0), "");
}

#[test]
fn empty_parsed_row_becomes_one_by_one() {
    // Review Focus: empty input produces a 1×1 table.
    let table = CsvTable::from_rows(vec![vec![String::new()]], None, 1);
    assert_eq!(table.row_count(), 1);
    assert_eq!(table.columns.len(), 1);
    assert_eq!(table.cell(0, 0), "");

    // A row with no fields is padded to the minimum column count.
    let padded = CsvTable::from_rows(vec![Vec::new()], None, 1);
    assert_eq!(padded.row_count(), 1);
    assert_eq!(padded.columns.len(), 1);
    assert_eq!(padded.cell(0, 0), "");
}

#[test]
fn add_delete_and_move_rows_keep_all_columns_aligned() {
    let mut table = CsvTable::from_rows(
        rows(&[&["0a", "0b"], &["1a", "1b"], &["2a", "2b"]]),
        None,
        1,
    );
    table.add_row(1);
    assert_eq!(table.row_count(), 4);
    assert_eq!(table.columns[0].cells, vec!["0a", "", "1a", "2a"]);
    assert_eq!(table.columns[1].cells, vec!["0b", "", "1b", "2b"]);
    table.delete_rows(&indexes(&[1]));
    assert_eq!(table.columns[0].cells, vec!["0a", "1a", "2a"]);
    table.move_rows(&indexes(&[0]), 3);
    assert_eq!(table.columns[0].cells, vec!["1a", "2a", "0a"]);
}

#[test]
fn add_delete_and_move_columns_preserve_headers() {
    let mut table = CsvTable::from_rows(rows(&[&["a", "b"], &["c", "d"]]), headers(&["A", "B"]), 1);
    table.add_column(0, Some("X"));
    assert_eq!(table.headers(), headers(&["X", "A", "B"]));
    assert_eq!(cells(&table), rows(&[&["", ""], &["a", "c"], &["b", "d"]]));
    table.move_column(2, 0);
    assert_eq!(table.headers(), headers(&["B", "X", "A"]));
    table.delete_columns(&indexes(&[1]));
    assert_eq!(table.headers(), headers(&["B", "A"]));
}

#[test]
fn move_non_contiguous_rows_to_end_keeps_order() {
    let mut table = CsvTable::from_rows(rows(&[&["0"], &["1"], &["2"], &["3"], &["4"]]), None, 1);
    table.move_rows(&indexes(&[1, 3]), 5);
    assert_eq!(table.columns[0].cells, vec!["0", "2", "4", "1", "3"]);
}

#[test]
fn headers_are_all_or_nothing() {
    let mut table = CsvTable::from_rows(rows(&[&["a", "b"]]), headers(&["A", "B"]), 1);
    table.set_headers(None);
    assert_eq!(table.headers(), None);
    assert!(table.columns.iter().all(|column| column.header.is_none()));
    table.set_headers(headers(&["only-one"]));
    assert_eq!(table.headers(), headers(&["only-one", ""]));
}

#[test]
fn decimal_conversion_only_touches_numeric_cells() {
    let mut table = CsvTable::from_rows(rows(&[&["1,50"], &["a,b"], &["001,00"]]), None, 1);
    table.convert_decimal_mark(",", ".");
    assert_eq!(table.columns[0].cells, vec!["1.50", "a,b", "001.00"]);

    // An empty source or unchanged mark is a no-op; an empty target strips it.
    table.convert_decimal_mark(".", ".");
    table.convert_decimal_mark("", ",");
    table.convert_decimal_mark(".", "");
    assert_eq!(table.columns[0].cells, vec!["150", "a,b", "00100"]);
}

#[test]
fn initializing_columns_pads_to_longest() {
    let table = CsvTable::new(vec![
        column(1, Some("A"), &["1"]),
        column(2, Some("B"), &["2", "3", "4"]),
    ]);
    assert_eq!(table.row_count(), 3);
    assert_eq!(cells(&table), rows(&[&["1", "", ""], &["2", "3", "4"]]));
    assert_eq!(table.headers(), headers(&["A", "B"]));
}

#[test]
fn mixed_column_headers_collapse_to_no_headers() {
    let table = CsvTable::new(vec![column(1, Some("A"), &["1"]), column(2, None, &["2"])]);
    assert_eq!(table.headers(), None);
    assert!(table.columns.iter().all(|column| column.header.is_none()));
}

#[test]
fn headers_longer_than_rows_extend_columns() {
    let table = CsvTable::from_rows(rows(&[&["a"]]), headers(&["h1", "h2"]), 1);
    assert_eq!(table.columns.len(), 2);
    assert_eq!(cells(&table), rows(&[&["a"], &[""]]));
    assert_eq!(table.headers(), headers(&["h1", "h2"]));
}

#[test]
fn invalid_indexes_are_ignored() {
    let mut table = CsvTable::from_rows(rows(&[&["a"]]), None, 1);
    // Swift passes row -1; a `usize` index can only overflow instead.
    assert_eq!(table.cell(usize::MAX, 0), "");
    assert_eq!(table.cell(0, 5), "");
    table.set_cell("x".to_owned(), 9, 0);
    table.set_cell("x".to_owned(), 0, 9);
    assert_eq!(table.cell(0, 0), "a");
    table.delete_rows(&indexes(&[5, 10]));
    table.delete_columns(&indexes(&[9]));
    table.move_rows(&indexes(&[7]), 0);
    table.move_column(4, 0);
    assert_eq!(table.row_count(), 1);
    assert_eq!(table.columns.len(), 1);
}

#[test]
fn out_of_range_insertions_clamp() {
    let mut table = CsvTable::from_rows(rows(&[&["a"]]), None, 1);
    table.add_row(99);
    assert_eq!(table.columns[0].cells, vec!["a", ""]);
    // Swift passes -3 to mean the start; `usize` cannot be negative.
    table.add_row(0);
    assert_eq!(table.columns[0].cells, vec!["", "a", ""]);
    table.add_column(99, None);
    assert_eq!(cells(&table), rows(&[&["", "a", ""], &["", "", ""]]));
}

#[test]
fn table_equality_compares_content_not_column_identity() {
    let first = CsvTable::from_rows(rows(&[&["a", "b"]]), headers(&["A", "B"]), 1);
    let second = CsvTable::from_rows(rows(&[&["a", "b"]]), headers(&["A", "B"]), 1);
    assert_eq!(first, second);

    let mut changed = second.clone();
    changed.set_cell("x".to_owned(), 0, 1);
    assert_ne!(first, changed);
    changed.set_headers(None);
    assert_ne!(first, changed);
}

#[test]
fn column_order_comparison_detects_identity_even_when_content_matches() {
    let mut table = CsvTable::from_rows(rows(&[&["", "", ""]]), None, 1);
    let original = table.clone();
    table.move_column(0, 2);

    // Content equality alone cannot see the move: all cells are empty.
    assert_eq!(table, original);
    assert!(!table.has_same_column_order(&original));
    assert!(table.has_same_column_order(&table));
}

#[test]
fn equal_content_with_different_ids_compares_equal_but_order_differs() {
    let first = CsvTable::new(vec![
        column(1, Some("A"), &["1"]),
        column(2, Some("B"), &["2"]),
    ]);
    let second = CsvTable::new(vec![
        column(99, Some("A"), &["1"]),
        column(100, Some("B"), &["2"]),
    ]);

    assert_eq!(first, second);
    assert!(!first.has_same_column_order(&second));
}

#[test]
fn add_column_on_headerless_table_ignores_header() {
    let mut table = CsvTable::from_rows(rows(&[&["a"]]), None, 1);
    table.add_column(1, Some("X"));
    assert_eq!(table.headers(), None);
    assert!(table.columns.iter().all(|column| column.header.is_none()));
    assert_eq!(cells(&table), rows(&[&["a"], &[""]]));
}

#[test]
fn deleting_every_column_keeps_one_empty_column() {
    let mut table = CsvTable::from_rows(rows(&[&["1", "2"], &["3", "4"]]), None, 1);
    table.delete_columns(&indexes(&[0, 1]));
    assert_eq!(table.columns.len(), 1);
    assert_eq!(table.row_count(), 2);
    assert_eq!(table.columns[0].cells, vec!["", ""]);
    assert_eq!(table.headers(), None);

    // Headers disappear as a whole when the last column is deleted.
    let mut named = CsvTable::from_rows(rows(&[&["1", "2"]]), headers(&["A", "B"]), 1);
    named.delete_columns(&indexes(&[0, 1]));
    assert_eq!(named.columns.len(), 1);
    assert_eq!(named.headers(), None);
}

#[test]
fn new_with_no_columns_keeps_one_column() {
    let table = CsvTable::new(Vec::new());
    assert_eq!(table.columns.len(), 1);
    assert_eq!(table.row_count(), 0);
    assert_eq!(table.headers(), None);
}

#[test]
fn generated_column_ids_are_unique_and_positive() {
    let first = CsvColumn::new(None, Vec::new());
    let second = CsvColumn::new(None, Vec::new());
    assert!(first.id >= 1);
    assert!(second.id > first.id);

    let table = CsvTable::from_rows(rows(&[&["a", "b", "c"]]), None, 1);
    let ids: BTreeSet<ColumnId> = table.columns.iter().map(|column| column.id).collect();
    assert_eq!(ids.len(), table.columns.len());
}

#[test]
fn rename_header_changes_only_that_column_and_keeps_identity() {
    let mut table = CsvTable::from_rows(rows(&[&["1", "2"]]), headers(&["a", "b"]), 1);
    let ids: Vec<ColumnId> = table.columns.iter().map(|column| column.id).collect();
    assert!(table.rename_header("x", 1));
    assert_eq!(table.headers(), headers(&["a", "x"]));
    assert_eq!(
        table
            .columns
            .iter()
            .map(|column| column.id)
            .collect::<Vec<_>>(),
        ids
    );
    assert_eq!(cells(&table), rows(&[&["1"], &["2"]]));
}

#[test]
fn rename_header_allows_empty_name() {
    let mut table = CsvTable::from_rows(rows(&[&["1"]]), headers(&["a"]), 1);
    assert!(table.rename_header("", 0));
    assert_eq!(table.headers(), headers(&[""]));
}

#[test]
fn rename_header_without_headers_generates_names_and_keeps_all_rows() {
    let mut table = CsvTable::from_rows(rows(&[&["1", "2"], &["3", "4"]]), None, 1);
    assert!(table.rename_header("x", 1));
    assert_eq!(table.headers(), headers(&["A", "x"]));
    assert_eq!(table.row_count(), 2);
    assert_eq!(cells(&table), rows(&[&["1", "3"], &["2", "4"]]));
}

#[test]
fn rename_header_to_generated_name_without_headers_changes_nothing() {
    let mut table = CsvTable::from_rows(rows(&[&["1", "2"]]), None, 1);
    assert!(!table.rename_header("B", 1));
    assert_eq!(table.headers(), None);
}

#[test]
fn rename_header_is_rejected_for_bad_index() {
    let mut plain = CsvTable::from_rows(rows(&[&["1"]]), None, 1);
    assert!(!plain.rename_header("x", 3));
    assert_eq!(plain.headers(), None);

    let mut named = CsvTable::from_rows(rows(&[&["1"]]), headers(&["a"]), 1);
    assert!(!named.rename_header("x", 3));
    assert_eq!(named.headers(), headers(&["a"]));
}

#[test]
fn column_names_follow_the_spreadsheet_sequence() {
    let cases = [
        (0, "A"),
        (25, "Z"),
        (26, "AA"),
        (27, "AB"),
        (51, "AZ"),
        (52, "BA"),
        (701, "ZZ"),
        (702, "AAA"),
    ];
    for (index, expected) in cases {
        assert_eq!(column_name(index), expected, "{index}");
    }
}

#[test]
fn negative_column_index_has_no_name() {
    assert_eq!(column_name_checked(-1), None);
    assert_eq!(column_name_checked(0), Some("A".to_owned()));
}

#[test]
fn csv_table_conforms_to_row_source() {
    let table = CsvTable::from_rows(rows(&[&["a", "b"], &["c"]]), headers(&["h1", "h2"]), 1);
    let source: &dyn RowSource = &table;
    assert_eq!(source.row_count(), 2);
    assert_eq!(source.column_count(), 2);
    assert_eq!(
        source.column_ids(),
        table
            .columns
            .iter()
            .map(|column| column.id)
            .collect::<Vec<_>>()
    );
    assert_eq!(source.headers(), headers(&["h1", "h2"]));
    assert_eq!(source.cell(1, 1), "");
    assert_eq!(source.cell(9, 0), "");
}
