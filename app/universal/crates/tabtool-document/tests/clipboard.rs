//! Port of the clipboard cases in `CSVDocumentFormatTests.swift`,
//! `GridInteractionTests.swift`, `CSVDocumentLargeEditingTests.swift` and the
//! `CSVClipboardTests` integration behavior for TabTool's Rust document.
//!
//! Copy reads the row source in visual column order and localizes numeric
//! cells to the supplied decimal separator. Paste parses TSV, converts the
//! source decimal mark to the document mark, inserts after the last selected
//! row and records one undo step.

use std::collections::BTreeSet;

use tabtool_core::{CsvConfiguration, Locale, SortDirection};
use tabtool_document::{Document, GridSelection, UndoAction};

fn locale(id: &str) -> Locale {
    id.parse().expect("test locale is valid")
}

fn loaded_with(csv: &str, configuration: CsvConfiguration) -> Document {
    let mut document = Document::new(configuration.clone());
    document
        .load_from_bytes(csv.as_bytes(), None, Some(configuration), false)
        .expect("load the fixture");
    document
}

fn loaded(csv: &str) -> Document {
    loaded_with(csv, CsvConfiguration::default())
}

fn loaded_with_header(csv: &str) -> Document {
    loaded_with(
        csv,
        CsvConfiguration {
            first_row_as_header: true,
            ..CsvConfiguration::default()
        },
    )
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

fn wide_document(rows: usize, columns: usize) -> Document {
    let mut csv = String::with_capacity(rows * columns * 2);
    for row in 0..rows {
        if row > 0 {
            csv.push('\n');
        }
        for column in 0..columns {
            if column > 0 {
                csv.push(',');
            }
            csv.push('a');
        }
    }
    loaded(&csv)
}

#[test]
fn copying_rows_uses_all_columns_in_visual_order() {
    let mut document = loaded("a,b\nc,d\n");
    select_rows(&mut document, [0]);

    assert_eq!(
        document.copy_selection(&locale("en-US"), "."),
        Some("a\tb".to_string())
    );
}

#[test]
fn copying_columns_uses_all_rows() {
    let mut document = loaded("a,b\nc,d\n");
    select_columns(&mut document, [1]);

    assert_eq!(
        document.copy_selection(&locale("en-US"), "."),
        Some("b\nd".to_string())
    );
}

#[test]
fn copy_selection_uses_visual_column_order() {
    let mut document = loaded("a,b\nc,d\n");
    document.move_columns(&BTreeSet::from([1]), 0);
    select_rows(&mut document, [0]);

    assert_eq!(
        document.copy_selection(&locale("en-US"), "."),
        Some("b\ta".to_string())
    );
}

#[test]
fn rows_win_over_selected_columns() {
    let mut document = loaded("a,b\nc,d\n");
    *document.selection_mut() = GridSelection {
        rows: BTreeSet::from([0]),
        columns: BTreeSet::from([1]),
    };

    assert_eq!(
        document.copy_selection(&locale("en-US"), "."),
        Some("a\tb".to_string())
    );
}

#[test]
fn copy_rows_copies_the_requested_rows() {
    let document = loaded("r0,v0,q0\nr1,v1,q 1\nr2,v2,q 2\n");

    assert_eq!(
        document.copy_rows(&BTreeSet::from([1, 2]), &locale("en-US"), "."),
        Some("r1\tv1\tq 1\nr2\tv2\tq 2".to_string())
    );
}

#[test]
fn copy_without_a_selection_copies_nothing() {
    let document = loaded("a,b\nc,d\n");

    assert_eq!(document.copy_selection(&locale("en-US"), "."), None);
}

#[test]
fn copy_ignores_out_of_range_indexes() {
    let mut document = loaded("a,b\nc,d\n");

    select_rows(&mut document, [9]);
    assert_eq!(document.copy_selection(&locale("en-US"), "."), None);

    select_columns(&mut document, [1, 9]);
    assert_eq!(
        document.copy_selection(&locale("en-US"), "."),
        Some("b\nd".to_string())
    );

    select_columns(&mut document, [9]);
    assert_eq!(document.copy_selection(&locale("en-US"), "."), None);
}

#[test]
fn copy_flattens_tabs_and_line_breaks_and_localizes_numbers() {
    let configuration = CsvConfiguration {
        column_separator: ";".to_string(),
        decimal_mark: ",".to_string(),
        ..CsvConfiguration::default()
    };
    let mut document = loaded_with("1,5;x\n2,5;y\n", configuration);
    document.set_cell(0, 1, "a\tb\nc");

    select_rows(&mut document, [0, 1]);

    assert_eq!(
        document.copy_selection(&locale("en-US"), "."),
        Some("1.5\ta b c\n2.5\ty".to_string())
    );
}

#[test]
fn copy_uses_the_supplied_decimal_separator() {
    let mut document = loaded("1.5,2\n");
    select_rows(&mut document, [0]);

    assert_eq!(
        document.copy_selection(&locale("de-DE"), ","),
        Some("1,5\t2".to_string())
    );
}

#[test]
fn an_in_memory_copy_has_no_cell_limit() {
    let mut document = wide_document(1_000, 1_001);
    let en = locale("en-US");

    select_columns(&mut document, 0..1_000);
    let copied = document
        .copy_selection(&en, ".")
        .expect("in-memory columns are not capped");
    assert_eq!(copied.len(), 1_999_999);
    assert_eq!(document.notice(), None);

    select_rows(&mut document, 0..1_000);
    assert!(
        document.copy_selection(&en, ".").is_some(),
        "in-memory rows are not capped"
    );
    assert_eq!(document.notice(), None);
}

#[test]
fn in_memory_row_copy_is_not_capped() {
    let document = wide_document(1_000, 1_001);
    let rows: BTreeSet<usize> = (0..1_000).collect();

    assert!(
        document.copy_rows(&rows, &locale("en-US"), ".").is_some(),
        "in-memory rows are not capped"
    );
    assert_eq!(document.notice(), None);
}

#[test]
fn paste_inserts_rows_adds_columns_and_converts_decimal() {
    let configuration = CsvConfiguration {
        decimal_mark: ",".to_string(),
        ..CsvConfiguration::default()
    };
    let mut document = loaded_with("a,b\n", configuration);
    select_rows(&mut document, [0]);

    document.paste_tsv("1.5\t2\t3", &locale("en-US"), ".");

    let table = document.table().expect("table");
    assert_eq!(table.row_count(), 2);
    assert_eq!(table.columns.len(), 3);
    assert_eq!(table.cell(1, 0), "1,5");
    assert_eq!(table.cell(1, 2), "3");
    assert_eq!(document.selection().rows, BTreeSet::from([1]));
}

#[test]
fn paste_is_one_undo_step() {
    let mut document = loaded("a,b\n");
    select_rows(&mut document, [0]);

    document.paste_tsv("1\t2\t3", &locale("en-US"), ".");

    assert_eq!(document.undo_action(), Some(UndoAction::PasteData));
    assert!(document.undo());
    let table = document.table().expect("table");
    assert_eq!(table.row_count(), 1);
    assert_eq!(table.columns.len(), 2);
    assert_eq!(table.cell(0, 0), "a");
    assert_eq!(document.selection().rows, BTreeSet::from([0]));

    assert!(document.redo());
    let table = document.table().expect("table");
    assert_eq!(table.row_count(), 2);
    assert_eq!(table.columns.len(), 3);
    assert_eq!(table.cell(1, 0), "1");
    assert_eq!(document.selection().rows, BTreeSet::from([1]));
}

#[test]
fn paste_appends_when_no_row_is_selected() {
    let mut document = loaded("a\nb\n");

    document.paste_tsv("x\ty", &locale("en-US"), ".");

    let table = document.table().expect("table");
    assert_eq!(table.row_count(), 3);
    assert_eq!(table.columns.len(), 2);
    assert_eq!(table.cell(2, 0), "x");
    assert_eq!(table.cell(2, 1), "y");
    assert_eq!(document.selection().rows, BTreeSet::from([2]));
}

#[test]
fn paste_inserts_after_the_last_selected_row() {
    let mut document = loaded("a\nb\nc\nd\n");
    select_rows(&mut document, [0, 2]);

    document.paste_tsv("x", &locale("en-US"), ".");

    let column: Vec<String> = document.table().expect("table").columns[0].cells.clone();
    assert_eq!(column, vec!["a", "b", "c", "x", "d"]);
    assert_eq!(document.selection().rows, BTreeSet::from([3]));
}

#[test]
fn paste_keeps_an_empty_string_as_one_empty_row() {
    let mut document = loaded("a,b\n");

    document.paste_tsv("", &locale("en-US"), ".");

    let table = document.table().expect("table");
    assert_eq!(table.row_count(), 2);
    assert_eq!(table.cell(1, 0), "");
    assert_eq!(table.cell(1, 1), "");
    assert_eq!(document.selection().rows, BTreeSet::from([1]));
}

#[test]
fn paste_preserves_empty_fields_and_missing_trailing_cells() {
    let mut document = loaded("x\n");

    document.paste_tsv("a\t\tc\nb", &locale("en-US"), ".");

    let table = document.table().expect("table");
    assert_eq!(table.row_count(), 3);
    assert_eq!(table.columns.len(), 3);
    assert_eq!(table.cell(1, 0), "a");
    assert_eq!(table.cell(1, 1), "");
    assert_eq!(table.cell(1, 2), "c");
    assert_eq!(table.cell(2, 0), "b");
    assert_eq!(table.cell(2, 1), "");
    assert_eq!(table.cell(2, 2), "");
}

#[test]
fn paste_does_not_trim_or_interpret_quotes() {
    let mut document = loaded("x\n");

    document.paste_tsv(" a \t\"b", &locale("en-US"), ".");

    let table = document.table().expect("table");
    assert_eq!(table.cell(1, 0), " a ");
    assert_eq!(table.cell(1, 1), "\"b");
}

#[test]
fn paste_converts_only_numeric_cells_from_the_locale_mark() {
    let configuration = CsvConfiguration {
        decimal_mark: ",".to_string(),
        ..CsvConfiguration::default()
    };
    let mut document = loaded_with("a\n", configuration);

    document.paste_tsv("1.5\thello\t1,5", &locale("en-US"), ".");

    let table = document.table().expect("table");
    assert_eq!(table.cell(1, 0), "1,5");
    assert_eq!(table.cell(1, 1), "hello");
    assert_eq!(table.cell(1, 2), "1,5");
}

#[test]
fn paste_adds_columns_on_the_right_in_header_mode() {
    let mut document = loaded_with_header("A,B\n1,2\n");

    document.paste_tsv("x\ty\tz", &locale("en-US"), ".");

    let table = document.table().expect("table");
    assert_eq!(table.row_count(), 2);
    assert_eq!(
        table.headers(),
        Some(vec!["A".to_string(), "B".to_string(), String::new()])
    );
    assert_eq!(table.cell(1, 2), "z");
    assert_eq!(document.selection().rows, BTreeSet::from([1]));
}

#[test]
fn paste_is_blocked_while_a_reread_is_in_progress() {
    let mut document = loaded("a\n");
    let revision = document.revision();

    assert!(document.begin_reread());
    document.paste_tsv("x", &locale("en-US"), ".");
    assert_eq!(document.revision(), revision);
    assert_eq!(document.table().expect("table").row_count(), 1);
    assert!(!document.can_undo());
    document.end_reread();

    document.paste_tsv("x", &locale("en-US"), ".");
    assert_eq!(document.table().expect("table").row_count(), 2);
}

#[test]
fn paste_clears_the_active_sort() {
    let mut document = loaded("b\na\n");
    document.sort_rows(0, SortDirection::Ascending, &locale("en-US"));

    document.paste_tsv("x", &locale("en-US"), ".");

    assert_eq!(document.active_sort(), None);
}

#[test]
fn paste_without_a_loaded_document_is_a_no_op() {
    let mut document = Document::new(CsvConfiguration::default());
    let revision = document.revision();

    document.paste_tsv("x", &locale("en-US"), ".");

    assert_eq!(document.revision(), revision);
    assert!(!document.can_undo());
    assert_eq!(document.table().expect("table").row_count(), 1);
}
