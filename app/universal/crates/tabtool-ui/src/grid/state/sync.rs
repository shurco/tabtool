//! Fitting the grid state to a source and sizing its columns.

use tabtool_core::RowSource;

use super::GridState;
use crate::grid::geometry::DEFAULT_COLUMN_WIDTH;
use crate::grid::options::GridOptions;
use crate::grid::selection::GridAnchor;

impl GridState {
    /// Fits the state to the current source without discarding user state.
    ///
    /// Missing widths get the option default, extra widths are dropped, and
    /// selection, focus, hover and editing are clamped to the new bounds.
    pub fn sync(&mut self, source: &dyn RowSource, options: &GridOptions) {
        let rows = source.row_count();
        let columns = source.column_count();
        let column_ids = source.column_ids();
        if self
            .header_editing
            .as_ref()
            .is_some_and(|editing| !column_ids.contains(&editing.column))
        {
            self.header_editing = None;
        }
        if self
            .column_drag
            .as_ref()
            .is_some_and(|drag| !column_ids.contains(&drag.column))
        {
            self.column_drag = None;
        }
        if let Some(drag) = &mut self.column_drag {
            drag.target = drag.target.min(columns);
        }
        if self
            .row_drag
            .as_ref()
            .is_some_and(|drag| drag.rows.iter().any(|row| *row >= rows))
        {
            self.row_drag = None;
        }
        if let Some(drag) = &mut self.row_drag
            && let Some(target) = drag.target
        {
            drag.target = Some(target.min(rows));
        }
        self.widths.retain(|id, _| column_ids.contains(id));
        for id in column_ids {
            let width = self
                .widths
                .entry(id)
                .or_insert(options.default_column_width);
            *width = clamp_width(*width, options);
        }
        self.selection.rows.retain(|row| *row < rows);
        self.selection.columns.retain(|column| *column < columns);
        if self.selection.rows.is_empty() && self.selection.columns.is_empty() {
            self.anchor = None;
        }
        if let Some(editing) = &self.editing
            && (editing.row >= rows || editing.column >= columns)
        {
            self.editing = None;
        }
        if self
            .focus
            .is_some_and(|focus| focus.row >= rows || focus.column >= columns)
        {
            self.focus = None;
        }
        if self
            .pending_focus
            .is_some_and(|focus| focus.row >= rows || focus.column >= columns)
        {
            self.pending_focus = None;
        }
        if self
            .hovered
            .is_some_and(|(row, column)| row >= rows || column >= columns)
        {
            self.hovered = None;
        }
        if self
            .pending_scroll
            .is_some_and(|focus| focus.row >= rows || focus.column >= columns)
        {
            self.pending_scroll = None;
        }
        self.anchor = match self.anchor {
            Some(GridAnchor::Row(row)) if row < rows => Some(GridAnchor::Row(row)),
            Some(GridAnchor::Column(column)) if column < columns => {
                Some(GridAnchor::Column(column))
            }
            _ => None,
        };
    }

    /// The column widths in `source`'s current visual order.
    pub fn ordered_widths(&self, source: &dyn RowSource) -> Vec<f32> {
        source
            .column_ids()
            .into_iter()
            .map(|id| {
                self.widths
                    .get(&id)
                    .copied()
                    .unwrap_or(DEFAULT_COLUMN_WIDTH)
            })
            .collect()
    }

    /// Resizes one column by a pointer delta, clamped to the options.
    pub fn resize_column(
        &mut self,
        source: &dyn RowSource,
        column: usize,
        delta: f32,
        options: &GridOptions,
    ) {
        let Some(id) = source.column_ids().get(column).copied() else {
            return;
        };
        let width = self
            .widths
            .entry(id)
            .or_insert(options.default_column_width);
        *width = clamp_width(*width + delta, options);
    }
}

fn clamp_width(width: f32, options: &GridOptions) -> f32 {
    let min = options.min_column_width.max(0.0);
    let max = options.max_column_width.max(min);
    width.clamp(min, max)
}
