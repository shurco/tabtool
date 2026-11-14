//! Reading the cell or header a native edit targets.
//!
//! The touch editor addresses display rows and column identities; these
//! helpers read the seeded text through the same filtered presenter the grid
//! paints, so a display row never leaks into the model.

use tabtool_core::ColumnId;
use tabtool_document::Document;

use crate::grid::{GridFocus, GridPresenter};

/// The text of a display cell and the displayed row/column counts.
pub(crate) fn cell_text(document: &Document, focus: GridFocus) -> Option<(String, usize, usize)> {
    let source = document.row_source()?;
    let visible = document.row_filter().visible_rows.as_deref();
    let presenter = GridPresenter::new(source, visible);
    if focus.row >= presenter.row_count() || focus.column >= presenter.column_count() {
        return None;
    }
    Some((
        presenter.cell(focus.row, focus.column),
        presenter.row_count(),
        presenter.column_count(),
    ))
}

/// The stable identity and text of a visual column header.
pub(crate) fn header(document: &Document, column: usize) -> Option<(ColumnId, String)> {
    let source = document.row_source()?;
    let id = *source.column_ids().get(column)?;
    Some((id, GridPresenter::new(source, None).header(column)))
}

/// The current visual index of a column identity.
pub(crate) fn column_of(document: &Document, id: ColumnId) -> Option<usize> {
    document
        .row_source()?
        .column_ids()
        .iter()
        .position(|candidate| *candidate == id)
}
