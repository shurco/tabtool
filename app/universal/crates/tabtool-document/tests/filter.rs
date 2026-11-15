//! Port of `CSVDocumentFilteringTests.swift` plus the copy, paste, move,
//! rename and column-move interactions of the applied filter.
//!
//! The document owns the filter model: applying evaluates the current table,
//! cell edits keep the snapshot, structural changes and undo/redo refresh it,
//! and an applied filter locks the row structure (insert, delete, move, paste)
//! while columns stay editable. Filters are not undoable.
//!
//! Four Swift cases are waived: `visibleTableCommitsTheCellBeingEdited` and
//! `clearFilterCommitsTheCellBeingEdited` (the Rust document has no field
//! editor), and the two `objectWillChange` republish cases (the Rust document
//! has no Combine publisher). Their observable state changes are covered here.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::io;
use std::path::Path;

use tabtool_core::{CsvConfiguration, FilterCriterion, FilterOperator, Locale, SortDirection};
use tabtool_document::{Document, GridSelection, Storage, UndoAction};

/// In-memory storage that serves the file a revert re-reads.
struct MemoryStorage {
    bytes: RefCell<Vec<u8>>,
}

impl MemoryStorage {
    fn new(bytes: &[u8]) -> Self {
        Self {
            bytes: RefCell::new(bytes.to_vec()),
        }
    }
}

impl Storage for MemoryStorage {
    fn read(&self, _path: &Path) -> io::Result<Vec<u8>> {
        Ok(self.bytes.borrow().clone())
    }

    fn write_atomic(&self, _path: &Path, data: &[u8]) -> io::Result<()> {
        self.bytes.replace(data.to_vec());
        Ok(())
    }
}

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

fn loaded_from_storage(storage: &MemoryStorage) -> Document {
    let configuration = CsvConfiguration::default();
    let mut document = Document::new(configuration.clone());
    document
        .load_from_path(
            Path::new("/virtual/filter.csv"),
            storage,
            Some(configuration),
            false,
        )
        .expect("load the fixture");
    document
}

fn column_id(document: &Document, column: usize) -> u64 {
    document.table().expect("table").columns[column].id
}

fn criterion(column_id: u64, op: FilterOperator, value: &str, id: u64) -> FilterCriterion {
    FilterCriterion {
        id,
        is_enabled: true,
        column_id: Some(column_id),
        op,
        value: value.to_owned(),
    }
}

/// Applies "column equals `value`" through the document.
fn apply_filter(document: &mut Document, value: &str, column: usize) {
    let id = column_id(document, column);
    document.row_filter_mut().criteria = vec![criterion(id, FilterOperator::Equals, value, 1)];
    document.apply_all_filters();
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

#[test]
fn apply_all_filters_evaluates_the_current_table() {
    let mut document = loaded("a,1\nb,2\na,3\n");
    apply_filter(&mut document, "a", 0);

    assert_eq!(document.row_filter().visible_rows, Some(vec![0, 2]));
    assert!(document.row_filter().is_filtering());
}

#[test]
fn apply_filter_uses_only_the_given_criterion() {
    let mut document = loaded("a,1\nb,2\na,3\n");
    let first = criterion(column_id(&document, 0), FilterOperator::Equals, "a", 1);
    let second = criterion(column_id(&document, 1), FilterOperator::Equals, "2", 2);
    let second_id = second.id;
    document.row_filter_mut().criteria = vec![first, second];

    document.apply_filter(second_id);
    assert_eq!(document.row_filter().visible_rows, Some(vec![1]));
}

#[test]
fn applying_drops_selected_rows_that_are_hidden() {
    let mut document = loaded("a\nb\na\n");
    select_rows(&mut document, [0, 1, 2]);
    apply_filter(&mut document, "a", 0);

    assert_eq!(
        document.selection(),
        &GridSelection {
            rows: BTreeSet::from([0, 2]),
            columns: BTreeSet::new(),
        }
    );
}

#[test]
fn visible_table_keeps_only_the_filtered_rows_with_their_edits() {
    let mut document = loaded("a,1\nb,2\na,3\n");
    document.add_column_right_of_selection();
    apply_filter(&mut document, "a", 0);
    document.set_cell(2, 1, "z");

    let table = document.visible_table().expect("visible table");
    assert_eq!(table.row_count(), 2);
    assert_eq!(table.columns.len(), 3);
    assert_eq!(table.cell(0, 1), "1");
    assert_eq!(table.cell(1, 1), "z");
}

#[test]
fn visible_table_is_the_whole_table_without_a_filter() {
    let document = loaded("a\nb\n");
    assert_eq!(
        document.visible_table(),
        Some(document.table().expect("table").as_ref().clone())
    );
}

#[test]
fn cell_edit_does_not_reevaluate_the_filter() {
    let mut document = loaded("a\nb\na\n");
    apply_filter(&mut document, "a", 0);
    document.set_cell(0, 0, "z");
    assert_eq!(document.row_filter().visible_rows, Some(vec![0, 2]));
}

#[test]
fn undo_and_redo_of_a_cell_edit_reevaluate_the_filter() {
    let mut document = loaded("a\nb\na\n");
    apply_filter(&mut document, "a", 0);
    document.set_cell(1, 0, "a");
    document.set_cell(0, 0, "z");
    // Cell edits keep the snapshot, so the rows have not changed yet.
    assert_eq!(document.row_filter().visible_rows, Some(vec![0, 2]));

    // Undoing the second edit restores "a,a,a"; only a refresh shows row 1.
    assert!(document.undo());
    assert_eq!(document.row_filter().visible_rows, Some(vec![0, 1, 2]));
    assert!(document.undo());
    assert_eq!(document.row_filter().visible_rows, Some(vec![0, 2]));
    assert!(document.redo());
    assert_eq!(document.row_filter().visible_rows, Some(vec![0, 1, 2]));
}

#[test]
fn sort_reevaluates_the_filter_and_undo_restores_it() {
    let mut document = loaded("a,2\nb,1\na,3\n");
    apply_filter(&mut document, "a", 0);
    assert_eq!(document.row_filter().visible_rows, Some(vec![0, 2]));

    document.sort_rows(1, SortDirection::Ascending, &locale("en-US"));
    assert_eq!(
        document.table().expect("table").columns[1].cells.clone(),
        vec!["1", "2", "3"]
    );
    assert_eq!(document.row_filter().visible_rows, Some(vec![1, 2]));

    assert!(document.undo());
    assert_eq!(document.row_filter().visible_rows, Some(vec![0, 2]));
    assert!(document.redo());
    assert_eq!(document.row_filter().visible_rows, Some(vec![1, 2]));
}

#[test]
fn undo_of_row_deletion_keeps_visible_rows_in_range() {
    let mut document = loaded("a\nb\na\na\n");
    document.delete_rows(&BTreeSet::from([3]));
    apply_filter(&mut document, "a", 0);
    assert_eq!(document.row_filter().visible_rows, Some(vec![0, 2]));

    assert!(document.undo());
    assert_eq!(document.table().expect("table").row_count(), 4);
    assert_eq!(document.row_filter().visible_rows, Some(vec![0, 2, 3]));
    assert!(document.redo());
    assert_eq!(document.row_filter().visible_rows, Some(vec![0, 2]));
}

#[test]
fn revert_to_saved_reevaluates_the_filter() {
    let storage = MemoryStorage::new(b"a\nb\na\n");
    let mut document = loaded_from_storage(&storage);
    document.set_cell(1, 0, "a");
    apply_filter(&mut document, "a", 0);
    assert_eq!(document.row_filter().visible_rows, Some(vec![0, 1, 2]));

    document.revert_to_saved(&storage).expect("revert the file");
    assert_eq!(document.table().expect("table").cell(1, 0), "b");
    assert_eq!(document.row_filter().visible_rows, Some(vec![0, 2]));

    // The reload replaced every column identity; the criterion follows the
    // column at the same position.
    assert_eq!(
        document.row_filter().criteria[0].column_id,
        Some(column_id(&document, 0))
    );
}

#[test]
fn deleting_a_column_drops_its_criteria() {
    let mut document = loaded("a,1\nb,2\na,3\n");
    let first = column_id(&document, 0);
    let second = column_id(&document, 1);
    document.row_filter_mut().criteria = vec![
        criterion(first, FilterOperator::Equals, "a", 1),
        criterion(second, FilterOperator::GreaterThan, "1", 2),
    ];
    document.apply_all_filters();
    assert_eq!(document.row_filter().visible_rows, Some(vec![2]));

    document.delete_columns(&BTreeSet::from([1]));
    assert_eq!(
        document
            .row_filter()
            .applied
            .as_ref()
            .expect("applied")
            .criteria
            .iter()
            .map(|criterion| criterion.column_id)
            .collect::<Vec<_>>(),
        vec![Some(first)]
    );
    assert_eq!(
        document
            .row_filter()
            .criteria
            .iter()
            .map(|criterion| criterion.column_id)
            .collect::<Vec<_>>(),
        vec![Some(first), None]
    );
    assert_eq!(document.row_filter().visible_rows, Some(vec![0, 2]));
}

#[test]
fn row_structure_is_locked_only_while_filtering() {
    let mut document = loaded("a\nb\na\n");
    select_rows(&mut document, [0]);
    assert!(document.can_edit_row_structure());
    assert!(document.can_insert_rows());
    assert!(document.can_delete_selected_rows());
    assert!(document.can_delete_selection());
    assert!(document.can_move_selected_rows(1));

    apply_filter(&mut document, "a", 0);
    assert!(!document.can_edit_row_structure());
    assert!(!document.can_insert_rows());
    assert!(!document.can_delete_selected_rows());
    assert!(!document.can_delete_selection());
    assert!(!document.can_move_selected_rows(1));

    document.add_row_below_selection();
    document.paste_tsv("x", ".");
    assert_eq!(document.table().expect("table").row_count(), 3);
    assert_eq!(
        document.table().expect("table").cell(0, 0),
        "a",
        "a blocked paste leaves the hidden rows untouched"
    );

    document.row_filter_mut().clear();
    assert!(document.can_edit_row_structure());
    assert!(document.can_insert_rows());
}

#[test]
fn deleting_rows_while_filtering_is_a_no_op() {
    let mut document = loaded("a\nb\na\n");
    select_rows(&mut document, [0]);
    apply_filter(&mut document, "a", 0);

    document.delete_selection();
    assert_eq!(document.table().expect("table").row_count(), 3);
}

#[test]
fn row_commands_are_blocked_while_filtering() {
    let mut document = loaded("a\nb\na\n");
    select_rows(&mut document, [0]);
    apply_filter(&mut document, "a", 0);
    let rows = document.table().expect("table").columns[0].cells.clone();

    document.delete_rows(&BTreeSet::from([0]));
    document.move_rows(&BTreeSet::from([0]), 2);
    document.move_selected_rows(1);

    assert_eq!(document.table().expect("table").columns[0].cells, rows);
}

#[test]
fn column_deletion_stays_available_while_filtering() {
    let mut document = loaded("a,1\nb,2\n");
    apply_filter(&mut document, "a", 0);
    select_columns(&mut document, [1]);

    // The Swift `canSortSelection` predicate is waived for the Rust document:
    // sorting takes an explicit column and stays available while filtering, as
    // `sort_reevaluates_the_filter_and_undo_restores_it` proves.
    assert!(document.can_delete_selection());
}

#[test]
fn column_copy_covers_only_visible_rows() {
    let mut document = loaded("a,1\nb,2\na,3\n");
    apply_filter(&mut document, "a", 0);
    select_columns(&mut document, [1]);

    assert_eq!(
        document.copy_selection(&locale("en-US"), "."),
        Some("1\n3".to_owned())
    );
}

#[test]
fn row_copy_stays_selection_based_while_filtering() {
    let mut document = loaded("a,1\nb,2\na,3\n");
    select_rows(&mut document, [0]);
    apply_filter(&mut document, "a", 0);

    assert_eq!(
        document.copy_selection(&locale("en-US"), "."),
        Some("a\t1".to_owned())
    );
}

#[test]
fn column_copy_limit_counts_only_visible_rows() {
    // 1001 rows by 1001 columns: one row fails the filter, so a 1000-column
    // copy covers exactly 1,000,000 visible cells and is allowed.
    let mut csv = String::new();
    for row in 0..1_001 {
        if row > 0 {
            csv.push('\n');
        }
        for column in 0..1_001 {
            if column > 0 {
                csv.push(',');
            }
            csv.push(if row == 1 && column == 0 { 'b' } else { 'a' });
        }
    }
    let mut document = loaded(&csv);
    apply_filter(&mut document, "a", 0);
    assert_eq!(
        document.row_filter().visible_rows.as_ref().map(Vec::len),
        Some(1_000)
    );
    select_columns(&mut document, 0..1_000);

    let copied = document
        .copy_selection(&locale("en-US"), ".")
        .expect("one million visible cells is allowed");
    assert_eq!(copied.len(), 1_999_999);
}

#[test]
fn a_large_visible_set_keeps_the_selection_and_export_correct() {
    // 100k matching rows: the membership set keeps the apply/selection trim
    // and the filtered export linear instead of rows × visible.
    let row_count = 100_000;
    let mut csv = String::with_capacity(row_count * 2);
    for _ in 0..row_count {
        csv.push_str("a\n");
    }
    let mut document = loaded(&csv);
    select_rows(&mut document, 0..row_count);

    apply_filter(&mut document, "a", 0);

    assert_eq!(
        document.row_filter().visible_rows.as_ref().map(Vec::len),
        Some(row_count)
    );
    assert_eq!(document.selection().rows.len(), row_count);
    assert_eq!(
        document.visible_table().expect("visible table").row_count(),
        row_count
    );
}

#[test]
fn memory_mode_can_filter_once_loaded() {
    let document = loaded("a\n");
    assert!(document.can_filter());

    let unloaded = Document::new(CsvConfiguration::default());
    assert!(!unloaded.can_filter());
}

#[test]
fn toggle_filter_bar_keeps_the_filter() {
    let mut document = loaded("a\nb\n");
    apply_filter(&mut document, "a", 0);

    document.toggle_filter_bar();
    assert!(document.row_filter().is_bar_visible);
    assert!(document.row_filter().is_filtering());
    document.toggle_filter_bar();
    assert!(!document.row_filter().is_bar_visible);
    assert!(document.row_filter().is_filtering());
}

#[test]
fn filters_are_not_undoable() {
    let mut document = loaded("a\nb\n");
    let revision = document.revision();
    apply_filter(&mut document, "a", 0);
    assert!(!document.can_undo());
    assert_eq!(document.revision(), revision);

    document.clear_filter();
    assert!(!document.can_undo());
    assert_eq!(document.revision(), revision);
    assert_eq!(document.row_filter().visible_rows, None);
}

#[test]
fn clear_filter_resets_the_draft_and_the_filter() {
    let mut document = loaded("a\nb\n");
    apply_filter(&mut document, "a", 0);
    document.clear_filter();

    assert!(!document.row_filter().is_filtering());
    assert_eq!(document.row_filter().visible_rows, None);
    assert_eq!(document.row_filter().criteria.len(), 1);
    assert_eq!(document.row_filter().criteria[0].column_id, None);
}

#[test]
fn moving_a_column_keeps_the_filter_on_it() {
    let mut document = loaded("a,1\nb,2\na,3\n");
    let filtered_id = column_id(&document, 0);
    apply_filter(&mut document, "a", 0);

    document.move_columns(&BTreeSet::from([0]), 2);
    assert_eq!(document.table().expect("table").columns[1].id, filtered_id);
    assert_eq!(
        document.row_filter().criteria[0].column_id,
        Some(filtered_id)
    );
    assert_eq!(document.row_filter().visible_rows, Some(vec![0, 2]));
}

#[test]
fn renaming_a_column_keeps_the_filter() {
    let mut document = loaded("a,1\nb,2\na,3\n");
    apply_filter(&mut document, "a", 0);

    document.rename_column(0, "name");
    assert_eq!(document.row_filter().visible_rows, Some(vec![0, 2]));
    assert!(document.row_filter().is_filtering());
    assert_eq!(document.undo_action(), Some(UndoAction::RenameColumn));

    assert!(document.undo());
    assert_eq!(document.row_filter().visible_rows, Some(vec![0, 2]));
}

#[test]
fn filter_state_is_not_part_of_an_undo_snapshot() {
    let mut document = loaded("a\nb\na\n");
    document.set_cell(0, 0, "z");
    apply_filter(&mut document, "z", 0);
    assert_eq!(document.row_filter().visible_rows, Some(vec![0]));

    assert!(document.undo());
    // The undo restores the row text and refreshes the filter; the filter
    // itself (a view state) is still applied.
    assert!(document.row_filter().is_filtering());
    assert_eq!(document.row_filter().visible_rows, Some(vec![]));
}
