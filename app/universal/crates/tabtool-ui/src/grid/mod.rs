//! The virtualized grid engine: geometry, selection, editing and rendering.
//!
//! [`GridView`] paints only the cells whose frames intersect the viewport plus
//! a one-cell overscan. The header row and the row-number gutter are pinned,
//! and the persistent [`GridState`] survives source swaps after indexing.
//! Commands that change the document are returned as [`GridAction`] values;
//! the grid never mutates the source.
//!
//! # Display rows and large mode
//!
//! [`GridView`] always indexes rows as **display rows**. A large source maps a
//! display row to its physical row inside `RowSource::cell` (for example
//! `LargeCSVSource` adds its own header offset), so callers must not add the
//! offset to cell or selection indexes. [`GridOptions::header_offset`] only
//! shifts the row-number labels in the gutter.
//!
//! # Filtering
//!
//! A filtered view is passed through [`GridPresenter`], whose `visible_rows`
//! maps display rows to model rows. Row numbers in a filtered grid are display
//! numbers (`1..visible row count`), not model numbers, and
//! [`GridOptions::header_offset`] still applies on top of them.

mod chrome;
mod editor;
mod geometry;
mod keyboard;
mod paint;
mod pointer;
mod render;
mod selection;
mod state;

use std::collections::BTreeSet;

use egui::Ui;
use tabtool_core::{RowSource, SortDirection};

pub use editor::CellEditor;
pub use geometry::{
    COLUMN_OVERSCAN, DEFAULT_COLUMN_WIDTH, GUTTER_WIDTH, GridGeometry, HEADER_HEIGHT,
    MIN_COLUMN_WIDTH, RESIZE_HANDLE_WIDTH, ROW_OVERSCAN, SORT_ARROW_WIDTH, gutter_width,
};
pub use keyboard::{EditorKeyAction, MoveDirection};
pub use selection::{
    GridAnchor, GridElement, GridFocus, GridSelection, GridTapOutcome, move_focus, resolve_tap,
};
pub use state::{CellAlignment, GridOptions, GridPresenter, GridState, SortSpec};

/// A command the grid asks its owner to apply.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GridAction {
    /// Write `text` into a display cell.
    CommitCell {
        /// Display row.
        row: usize,
        /// Visual column.
        column: usize,
        /// The committed text.
        text: String,
    },
    /// Insert a row above the selection.
    AddRowAbove,
    /// Insert a row below the selection.
    AddRowBelow,
    /// Insert a column left of the selection.
    AddColumnLeft,
    /// Insert a column right of the selection.
    AddColumnRight,
    /// Delete the selected rows or columns.
    DeleteSelection,
    /// Move rows before `to`.
    MoveRows {
        /// Source display rows.
        from: BTreeSet<usize>,
        /// Destination display row.
        to: usize,
    },
    /// Move a column to a new visual position.
    MoveColumn {
        /// Source visual column.
        from: usize,
        /// Destination visual column.
        to: usize,
    },
    /// Sort by a column.
    Sort {
        /// The sorted visual column.
        column: usize,
        /// The direction.
        direction: SortDirection,
    },
    /// Rename a column.
    RenameColumn {
        /// The visual column.
        column: usize,
        /// The new name.
        name: String,
    },
    /// Copy the selection as TSV.
    Copy,
    /// Paste TSV into the selection.
    Paste,
    /// Select every row.
    SelectAll,
}

/// The virtualized grid widget.
///
/// Build one each frame with the current source and the persistent state.
/// `source` is indexed by display row; a large source adds its own physical
/// header offset inside `cell`, and `options.header_offset` is only used for
/// row-number labels:
///
/// ```no_run
/// # use tabtool_ui::grid::{GridView, GridState, GridOptions};
/// # fn show(ui: &mut egui::Ui, source: &dyn tabtool_core::RowSource) {
/// let mut state = GridState::default();
/// let actions = GridView {
///     source,
///     state: &mut state,
///     options: GridOptions::default(),
/// }
/// .show(ui);
/// # let _ = actions;
/// # }
/// ```
pub struct GridView<'a> {
    /// The rows to display.
    pub source: &'a dyn RowSource,
    /// The persistent scroll, selection, width and editor state.
    pub state: &'a mut GridState,
    /// The presentation settings.
    pub options: GridOptions,
}

impl GridView<'_> {
    /// Paints the grid and returns the commands the user requested.
    pub fn show(self, ui: &mut Ui) -> Vec<GridAction> {
        render::show(self, ui)
    }
}
