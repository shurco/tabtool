//! Shared sources and the kittest fixture for the grid tests.

use core::ops::{Deref, DerefMut};
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tabtool_core::{ColumnId, CsvTable, RowSource, new_column_id};
use tabtool_ui::grid::{GridAction, GridOptions, GridState, GridView};
use tabtool_ui::{Theme, UiHarness};

/// A four-row, three-column table with headers.
pub fn sample_table() -> CsvTable {
    CsvTable::from_rows(
        vec![
            vec!["Alice".into(), "1.5".into(), "first".into()],
            vec!["Bob".into(), "20".into(), "second".into()],
            vec!["Carol".into(), "-3.25".into(), "third".into()],
            vec!["Dave".into(), "4".into(), "fourth".into()],
        ],
        Some(vec!["Name".into(), "Amount".into(), "Note".into()]),
        3,
    )
}

/// A row source that does not store cells and records every read.
#[derive(Debug)]
pub struct CountingSource {
    rows: usize,
    columns: usize,
    headers: Option<Vec<String>>,
    cell_calls: AtomicUsize,
    min_row: AtomicUsize,
    max_row: AtomicUsize,
    min_column: AtomicUsize,
    max_column: AtomicUsize,
}

impl CountingSource {
    /// A source without headers.
    pub fn new(rows: usize, columns: usize) -> Self {
        Self::with_headers(rows, columns, None)
    }

    /// A source with optional headers.
    pub fn with_headers(rows: usize, columns: usize, headers: Option<Vec<String>>) -> Self {
        Self {
            rows,
            columns,
            headers,
            cell_calls: AtomicUsize::new(0),
            min_row: AtomicUsize::new(usize::MAX),
            max_row: AtomicUsize::new(0),
            min_column: AtomicUsize::new(usize::MAX),
            max_column: AtomicUsize::new(0),
        }
    }

    /// Forgets the reads recorded so far.
    pub fn reset(&self) {
        self.cell_calls.store(0, Ordering::Relaxed);
        self.min_row.store(usize::MAX, Ordering::Relaxed);
        self.max_row.store(0, Ordering::Relaxed);
        self.min_column.store(usize::MAX, Ordering::Relaxed);
        self.max_column.store(0, Ordering::Relaxed);
    }

    /// How many cells were read.
    pub fn calls(&self) -> usize {
        self.cell_calls.load(Ordering::Relaxed)
    }

    /// The inclusive row and column bounds that were read, if any.
    pub fn read_bounds(&self) -> Option<(usize, usize, usize, usize)> {
        let calls = self.calls();
        if calls == 0 {
            return None;
        }
        Some((
            self.min_row.load(Ordering::Relaxed),
            self.max_row.load(Ordering::Relaxed),
            self.min_column.load(Ordering::Relaxed),
            self.max_column.load(Ordering::Relaxed),
        ))
    }

    /// The deterministic text of a cell.
    pub fn cell_text(row: usize, column: usize) -> String {
        format!("r{row}c{column}")
    }
}

impl RowSource for CountingSource {
    fn row_count(&self) -> usize {
        self.rows
    }

    fn column_count(&self) -> usize {
        self.columns
    }

    fn column_ids(&self) -> Vec<ColumnId> {
        (0..self.columns).map(|_| new_column_id()).collect()
    }

    fn headers(&self) -> Option<Vec<String>> {
        self.headers.clone()
    }

    fn cell(&self, row: usize, column: usize) -> String {
        self.cell_calls.fetch_add(1, Ordering::Relaxed);
        self.min_row.fetch_min(row, Ordering::Relaxed);
        self.max_row.fetch_max(row, Ordering::Relaxed);
        self.min_column.fetch_min(column, Ordering::Relaxed);
        self.max_column.fetch_max(column, Ordering::Relaxed);
        Self::cell_text(row, column)
    }
}

/// A grid rendered by the deterministic kittest harness.
pub struct GridFixture<'a> {
    ui: UiHarness<'a>,
    /// The persistent grid state.
    pub state: Rc<RefCell<GridState>>,
    /// The actions the grid produced while rendering.
    pub actions: Rc<RefCell<Vec<GridAction>>>,
}

impl<'a> GridFixture<'a> {
    /// Builds a fixture at `size` logical points.
    pub fn new(
        source: &'a dyn RowSource,
        state: GridState,
        options: GridOptions,
        size: [f32; 2],
    ) -> Self {
        let state = Rc::new(RefCell::new(state));
        let actions = Rc::new(RefCell::new(Vec::new()));
        let ui_state = Rc::clone(&state);
        let ui_actions = Rc::clone(&actions);
        let ui = UiHarness::new(Theme::desktop_light(), size, move |ui| {
            let mut guard = ui_state.borrow_mut();
            let shown = GridView {
                source,
                state: &mut guard,
                options: options.clone(),
            }
            .show(ui);
            drop(guard);
            ui_actions.borrow_mut().extend(shown);
        });
        Self { ui, state, actions }
    }

    /// Runs one frame.
    pub fn run(&mut self) {
        self.ui.run();
    }

    /// Takes the actions produced since the last call.
    pub fn take_actions(&self) -> Vec<GridAction> {
        std::mem::take(&mut *self.actions.borrow_mut())
    }

    /// The selected rows.
    pub fn selected_rows(&self) -> BTreeSet<usize> {
        self.state.borrow().selection.rows.clone()
    }

    /// The selected columns.
    pub fn selected_columns(&self) -> BTreeSet<usize> {
        self.state.borrow().selection.columns.clone()
    }
}

impl<'a> Deref for GridFixture<'a> {
    type Target = UiHarness<'a>;

    fn deref(&self) -> &Self::Target {
        &self.ui
    }
}

impl DerefMut for GridFixture<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ui
    }
}
