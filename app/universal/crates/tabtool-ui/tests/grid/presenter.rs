//! Presenter projection tests.

use tabtool_core::{ColumnId, CsvTable, RowSource, new_column_id};
use tabtool_ui::grid::CellAlignment;
use tabtool_ui::grid::{GridGeometry, GridPresenter};

use crate::support::{CountingSource, sample_table};

fn presenter<'a>(table: &'a CsvTable, visible_rows: Option<&'a [usize]>) -> GridPresenter<'a> {
    GridPresenter::new(table, visible_rows)
}

#[test]
fn headers_prefer_document_headers() {
    let table = sample_table();
    let presenter = presenter(&table, None);

    assert_eq!(presenter.header(0), "Name");
    assert_eq!(presenter.header(1), "Amount");
}

#[test]
fn headers_fall_back_to_generated_names() {
    let table = CsvTable::from_rows(vec![vec!["a".into(), "b".into()]], None, 2);
    let presenter = presenter(&table, None);

    assert_eq!(presenter.header(0), "A");
    assert_eq!(presenter.header(1), "B");
    assert_eq!(presenter.header(2), "");
}

#[test]
fn cells_and_alignment_follow_the_decimal_mark() {
    let table = CsvTable::from_rows(vec![vec!["1,5".into(), "x".into()]], None, 2);
    let presenter = presenter(&table, None);

    assert_eq!(presenter.cell(0, 0), "1,5");
    assert_eq!(presenter.alignment(0, 0, ","), CellAlignment::Right);
    assert_eq!(presenter.alignment(0, 1, ","), CellAlignment::Left);
}

#[test]
fn row_and_column_counts_follow_the_table() {
    let table = CsvTable::from_rows(
        vec![
            vec!["a".into(), "b".into(), "c".into()],
            vec!["d".into(), "e".into(), "f".into()],
        ],
        None,
        3,
    );
    let presenter = presenter(&table, None);

    assert_eq!(presenter.row_count(), 2);
    assert_eq!(presenter.column_count(), 3);
}

#[test]
fn the_presenter_reads_any_row_source() {
    let source = CountingSource::new(3, 2);
    let presenter = GridPresenter::new(&source, None);

    assert_eq!(presenter.row_count(), 3);
    assert_eq!(presenter.column_count(), 2);
    assert_eq!(presenter.header(1), "B");
    assert_eq!(presenter.header(2), "");
    assert_eq!(presenter.cell(2, 1), "r2c1");
}

#[test]
fn visible_rows_define_the_displayed_rows() {
    let table = CsvTable::from_rows(
        vec![
            vec!["0".into(), "x".into()],
            vec!["1".into(), "y".into()],
            vec!["2".into(), "z".into()],
            vec!["3".into(), "w".into()],
        ],
        None,
        2,
    );
    let visible = [1, 3];
    let presenter = presenter(&table, Some(&visible));

    assert_eq!(presenter.row_count(), 2);
    assert_eq!(presenter.cell(0, 1), "y");
    assert_eq!(presenter.cell(1, 1), "w");
    assert_eq!(presenter.alignment(1, 0, "."), CellAlignment::Right);
    assert_eq!(presenter.alignment(1, 1, "."), CellAlignment::Left);
}

#[test]
fn rows_map_between_display_and_model_indexes() {
    let table = CsvTable::from_rows(
        vec![
            vec!["0".into()],
            vec!["1".into()],
            vec!["2".into()],
            vec!["3".into()],
        ],
        None,
        1,
    );
    let visible = [1, 3];
    let presenter = presenter(&table, Some(&visible));

    assert_eq!(presenter.model_row(1), Some(3));
    assert_eq!(presenter.display_row(3), Some(1));
    assert_eq!(presenter.display_row(2), None);
    assert_eq!(presenter.model_row(2), None);
    assert_eq!(
        presenter.model_rows(&[0, 1].into_iter().collect()),
        [1, 3].into_iter().collect()
    );
    assert_eq!(
        presenter.display_rows(&[0, 1, 2, 3].into_iter().collect()),
        [0, 1].into_iter().collect()
    );
}

#[test]
fn no_visible_row_shows_an_empty_grid() {
    let table = sample_table();
    let presenter = presenter(&table, Some(&[]));

    assert_eq!(presenter.row_count(), 0);
    assert!(
        presenter
            .display_rows(&[0, 1].into_iter().collect())
            .is_empty()
    );
}

/// A source whose cell ids add the physical header offset, like
/// `LargeCSVSource`, while `GridView` still indexes display rows.
struct OffsetSource {
    rows: usize,
    offset: usize,
}

impl RowSource for OffsetSource {
    fn row_count(&self) -> usize {
        self.rows
    }

    fn column_count(&self) -> usize {
        1
    }

    fn column_ids(&self) -> Vec<ColumnId> {
        vec![new_column_id()]
    }

    fn headers(&self) -> Option<Vec<String>> {
        None
    }

    fn cell(&self, row: usize, column: usize) -> String {
        if column == 0 {
            format!("physical {}", row + self.offset)
        } else {
            String::new()
        }
    }
}

#[test]
fn an_offset_source_and_a_filter_mapping_use_display_rows() {
    let source = OffsetSource { rows: 4, offset: 1 };
    let visible = [0, 2];
    let presenter = GridPresenter::new(&source, Some(&visible));

    assert_eq!(presenter.row_count(), 2);
    assert_eq!(presenter.cell(0, 0), "physical 1");
    assert_eq!(presenter.cell(1, 0), "physical 3");

    let widths = [100.0];
    let geometry = GridGeometry {
        widths: &widths,
        row_height: 24.0,
        row_count: 2,
        shows_row_numbers: true,
        header_offset: 1,
    };
    assert_eq!(geometry.row_number(0), 2);
    assert_eq!(geometry.row_number(1), 3);
}
