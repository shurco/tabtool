//! Ports of the Apple `CSVCellOrderTests` suite, plus the Review Focus cases
//! for numbers, text, empties, stability and locale collation.

use tabtool_core::{CsvColumn, CsvTable, Locale, SortDirection, row_order_sorted_by};

fn locale(id: &str) -> Locale {
    id.parse().expect("test locale is valid")
}

fn single_column(cells: &[&str]) -> CsvTable {
    let rows = cells.iter().map(|cell| vec![(*cell).to_owned()]).collect();
    CsvTable::from_rows(rows, None, 1)
}

fn order(cells: &[&str], direction: SortDirection, decimal_mark: &str) -> Vec<usize> {
    row_order_sorted_by(
        &single_column(cells),
        0,
        direction,
        decimal_mark,
        &locale("en-US"),
    )
}

fn ordered(cells: &[&str], direction: SortDirection, decimal_mark: &str) -> Vec<String> {
    order(cells, direction, decimal_mark)
        .iter()
        .map(|&index| cells[index].to_owned())
        .collect()
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn ascending_puts_numbers_first_then_natural_text_then_empty() {
    assert_eq!(
        ordered(
            &["b", "", "10", "item10", "2", " ", "item2", "-3"],
            SortDirection::Ascending,
            "."
        ),
        strings(&["-3", "2", "10", "b", "item2", "item10", "", " "])
    );
}

#[test]
fn surrounding_whitespace_is_ignored_when_comparing_text() {
    assert_eq!(
        ordered(&[" b", "a", "c"], SortDirection::Ascending, "."),
        strings(&["a", " b", "c"])
    );
}

#[test]
fn descending_reverses_values_but_keeps_empty_last() {
    assert_eq!(
        ordered(&["1", "", "a", "3"], SortDirection::Descending, "."),
        strings(&["a", "3", "1", ""])
    );
}

#[test]
fn decimal_comma_is_numeric_only_for_comma_mark() {
    assert_eq!(
        ordered(&["1,5", "1,25", "1.0"], SortDirection::Ascending, ","),
        strings(&["1,25", "1,5", "1.0"])
    );
}

#[test]
fn equal_keys_keep_row_order() {
    let table = single_column(&["x", "1", "x", "1"]);
    assert_eq!(
        row_order_sorted_by(&table, 0, SortDirection::Ascending, ".", &locale("en-US")),
        vec![1, 3, 0, 2]
    );
    assert_eq!(
        row_order_sorted_by(&table, 0, SortDirection::Descending, ".", &locale("en-US")),
        vec![0, 2, 1, 3]
    );
}

#[test]
fn invalid_column_returns_identity() {
    let table = CsvTable::from_rows(vec![vec!["b".to_owned()], vec!["a".to_owned()]], None, 1);
    assert_eq!(
        row_order_sorted_by(&table, 4, SortDirection::Ascending, ".", &locale("en-US")),
        vec![0, 1]
    );
}

#[test]
fn empty_keys_are_last_in_both_directions_and_keep_their_order() {
    assert_eq!(
        order(&["", "b", "", "a", " "], SortDirection::Ascending, "."),
        vec![3, 1, 0, 2, 4]
    );
    assert_eq!(
        order(&["", "b", "", "a", " "], SortDirection::Descending, "."),
        vec![1, 3, 0, 2, 4]
    );
}

#[test]
fn leading_zeros_still_parse_as_numbers() {
    assert_eq!(
        order(&["001.5", "2", "1.25"], SortDirection::Ascending, "."),
        vec![2, 0, 1]
    );
}

#[test]
fn numbers_sort_before_text() {
    assert_eq!(
        order(&["b", "2"], SortDirection::Ascending, "."),
        vec![1, 0]
    );
}

#[test]
fn text_comparison_uses_the_numeric_ordering_of_the_collator() {
    // Ordinal comparison would put "item10" before "item2".
    assert_eq!(
        ordered(
            &["item10", "item2", "b", "a"],
            SortDirection::Ascending,
            "."
        ),
        strings(&["a", "b", "item2", "item10"])
    );
}

#[test]
fn case_is_a_tertiary_difference_and_ties_keep_original_order() {
    // Foundation's localizedStandardCompare orders lowercase before uppercase.
    assert_eq!(
        order(&["b", "A", "a"], SortDirection::Ascending, "."),
        vec![2, 1, 0]
    );
    assert_eq!(
        order(&["a", "A"], SortDirection::Ascending, "."),
        vec![0, 1]
    );
}

#[test]
fn locale_controls_text_ordering() {
    let table = single_column(&["ä", "z", "a"]);
    let english = row_order_sorted_by(&table, 0, SortDirection::Ascending, ".", &locale("en-US"));
    let swedish = row_order_sorted_by(&table, 0, SortDirection::Ascending, ".", &locale("sv-SE"));
    assert_eq!(english, vec![2, 0, 1]);
    assert_eq!(swedish, vec![2, 1, 0]);
}

#[test]
fn short_columns_sort_as_empty_cells_instead_of_panicking() {
    let table = CsvTable {
        columns: vec![
            CsvColumn {
                id: 1,
                header: None,
                cells: vec!["a".to_owned(), "b".to_owned()],
            },
            CsvColumn {
                id: 2,
                header: None,
                cells: vec!["x".to_owned()],
            },
        ],
    };

    // `row_count` comes from the first column, so the short second column
    // reads an empty cell for the missing row instead of panicking.
    let order = row_order_sorted_by(&table, 1, SortDirection::Ascending, ".", &locale("en-US"));
    assert_eq!(order, vec![0, 1]);
}
