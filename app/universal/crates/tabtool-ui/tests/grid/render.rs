//! Virtualization read counts and render snapshots.

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use egui::{Rect, pos2, vec2};
use tabtool_core::{CsvConfiguration, CsvTable, RowSource, SortDirection};
use tabtool_document::Document;
use tabtool_ui::grid::{CellEditor, GridOptions, GridState, SortSpec};
use tabtool_ui::harness::kittest::Queryable;

use crate::support::{CountingSource, GridFixture, sample_table};

/// Wraps a source and rejects any request outside its reported bounds.
///
/// The grid must never ask for rows that the large-file index has not
/// published or columns beyond the widest scanned row.
struct GuardedSource<'a> {
    inner: &'a dyn RowSource,
    published_rows: Option<usize>,
    out_of_bounds: AtomicUsize,
    calls: AtomicUsize,
    min_row: AtomicUsize,
    max_row: AtomicUsize,
    min_column: AtomicUsize,
    max_column: AtomicUsize,
}

impl<'a> GuardedSource<'a> {
    fn new(inner: &'a dyn RowSource) -> Self {
        Self::with_row_limit(inner, None)
    }

    /// Serves only `rows` published rows, the shape of a partial index.
    fn with_row_limit(inner: &'a dyn RowSource, rows: Option<usize>) -> Self {
        Self {
            inner,
            published_rows: rows,
            out_of_bounds: AtomicUsize::new(0),
            calls: AtomicUsize::new(0),
            min_row: AtomicUsize::new(usize::MAX),
            max_row: AtomicUsize::new(0),
            min_column: AtomicUsize::new(usize::MAX),
            max_column: AtomicUsize::new(0),
        }
    }

    fn reset(&self) {
        self.out_of_bounds.store(0, Ordering::Relaxed);
        self.calls.store(0, Ordering::Relaxed);
        self.min_row.store(usize::MAX, Ordering::Relaxed);
        self.max_row.store(0, Ordering::Relaxed);
        self.min_column.store(usize::MAX, Ordering::Relaxed);
        self.max_column.store(0, Ordering::Relaxed);
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::Relaxed)
    }

    fn out_of_bounds(&self) -> usize {
        self.out_of_bounds.load(Ordering::Relaxed)
    }

    fn read_bounds(&self) -> Option<(usize, usize, usize, usize)> {
        (self.calls() > 0).then(|| {
            (
                self.min_row.load(Ordering::Relaxed),
                self.max_row.load(Ordering::Relaxed),
                self.min_column.load(Ordering::Relaxed),
                self.max_column.load(Ordering::Relaxed),
            )
        })
    }
}

impl RowSource for GuardedSource<'_> {
    fn row_count(&self) -> usize {
        self.published_rows
            .unwrap_or_else(|| self.inner.row_count())
            .min(self.inner.row_count())
    }

    fn column_count(&self) -> usize {
        self.inner.column_count()
    }

    fn column_ids(&self) -> Vec<tabtool_core::ColumnId> {
        self.inner.column_ids()
    }

    fn headers(&self) -> Option<Vec<String>> {
        self.inner.headers()
    }

    fn cell(&self, row: usize, column: usize) -> String {
        self.calls.fetch_add(1, Ordering::Relaxed);
        self.min_row.fetch_min(row, Ordering::Relaxed);
        self.max_row.fetch_max(row, Ordering::Relaxed);
        self.min_column.fetch_min(column, Ordering::Relaxed);
        self.max_column.fetch_max(column, Ordering::Relaxed);
        if row >= self.row_count() || column >= self.column_count() {
            self.out_of_bounds.fetch_add(1, Ordering::Relaxed);
        }
        self.inner.cell(row, column)
    }
}

/// Opens `path` as a large document and waits for its index.
fn open_large(path: &Path, configuration: CsvConfiguration) -> Document {
    let mut document = Document::new(configuration.clone());
    document
        .open_large(path, Some(configuration), false, 0)
        .expect("open the large fixture");
    if let Some(state) = document.large_state() {
        state.indexer.wait();
    }
    document.poll_large_indexing();
    document
}

#[test]
fn only_the_visible_cells_of_a_large_source_are_read() {
    let source = CountingSource::new(1_000_000, 20);
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    fixture.run();
    source.reset();
    fixture.run();

    let calls = source.calls();
    let viewport = fixture.state.borrow().viewport;
    assert_eq!(
        viewport,
        Rect::from_min_size(pos2(0.0, 0.0), vec2(624.0, 384.0))
    );
    assert_eq!(source.read_bounds(), Some((0, 15, 0, 6)));
    eprintln!("large row count: read {calls} cells, viewport {viewport:?}");
    assert_eq!(calls, 16 * 7);
    assert_eq!(calls, 112);
}

#[test]
fn scrolling_far_reads_only_the_rows_around_the_viewport() {
    let source = CountingSource::new(1_000_000, 20);
    let mut state = GridState::default();
    state.scroll = vec2(480.0, 999_000.0 * 24.0);
    let mut fixture = GridFixture::new(&source, state, GridOptions::default(), [640.0, 400.0]);
    fixture.run();
    fixture.run();
    source.reset();
    fixture.run();

    let calls = source.calls();
    let viewport = fixture.state.borrow().viewport;
    assert_eq!(viewport.min, pos2(480.0, 999_000.0 * 24.0));
    assert_eq!(viewport.size(), vec2(624.0, 384.0));
    assert_eq!(source.read_bounds(), Some((998_999, 999_015, 3, 11)));
    eprintln!("scrolled large row count: read {calls} cells, viewport {viewport:?}");
    assert_eq!(calls, 17 * 9);
    assert_eq!(calls, 153);
}

#[test]
fn a_hundred_thousand_row_scroll_stays_inside_the_viewport() {
    let source = CountingSource::new(100_000, 12);
    let mut state = GridState::default();
    state.scroll = vec2(0.0, 99_000.0 * 24.0);
    let mut fixture = GridFixture::new(&source, state, GridOptions::default(), [640.0, 400.0]);
    fixture.run();
    fixture.run();
    source.reset();
    let started = Instant::now();
    fixture.run();
    let elapsed = started.elapsed();

    let calls = source.calls();
    let viewport = fixture.state.borrow().viewport;
    assert_eq!(viewport.min, pos2(0.0, 99_000.0 * 24.0));
    assert_eq!(viewport.size(), vec2(624.0, 384.0));
    assert_eq!(source.read_bounds(), Some((98_999, 99_015, 0, 6)));
    assert_eq!(calls, 17 * 7);
    assert_eq!(source.out_of_bounds(), 0);
    eprintln!("100k row scroll: read {calls} cells in {elapsed:?}, viewport {viewport:?}");
}

fn large_document(rows: usize) -> (tempfile::TempDir, Document) {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("large.csv");
    let text: String = (0..rows)
        .map(|row| format!("r{row},v{row},q{row}\n"))
        .collect();
    std::fs::write(&path, text).expect("fixture file is written");
    let configuration = CsvConfiguration {
        first_row_as_header: false,
        ..CsvConfiguration::default()
    };
    let document = open_large(&path, configuration);
    (directory, document)
}

#[test]
fn a_large_document_reads_only_rows_inside_its_published_index() {
    let (_directory, document) = large_document(5_000);
    let state = document.large_state().expect("large state");
    let counters = state.indexer.counters();
    let source = document.row_source().expect("large row source");
    let scanned = u128::try_from(counters.scanned_bytes).expect("scanned bytes fit");
    let file_size = u128::try_from(state.file.size()).expect("file size fits");
    assert_eq!(
        scanned, file_size,
        "the fixture's index covers the whole file"
    );
    assert_eq!(source.row_count(), counters.row_count);

    let guarded = GuardedSource::new(source);
    let mut fixture = GridFixture::new(
        &guarded,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    fixture.run();
    guarded.reset();
    let started = Instant::now();
    fixture.run();
    let elapsed = started.elapsed();
    let calls = guarded.calls();
    let (min_row, max_row, min_column, max_column) =
        guarded.read_bounds().expect("the grid read cells");

    assert_eq!(guarded.out_of_bounds(), 0);
    assert!(min_row < max_row);
    assert!(max_row < counters.row_count);
    assert!(max_column < counters.max_field_count);
    assert_eq!(calls, 16 * 3);
    assert_eq!((min_row, max_row, min_column, max_column), (0, 15, 0, 2));
    eprintln!(
        "large document: {} rows indexed, {} columns, read {calls} cells in {elapsed:?} in rows {min_row}..={max_row} and columns {min_column}..={max_column}",
        counters.row_count, counters.max_field_count
    );

    let viewport = fixture.state.borrow().viewport;
    assert_eq!(viewport.min, pos2(0.0, 0.0));
    assert_eq!(viewport.size(), vec2(624.0, 384.0));
}

#[test]
fn a_partially_indexed_large_source_never_reads_unpublished_rows() {
    let (_directory, document) = large_document(5_000);
    let source = document.row_source().expect("large row source");
    // A partial publication: only the first 120 of 5,000 rows have known ends.
    let guarded = GuardedSource::with_row_limit(source, Some(120));
    assert_eq!(guarded.row_count(), 120);
    assert!(guarded.row_count() < source.row_count());

    let mut fixture = GridFixture::new(
        &guarded,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    fixture.run();
    guarded.reset();
    let started = Instant::now();
    fixture.run();
    let elapsed = started.elapsed();
    let (min_row, max_row, min_column, max_column) =
        guarded.read_bounds().expect("the grid read cells");

    assert_eq!(guarded.out_of_bounds(), 0);
    assert!(max_row < 120, "read row {max_row} past the partial index");
    assert!(max_column < source.column_count());
    assert_eq!((min_row, max_row, min_column, max_column), (0, 15, 0, 2));
    assert_eq!(guarded.calls(), 16 * 3);
    eprintln!(
        "partial index: 120 of {} rows published, read {min_column}..={max_column} in rows {min_row}..={max_row} in {elapsed:?}",
        source.row_count()
    );

    let viewport = fixture.state.borrow().viewport;
    assert_eq!(viewport.min, pos2(0.0, 0.0));
    assert_eq!(viewport.size(), vec2(624.0, 384.0));
}

#[cfg(target_os = "macos")]
fn snapshot(
    name: &str,
    source: &dyn tabtool_core::RowSource,
    state: GridState,
    options: GridOptions,
) {
    if std::env::var_os("TABTOOL_SKIP_SNAPSHOTS").is_some() {
        return;
    }
    let mut fixture = GridFixture::new(source, state, options, [640.0, 400.0]);
    fixture.run();
    fixture.snapshot(name);
}

#[cfg(target_os = "macos")]
#[test]
fn empty_one_by_one_matches_the_snapshot() {
    let source = CsvTable::new_document(1, 1);
    snapshot(
        "grid-empty-1x1",
        &source,
        GridState::default(),
        GridOptions::default(),
    );
}

#[cfg(target_os = "macos")]
#[test]
fn header_and_data_match_the_snapshot() {
    let source = sample_table();
    let options = GridOptions {
        sort: Some(SortSpec {
            column: 1,
            direction: SortDirection::Ascending,
        }),
        ..GridOptions::default()
    };
    snapshot("grid-header-data", &source, GridState::default(), options);
}

#[cfg(target_os = "macos")]
#[test]
fn the_focused_cell_draws_a_two_pixel_border() {
    if std::env::var_os("TABTOOL_SKIP_SNAPSHOTS").is_some() {
        return;
    }
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    fixture.get_by_label("Bob").click();
    fixture.run();
    fixture.run();
    fixture.remove_cursor();
    fixture.run();
    fixture.snapshot("grid-focused-cell");
}

#[cfg(target_os = "macos")]
#[test]
fn selected_row_matches_the_snapshot() {
    let source = sample_table();
    let mut state = GridState::default();
    state.selection.rows.insert(1);
    snapshot("grid-selected-row", &source, state, GridOptions::default());
}

#[cfg(target_os = "macos")]
#[test]
fn selected_column_matches_the_snapshot() {
    let source = sample_table();
    let mut state = GridState::default();
    state.selection.columns.insert(1);
    snapshot(
        "grid-selected-column",
        &source,
        state,
        GridOptions::default(),
    );
}

#[cfg(target_os = "macos")]
#[test]
fn editing_cell_matches_the_snapshot() {
    let source = sample_table();
    let mut state = GridState::default();
    state.editing = Some(CellEditor::new(1, 1, "20"));
    snapshot("grid-editing-cell", &source, state, GridOptions::default());
}

#[cfg(target_os = "macos")]
#[test]
fn hidden_gutter_matches_the_snapshot() {
    let source = sample_table();
    let options = GridOptions {
        shows_row_numbers: false,
        ..GridOptions::default()
    };
    snapshot("grid-hidden-gutter", &source, GridState::default(), options);
}

#[cfg(target_os = "macos")]
#[test]
fn large_mode_header_offset_matches_the_snapshot() {
    let source = sample_table();
    let options = GridOptions {
        header_offset: 1,
        ..GridOptions::default()
    };
    snapshot(
        "grid-large-header-offset",
        &source,
        GridState::default(),
        options,
    );
}

#[test]
fn dragging_the_header_edge_resizes_the_column() {
    let source = sample_table();
    let mut fixture = GridFixture::new(
        &source,
        GridState::default(),
        GridOptions::default(),
        [640.0, 400.0],
    );
    fixture.run();
    let rect = fixture.get_by_label("Name").rect();
    let start = egui::pos2(rect.max.x - 1.0, rect.center().y);
    let end = start + egui::vec2(60.0, 0.0);

    fixture.hover_at(start);
    fixture.run();
    fixture.drag_at(start);
    fixture.run();
    fixture.hover_at(end);
    fixture.run();
    fixture.drop_at(end);
    fixture.run();

    let width = fixture.state.borrow().ordered_widths(&source)[0];
    assert!(width > 100.0, "the dragged column stayed at {width} pt");
}

#[cfg(target_os = "macos")]
#[test]
fn scrolled_chrome_matches_the_snapshot() {
    if std::env::var_os("TABTOOL_SKIP_SNAPSHOTS").is_some() {
        return;
    }
    let headers = (0..12).map(|index| format!("Column {index}")).collect();
    let source = CountingSource::with_headers(100, 12, Some(headers));
    let mut state = GridState::default();
    state.scroll = vec2(100.0, 200.0);
    let mut fixture = GridFixture::new(&source, state, GridOptions::default(), [640.0, 400.0]);
    fixture.run();

    let header = fixture.get_by_label("Column 1").rect();
    let cell = fixture.get_by_label("r9c1").rect();
    let row_number = fixture.get_by_label("10").rect();
    assert!(
        header.min.y < cell.min.y,
        "the header moved with the vertical scroll"
    );
    assert!(
        row_number.min.x < cell.min.x,
        "the row-number gutter moved with the horizontal scroll"
    );

    fixture.remove_cursor();
    fixture.run();
    fixture.snapshot("grid-scrolled-chrome");
}
