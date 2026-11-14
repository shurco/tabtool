//! Screen placement of the native text editor over a grid cell or header.
//!
//! The grid paints in content coordinates; the native editor is placed in
//! screen points. These functions convert between the two using the viewport
//! and grid rectangle from the last frame, and report the area the target must
//! stay inside so the caller can scroll it above the keyboard first.

use egui::{Rect, pos2, vec2};
use tabtool_document::Document;

use super::editor_state::{MobileEditTarget, MobileEditorState};
use crate::grid::{GridFocus, GridGeometry, GridOptions, GridState, HEADER_HEIGHT};

/// A native editor rectangle and the grid area it must stay inside.
pub(crate) struct Placement {
    /// The target in screen points.
    pub rect: Rect,
    /// The grid area that is not covered by the pinned header or gutter.
    pub visible: Rect,
}

/// The screen rectangle of `target` in the last grid frame.
pub(crate) fn placement(
    document: &Document,
    state: &GridState,
    editor: &MobileEditorState,
    options: &GridOptions,
    target: MobileEditTarget,
) -> Option<Placement> {
    let source = document.row_source()?;
    let widths = state.ordered_widths(source);
    let geometry = GridGeometry {
        widths: &widths,
        row_height: options.row_height,
        row_count: source.row_count(),
        shows_row_numbers: options.shows_row_numbers,
        header_offset: options.header_offset,
    };
    let grid = editor.grid_rect;
    let viewport = state.viewport;
    let gutter = geometry.gutter();
    match target {
        MobileEditTarget::Cell { row, column } => {
            let rect = geometry
                .cell_rect(row, column)
                .translate(grid.min.to_vec2() - viewport.min.to_vec2());
            let visible = Rect::from_min_max(
                pos2(grid.min.x + gutter, grid.min.y + HEADER_HEIGHT),
                grid.max,
            );
            Some(Placement { rect, visible })
        }
        MobileEditTarget::Column(id) => {
            let column = source
                .column_ids()
                .iter()
                .position(|candidate| *candidate == id)?;
            let rect = Rect::from_min_size(
                pos2(
                    grid.min.x + geometry.column_x(column) - viewport.min.x,
                    grid.min.y,
                ),
                vec2(geometry.column_width(column), HEADER_HEIGHT),
            );
            let visible = Rect::from_min_max(
                pos2(grid.min.x + gutter, grid.min.y),
                pos2(grid.max.x, grid.min.y + HEADER_HEIGHT),
            );
            Some(Placement { rect, visible })
        }
    }
}

/// The cell that scrolling must reveal for `target`.
pub(crate) fn scroll_target(
    document: &Document,
    state: &GridState,
    target: MobileEditTarget,
) -> Option<GridFocus> {
    match target {
        MobileEditTarget::Cell { row, column } => Some(GridFocus::new(row, column)),
        MobileEditTarget::Column(id) => {
            let column = document
                .row_source()?
                .column_ids()
                .iter()
                .position(|candidate| *candidate == id)?;
            let row = state.focus.map_or(0, |focus| focus.row);
            Some(GridFocus::new(row, column))
        }
    }
}
